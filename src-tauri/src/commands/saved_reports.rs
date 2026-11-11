use crate::{
    infrastructure::database::Database,
    persistence::saved_reports::{
        SaveAnalyticsReportRequest, SavedAnalyticsReport, SqliteSavedReportsRepository,
    },
};
use tauri::State;

#[tauri::command]
pub fn list_saved_analytics_reports(
    state: State<'_, Database>,
) -> Result<Vec<SavedAnalyticsReport>, String> {
    SqliteSavedReportsRepository::new(state.inner()).list()
}

#[tauri::command]
pub fn save_analytics_report(
    state: State<'_, Database>,
    request: SaveAnalyticsReportRequest,
) -> Result<SavedAnalyticsReport, String> {
    SqliteSavedReportsRepository::new(state.inner()).save(&request)
}

#[tauri::command]
pub fn delete_saved_analytics_report(
    state: State<'_, Database>,
    report_id: String,
) -> Result<bool, String> {
    SqliteSavedReportsRepository::new(state.inner()).delete(&report_id)
}
