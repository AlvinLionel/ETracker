use crate::{
    application::imports::{ImportService, NewImportBatch},
    infrastructure::database::Database,
    persistence::imports::{ImportBatchRecord, SqliteImportRepository},
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
