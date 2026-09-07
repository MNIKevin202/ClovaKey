//! The encrypted vault: an SQLite database plus an in-memory master key.
//!
//! ## At rest
//! Each account's OTP secret is encrypted individually with XChaCha20-Poly1305,
//! with the account id as AAD (so a ciphertext cannot be moved between records).
//! All other columns are non-sensitive, searchable metadata.
//!
//! ## Master key
//! A random 256-bit key encrypts every secret. It is protected in one of two
//! ways, recorded in `vault_meta.protection`:
//! - `keychain` — stored in the OS credential store; unlock re-fetches it.
//! - `passphrase` — Argon2id-wrapped by a user passphrase and stored in the DB;
//!   unlock requires the passphrase. This is a *hard* lock: while locked, no
//!   secret can be decrypted and no code can be generated.
//!
//! The key lives in memory only while unlocked and is zeroized on lock.

pub mod migrations;

use crate::backup;
use crate::crypto::{aead, kdf, kdf::KdfParams, SymmetricKey};
use crate::dedupe;
use crate::error::{CoreError, Result};
use crate::keystore::{KeyStore, OsKeyStore};
use crate::model::{new_id, Account, AccountPatch, GeneratedCode, Group, NewAccount, OtpConfig};
use crate::otp::{self, base32, Algorithm, OtpType};
use data_encoding::BASE64;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use zeroize::Zeroizing;

const AAD_WRAP: &[u8] = b"clovakey-master-wrap-v1";
const AAD_CHECK: &[u8] = b"clovakey-vault-check-v1";
const VAULT_CHECK_PLAINTEXT: &[u8] = b"clovakey-vault-ok";

const META_PROTECTION: &str = "protection";
const META_WRAPPED_KEY: &str = "wrapped_key";
const META_KDF_SALT: &str = "kdf_salt";
const META_KDF_PARAMS: &str = "kdf_params";
const META_VAULT_CHECK: &str = "vault_check";
const MASTER_KEY_LABEL: &str = "vault-master-key";

/// How the master key is protected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protection {
    Keychain,
    Passphrase,
}

impl Protection {
    fn as_str(self) -> &'static str {
        match self {
            Protection::Keychain => "keychain",
            Protection::Passphrase => "passphrase",
        }
    }
    fn parse(s: &str) -> Option<Self> {
        match s {
            "keychain" => Some(Protection::Keychain),
            "passphrase" => Some(Protection::Passphrase),
            _ => None,
        }
    }
}

/// Snapshot of vault status for the UI (never contains secrets).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub initialized: bool,
    pub locked: bool,
    pub protection: Option<String>,
}

/// The vault.
pub struct Vault {
    conn: Mutex<Connection>,
    keystore: Box<dyn KeyStore>,
    master: Mutex<Option<SymmetricKey>>,
}

impl Vault {
    /// Open (creating if needed) the vault database at `path`, using the OS
    /// keychain under `service` for master-key protection.
    pub fn open(path: impl AsRef<Path>, service: impl Into<String>) -> Result<Self> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn, Box::new(OsKeyStore::new(service)))
    }

    /// Open a vault with an explicit key store (used by tests with a memory store).
    pub fn from_connection(conn: Connection, keystore: Box<dyn KeyStore>) -> Result<Self> {
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        migrations::migrate(&conn)?;
        Ok(Vault {
            conn: Mutex::new(conn),
            keystore,
            master: Mutex::new(None),
        })
    }

    // ── meta helpers ─────────────────────────────────────────────────────

    fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row("SELECT value FROM vault_meta WHERE key = ?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .optional()?)
    }

    fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO vault_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    fn del_meta(&self, key: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM vault_meta WHERE key = ?1", [key])?;
        Ok(())
    }

    // ── lifecycle ────────────────────────────────────────────────────────

    /// Whether the vault has been set up (a protection mode is recorded).
    pub fn is_initialized(&self) -> Result<bool> {
        Ok(self.get_meta(META_PROTECTION)?.is_some())
    }

    /// Whether the vault is currently locked (no master key in memory).
    pub fn is_locked(&self) -> bool {
        self.master.lock().unwrap().is_none()
    }

    /// Current status snapshot.
    pub fn status(&self) -> Result<VaultStatus> {
        Ok(VaultStatus {
            initialized: self.is_initialized()?,
            locked: self.is_locked(),
            protection: self.get_meta(META_PROTECTION)?,
        })
    }

    fn protection(&self) -> Result<Protection> {
        self.get_meta(META_PROTECTION)?
            .and_then(|s| Protection::parse(&s))
            .ok_or(CoreError::VaultUninitialized)
    }

    /// First-run setup. `passphrase = None` uses keychain protection.
    pub fn setup(&self, passphrase: Option<&str>) -> Result<()> {
        if self.is_initialized()? {
            return Err(CoreError::Message("vault is already set up".into()));
        }
        let master = SymmetricKey::generate()?;

        match passphrase {
            Some(pass) if !pass.is_empty() => self.write_passphrase_wrapping(&master, pass)?,
            _ => {
                self.keystore.store(MASTER_KEY_LABEL, master.as_bytes())?;
                self.set_meta(META_PROTECTION, Protection::Keychain.as_str())?;
            }
        }

        let check = aead::seal(&master, VAULT_CHECK_PLAINTEXT, AAD_CHECK)?;
        self.set_meta(META_VAULT_CHECK, &BASE64.encode(&check))?;

        *self.master.lock().unwrap() = Some(master);
        Ok(())
    }

    fn write_passphrase_wrapping(&self, master: &SymmetricKey, pass: &str) -> Result<()> {
        let salt = kdf::generate_salt()?;
        let params = KdfParams::default();
        let wrap = kdf::derive_key(pass.as_bytes(), &salt, &params)?;
        let blob = aead::seal(&wrap, master.as_bytes(), AAD_WRAP)?;
        self.set_meta(META_WRAPPED_KEY, &BASE64.encode(&blob))?;
        self.set_meta(META_KDF_SALT, &BASE64.encode(&salt))?;
        self.set_meta(META_KDF_PARAMS, &serde_json::to_string(&params).unwrap())?;
        self.set_meta(META_PROTECTION, Protection::Passphrase.as_str())?;
        Ok(())
    }

    fn unwrap_with_passphrase(&self, pass: &str) -> Result<SymmetricKey> {
        let salt = BASE64
            .decode(
                self.get_meta(META_KDF_SALT)?
                    .ok_or(CoreError::VaultUninitialized)?
                    .as_bytes(),
            )
            .map_err(|_| CoreError::Crypto)?;
        let params: KdfParams = serde_json::from_str(
            &self
                .get_meta(META_KDF_PARAMS)?
                .ok_or(CoreError::VaultUninitialized)?,
        )
        .map_err(|_| CoreError::Crypto)?;
        let blob = BASE64
            .decode(
                self.get_meta(META_WRAPPED_KEY)?
                    .ok_or(CoreError::VaultUninitialized)?
                    .as_bytes(),
            )
            .map_err(|_| CoreError::Crypto)?;
        let wrap = kdf::derive_key(pass.as_bytes(), &salt, &params)?;
        let master_bytes =
            aead::open(&wrap, &blob, AAD_WRAP).map_err(|_| CoreError::BadPassphrase)?;
        SymmetricKey::from_slice(&master_bytes)
    }

    /// Unlock the vault. Requires the passphrase in passphrase mode.
    pub fn unlock(&self, passphrase: Option<&str>) -> Result<()> {
        let master = match self.protection()? {
            Protection::Passphrase => {
                let pass = passphrase.ok_or(CoreError::BadPassphrase)?;
                self.unwrap_with_passphrase(pass)?
            }
            Protection::Keychain => {
                let bytes = self
                    .keystore
                    .retrieve(MASTER_KEY_LABEL)?
                    .ok_or_else(|| CoreError::KeyStore("master key is missing".into()))?;
                SymmetricKey::from_slice(&bytes)?
            }
        };

        // Verify the key actually decrypts this vault.
        let check = BASE64
            .decode(
                self.get_meta(META_VAULT_CHECK)?
                    .ok_or(CoreError::VaultUninitialized)?
                    .as_bytes(),
            )
            .map_err(|_| CoreError::Crypto)?;
        let plain = aead::open(&master, &check, AAD_CHECK).map_err(|_| CoreError::Crypto)?;
        if plain.as_slice() != VAULT_CHECK_PLAINTEXT {
            return Err(CoreError::Crypto);
        }

        *self.master.lock().unwrap() = Some(master);
        Ok(())
    }

    /// Lock the vault, zeroizing the master key.
    pub fn lock(&self) {
        *self.master.lock().unwrap() = None;
    }

    /// Verify a passphrase without changing lock state. Returns false in
    /// keychain mode (there is no passphrase to check).
    pub fn verify_passphrase(&self, pass: &str) -> Result<bool> {
        match self.protection()? {
            Protection::Passphrase => Ok(self.unwrap_with_passphrase(pass).is_ok()),
            Protection::Keychain => Ok(false),
        }
    }

    /// Run `f` with the unlocked master key, or fail with `VaultLocked`.
    fn with_master<T>(&self, f: impl FnOnce(&SymmetricKey) -> Result<T>) -> Result<T> {
        let guard = self.master.lock().unwrap();
        let key = guard.as_ref().ok_or(CoreError::VaultLocked)?;
        f(key)
    }

    // ── passphrase management (while unlocked) ─────────────────────────────

    /// Turn on passphrase protection (moving the key out of the keychain).
    pub fn enable_passphrase(&self, new_pass: &str) -> Result<()> {
        if new_pass.is_empty() {
            return Err(CoreError::InvalidParams("passphrase is empty".into()));
        }
        self.with_master(|master| self.write_passphrase_wrapping(master, new_pass))?;
        // Best-effort removal of the now-unused keychain copy.
        let _ = self.keystore.delete(MASTER_KEY_LABEL);
        Ok(())
    }

    /// Turn off passphrase protection (storing the key in the keychain).
    pub fn disable_passphrase(&self) -> Result<()> {
        self.with_master(|master| self.keystore.store(MASTER_KEY_LABEL, master.as_bytes()))?;
        self.set_meta(META_PROTECTION, Protection::Keychain.as_str())?;
        self.del_meta(META_WRAPPED_KEY)?;
        self.del_meta(META_KDF_SALT)?;
        self.del_meta(META_KDF_PARAMS)?;
        Ok(())
    }

    /// Change the passphrase (verifies the old one first).
    pub fn change_passphrase(&self, old: &str, new: &str) -> Result<()> {
        if !self.verify_passphrase(old)? {
            return Err(CoreError::BadPassphrase);
        }
        self.enable_passphrase(new)
    }

    // ── accounts ──────────────────────────────────────────────────────────

    fn next_sort_order(conn: &Connection) -> Result<i64> {
        let max: Option<i64> =
            conn.query_row("SELECT MAX(sort_order) FROM accounts", [], |r| r.get(0))?;
        Ok(max.unwrap_or(0) + 1)
    }

    /// Add a new account from raw secret bytes.
    pub fn add_account(&self, new: &NewAccount, secret: &[u8]) -> Result<Account> {
        otp::validate_digits(new.otp.digits)?;
        if new.otp.otp_type == OtpType::Totp {
            otp::validate_period(new.otp.period)?;
        }
        if secret.is_empty() {
            return Err(CoreError::InvalidSecret("secret is empty"));
        }

        self.with_master(|key| {
            let id = new_id();
            let (nonce, ct) = aead::encrypt(key, secret, id.as_bytes())?;
            let fp = dedupe::secret_fingerprint(key, secret, &new.otp);
            let now = otp::unix_now() as i64;

            let conn = self.conn.lock().unwrap();
            let sort_order = Self::next_sort_order(&conn)?;
            conn.execute(
                "INSERT INTO accounts
                   (id, issuer, account_name, otp_type, algorithm, digits, period, counter,
                    secret_nonce, secret_ct, secret_fp, group_id, favorite, sort_order, icon,
                    created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
                params![
                    id,
                    new.issuer,
                    new.account_name,
                    new.otp.otp_type.as_str(),
                    new.otp.algorithm.as_str(),
                    new.otp.digits as i64,
                    new.otp.period as i64,
                    new.otp.counter as i64,
                    nonce,
                    ct,
                    fp,
                    new.group_id,
                    new.favorite as i64,
                    sort_order,
                    new.icon,
                    now,
                    now,
                ],
            )?;

            Ok(Account {
                id,
                issuer: new.issuer.clone(),
                account_name: new.account_name.clone(),
                otp: new.otp,
                group_id: new.group_id.clone(),
                favorite: new.favorite,
                sort_order,
                icon: new.icon.clone(),
                created_at: now,
                updated_at: now,
            })
        })
    }

    fn row_to_account(row: &rusqlite::Row) -> rusqlite::Result<Account> {
        let otp_type = match row.get::<_, String>("otp_type")?.as_str() {
            "hotp" => OtpType::Hotp,
            _ => OtpType::Totp,
        };
        let algorithm = match row.get::<_, String>("algorithm")?.as_str() {
            "SHA256" => Algorithm::Sha256,
            "SHA512" => Algorithm::Sha512,
            _ => Algorithm::Sha1,
        };
        Ok(Account {
            id: row.get("id")?,
            issuer: row.get("issuer")?,
            account_name: row.get("account_name")?,
            otp: OtpConfig {
                otp_type,
                algorithm,
                digits: row.get::<_, i64>("digits")? as u8,
                period: row.get::<_, i64>("period")? as u32,
                counter: row.get::<_, i64>("counter")? as u64,
            },
            group_id: row.get("group_id")?,
            favorite: row.get::<_, i64>("favorite")? != 0,
            sort_order: row.get("sort_order")?,
            icon: row.get("icon")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }

    /// List all accounts (metadata only), ordered for display.
    pub fn list_accounts(&self) -> Result<Vec<Account>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, issuer, account_name, otp_type, algorithm, digits, period, counter,
                    group_id, favorite, sort_order, icon, created_at, updated_at
             FROM accounts
             ORDER BY favorite DESC, sort_order ASC, created_at ASC",
        )?;
        let rows = stmt.query_map([], Self::row_to_account)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    /// Fetch a single account's metadata.
    pub fn get_account(&self, id: &str) -> Result<Account> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT id, issuer, account_name, otp_type, algorithm, digits, period, counter,
                    group_id, favorite, sort_order, icon, created_at, updated_at
             FROM accounts WHERE id = ?1",
            [id],
            Self::row_to_account,
        )
        .optional()?
        .ok_or(CoreError::NotFound)
    }

    /// Apply a patch of mutable fields.
    pub fn update_account(&self, id: &str, patch: &AccountPatch) -> Result<Account> {
        {
            let conn = self.conn.lock().unwrap();
            let now = otp::unix_now() as i64;
            if let Some(issuer) = &patch.issuer {
                conn.execute(
                    "UPDATE accounts SET issuer=?2, updated_at=?3 WHERE id=?1",
                    params![id, issuer, now],
                )?;
            }
            if let Some(name) = &patch.account_name {
                conn.execute(
                    "UPDATE accounts SET account_name=?2, updated_at=?3 WHERE id=?1",
                    params![id, name, now],
                )?;
            }
            if let Some(group) = &patch.group_id {
                conn.execute(
                    "UPDATE accounts SET group_id=?2, updated_at=?3 WHERE id=?1",
                    params![id, group, now],
                )?;
            }
            if let Some(fav) = patch.favorite {
                conn.execute(
                    "UPDATE accounts SET favorite=?2, updated_at=?3 WHERE id=?1",
                    params![id, fav as i64, now],
                )?;
            }
            if let Some(icon) = &patch.icon {
                conn.execute(
                    "UPDATE accounts SET icon=?2, updated_at=?3 WHERE id=?1",
                    params![id, icon, now],
                )?;
            }
            if let Some(sort) = patch.sort_order {
                conn.execute(
                    "UPDATE accounts SET sort_order=?2, updated_at=?3 WHERE id=?1",
                    params![id, sort, now],
                )?;
            }
        }
        self.get_account(id)
    }

    /// Reorder accounts by applying explicit sort orders in one transaction.
    pub fn reorder_accounts(&self, ordered_ids: &[String]) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let tx = conn.unchecked_transaction()?;
        for (i, id) in ordered_ids.iter().enumerate() {
            tx.execute(
                "UPDATE accounts SET sort_order=?2 WHERE id=?1",
                params![id, i as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete an account.
    pub fn delete_account(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let n = conn.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
        if n == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    fn read_secret(&self, key: &SymmetricKey, id: &str) -> Result<(OtpConfig, Zeroizing<Vec<u8>>)> {
        let conn = self.conn.lock().unwrap();
        let (otp, nonce, ct) = conn
            .query_row(
                "SELECT otp_type, algorithm, digits, period, counter, secret_nonce, secret_ct
                 FROM accounts WHERE id = ?1",
                [id],
                |row| {
                    let otp_type = match row.get::<_, String>(0)?.as_str() {
                        "hotp" => OtpType::Hotp,
                        _ => OtpType::Totp,
                    };
                    let algorithm = match row.get::<_, String>(1)?.as_str() {
                        "SHA256" => Algorithm::Sha256,
                        "SHA512" => Algorithm::Sha512,
                        _ => Algorithm::Sha1,
                    };
                    let otp = OtpConfig {
                        otp_type,
                        algorithm,
                        digits: row.get::<_, i64>(2)? as u8,
                        period: row.get::<_, i64>(3)? as u32,
                        counter: row.get::<_, i64>(4)? as u64,
                    };
                    let nonce: Vec<u8> = row.get(5)?;
                    let ct: Vec<u8> = row.get(6)?;
                    Ok((otp, nonce, ct))
                },
            )
            .optional()?
            .ok_or(CoreError::NotFound)?;
        drop(conn);
        let secret = aead::decrypt(key, &nonce, &ct, id.as_bytes())?;
        Ok((otp, secret))
    }

    /// Generate the current code for one account.
    pub fn generate_code(&self, id: &str) -> Result<GeneratedCode> {
        self.with_master(|key| {
            let (otp, secret) = self.read_secret(key, id)?;
            self.build_code(id, &otp, &secret)
        })
    }

    fn build_code(&self, id: &str, otp: &OtpConfig, secret: &[u8]) -> Result<GeneratedCode> {
        match otp.otp_type {
            OtpType::Totp => {
                let now = otp::unix_now();
                let code = otp::totp_at(otp.algorithm, secret, now, otp.period, otp.digits)?;
                let remaining = otp::seconds_remaining(now, otp.period);
                Ok(GeneratedCode {
                    account_id: id.to_string(),
                    code,
                    otp_type: OtpType::Totp,
                    digits: otp.digits,
                    period: otp.period,
                    expires_at: now + u64::from(remaining),
                    seconds_remaining: remaining,
                    counter: 0,
                })
            }
            OtpType::Hotp => {
                let code = otp::hotp(otp.algorithm, secret, otp.counter, otp.digits)?;
                Ok(GeneratedCode {
                    account_id: id.to_string(),
                    code,
                    otp_type: OtpType::Hotp,
                    digits: otp.digits,
                    period: 0,
                    expires_at: 0,
                    seconds_remaining: 0,
                    counter: otp.counter,
                })
            }
        }
    }

    /// Generate current codes for all accounts. HOTP codes reflect the current
    /// stored counter and are **not** advanced here.
    pub fn generate_all(&self) -> Result<Vec<GeneratedCode>> {
        self.with_master(|key| {
            let ids: Vec<String> = {
                let conn = self.conn.lock().unwrap();
                let mut stmt = conn.prepare("SELECT id FROM accounts")?;
                let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            let mut out = Vec::with_capacity(ids.len());
            for id in ids {
                let (otp, secret) = self.read_secret(key, &id)?;
                out.push(self.build_code(&id, &otp, &secret)?);
            }
            Ok(out)
        })
    }

    /// Advance an HOTP counter by one and return the new code. No-op error for TOTP.
    pub fn advance_hotp(&self, id: &str) -> Result<GeneratedCode> {
        self.with_master(|key| {
            let (mut otp, secret) = self.read_secret(key, id)?;
            if otp.otp_type != OtpType::Hotp {
                return Err(CoreError::InvalidParams("account is not HOTP".into()));
            }
            otp.counter = otp.counter.wrapping_add(1);
            {
                let conn = self.conn.lock().unwrap();
                conn.execute(
                    "UPDATE accounts SET counter=?2, updated_at=?3 WHERE id=?1",
                    params![id, otp.counter as i64, otp::unix_now() as i64],
                )?;
            }
            self.build_code(id, &otp, &secret)
        })
    }

    /// Produce an `otpauth://` URI for a single account (deliberate export).
    /// Requires the vault to be unlocked; caller must gate this behind re-auth.
    pub fn export_account_uri(&self, id: &str) -> Result<Zeroizing<String>> {
        self.with_master(|key| {
            let (otp, secret) = self.read_secret(key, id)?;
            let acct = self.get_account(id)?;
            let uri = otp::uri::build(
                otp.otp_type,
                acct.issuer.as_deref(),
                &acct.account_name,
                &secret,
                otp.algorithm,
                otp.digits,
                otp.period,
                otp.counter,
            );
            Ok(Zeroizing::new(uri))
        })
    }

    // ── dedupe support ─────────────────────────────────────────────────────

    /// All stored secret fingerprints (for duplicate detection).
    pub fn existing_fingerprints(&self) -> Result<Vec<Vec<u8>>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT secret_fp FROM accounts WHERE secret_fp IS NOT NULL")?;
        let rows = stmt.query_map([], |r| r.get::<_, Vec<u8>>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Compute the fingerprint of a candidate secret using the unlocked key.
    pub fn fingerprint_for(&self, secret: &[u8], otp: &OtpConfig) -> Result<Vec<u8>> {
        self.with_master(|key| Ok(dedupe::secret_fingerprint(key, secret, otp)))
    }

    // ── groups ──────────────────────────────────────────────────────────────

    pub fn list_groups(&self) -> Result<Vec<Group>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT id, name, sort_order FROM groups ORDER BY sort_order ASC, name ASC")?;
        let rows = stmt.query_map([], |r| {
            Ok(Group {
                id: r.get(0)?,
                name: r.get(1)?,
                sort_order: r.get(2)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn create_group(&self, name: &str) -> Result<Group> {
        let conn = self.conn.lock().unwrap();
        let id = new_id();
        let sort: i64 = conn
            .query_row("SELECT MAX(sort_order) FROM groups", [], |r| {
                r.get::<_, Option<i64>>(0)
            })?
            .unwrap_or(0)
            + 1;
        conn.execute(
            "INSERT INTO groups (id, name, sort_order) VALUES (?1, ?2, ?3)",
            params![id, name, sort],
        )?;
        Ok(Group {
            id,
            name: name.to_string(),
            sort_order: sort,
        })
    }

    pub fn rename_group(&self, id: &str, name: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("UPDATE groups SET name=?2 WHERE id=?1", params![id, name])?;
        Ok(())
    }

    pub fn delete_group(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM groups WHERE id=?1", [id])?;
        Ok(())
    }

    // ── settings (non-sensitive) ─────────────────────────────────────────────

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        Ok(conn
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get::<_, String>(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn all_settings(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    // ── backup export / restore ──────────────────────────────────────────────

    /// Build a portable [`backup::BackupPayload`] of the whole vault (requires
    /// unlock). Secrets are decrypted and Base32-encoded for portability.
    pub fn export_payload(&self) -> Result<backup::BackupPayload> {
        let groups = self.list_groups()?;
        let group_name: HashMap<String, String> = groups
            .iter()
            .map(|g| (g.id.clone(), g.name.clone()))
            .collect();
        let accounts = self.list_accounts()?;

        let mut out_accounts = Vec::with_capacity(accounts.len());
        self.with_master(|key| {
            for a in &accounts {
                let (otp, secret) = self.read_secret(key, &a.id)?;
                out_accounts.push(backup::BackupAccount {
                    issuer: a.issuer.clone(),
                    account_name: a.account_name.clone(),
                    otp_type: otp.otp_type,
                    algorithm: otp.algorithm,
                    digits: otp.digits,
                    period: otp.period,
                    counter: otp.counter,
                    secret: base32::encode_secret(&secret),
                    favorite: a.favorite,
                    group: a
                        .group_id
                        .as_ref()
                        .and_then(|gid| group_name.get(gid).cloned()),
                    icon: a.icon.clone(),
                    sort_order: a.sort_order,
                });
            }
            Ok(())
        })?;

        Ok(backup::BackupPayload {
            format_version: backup::FORMAT_VERSION,
            exported_at: otp::unix_now() as i64,
            accounts: out_accounts,
            groups: groups
                .iter()
                .map(|g| backup::BackupGroup {
                    name: g.name.clone(),
                    sort_order: g.sort_order,
                })
                .collect(),
            settings: self.all_settings()?,
        })
    }

    /// Import accounts, groups from a [`backup::BackupPayload`] (requires unlock).
    /// Groups are remapped/created by name; duplicates are handled per `policy`.
    pub fn import_payload(
        &self,
        payload: &backup::BackupPayload,
        policy: backup::DuplicatePolicy,
    ) -> Result<backup::ImportSummary> {
        let mut summary = backup::ImportSummary::default();

        // Map existing group names → id, creating any missing ones.
        let mut group_ids: HashMap<String, String> = self
            .list_groups()?
            .into_iter()
            .map(|g| (dedupe::normalize_name(&g.name), g.id))
            .collect();
        for g in &payload.groups {
            let norm = dedupe::normalize_name(&g.name);
            if let std::collections::hash_map::Entry::Vacant(e) = group_ids.entry(norm) {
                let created = self.create_group(&g.name)?;
                e.insert(created.id);
                summary.groups_created += 1;
            }
        }

        let mut seen = self.existing_fingerprints()?;
        for a in &payload.accounts {
            let secret = base32::decode_secret(&a.secret)?;
            let otp = OtpConfig {
                otp_type: a.otp_type,
                algorithm: a.algorithm,
                digits: otp::validate_digits(a.digits)?,
                period: a.period,
                counter: a.counter,
            };
            let fp = self.fingerprint_for(&secret, &otp)?;
            let is_dup = seen.iter().any(|e| dedupe::fingerprints_match(e, &fp));
            if is_dup && policy == backup::DuplicatePolicy::Skip {
                summary.skipped += 1;
                continue;
            }
            let group_id = a
                .group
                .as_ref()
                .and_then(|n| group_ids.get(&dedupe::normalize_name(n)).cloned());
            let new = NewAccount {
                issuer: a.issuer.clone(),
                account_name: a.account_name.clone(),
                otp,
                group_id,
                favorite: a.favorite,
                icon: a.icon.clone(),
            };
            self.add_account(&new, &secret)?;
            seen.push(fp);
            summary.imported += 1;
        }

        Ok(summary)
    }
}
