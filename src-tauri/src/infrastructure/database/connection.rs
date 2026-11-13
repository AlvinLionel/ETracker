use super::migrations;
use crate::infrastructure::security::{DataKey, replace_file};
use rusqlite::{Connection, OptionalExtension, Result};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

pub struct Database {
    connection: Mutex<Connection>,
    key: Option<DataKey>,
    path: Option<PathBuf>,
}

impl Database {
    pub fn initialize(path: &Path) -> Result<Self> {
        let mut connection = Connection::open(path)?;
        initialize_connection(&mut connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            key: None,
            path: Some(path.to_owned()),
        })
    }

    pub fn initialize_encrypted(path: &Path, key: DataKey) -> Result<Self, String> {
        if path != Path::new(":memory:") {
            recover_interrupted_database_replace(path, &key)?;
            cleanup_stale_database_staging(path)?;
            migrate_plain_database(path, &key)?;
        }
        let mut connection = Connection::open(path)
            .map_err(|error| format!("The encrypted database could not be opened: {error}"))?;
        set_encryption_key(&connection, &key)?;
        initialize_connection(&mut connection)
            .map_err(|error| format!("The encrypted database could not be initialized: {error}"))?;

        Ok(Self {
            connection: Mutex::new(connection),
            key: Some(key),
            path: Some(path.to_owned()),
        })
    }

    pub fn connection(&self) -> &Mutex<Connection> {
        &self.connection
    }

    pub fn data_key(&self) -> Option<&DataKey> {
        self.key.as_ref()
    }

    pub fn replace_encrypted_database(&self, source: &Connection) -> Result<(), String> {
        let key = self
            .key
            .as_ref()
            .ok_or("The database does not use local encryption.")?;
        let path = self
            .path
            .as_deref()
            .filter(|path| *path != Path::new(":memory:"))
            .ok_or("The encrypted database file location is unavailable.")?;
        let staging = migration_path(path);
        if staging.exists() {
            fs::remove_file(&staging).map_err(|error| {
                format!("A previous database staging file could not be removed: {error}")
            })?;
        }
        export_encrypted_database(source, &staging, key)?;

        {
            let staged = Connection::open(&staging).map_err(|error| {
                format!("The encrypted database staging file could not be opened: {error}")
            })?;
            set_encryption_key(&staged, key)?;
            let integrity: String = staged
                .query_row("PRAGMA integrity_check", [], |row| row.get(0))
                .map_err(|error| format!("The restored database could not be checked: {error}"))?;
            let has_foreign_key_violations: bool = staged
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| {
                    format!("The restored database references could not be checked: {error}")
                })?;
            if !integrity.eq_ignore_ascii_case("ok") || has_foreign_key_violations {
                drop(staged);
                let _ = fs::remove_file(&staging);
                return Err("The restored encrypted database failed its integrity checks.".into());
            }
        }

        let mut current = self.connection.lock().map_err(|_| "Database unavailable")?;
        let _: (i64, i64, i64) = current
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|error| format!("The current database could not be checkpointed: {error}"))?;
        current
            .pragma_update(None, "journal_mode", "DELETE")
            .map_err(|error| {
                format!("The current database could not be prepared for restore: {error}")
            })?;

        let placeholder = Connection::open_in_memory()
            .map_err(|error| format!("The restored database could not be prepared: {error}"))?;
        drop(std::mem::replace(&mut *current, placeholder));
        let rollback_path = restore_rollback_path(path);
        if rollback_path.exists() {
            fs::remove_file(&rollback_path).map_err(|error| {
                format!("A previous database recovery copy could not be removed: {error}")
            })?;
        }
        if let Err(error) = fs::copy(path, &rollback_path) {
            *current = open_encrypted_database(path, key)?;
            let _ = fs::remove_file(&staging);
            return Err(format!(
                "The current database could not be protected for restore: {error}"
            ));
        }
        if let Err(error) = validate_encrypted_file(&rollback_path, key) {
            *current = open_encrypted_database(path, key)?;
            let _ = fs::remove_file(&rollback_path);
            let _ = fs::remove_file(&staging);
            return Err(format!(
                "The current database recovery copy failed validation: {error}"
            ));
        }
        if let Err(error) = replace_file(&staging, path) {
            let reopened = open_encrypted_database(path, key).map_err(|open_error| {
                format!("{error} The existing database could not be reopened: {open_error}")
            })?;
            *current = reopened;
            let _ = fs::remove_file(&rollback_path);
            return Err(error);
        }
        let restored = match open_encrypted_database(path, key) {
            Ok(restored) => restored,
            Err(open_error) => {
                replace_file(&rollback_path, path).map_err(|rollback_error| {
                    format!(
                        "The restored database could not be opened: {open_error}. The previous database could not be recovered: {rollback_error}"
                    )
                })?;
                *current = open_encrypted_database(path, key).map_err(|rollback_error| {
                    format!(
                        "The restored database could not be opened: {open_error}. The previous database could not be reopened: {rollback_error}"
                    )
                })?;
                return Err(format!(
                    "The restored database could not be opened; the previous database was restored: {open_error}"
                ));
            }
        };
        *current = restored;
        if let Err(error) = fs::remove_file(&rollback_path) {
            eprintln!("An encrypted database recovery copy could not be cleaned up: {error}");
        }
        Ok(())
    }
}

fn validate_encrypted_file(path: &Path, key: &DataKey) -> Result<(), String> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("The encrypted database could not be opened: {error}"))?;
    set_encryption_key(&connection, key)?;
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("The encrypted database could not be checked: {error}"))?;
    let has_foreign_key_violations: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
            [],
            |row| row.get(0),
        )
        .map_err(|error| {
            format!("The encrypted database references could not be checked: {error}")
        })?;
    if !integrity.eq_ignore_ascii_case("ok") || has_foreign_key_violations {
        return Err("The encrypted database failed its integrity checks.".into());
    }
    Ok(())
}

fn recover_interrupted_database_replace(path: &Path, key: &DataKey) -> Result<(), String> {
    let rollback_path = restore_rollback_path(path);
    if !rollback_path.exists() {
        return Ok(());
    }
    if path.exists() && validate_encrypted_file(path, key).is_ok() {
        fs::remove_file(&rollback_path).map_err(|error| {
            format!("A stale database recovery copy could not be removed: {error}")
        })?;
        return Ok(());
    }
    validate_encrypted_file(&rollback_path, key)
        .map_err(|error| format!("Interrupted restore recovery failed: {error}"))?;
    replace_file(&rollback_path, path)?;
    Ok(())
}

fn cleanup_stale_database_staging(path: &Path) -> Result<(), String> {
    let staging = migration_path(path);
    match fs::symlink_metadata(&staging) {
        Ok(metadata) if metadata.file_type().is_file() => {
            fs::remove_file(staging).map_err(|error| {
                format!("A stale database staging file could not be removed: {error}")
            })
        }
        Ok(_) => Err("A database staging path is not a regular file.".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "A database staging file could not be checked: {error}"
        )),
    }
}

fn initialize_connection(connection: &mut Connection) -> Result<()> {
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

    migrations::run(connection)?;

    connection.execute(
        "DELETE FROM import_source_files WHERE expires_at <= CURRENT_TIMESTAMP",
        [],
    )?;
    let _: (i64, i64, i64) =
        connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

    Ok(())
}

fn set_encryption_key(connection: &Connection, key: &DataKey) -> Result<(), String> {
    let hexadecimal = key_hex(key);
    connection
        .execute_batch(&format!(
            "PRAGMA key = \"x'{hexadecimal}'\"; PRAGMA cipher_memory_security = ON;"
        ))
        .map_err(|error| format!("SQLCipher could not configure database encryption: {error}"))?;
    let cipher_version: Option<String> = connection
        .query_row("PRAGMA cipher_version", [], |row| row.get(0))
        .optional()
        .map_err(|error| format!("SQLCipher could not be verified: {error}"))?;
    if cipher_version.as_deref().map_or(true, str::is_empty) {
        return Err("SQLCipher is unavailable; the database was not opened.".into());
    }
    Ok(())
}

fn key_hex(key: &DataKey) -> String {
    let mut hexadecimal = String::with_capacity(key.bytes().len() * 2);
    for byte in key.bytes() {
        use std::fmt::Write as _;
        write!(&mut hexadecimal, "{byte:02x}").expect("writing to a string cannot fail");
    }
    hexadecimal
}

fn open_encrypted_database(path: &Path, key: &DataKey) -> Result<Connection, String> {
    let mut connection = Connection::open(path)
        .map_err(|error| format!("The encrypted database could not be opened: {error}"))?;
    set_encryption_key(&connection, key)?;
    initialize_connection(&mut connection)
        .map_err(|error| format!("The encrypted database could not be initialized: {error}"))?;
    Ok(connection)
}

fn migrate_plain_database(path: &Path, key: &DataKey) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let mut header = [0; 16];
    let read = fs::File::open(path)
        .and_then(|mut file| file.read(&mut header))
        .map_err(|error| format!("The existing database could not be inspected: {error}"))?;
    if read == 0 || !header.starts_with(b"SQLite format 3\0") {
        return Ok(());
    }

    let staging = migration_path(path);
    if staging.exists() {
        fs::remove_file(&staging).map_err(|error| {
            format!("An incomplete encrypted migration file could not be removed: {error}")
        })?;
    }

    let source = Connection::open(path)
        .map_err(|error| format!("The existing database could not be opened: {error}"))?;
    source
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| format!("The existing database could not be prepared: {error}"))?;
    source
        .pragma_update(None, "secure_delete", "ON")
        .map_err(|error| format!("The existing database could not be prepared: {error}"))?;
    let _: (i64, i64, i64) = source
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|error| format!("The existing database could not be checkpointed: {error}"))?;
    let source_integrity: String = source
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("The existing database could not be checked: {error}"))?;
    if !source_integrity.eq_ignore_ascii_case("ok") {
        return Err("The existing database failed its integrity check; it was not changed.".into());
    }

    export_encrypted_database(&source, &staging, key)?;

    let destination = Connection::open(&staging)
        .map_err(|error| format!("The encrypted migration copy could not be reopened: {error}"))?;
    set_encryption_key(&destination, key)?;
    let encrypted_integrity: String = destination
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| format!("The encrypted migration copy could not be checked: {error}"))?;
    if !encrypted_integrity.eq_ignore_ascii_case("ok") {
        drop(destination);
        drop(source);
        let _ = fs::remove_file(&staging);
        return Err("The encrypted migration copy failed its integrity check.".into());
    }
    let foreign_key_violation: bool = destination
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check)",
            [],
            |row| row.get(0),
        )
        .map_err(|error| format!("The encrypted migration copy could not be checked: {error}"))?;
    if foreign_key_violation {
        drop(destination);
        drop(source);
        let _ = fs::remove_file(&staging);
        return Err("The encrypted migration copy has invalid database references.".into());
    }
    drop(destination);
    drop(source);

    replace_file(&staging, path)?;
    Ok(())
}

fn export_encrypted_database(
    source: &Connection,
    destination_path: &Path,
    key: &DataKey,
) -> Result<(), String> {
    let hexadecimal = key_hex(key);
    let attach = format!("ATTACH DATABASE ?1 AS encrypted KEY \"x'{hexadecimal}'\"");
    source
        .execute(&attach, [destination_path.to_string_lossy().as_ref()])
        .map_err(|error| format!("An encrypted database copy could not be created: {error}"))?;
    let export_result = source.execute_batch("SELECT sqlcipher_export('encrypted');");
    let detach_result = source.execute_batch("DETACH DATABASE encrypted;");
    export_result
        .map_err(|error| format!("Database data could not be encrypted safely: {error}"))?;
    detach_result
        .map_err(|error| format!("The encrypted database copy could not be finalized: {error}"))?;
    Ok(())
}

fn migration_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".encrypted-migration");
    path.with_file_name(name)
}

fn restore_rollback_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".restore-rollback");
    path.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::{
        application::imports::{ImportRepository, NewImportBatch},
        infrastructure::security::DataKey,
        persistence::imports::SqliteImportRepository,
    };
    use rusqlite::OptionalExtension;
    use std::{env, fs, io::Read, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn encrypts_existing_database_and_preserves_rows_and_foreign_keys() {
        let database_path =
            env::temp_dir().join(format!("etracker-encryption-{}.sqlite", Uuid::new_v4()));
        let wal_path = PathBuf::from(format!("{}-wal", database_path.display()));
        let shm_path = PathBuf::from(format!("{}-shm", database_path.display()));
        {
            let database = Database::initialize(&database_path).unwrap();
            database.connection().lock().unwrap().execute(
                "INSERT INTO accounts(id,name,account_type) VALUES ('preserved-account','Cash','cash')",
                [],
            ).unwrap();
        }

        {
            let database =
                Database::initialize_encrypted(&database_path, DataKey::from_bytes([17; 32]))
                    .unwrap();
            let connection = database.connection().lock().unwrap();
            let account_name: String = connection
                .query_row(
                    "SELECT name FROM accounts WHERE id='preserved-account'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(account_name, "Cash");
            let violations: i64 = connection
                .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(violations, 0);
        }

        let mut header = [0; 16];
        fs::File::open(&database_path)
            .unwrap()
            .read_exact(&mut header)
            .unwrap();
        assert!(!header.starts_with(b"SQLite format 3\0"));

        let reopened =
            Database::initialize_encrypted(&database_path, DataKey::from_bytes([17; 32])).unwrap();
        let account_count: i64 = reopened
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        assert_eq!(account_count, 1);
        drop(reopened);
        for path in [database_path, wal_path, shm_path] {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn interrupted_encrypted_restore_recovers_the_previous_database() {
        let database_path = env::temp_dir().join(format!(
            "etracker-restore-recovery-{}.sqlite",
            Uuid::new_v4()
        ));
        let rollback_path = super::restore_rollback_path(&database_path);
        let staging_path = super::migration_path(&database_path);
        let key = [29; 32];
        {
            let database =
                Database::initialize_encrypted(&database_path, DataKey::from_bytes(key)).unwrap();
            database.connection().lock().unwrap().execute(
                "INSERT INTO accounts(id,name,account_type) VALUES ('preserved','Preserved cash','cash')",
                [],
            ).unwrap();
        }
        fs::copy(&database_path, &rollback_path).unwrap();
        fs::write(&staging_path, b"incomplete staging file").unwrap();
        fs::write(&database_path, b"interrupted replacement").unwrap();

        let recovered =
            Database::initialize_encrypted(&database_path, DataKey::from_bytes(key)).unwrap();
        let account: String = recovered
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT name FROM accounts WHERE id='preserved'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(account, "Preserved cash");
        assert!(!rollback_path.exists());
        assert!(!staging_path.exists());
        drop(recovered);
        let _ = fs::remove_file(database_path);
    }

    #[test]
    fn wrong_encryption_key_does_not_replace_the_database() {
        let database_path =
            env::temp_dir().join(format!("etracker-wrong-key-{}.sqlite", Uuid::new_v4()));
        let database =
            Database::initialize_encrypted(&database_path, DataKey::from_bytes([31; 32])).unwrap();
        database
            .connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO accounts(id,name,account_type) VALUES ('preserved','Cash','cash')",
                [],
            )
            .unwrap();
        drop(database);

        assert!(
            Database::initialize_encrypted(&database_path, DataKey::from_bytes([32; 32])).is_err()
        );
        let reopened =
            Database::initialize_encrypted(&database_path, DataKey::from_bytes([31; 32])).unwrap();
        let preserved: i64 = reopened
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM accounts WHERE id='preserved'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved, 1);
        drop(reopened);
        let _ = fs::remove_file(database_path);
    }

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
