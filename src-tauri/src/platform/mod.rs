//! Platform-specific integration points.
//!
//! Kept behind clean abstractions so per-OS behaviour (and future biometric
//! unlock) can be added without touching the rest of the app.

pub mod biometric;
