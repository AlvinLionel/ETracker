use tauri::Manager;
use infrastructure::database::Database;

mod infrastructure;
mod domain;
mod application;
mod persistence;
mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::accounts::create_account,
            commands::accounts::get_account,
            commands::accounts::list_active_accounts,
            commands::accounts::update_account,
            commands::accounts::archive_account,
            commands::accounts::reactivate_account,
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_data_dir)?;

            let database_path = app_data_dir.join("etracker.db");
            let database = Database::initialize(&database_path)?;

            app.manage(database);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ETracker application");
}