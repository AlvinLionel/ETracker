use crate::{
    infrastructure::database::Database,
    persistence::search::{SearchResult, search},
};
use tauri::State;

#[tauri::command]
pub fn search_ledger(
    state: State<'_, Database>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    search(state.inner(), &query, limit.unwrap_or(12))
}
