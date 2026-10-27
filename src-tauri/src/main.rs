// Wishtory: SAFETY MODEL
// -----------------------------------------------------------------------------
// * Game files are accessed READ-ONLY, and only from Rust (discover.rs, authkey.rs).
// * We never attach to / inject into / talk to the game process.
// * We never write to game folders, never set proxies, never require admin rights.
// * Network traffic is the official HoYoverse history API (fetch.rs), the signed catalog and banner
//   list (catalog.rs) and a version check of this project's GitHub releases (update.rs). None of
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
mod uigf;
mod update;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(db::Db::open(&dir.join("wishtory.db"))?);
            let catalog = catalog::CatalogStore::open(&dir);
            app.manage(catalog);
            app.manage(catalog::BannerStore::open(&dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::discover_games,
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
