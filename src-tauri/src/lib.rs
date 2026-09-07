//! ClovaKey desktop application entry point (Tauri shell).
//!
//! This is a thin, non-security-critical layer: it wires the `clovakey-core`
//! library to the UI via Tauri commands, and owns desktop concerns (tray,
//! clipboard, idle-lock, autostart). All secret handling stays in the core.

mod clipboard;
mod commands;
mod error;
mod platform;
mod settings;
mod state;
mod tray;

use state::AppState;
use std::time::Duration;
use tauri::{Emitter, Manager};

const KEYCHAIN_SERVICE: &str = "com.clova.clovakey";
const VAULT_FILE: &str = "vault.db";

/// Build the vault at the app data directory and manage the app state.
fn init_state(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&data_dir)?;
    let vault_path = data_dir.join(VAULT_FILE);

    let vault = clovakey_core::Vault::open(&vault_path, KEYCHAIN_SERVICE)
        .map_err(|e| format!("failed to open vault: {e}"))?;

    app.manage(AppState::new(vault));
    Ok(())
}

/// Background idle auto-lock: locks the vault after the configured inactivity
/// timeout and notifies the UI. A timeout of 0 disables it.
fn spawn_idle_locker(handle: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(10));
        let state = handle.state::<AppState>();
        let timeout = settings::auto_lock_secs(&state.vault);
        if timeout == 0 || state.vault.is_locked() {
            continue;
        }
        let idle = state
            .activity
            .lock()
            .unwrap()
            .last_active
            .elapsed()
            .as_secs();
        if idle >= timeout {
            state.vault.lock();
            let _ = handle.emit("clovakey://locked", ());
        }
    });
}

/// Application run entry, invoked from `main.rs` (and, later, mobile entry).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Single-instance must be registered first so a second launch focuses the
    // existing window instead of starting a new process.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_main(app);
        }));
    }

    builder
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    None,
                ))?;
            }

            init_state(app)?;

            // Build the tray unless the user disabled it.
            let show_tray = {
                let state = app.state::<AppState>();
                state
                    .vault
                    .get_setting("show_tray")
                    .ok()
                    .flatten()
                    .map(|v| v != "false")
                    .unwrap_or(true)
            };
            if show_tray {
                if let Err(e) = tray::build(&app.handle().clone()) {
                    eprintln!("clovakey: tray unavailable: {e}");
                }
            }

            spawn_idle_locker(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::vault_setup,
            commands::vault_unlock,
            commands::vault_lock,
            commands::vault_set_passphrase,
            commands::vault_remove_passphrase,
            commands::vault_change_passphrase,
            commands::note_activity,
            commands::accounts_list,
            commands::codes_generate_all,
            commands::code_generate,
            commands::hotp_advance,
            commands::account_add_manual,
            commands::account_add_uri,
            commands::account_update,
            commands::account_delete,
            commands::account_reorder,
            commands::account_reveal,
            commands::groups_list,
            commands::group_create,
            commands::group_rename,
            commands::group_delete,
            commands::copy_code,
            commands::scan_qr_text,
            commands::scan_qr_image,
            commands::google_import_begin,
            commands::google_import_add_text,
            commands::google_import_add_image,
            commands::google_import_preview,
            commands::google_import_commit,
            commands::google_import_cancel,
            commands::backup_create,
            commands::backup_restore,
            commands::settings_get_all,
            commands::settings_set,
            commands::data_wipe,
            commands::time_status,
            commands::security_capabilities,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClovaKey");
}
