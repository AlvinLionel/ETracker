use crate::infrastructure::database::Database;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserPreferences {
    pub subscription_reminders_enabled: bool,
    pub subscription_reminder_days: u8,
    pub theme: String,
}

pub struct SqlitePreferencesRepository<'a> {
    database: &'a Database,
}

impl<'a> SqlitePreferencesRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn get(&self) -> Result<UserPreferences, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let enabled: String = connection.query_row(
            "SELECT preference_value FROM user_preferences WHERE preference_key='subscription_reminders_enabled'",
            [], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let days: String = connection.query_row(
            "SELECT preference_value FROM user_preferences WHERE preference_key='subscription_reminder_days'",
            [], |row| row.get(0),
        ).map_err(|e| e.to_string())?;
        let theme: String = connection
            .query_row(
                "SELECT preference_value FROM user_preferences WHERE preference_key='theme'",
                [],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !["system", "light", "dark"].contains(&theme.as_str()) {
            return Err("Stored theme preference is invalid.".into());
        }
        Ok(UserPreferences {
            subscription_reminders_enabled: enabled
                .parse()
                .map_err(|_| "Stored reminder preference is invalid")?,
            subscription_reminder_days: days
                .parse()
                .map_err(|_| "Stored reminder window is invalid")?,
            theme,
        })
    }

    pub fn set(&self, preferences: &UserPreferences) -> Result<(), String> {
        if !(1..=30).contains(&preferences.subscription_reminder_days) {
            return Err("Subscription reminder window must be between 1 and 30 days.".into());
        }
        if !["system", "light", "dark"].contains(&preferences.theme.as_str()) {
            return Err("Theme must be system, light, or dark.".into());
        }
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let transaction = connection.transaction().map_err(|e| e.to_string())?;
        for (key, value) in [
            (
                "subscription_reminders_enabled",
                preferences.subscription_reminders_enabled.to_string(),
            ),
            (
                "subscription_reminder_days",
                preferences.subscription_reminder_days.to_string(),
            ),
            ("theme", preferences.theme.clone()),
        ] {
            transaction.execute(
                "INSERT INTO user_preferences(preference_key,preference_value,updated_at) VALUES (?1,?2,CURRENT_TIMESTAMP) ON CONFLICT(preference_key) DO UPDATE SET preference_value=excluded.preference_value,updated_at=CURRENT_TIMESTAMP",
                params![key, value],
            ).map_err(|e| e.to_string())?;
        }
        transaction.commit().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn defaults_are_persisted_and_invalid_windows_are_rejected() {
        let db = Database::initialize(Path::new(":memory:")).unwrap();
        let repository = SqlitePreferencesRepository::new(&db);
        assert_eq!(
            repository.get().unwrap(),
            UserPreferences {
                subscription_reminders_enabled: true,
                subscription_reminder_days: 7,
                theme: "system".into()
            }
        );
        assert!(
            repository
                .set(&UserPreferences {
                    subscription_reminders_enabled: false,
                    subscription_reminder_days: 0,
                    theme: "dark".into()
                })
                .is_err()
        );
        let updated = UserPreferences {
            subscription_reminders_enabled: false,
            subscription_reminder_days: 14,
            theme: "dark".into(),
        };
        repository.set(&updated).unwrap();
        assert_eq!(repository.get().unwrap(), updated);
    }
}
