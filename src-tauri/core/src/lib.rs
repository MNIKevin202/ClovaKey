//! # ClovaKey core
//!
//! The security- and standards-sensitive heart of ClovaKey, kept free of any
//! Tauri/UI dependency so it can be exhaustively unit-tested in isolation.
//!
//! Modules:
//! - [`otp`] — RFC 4226/6238 HOTP & TOTP plus the `otpauth://` URI parser.
//! - [`crypto`] — audited AEAD, Argon2id KDF, and CSPRNG wrappers.
//! - [`keystore`] — OS-backed secure master-key storage.
//! - [`storage`] — the encrypted SQLite vault.
//! - [`model`] — non-sensitive account/group metadata.
//! - [`dedupe`] — keyed duplicate-detection fingerprints.
//! - [`error`] — the shared, secret-free error type.

pub mod backup;
pub mod crypto;
pub mod dedupe;
pub mod error;
pub mod import;
pub mod keystore;
pub mod model;
pub mod otp;
pub mod qr;
pub mod storage;

pub use error::{CoreError, Result};
pub use storage::Vault;
