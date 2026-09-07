//! Clipboard copy with a race-safe auto-clear.
//!
//! When ClovaKey copies a code it remembers exactly what it wrote and a
//! monotonic token. The scheduled clear only fires if (a) no newer copy has
//! happened (token unchanged) and (b) the clipboard still holds the value we
//! wrote — so we never erase something the user copied afterwards.

use crate::state::AppState;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Copy `text` to the clipboard and, if `clear_after_secs > 0`, schedule an
/// auto-clear.
pub fn copy_with_autoclear(
    app: &AppHandle,
    text: String,
    clear_after_secs: u64,
) -> Result<(), String> {
    app.clipboard()
        .write_text(text.clone())
        .map_err(|e| e.to_string())?;

    let state = app.state::<AppState>();
    let token = {
        let mut cb = state.clipboard.lock().unwrap();
        cb.token = cb.token.wrapping_add(1);
        cb.last_value = Some(text.clone());
        cb.token
    };

    if clear_after_secs == 0 {
        return Ok(());
    }

    let app = app.clone();
    let expected = text;
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(clear_after_secs));
        let state = app.state::<AppState>();

        // (a) Skip if a newer copy happened or we no longer own the value.
        {
            let cb = state.clipboard.lock().unwrap();
            if cb.token != token || cb.last_value.as_deref() != Some(expected.as_str()) {
                return;
            }
        }

        // (b) Skip if the user has since copied something else.
        match app.clipboard().read_text() {
            Ok(current) if current == expected => {
                let _ = app.clipboard().clear();
                let mut cb = state.clipboard.lock().unwrap();
                if cb.token == token {
                    cb.last_value = None;
                }
            }
            _ => {}
        }
    });

    Ok(())
}
