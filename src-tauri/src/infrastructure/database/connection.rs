use super::migrations;
use rusqlite::{Connection, Result};
use std::{path::Path, sync::Mutex, time::Duration};

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn initialize(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;

        connection.busy_timeout(Duration::from_secs(5))?;

        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "secure_delete", "ON")?;

        connection.pragma_update(None, "journal_mode", "WAL")?;

        connection.pragma_update(None, "synchronous", "NORMAL")?;

        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;

        migrations::run(&mut connection)?;

        connection.execute(
            "DELETE FROM import_source_files WHERE expires_at <= CURRENT_TIMESTAMP",
            [],
        )?;
        let _: (i64, i64, i64) =
            connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn connection(&self) -> &Mutex<Connection> {
        &self.connection
    }
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::{
        application::imports::{ImportRepository, NewImportBatch},
        persistence::imports::SqliteImportRepository,
    };
    use rusqlite::OptionalExtension;
    use std::{env, fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn startup_securely_cleans_expired_source_bytes_from_sqlite_pages_and_wal() {
        let database_path =
            env::temp_dir().join(format!("etracker-expiry-{}.sqlite", Uuid::new_v4()));
        let wal_path = PathBuf::from(format!("{}-wal", database_path.display()));
        let shm_path = PathBuf::from(format!("{}-shm", database_path.display()));
        let batch_id: String;
        {
            let database = Database::initialize(&database_path).unwrap();
            database
                .connection()
                .lock()
                .unwrap()
                .execute(
                    "INSERT INTO accounts(id, name, account_type) VALUES ('retention-account', 'Retention account', 'cash')",
                    [],
                )
                .unwrap();
            let account_id = "retention-account".to_owned();
            batch_id = SqliteImportRepository::new(&database)
                .create_batch(&NewImportBatch {
                    account_id,
                    source_file_name: "expired.csv".into(),
                    source_format: "csv".into(),
                    revision_of_batch_id: None,
                    source_file_contents: b"sensitive source fixture".to_vec(),
                })
                .unwrap();
            database
                .connection()
                .lock()
                .unwrap()
                .execute(
                    "UPDATE import_source_files SET expires_at=datetime('now', '-1 day') WHERE batch_id=?1",
                    [&batch_id],
                )
                .unwrap();
        }

        let database = Database::initialize(&database_path).unwrap();
        let connection = database.connection().lock().unwrap();
        let remaining: Option<String> = connection
            .query_row(
                "SELECT batch_id FROM import_source_files WHERE batch_id=?1",
                [&batch_id],
                |row| row.get(0),
            )
            .optional()
            .unwrap();
        assert_eq!(remaining, None);
        let secure_delete: i64 = connection
            .pragma_query_value(None, "secure_delete", |row| row.get(0))
            .unwrap();
        assert_eq!(secure_delete, 1);
        assert_eq!(fs::metadata(&wal_path).unwrap().len(), 0);
        drop(connection);
        drop(database);
        for path in [database_path, wal_path, shm_path] {
            let _ = fs::remove_file(path);
        }
    }
}
