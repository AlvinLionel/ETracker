use crate::{
    infrastructure::database::Database,
    persistence::attachments::{AttachmentRecord, SqliteAttachmentRepository},
};
use std::path::PathBuf;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|_| "Application storage is unavailable.".into())
}

#[tauri::command]
pub fn list_transaction_attachments(
    state: State<'_, Database>,
    app: AppHandle,
    transaction_id: String,
) -> Result<Vec<AttachmentRecord>, String> {
    SqliteAttachmentRepository::new(state.inner(), &app_data_dir(&app)?).list(&transaction_id)
}

#[tauri::command]
pub async fn add_transaction_attachment(
    state: State<'_, Database>,
    app: AppHandle,
    transaction_id: String,
) -> Result<Option<AttachmentRecord>, String> {
    let selected = app
        .dialog()
        .file()
        .add_filter("Receipt or document", &["pdf", "jpg", "jpeg", "png"])
        .blocking_pick_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    let source = selected
        .into_path()
        .map_err(|_| "The selected file path is unavailable.")?;
    let record = SqliteAttachmentRepository::new(state.inner(), &app_data_dir(&app)?)
        .add(&transaction_id, &source)?;
    Ok(Some(record))
}

#[tauri::command]
pub fn open_transaction_attachment(
    state: State<'_, Database>,
    app: AppHandle,
    attachment_id: String,
) -> Result<(), String> {
    let path = SqliteAttachmentRepository::new(state.inner(), &app_data_dir(&app)?)
        .file_path(&attachment_id)?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|_| "The attachment could not be opened.".into())
}

#[tauri::command]
pub fn remove_transaction_attachment(
    state: State<'_, Database>,
    app: AppHandle,
    attachment_id: String,
) -> Result<(), String> {
    SqliteAttachmentRepository::new(state.inner(), &app_data_dir(&app)?).remove(&attachment_id)
}
