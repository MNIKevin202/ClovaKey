//! ClovaKey desktop application entry point (Tauri shell).
//!
//! This is a thin, non-security-critical layer: it wires the `clovakey-core`
//! library to the UI via Tauri commands, and owns desktop concerns (tray,
//! clipboard, updater, autostart). All secret handling stays in the core.

/// Application run entry, invoked from `main.rs` (and, later, mobile entry).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Filled in during M1 (app shell wiring).
}
