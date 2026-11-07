use crate::{
    infrastructure::database::Database,
    persistence::forecasting::{
        ForecastRequest, ForecastSchedule, ForecastSnapshot, SqliteForecastRepository,
    },
};
use tauri::State;

#[tauri::command]
pub fn generate_forecast(
    state: State<'_, Database>,
    request: ForecastRequest,
) -> Result<String, String> {
    SqliteForecastRepository::new(state.inner()).generate(&request)
}

#[tauri::command]
pub fn list_forecasts(state: State<'_, Database>) -> Result<Vec<ForecastSnapshot>, String> {
    SqliteForecastRepository::new(state.inner()).list()
}

#[tauri::command]
pub fn evaluate_forecast(
    state: State<'_, Database>,
    snapshot_id: String,
    month_start: String,
) -> Result<(), String> {
    SqliteForecastRepository::new(state.inner()).evaluate(&snapshot_id, &month_start)
}

#[tauri::command]
pub fn get_forecast_schedule(state: State<'_, Database>) -> Result<ForecastSchedule, String> {
    SqliteForecastRepository::new(state.inner()).get_schedule()
}

#[tauri::command]
pub fn set_forecast_schedule(
    state: State<'_, Database>,
    schedule: ForecastSchedule,
) -> Result<bool, String> {
    SqliteForecastRepository::new(state.inner()).set_schedule(&schedule)
}
