// Wishtory: SAFETY MODEL
// -----------------------------------------------------------------------------
// * Game files are accessed READ-ONLY, and only from Rust (discover.rs, authkey.rs).
// * We never attach to / inject into / talk to the game process.
// * We never write to game folders, never set proxies, never require admin rights.
// * The only network traffic is the official HoYoverse history API (fetch.rs).
// -----------------------------------------------------------------------------
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod authkey;
mod commands;
mod db;
mod discover;
mod error;
mod fetch;
mod fsutil;
mod models;
mod pity;
mod uigf;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            app.manage(db::Db::open(&dir.join("wishtory.db"))?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::discover_games,
            commands::extract_authkey,
            commands::fetch_history,
            commands::refresh_from_game,
            commands::list_accounts,
            commands::delete_account,
            commands::get_history,
            commands::get_pity,
            commands::import_file,
            commands::export_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Wishtory");
}
