//! Tolerant RFC 4648 Base32 handling for OTP secrets.
//!
//! Real-world otpauth secrets are Base32 (uppercase, no padding) but are often
//! pasted with spaces, lowercase letters, or trailing `=` padding. We normalise
//! defensively and then decode strictly, so a malformed secret is *rejected*
//! rather than silently reinterpreted.

use crate::error::{CoreError, Result};
use data_encoding::BASE32_NOPAD;
use zeroize::Zeroizing;

/// Decode a user-supplied Base32 secret into raw key bytes.
///
/// Accepts spaces and lowercase, tolerates padding, and rejects anything that is
/// not valid Base32. The returned buffer zeroizes itself on drop.
pub fn decode_secret(input: &str) -> Result<Zeroizing<Vec<u8>>> {
    let normalized: String = input
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| c.to_ascii_uppercase())
        .collect();

    // Strip any RFC 4648 padding — we decode with the no-pad alphabet.
    let trimmed = normalized.trim_end_matches('=');

    if trimmed.is_empty() {
        return Err(CoreError::InvalidSecret("secret is empty"));
    }

    let bytes = BASE32_NOPAD
        .decode(trimmed.as_bytes())
        .map_err(|_| CoreError::InvalidSecret("not valid Base32"))?;

    if bytes.is_empty() {
        return Err(CoreError::InvalidSecret("secret decoded to zero bytes"));
    }

    Ok(Zeroizing::new(bytes))
}

/// Encode raw key bytes as an (unpadded, uppercase) Base32 string.
///
/// Used only for deliberate export/reveal flows — never for logging.
pub fn encode_secret(bytes: &[u8]) -> String {
    BASE32_NOPAD.encode(bytes)
}

/// Validate that a string is a decodable Base32 secret without retaining the
/// decoded bytes (used for form validation).
pub fn is_valid_secret(input: &str) -> bool {
    decode_secret(input).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_rfc4648_example() {
        // "12345678901234567890" is the RFC 4226/6238 test seed.
        let secret = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";
        let bytes = decode_secret(secret).unwrap();
        assert_eq!(&bytes[..], b"12345678901234567890");
    }

    #[test]
    fn tolerates_spaces_and_lowercase() {
        let a = decode_secret("gezd gnbv gy3t qojq").unwrap();
        let b = decode_secret("GEZDGNBVGY3TQOJQ").unwrap();
        assert_eq!(&a[..], &b[..]);
    }

    #[test]
    fn tolerates_padding() {
        // JBSWY3DPEHPK3PXP with padding variants must decode identically.
        let a = decode_secret("JBSWY3DPEHPK3PXP").unwrap();
        let b = decode_secret("JBSWY3DPEHPK3PXP======").unwrap();
        assert_eq!(&a[..], &b[..]);
    }

    #[test]
    fn rejects_invalid_characters() {
        // '1' and '8' are not in the Base32 alphabet.
        assert!(decode_secret("JBSWY3DP1818").is_err());
        assert!(decode_secret("").is_err());
        assert!(decode_secret("   ").is_err());
    }

    #[test]
    fn round_trips() {
        let raw = b"hello world secret!!";
        let encoded = encode_secret(raw);
        let decoded = decode_secret(&encoded).unwrap();
        assert_eq!(&decoded[..], raw);
    }
}
