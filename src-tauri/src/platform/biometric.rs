//! Biometric unlock abstraction (Touch ID / Windows Hello).
//!
//! # Status
//! ClovaKey's lock model is intentionally solid without biometrics: a hard
//! passphrase lock (Argon2id) or the OS-keychain-backed soft lock. Reliable
//! desktop biometrics require native bridging (`LocalAuthentication` on macOS,
//! `Windows.Security.Credentials.UI` on Windows) that must be done carefully to
//! avoid a fragile "sometimes works" experience.
//!
//! Rather than ship that half-done, we expose this stable abstraction now and
//! report biometrics as unavailable. A future milestone implements
//! [`authenticate`] per platform; the passphrase-wrapping design already
//! supports gating an unlock behind a successful biometric check.

/// Whether biometric unlock is currently usable.
pub fn is_available() -> bool {
    false
}

/// The biometric method that applies to this platform (shown in Settings even
/// before it is implemented, e.g. "Touch ID (coming soon)").
pub fn preferred_kind() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        Some("Touch ID".to_string())
    }
    #[cfg(target_os = "windows")]
    {
        Some("Windows Hello".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Prompt for biometric authentication. Returns `Ok(true)` on success.
///
/// Not yet implemented on desktop; see the module docs.
#[allow(dead_code)]
pub fn authenticate(_reason: &str) -> Result<bool, String> {
    Err("biometric authentication is not available on this platform yet".to_string())
}
