use crate::{
    application::imports::{CsvImportMapping, ImportService, NewImportBatch},
    infrastructure::database::Database,
    persistence::imports::{ImportBatchRecord, SqliteImportRepository, StagedImportRecord},
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
    SqliteImportRepository::new(state.inner())
        .stage_csv(&request.batch_id, &request.contents, &mapping)
        .map_err(|_| {
            "CSV rows could not be staged. Check the selected columns and file format.".into()
        })
}
