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

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn connection(&self) -> &Mutex<Connection> {
        &self.connection
    }
}
