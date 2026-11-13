use crate::infrastructure::database::{Database, LATEST_SCHEMA_VERSION};
use rusqlite::OptionalExtension;
use serde::Serialize;
use uuid::Uuid;

const EVENT_RETENTION_DAYS: u8 = 30;
const MAX_REPORTED_EVENTS: i64 = 100;

#[derive(Clone, Copy)]
pub enum DiagnosticEventCode {
    AppStarted,
    AttachmentStorageReady,
    BackupCreated,
    BackupFailed,
    BackupRestored,
    RestoreFailed,
}

impl DiagnosticEventCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::AppStarted => "app_started",
            Self::AttachmentStorageReady => "attachment_storage_ready",
            Self::BackupCreated => "backup_created",
            Self::BackupFailed => "backup_failed",
            Self::BackupRestored => "backup_restored",
            Self::RestoreFailed => "restore_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagnosticEvent {
    pub event_code: String,
    pub occurred_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagnosticReport {
    pub app_version: &'static str,
    pub operating_system: &'static str,
    pub architecture: &'static str,
    pub sqlite_version: String,
    pub schema_version: i64,
    pub latest_supported_schema_version: i64,
    pub database_integrity: &'static str,
    pub foreign_key_check: &'static str,
    pub diagnostic_retention_days: u8,
    pub recent_events: Vec<DiagnosticEvent>,
}

pub fn record_diagnostic_event(
    database: &Database,
    event_code: DiagnosticEventCode,
) -> Result<(), String> {
    let connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    connection
        .execute(
            "INSERT INTO diagnostic_events(id,event_code) VALUES (?1,?2)",
            rusqlite::params![Uuid::new_v4().to_string(), event_code.as_str()],
        )
        .map_err(|error| error.to_string())?;
    connection
        .execute(
            "DELETE FROM diagnostic_events WHERE occurred_at < datetime('now','-30 days')",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn cleanup_expired_diagnostic_events(database: &Database) -> Result<(), String> {
    let connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    connection
        .execute(
            "DELETE FROM diagnostic_events WHERE occurred_at < datetime('now','-30 days')",
            [],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub fn create_diagnostic_report(database: &Database) -> Result<DiagnosticReport, String> {
    let connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    let sqlite_version: String = connection
        .query_row("SELECT sqlite_version()", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let schema_version: i64 = connection
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get::<_, Option<i64>>(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
        .flatten()
        .ok_or_else(|| "Database schema version is unavailable".to_string())?;
    let database_integrity = if connection
        .query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        .map(|result| result.eq_ignore_ascii_case("ok"))
        .unwrap_or(false)
    {
        "passed"
    } else {
        "issues_found"
    };
    let foreign_key_check = if connection
        .query_row(
            "SELECT 1 FROM pragma_foreign_key_check LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map(|result| result.is_none())
        .unwrap_or(false)
    {
        "passed"
    } else {
        "issues_found"
    };
    connection
        .execute(
            "DELETE FROM diagnostic_events WHERE occurred_at < datetime('now','-30 days')",
            [],
        )
        .map_err(|error| error.to_string())?;
    let recent_events = {
        let mut statement = connection
            .prepare(
                "SELECT event_code,occurred_at FROM diagnostic_events ORDER BY occurred_at DESC,id DESC LIMIT ?1",
            )
            .map_err(|error| error.to_string())?;
        statement
            .query_map([MAX_REPORTED_EVENTS], |row| {
                Ok(DiagnosticEvent {
                    event_code: row.get(0)?,
                    occurred_at: row.get(1)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|error| error.to_string())?
    };

    Ok(DiagnosticReport {
        app_version: env!("CARGO_PKG_VERSION"),
        operating_system: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        sqlite_version,
        schema_version,
        latest_supported_schema_version: LATEST_SCHEMA_VERSION,
        database_integrity,
        foreign_key_check,
        diagnostic_retention_days: EVENT_RETENTION_DAYS,
        recent_events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn diagnostic_report_contains_environment_and_redacted_database_health() {
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        let report = create_diagnostic_report(&database).unwrap();

        assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(report.operating_system, std::env::consts::OS);
        assert_eq!(report.architecture, std::env::consts::ARCH);
        assert!(!report.sqlite_version.is_empty());
        assert_eq!(report.schema_version, LATEST_SCHEMA_VERSION);
        assert_eq!(report.database_integrity, "passed");
        assert_eq!(report.foreign_key_check, "passed");

        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("accounts"));
        assert!(!serialized.contains("transactions"));
        assert!(!serialized.contains("KES"));
    }

    #[test]
    fn diagnostic_report_redacts_foreign_key_failure_details() {
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        {
            let connection = database.connection().lock().unwrap();
            connection
                .pragma_update(None, "foreign_keys", "OFF")
                .unwrap();
            connection.execute(
                "INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('private-row-id','missing-transaction','missing-account',1)",
                [],
            ).unwrap();
        }

        let report = create_diagnostic_report(&database).unwrap();
        assert_eq!(report.database_integrity, "passed");
        assert_eq!(report.foreign_key_check, "issues_found");
        assert!(
            !serde_json::to_string(&report)
                .unwrap()
                .contains("private-row-id")
        );
    }

    #[test]
    fn diagnostic_events_are_allowlisted_retained_and_exported_without_user_data() {
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        {
            let connection = database.connection().lock().unwrap();
            connection
                .execute(
                    "INSERT INTO diagnostic_events(id,event_code,occurred_at) VALUES ('old','backup_created',datetime('now','-31 days'))",
                    [],
                )
                .unwrap();
        }
        record_diagnostic_event(&database, DiagnosticEventCode::BackupCreated).unwrap();
        let report = create_diagnostic_report(&database).unwrap();

        assert_eq!(report.diagnostic_retention_days, 30);
        assert_eq!(report.recent_events.len(), 1);
        assert_eq!(report.recent_events[0].event_code, "backup_created");
        let serialized = serde_json::to_string(&report).unwrap();
        assert!(!serialized.contains("private-row-id"));
        assert!(!serialized.contains("transaction"));
        assert!(
            database
                .connection()
                .lock()
                .unwrap()
                .execute(
                    "INSERT INTO diagnostic_events(id,event_code) VALUES ('invalid','private account name')",
                    [],
                )
                .is_err()
        );
    }
}
