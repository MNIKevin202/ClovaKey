//! Parser and builder for standard `otpauth://` URIs (Key Uri Format).
//!
//! We parse by hand rather than pulling a general URL crate: the format is
//! small and well understood, and a bounded parser lets us reject malformed
//! input crisply instead of silently reinterpreting it. Secrets are returned as
//! self-zeroizing raw bytes.

use super::{Algorithm, OtpType, DEFAULT_DIGITS, DEFAULT_PERIOD};
use crate::error::{CoreError, Result};
use zeroize::Zeroizing;

/// A fully parsed otpauth descriptor with the secret decoded to raw bytes.
pub struct ParsedOtp {
    pub otp_type: OtpType,
    pub issuer: Option<String>,
    pub account: String,
    pub secret: Zeroizing<Vec<u8>>,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub period: u32,
    pub counter: u64,
}

/// Guard against pathological input (e.g. a giant pasted blob).
const MAX_URI_LEN: usize = 8 * 1024;

/// Percent-decode a component. When `plus_is_space` is set, `+` decodes to a
/// space (form-encoding semantics used in query values).
fn percent_decode(input: &str, plus_is_space: bool) -> Result<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(CoreError::InvalidUri("truncated percent-escape".into()));
                }
                let hi = (bytes[i + 1] as char)
                    .to_digit(16)
                    .ok_or_else(|| CoreError::InvalidUri("bad percent-escape".into()))?;
                let lo = (bytes[i + 2] as char)
                    .to_digit(16)
                    .ok_or_else(|| CoreError::InvalidUri("bad percent-escape".into()))?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| CoreError::InvalidUri("invalid UTF-8 in URI".into()))
}

/// Percent-encode a component for building a URI (encodes everything that is
/// not an RFC 3986 unreserved character).
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Parse an `otpauth://` URI into a validated [`ParsedOtp`].
pub fn parse(uri: &str) -> Result<ParsedOtp> {
    let uri = uri.trim();
    if uri.len() > MAX_URI_LEN {
        return Err(CoreError::InvalidUri("URI is unreasonably long".into()));
    }

    // Case-insensitive scheme match.
    let rest = uri
        .strip_prefix("otpauth://")
        .or_else(|| uri.strip_prefix("OTPAUTH://"))
        .or_else(|| {
            let lower = uri.get(..10)?.to_ascii_lowercase();
            if lower == "otpauth://" {
                uri.get(10..)
            } else {
                None
            }
        })
        .ok_or_else(|| CoreError::InvalidUri("not an otpauth:// URI".into()))?;

    // Split "<type>/<label>?<query>".
    let (type_and_label, query) = match rest.split_once('?') {
        Some((a, b)) => (a, b),
        None => (rest, ""),
    };

    let (type_str, label) = type_and_label
        .split_once('/')
        .ok_or_else(|| CoreError::InvalidUri("missing account label".into()))?;

    let otp_type = OtpType::parse(type_str)?;

    // Decode label → optional "issuer:account".
    let label = percent_decode(label, false)?;
    let (label_issuer, account) = match label.split_once(':') {
        Some((iss, acc)) => (Some(iss.trim().to_string()), acc.trim().to_string()),
        None => (None, label.trim().to_string()),
    };

    // Parse query parameters.
    let mut secret_b32: Option<String> = None;
    let mut issuer_param: Option<String> = None;
    let mut algorithm = Algorithm::Sha1;
    let mut digits = DEFAULT_DIGITS;
    let mut period = DEFAULT_PERIOD;
    let mut counter: Option<u64> = None;

    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value, true)?;
        match key.to_ascii_lowercase().as_str() {
            "secret" => secret_b32 = Some(value),
            "issuer" => {
                if !value.trim().is_empty() {
                    issuer_param = Some(value.trim().to_string());
                }
            }
            "algorithm" => algorithm = Algorithm::parse(&value)?,
            "digits" => {
                digits = value
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| CoreError::InvalidParams("digits is not a number".into()))?;
            }
            "period" => {
                period = value
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| CoreError::InvalidParams("period is not a number".into()))?;
            }
            "counter" => {
                counter = Some(
                    value
                        .trim()
                        .parse::<u64>()
                        .map_err(|_| CoreError::InvalidParams("counter is not a number".into()))?,
                );
            }
            _ => { /* ignore unknown params (forward-compatible) */ }
        }
    }

    let secret_b32 = secret_b32.ok_or(CoreError::InvalidSecret("URI has no secret"))?;
    let secret = base32::decode_secret(&secret_b32)?;

    let digits = super::validate_digits(digits)?;
    let period = super::validate_period(period)?;

    // Issuer precedence: explicit ?issuer= wins, else the label prefix.
    let issuer = issuer_param.or(label_issuer).filter(|s| !s.is_empty());

    let counter = match otp_type {
        OtpType::Hotp => counter.unwrap_or(0),
        OtpType::Totp => 0,
    };

    if account.is_empty() && issuer.is_none() {
        return Err(CoreError::InvalidUri("URI has no account or issuer".into()));
    }

    Ok(ParsedOtp {
        otp_type,
        issuer,
        account,
        secret,
        algorithm,
        digits,
        period,
        counter,
    })
}

use super::base32;

/// Build a canonical `otpauth://` URI from raw secret bytes and metadata.
///
/// Used only for deliberate single-account export; callers must treat the
/// result as credential material.
#[allow(clippy::too_many_arguments)]
pub fn build(
    otp_type: OtpType,
    issuer: Option<&str>,
    account: &str,
    secret_bytes: &[u8],
    algorithm: Algorithm,
    digits: u8,
    period: u32,
    counter: u64,
) -> String {
    let label = match issuer {
        Some(iss) if !iss.is_empty() => {
            format!("{}:{}", percent_encode(iss), percent_encode(account))
        }
        _ => percent_encode(account),
    };

    let mut query = format!("secret={}", base32::encode_secret(secret_bytes));
    if let Some(iss) = issuer {
        if !iss.is_empty() {
            query.push_str(&format!("&issuer={}", percent_encode(iss)));
        }
    }
    query.push_str(&format!("&algorithm={}", algorithm.as_str()));
    query.push_str(&format!("&digits={digits}"));
    match otp_type {
        OtpType::Totp => query.push_str(&format!("&period={period}")),
        OtpType::Hotp => query.push_str(&format!("&counter={counter}")),
    }

    format!("otpauth://{}/{}?{}", otp_type.as_str(), label, query)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_totp() {
        let p = parse("otpauth://totp/GitHub:kevin@example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&algorithm=SHA1&digits=6&period=30").unwrap();
        assert_eq!(p.otp_type, OtpType::Totp);
        assert_eq!(p.issuer.as_deref(), Some("GitHub"));
        assert_eq!(p.account, "kevin@example.com");
        assert_eq!(p.algorithm, Algorithm::Sha1);
        assert_eq!(p.digits, 6);
        assert_eq!(p.period, 30);
        assert!(!p.secret.is_empty());
    }

    #[test]
    fn issuer_from_label_when_no_param() {
        let p = parse("otpauth://totp/Google:me@gmail.com?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(p.issuer.as_deref(), Some("Google"));
        assert_eq!(p.account, "me@gmail.com");
        assert_eq!(p.digits, DEFAULT_DIGITS);
        assert_eq!(p.period, DEFAULT_PERIOD);
    }

    #[test]
    fn param_issuer_overrides_label() {
        let p = parse("otpauth://totp/Wrong:acct?secret=JBSWY3DPEHPK3PXP&issuer=Right").unwrap();
        assert_eq!(p.issuer.as_deref(), Some("Right"));
    }

    #[test]
    fn parses_hotp_with_counter() {
        let p = parse("otpauth://hotp/Acme:bob?secret=JBSWY3DPEHPK3PXP&counter=42").unwrap();
        assert_eq!(p.otp_type, OtpType::Hotp);
        assert_eq!(p.counter, 42);
    }

    #[test]
    fn decodes_percent_and_spaces() {
        let p = parse("otpauth://totp/My%20Bank:john%40doe.com?secret=JBSWY3DPEHPK3PXP").unwrap();
        assert_eq!(p.issuer.as_deref(), Some("My Bank"));
        assert_eq!(p.account, "john@doe.com");
    }

    #[test]
    fn sha512_and_eight_digits() {
        let p =
            parse("otpauth://totp/x:y?secret=JBSWY3DPEHPK3PXP&algorithm=SHA512&digits=8").unwrap();
        assert_eq!(p.algorithm, Algorithm::Sha512);
        assert_eq!(p.digits, 8);
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse("https://example.com").is_err());
        assert!(parse("otpauth://totp/acct").is_err()); // no secret
        assert!(parse("otpauth://foo/acct?secret=JBSWY3DPEHPK3PXP").is_err()); // bad type
        assert!(parse("otpauth://totp/acct?secret=not_base32!!").is_err());
        assert!(parse("otpauth://totp/acct?secret=JBSWY3DP&digits=99").is_err());
        assert!(parse("otpauth://totp?secret=JBSWY3DPEHPK3PXP").is_err()); // no label
    }

    #[test]
    fn build_round_trips() {
        let original = parse("otpauth://totp/GitHub:kevin@example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub&algorithm=SHA256&digits=8&period=45").unwrap();
        let uri = build(
            original.otp_type,
            original.issuer.as_deref(),
            &original.account,
            &original.secret,
            original.algorithm,
            original.digits,
            original.period,
            original.counter,
        );
        let reparsed = parse(&uri).unwrap();
        assert_eq!(reparsed.issuer.as_deref(), Some("GitHub"));
        assert_eq!(reparsed.account, "kevin@example.com");
        assert_eq!(reparsed.algorithm, Algorithm::Sha256);
        assert_eq!(reparsed.digits, 8);
        assert_eq!(reparsed.period, 45);
        assert_eq!(&reparsed.secret[..], &original.secret[..]);
    }
}
