use crate::infrastructure::database::Database;
use chrono::NaiveDate;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedAnalyticsReport {
    pub id: String,
    pub name: String,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub currency_code: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveAnalyticsReportRequest {
    pub name: String,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub currency_code: Option<String>,
}

pub struct SqliteSavedReportsRepository<'a> {
    database: &'a Database,
}

impl<'a> SqliteSavedReportsRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn list(&self) -> Result<Vec<SavedAnalyticsReport>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let mut statement = connection.prepare(
            "SELECT id,name,from_date,to_date,account_id,category_id,currency_code,created_at,updated_at FROM saved_analytics_reports ORDER BY updated_at DESC,name COLLATE NOCASE",
        ).map_err(|e| e.to_string())?;
        statement
            .query_map([], map_report)
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn save(
        &self,
        request: &SaveAnalyticsReportRequest,
    ) -> Result<SavedAnalyticsReport, String> {
        let name = request.name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err("Saved report names must contain between 1 and 64 characters.".into());
        }
        for date in [&request.from_date, &request.to_date].into_iter().flatten() {
            NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| "Report dates must use the YYYY-MM-DD format.")?;
        }
        if matches!((&request.from_date, &request.to_date), (Some(from), Some(to)) if from > to) {
            return Err("The report start date must be on or before its end date.".into());
        }
        let currency = request
            .currency_code
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        if currency.is_some_and(|value| {
            value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase())
        }) {
            return Err("Report currency must be a three-letter uppercase code.".into());
        }

        let id = uuid::Uuid::new_v4().to_string();
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        connection.execute(
            "INSERT INTO saved_analytics_reports(id,name,from_date,to_date,account_id,category_id,currency_code) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, name, request.from_date, request.to_date, request.account_id, request.category_id, currency],
        ).map_err(|error| {
            if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) && error.to_string().contains("saved_analytics_reports.name") {
                "A saved report with this name already exists.".to_string()
            } else { error.to_string() }
        })?;
        drop(connection);
        self.get(&id)?
            .ok_or_else(|| "The saved report could not be retrieved.".into())
    }

    pub fn delete(&self, id: &str) -> Result<bool, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        connection
            .execute("DELETE FROM saved_analytics_reports WHERE id=?1", [id])
            .map(|count| count == 1)
            .map_err(|e| e.to_string())
    }

    fn get(&self, id: &str) -> Result<Option<SavedAnalyticsReport>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        connection.query_row(
            "SELECT id,name,from_date,to_date,account_id,category_id,currency_code,created_at,updated_at FROM saved_analytics_reports WHERE id=?1",
            [id], map_report,
        ).optional().map_err(|e| e.to_string())
    }
}

fn map_report(row: &rusqlite::Row<'_>) -> rusqlite::Result<SavedAnalyticsReport> {
    Ok(SavedAnalyticsReport {
        id: row.get(0)?,
        name: row.get(1)?,
        from_date: row.get(2)?,
        to_date: row.get(3)?,
        account_id: row.get(4)?,
        category_id: row.get(5)?,
        currency_code: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn saves_lists_and_deletes_a_named_filtered_report() {
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        let repository = SqliteSavedReportsRepository::new(&database);
        let saved = repository
            .save(&SaveAnalyticsReportRequest {
                name: "  Monthly food review ".into(),
                from_date: Some("2026-10-01".into()),
                to_date: Some("2026-10-31".into()),
                account_id: None,
                category_id: None,
                currency_code: Some("KES".into()),
            })
            .unwrap();
        assert_eq!(saved.name, "Monthly food review");
        assert_eq!(repository.list().unwrap(), vec![saved.clone()]);
        assert!(repository.delete(&saved.id).unwrap());
        assert!(repository.list().unwrap().is_empty());
        assert!(!repository.delete(&saved.id).unwrap());
    }

    #[test]
    fn rejects_invalid_or_duplicate_report_definitions() {
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        let repository = SqliteSavedReportsRepository::new(&database);
        let base = SaveAnalyticsReportRequest {
            name: "Monthly report".into(),
            from_date: Some("2026-11-31".into()),
            to_date: None,
            account_id: None,
            category_id: None,
            currency_code: None,
        };
        assert!(repository.save(&base).unwrap_err().contains("YYYY-MM-DD"));
        let reversed = SaveAnalyticsReportRequest {
            from_date: Some("2026-11-02".into()),
            to_date: Some("2026-11-01".into()),
            ..base.clone()
        };
        assert!(
            repository
                .save(&reversed)
                .unwrap_err()
                .contains("on or before")
        );
        let valid = SaveAnalyticsReportRequest {
            from_date: None,
            to_date: None,
            ..base.clone()
        };
        repository.save(&valid).unwrap();
        let duplicate = SaveAnalyticsReportRequest {
            name: "monthly REPORT".into(),
            ..valid
        };
        assert!(
            repository
                .save(&duplicate)
                .unwrap_err()
                .contains("already exists")
        );
    }
}
