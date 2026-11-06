use crate::{
    infrastructure::database::Database,
    persistence::analytics::{AnalyticsSummary, SqliteAnalyticsRepository},
};
use tauri::State;

#[tauri::command]
pub fn get_analytics_summary(
    state: State<'_, Database>,
    from: Option<String>,
    to: Option<String>,
    account_id: Option<String>,
    category_id: Option<String>,
    currency_code: Option<String>,
) -> Result<AnalyticsSummary, String> {
    SqliteAnalyticsRepository::new(state.inner()).summarize(
        from.as_deref(),
        to.as_deref(),
        account_id.as_deref(),
        category_id.as_deref(),
        currency_code.as_deref(),
    )
}
