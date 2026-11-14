use crate::infrastructure::database::Database;
use rusqlite::TransactionBehavior;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn reset_all_user_data(database: &Database, app_data_dir: &Path) -> Result<(), String> {
    {
        let mut connection = database
            .connection()
            .lock()
            .map_err(|_| "The local database is unavailable.")?;
        let tables = {
            let mut statement = connection
                .prepare(
                    "SELECT name FROM sqlite_schema
                     WHERE type='table' AND name NOT LIKE 'sqlite_%'
                       AND name <> 'schema_migrations'",
                )
                .map_err(|error| format!("The saved data could not be inspected: {error}"))?;
            statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(|error| format!("The saved data could not be inspected: {error}"))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| format!("The saved data could not be inspected: {error}"))?
        };
        let has_sequence_table: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name='sqlite_sequence')",
                [],
                |row| row.get(0),
            )
            .map_err(|error| format!("The saved data could not be inspected: {error}"))?;

        connection
            .pragma_update(None, "foreign_keys", "OFF")
            .map_err(|error| format!("The saved data could not be prepared for reset: {error}"))?;
        let reset_result: Result<(), String> = (|| {
            let transaction = connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(|error| format!("The saved data could not be reset: {error}"))?;
            for table in &tables {
                let quoted_table = table.replace('"', "\"\"");
                transaction
                    .execute(&format!("DELETE FROM \"{quoted_table}\""), [])
                    .map_err(|error| format!("The saved data could not be reset: {error}"))?;
            }
            if has_sequence_table {
                transaction
                    .execute("DELETE FROM sqlite_sequence", [])
                    .map_err(|error| format!("The saved data could not be reset: {error}"))?;
            }
            transaction
                .commit()
                .map_err(|error| format!("The saved data could not be reset: {error}"))?;
            Ok(())
        })();
        let foreign_keys_result = connection.pragma_update(None, "foreign_keys", "ON");
        reset_result?;
        foreign_keys_result
            .map_err(|error| format!("Database safety checks could not be restored: {error}"))?;
        connection.execute_batch("VACUUM").map_err(|error| {
            format!(
                "Saved records were erased, but database storage could not be compacted: {error}"
            )
        })?;
        let checkpoint: (i64, i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|error| {
                format!("Saved records were erased, but temporary database data could not be cleared: {error}")
            })?;
        if checkpoint.0 != 0 {
            return Err(
                "Saved records were erased, but temporary database data could not be cleared. Close other ETracker windows and try Reset system again."
                    .into(),
            );
        }
    }

    let attachments_dir = app_data_dir.join("attachments");
    let temporary_attachments_dir = app_data_dir.join(".attachment-viewer-temp");
    remove_data_directory(&attachments_dir).map_err(|_| {
        "Saved records were erased, but attached documents could not all be removed. Close any open documents and try Reset system again."
    })?;
    remove_data_directory(&temporary_attachments_dir).map_err(|_| {
        "Saved records were erased, but temporary document copies could not all be removed. Close any open documents and try Reset system again."
    })?;
    fs::create_dir_all(&attachments_dir).map_err(|_| {
        "Saved records and documents were erased, but local attachment storage could not be prepared."
    })?;
    Ok(())
}

fn remove_data_directory(path: &PathBuf) -> std::io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => fs::remove_dir_all(path),
        Ok(metadata) if metadata.file_type().is_file() || metadata.file_type().is_symlink() => {
            fs::remove_file(path)
        }
        Ok(_) => Err(std::io::Error::other(
            "The local data path is not a regular file or directory.",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::reset_all_user_data;
    use crate::infrastructure::database::Database;
    use std::{env, fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn clears_saved_records_and_attachment_files_but_keeps_schema() {
        let app_data_dir = env::temp_dir().join(format!("etracker-reset-{}", Uuid::new_v4()));
        let attachments_dir = app_data_dir.join("attachments");
        let temporary_dir = app_data_dir.join(".attachment-viewer-temp");
        fs::create_dir_all(&attachments_dir).unwrap();
        fs::create_dir_all(&temporary_dir).unwrap();
        fs::write(
            attachments_dir.join("attachment.bin"),
            b"encrypted attachment",
        )
        .unwrap();
        fs::write(temporary_dir.join("document.pdf"), b"temporary document").unwrap();

        let database = Database::initialize(PathBuf::from(":memory:").as_path()).unwrap();
        database
            .connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO accounts(id,name,account_type) VALUES ('reset-account','Cash','cash')",
                [],
            )
            .unwrap();
        let migrations_before: i64 = database
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();

        reset_all_user_data(&database, &app_data_dir).unwrap();

        let account_count: i64 = database
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        let migrations_after: i64 = database
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        let attachment_count = fs::read_dir(&attachments_dir).unwrap().count();
        let temporary_dir_exists = temporary_dir.try_exists().unwrap();

        assert_eq!(account_count, 0);
        assert_eq!(migrations_after, migrations_before);
        assert_eq!(attachment_count, 0);
        assert!(!temporary_dir_exists);

        fs::remove_dir_all(app_data_dir).unwrap();
    }
}
