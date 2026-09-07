//! Application state managed by Tauri.
//!
//! Holds the [`Vault`] (the single security boundary), transient Google-import
//! sessions (whose staged secrets live only in Rust memory), and the clipboard
//! auto-clear + idle-lock trackers.

use clovakey_core::import::google::BatchCollector;
use clovakey_core::model::OtpConfig;
use clovakey_core::otp::{Algorithm, OtpType};
use clovakey_core::{dedupe, Vault};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use zeroize::Zeroizing;

/// A staged, not-yet-committed OTP record from an import. The secret stays in
/// Rust; only the metadata below is ever serialized to the UI.
pub struct StagedRecord {
    pub secret: Zeroizing<Vec<u8>>,
    pub issuer: Option<String>,
    pub account_name: String,
    pub otp: OtpConfig,
    pub duplicate: bool,
}

/// A preview row for the import wizard — deliberately excludes the secret.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewItem {
    pub index: usize,
    pub issuer: Option<String>,
    pub account_name: String,
    #[serde(rename = "type")]
    pub otp_type: OtpType,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub period: u32,
    pub counter: u64,
    pub duplicate: bool,
}

/// An in-progress Google Authenticator import.
pub struct GoogleSession {
    pub collector: BatchCollector,
    pub records: Vec<StagedRecord>,
    pub finalized: bool,
}

impl GoogleSession {
    pub fn new() -> Self {
        GoogleSession {
            collector: BatchCollector::new(),
            records: Vec::new(),
            finalized: false,
        }
    }

    /// Once every batch has arrived, decode records, fingerprint them against
    /// the vault, and flag duplicates. Requires the vault to be unlocked.
    pub fn finalize(&mut self, vault: &Vault) -> clovakey_core::Result<()> {
        if self.finalized {
            return Ok(());
        }
        let collector = std::mem::take(&mut self.collector);
        let migration_records = collector.into_records();
        let existing = vault.existing_fingerprints()?;
        let mut seen: Vec<Vec<u8>> = Vec::new();

        for m in migration_records {
            let otp = OtpConfig {
                otp_type: m.otp_type,
                algorithm: m.algorithm,
                digits: m.digits,
                period: clovakey_core::otp::DEFAULT_PERIOD,
                counter: m.counter,
            };
            let fp = vault.fingerprint_for(&m.secret, &otp)?;
            let dup_existing = existing.iter().any(|e| dedupe::fingerprints_match(e, &fp));
            let dup_within = seen.iter().any(|e| dedupe::fingerprints_match(e, &fp));
            seen.push(fp);
            self.records.push(StagedRecord {
                secret: m.secret,
                issuer: if m.issuer.is_empty() {
                    None
                } else {
                    Some(m.issuer)
                },
                account_name: m.name,
                otp,
                duplicate: dup_existing || dup_within,
            });
        }
        self.finalized = true;
        Ok(())
    }

    pub fn preview(&self) -> Vec<PreviewItem> {
        self.records
            .iter()
            .enumerate()
            .map(|(index, r)| PreviewItem {
                index,
                issuer: r.issuer.clone(),
                account_name: r.account_name.clone(),
                otp_type: r.otp.otp_type,
                algorithm: r.otp.algorithm,
                digits: r.otp.digits,
                period: r.otp.period,
                counter: r.otp.counter,
                duplicate: r.duplicate,
            })
            .collect()
    }
}

impl Default for GoogleSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks what ClovaKey last placed on the clipboard so an auto-clear never
/// erases something the user copied afterwards.
#[derive(Default)]
pub struct ClipboardTracker {
    /// Bumped on every copy; a scheduled clear only fires if it still matches.
    pub token: u64,
    /// The exact text ClovaKey last wrote (so we only clear our own value).
    pub last_value: Option<String>,
}

/// Tracks user activity for idle auto-lock.
pub struct ActivityTracker {
    pub last_active: Instant,
}

impl Default for ActivityTracker {
    fn default() -> Self {
        ActivityTracker {
            last_active: Instant::now(),
        }
    }
}

/// The managed application state.
pub struct AppState {
    pub vault: Arc<Vault>,
    pub google_sessions: Mutex<HashMap<String, GoogleSession>>,
    pub clipboard: Mutex<ClipboardTracker>,
    pub activity: Mutex<ActivityTracker>,
}

impl AppState {
    pub fn new(vault: Vault) -> Self {
        AppState {
            vault: Arc::new(vault),
            google_sessions: Mutex::new(HashMap::new()),
            clipboard: Mutex::new(ClipboardTracker::default()),
            activity: Mutex::new(ActivityTracker::default()),
        }
    }

    pub fn note_activity(&self) {
        self.activity.lock().unwrap().last_active = Instant::now();
    }
}
