use crate::{
    infrastructure::database::Database,
    persistence::backup::{create_encrypted_backup, restore_encrypted_backup as restore_backup},
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
    create_encrypted_backup(state.inner(), &app_data_dir(&app)?, &passphrase)
}

#[tauri::command]
pub fn restore_encrypted_backup(
    state: State<'_, Database>,
    app: AppHandle,
    backup_base64: String,
    passphrase: String,
) -> Result<(), String> {
    restore_backup(
        state.inner(),
        &app_data_dir(&app)?,
        &backup_base64,
        &passphrase,
    )
}
