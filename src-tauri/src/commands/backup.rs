use crate::{
    infrastructure::database::Database,
    persistence::backup::{create_encrypted_backup, restore_encrypted_backup as restore_backup},
    persistence::diagnostics::{DiagnosticEventCode, record_diagnostic_event},
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|_| "Application storage is unavailable.".into())
}

#[tauri::command]
pub fn export_encrypted_backup(
    state: State<'_, Database>,
    app: AppHandle,
    passphrase: String,
) -> Result<String, String> {
    let result = create_encrypted_backup(state.inner(), &app_data_dir(&app)?, &passphrase);
    record_event(
        state.inner(),
        if result.is_ok() {
            DiagnosticEventCode::BackupCreated
        } else {
            DiagnosticEventCode::BackupFailed
        },
    );
    result
}

#[tauri::command]
pub fn restore_encrypted_backup(
    state: State<'_, Database>,
    app: AppHandle,
    backup_base64: String,
    passphrase: String,
) -> Result<(), String> {
    let result = restore_backup(
        state.inner(),
        &app_data_dir(&app)?,
        &backup_base64,
        &passphrase,
    );
    record_event(
        state.inner(),
        if result.is_ok() {
            DiagnosticEventCode::BackupRestored
        } else {
            DiagnosticEventCode::RestoreFailed
        },
    );
    result
}

fn record_event(database: &Database, event: DiagnosticEventCode) {
    if let Err(error) = record_diagnostic_event(database, event) {
        eprintln!("A redacted backup event could not be recorded: {error}");
    }
}
