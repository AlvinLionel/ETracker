use crate::infrastructure::database::{Database, LATEST_SCHEMA_VERSION, run_migrations};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use pbkdf2::{pbkdf2_hmac, sha2::Sha256};
use rusqlite::{Connection, MAIN_DB, backup::Backup};
use std::io::Cursor;

const MAGIC: &[u8; 8] = b"ETRBKUP1";
const SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 12;
const KEY_BYTES: usize = 32;
const PBKDF2_ITERATIONS: u32 = if cfg!(test) { 2_000 } else { 600_000 };
const MIN_PASSPHRASE_CHARS: usize = 12;

fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err("Use a passphrase with at least 12 characters.".into());
    }
    Ok(())
}

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_BYTES] {
    let mut key = [0u8; KEY_BYTES];
    pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
    key
}

fn encrypt_snapshot(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    validate_passphrase(passphrase)?;
    let mut salt = [0u8; SALT_BYTES];
    let mut nonce = [0u8; NONCE_BYTES];
    getrandom::fill(&mut salt).map_err(|e| format!("Secure random generation failed: {e}"))?;
    getrandom::fill(&mut nonce).map_err(|e| format!("Secure random generation failed: {e}"))?;
    let mut key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Backup encryption setup failed")?;
    let nonce =
        Nonce::try_from(nonce.as_slice()).map_err(|_| "Backup encryption nonce is invalid")?;
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| "Backup encryption failed")?;
    key.fill(0);
    let mut result = Vec::with_capacity(MAGIC.len() + SALT_BYTES + NONCE_BYTES + ciphertext.len());
    result.extend_from_slice(MAGIC);
    result.extend_from_slice(&salt);
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

fn decrypt_snapshot(encrypted: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    validate_passphrase(passphrase)?;
    let header_len = MAGIC.len() + SALT_BYTES + NONCE_BYTES;
    if encrypted.len() < header_len + 16 || !encrypted.starts_with(MAGIC) {
        return Err("This is not a supported ETracker backup file.".into());
    }
    let salt = &encrypted[MAGIC.len()..MAGIC.len() + SALT_BYTES];
    let nonce = &encrypted[MAGIC.len() + SALT_BYTES..header_len];
    let mut key = derive_key(passphrase, salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Backup decryption setup failed")?;
    let nonce = Nonce::try_from(nonce).map_err(|_| "Backup file nonce is invalid")?;
    let result = cipher
        .decrypt(&nonce, &encrypted[header_len..])
        .map_err(|_| "The passphrase is incorrect or the backup file is damaged.".to_string());
    key.fill(0);
    result
}

pub fn create_encrypted_backup(database: &Database, passphrase: &str) -> Result<String, String> {
    validate_passphrase(passphrase)?;
    let mut connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    let plaintext = connection
        .serialize(MAIN_DB)
        .map_err(|e| format!("Could not create database snapshot: {e}"))?
        .to_vec();
    let encrypted = encrypt_snapshot(&plaintext, passphrase)?;
    Ok(STANDARD.encode(encrypted))
}

fn load_validated_database(plaintext: &mut [u8]) -> Result<Connection, String> {
    if plaintext.len() < 100 || !plaintext.starts_with(b"SQLite format 3\0") {
        return Err("The backup does not contain a readable SQLite database.".into());
    }
    match (plaintext[18], plaintext[19]) {
        (1, 1) => {}
        (2, 2) => {
            plaintext[18] = 1;
            plaintext[19] = 1;
        }
        _ => return Err("The backup uses an unsupported SQLite journal format.".into()),
    }
    let mut connection = Connection::open_in_memory().map_err(|e| e.to_string())?;
    let plaintext_len = plaintext.len();
    connection
        .deserialize_read_exact(MAIN_DB, Cursor::new(&*plaintext), plaintext_len, false)
        .map_err(|_| "The backup does not contain a readable SQLite database.")?;
    connection
        .pragma_update(None, "journal_mode", "MEMORY")
        .map_err(|e| format!("Could not load backup for validation: {e}"))?;
    let migrations_table: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !migrations_table {
        return Err("This SQLite file is not an ETracker database.".into());
    }
    let latest: i64 = connection
        .query_row(
            "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if latest == 0 || latest > LATEST_SCHEMA_VERSION {
        return Err("This backup was created by an unsupported ETracker database version.".into());
    }
    let quick_check: String = connection
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if !quick_check.eq_ignore_ascii_case("ok") {
        return Err("The backup database failed its integrity check.".into());
    }
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    run_migrations(&mut connection)
        .map_err(|e| format!("Could not upgrade backup database: {e}"))?;
    let has_accounts: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='accounts')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if !has_accounts {
        return Err("This backup is missing required ETracker records.".into());
    }
    let integrity: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    let mut fk = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|e| e.to_string())?;
    let has_fk_violations = fk
        .query([])
        .map_err(|e| e.to_string())?
        .next()
        .map_err(|e| e.to_string())?
        .is_some();
    drop(fk);
    if !integrity.eq_ignore_ascii_case("ok") || has_fk_violations {
        return Err("The backup database failed its integrity checks.".into());
    }
    Ok(connection)
}

pub fn restore_encrypted_backup(
    database: &Database,
    encoded: &str,
    passphrase: &str,
) -> Result<(), String> {
    validate_passphrase(passphrase)?;
    let encrypted = STANDARD
        .decode(encoded)
        .map_err(|_| "Backup file encoding is invalid.")?;
    let mut plaintext = decrypt_snapshot(&encrypted, passphrase)?;
    let source = load_validated_database(&mut plaintext)?;
    plaintext.fill(0);
    let mut connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    let backup = Backup::new(&source, &mut connection)
        .map_err(|e| format!("Could not prepare database restore: {e}"))?;
    backup
        .run_to_completion(5, std::time::Duration::from_millis(250), None)
        .map_err(|e| format!("Could not restore the database: {e}"))?;
    drop(backup);
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(|e| format!("Database restored, but WAL mode could not be re-enabled: {e}"))?;
    connection
        .execute(
            "DELETE FROM import_source_files WHERE expires_at <= CURRENT_TIMESTAMP",
            [],
        )
        .map_err(|e| {
            format!("Backup restored, but expired import source files could not be cleaned up: {e}")
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

    const PASSPHRASE: &str = "a-long-test-passphrase-42";

    struct TestDatabase(PathBuf);

    impl TestDatabase {
        fn new() -> Self {
            Self(
                std::env::temp_dir()
                    .join(format!("etracker-backup-test-{}.db", uuid::Uuid::new_v4())),
            )
        }
    }

    impl Drop for TestDatabase {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
            let _ = fs::remove_file(self.0.with_extension("db-wal"));
            let _ = fs::remove_file(self.0.with_extension("db-shm"));
        }
    }

    #[test]
    fn encrypts_and_authenticates_backup_bytes() {
        let encrypted = encrypt_snapshot(b"sqlite test payload", PASSPHRASE).unwrap();
        assert!(encrypted.starts_with(MAGIC));
        assert_eq!(
            decrypt_snapshot(&encrypted, PASSPHRASE).unwrap(),
            b"sqlite test payload"
        );
        assert!(decrypt_snapshot(&encrypted, "a-different-passphrase-99").is_err());
        let mut tampered = encrypted;
        *tampered.last_mut().unwrap() ^= 1;
        assert!(decrypt_snapshot(&tampered, PASSPHRASE).is_err());
    }

    #[test]
    fn restores_only_a_valid_encrypted_etracker_database() {
        let source_path = TestDatabase::new();
        let source = Database::initialize(&source_path.0).unwrap();
        source.connection().lock().unwrap()
            .execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('restore-test','Restored cash','cash','KES')", []).unwrap();
        let encoded = create_encrypted_backup(&source, PASSPHRASE).unwrap();

        let destination = Database::initialize(Path::new(":memory:")).unwrap();
        destination.connection().lock().unwrap()
            .execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('old','Old cash','cash','KES')", []).unwrap();
        restore_encrypted_backup(&destination, &encoded, PASSPHRASE).unwrap();
        let restored: bool = destination
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id='restore-test')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let old: bool = destination
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id='old')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(restored);
        assert!(!old);
        assert!(
            restore_encrypted_backup(&destination, &encoded, "another-invalid-passphrase").is_err()
        );
        let restored_after_failure: bool = destination
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM accounts WHERE id='restore-test')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(restored_after_failure);
        assert!(
            restore_encrypted_backup(
                &destination,
                &STANDARD.encode(b"not a backup file"),
                PASSPHRASE
            )
            .is_err()
        );
    }
}
