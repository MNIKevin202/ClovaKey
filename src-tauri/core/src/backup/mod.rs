//! The portable, encrypted `.clovakey` backup format.
//!
//! A backup is a small JSON *envelope* whose payload (accounts with their
//! secrets, groups, and settings) is encrypted under a key derived from a
//! user-chosen passphrase — independent of the local machine key — so it can be
//! restored on another computer. Argon2id derives the key; XChaCha20-Poly1305
//! provides authenticated encryption. The format is versioned from day one.
//!
//! The envelope header (format, version, KDF parameters) is bound as AAD, so
//! tampering with it fails authentication. KDF parameters read from an untrusted
//! backup are clamped to safe bounds before use (defence against a malicious
//! header that would otherwise force huge memory allocation).

use crate::crypto::{aead, kdf, kdf::KdfParams};
use crate::error::{CoreError, Result};
use crate::otp::{Algorithm, OtpType};
use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Current backup format version.
pub const FORMAT_VERSION: u32 = 1;

const FORMAT_TAG: &str = "clovakey-backup";
const CIPHER_TAG: &str = "xchacha20poly1305";
const KDF_TAG: &str = "argon2id";

/// Reject backup files larger than this before parsing.
const MAX_BACKUP_BYTES: usize = 64 * 1024 * 1024;

/// One account inside a backup. The secret is Base32 (the portable standard).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupAccount {
    pub issuer: Option<String>,
    pub account_name: String,
    #[serde(rename = "type")]
    pub otp_type: OtpType,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub period: u32,
    pub counter: u64,
    /// Base32-encoded secret.
    pub secret: String,
    #[serde(default)]
    pub favorite: bool,
    /// Group name (remapped to the target vault's group id on restore).
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub sort_order: i64,
}

/// One group inside a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupGroup {
    pub name: String,
    #[serde(default)]
    pub sort_order: i64,
}

/// The decrypted inner contents of a backup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPayload {
    pub format_version: u32,
    pub exported_at: i64,
    pub accounts: Vec<BackupAccount>,
    #[serde(default)]
    pub groups: Vec<BackupGroup>,
    #[serde(default)]
    pub settings: Vec<(String, String)>,
}

impl Drop for BackupPayload {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        for a in &mut self.accounts {
            a.secret.zeroize();
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KdfHeader {
    algorithm: String,
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
    salt: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    format: String,
    format_version: u32,
    kdf: KdfHeader,
    cipher: String,
    nonce: String,
    ciphertext: String,
}

/// Additional authenticated data binding the header to the ciphertext.
fn aad(version: u32, params: &KdfParams) -> String {
    format!(
        "{FORMAT_TAG}|v{version}|{KDF_TAG}|m={}|t={}|p={}|{CIPHER_TAG}",
        params.memory_kib, params.iterations, params.parallelism
    )
}

/// Clamp/validate KDF params from an untrusted backup header.
fn validate_params(p: &KdfParams) -> Result<()> {
    let ok = (8..=2 * 1024 * 1024).contains(&p.memory_kib) // up to 2 GiB
        && (1..=20).contains(&p.iterations)
        && (1..=16).contains(&p.parallelism);
    if ok {
        Ok(())
    } else {
        Err(CoreError::BackupFormat("backup KDF parameters are invalid"))
    }
}

/// Encrypt a payload into `.clovakey` bytes using `password`.
pub fn seal_payload(payload: &BackupPayload, password: &str) -> Result<Vec<u8>> {
    if password.is_empty() {
        return Err(CoreError::InvalidParams(
            "backup passphrase is empty".into(),
        ));
    }
    let salt = kdf::generate_salt()?;
    let params = KdfParams::default();
    let key = kdf::derive_key(password.as_bytes(), &salt, &params)?;

    let plaintext = Zeroizing::new(
        serde_json::to_vec(payload)
            .map_err(|_| CoreError::Message("serialization failed".into()))?,
    );
    let (nonce, ciphertext) =
        aead::encrypt(&key, &plaintext, aad(FORMAT_VERSION, &params).as_bytes())?;

    let envelope = Envelope {
        format: FORMAT_TAG.to_string(),
        format_version: FORMAT_VERSION,
        kdf: KdfHeader {
            algorithm: KDF_TAG.to_string(),
            memory_kib: params.memory_kib,
            iterations: params.iterations,
            parallelism: params.parallelism,
            salt: BASE64.encode(&salt),
        },
        cipher: CIPHER_TAG.to_string(),
        nonce: BASE64.encode(&nonce),
        ciphertext: BASE64.encode(&ciphertext),
    };

    serde_json::to_vec_pretty(&envelope)
        .map_err(|_| CoreError::Message("serialization failed".into()))
}

/// Decrypt `.clovakey` bytes with `password`, returning the payload.
///
/// A wrong password fails as [`CoreError::BadPassphrase`] (authenticated
/// decryption). Structural problems fail as [`CoreError::BackupFormat`].
pub fn open_payload(bytes: &[u8], password: &str) -> Result<BackupPayload> {
    if bytes.len() > MAX_BACKUP_BYTES {
        return Err(CoreError::BackupFormat("backup file is too large"));
    }
    let env: Envelope = serde_json::from_slice(bytes)
        .map_err(|_| CoreError::BackupFormat("not a ClovaKey backup"))?;

    if env.format != FORMAT_TAG {
        return Err(CoreError::BackupFormat("not a ClovaKey backup"));
    }
    if env.format_version != FORMAT_VERSION {
        return Err(CoreError::BackupVersion(env.format_version));
    }
    if env.kdf.algorithm != KDF_TAG || env.cipher != CIPHER_TAG {
        return Err(CoreError::BackupFormat("unsupported backup algorithms"));
    }

    let params = KdfParams {
        memory_kib: env.kdf.memory_kib,
        iterations: env.kdf.iterations,
        parallelism: env.kdf.parallelism,
    };
    validate_params(&params)?;

    let salt = BASE64
        .decode(env.kdf.salt.as_bytes())
        .map_err(|_| CoreError::BackupFormat("corrupt salt"))?;
    let nonce = BASE64
        .decode(env.nonce.as_bytes())
        .map_err(|_| CoreError::BackupFormat("corrupt nonce"))?;
    let ciphertext = BASE64
        .decode(env.ciphertext.as_bytes())
        .map_err(|_| CoreError::BackupFormat("corrupt ciphertext"))?;

    let key = kdf::derive_key(password.as_bytes(), &salt, &params)?;
    let plaintext = aead::decrypt(
        &key,
        &nonce,
        &ciphertext,
        aad(env.format_version, &params).as_bytes(),
    )
    .map_err(|_| CoreError::BadPassphrase)?;

    let payload: BackupPayload = serde_json::from_slice(&plaintext)
        .map_err(|_| CoreError::BackupFormat("corrupt backup contents"))?;
    Ok(payload)
}

/// How to handle records that already exist in the target vault on import.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicatePolicy {
    Skip,
    ImportAll,
}

/// Summary of an import/restore operation.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub imported: usize,
    pub skipped: usize,
    pub groups_created: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> BackupPayload {
        BackupPayload {
            format_version: FORMAT_VERSION,
            exported_at: 1_700_000_000,
            accounts: vec![BackupAccount {
                issuer: Some("GitHub".into()),
                account_name: "kevin@example.com".into(),
                otp_type: OtpType::Totp,
                algorithm: Algorithm::Sha1,
                digits: 6,
                period: 30,
                counter: 0,
                secret: "JBSWY3DPEHPK3PXP".into(),
                favorite: true,
                group: Some("Work".into()),
                icon: None,
                sort_order: 0,
            }],
            groups: vec![BackupGroup {
                name: "Work".into(),
                sort_order: 0,
            }],
            settings: vec![("theme".into(), "dark".into())],
        }
    }

    #[test]
    fn seal_open_round_trip() {
        let bytes = seal_payload(&sample_payload(), "hunter2-correct-horse").unwrap();
        // The file is JSON and must not contain the plaintext secret.
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(text.contains("clovakey-backup"));
        assert!(!text.contains("JBSWY3DPEHPK3PXP"));

        let restored = open_payload(&bytes, "hunter2-correct-horse").unwrap();
        assert_eq!(restored.accounts.len(), 1);
        assert_eq!(restored.accounts[0].secret, "JBSWY3DPEHPK3PXP");
        assert_eq!(restored.accounts[0].issuer.as_deref(), Some("GitHub"));
        assert_eq!(restored.groups[0].name, "Work");
    }

    #[test]
    fn wrong_password_fails() {
        let bytes = seal_payload(&sample_payload(), "correct").unwrap();
        let err = open_payload(&bytes, "incorrect").unwrap_err();
        assert!(matches!(err, CoreError::BadPassphrase));
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let bytes = seal_payload(&sample_payload(), "pw").unwrap();
        let mut env: Envelope = serde_json::from_slice(&bytes).unwrap();
        // Flip a byte in the ciphertext.
        let mut ct = BASE64.decode(env.ciphertext.as_bytes()).unwrap();
        ct[0] ^= 0xff;
        env.ciphertext = BASE64.encode(&ct);
        let tampered = serde_json::to_vec(&env).unwrap();
        assert!(open_payload(&tampered, "pw").is_err());
    }

    #[test]
    fn tampered_header_fails_auth() {
        let bytes = seal_payload(&sample_payload(), "pw").unwrap();
        let mut env: Envelope = serde_json::from_slice(&bytes).unwrap();
        // Changing the KDF params (part of the AAD) must break authentication.
        env.kdf.iterations += 1;
        let tampered = serde_json::to_vec(&env).unwrap();
        assert!(open_payload(&tampered, "pw").is_err());
    }

    #[test]
    fn rejects_bad_format_and_version() {
        assert!(open_payload(b"{}", "pw").is_err());
        assert!(open_payload(b"not json", "pw").is_err());

        let mut env: Envelope =
            serde_json::from_slice(&seal_payload(&sample_payload(), "pw").unwrap()).unwrap();
        env.format_version = 99;
        let bytes = serde_json::to_vec(&env).unwrap();
        assert!(matches!(
            open_payload(&bytes, "pw"),
            Err(CoreError::BackupVersion(99))
        ));
    }

    #[test]
    fn rejects_malicious_kdf_params() {
        let mut env: Envelope =
            serde_json::from_slice(&seal_payload(&sample_payload(), "pw").unwrap()).unwrap();
        env.kdf.memory_kib = u32::MAX; // would try to allocate ~4 TiB
        let bytes = serde_json::to_vec(&env).unwrap();
        assert!(matches!(
            open_payload(&bytes, "pw"),
            Err(CoreError::BackupFormat(_))
        ));
    }
}
