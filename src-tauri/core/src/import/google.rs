//! Google Authenticator "Export accounts" migration import.
//!
//! Pipeline: `otpauth-migration://offline?data=…` → percent-decode → Base64
//! decode → protobuf `MigrationPayload` → OTP records. Large exports are split
//! across several QR codes ("batches"); [`BatchCollector`] reassembles them and
//! refuses to mix codes from different exports.
//!
//! Secret bytes in a migration payload are **raw key material**, not Base32
//! text — we keep them raw and hand them to the vault, which encrypts them.

use super::protobuf::{Reader, WIRE_LEN, WIRE_VARINT};
use crate::error::{CoreError, Result};
use crate::otp::{Algorithm, OtpType};
use data_encoding::{BASE64, BASE64_NOPAD};
use std::collections::BTreeMap;
use zeroize::Zeroizing;

/// Upper bounds to defend against hostile / malformed payloads.
const MAX_PAYLOAD_BYTES: usize = 512 * 1024;
const MAX_OTPS_PER_PAYLOAD: usize = 10_000;

/// One decoded OTP record from a migration payload.
pub struct MigrationOtp {
    pub secret: Zeroizing<Vec<u8>>,
    pub name: String,
    pub issuer: String,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub otp_type: OtpType,
    pub counter: u64,
}

/// A decoded `MigrationPayload` (one QR code's worth).
pub struct MigrationPayload {
    pub otps: Vec<MigrationOtp>,
    pub version: i32,
    pub batch_size: i32,
    pub batch_index: i32,
    pub batch_id: i32,
}

fn map_algorithm(v: u64) -> Result<Algorithm> {
    match v {
        0 | 1 => Ok(Algorithm::Sha1), // UNSPECIFIED defaults to SHA1
        2 => Ok(Algorithm::Sha256),
        3 => Ok(Algorithm::Sha512),
        4 => Err(CoreError::MigrationDecode("MD5 OTP is not supported")),
        _ => Err(CoreError::MigrationDecode("unknown algorithm")),
    }
}

fn map_digits(v: u64) -> Result<u8> {
    match v {
        0 | 1 => Ok(6), // UNSPECIFIED defaults to 6
        2 => Ok(8),
        _ => Err(CoreError::MigrationDecode("unknown digit count")),
    }
}

fn map_type(v: u64) -> Result<OtpType> {
    match v {
        1 => Ok(OtpType::Hotp),
        2 => Ok(OtpType::Totp),
        _ => Err(CoreError::MigrationDecode("unknown OTP type")),
    }
}

/// Parse a single `OtpParameters` sub-message.
fn parse_otp_parameters(bytes: &[u8]) -> Result<MigrationOtp> {
    let mut r = Reader::new(bytes);
    let mut secret: Option<Zeroizing<Vec<u8>>> = None;
    let mut name = String::new();
    let mut issuer = String::new();
    let mut algorithm = Algorithm::Sha1;
    let mut digits = 6u8;
    let mut otp_type: Option<OtpType> = None;
    let mut counter = 0u64;

    while !r.is_empty() {
        let (field, wire) = r.read_tag()?;
        match (field, wire) {
            (1, WIRE_LEN) => secret = Some(Zeroizing::new(r.read_bytes()?.to_vec())),
            (2, WIRE_LEN) => {
                name = String::from_utf8_lossy(r.read_bytes()?).into_owned();
            }
            (3, WIRE_LEN) => {
                issuer = String::from_utf8_lossy(r.read_bytes()?).into_owned();
            }
            (4, WIRE_VARINT) => algorithm = map_algorithm(r.read_varint()?)?,
            (5, WIRE_VARINT) => digits = map_digits(r.read_varint()?)?,
            (6, WIRE_VARINT) => otp_type = Some(map_type(r.read_varint()?)?),
            (7, WIRE_VARINT) => counter = r.read_varint()?,
            (_, wire) => r.skip(wire)?, // forward-compatible
        }
    }

    let secret = secret
        .filter(|s| !s.is_empty())
        .ok_or(CoreError::MigrationDecode(
            "an account in the export has no secret",
        ))?;
    let otp_type = otp_type.ok_or(CoreError::MigrationDecode("an account has no OTP type"))?;

    Ok(MigrationOtp {
        secret,
        name: name.trim().to_string(),
        issuer: issuer.trim().to_string(),
        algorithm,
        digits,
        otp_type,
        counter,
    })
}

/// Parse a raw `MigrationPayload` protobuf message.
pub fn parse_payload(bytes: &[u8]) -> Result<MigrationPayload> {
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(CoreError::MigrationDecode("export is too large"));
    }

    let mut r = Reader::new(bytes);
    let mut otps = Vec::new();
    let mut version = 0i32;
    let mut batch_size = 0i32;
    let mut batch_index = 0i32;
    let mut batch_id = 0i32;

    while !r.is_empty() {
        let (field, wire) = r.read_tag()?;
        match (field, wire) {
            (1, WIRE_LEN) => {
                if otps.len() >= MAX_OTPS_PER_PAYLOAD {
                    return Err(CoreError::MigrationDecode("export has too many accounts"));
                }
                otps.push(parse_otp_parameters(r.read_bytes()?)?);
            }
            (2, WIRE_VARINT) => version = r.read_varint()? as i32,
            (3, WIRE_VARINT) => batch_size = r.read_varint()? as i32,
            (4, WIRE_VARINT) => batch_index = r.read_varint()? as i32,
            (5, WIRE_VARINT) => batch_id = r.read_varint()? as i32,
            (_, wire) => r.skip(wire)?,
        }
    }

    Ok(MigrationPayload {
        otps,
        version,
        batch_size,
        batch_index,
        batch_id,
    })
}

/// Decode the Base64 `data` value (tolerant of padded / unpadded forms).
fn decode_base64(data: &str) -> Result<Vec<u8>> {
    let cleaned: String = data.chars().filter(|c| !c.is_whitespace()).collect();
    if let Ok(bytes) = BASE64.decode(cleaned.as_bytes()) {
        return Ok(bytes);
    }
    let trimmed = cleaned.trim_end_matches('=');
    BASE64_NOPAD
        .decode(trimmed.as_bytes())
        .map_err(|_| CoreError::MigrationDecode("export data is not valid Base64"))
}

/// Percent-decode a URI query value.
fn percent_decode(input: &str) -> Result<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                match (hi, lo) {
                    (Some(h), Some(l)) => {
                        out.push((h * 16 + l) as u8);
                        i += 3;
                    }
                    _ => return Err(CoreError::MigrationDecode("malformed export URI")),
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| CoreError::MigrationDecode("malformed export URI"))
}

/// Parse a full `otpauth-migration://offline?data=…` URI into a payload.
pub fn parse_migration_uri(uri: &str) -> Result<MigrationPayload> {
    let uri = uri.trim();
    let lower = uri.to_ascii_lowercase();
    if !lower.starts_with("otpauth-migration://") {
        return Err(CoreError::MigrationDecode(
            "this is not a Google Authenticator export",
        ));
    }
    let query = uri
        .split_once('?')
        .map(|(_, q)| q)
        .ok_or(CoreError::MigrationDecode("export URI has no data"))?;

    let mut data: Option<String> = None;
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k.eq_ignore_ascii_case("data") {
                data = Some(percent_decode(v)?);
            }
        }
    }
    let data = data.ok_or(CoreError::MigrationDecode("export URI has no data"))?;
    let raw = decode_base64(&data)?;
    parse_payload(&raw)
}

/// Progress of a multi-QR batch import.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgress {
    pub received: usize,
    pub total: usize,
    pub complete: bool,
}

/// Reassembles a multi-QR Google export, rejecting mismatched batches.
#[derive(Default)]
pub struct BatchCollector {
    batch_id: Option<i32>,
    batch_size: usize,
    /// index → records for that QR code
    received: BTreeMap<i32, Vec<MigrationOtp>>,
}

impl BatchCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one decoded payload. Returns current progress.
    ///
    /// Errors if this QR belongs to a different export than the ones already
    /// scanned, or if its batch metadata is inconsistent.
    pub fn add(&mut self, payload: MigrationPayload) -> Result<BatchProgress> {
        // Treat a missing/zero batch_size as a single-code export.
        let size = if payload.batch_size <= 0 {
            1
        } else {
            payload.batch_size as usize
        };
        let index = payload.batch_index.max(0);

        if index as usize >= size {
            return Err(CoreError::MigrationDecode(
                "export QR has an out-of-range batch index",
            ));
        }

        match self.batch_id {
            None => {
                self.batch_id = Some(payload.batch_id);
                self.batch_size = size;
            }
            Some(existing) => {
                if existing != payload.batch_id || self.batch_size != size {
                    return Err(CoreError::MigrationDecode(
                        "this QR code is from a different export — scan the codes from a single export",
                    ));
                }
            }
        }

        self.received.insert(index, payload.otps);
        Ok(self.progress())
    }

    pub fn progress(&self) -> BatchProgress {
        BatchProgress {
            received: self.received.len(),
            total: self.batch_size.max(1),
            complete: self.is_complete(),
        }
    }

    pub fn is_complete(&self) -> bool {
        self.batch_size > 0 && self.received.len() == self.batch_size
    }

    /// Consume the collector, returning all records in batch order.
    pub fn into_records(self) -> Vec<MigrationOtp> {
        self.received.into_values().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::protobuf::encode::*;
    use super::*;

    // ── Synthetic fixture builders (no real credentials) ─────────────────

    struct OtpFixture {
        secret: Vec<u8>,
        name: &'static str,
        issuer: &'static str,
        algorithm: u64,
        digits: u64,
        otp_type: u64,
        counter: u64,
    }

    fn encode_otp(o: &OtpFixture) -> Vec<u8> {
        let mut b = Vec::new();
        field_bytes(&mut b, 1, &o.secret);
        field_bytes(&mut b, 2, o.name.as_bytes());
        field_bytes(&mut b, 3, o.issuer.as_bytes());
        field_varint(&mut b, 4, o.algorithm);
        field_varint(&mut b, 5, o.digits);
        field_varint(&mut b, 6, o.otp_type);
        field_varint(&mut b, 7, o.counter);
        b
    }

    fn encode_payload(otps: &[OtpFixture], size: u64, index: u64, id: i32) -> Vec<u8> {
        let mut b = Vec::new();
        for o in otps {
            field_bytes(&mut b, 1, &encode_otp(o));
        }
        field_varint(&mut b, 2, 1); // version
        field_varint(&mut b, 3, size);
        field_varint(&mut b, 4, index);
        field_varint(&mut b, 5, id as u64);
        b
    }

    fn to_migration_uri(payload: &[u8]) -> String {
        // Standard Base64 then percent-encode the +/=/ characters as GA does.
        let b64 = BASE64.encode(payload);
        let enc: String = b64
            .chars()
            .map(|c| match c {
                '+' => "%2B".to_string(),
                '/' => "%2F".to_string(),
                '=' => "%3D".to_string(),
                other => other.to_string(),
            })
            .collect();
        format!("otpauth-migration://offline?data={enc}")
    }

    fn sample() -> OtpFixture {
        OtpFixture {
            // Raw bytes for "12345678901234567890" (the RFC seed) — synthetic.
            secret: b"12345678901234567890".to_vec(),
            name: "kevin@example.com",
            issuer: "GitHub",
            algorithm: 1,
            digits: 1,
            otp_type: 2,
            counter: 0,
        }
    }

    #[test]
    fn parses_single_account_payload() {
        let bytes = encode_payload(&[sample()], 1, 0, 42);
        let p = parse_payload(&bytes).unwrap();
        assert_eq!(p.otps.len(), 1);
        let otp = &p.otps[0];
        assert_eq!(&otp.secret[..], b"12345678901234567890");
        assert_eq!(otp.name, "kevin@example.com");
        assert_eq!(otp.issuer, "GitHub");
        assert_eq!(otp.algorithm, Algorithm::Sha1);
        assert_eq!(otp.digits, 6);
        assert_eq!(otp.otp_type, OtpType::Totp);
        assert_eq!(p.batch_size, 1);
    }

    #[test]
    fn parses_full_migration_uri() {
        let bytes = encode_payload(&[sample()], 1, 0, 7);
        let uri = to_migration_uri(&bytes);
        let p = parse_migration_uri(&uri).unwrap();
        assert_eq!(p.otps.len(), 1);
        assert_eq!(p.otps[0].issuer, "GitHub");
    }

    #[test]
    fn secret_bytes_are_raw_not_base32() {
        // A crucial correctness check: the raw secret must round-trip byte for
        // byte, never be reinterpreted as Base32 text.
        let raw = vec![0x00u8, 0xFF, 0x10, 0x99, 0xAB];
        let fixture = OtpFixture {
            secret: raw.clone(),
            ..sample()
        };
        let bytes = encode_payload(&[fixture], 1, 0, 1);
        let p = parse_payload(&bytes).unwrap();
        assert_eq!(&p.otps[0].secret[..], &raw[..]);
    }

    #[test]
    fn hotp_and_sha256_eight_digits() {
        let fixture = OtpFixture {
            algorithm: 2, // SHA256
            digits: 2,    // 8
            otp_type: 1,  // HOTP
            counter: 99,
            ..sample()
        };
        let bytes = encode_payload(&[fixture], 1, 0, 1);
        let otp = &parse_payload(&bytes).unwrap().otps[0];
        assert_eq!(otp.algorithm, Algorithm::Sha256);
        assert_eq!(otp.digits, 8);
        assert_eq!(otp.otp_type, OtpType::Hotp);
        assert_eq!(otp.counter, 99);
    }

    #[test]
    fn multi_batch_reassembly() {
        let id = 555;
        let p0 = parse_payload(&encode_payload(&[sample()], 3, 0, id)).unwrap();
        let p1 = parse_payload(&encode_payload(&[sample()], 3, 1, id)).unwrap();
        let p2 = parse_payload(&encode_payload(&[sample()], 3, 2, id)).unwrap();

        let mut c = BatchCollector::new();
        let pr = c.add(p0).unwrap();
        assert_eq!((pr.received, pr.total, pr.complete), (1, 3, false));
        c.add(p1).unwrap();
        let pr = c.add(p2).unwrap();
        assert!(pr.complete);
        assert_eq!(c.into_records().len(), 3);
    }

    #[test]
    fn rejects_mixed_batches() {
        let p_a = parse_payload(&encode_payload(&[sample()], 2, 0, 100)).unwrap();
        let p_b = parse_payload(&encode_payload(&[sample()], 2, 1, 999)).unwrap(); // different id
        let mut c = BatchCollector::new();
        c.add(p_a).unwrap();
        assert!(c.add(p_b).is_err());
    }

    #[test]
    fn rejects_out_of_range_index() {
        let bad = parse_payload(&encode_payload(&[sample()], 2, 5, 1)).unwrap();
        let mut c = BatchCollector::new();
        assert!(c.add(bad).is_err());
    }

    #[test]
    fn rejects_non_migration_uri() {
        assert!(parse_migration_uri("otpauth://totp/x?secret=AAAA").is_err());
        assert!(parse_migration_uri("otpauth-migration://offline").is_err());
        assert!(parse_migration_uri("otpauth-migration://offline?data=@@@@").is_err());
    }

    #[test]
    fn rejects_garbage_protobuf() {
        assert!(parse_payload(&[0xff, 0xff, 0xff, 0xff]).is_err());
        assert!(parse_payload(&[0x0a, 0x7f]).is_err()); // len-delimited claims 127 bytes
    }
}
