use crate::{
    infrastructure::database::Database,
    persistence::diagnostics::{DiagnosticReport, create_diagnostic_report},
};
use tauri::State;

#[tauri::command]
pub fn get_diagnostic_report(state: State<'_, Database>) -> Result<DiagnosticReport, String> {
    create_diagnostic_report(state.inner())
}
