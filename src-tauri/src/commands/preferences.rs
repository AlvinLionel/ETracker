use crate::{
    infrastructure::database::Database,
    persistence::preferences::{SqlitePreferencesRepository, UserPreferences},
};
use tauri::State;

#[tauri::command]
pub fn get_user_preferences(state: State<'_, Database>) -> Result<UserPreferences, String> {
    SqlitePreferencesRepository::new(state.inner()).get()
}

#[tauri::command]
pub fn set_user_preferences(
    state: State<'_, Database>,
    preferences: UserPreferences,
) -> Result<(), String> {
    SqlitePreferencesRepository::new(state.inner()).set(&preferences)
}
