//! Non-sensitive application settings, stored as key/value strings in the vault
//! DB (readable without unlocking, since they hold no secrets).
//!
//! Rust reads a couple of these directly (clipboard clear delay, idle-lock
//! timeout); the rest are applied by the frontend.

use crate::error::{AppError, AppResult};
use clovakey_core::Vault;
use serde_json::{Map, Value};

pub const CLIPBOARD_CLEAR_SECS: &str = "clipboard_clear_secs";
pub const AUTO_LOCK_SECS: &str = "auto_lock_secs";

/// Known settings and their string defaults. Also the allowlist for writes.
const DEFAULTS: &[(&str, &str)] = &[
    ("theme", "system"),          // system | light | dark
    ("density", "comfortable"),   // comfortable | compact
    (CLIPBOARD_CLEAR_SECS, "30"), // 0 = never
    (AUTO_LOCK_SECS, "0"),        // 0 = never; else seconds
    ("lock_on_start", "false"),
    ("show_tray", "true"),
    ("close_to_tray", "false"),
    ("minimize_to_tray", "false"),
    ("reduce_motion", "false"),
    ("hide_codes", "false"), // blur codes until hovered/focused
    ("onboarded", "false"),
];

fn get_u64(vault: &Vault, key: &str, default: u64) -> u64 {
    vault
        .get_setting(key)
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(default)
}

pub fn clipboard_clear_secs(vault: &Vault) -> u64 {
    get_u64(vault, CLIPBOARD_CLEAR_SECS, 30)
}

pub fn auto_lock_secs(vault: &Vault) -> u64 {
    get_u64(vault, AUTO_LOCK_SECS, 0)
}

/// Return all settings as a JSON object, defaults filled in for missing keys.
pub fn get_all(vault: &Vault) -> AppResult<Value> {
    let mut map = Map::new();
    for (k, v) in vault.all_settings()? {
        map.insert(k, Value::String(v));
    }
    for (k, d) in DEFAULTS {
        map.entry((*k).to_string())
            .or_insert_with(|| Value::String((*d).to_string()));
    }
    Ok(Value::Object(map))
}

/// Write a setting after validating it is a known key.
pub fn set(vault: &Vault, key: &str, value: &str) -> AppResult<()> {
    if !DEFAULTS.iter().any(|(k, _)| *k == key) {
        return Err(AppError::new("invalid_params", "Unknown setting."));
    }
    // Bound numeric settings to sane ranges.
    if key == CLIPBOARD_CLEAR_SECS || key == AUTO_LOCK_SECS {
        let n: u64 = value
            .parse()
            .map_err(|_| AppError::new("invalid_params", "Expected a number."))?;
        if n > 86_400 {
            return Err(AppError::new("invalid_params", "Value is too large."));
        }
    }
    vault.set_setting(key, value)?;
    Ok(())
}
