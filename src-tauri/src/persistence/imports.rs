use crate::{
    application::imports::{CsvImportMapping, ImportError, ImportRepository, NewImportBatch},
    infrastructure::database::Database,
};
use rusqlite::{OptionalExtension, params};
use uuid::Uuid;
pub struct SqliteImportRepository<'a> {
    database: &'a Database,
}
impl<'a> SqliteImportRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }
}
impl ImportRepository for SqliteImportRepository<'_> {
    fn create_batch(&self, batch: &NewImportBatch) -> Result<String, ImportError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let active: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL)",
                [&batch.account_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !active {
            return Err(ImportError::InvalidBatch);
        }
        let id = Uuid::new_v4().to_string();
        connection.execute("INSERT INTO import_batches(id,account_id,source_file_name,source_file_hash,source_format) VALUES (?1,?2,?3,?4,?5)",params![id,batch.account_id,batch.source_file_name,batch.source_file_hash,batch.source_format]).map_err(|e|ImportError::Database(e.to_string()))?;
        connection.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'staging','INFO','Import batch created')",params![Uuid::new_v4().to_string(),id]).map_err(|e|ImportError::Database(e.to_string()))?;
        Ok(id)
    }
    fn correct_record(
        &self,
        record_id: &str,
        date: &str,
        amount_minor: i64,
        kind: &str,
        description: Option<&str>,
    ) -> Result<(), ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let batch:Option<String>=tx.query_row("SELECT b.id FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=?1 AND b.status='review'",[record_id],|r|r.get(0)).optional().map_err(|e|ImportError::Database(e.to_string()))?;
        let Some(batch) = batch else {
            return Err(ImportError::InvalidBatch);
        };
        tx.execute("UPDATE imported_records SET normalized_date=?1,normalized_amount_minor=?2,transaction_type=?3,normalized_description=?4,validation_status='valid',review_status='corrected' WHERE id=?5",params![date,amount_minor,kind,description,record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message) VALUES (?1,?2,?3,'review','INFO','Staged record corrected')",params![Uuid::new_v4().to_string(),batch,record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
}
#[derive(serde::Serialize)]
pub struct ImportBatchRecord {
    pub id: String,
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub status: String,
    pub reconciliation_status: String,
    pub created_at: String,
}
#[derive(serde::Serialize)]
pub struct StagedImportRecord {
    pub id: String,
    pub source_row_reference: String,
    pub original_date: Option<String>,
    pub normalized_date: Option<String>,
    pub original_description: Option<String>,
    pub original_amount: Option<String>,
    pub normalized_amount_minor: Option<i64>,
    pub transaction_type: Option<String>,
    pub validation_status: String,
    pub review_status: String,
}
impl SqliteImportRepository<'_> {
    pub fn list_records(&self, batch_id: &str) -> Result<Vec<StagedImportRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut s=c.prepare("SELECT id,source_row_reference,original_date,normalized_date,original_description,original_amount,normalized_amount_minor,transaction_type,validation_status,review_status FROM imported_records WHERE batch_id=?1 ORDER BY CAST(source_row_reference AS INTEGER)").map_err(|e|ImportError::Database(e.to_string()))?;
        s.query_map([batch_id], |r| {
            Ok(StagedImportRecord {
                id: r.get(0)?,
                source_row_reference: r.get(1)?,
                original_date: r.get(2)?,
                normalized_date: r.get(3)?,
                original_description: r.get(4)?,
                original_amount: r.get(5)?,
                normalized_amount_minor: r.get(6)?,
                transaction_type: r.get(7)?,
                validation_status: r.get(8)?,
                review_status: r.get(9)?,
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))
    }
    pub fn stage_csv(
        &self,
        batch_id: &str,
        contents: &str,
        mapping: &CsvImportMapping,
    ) -> Result<usize, ImportError> {
        if !matches!(mapping.positive_amount_type.as_str(), "income" | "expense")
            || !matches!(mapping.date_format.as_str(), "iso_date" | "rfc3339")
        {
            return Err(ImportError::InvalidBatch);
        }
        let mut reader = csv::ReaderBuilder::new()
            .flexible(false)
            .from_reader(contents.as_bytes());
        let headers = reader
            .headers()
            .map_err(|e| ImportError::Database(e.to_string()))?
            .clone();
        let date_index = headers
            .iter()
            .position(|h| h == mapping.date_column)
            .ok_or(ImportError::InvalidBatch)?;
        let amount_index = headers
            .iter()
            .position(|h| h == mapping.amount_column)
            .ok_or(ImportError::InvalidBatch)?;
        let description_index = mapping
            .description_column
            .as_ref()
            .and_then(|column| headers.iter().position(|h| h == column));
        if mapping.description_column.is_some() && description_index.is_none() {
            return Err(ImportError::InvalidBatch);
        }
        let mut rows = Vec::new();
        for (index, row) in reader.records().enumerate() {
            let row = row.map_err(|e| ImportError::Database(e.to_string()))?;
            let raw_date = row.get(date_index).unwrap_or("").trim().to_string();
            let raw_amount = row.get(amount_index).unwrap_or("").trim().to_string();
            let description = description_index
                .and_then(|i| row.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned);
            let date = if mapping.date_format == "iso_date" {
                chrono::NaiveDate::parse_from_str(&raw_date, "%Y-%m-%d")
                    .ok()
                    .map(|d| format!("{}T00:00:00Z", d))
            } else {
                chrono::DateTime::parse_from_rfc3339(&raw_date)
                    .ok()
                    .map(|d| d.to_rfc3339())
            };
            let signed = raw_amount.starts_with('-');
            let amount = parse_minor_units(raw_amount.strip_prefix('-').unwrap_or(&raw_amount));
            let status = if raw_date.is_empty() || raw_amount.is_empty() {
                "invalid"
            } else if date.is_none() || amount.is_none() {
                "uncertain"
            } else {
                "valid"
            };
            let transaction_type = if signed {
                Some(if mapping.positive_amount_type == "income" {
                    "expense"
                } else {
                    "income"
                })
            } else if amount.is_some() {
                Some(mapping.positive_amount_type.as_str())
            } else {
                None
            };
            rows.push((
                index + 2,
                raw_date,
                date,
                description,
                raw_amount,
                amount,
                transaction_type.map(str::to_owned),
                status.to_string(),
            ));
        }
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = connection
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let exists:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=?1 AND status='review' AND source_format='csv')",[batch_id],|r|r.get(0)).map_err(|e|ImportError::Database(e.to_string()))?;
        if !exists {
            return Err(ImportError::InvalidBatch);
        }
        for (line, raw_date, date, description, raw_amount, amount, kind, status) in &rows {
            tx.execute("INSERT INTO imported_records(id,batch_id,source_row_reference,original_date,normalized_date,original_description,normalized_description,original_amount,normalized_amount_minor,currency_code,transaction_type,validation_status) SELECT ?1,?2,?3,?4,?5,?6,?6,?7,?8,a.currency_code,?9,?10 FROM import_batches b JOIN accounts a ON a.id=b.account_id WHERE b.id=?2",params![Uuid::new_v4().to_string(),batch_id,line.to_string(),raw_date,date,description,raw_amount,amount,kind,status]).map_err(|e|ImportError::Database(e.to_string()))?;
        }
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'parse','INFO',?3)",params![Uuid::new_v4().to_string(),batch_id,format!("Staged {} CSV rows",rows.len())]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(rows.len())
    }
    pub fn list_batches(&self) -> Result<Vec<ImportBatchRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut s=c.prepare("SELECT id,account_id,source_file_name,source_format,status,reconciliation_status,created_at FROM import_batches ORDER BY created_at DESC").map_err(|e|ImportError::Database(e.to_string()))?;
        s.query_map([], |r| {
            Ok(ImportBatchRecord {
                id: r.get(0)?,
                account_id: r.get(1)?,
                source_file_name: r.get(2)?,
                source_format: r.get(3)?,
                status: r.get(4)?,
                reconciliation_status: r.get(5)?,
                created_at: r.get(6)?,
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))
    }
}
fn parse_minor_units(raw: &str) -> Option<i64> {
    let (whole, fraction) = raw.split_once('.').unwrap_or((raw, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || fraction.len() > 2
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let major = whole.parse::<i64>().ok()?;
    let cents = if fraction.is_empty() {
        0
    } else if fraction.len() == 1 {
        fraction.parse::<i64>().ok()? * 10
    } else {
        fraction.parse::<i64>().ok()?
    };
    let amount = major.checked_mul(100)?.checked_add(cents)?;
    (amount > 0).then_some(amount)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::database::Database;
    use std::path::Path;
    fn db() -> Database {
        Database::initialize(Path::new(":memory:")).unwrap()
    }
    fn account(db: &Database) {
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO accounts(id,name,account_type) VALUES ('a','Main','cash')",
                [],
            )
            .unwrap();
    }
    #[test]
    fn persists_batch_for_resume_without_ledger_entries() {
        let db = db();
        account(&db);
        let repository = SqliteImportRepository::new(&db);
        let batch = NewImportBatch {
            account_id: "a".into(),
            source_file_name: "statement.csv".into(),
            source_format: "csv".into(),
            source_file_hash: None,
        };
        let id = repository.create_batch(&batch).unwrap();
        assert_eq!(repository.list_batches().unwrap().len(), 1);
        let c = db.connection().lock().unwrap();
        let entries: i64 = c
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        let logs: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM import_logs WHERE batch_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(entries, 0);
        assert_eq!(logs, 1);
    }
    #[test]
    fn rejects_archived_account() {
        let db = db();
        account(&db);
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE accounts SET archived_at=CURRENT_TIMESTAMP WHERE id='a'",
                [],
            )
            .unwrap();
        let repo = SqliteImportRepository::new(&db);
        let batch = NewImportBatch {
            account_id: "a".into(),
            source_file_name: "statement.csv".into(),
            source_format: "csv".into(),
            source_file_hash: None,
        };
        assert!(matches!(
            repo.create_batch(&batch),
            Err(ImportError::InvalidBatch)
        ));
    }
    #[test]
    fn csv_staging_preserves_quoted_fields_and_marks_bad_dates_uncertain() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let id = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "mapped.csv".into(),
                source_format: "csv".into(),
                source_file_hash: None,
            })
            .unwrap();
        let csv = "Date,Description,Amount\n2026-11-01,\"Cafe, downtown\",12.50\nnot-a-date,Unknown,4.00\n";
        let mapping = CsvImportMapping {
            date_column: "Date".into(),
            description_column: Some("Description".into()),
            amount_column: "Amount".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        assert_eq!(repo.stage_csv(&id, csv, &mapping).unwrap(), 2);
        let rows = repo.list_records(&id).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].original_description.as_deref(),
            Some("Cafe, downtown")
        );
        assert_eq!(rows[0].normalized_amount_minor, Some(1250));
        assert_eq!(rows[1].validation_status, "uncertain");
        let entries: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entries, 0);
    }
    #[test]
    fn corrected_csv_record_stays_staged_and_is_audited() {
        let db = db();
        account(&db);
        let repo = SqliteImportRepository::new(&db);
        let batch = repo
            .create_batch(&NewImportBatch {
                account_id: "a".into(),
                source_file_name: "x.csv".into(),
                source_format: "csv".into(),
                source_file_hash: None,
            })
            .unwrap();
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: None,
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(&batch, "d,a\nbad,1.00\n", &mapping).unwrap();
        let row = repo.list_records(&batch).unwrap().remove(0);
        repo.correct_record(
            &row.id,
            "2026-11-06T00:00:00Z",
            100,
            "income",
            Some("Corrected"),
        )
        .unwrap();
        let corrected = repo.list_records(&batch).unwrap().remove(0);
        assert_eq!(corrected.validation_status, "valid");
        assert_eq!(corrected.review_status, "corrected");
        let logs: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM import_logs WHERE batch_id=?1 AND stage='review'",
                [&batch],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(logs, 1);
    }
}
