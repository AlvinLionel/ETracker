use crate::{
    infrastructure::database::Database,
    persistence::integrity::{IntegrityReport, SqliteIntegrityRepository},
};
use tauri::State;

#[tauri::command]
pub fn check_database_integrity(state: State<'_, Database>) -> Result<IntegrityReport, String> {
    SqliteIntegrityRepository::new(state.inner()).check()
}
