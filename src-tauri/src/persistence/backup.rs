use crate::infrastructure::database::{Database, LATEST_SCHEMA_VERSION, run_migrations};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use pbkdf2::{pbkdf2_hmac, sha2::Sha256 as PbkdfSha256};
use rusqlite::{Connection, MAIN_DB, backup::Backup};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAGIC: &[u8; 8] = b"ETRBKUP1";
const SALT_BYTES: usize = 16;
const NONCE_BYTES: usize = 12;
const KEY_BYTES: usize = 32;
const PBKDF2_ITERATIONS: u32 = if cfg!(test) { 2_000 } else { 600_000 };
const MIN_PASSPHRASE_CHARS: usize = 12;
const BUNDLE_MAGIC: &[u8; 8] = b"ETRBNDL2";
const MAX_ATTACHMENT_BYTES: u64 = 10 * 1024 * 1024;
const MAX_BACKUP_ATTACHMENTS_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
struct BackupAttachment {
    stored_name: String,
    bytes: Vec<u8>,
}

struct BackupPayload {
    database: Vec<u8>,
    attachments: Vec<BackupAttachment>,
}

impl Drop for BackupAttachment {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

impl Drop for BackupPayload {
    fn drop(&mut self) {
        self.database.fill(0);
    }
}

fn put_length(output: &mut Vec<u8>, value: usize) -> Result<(), String> {
    let length = u32::try_from(value).map_err(|_| "Backup item is too large.")?;
    output.extend_from_slice(&length.to_le_bytes());
    Ok(())
}

fn read_length(input: &mut Cursor<&[u8]>) -> Result<usize, String> {
    let mut bytes = [0u8; 4];
    std::io::Read::read_exact(input, &mut bytes)
        .map_err(|_| "The backup attachment archive is incomplete.")?;
    Ok(u32::from_le_bytes(bytes) as usize)
}

fn encode_payload(
    database: Vec<u8>,
    attachments: Vec<BackupAttachment>,
) -> Result<Vec<u8>, String> {
    if attachments.len() > 10_000 {
        return Err("The backup contains too many attachments.".into());
    }
    let total = attachments.iter().try_fold(0u64, |sum, item| {
        sum.checked_add(item.bytes.len() as u64)
            .ok_or_else(|| "The backup attachment size is invalid.".to_string())
    })?;
    if total > MAX_BACKUP_ATTACHMENTS_BYTES {
        return Err(
            "Attachments exceed the 100 MiB backup limit. Remove files or create a smaller backup."
                .into(),
        );
    }
    let mut output = Vec::new();
    output.extend_from_slice(BUNDLE_MAGIC);
    put_length(&mut output, database.len())?;
    output.extend_from_slice(&database);
    put_length(&mut output, attachments.len())?;
    for item in attachments {
        if !valid_stored_name(&item.stored_name) || item.bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
            return Err("The backup contains an invalid attachment.".into());
        }
        put_length(&mut output, item.stored_name.len())?;
        output.extend_from_slice(item.stored_name.as_bytes());
        put_length(&mut output, item.bytes.len())?;
        output.extend_from_slice(&item.bytes);
    }
    Ok(output)
}

fn decode_payload(bytes: Vec<u8>) -> Result<BackupPayload, String> {
    if bytes.starts_with(b"SQLite format 3\0") {
        return Ok(BackupPayload {
            database: bytes,
            attachments: Vec::new(),
        });
    }
    if !bytes.starts_with(BUNDLE_MAGIC) {
        return Err("The backup does not contain a supported ETracker data bundle.".into());
    }
    let mut input = Cursor::new(&bytes[BUNDLE_MAGIC.len()..]);
    let database_len = read_length(&mut input)?;
    if database_len < 100 || database_len > bytes.len() {
        return Err("The backup database section is invalid.".into());
    }
    let mut database = vec![0; database_len];
    std::io::Read::read_exact(&mut input, &mut database)
        .map_err(|_| "The backup database section is incomplete.")?;
    let count = read_length(&mut input)?;
    if count > 10_000 {
        return Err("The backup contains too many attachments.".into());
    }
    let mut attachments = Vec::with_capacity(count);
    let mut names = std::collections::HashSet::new();
    let mut total = 0u64;
    for _ in 0..count {
        let name_len = read_length(&mut input)?;
        if name_len > 64 {
            return Err("The backup contains an invalid attachment name.".into());
        }
        let mut name = vec![0; name_len];
        std::io::Read::read_exact(&mut input, &mut name)
            .map_err(|_| "The backup attachment entry is incomplete.")?;
        let stored_name = String::from_utf8(name)
            .map_err(|_| "The backup contains an invalid attachment name.")?;
        if !valid_stored_name(&stored_name) || !names.insert(stored_name.clone()) {
            return Err("The backup contains an unsafe or duplicate attachment name.".into());
        }
        let file_len = read_length(&mut input)?;
        total = total
            .checked_add(file_len as u64)
            .ok_or_else(|| "The backup attachment size is invalid.".to_string())?;
        if file_len == 0
            || file_len as u64 > MAX_ATTACHMENT_BYTES
            || total > MAX_BACKUP_ATTACHMENTS_BYTES
        {
            return Err("The backup attachments exceed supported size limits.".into());
        }
        let mut file = vec![0; file_len];
        std::io::Read::read_exact(&mut input, &mut file)
            .map_err(|_| "The backup attachment data is incomplete.")?;
        attachments.push(BackupAttachment {
            stored_name,
            bytes: file,
        });
    }
    if input.position() as usize != bytes.len() - BUNDLE_MAGIC.len() {
        return Err("The backup contains unexpected trailing data.".into());
    }
    Ok(BackupPayload {
        database,
        attachments,
    })
}

fn valid_stored_name(value: &str) -> bool {
    let Some((stem, extension)) = value.rsplit_once('.') else {
        return false;
    };
    Uuid::parse_str(stem).is_ok() && matches!(extension, "pdf" | "jpg" | "png")
}

fn collect_attachments(
    connection: &Connection,
    root: &Path,
) -> Result<Vec<BackupAttachment>, String> {
    let table_exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='transaction_attachments')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !table_exists {
        return Ok(Vec::new());
    }
    let records = {
        let mut statement = connection.prepare("SELECT stored_name,size_bytes,sha256 FROM transaction_attachments ORDER BY stored_name")
            .map_err(|e| e.to_string())?;
        statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
    };
    let mut attachments = Vec::new();
    let mut total = 0u64;
    for (stored_name, expected_size, expected_hash) in records {
        if !valid_stored_name(&stored_name) {
            return Err("The database contains an unsafe attachment reference.".into());
        }
        let file_path = root.join(&stored_name);
        let metadata = fs::symlink_metadata(&file_path)
            .map_err(|_| "A transaction attachment is missing; the backup was not created.")?;
        if !metadata.file_type().is_file()
            || metadata.len() == 0
            || metadata.len() > MAX_ATTACHMENT_BYTES
        {
            return Err(
                "A transaction attachment is unavailable or exceeds the supported size.".into(),
            );
        }
        total = total
            .checked_add(metadata.len())
            .ok_or_else(|| "The attachment size is invalid.".to_string())?;
        if total > MAX_BACKUP_ATTACHMENTS_BYTES {
            return Err("Attachments exceed the 100 MiB backup limit. Remove files or create a smaller backup.".into());
        }
        let bytes = fs::read(&file_path)
            .map_err(|_| "A transaction attachment could not be read for backup.")?;
        if bytes.len() as i64 != expected_size
            || format!("{:x}", Sha256::digest(&bytes)) != expected_hash
        {
            return Err(
                "A transaction attachment failed its integrity check; the backup was not created."
                    .into(),
            );
        }
        attachments.push(BackupAttachment { stored_name, bytes });
    }
    Ok(attachments)
}

fn validate_attachment_set(
    connection: &Connection,
    attachments: &[BackupAttachment],
) -> Result<(), String> {
    let mut statement = connection.prepare("SELECT stored_name,size_bytes,sha256,media_type FROM transaction_attachments ORDER BY stored_name")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let expected = rows
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())?;
    if expected.len() != attachments.len() {
        return Err(
            "The backup is missing transaction attachment files or contains unreferenced files."
                .into(),
        );
    }
    for (name, size, hash, media_type) in expected {
        let file = attachments
            .iter()
            .find(|item| item.stored_name == name)
            .ok_or("The backup is missing a transaction attachment file.")?;
        let actual_hash = format!("{:x}", Sha256::digest(&file.bytes));
        let valid_media = match media_type.as_str() {
            "application/pdf" => file.bytes.starts_with(b"%PDF-"),
            "image/jpeg" => file.bytes.starts_with(&[0xff, 0xd8, 0xff]),
            "image/png" => file.bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
            _ => false,
        };
        if file.bytes.len() as i64 != size || actual_hash != hash || !valid_media {
            return Err("A transaction attachment failed its integrity check.".into());
        }
    }
    Ok(())
}

fn validate_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.chars().count() < MIN_PASSPHRASE_CHARS {
        return Err("Use a passphrase with at least 12 characters.".into());
    }
    Ok(())
}

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_BYTES] {
    let mut key = [0u8; KEY_BYTES];
    pbkdf2_hmac::<PbkdfSha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
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

pub fn create_encrypted_backup(
    database: &Database,
    app_data_dir: &Path,
    passphrase: &str,
) -> Result<String, String> {
    validate_passphrase(passphrase)?;
    let mut connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    let database_bytes = connection
        .serialize(MAIN_DB)
        .map_err(|e| format!("Could not create database snapshot: {e}"))?
        .to_vec();
    let attachments = collect_attachments(&connection, &app_data_dir.join("attachments"))?;
    drop(connection);
    let plaintext = encode_payload(database_bytes, attachments)?;
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
    app_data_dir: &Path,
    encoded: &str,
    passphrase: &str,
) -> Result<(), String> {
    validate_passphrase(passphrase)?;
    let encrypted = STANDARD
        .decode(encoded)
        .map_err(|_| "Backup file encoding is invalid.")?;
    let plaintext = decrypt_snapshot(&encrypted, passphrase)?;
    let mut payload = decode_payload(plaintext)?;
    let source_result = (|| {
        let source = load_validated_database(&mut payload.database)?;
        validate_attachment_set(&source, &payload.attachments)?;
        Ok::<_, String>(source)
    })();
    payload.database.fill(0);
    let source = source_result?;

    let attachments_root = app_data_dir.join("attachments");
    let old_names = current_attachment_names(database)?;
    let mut rollback =
        install_attachment_files(app_data_dir, &attachments_root, &payload.attachments)?;
    let mut connection = match database.connection().lock() {
        Ok(connection) => connection,
        Err(_) => {
            rollback.restore();
            return Err("Database unavailable".into());
        }
    };
    let backup = match Backup::new(&source, &mut connection) {
        Ok(backup) => backup,
        Err(error) => {
            rollback.restore();
            return Err(format!("Could not prepare database restore: {error}"));
        }
    };
    if let Err(error) = backup.run_to_completion(5, std::time::Duration::from_millis(250), None) {
        drop(backup);
        drop(connection);
        rollback.restore();
        return Err(format!("Could not restore the database: {error}"));
    }
    drop(backup);
    if let Err(error) = connection.pragma_update(None, "journal_mode", "WAL") {
        drop(connection);
        rollback.finish(&old_names, &payload.attachments);
        return Err(format!(
            "Database restored, but WAL mode could not be re-enabled: {error}"
        ));
    }
    if let Err(error) = connection.execute(
        "DELETE FROM import_source_files WHERE expires_at <= CURRENT_TIMESTAMP",
        [],
    ) {
        drop(connection);
        rollback.finish(&old_names, &payload.attachments);
        return Err(format!(
            "Backup restored, but expired import source files could not be cleaned up: {error}"
        ));
    }
    drop(connection);
    rollback.finish(&old_names, &payload.attachments);
    Ok(())
}

fn current_attachment_names(database: &Database) -> Result<Vec<String>, String> {
    let connection = database
        .connection()
        .lock()
        .map_err(|_| "Database unavailable")?;
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='transaction_attachments')",
        [], |row| row.get(0),
    ).map_err(|e| e.to_string())?;
    if !exists {
        return Ok(Vec::new());
    }
    let mut statement = connection
        .prepare("SELECT stored_name FROM transaction_attachments")
        .map_err(|e| e.to_string())?;
    let names = statement
        .query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<rusqlite::Result<Vec<String>>>()
        .map_err(|e| e.to_string())?;
    Ok(names)
}

struct AttachmentRollback {
    root: PathBuf,
    before: Vec<(String, Option<Vec<u8>>)>,
}

impl AttachmentRollback {
    fn restore(&mut self) {
        for (name, previous) in &self.before {
            let path = self.root.join(name);
            if let Some(bytes) = previous {
                let _ = fs::write(path, bytes);
            } else {
                let _ = fs::remove_file(path);
            }
        }
    }

    fn finish(&mut self, old_names: &[String], new_attachments: &[BackupAttachment]) {
        let keep = new_attachments
            .iter()
            .map(|item| item.stored_name.as_str())
            .collect::<std::collections::HashSet<_>>();
        for name in old_names {
            if valid_stored_name(name) && !keep.contains(name.as_str()) {
                let _ = fs::remove_file(self.root.join(name));
            }
        }
        if let Ok(entries) = fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if valid_stored_name(&name) && !keep.contains(name.as_str()) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
        self.before.clear();
    }
}

fn install_attachment_files(
    app_data_dir: &Path,
    root: &Path,
    attachments: &[BackupAttachment],
) -> Result<AttachmentRollback, String> {
    fs::create_dir_all(root).map_err(|_| "Attachment storage is unavailable.")?;
    let stage = app_data_dir.join(format!(".attachments-restore-{}", Uuid::new_v4()));
    fs::create_dir_all(&stage).map_err(|_| "Could not prepare attachment restore.")?;
    for item in attachments {
        if let Err(error) = fs::write(stage.join(&item.stored_name), &item.bytes) {
            let _ = fs::remove_dir_all(&stage);
            return Err(format!("Could not stage a restored attachment: {error}"));
        }
    }
    let mut rollback = AttachmentRollback {
        root: root.to_owned(),
        before: Vec::new(),
    };
    for item in attachments {
        let destination = root.join(&item.stored_name);
        let previous = if destination.exists() {
            if !fs::symlink_metadata(&destination)
                .is_ok_and(|metadata| metadata.file_type().is_file())
            {
                rollback.restore();
                let _ = fs::remove_dir_all(&stage);
                return Err("An existing attachment path is not a regular file.".into());
            }
            match fs::read(&destination) {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    rollback.restore();
                    let _ = fs::remove_dir_all(&stage);
                    return Err(format!(
                        "Could not protect existing attachment data: {error}"
                    ));
                }
            }
        } else {
            None
        };
        rollback.before.push((item.stored_name.clone(), previous));
        if let Err(error) = fs::copy(stage.join(&item.stored_name), &destination) {
            rollback.restore();
            let _ = fs::remove_dir_all(&stage);
            return Err(format!("Could not restore an attachment file: {error}"));
        }
    }
    let _ = fs::remove_dir_all(&stage);
    Ok(rollback)
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
        let source_data =
            std::env::temp_dir().join(format!("etracker-backup-data-{}", Uuid::new_v4()));
        fs::create_dir_all(&source_data).unwrap();
        source.connection().lock().unwrap()
            .execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('restore-test','Restored cash','cash','KES')", []).unwrap();
        let encoded = create_encrypted_backup(&source, &source_data, PASSPHRASE).unwrap();

        let destination = Database::initialize(Path::new(":memory:")).unwrap();
        let destination_data =
            std::env::temp_dir().join(format!("etracker-restore-data-{}", Uuid::new_v4()));
        fs::create_dir_all(&destination_data).unwrap();
        destination.connection().lock().unwrap()
            .execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('old','Old cash','cash','KES')", []).unwrap();
        restore_encrypted_backup(&destination, &destination_data, &encoded, PASSPHRASE).unwrap();
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
            restore_encrypted_backup(
                &destination,
                &destination_data,
                &encoded,
                "another-invalid-passphrase"
            )
            .is_err()
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
                &destination_data,
                &STANDARD.encode(b"not a backup file"),
                PASSPHRASE
            )
            .is_err()
        );
        fs::remove_dir_all(source_data).unwrap();
        fs::remove_dir_all(destination_data).unwrap();
    }

    #[test]
    fn encrypted_backup_restores_attachment_bytes_with_database_metadata() {
        let source = Database::initialize(Path::new(":memory:")).unwrap();
        source.connection().lock().unwrap().execute(
            "INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('receipt-transaction','expense',1200,'KES','2026-11-08T10:00:00Z','manual')",
            [],
        ).unwrap();
        let source_data =
            std::env::temp_dir().join(format!("etracker-backup-data-{}", Uuid::new_v4()));
        let source_files = source_data.join("attachments");
        fs::create_dir_all(&source_files).unwrap();
        let stored_name = format!("{}.png", Uuid::new_v4());
        let contents = b"\x89PNG\r\n\x1a\nfixture attachment";
        fs::write(source_files.join(&stored_name), contents).unwrap();
        source.connection().lock().unwrap().execute(
            "INSERT INTO transaction_attachments(id,transaction_id,stored_name,original_name,media_type,size_bytes,sha256) VALUES (?1,'receipt-transaction',?2,'receipt.png','image/png',?3,?4)",
            rusqlite::params![Uuid::new_v4().to_string(), stored_name, contents.len() as i64, format!("{:x}", Sha256::digest(contents))],
        ).unwrap();
        let encoded = create_encrypted_backup(&source, &source_data, PASSPHRASE).unwrap();
        let destination = Database::initialize(Path::new(":memory:")).unwrap();
        let destination_data =
            std::env::temp_dir().join(format!("etracker-restore-data-{}", Uuid::new_v4()));
        fs::create_dir_all(&destination_data).unwrap();
        restore_encrypted_backup(&destination, &destination_data, &encoded, PASSPHRASE).unwrap();
        let restored_name: String = destination.connection().lock().unwrap().query_row(
            "SELECT stored_name FROM transaction_attachments WHERE transaction_id='receipt-transaction'",
            [], |row| row.get(0),
        ).unwrap();
        assert_eq!(
            fs::read(destination_data.join("attachments").join(restored_name)).unwrap(),
            contents
        );
        fs::remove_dir_all(source_data).unwrap();
        fs::remove_dir_all(destination_data).unwrap();
    }
}
