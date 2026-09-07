//! Standards-compliant OTP engine (RFC 4226 HOTP / RFC 6238 TOTP).
//!
//! Built directly on the audited RustCrypto `hmac`, `sha1`, and `sha2` crates.
//! We implement the dynamic-truncation and time-step logic ourselves so the
//! trusted surface is small and verifiable against the published RFC vectors
//! (see the tests in this module and `tests/`).

pub mod base32;
pub mod uri;

use crate::error::{CoreError, Result};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};
use std::time::{SystemTime, UNIX_EPOCH};

/// Hash algorithm used inside the HMAC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum Algorithm {
    #[default]
    Sha1,
    Sha256,
    Sha512,
}

impl Algorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Sha1 => "SHA1",
            Algorithm::Sha256 => "SHA256",
            Algorithm::Sha512 => "SHA512",
        }
    }

    /// Parse the algorithm label from an otpauth URI (case-insensitive).
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "SHA1" | "" => Ok(Algorithm::Sha1),
            "SHA256" => Ok(Algorithm::Sha256),
            "SHA512" => Ok(Algorithm::Sha512),
            other => Err(CoreError::InvalidParams(format!(
                "unsupported algorithm '{other}'"
            ))),
        }
    }
}

/// TOTP (time-based) or HOTP (counter-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OtpType {
    Totp,
    Hotp,
}

impl OtpType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OtpType::Totp => "totp",
            OtpType::Hotp => "hotp",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "totp" => Ok(OtpType::Totp),
            "hotp" => Ok(OtpType::Hotp),
            other => Err(CoreError::InvalidParams(format!(
                "unsupported OTP type '{other}'"
            ))),
        }
    }
}

/// Bounds we accept for digit counts. RFC values are 6–8; we allow a little
/// slack but reject absurd values that would overflow or be unusable.
pub const MIN_DIGITS: u8 = 6;
pub const MAX_DIGITS: u8 = 10;
pub const DEFAULT_DIGITS: u8 = 6;
pub const DEFAULT_PERIOD: u32 = 30;

/// Validate a digit count into the accepted range.
pub fn validate_digits(digits: u8) -> Result<u8> {
    if (MIN_DIGITS..=MAX_DIGITS).contains(&digits) {
        Ok(digits)
    } else {
        Err(CoreError::InvalidParams(format!(
            "digits must be between {MIN_DIGITS} and {MAX_DIGITS}"
        )))
    }
}

/// Validate a TOTP period (seconds).
pub fn validate_period(period: u32) -> Result<u32> {
    if (1..=3600).contains(&period) {
        Ok(period)
    } else {
        Err(CoreError::InvalidParams(
            "period must be between 1 and 3600 seconds".into(),
        ))
    }
}

/// Compute the raw HMAC of an 8-byte big-endian counter under `algorithm`.
fn hmac_counter(algorithm: Algorithm, key: &[u8], counter: u64) -> Result<Vec<u8>> {
    let msg = counter.to_be_bytes();
    // `new_from_slice` accepts arbitrary key lengths for HMAC and never errors
    // in practice; we still surface a crypto error rather than unwrap.
    let mac = match algorithm {
        Algorithm::Sha1 => {
            let mut m = Hmac::<Sha1>::new_from_slice(key).map_err(|_| CoreError::Crypto)?;
            m.update(&msg);
            m.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha256 => {
            let mut m = Hmac::<Sha256>::new_from_slice(key).map_err(|_| CoreError::Crypto)?;
            m.update(&msg);
            m.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha512 => {
            let mut m = Hmac::<Sha512>::new_from_slice(key).map_err(|_| CoreError::Crypto)?;
            m.update(&msg);
            m.finalize().into_bytes().to_vec()
        }
    };
    Ok(mac)
}

/// RFC 4226 dynamic truncation → a `digits`-length decimal string.
fn truncate(hs: &[u8], digits: u8) -> String {
    // Offset is the low nibble of the last byte.
    let offset = (hs[hs.len() - 1] & 0x0f) as usize;
    let bin = ((u32::from(hs[offset]) & 0x7f) << 24)
        | (u32::from(hs[offset + 1]) << 16)
        | (u32::from(hs[offset + 2]) << 8)
        | u32::from(hs[offset + 3]);

    let modulus = 10u64.pow(u32::from(digits));
    let code = u64::from(bin) % modulus;
    format!("{code:0width$}", width = digits as usize)
}

/// Compute an HOTP value for `secret` at `counter`.
///
/// `secret` is the **raw** decoded key material (not Base32 text).
pub fn hotp(algorithm: Algorithm, secret: &[u8], counter: u64, digits: u8) -> Result<String> {
    if secret.is_empty() {
        return Err(CoreError::InvalidSecret("secret is empty"));
    }
    let digits = validate_digits(digits)?;
    let hs = hmac_counter(algorithm, secret, counter)?;
    Ok(truncate(&hs, digits))
}

/// Compute a TOTP value for `secret` at `unix_time` seconds.
pub fn totp_at(
    algorithm: Algorithm,
    secret: &[u8],
    unix_time: u64,
    period: u32,
    digits: u8,
) -> Result<String> {
    let period = validate_period(period)?;
    let counter = unix_time / u64::from(period);
    hotp(algorithm, secret, counter, digits)
}

/// Current wall-clock UNIX time in seconds.
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Compute a TOTP value for `secret` at the current time.
pub fn totp_now(algorithm: Algorithm, secret: &[u8], period: u32, digits: u8) -> Result<String> {
    totp_at(algorithm, secret, unix_now(), period, digits)
}

/// Seconds remaining in the current TOTP window for a given `period`.
pub fn seconds_remaining(unix_time: u64, period: u32) -> u32 {
    if period == 0 {
        return 0;
    }
    let p = u64::from(period);
    (p - (unix_time % p)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 4226 Appendix D — secret = ASCII "12345678901234567890".
    const HOTP_SECRET: &[u8] = b"12345678901234567890";
    const HOTP_EXPECTED: [&str; 10] = [
        "755224", "287082", "359152", "969429", "338314", "254676", "287922", "162583", "399871",
        "520489",
    ];

    #[test]
    fn rfc4226_hotp_vectors() {
        for (counter, expected) in HOTP_EXPECTED.iter().enumerate() {
            let code = hotp(Algorithm::Sha1, HOTP_SECRET, counter as u64, 6).unwrap();
            assert_eq!(&code, expected, "counter {counter}");
        }
    }

    // RFC 6238 Appendix B — per-algorithm seeds (8-digit codes).
    const SEED_SHA1: &[u8] = b"12345678901234567890";
    const SEED_SHA256: &[u8] = b"12345678901234567890123456789012";
    const SEED_SHA512: &[u8] = b"1234567890123456789012345678901234567890123456789012345678901234";

    #[test]
    fn rfc6238_totp_vectors() {
        struct V {
            time: u64,
            sha1: &'static str,
            sha256: &'static str,
            sha512: &'static str,
        }
        let vectors = [
            V {
                time: 59,
                sha1: "94287082",
                sha256: "46119246",
                sha512: "90693936",
            },
            V {
                time: 1111111109,
                sha1: "07081804",
                sha256: "68084774",
                sha512: "25091201",
            },
            V {
                time: 1111111111,
                sha1: "14050471",
                sha256: "67062674",
                sha512: "99943326",
            },
            V {
                time: 1234567890,
                sha1: "89005924",
                sha256: "91819424",
                sha512: "93441116",
            },
            V {
                time: 2000000000,
                sha1: "69279037",
                sha256: "90698825",
                sha512: "38618901",
            },
            V {
                time: 20000000000,
                sha1: "65353130",
                sha256: "77737706",
                sha512: "47863826",
            },
        ];
        for v in vectors {
            assert_eq!(
                totp_at(Algorithm::Sha1, SEED_SHA1, v.time, 30, 8).unwrap(),
                v.sha1,
                "sha1 @ {}",
                v.time
            );
            assert_eq!(
                totp_at(Algorithm::Sha256, SEED_SHA256, v.time, 30, 8).unwrap(),
                v.sha256,
                "sha256 @ {}",
                v.time
            );
            assert_eq!(
                totp_at(Algorithm::Sha512, SEED_SHA512, v.time, 30, 8).unwrap(),
                v.sha512,
                "sha512 @ {}",
                v.time
            );
        }
    }

    #[test]
    fn six_and_eight_digits() {
        // The 6-digit form is the low 6 digits of the 8-digit form.
        let eight = totp_at(Algorithm::Sha1, SEED_SHA1, 59, 30, 8).unwrap();
        let six = totp_at(Algorithm::Sha1, SEED_SHA1, 59, 30, 6).unwrap();
        assert_eq!(eight, "94287082");
        assert_eq!(six, "287082");
    }

    #[test]
    fn unusual_period() {
        // Sanity: a 60s period halves the counter progression vs 30s.
        let a = totp_at(Algorithm::Sha1, SEED_SHA1, 60, 60, 6).unwrap();
        let b = hotp(Algorithm::Sha1, SEED_SHA1, 1, 6).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn seconds_remaining_math() {
        assert_eq!(seconds_remaining(0, 30), 30);
        assert_eq!(seconds_remaining(1, 30), 29);
        assert_eq!(seconds_remaining(29, 30), 1);
        assert_eq!(seconds_remaining(30, 30), 30);
    }

    #[test]
    fn rejects_bad_params() {
        assert!(hotp(Algorithm::Sha1, HOTP_SECRET, 0, 5).is_err());
        assert!(hotp(Algorithm::Sha1, HOTP_SECRET, 0, 11).is_err());
        assert!(hotp(Algorithm::Sha1, b"", 0, 6).is_err());
        assert!(validate_period(0).is_err());
    }
}
