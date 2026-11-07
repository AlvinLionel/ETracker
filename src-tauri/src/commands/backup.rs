use crate::{
    infrastructure::database::Database,
    persistence::backup::{create_encrypted_backup, restore_encrypted_backup as restore_backup},
};
use tauri::State;

#[tauri::command]
pub fn export_encrypted_backup(
    state: State<'_, Database>,
    passphrase: String,
) -> Result<String, String> {
    create_encrypted_backup(state.inner(), &passphrase)
}

#[tauri::command]
pub fn restore_encrypted_backup(
    state: State<'_, Database>,
    backup_base64: String,
    passphrase: String,
) -> Result<(), String> {
    restore_backup(state.inner(), &backup_base64, &passphrase)
}
