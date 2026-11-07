use crate::infrastructure::database::Database;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IntegrityReport {
    pub integrity_messages: Vec<String>,
    pub foreign_key_violations: Vec<String>,
}

pub struct SqliteIntegrityRepository<'a> {
    database: &'a Database,
}

impl<'a> SqliteIntegrityRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn check(&self) -> Result<IntegrityReport, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let mut integrity = connection
            .prepare("PRAGMA integrity_check")
            .map_err(|e| e.to_string())?;
        let integrity_messages = integrity
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let mut foreign_keys = connection
            .prepare("PRAGMA foreign_key_check")
            .map_err(|e| e.to_string())?;
        let foreign_key_violations = foreign_keys
            .query_map([], |row| {
                let table: String = row.get(0)?;
                let rowid: Option<i64> = row.get(1)?;
                let parent: String = row.get(2)?;
                let index: i64 = row.get(3)?;
                Ok(format!(
                    "{table} row {rowid:?} references {parent} (constraint {index})"
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(IntegrityReport {
            integrity_messages,
            foreign_key_violations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn reports_database_and_foreign_key_health() {
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        let report = SqliteIntegrityRepository::new(&database).check().unwrap();
        assert_eq!(report.integrity_messages, vec!["ok"]);
        assert!(report.foreign_key_violations.is_empty());
    }
}
