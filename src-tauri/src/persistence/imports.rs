use crate::{
    application::imports::{ImportError, ImportRepository, NewImportBatch},
    infrastructure::database::Database,
};
use rusqlite::params;
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
impl SqliteImportRepository<'_> {
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
}
