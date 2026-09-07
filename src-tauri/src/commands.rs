//! Tauri command surface.
//!
//! Security invariant: OTP **secrets** never leave Rust during normal use. The
//! frontend asks for codes and metadata by account id. The only places raw
//! secret material crosses the boundary are (a) creation, where the user
//! themselves supplied it, and (b) explicit, re-authenticated export/reveal.

use crate::error::{AppError, AppResult};
use crate::state::{AppState, GoogleSession, PreviewItem};
use crate::{clipboard, settings};
use clovakey_core::backup::{self, DuplicatePolicy, ImportSummary};
use clovakey_core::import::google;
use clovakey_core::import::{classify, ScannedKind};
use clovakey_core::model::{
    new_id, Account, AccountPatch, GeneratedCode, Group, NewAccount, OtpConfig,
};
use clovakey_core::otp::{self, base32, Algorithm, OtpType};
use clovakey_core::storage::VaultStatus;
use clovakey_core::Vault;
use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn decode_image_b64(image_base64: &str) -> AppResult<Vec<u8>> {
    BASE64
        .decode(image_base64.as_bytes())
        .map_err(|_| AppError::new("image", "That image could not be read."))
}

/// Gate an explicit secret-reveal/export. In passphrase mode the passphrase is
/// required and re-verified; in keychain mode the unlocked session is the gate.
fn require_reveal_auth(vault: &Vault, passphrase: Option<&str>) -> AppResult<()> {
    if vault.status()?.protection.as_deref() == Some("passphrase") {
        let p = passphrase
            .ok_or_else(|| AppError::new("bad_passphrase", "Enter your passphrase to continue."))?;
        if !vault.verify_passphrase(p)? {
            return Err(AppError::new("bad_passphrase", "Incorrect passphrase."));
        }
    }
    Ok(())
}

fn add_uri_internal(vault: &Vault, uri: &str) -> AppResult<Account> {
    let p = otp::uri::parse(uri)?;
    let account_name = if p.account.trim().is_empty() {
        p.issuer.clone().unwrap_or_else(|| "Account".to_string())
    } else {
        p.account.clone()
    };
    let new = NewAccount {
        issuer: p.issuer.clone(),
        account_name,
        otp: OtpConfig {
            otp_type: p.otp_type,
            algorithm: p.algorithm,
            digits: p.digits,
            period: p.period,
            counter: p.counter,
        },
        group_id: None,
        favorite: false,
        icon: None,
    };
    Ok(vault.add_account(&new, &p.secret)?)
}

// ─────────────────────────────────────────────────────────────────────────────
// Vault lifecycle
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn vault_status(state: State<AppState>) -> AppResult<VaultStatus> {
    Ok(state.vault.status()?)
}

#[tauri::command]
pub fn vault_setup(state: State<AppState>, passphrase: Option<String>) -> AppResult<VaultStatus> {
    state.vault.setup(passphrase.as_deref())?;
    state.note_activity();
    Ok(state.vault.status()?)
}

#[tauri::command]
pub fn vault_unlock(state: State<AppState>, passphrase: Option<String>) -> AppResult<VaultStatus> {
    state.vault.unlock(passphrase.as_deref())?;
    state.note_activity();
    Ok(state.vault.status()?)
}

#[tauri::command]
pub fn vault_lock(app: AppHandle, state: State<AppState>) -> AppResult<()> {
    state.vault.lock();
    let _ = app.emit("clovakey://locked", ());
    Ok(())
}

#[tauri::command]
pub fn vault_set_passphrase(state: State<AppState>, passphrase: String) -> AppResult<()> {
    state.vault.enable_passphrase(&passphrase)?;
    Ok(())
}

#[tauri::command]
pub fn vault_remove_passphrase(state: State<AppState>, passphrase: String) -> AppResult<()> {
    // Verify current passphrase before downgrading to keychain protection.
    if !state.vault.verify_passphrase(&passphrase)? {
        return Err(AppError::new("bad_passphrase", "Incorrect passphrase."));
    }
    state.vault.disable_passphrase()?;
    Ok(())
}

#[tauri::command]
pub fn vault_change_passphrase(
    state: State<AppState>,
    old_passphrase: String,
    new_passphrase: String,
) -> AppResult<()> {
    state
        .vault
        .change_passphrase(&old_passphrase, &new_passphrase)?;
    Ok(())
}

#[tauri::command]
pub fn note_activity(state: State<AppState>) {
    state.note_activity();
}

// ─────────────────────────────────────────────────────────────────────────────
// Accounts & codes
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn accounts_list(state: State<AppState>) -> AppResult<Vec<Account>> {
    Ok(state.vault.list_accounts()?)
}

#[tauri::command]
pub fn codes_generate_all(state: State<AppState>) -> AppResult<Vec<GeneratedCode>> {
    Ok(state.vault.generate_all()?)
}

#[tauri::command]
pub fn code_generate(state: State<AppState>, id: String) -> AppResult<GeneratedCode> {
    Ok(state.vault.generate_code(&id)?)
}

#[tauri::command]
pub fn hotp_advance(state: State<AppState>, id: String) -> AppResult<GeneratedCode> {
    state.note_activity();
    Ok(state.vault.advance_hotp(&id)?)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualAccountInput {
    pub issuer: Option<String>,
    pub account_name: String,
    pub secret: String,
    #[serde(rename = "type")]
    pub otp_type: OtpType,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub period: u32,
    #[serde(default)]
    pub counter: u64,
    pub group_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
    pub icon: Option<String>,
}

#[tauri::command]
pub fn account_add_manual(state: State<AppState>, input: ManualAccountInput) -> AppResult<Account> {
    let secret = base32::decode_secret(&input.secret)?;
    let issuer = input
        .issuer
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let account_name = input.account_name.trim().to_string();
    if account_name.is_empty() && issuer.is_none() {
        return Err(AppError::new(
            "invalid_params",
            "Enter an account name or issuer.",
        ));
    }
    let new = NewAccount {
        issuer,
        account_name: if account_name.is_empty() {
            "Account".to_string()
        } else {
            account_name
        },
        otp: OtpConfig {
            otp_type: input.otp_type,
            algorithm: input.algorithm,
            digits: input.digits,
            period: input.period,
            counter: input.counter,
        },
        group_id: input.group_id,
        favorite: input.favorite,
        icon: input.icon,
    };
    Ok(state.vault.add_account(&new, &secret)?)
}

#[tauri::command]
pub fn account_add_uri(state: State<AppState>, uri: String) -> AppResult<Account> {
    add_uri_internal(&state.vault, &uri)
}

#[tauri::command]
pub fn account_update(
    state: State<AppState>,
    id: String,
    patch: AccountPatch,
) -> AppResult<Account> {
    Ok(state.vault.update_account(&id, &patch)?)
}

#[tauri::command]
pub fn account_delete(state: State<AppState>, id: String) -> AppResult<()> {
    Ok(state.vault.delete_account(&id)?)
}

#[tauri::command]
pub fn account_reorder(state: State<AppState>, ids: Vec<String>) -> AppResult<()> {
    Ok(state.vault.reorder_accounts(&ids)?)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealedSecret {
    pub uri: String,
    pub secret: String,
    pub qr_svg: String,
}

#[tauri::command]
pub fn account_reveal(
    state: State<AppState>,
    id: String,
    passphrase: Option<String>,
) -> AppResult<RevealedSecret> {
    require_reveal_auth(&state.vault, passphrase.as_deref())?;
    let uri = state.vault.export_account_uri(&id)?;
    let parsed = otp::uri::parse(&uri)?;
    let secret = base32::encode_secret(&parsed.secret);
    let qr_svg = clovakey_core::qr::encode_svg(&uri)?;
    Ok(RevealedSecret {
        uri: uri.to_string(),
        secret,
        qr_svg,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Groups
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn groups_list(state: State<AppState>) -> AppResult<Vec<Group>> {
    Ok(state.vault.list_groups()?)
}

#[tauri::command]
pub fn group_create(state: State<AppState>, name: String) -> AppResult<Group> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            "invalid_params",
            "Group name cannot be empty.",
        ));
    }
    Ok(state.vault.create_group(name)?)
}

#[tauri::command]
pub fn group_rename(state: State<AppState>, id: String, name: String) -> AppResult<()> {
    Ok(state.vault.rename_group(&id, name.trim())?)
}

#[tauri::command]
pub fn group_delete(state: State<AppState>, id: String) -> AppResult<()> {
    Ok(state.vault.delete_group(&id)?)
}

// ─────────────────────────────────────────────────────────────────────────────
// Clipboard
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn copy_code(app: AppHandle, state: State<AppState>, id: String) -> AppResult<()> {
    state.note_activity();
    let code = state.vault.generate_code(&id)?;
    let secs = settings::clipboard_clear_secs(&state.vault);
    clipboard::copy_with_autoclear(&app, code.code, secs)
        .map_err(|e| AppError::new("clipboard", e))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Scan (single account) — otpauth is added; migration routes to the wizard.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOutcome {
    pub kind: String,
    pub account: Option<Account>,
}

fn kind_str(k: ScannedKind) -> &'static str {
    match k {
        ScannedKind::Otpauth => "otpauth",
        ScannedKind::Migration => "migration",
        ScannedKind::Unknown => "unknown",
    }
}

#[tauri::command]
pub fn scan_qr_text(state: State<AppState>, text: String) -> AppResult<ScanOutcome> {
    match classify(&text) {
        ScannedKind::Otpauth => {
            let account = add_uri_internal(&state.vault, &text)?;
            Ok(ScanOutcome {
                kind: "otpauth".into(),
                account: Some(account),
            })
        }
        k => Ok(ScanOutcome {
            kind: kind_str(k).into(),
            account: None,
        }),
    }
}

#[tauri::command]
pub fn scan_qr_image(state: State<AppState>, image_base64: String) -> AppResult<ScanOutcome> {
    let bytes = decode_image_b64(&image_base64)?;
    let contents = clovakey_core::qr::decode_all(&bytes)?;
    if contents.is_empty() {
        return Err(AppError::new(
            "qr_decode",
            "No QR code was found in that image.",
        ));
    }
    for c in &contents {
        if classify(c) == ScannedKind::Otpauth {
            let account = add_uri_internal(&state.vault, c)?;
            return Ok(ScanOutcome {
                kind: "otpauth".into(),
                account: Some(account),
            });
        }
    }
    if contents
        .iter()
        .any(|c| classify(c) == ScannedKind::Migration)
    {
        return Ok(ScanOutcome {
            kind: "migration".into(),
            account: None,
        });
    }
    Ok(ScanOutcome {
        kind: "unknown".into(),
        account: None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Google Authenticator import wizard
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgressDto {
    pub received: usize,
    pub total: usize,
    pub complete: bool,
}

impl From<google::BatchProgress> for BatchProgressDto {
    fn from(p: google::BatchProgress) -> Self {
        BatchProgressDto {
            received: p.received,
            total: p.total,
            complete: p.complete,
        }
    }
}

#[tauri::command]
pub fn google_import_begin(state: State<AppState>) -> AppResult<String> {
    let id = new_id();
    state
        .google_sessions
        .lock()
        .unwrap()
        .insert(id.clone(), GoogleSession::new());
    Ok(id)
}

fn add_migration_payload(
    state: &AppState,
    session_id: &str,
    payload: google::MigrationPayload,
) -> AppResult<BatchProgressDto> {
    let mut sessions = state.google_sessions.lock().unwrap();
    let session = sessions
        .get_mut(session_id)
        .ok_or_else(|| AppError::new("not_found", "Import session expired. Start again."))?;
    let progress = session.collector.add(payload)?;
    if progress.complete {
        session.finalize(&state.vault)?;
    }
    Ok(progress.into())
}

#[tauri::command]
pub fn google_import_add_text(
    state: State<AppState>,
    session_id: String,
    text: String,
) -> AppResult<BatchProgressDto> {
    let payload = google::parse_migration_uri(&text)?;
    add_migration_payload(&state, &session_id, payload)
}

#[tauri::command]
pub fn google_import_add_image(
    state: State<AppState>,
    session_id: String,
    image_base64: String,
) -> AppResult<BatchProgressDto> {
    let bytes = decode_image_b64(&image_base64)?;
    let contents = clovakey_core::qr::decode_all(&bytes)?;
    let migrations: Vec<_> = contents
        .iter()
        .filter(|c| classify(c) == ScannedKind::Migration)
        .collect();
    if migrations.is_empty() {
        return Err(AppError::new(
            "migration_decode",
            "That image doesn't contain a Google Authenticator export QR code.",
        ));
    }
    let mut last = None;
    for c in migrations {
        let payload = google::parse_migration_uri(c)?;
        last = Some(add_migration_payload(&state, &session_id, payload)?);
    }
    Ok(last.expect("at least one migration payload processed"))
}

#[tauri::command]
pub fn google_import_preview(
    state: State<AppState>,
    session_id: String,
) -> AppResult<Vec<PreviewItem>> {
    let mut sessions = state.google_sessions.lock().unwrap();
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| AppError::new("not_found", "Import session expired. Start again."))?;
    if !session.finalized && session.collector.is_complete() {
        session.finalize(&state.vault)?;
    }
    Ok(session.preview())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSelection {
    pub index: usize,
    pub issuer: Option<String>,
    pub account_name: Option<String>,
    pub group_id: Option<String>,
    #[serde(default)]
    pub favorite: bool,
}

#[tauri::command]
pub fn google_import_commit(
    state: State<AppState>,
    session_id: String,
    selections: Vec<ImportSelection>,
) -> AppResult<ImportSummary> {
    // Take ownership of the session so its secrets are dropped/zeroized when done.
    let mut session = {
        let mut sessions = state.google_sessions.lock().unwrap();
        sessions
            .remove(&session_id)
            .ok_or_else(|| AppError::new("not_found", "Import session expired. Start again."))?
    };
    if !session.finalized {
        if session.collector.is_complete() {
            session.finalize(&state.vault)?;
        } else {
            return Err(AppError::new(
                "migration_decode",
                "Scan all export QR codes before importing.",
            ));
        }
    }

    let mut summary = ImportSummary::default();
    for sel in selections {
        let rec = session
            .records
            .get(sel.index)
            .ok_or_else(|| AppError::new("not_found", "Selected account no longer available."))?;
        let issuer = sel
            .issuer
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| rec.issuer.clone());
        let account_name = sel
            .account_name
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| rec.account_name.clone());
        let new = NewAccount {
            issuer,
            account_name: if account_name.is_empty() {
                "Account".to_string()
            } else {
                account_name
            },
            otp: rec.otp,
            group_id: sel.group_id,
            favorite: sel.favorite,
            icon: None,
        };
        state.vault.add_account(&new, &rec.secret)?;
        summary.imported += 1;
    }
    Ok(summary)
}

#[tauri::command]
pub fn google_import_cancel(state: State<AppState>, session_id: String) {
    // Dropping the session zeroizes its staged secrets.
    state.google_sessions.lock().unwrap().remove(&session_id);
}

// ─────────────────────────────────────────────────────────────────────────────
// Backup & restore
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn backup_create(state: State<AppState>, path: String, password: String) -> AppResult<()> {
    if password.len() < 8 {
        return Err(AppError::new(
            "invalid_params",
            "Use a backup passphrase of at least 8 characters.",
        ));
    }
    let payload = state.vault.export_payload()?;
    let bytes = backup::seal_payload(&payload, &password)?;
    std::fs::write(&path, &bytes)
        .map_err(|e| AppError::new("io", format!("Could not write the backup file: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn backup_restore(
    state: State<AppState>,
    path: String,
    password: String,
    import_all: bool,
) -> AppResult<ImportSummary> {
    let bytes = std::fs::read(&path)
        .map_err(|e| AppError::new("io", format!("Could not read the backup file: {e}")))?;
    let payload = backup::open_payload(&bytes, &password)?;
    let policy = if import_all {
        DuplicatePolicy::ImportAll
    } else {
        DuplicatePolicy::Skip
    };
    Ok(state.vault.import_payload(&payload, policy)?)
}

// ─────────────────────────────────────────────────────────────────────────────
// Settings & data
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn settings_get_all(state: State<AppState>) -> AppResult<serde_json::Value> {
    settings::get_all(&state.vault)
}

#[tauri::command]
pub fn settings_set(state: State<AppState>, key: String, value: String) -> AppResult<()> {
    settings::set(&state.vault, &key, &value)?;
    Ok(())
}

#[tauri::command]
pub fn data_wipe(
    app: AppHandle,
    state: State<AppState>,
    passphrase: Option<String>,
) -> AppResult<()> {
    require_reveal_auth(&state.vault, passphrase.as_deref())?;
    state.vault.wipe()?;
    let _ = app.emit("clovakey://locked", ());
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Environment / time / capabilities
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeStatus {
    pub unix_time: u64,
    pub ok: bool,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn time_status() -> TimeStatus {
    let now = otp::unix_now();
    let build_ts: u64 = env!("CLOVAKEY_BUILD_TS").parse().unwrap_or(0);
    // Codes fail if the clock is wildly off. Flag a clock set before the build
    // (with a day of slack for timezone/build skew) or absurdly far ahead.
    let (ok, reason) = if build_ts > 0 && now + 86_400 < build_ts {
        (
            false,
            Some("Your system clock appears to be set in the past. Authenticator codes may be rejected until the time is corrected.".to_string()),
        )
    } else if build_ts > 0 && now > build_ts + 20 * 365 * 86_400 {
        (
            false,
            Some("Your system clock appears to be set far in the future. Authenticator codes may be rejected until the time is corrected.".to_string()),
        )
    } else {
        (true, None)
    };
    TimeStatus {
        unix_time: now,
        ok,
        reason,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityCapabilities {
    pub platform: String,
    pub biometric_available: bool,
    pub biometric_kind: Option<String>,
}

#[tauri::command]
pub fn security_capabilities() -> SecurityCapabilities {
    // Biometric unlock (Touch ID / Windows Hello) is planned; the abstraction
    // exists in `platform::biometric` but no reliable desktop implementation is
    // wired yet, so we report it unavailable rather than shipping something
    // fragile. See docs/SECURITY.md.
    let kind = crate::platform::biometric::preferred_kind();
    SecurityCapabilities {
        platform: std::env::consts::OS.to_string(),
        biometric_available: crate::platform::biometric::is_available(),
        biometric_kind: kind,
    }
}
