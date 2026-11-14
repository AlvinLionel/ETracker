use crate::{infrastructure::database::Database, persistence::reset::reset_all_user_data};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub fn reset_all_data(state: State<'_, Database>, app: AppHandle) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "Application storage is unavailable.")?;
    reset_all_user_data(state.inner(), &app_data_dir)
}
