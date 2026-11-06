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
        let duplicate: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM imported_records r JOIN import_batches b ON b.id=r.batch_id JOIN accounts a ON a.id=b.account_id JOIN financial_transactions t ON t.transaction_type=r.transaction_type JOIN account_entries e ON e.transaction_id=t.id AND e.account_id=a.id WHERE r.id=?1 AND t.status='posted' AND t.currency_code=a.currency_code AND t.amount_minor=?2 AND date(t.occurred_at)=date(?3)) OR EXISTS(SELECT 1 FROM imported_records r JOIN imported_records other ON other.batch_id=r.batch_id AND other.id<>r.id WHERE r.id=?1 AND other.validation_status='valid' AND other.normalized_amount_minor=?2 AND other.transaction_type=?4 AND date(other.normalized_date)=date(?3))",
            params![record_id, amount_minor, date, kind], |r| r.get(0),
        ).map_err(|e| ImportError::Database(e.to_string()))?;
        if duplicate {
            tx.execute("UPDATE imported_records SET duplicate_status='probable',review_status='pending' WHERE id=?1", [record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        } else {
            tx.execute("UPDATE imported_records SET duplicate_status='unmatched',review_status='corrected' WHERE id=?1", [record_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        }
        let message = if duplicate {
            "Staged record corrected; possible duplicate requires review"
        } else {
            "Staged record corrected"
        };
        tx.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message) VALUES (?1,?2,?3,'review',?4,?5)",params![Uuid::new_v4().to_string(),batch,record_id,if duplicate {"WARNING"} else {"INFO"},message]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn review_record(&self, record_id: &str, keep: bool) -> Result<(), ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let batch:Option<String>=tx.query_row("SELECT b.id FROM imported_records r JOIN import_batches b ON b.id=r.batch_id WHERE r.id=?1 AND b.status='review' AND r.validation_status='valid'",[record_id],|r|r.get(0)).optional().map_err(|e|ImportError::Database(e.to_string()))?;
        let Some(batch) = batch else {
            return Err(ImportError::InvalidBatch);
        };
        let (review, duplicate) = if keep {
            ("confirmed", "confirmed_new")
        } else {
            ("skipped", "skip")
        };
        tx.execute(
            "UPDATE imported_records SET review_status=?1,duplicate_status=?2 WHERE id=?3",
            params![review, duplicate, record_id],
        )
        .map_err(|e| ImportError::Database(e.to_string()))?;
        let message = if keep {
            "Candidate confirmed as new"
        } else {
            "Candidate skipped by user"
        };
        tx.execute("INSERT INTO import_logs(id,batch_id,record_id,stage,severity,message) VALUES (?1,?2,?3,'review','INFO',?4)",params![Uuid::new_v4().to_string(),batch,record_id,message]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(())
    }
    fn finalize(&self, batch_id: &str) -> Result<usize, ImportError> {
        let mut c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let batch: Option<(String, String, String)> = tx
            .query_row(
                "SELECT account_id,status,reconciliation_status FROM import_batches WHERE id=?1",
                [batch_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        let Some((account_id, status, reconciliation)) = batch else {
            return Err(ImportError::InvalidBatch);
        };
        if status != "review" || reconciliation == "discrepancy" {
            return Err(ImportError::InvalidBatch);
        }
        let active: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL)",
                [&account_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if !active {
            return Err(ImportError::InvalidBatch);
        }
        let unresolved:i64=tx.query_row("SELECT COUNT(*) FROM imported_records WHERE batch_id=?1 AND (validation_status<>'valid' OR review_status='pending' OR (duplicate_status='probable' AND review_status NOT IN ('confirmed','skipped','corrected')))",[batch_id],|r|r.get(0)).map_err(|e|ImportError::Database(e.to_string()))?;
        if unresolved > 0 {
            return Err(ImportError::InvalidBatch);
        }
        let rows: Vec<(String, i64, String, String, Option<String>)> = {
            let mut s=tx.prepare("SELECT id,normalized_amount_minor,transaction_type,normalized_date,normalized_description FROM imported_records WHERE batch_id=?1 AND review_status<>'skipped' ORDER BY CAST(source_row_reference AS INTEGER)").map_err(|e|ImportError::Database(e.to_string()))?;
            s.query_map([batch_id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(|e| ImportError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| ImportError::Database(e.to_string()))?
        };
        let currency: String = tx
            .query_row(
                "SELECT currency_code FROM accounts WHERE id=?1",
                [&account_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        for (record_id, amount, kind, date, description) in &rows {
            if *amount <= 0
                || !matches!(kind.as_str(), "income" | "expense")
                || chrono::DateTime::parse_from_rfc3339(date).is_err()
            {
                return Err(ImportError::InvalidBatch);
            }
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type) VALUES (?1,?2,?3,?4,?5,?6,'import')",params![id,kind,amount,currency,date,description]).map_err(|e|ImportError::Database(e.to_string()))?;
            let signed = if kind == "income" { *amount } else { -*amount };
            tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,account_id,signed]).map_err(|e|ImportError::Database(e.to_string()))?;
            tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'created',?3)",params![Uuid::new_v4().to_string(),id,serde_json::json!({"source":"import","batch_id":batch_id,"record_id":record_id,"amount_minor":amount,"transaction_type":kind,"occurred_at":date}).to_string()]).map_err(|e|ImportError::Database(e.to_string()))?;
            tx.execute(
                "UPDATE imported_records SET linked_transaction_id=?1 WHERE id=?2",
                params![id, record_id],
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        }
        tx.execute("UPDATE import_batches SET status='finalized',finalized_at=CURRENT_TIMESTAMP WHERE id=?1 AND status='review'",[batch_id]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.execute("INSERT INTO import_logs(id,batch_id,stage,severity,message) VALUES (?1,?2,'finalize','INFO',?3)",params![Uuid::new_v4().to_string(),batch_id,format!("Finalized {} staged transactions",rows.len())]).map_err(|e|ImportError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| ImportError::Database(e.to_string()))?;
        Ok(rows.len())
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
    pub duplicate_status: String,
    pub review_status: String,
}
#[derive(serde::Serialize)]
pub struct ImportLogRecord {
    pub id: String,
    pub timestamp: String,
    pub stage: String,
    pub severity: String,
    pub message: String,
    pub record_id: Option<String>,
    pub corrective_action: Option<String>,
}
impl SqliteImportRepository<'_> {
    pub fn list_logs(&self, batch_id: &str) -> Result<Vec<ImportLogRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut s = c.prepare("SELECT id,timestamp,stage,severity,message,record_id,corrective_action FROM import_logs WHERE batch_id=?1 ORDER BY timestamp,id").map_err(|e| ImportError::Database(e.to_string()))?;
        s.query_map([batch_id], |r| {
            Ok(ImportLogRecord {
                id: r.get(0)?,
                timestamp: r.get(1)?,
                stage: r.get(2)?,
                severity: r.get(3)?,
                message: r.get(4)?,
                record_id: r.get(5)?,
                corrective_action: r.get(6)?,
            })
        })
        .map_err(|e| ImportError::Database(e.to_string()))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| ImportError::Database(e.to_string()))
    }

    pub fn list_records(&self, batch_id: &str) -> Result<Vec<StagedImportRecord>, ImportError> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| ImportError::ConnectionPoisoned)?;
        let mut s=c.prepare("SELECT id,source_row_reference,original_date,normalized_date,original_description,original_amount,normalized_amount_minor,transaction_type,validation_status,duplicate_status,review_status FROM imported_records WHERE batch_id=?1 ORDER BY CAST(source_row_reference AS INTEGER)").map_err(|e|ImportError::Database(e.to_string()))?;
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
                duplicate_status: r.get(9)?,
                review_status: r.get(10)?,
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
        if rows.is_empty() {
            return Err(ImportError::InvalidBatch);
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
        let already_staged: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM imported_records WHERE batch_id=?1)",
                [batch_id],
                |r| r.get(0),
            )
            .map_err(|e| ImportError::Database(e.to_string()))?;
        if already_staged {
            return Err(ImportError::InvalidBatch);
        }
        for (line, raw_date, date, description, raw_amount, amount, kind, status) in &rows {
            let duplicate = if status == "valid" {
                tx.query_row("SELECT EXISTS(SELECT 1 FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN import_batches b ON b.id=?1 JOIN accounts a ON a.id=b.account_id WHERE e.account_id=a.id AND t.status='posted' AND t.currency_code=a.currency_code AND t.amount_minor=?2 AND t.transaction_type=?3 AND date(t.occurred_at)=date(?4)) OR EXISTS(SELECT 1 FROM imported_records r WHERE r.batch_id=?1 AND r.validation_status='valid' AND r.normalized_amount_minor=?2 AND r.transaction_type=?3 AND date(r.normalized_date)=date(?4))",params![batch_id,amount,kind,date],|r|r.get::<_,bool>(0)).map_err(|e|ImportError::Database(e.to_string()))?
            } else {
                false
            };
            let duplicate_status = if duplicate { "probable" } else { "unmatched" };
            tx.execute("INSERT INTO imported_records(id,batch_id,source_row_reference,original_date,normalized_date,original_description,normalized_description,original_amount,normalized_amount_minor,currency_code,transaction_type,validation_status,duplicate_status) SELECT ?1,?2,?3,?4,?5,?6,?6,?7,?8,a.currency_code,?9,?10,?11 FROM import_batches b JOIN accounts a ON a.id=b.account_id WHERE b.id=?2",params![Uuid::new_v4().to_string(),batch_id,line.to_string(),raw_date,date,description,raw_amount,amount,kind,status,duplicate_status]).map_err(|e|ImportError::Database(e.to_string()))?;
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

    #[test]
    fn correcting_into_existing_transaction_requires_duplicate_review() {
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
        {
            let c = db.connection().lock().unwrap();
            c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('existing','expense',1250,'KES','2026-11-06T00:00:00Z','manual')", []).unwrap();
            c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('existing-entry','existing','a',-1250)", []).unwrap();
        }
        repo.correct_record(&row.id, "2026-11-06T00:00:00Z", 1250, "expense", None)
            .unwrap();
        let corrected = repo.list_records(&batch).unwrap().remove(0);
        assert_eq!(corrected.duplicate_status, "probable");
        assert_eq!(corrected.review_status, "pending");
        assert!(matches!(
            repo.finalize(&batch),
            Err(ImportError::InvalidBatch)
        ));
    }
    #[test]
    fn detects_multifactor_duplicate_candidate_without_auto_discard() {
        let db = db();
        account(&db);
        {
            let c = db.connection().lock().unwrap();
            c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('existing','expense',1250,'KES','2026-11-01T00:00:00Z','manual')",[]).unwrap();
            c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('entry','existing','a',-1250)",[]).unwrap();
        }
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
        repo.stage_csv(&batch, "d,a\n2026-11-01,12.50\n", &mapping)
            .unwrap();
        let row = repo.list_records(&batch).unwrap().remove(0);
        assert_eq!(row.duplicate_status, "probable");
        assert_eq!(row.review_status, "pending");
        let count: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    fn staged_expense(repo: &SqliteImportRepository<'_>, batch: &str) -> String {
        let mapping = CsvImportMapping {
            date_column: "d".into(),
            description_column: Some("memo".into()),
            amount_column: "a".into(),
            positive_amount_type: "expense".into(),
            date_format: "iso_date".into(),
        };
        repo.stage_csv(batch, "d,memo,a\n2026-11-05,Groceries,12.50\n", &mapping)
            .unwrap();
        repo.list_records(batch).unwrap().remove(0).id
    }

    #[test]
    fn finalization_posts_import_and_is_one_shot() {
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
        let record = staged_expense(&repo, &batch);
        repo.review_record(&record, true).unwrap();
        assert_eq!(repo.finalize(&batch).unwrap(), 1);
        assert!(matches!(
            repo.finalize(&batch),
            Err(ImportError::InvalidBatch)
        ));
        let c = db.connection().lock().unwrap();
        let posted: (i64, i64) = c
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(amount_minor),0) FROM account_entries",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(posted, (1, -1250));
        let linked: bool = c
            .query_row(
                "SELECT linked_transaction_id IS NOT NULL FROM imported_records WHERE id=?1",
                [&record],
                |r| r.get(0),
            )
            .unwrap();
        assert!(linked);
        let audit: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM transaction_audit_log WHERE action='created'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(audit, 1);
    }

    #[test]
    fn finalization_rejects_unreviewed_rows_without_posting() {
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
        staged_expense(&repo, &batch);
        assert!(matches!(
            repo.finalize(&batch),
            Err(ImportError::InvalidBatch)
        ));
        let entries: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entries, 0);
    }

    #[test]
    fn finalization_rolls_back_everything_when_ledger_insert_fails() {
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
        let record = staged_expense(&repo, &batch);
        repo.review_record(&record, true).unwrap();
        db.connection().lock().unwrap().execute_batch("CREATE TRIGGER fail_import_entry BEFORE INSERT ON account_entries BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
        assert!(repo.finalize(&batch).is_err());
        let c = db.connection().lock().unwrap();
        let status: String = c
            .query_row(
                "SELECT status FROM import_batches WHERE id=?1",
                [&batch],
                |r| r.get(0),
            )
            .unwrap();
        let (txns, entries): (i64,i64) = c.query_row("SELECT (SELECT COUNT(*) FROM financial_transactions WHERE source_type='import'), (SELECT COUNT(*) FROM account_entries)", [], |r|Ok((r.get(0)?,r.get(1)?))).unwrap();
        let linked: bool = c
            .query_row(
                "SELECT linked_transaction_id IS NOT NULL FROM imported_records WHERE id=?1",
                [&record],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "review");
        assert_eq!((txns, entries), (0, 0));
        assert!(!linked);
    }
}
