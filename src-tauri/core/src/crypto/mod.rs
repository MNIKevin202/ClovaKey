//! Cryptographic building blocks for ClovaKey.
//!
//! We never invent primitives. Everything here is a thin, well-documented
//! wrapper over audited RustCrypto crates:
//! - [`aead`] — XChaCha20-Poly1305 authenticated encryption (192-bit nonces,
//!   so random nonces are safe without a counter).
//! - [`kdf`] — Argon2id password-based key derivation for backups & passphrase.
//! - [`rng`] — OS CSPRNG access via `getrandom`.

pub mod aead;
pub mod kdf;
pub mod rng;

use zeroize::{Zeroize, ZeroizeOnDrop};

/// Length of a symmetric key in bytes (256-bit).
pub const KEY_LEN: usize = 32;

/// A 256-bit symmetric key that is zeroized when dropped.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SymmetricKey(pub(crate) [u8; KEY_LEN]);

impl SymmetricKey {
    /// Wrap raw key bytes.
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        SymmetricKey(bytes)
    }

    /// Generate a fresh random key from the OS CSPRNG.
    pub fn generate() -> crate::Result<Self> {
        Ok(SymmetricKey(rng::random_array::<KEY_LEN>()?))
    }

    /// Copy a key out of a byte slice, zeroizing the temporary buffer.
    pub(crate) fn from_slice(bytes: &[u8]) -> crate::Result<Self> {
        if bytes.len() != KEY_LEN {
            return Err(crate::CoreError::Crypto);
        }
        let mut arr = [0u8; KEY_LEN];
        arr.copy_from_slice(bytes);
        let key = SymmetricKey(arr);
        arr.zeroize();
        Ok(key)
    }

    pub(crate) fn as_bytes(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl std::fmt::Debug for SymmetricKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print key material.
        f.write_str("SymmetricKey(<redacted>)")
    }
}
