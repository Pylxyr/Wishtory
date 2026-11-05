// Wishtory: SAFETY MODEL
// -----------------------------------------------------------------------------
// * Game files are accessed READ-ONLY, and only from Rust (discover.rs, authkey.rs).
// * We never attach to / inject into / talk to the game process.
// * We never write to game folders, never set proxies, never require admin rights.
// * Network traffic is the official HoYoverse history API (fetch.rs), the signed catalog, banner
//   list and pity rules (catalog.rs, rules.rs) and a version check of this project's GitHub releases (update.rs). None of
//   them is sent an account or wish data.
// -----------------------------------------------------------------------------
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[macro_use]
mod logging;
mod about;
mod authkey;
mod catalog;
mod commands;
mod db;
mod discover;
mod error;
mod fetch;
mod fsutil;
mod models;
mod paimon_items;
mod pity;
mod rules;
mod settings;
mod uigf;
mod update;
#[cfg(unix)]
mod wine;

use tauri::Manager;

/// WebKitGTK's DMABUF renderer can leave the window blank on NVIDIA's proprietary driver (common on
/// Wayland). Turn it off there, unless the person already chose a value themselves.
#[cfg(target_os = "linux")]
fn tune_webkit() {
    let nvidia = std::path::Path::new("/proc/driver/nvidia").exists();
    if nvidia && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    tune_webkit();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(db::Db::open(&dir.join("wishtory.db"))?);
            let catalog = catalog::CatalogStore::open(&dir);
            app.manage(catalog);
            app.manage(catalog::BannerStore::open(&dir));
            app.manage(rules::RulesStore::open(&dir));
            app.manage(settings::SettingsStore::open(&dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::discover_games,
            commands::get_game_locations,
            commands::set_game_locations,
            commands::extract_authkey,
            commands::fetch_history,
            commands::refresh_from_game,
            commands::list_accounts,
            commands::delete_account,
            commands::clear_all_history,
            commands::get_catalog,
            commands::refresh_catalog,
            commands::get_banners,
            commands::get_banner_update,
            commands::refresh_banners,
            commands::get_game_meta,
            commands::get_history,
            commands::get_pity,
            about::get_about,
            about::open_external,
            update::check_app_update,
            commands::import_file,
            commands::export_file,
            logging::set_logging_enabled,
            logging::get_logs,
            logging::clear_logs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wishtory");
}
