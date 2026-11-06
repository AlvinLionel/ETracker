use crate::{
    application::imports::{
        CsvImportMapping, ImportService, NewImportBatch, StagedCategoryAllocation,
    },
    infrastructure::database::Database,
    persistence::imports::{
        ImportBatchRecord, ImportLogRecord, SqliteImportRepository, StagedImportRecord,
    },
};
use serde::Deserialize;
use tauri::State;
#[derive(Deserialize)]
pub struct CreateImportBatchRequest {
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub source_file_hash: Option<String>,
}
#[derive(Deserialize)]
pub struct StageCsvRequest {
    pub batch_id: String,
    pub contents: String,
    pub date_column: String,
    pub description_column: Option<String>,
    pub amount_column: String,
    pub positive_amount_type: String,
    pub date_format: String,
}
#[derive(Deserialize)]
pub struct CorrectImportRecordRequest {
    pub record_id: String,
    pub normalized_date: String,
    pub amount_minor: i64,
    pub transaction_type: String,
    pub description: Option<String>,
}
#[derive(Deserialize)]
pub struct ReconcileImportRequest {
    pub batch_id: String,
    pub opening_balance_minor: Option<i64>,
    pub closing_balance_minor: Option<i64>,
}
#[derive(Deserialize)]
pub struct SetImportAllocationsRequest {
    pub record_id: String,
    pub allocations: Vec<StagedCategoryAllocation>,
}
#[tauri::command]
pub fn correct_import_record(
    state: State<'_, Database>,
    request: CorrectImportRecordRequest,
) -> Result<(), String> {
    let service = ImportService::new(SqliteImportRepository::new(state.inner()));
    service
        .correct_record(
            &request.record_id,
            &request.normalized_date,
            request.amount_minor,
            &request.transaction_type,
            request.description.as_deref(),
        )
        .map_err(|_| "The staged record could not be corrected.".into())
}
#[tauri::command]
pub fn review_import_record(
    state: State<'_, Database>,
    record_id: String,
    keep: bool,
) -> Result<(), String> {
    ImportService::new(SqliteImportRepository::new(state.inner()))
        .review_record(&record_id, keep)
        .map_err(|_| "The staged record could not be reviewed.".into())
}
#[tauri::command]
pub fn finalize_import_batch(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<usize, String> {
    let service = ImportService::new(SqliteImportRepository::new(state.inner()));
    match service.finalize(&batch_id) {
        Ok(count) => Ok(count),
        Err(_) => {
            let _ = service.record_failure(
                &batch_id,
                None,
                "finalize",
                "Import finalization was blocked or rolled back",
                "Resolve validation, duplicate review and reconciliation issues, then try again.",
            );
            Err("This import cannot be finalized. Resolve invalid, uncertain, duplicate, or reconciliation issues first.".into())
        }
    }
}
#[tauri::command]
pub fn csv_headers(contents: String) -> Result<Vec<String>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(false)
        .from_reader(contents.as_bytes());
    reader
        .headers()
        .map(|h| h.iter().map(str::to_owned).collect())
        .map_err(|_| "The file does not contain a readable CSV header row.".into())
}
#[tauri::command]
pub fn create_import_batch(
    state: State<'_, Database>,
    request: CreateImportBatchRequest,
) -> Result<String, String> {
    let repo = SqliteImportRepository::new(state.inner());
    ImportService::new(repo)
        .create_batch(&NewImportBatch {
            account_id: request.account_id,
            source_file_name: request.source_file_name,
            source_format: request.source_format,
            source_file_hash: request.source_file_hash,
        })
        .map_err(|_| {
            "Import batch could not be created. Check the account and file details.".into()
        })
}
#[tauri::command]
pub fn list_import_batches(state: State<'_, Database>) -> Result<Vec<ImportBatchRecord>, String> {
    SqliteImportRepository::new(state.inner())
        .list_batches()
        .map_err(|_| "Import batches could not be loaded.".into())
}
#[tauri::command]
pub fn list_staged_import_records(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<Vec<StagedImportRecord>, String> {
    SqliteImportRepository::new(state.inner())
        .list_records(&batch_id)
        .map_err(|_| "Staged records could not be loaded.".into())
}
#[tauri::command]
pub fn list_import_logs(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<Vec<ImportLogRecord>, String> {
    SqliteImportRepository::new(state.inner())
        .list_logs(&batch_id)
        .map_err(|_| "Import logs could not be loaded.".into())
}
#[tauri::command]
pub fn get_import_reconciliation(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<crate::application::imports::ImportReconciliation, String> {
    SqliteImportRepository::new(state.inner())
        .get_reconciliation(&batch_id)
        .map_err(|_| "Reconciliation could not be loaded.".into())
}
#[tauri::command]
pub fn reconcile_import_batch(
    state: State<'_, Database>,
    request: ReconcileImportRequest,
) -> Result<crate::application::imports::ImportReconciliation, String> {
    let service = ImportService::new(SqliteImportRepository::new(state.inner()));
    service.reconcile(&request.batch_id, request.opening_balance_minor, request.closing_balance_minor).map_err(|_| "Reconciliation could not be completed. Ensure all staged rows have valid dates, amounts and types.".into())
}
#[tauri::command]
pub fn set_import_record_allocations(
    state: State<'_, Database>,
    request: SetImportAllocationsRequest,
) -> Result<(), String> {
    ImportService::new(SqliteImportRepository::new(state.inner())).set_record_allocations(&request.record_id, &request.allocations).map_err(|_| "Category allocations must use active matching categories and add up to the full transaction amount.".into())
}
#[tauri::command]
pub fn stage_csv_import(
    state: State<'_, Database>,
    request: StageCsvRequest,
) -> Result<usize, String> {
    let mapping = CsvImportMapping {
        date_column: request.date_column,
        description_column: request.description_column,
        amount_column: request.amount_column,
        positive_amount_type: request.positive_amount_type,
        date_format: request.date_format,
    };
    let repository = SqliteImportRepository::new(state.inner());
    match repository.stage_csv(&request.batch_id, &request.contents, &mapping) {
        Ok(count) => Ok(count),
        Err(_) => {
            let _ = ImportService::new(repository).record_failure(
                &request.batch_id,
                None,
                "parse",
                "CSV parsing or validation failed",
                "Check the mapped columns, date format, and CSV structure.",
            );
            Err("CSV rows could not be staged. Check the selected columns and file format.".into())
        }
    }
}
