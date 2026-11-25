// Wishtory: SAFETY MODEL
// -----------------------------------------------------------------------------
// The rules live in `wishtory-core` (see the header of its lib.rs) and are the same for every UI:
// game files are only read, never written; nothing attaches to the game; no account or wish data is
// ever sent anywhere except, encrypted, to the person's own Google Drive when they switch backup on.
// This shell only forwards calls from the window to `wishtory-api` and opens file dialogs.
// -----------------------------------------------------------------------------
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use tauri::Manager;
use wishtory_api::api::core_api::WishtoryCore;

/// `WebKitGTK`'s DMABUF renderer can leave the window blank on NVIDIA's proprietary driver (common on
/// Wayland). Turn it off there, unless the person already chose a value themselves.
#[cfg(target_os = "linux")]
fn tune_webkit() {
    let nvidia = std::path::Path::new("/proc/driver/nvidia").exists();
    if nvidia && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        // SAFETY: called first thing in `main`, before any thread exists.
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    tune_webkit();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Opening can fail (for example a second copy is running). The window still starts and
            // shows that message, instead of the app vanishing.
            let opened = tauri::async_runtime::block_on(WishtoryCore::open(None));
            app.manage(commands::Shell::new(opened));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::startup_error,
            commands::discover_games,
            commands::get_game_locations,
            commands::set_game_locations,
            commands::refresh_from_game,
            commands::fetch_history,
            commands::list_accounts,
            commands::delete_account,
            commands::clear_all_history,
            commands::get_history,
            commands::get_pity,
            commands::get_catalog,
            commands::get_banners,
            commands::get_catalog_status,
            commands::check_catalog,
            commands::get_catalog_review,
            commands::get_impact_page,
            commands::apply_catalog,
            commands::dismiss_catalog,
            commands::get_catalog_history,
            commands::set_catalog_auto_apply,
            commands::cloud_status,
            commands::cloud_sign_in,
            commands::cloud_cancel_sign_in,
            commands::cloud_sign_out,
            commands::cloud_set_auto_backup,
            commands::cloud_backup_now,
            commands::cloud_backup_if_due,
            commands::cloud_list_backups,
            commands::cloud_restore,
            commands::import_file,
            commands::export_file,
            commands::game_meta,
            commands::get_about,
            commands::open_external,
            commands::check_app_update,
            commands::set_logging_enabled,
            commands::get_logs,
            commands::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wishtory");
}
