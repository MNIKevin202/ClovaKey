//! Central error type for the ClovaKey core.
//!
//! Errors are deliberately written to be safe to surface to users and logs:
//! they never embed secret material (OTP seeds, keys, passphrases). Parsers
//! that touch untrusted input return descriptive-but-redacted variants.

use std::fmt;

/// Result alias used throughout the core.
pub type Result<T> = std::result::Result<T, CoreError>;

/// All errors produced by the ClovaKey core.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid OTP secret: {0}")]
    InvalidSecret(&'static str),

    #[error("invalid OTP parameters: {0}")]
    InvalidParams(String),

    #[error("could not parse otpauth URI: {0}")]
    InvalidUri(String),

    #[error("could not read this authenticator export")]
    MigrationDecode(&'static str),

    #[error("QR code could not be read")]
    QrDecode(&'static str),

    #[error("the vault is locked")]
    VaultLocked,

    #[error("the vault has not been set up yet")]
    VaultUninitialized,

    #[error("incorrect passphrase")]
    BadPassphrase,

    #[error("cryptographic operation failed")]
    Crypto,

    #[error("secure key storage is unavailable: {0}")]
    KeyStore(String),

    #[error("this backup file is not a valid ClovaKey backup")]
    BackupFormat(&'static str),

    #[error("unsupported backup version: {0}")]
    BackupVersion(u32),

    #[error("account not found")]
    NotFound,

    #[error("database error: {0}")]
    Db(String),

    #[error("randomness source failed")]
    Random,

    #[error("{0}")]
    Message(String),
}

impl CoreError {
    /// A machine-readable, non-sensitive code for the frontend to branch on.
    pub fn code(&self) -> &'static str {
        match self {
            CoreError::InvalidSecret(_) => "invalid_secret",
            CoreError::InvalidParams(_) => "invalid_params",
            CoreError::InvalidUri(_) => "invalid_uri",
            CoreError::MigrationDecode(_) => "migration_decode",
            CoreError::QrDecode(_) => "qr_decode",
            CoreError::VaultLocked => "vault_locked",
            CoreError::VaultUninitialized => "vault_uninitialized",
            CoreError::BadPassphrase => "bad_passphrase",
            CoreError::Crypto => "crypto",
            CoreError::KeyStore(_) => "key_store",
            CoreError::BackupFormat(_) => "backup_format",
            CoreError::BackupVersion(_) => "backup_version",
            CoreError::NotFound => "not_found",
            CoreError::Db(_) => "db",
            CoreError::Random => "random",
            CoreError::Message(_) => "error",
        }
    }
}

// Map foreign errors without leaking internals that might carry sensitive data.
impl From<rusqlite::Error> for CoreError {
    fn from(e: rusqlite::Error) -> Self {
        CoreError::Db(e.to_string())
    }
}

/// Wrapper so we can `?` on `getrandom` failures.
#[derive(Debug)]
pub struct RandomError;

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "randomness source failed")
    }
}
