//! Optional menu-bar / system-tray presence.
//!
//! Exposes only Open, Lock, and Quit. It never surfaces account names or codes
//! — those require an unlocked window.

use crate::state::AppState;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// Bring the main window to the foreground.
pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// Lock the vault from anywhere and notify the UI.
fn lock(app: &AppHandle) {
    let state = app.state::<AppState>();
    state.vault.lock();
    let _ = app.emit("clovakey://locked", ());
}

/// Build the tray icon + menu. Called once at startup when the tray is enabled.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "tray_open", "Open ClovaKey", true, None::<&str>)?;
    let lock_item = MenuItem::with_id(app, "tray_lock", "Lock ClovaKey", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "tray_quit", "Quit ClovaKey", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &lock_item, &sep, &quit])?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("ClovaKey")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray_open" => show_main(app),
            "tray_lock" => {
                lock(app);
                show_main(app);
            }
            "tray_quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}
