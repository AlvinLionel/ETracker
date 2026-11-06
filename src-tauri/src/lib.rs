use infrastructure::database::Database;
use tauri::Manager;

mod application;
mod commands;
mod domain;
mod infrastructure;
mod persistence;

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
            commands::accounts::list_archived_accounts,
            commands::accounts::get_account_balance,
            commands::transactions::create_transaction,
            commands::transactions::update_transaction,
            commands::transactions::void_transaction,
            commands::transactions::reverse_transaction,
            commands::transactions::create_transfer,
            commands::imports::create_import_batch,
            commands::imports::list_import_batches,
            commands::imports::stage_csv_import,
            commands::imports::csv_headers,
            commands::imports::correct_import_record,
            commands::imports::list_staged_import_records,
            commands::transactions::list_transactions,
            commands::transactions::list_active_categories,
            commands::transactions::get_transaction_audit_history,
            commands::transactions::get_transaction_allocations,
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
