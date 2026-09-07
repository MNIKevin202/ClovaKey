//! Argon2id password-based key derivation.
//!
//! Used to turn a user passphrase into a 256-bit key for (a) portable backups
//! and (b) the optional passphrase that wraps the vault master key. Parameters
//! are stored alongside the ciphertext so they can be tuned in future versions
//! without breaking old data.

use crate::crypto::{SymmetricKey, KEY_LEN};
use crate::error::{CoreError, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};

/// Salt length in bytes.
pub const SALT_LEN: usize = 16;

/// Tunable Argon2id parameters, persisted with derived data.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParams {
    /// Memory cost in KiB.
    pub memory_kib: u32,
    /// Time cost (iterations / passes).
    pub iterations: u32,
    /// Degree of parallelism (lanes).
    pub parallelism: u32,
}

impl Default for KdfParams {
    fn default() -> Self {
        // ~64 MiB, 3 passes, single lane. Comfortably above OWASP minimums for
        // an interactive desktop unlock while keeping restore times reasonable.
        KdfParams {
            memory_kib: 64 * 1024,
            iterations: 3,
            parallelism: 1,
        }
    }
}

impl KdfParams {
    fn to_argon2(self) -> Result<Argon2<'static>> {
        let params = Params::new(
            self.memory_kib,
            self.iterations,
            self.parallelism,
            Some(KEY_LEN),
        )
        .map_err(|_| CoreError::InvalidParams("invalid KDF parameters".into()))?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }
}

/// Generate a fresh random salt.
pub fn generate_salt() -> Result<[u8; SALT_LEN]> {
    crate::crypto::rng::random_array::<SALT_LEN>()
}

/// Derive a 256-bit key from `password` and `salt` using Argon2id.
pub fn derive_key(password: &[u8], salt: &[u8], params: &KdfParams) -> Result<SymmetricKey> {
    if salt.len() < 8 {
        return Err(CoreError::InvalidParams("salt is too short".into()));
    }
    let argon = params.to_argon2()?;
    let mut key = [0u8; KEY_LEN];
    argon
        .hash_password_into(password, salt, &mut key)
        .map_err(|_| CoreError::Crypto)?;
    let out = SymmetricKey::from_bytes(key);
    key.iter_mut().for_each(|b| *b = 0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Use tiny params in tests so they run fast; production uses the defaults.
    fn fast() -> KdfParams {
        KdfParams {
            memory_kib: 8,
            iterations: 1,
            parallelism: 1,
        }
    }

    #[test]
    fn deterministic_for_same_inputs() {
        let salt = [1u8; SALT_LEN];
        let a = derive_key(b"correct horse", &salt, &fast()).unwrap();
        let b = derive_key(b"correct horse", &salt, &fast()).unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn differs_by_password_and_salt() {
        let salt1 = [1u8; SALT_LEN];
        let salt2 = [2u8; SALT_LEN];
        let a = derive_key(b"password", &salt1, &fast()).unwrap();
        let b = derive_key(b"password", &salt2, &fast()).unwrap();
        let c = derive_key(b"different", &salt1, &fast()).unwrap();
        assert_ne!(a.as_bytes(), b.as_bytes());
        assert_ne!(a.as_bytes(), c.as_bytes());
    }

    #[test]
    fn rejects_short_salt() {
        assert!(derive_key(b"x", &[0u8; 4], &fast()).is_err());
    }
}
