//! Core data model.
//!
//! A key design rule: **`Account` never contains the OTP secret.** The secret
//! lives only as encrypted bytes in the vault and is decrypted transiently
//! inside the core when a code is generated. Everything here is non-sensitive,
//! searchable metadata safe to hand to the UI.

use crate::crypto::rng;
use crate::otp::{Algorithm, OtpType, DEFAULT_DIGITS, DEFAULT_PERIOD};
use serde::{Deserialize, Serialize};

/// The OTP configuration for an account (no secret).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OtpConfig {
    #[serde(rename = "type")]
    pub otp_type: OtpType,
    pub algorithm: Algorithm,
    pub digits: u8,
    pub period: u32,
    pub counter: u64,
}

impl Default for OtpConfig {
    fn default() -> Self {
        OtpConfig {
            otp_type: OtpType::Totp,
            algorithm: Algorithm::Sha1,
            digits: DEFAULT_DIGITS,
            period: DEFAULT_PERIOD,
            counter: 0,
        }
    }
}

/// Non-sensitive account metadata as stored and shown in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub issuer: Option<String>,
    pub account_name: String,
    #[serde(flatten)]
    pub otp: OtpConfig,
    pub group_id: Option<String>,
    pub favorite: bool,
    pub sort_order: i64,
    /// Optional icon slug (resolves to a bundled/cached icon) — never a URL.
    pub icon: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Fields required to create a new account. The secret is passed separately as
/// raw bytes so it can be encrypted immediately and never stored on this struct.
#[derive(Debug, Clone)]
pub struct NewAccount {
    pub issuer: Option<String>,
    pub account_name: String,
    pub otp: OtpConfig,
    pub group_id: Option<String>,
    pub favorite: bool,
    pub icon: Option<String>,
}

impl NewAccount {
    pub fn new(account_name: impl Into<String>) -> Self {
        NewAccount {
            issuer: None,
            account_name: account_name.into(),
            otp: OtpConfig::default(),
            group_id: None,
            favorite: false,
            icon: None,
        }
    }
}

/// A patch of mutable account fields. `None` means "leave unchanged"; the
/// double-`Option` fields allow explicitly clearing a value.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountPatch {
    pub issuer: Option<Option<String>>,
    pub account_name: Option<String>,
    pub group_id: Option<Option<String>>,
    pub favorite: Option<bool>,
    pub icon: Option<Option<String>>,
    pub sort_order: Option<i64>,
}

/// An optional organizational group.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
}

/// A generated code plus the timing/state the UI needs to render it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCode {
    pub account_id: String,
    pub code: String,
    #[serde(rename = "type")]
    pub otp_type: OtpType,
    pub digits: u8,
    /// TOTP: seconds in a full window. HOTP: 0.
    pub period: u32,
    /// TOTP: UNIX second at which this code expires. HOTP: 0.
    pub expires_at: u64,
    /// TOTP: seconds remaining. HOTP: 0.
    pub seconds_remaining: u32,
    /// HOTP: current counter. TOTP: 0.
    pub counter: u64,
}

/// Generate a random, opaque 128-bit identifier rendered as lowercase hex.
pub fn new_id() -> String {
    let bytes = rng::random_array::<16>().unwrap_or([0u8; 16]);
    let mut s = String::with_capacity(32);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_hex() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn account_flattens_otp_config() {
        let acct = Account {
            id: "x".into(),
            issuer: Some("GitHub".into()),
            account_name: "kevin".into(),
            otp: OtpConfig::default(),
            group_id: None,
            favorite: false,
            sort_order: 0,
            icon: None,
            created_at: 0,
            updated_at: 0,
        };
        let json = serde_json::to_value(&acct).unwrap();
        // Flattened OTP fields appear at the top level for the UI.
        assert_eq!(json["type"], "totp");
        assert_eq!(json["digits"], 6);
        assert_eq!(json["accountName"], "kevin");
    }
}
