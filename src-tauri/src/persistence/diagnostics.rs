use crate::infrastructure::database::{Database, LATEST_SCHEMA_VERSION};
use rusqlite::OptionalExtension;
use serde::Serialize;

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

    Ok(DiagnosticReport {
        app_version: env!("CARGO_PKG_VERSION"),
        operating_system: std::env::consts::OS,
        architecture: std::env::consts::ARCH,
        sqlite_version,
        schema_version,
        latest_supported_schema_version: LATEST_SCHEMA_VERSION,
        database_integrity,
        foreign_key_check,
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
}
