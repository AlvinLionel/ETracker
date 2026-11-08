use crate::{
    application::imports::{
        CsvImportMapping, ImportError, ImportService, NewImportBatch, StagedCategoryAllocation,
    },
    infrastructure::database::Database,
    persistence::imports::{
        ImportBatchRecord, ImportLogRecord, ImportRevisionChangeRecord, SqliteImportRepository,
        StagedImportRecord,
    },
};
use serde::Deserialize;
use tauri::State;
#[derive(Deserialize)]
pub struct CreateImportBatchRequest {
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub revision_of_batch_id: Option<String>,
    pub source_file_contents: Vec<u8>,
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
pub struct StageXlsxRequest {
    pub batch_id: String,
    pub contents: Vec<u8>,
    pub sheet_name: String,
    pub header_row: usize,
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
#[derive(Deserialize)]
pub struct SetImportRevisionResolutionRequest {
    pub change_id: String,
    pub resolution: String,
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
pub fn csv_rows(contents: String) -> Result<Vec<Vec<String>>, String> {
    if contents.len() > crate::application::imports::MAX_SOURCE_FILE_BYTES {
        return Err("CSV and spreadsheet files must be 25 MiB or smaller.".into());
    }
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(contents.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| "The CSV file does not contain a readable header row.".to_string())?
        .iter()
        .map(str::to_owned)
        .collect();
    let mut rows = vec![headers];
    rows.extend(
        reader
            .records()
            .map(|record| {
                record
                    .map(|row| row.iter().map(str::to_owned).collect())
                    .map_err(|_| "The CSV file contains an unreadable row.".to_string())
            })
            .collect::<Result<Vec<Vec<String>>, String>>()?,
    );
    Ok(rows)
}
#[tauri::command]
pub fn xlsx_sheet_names(
    state: State<'_, Database>,
    contents: Vec<u8>,
) -> Result<Vec<String>, String> {
    SqliteImportRepository::new(state.inner())
        .xlsx_sheet_names(contents)
        .map_err(|error| match error {
            ImportError::SourceFileTooLarge => {
                "CSV and spreadsheet files must be 25 MiB or smaller.".into()
            }
            ImportError::WorkbookTooLarge => {
                "The workbook expands beyond the 100 MiB processing limit.".into()
            }
            _ => "The workbook could not be read. Choose an unencrypted XLSX workbook.".into(),
        })
}
#[tauri::command]
pub fn xlsx_headers(
    state: State<'_, Database>,
    contents: Vec<u8>,
    sheet_name: String,
    header_row: usize,
) -> Result<Vec<String>, String> {
    SqliteImportRepository::new(state.inner())
        .xlsx_headers(contents, &sheet_name, header_row)
        .map_err(|error| match error {
            ImportError::WorkbookTooLarge => {
                "The workbook expands beyond the 100 MiB processing limit.".into()
            }
            _ => "The selected worksheet header row could not be read.".into(),
        })
}
#[tauri::command]
pub fn xlsx_rows(
    state: State<'_, Database>,
    contents: Vec<u8>,
    sheet_name: String,
    header_row: usize,
) -> Result<Vec<Vec<String>>, String> {
    SqliteImportRepository::new(state.inner())
        .xlsx_rows(contents, &sheet_name, header_row)
        .map_err(|error| match error {
            ImportError::SourceFileTooLarge => {
                "CSV and spreadsheet files must be 25 MiB or smaller.".into()
            }
            ImportError::WorkbookTooLarge => {
                "The workbook expands beyond the 100 MiB processing limit.".into()
            }
            _ => "Rows could not be read from the selected worksheet.".into(),
        })
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
            revision_of_batch_id: request.revision_of_batch_id,
            source_file_contents: request.source_file_contents,
        })
        .map_err(|error| match error {
            ImportError::SourceFileTooLarge => "Statement files must be 25 MiB or smaller.".into(),
            _ => "Import batch could not be created. Check the account and file details.".into(),
        })
}
#[tauri::command]
pub fn get_import_source_file(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<Vec<u8>, String> {
    SqliteImportRepository::new(state.inner())
        .source_file_contents(&batch_id)
        .map_err(|_| "The retained source file is unavailable or past its retention period.".into())
}
#[tauri::command]
pub fn list_import_batches(state: State<'_, Database>) -> Result<Vec<ImportBatchRecord>, String> {
    SqliteImportRepository::new(state.inner())
        .list_batches()
        .map_err(|_| "Import batches could not be loaded.".into())
}
#[tauri::command]
pub fn discard_import_batch(state: State<'_, Database>, batch_id: String) -> Result<(), String> {
    SqliteImportRepository::new(state.inner())
        .discard_batch(&batch_id)
        .map_err(|_| "Only an unfinished import workspace can be discarded.".into())
}
#[tauri::command]
pub fn compare_import_revision(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<Vec<ImportRevisionChangeRecord>, String> {
    SqliteImportRepository::new(state.inner())
        .compare_revision(&batch_id)
        .map_err(|_| "The selected statements could not be compared.".into())
}
#[tauri::command]
pub fn list_import_revision_changes(
    state: State<'_, Database>,
    batch_id: String,
) -> Result<Vec<ImportRevisionChangeRecord>, String> {
    SqliteImportRepository::new(state.inner())
        .list_revision_changes(&batch_id)
        .map_err(|_| "Statement comparison could not be loaded.".into())
}
#[tauri::command]
pub fn set_import_revision_resolution(
    state: State<'_, Database>,
    request: SetImportRevisionResolutionRequest,
) -> Result<(), String> {
    SqliteImportRepository::new(state.inner())
        .set_revision_resolution(&request.change_id, &request.resolution)
        .map_err(|_| "That revised-statement action is not compatible with this change.".into())
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
#[tauri::command]
pub fn stage_xlsx_import(
    state: State<'_, Database>,
    request: StageXlsxRequest,
) -> Result<usize, String> {
    let mapping = CsvImportMapping {
        date_column: request.date_column,
        description_column: request.description_column,
        amount_column: request.amount_column,
        positive_amount_type: request.positive_amount_type,
        date_format: request.date_format,
    };
    let repository = SqliteImportRepository::new(state.inner());
    match repository.stage_xlsx(
        &request.batch_id,
        request.contents,
        &request.sheet_name,
        request.header_row,
        &mapping,
    ) {
        Ok(count) => Ok(count),
        Err(ImportError::WorkbookTooLarge) => {
            Err("The workbook expands beyond the 100 MiB processing limit.".into())
        }
        Err(_) => {
            let _ = ImportService::new(repository).record_failure(
                &request.batch_id,
                None,
                "parse",
                "Workbook parsing or validation failed",
                "Check the worksheet, header row, mapped columns and workbook structure.",
            );
            Err(
                "Workbook rows could not be staged. Check the selected worksheet and columns."
                    .into(),
            )
        }
    }
}
#[tauri::command]
pub fn stage_pdf_extracted_text(
    state: State<'_, Database>,
    batch_id: String,
    pages: Vec<String>,
) -> Result<usize, String> {
    let repository = SqliteImportRepository::new(state.inner());
    match repository.stage_pdf_pages(&batch_id, pages) {
        Ok(count) => Ok(count),
        Err(ImportError::PdfNoExtractableText) => {
            let _ = ImportService::new(repository).record_failure(
                &batch_id,
                None,
                "parse",
                "PDF was unlocked but no selectable text could be extracted",
                "Try another PDF export or download the retained source file. Scanned-PDF OCR is not available.",
            );
            Err("The PDF was unlocked, but no selectable transaction text could be extracted. The source file is still available for retry or download.".into())
        }
        Err(ImportError::ExtractedTextTooLarge) => {
            Err("The extracted PDF text exceeds the 100 MiB processing limit.".into())
        }
        Err(_) => {
            Err("PDF text could not be staged. The original source file is still available.".into())
        }
    }
}
