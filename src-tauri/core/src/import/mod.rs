//! Account import: standard `otpauth://` URIs and Google Authenticator
//! migration exports.
//!
//! - [`google`] — the `otpauth-migration://` protobuf pipeline + batch handling.
//! - [`protobuf`] — the bounded wire-format reader it uses.
//!
//! Single `otpauth://` URIs are parsed by [`crate::otp::uri`].

pub mod google;
pub mod protobuf;

/// Classify a scanned/pasted string so the UI can route it correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScannedKind {
    Otpauth,
    Migration,
    Unknown,
}

/// Best-effort classification of arbitrary scanned text.
pub fn classify(text: &str) -> ScannedKind {
    let lower = text.trim().to_ascii_lowercase();
    if lower.starts_with("otpauth-migration://") {
        ScannedKind::Migration
    } else if lower.starts_with("otpauth://") {
        ScannedKind::Otpauth
    } else {
        ScannedKind::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_scanned_text() {
        assert_eq!(classify("otpauth://totp/x?secret=A"), ScannedKind::Otpauth);
        assert_eq!(
            classify("otpauth-migration://offline?data=AAAA"),
            ScannedKind::Migration
        );
        assert_eq!(classify("https://example.com"), ScannedKind::Unknown);
        assert_eq!(classify("   OTPAUTH://TOTP/x  "), ScannedKind::Otpauth);
    }
}
