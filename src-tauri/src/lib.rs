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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::backup::export_encrypted_backup,
            commands::backup::restore_encrypted_backup,
            commands::analytics::get_analytics_summary,
            commands::dashboard::get_dashboard_summary,
            commands::search::search_ledger,
            commands::preferences::get_user_preferences,
            commands::preferences::set_user_preferences,
            commands::integrity::check_database_integrity,
            commands::diagnostics::get_diagnostic_report,
            commands::forecasting::generate_forecast,
            commands::forecasting::list_forecasts,
            commands::forecasting::evaluate_forecast,
            commands::forecasting::get_forecast_schedule,
            commands::forecasting::set_forecast_schedule,
            commands::accounts::create_account,
            commands::accounts::get_account,
            commands::accounts::list_active_accounts,
            commands::accounts::update_account,
            commands::accounts::archive_account,
            commands::accounts::reactivate_account,
            commands::accounts::list_archived_accounts,
            commands::accounts::get_account_balance,
            commands::attachments::list_transaction_attachments,
            commands::attachments::add_transaction_attachment,
            commands::attachments::open_transaction_attachment,
            commands::attachments::remove_transaction_attachment,
            commands::transactions::create_transaction,
            commands::transactions::update_transaction,
            commands::transactions::void_transaction,
            commands::transactions::reverse_transaction,
            commands::transactions::create_transfer,
            commands::imports::create_import_batch,
            commands::imports::get_import_source_file,
            commands::imports::list_import_batches,
            commands::imports::discard_import_batch,
            commands::imports::compare_import_revision,
            commands::imports::list_import_revision_changes,
            commands::imports::set_import_revision_resolution,
            commands::imports::stage_csv_import,
            commands::imports::csv_headers,
            commands::imports::csv_rows,
            commands::imports::xlsx_sheet_names,
            commands::imports::xlsx_headers,
            commands::imports::xlsx_rows,
            commands::imports::stage_xlsx_import,
            commands::imports::stage_pdf_extracted_text,
            commands::imports::correct_import_record,
            commands::imports::review_import_record,
            commands::imports::finalize_import_batch,
            commands::imports::list_staged_import_records,
            commands::imports::list_import_logs,
            commands::imports::get_import_reconciliation,
            commands::imports::reconcile_import_batch,
            commands::imports::set_import_record_allocations,
            commands::planning::list_budget_templates,
            commands::planning::list_budget_template_items,
            commands::planning::update_budget_template,
            commands::planning::create_budget_template,
            commands::planning::create_budget_period,
            commands::planning::list_budget_periods,
            commands::planning::get_budget_period_items,
            commands::planning::get_budget_period_totals,
            commands::planning::update_budget_period_items,
            commands::planning::list_savings_goals,
            commands::planning::list_savings_contributions,
            commands::planning::create_savings_goal,
            commands::planning::add_savings_contribution,
            commands::planning::set_savings_goal_status,
            commands::planning::list_eligible_savings_transfers,
            commands::planning::add_savings_transfer,
            commands::planning::list_subscriptions,
            commands::planning::create_subscription,
            commands::planning::update_subscription,
            commands::planning::set_subscription_status,
            commands::planning::list_eligible_subscription_payments,
            commands::planning::link_subscription_payment,
            commands::transactions::list_transactions,
            commands::transactions::search_transactions,
            commands::transactions::list_active_categories,
            commands::transactions::create_category,
            commands::transactions::archive_category,
            commands::transactions::get_transaction_audit_history,
            commands::transactions::get_transaction_allocations,
            commands::transactions::list_eligible_refunds,
        ])
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;

            std::fs::create_dir_all(&app_data_dir)?;

            let database_path = app_data_dir.join("etracker.db");
            let database = Database::initialize(&database_path)?;

            let _ = crate::persistence::forecasting::SqliteForecastRepository::new(&database)
                .generate_scheduled_if_due();

            app.manage(database);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ETracker application");
}
