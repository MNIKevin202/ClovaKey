//! Duplicate detection for imports.
//!
//! We identify "the same credential" via a **keyed** fingerprint: an HMAC-SHA256
//! over the secret plus its OTP configuration, keyed with the vault master key.
//! Because it is keyed, the stored fingerprint is meaningless to anyone without
//! the unlocked vault, cannot be precomputed against a dictionary, and — being a
//! one-way MAC — never exposes the secret itself.

use crate::crypto::SymmetricKey;
use crate::model::OtpConfig;
use crate::otp::OtpType;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use subtle::ConstantTimeEq;

const DOMAIN: &[u8] = b"clovakey-fingerprint-v1";

/// Compute the keyed fingerprint of a secret + its OTP configuration.
pub fn secret_fingerprint(key: &SymmetricKey, secret: &[u8], otp: &OtpConfig) -> Vec<u8> {
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(DOMAIN);
    // Length-prefix the secret so it can't be confused with the config bytes.
    mac.update(&(secret.len() as u64).to_be_bytes());
    mac.update(secret);
    mac.update(otp.otp_type.as_str().as_bytes());
    mac.update(&[0]);
    mac.update(otp.algorithm.as_str().as_bytes());
    mac.update(&[0]);
    mac.update(&[otp.digits]);
    mac.update(&otp.period.to_be_bytes());
    // Counter only distinguishes HOTP credentials.
    if otp.otp_type == OtpType::Hotp {
        mac.update(&otp.counter.to_be_bytes());
    }
    mac.finalize().into_bytes().to_vec()
}

/// Constant-time comparison of two fingerprints.
pub fn fingerprints_match(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.ct_eq(b).into()
}

/// Normalise a name for a secondary (name-based) duplicate heuristic:
/// case-insensitive, trimmed, internal whitespace collapsed.
pub fn normalize_name(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::otp::Algorithm;

    fn cfg() -> OtpConfig {
        OtpConfig::default()
    }

    #[test]
    fn same_inputs_same_fingerprint() {
        let key = SymmetricKey::from_bytes([3u8; 32]);
        let a = secret_fingerprint(&key, b"seed-bytes", &cfg());
        let b = secret_fingerprint(&key, b"seed-bytes", &cfg());
        assert!(fingerprints_match(&a, &b));
    }

    #[test]
    fn differs_by_secret_and_config() {
        let key = SymmetricKey::from_bytes([3u8; 32]);
        let base = secret_fingerprint(&key, b"seed-bytes", &cfg());
        assert!(!fingerprints_match(
            &base,
            &secret_fingerprint(&key, b"other-seed", &cfg())
        ));
        let mut other = cfg();
        other.algorithm = Algorithm::Sha256;
        assert!(!fingerprints_match(
            &base,
            &secret_fingerprint(&key, b"seed-bytes", &other)
        ));
    }

    #[test]
    fn differs_by_key() {
        let a = secret_fingerprint(&SymmetricKey::from_bytes([1u8; 32]), b"seed", &cfg());
        let b = secret_fingerprint(&SymmetricKey::from_bytes([2u8; 32]), b"seed", &cfg());
        assert!(!fingerprints_match(&a, &b));
    }

    #[test]
    fn name_normalization() {
        assert_eq!(normalize_name("  GitHub  "), "github");
        assert_eq!(normalize_name("My   Bank"), "my bank");
    }
}
