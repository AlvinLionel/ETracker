use crate::{
    infrastructure::database::Database,
    persistence::dashboard::{DashboardSummary, SqliteDashboardRepository},
};
use tauri::State;

#[tauri::command]
pub fn get_dashboard_summary(state: State<'_, Database>) -> Result<DashboardSummary, String> {
    SqliteDashboardRepository::new(state.inner()).summary()
}
