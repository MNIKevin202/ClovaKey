//! Authenticated encryption using XChaCha20-Poly1305.
//!
//! The 192-bit (24-byte) nonce means random nonces have a negligible collision
//! probability, so we never need a nonce counter or state. Additional
//! authenticated data (AAD) binds a ciphertext to its context (e.g. an account
//! id), preventing ciphertexts from being swapped between records.

use crate::crypto::{rng, SymmetricKey};
use crate::error::{CoreError, Result};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

/// Nonce length for XChaCha20-Poly1305.
pub const NONCE_LEN: usize = 24;

/// Encrypt `plaintext` with a fresh random nonce.
///
/// Returns `(nonce, ciphertext_with_tag)`. Callers store both.
pub fn encrypt(key: &SymmetricKey, plaintext: &[u8], aad: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
    let nonce_bytes = rng::random_array::<NONCE_LEN>()?;
    let nonce = XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(
            nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| CoreError::Crypto)?;
    Ok((nonce_bytes.to_vec(), ciphertext))
}

/// Decrypt and verify a ciphertext produced by [`encrypt`].
///
/// Returns a zeroizing buffer. Any tampering, wrong key, or wrong AAD fails.
pub fn decrypt(
    key: &SymmetricKey,
    nonce: &[u8],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    if nonce.len() != NONCE_LEN {
        return Err(CoreError::Crypto);
    }
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key.as_bytes()));
    let nonce = XNonce::from_slice(nonce);
    let plaintext = cipher
        .decrypt(
            nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| CoreError::Crypto)?;
    Ok(Zeroizing::new(plaintext))
}

/// Encrypt into a single self-contained blob: `nonce || ciphertext`.
///
/// Convenient for backup payloads where nonce and ciphertext travel together.
pub fn seal(key: &SymmetricKey, plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
    let (nonce, ciphertext) = encrypt(key, plaintext, aad)?;
    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Inverse of [`seal`].
pub fn open(key: &SymmetricKey, blob: &[u8], aad: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    if blob.len() < NONCE_LEN {
        return Err(CoreError::Crypto);
    }
    let (nonce, ciphertext) = blob.split_at(NONCE_LEN);
    decrypt(key, nonce, ciphertext, aad)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SymmetricKey {
        SymmetricKey::from_bytes([7u8; 32])
    }

    #[test]
    fn round_trip() {
        let (nonce, ct) = encrypt(&key(), b"top secret seed", b"account:1").unwrap();
        let pt = decrypt(&key(), &nonce, &ct, b"account:1").unwrap();
        assert_eq!(&pt[..], b"top secret seed");
    }

    #[test]
    fn nonces_are_unique() {
        let (n1, _) = encrypt(&key(), b"x", b"").unwrap();
        let (n2, _) = encrypt(&key(), b"x", b"").unwrap();
        assert_ne!(n1, n2);
    }

    #[test]
    fn tamper_is_detected() {
        let (nonce, mut ct) = encrypt(&key(), b"data", b"aad").unwrap();
        ct[0] ^= 0xff;
        assert!(decrypt(&key(), &nonce, &ct, b"aad").is_err());
    }

    #[test]
    fn wrong_aad_fails() {
        let (nonce, ct) = encrypt(&key(), b"data", b"account:1").unwrap();
        assert!(decrypt(&key(), &nonce, &ct, b"account:2").is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let (nonce, ct) = encrypt(&key(), b"data", b"aad").unwrap();
        let other = SymmetricKey::from_bytes([9u8; 32]);
        assert!(decrypt(&other, &nonce, &ct, b"aad").is_err());
    }

    #[test]
    fn seal_open_round_trip() {
        let blob = seal(&key(), b"portable", b"backup").unwrap();
        assert!(blob.len() > NONCE_LEN);
        let out = open(&key(), &blob, b"backup").unwrap();
        assert_eq!(&out[..], b"portable");
        assert!(open(&key(), &blob, b"wrong").is_err());
    }
}
