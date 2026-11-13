use crate::infrastructure::database::Database;
use crate::infrastructure::security::replace_file;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce, aead::Aead};
use getrandom::fill as fill_random;
use rusqlite::params;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub const MAX_ATTACHMENT_BYTES: u64 = 10 * 1024 * 1024;
pub const MAX_TRANSACTION_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const ENCRYPTED_ATTACHMENT_MAGIC: &[u8; 8] = b"ETRATT01";
const NONCE_BYTES: usize = 12;
const TEMP_ATTACHMENT_DIR: &str = ".attachment-viewer-temp";

#[derive(Clone, Debug, Serialize)]
pub struct AttachmentRecord {
    pub id: String,
    pub transaction_id: String,
    pub original_name: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub created_at: String,
    pub missing: bool,
    pub damaged: bool,
}

pub struct SqliteAttachmentRepository<'a> {
    database: &'a Database,
    root: PathBuf,
    temporary_root: PathBuf,
    key: Option<[u8; 32]>,
}

impl<'a> SqliteAttachmentRepository<'a> {
    pub fn new(database: &'a Database, app_data_dir: &Path) -> Self {
        Self {
            database,
            root: app_data_dir.join("attachments"),
            temporary_root: app_data_dir.join(TEMP_ATTACHMENT_DIR),
            key: database.data_key().map(|key| key.attachment_key()),
        }
    }

    pub fn migrate_to_encrypted(&self) -> Result<(), String> {
        let Some(key) = self.key.as_ref() else {
            return Ok(());
        };
        fs::create_dir_all(&self.root).map_err(|_| "Attachment storage is unavailable.")?;
        self.remove_stale_encryption_files()?;
        let attachments = {
            let connection = self
                .database
                .connection()
                .lock()
                .map_err(|_| "Database unavailable")?;
            let mut statement = connection
                .prepare("SELECT stored_name,size_bytes,sha256,media_type FROM transaction_attachments ORDER BY stored_name")
                .map_err(|error| error.to_string())?;
            statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|error| error.to_string())?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|error| error.to_string())?
        };

        for (stored_name, expected_size, expected_hash, media_type) in attachments {
            if !valid_stored_name(&stored_name) {
                return Err("The database contains an invalid attachment reference.".into());
            }
            let path = self.root.join(&stored_name);
            let metadata = fs::symlink_metadata(&path).map_err(
                |_| "A transaction attachment is missing; encryption was not completed.",
            )?;
            if !metadata.file_type().is_file() || metadata.len() > MAX_ATTACHMENT_BYTES + 64 {
                return Err("A transaction attachment is not a supported regular file.".into());
            }
            let stored_bytes =
                fs::read(&path).map_err(|_| "A transaction attachment could not be read.")?;
            if stored_bytes.starts_with(ENCRYPTED_ATTACHMENT_MAGIC) {
                let plaintext = decrypt_attachment(&stored_bytes, key)?;
                validate_plaintext_attachment(
                    &plaintext,
                    expected_size,
                    &expected_hash,
                    &media_type,
                )?;
                continue;
            }
            validate_plaintext_attachment(
                &stored_bytes,
                expected_size,
                &expected_hash,
                &media_type,
            )?;
            let encrypted = encrypt_attachment(&stored_bytes, key)?;
            let temporary = self
                .root
                .join(format!(".{stored_name}.{}.encrypting", Uuid::new_v4()));
            write_new_file(&temporary, &encrypted)?;
            if let Err(error) = replace_file(&temporary, &path) {
                let _ = fs::remove_file(&temporary);
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn cleanup_temporary_decrypted_files(&self) -> Result<(), String> {
        let metadata = match fs::symlink_metadata(&self.temporary_root) {
            Ok(metadata) if metadata.file_type().is_dir() => Some(metadata),
            Ok(_) => return Err("Temporary attachment storage is not a directory.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err("Temporary attachment files could not be checked.".into()),
        };
        if metadata.is_none() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.temporary_root)
            .map_err(|_| "Temporary attachment files could not be listed.")?
        {
            let entry = entry.map_err(|_| "Temporary attachment files could not be listed.")?;
            let file_type = entry
                .file_type()
                .map_err(|_| "A temporary attachment could not be checked.")?;
            if file_type.is_file() {
                fs::remove_file(entry.path())
                    .map_err(|_| "A temporary attachment could not be removed.")?;
            }
        }
        Ok(())
    }

    pub fn list(&self, transaction_id: &str) -> Result<Vec<AttachmentRecord>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let mut statement = connection
            .prepare("SELECT id,transaction_id,stored_name,original_name,media_type,size_bytes,sha256,created_at FROM transaction_attachments WHERE transaction_id=?1 ORDER BY created_at,id")
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map([transaction_id], |row| {
                let stored_name: String = row.get(2)?;
                let size: i64 = row.get(5)?;
                Ok((
                    AttachmentRecord {
                        id: row.get(0)?,
                        transaction_id: row.get(1)?,
                        original_name: row.get(3)?,
                        media_type: row.get(4)?,
                        size_bytes: size.max(0) as u64,
                        sha256: row.get(6)?,
                        created_at: row.get(7)?,
                        missing: false,
                        damaged: false,
                    },
                    stored_name,
                ))
            })
            .map_err(|e| e.to_string())?;
        rows.map(|row| {
            let (mut record, stored_name) = row.map_err(|e| e.to_string())?;
            if !valid_stored_name(&stored_name) {
                record.damaged = true;
                return Ok(record);
            }
            let path = self.root.join(stored_name);
            let metadata = fs::symlink_metadata(&path);
            if !metadata
                .as_ref()
                .is_ok_and(|metadata| metadata.file_type().is_file())
            {
                record.missing = true;
            } else if metadata.as_ref().is_ok_and(|metadata| {
                metadata.len() > MAX_ATTACHMENT_BYTES + 64
                    || (self.key.is_none() && metadata.len() != record.size_bytes)
            }) {
                record.damaged = true;
            } else if let Ok(stored_bytes) = fs::read(path) {
                let bytes = match self.key.as_ref() {
                    Some(key) => decrypt_attachment(&stored_bytes, key),
                    None => Ok(stored_bytes),
                };
                record.damaged = bytes.map_or(true, |bytes| {
                    bytes.len() as u64 != record.size_bytes
                        || format!("{:x}", Sha256::digest(&bytes)) != record.sha256
                        || !matches_content(&bytes, &record.media_type)
                });
            } else {
                record.missing = true;
            }
            Ok(record)
        })
        .collect()
    }

    pub fn add(&self, transaction_id: &str, source: &Path) -> Result<AttachmentRecord, String> {
        let info = fs::symlink_metadata(source).map_err(|_| "The selected file is unavailable.")?;
        if !info.file_type().is_file() {
            return Err("Choose a regular PDF, JPEG, or PNG file.".into());
        }
        if info.len() == 0 || info.len() > MAX_ATTACHMENT_BYTES {
            return Err("Attachments must be between 1 byte and 10 MiB.".into());
        }
        let original_name = safe_original_name(source)?;
        let (extension, media_type) = allowed_type(source)?;
        let mut bytes = Vec::with_capacity(info.len() as usize);
        File::open(source)
            .map_err(|_| "The selected file could not be read.")?
            .take(MAX_ATTACHMENT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "The selected file could not be read.")?;
        if bytes.len() as u64 != info.len() || !matches_content(&bytes, &media_type) {
            return Err("The selected file content does not match its file type.".into());
        }
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let stored_bytes = match self.key.as_ref() {
            Some(key) => encrypt_attachment(&bytes, key)?,
            None => bytes.clone(),
        };
        let id = Uuid::new_v4().to_string();
        let stored_name = format!("{id}.{extension}");
        fs::create_dir_all(&self.root).map_err(|_| "Attachment storage is unavailable.")?;
        let path = self.root.join(&stored_name);
        write_new_file(&path, &stored_bytes)?;

        let insert_result = (|| {
            let connection = self
                .database
                .connection()
                .lock()
                .map_err(|_| "Database unavailable")?;
            let transaction = connection
                .unchecked_transaction()
                .map_err(|e| e.to_string())?;
            let current: i64 = transaction
                .query_row(
                    "SELECT COALESCE(SUM(size_bytes),0) FROM transaction_attachments WHERE transaction_id=?1",
                    [transaction_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if current < 0 || current as u64 + info.len() > MAX_TRANSACTION_ATTACHMENT_BYTES {
                return Err("Attachments for this transaction cannot exceed 25 MiB.".into());
            }
            transaction
                .execute(
                    "INSERT INTO transaction_attachments(id,transaction_id,stored_name,original_name,media_type,size_bytes,sha256) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![id, transaction_id, stored_name, original_name, media_type, info.len() as i64, digest],
                )
                .map_err(|error| {
                    if error.to_string().contains("FOREIGN KEY") {
                        "The transaction no longer exists.".to_string()
                    } else {
                        error.to_string()
                    }
                })?;
            transaction.commit().map_err(|e| e.to_string())?;
            Ok(())
        })();
        if let Err(error) = insert_result {
            let _ = fs::remove_file(&path);
            return Err(error);
        }

        Ok(AttachmentRecord {
            id,
            transaction_id: transaction_id.to_owned(),
            original_name,
            media_type,
            size_bytes: info.len(),
            sha256: digest,
            created_at: "".into(),
            missing: false,
            damaged: false,
        })
    }

    pub fn file_path(&self, attachment_id: &str) -> Result<PathBuf, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let (stored_name, expected_size, expected_hash, media_type): (String, i64, String, String) = connection
            .query_row(
                "SELECT stored_name,size_bytes,sha256,media_type FROM transaction_attachments WHERE id=?1",
                [attachment_id],
                |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)),
            )
            .map_err(|_| "The attachment could not be found.")?;
        if !valid_stored_name(&stored_name) {
            return Err("The attachment reference is invalid.".into());
        }
        let path = self.root.join(&stored_name);
        if !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_file()) {
            return Err("The attachment file is missing or inaccessible.".into());
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| "The attachment file is missing or inaccessible.")?;
        if metadata.len() > MAX_ATTACHMENT_BYTES + 64
            || (self.key.is_none() && metadata.len() != expected_size.max(0) as u64)
        {
            return Err("The attachment file failed its integrity check.".into());
        }
        let stored_bytes =
            fs::read(&path).map_err(|_| "The attachment file is missing or inaccessible.")?;
        let bytes = match self.key.as_ref() {
            Some(key) => decrypt_attachment(&stored_bytes, key)?,
            None => stored_bytes,
        };
        if bytes.len() as i64 != expected_size
            || format!("{:x}", Sha256::digest(&bytes)) != expected_hash
            || !matches_content(&bytes, &media_type)
        {
            return Err("The attachment file failed its integrity check.".into());
        }
        if self.key.is_none() {
            return Ok(path);
        }
        fs::create_dir_all(&self.temporary_root)
            .map_err(|_| "Temporary attachment storage is unavailable.")?;
        let temporary_path = self.temporary_root.join(format!(
            "{}.{}",
            Uuid::new_v4(),
            Path::new(&stored_name)
                .extension()
                .unwrap()
                .to_string_lossy()
        ));
        write_new_file(&temporary_path, &bytes)?;
        Ok(temporary_path)
    }

    pub fn remove(&self, attachment_id: &str) -> Result<(), String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let stored_name: String = connection
            .query_row(
                "SELECT stored_name FROM transaction_attachments WHERE id=?1",
                [attachment_id],
                |row| row.get(0),
            )
            .map_err(|_| "The attachment could not be found.")?;
        if !valid_stored_name(&stored_name) {
            return Err("The attachment reference is invalid.".into());
        }
        let path = self.root.join(stored_name);
        if path.exists() {
            fs::remove_file(&path).map_err(|_| "The attachment file could not be removed.")?;
        }
        let deleted = connection
            .execute(
                "DELETE FROM transaction_attachments WHERE id=?1",
                [attachment_id],
            )
            .map_err(|error| error.to_string());
        match deleted {
            Ok(0) => Err("The attachment could not be found.".into()),
            Ok(_) => Ok(()),
            // If this uncommon database failure follows file deletion, the remaining metadata
            // makes the loss visible as a missing attachment instead of leaving an untracked file.
            Err(error) => Err(format!(
                "The file was removed but its record could not be updated: {error}"
            )),
        }
    }
}

impl Drop for SqliteAttachmentRepository<'_> {
    fn drop(&mut self) {
        if let Some(key) = self.key.as_mut() {
            key.fill(0);
        }
    }
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "The attachment could not be stored safely.")?;
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(path);
        return Err("The attachment could not be stored completely.".into());
    }
    Ok(())
}

pub(crate) fn encrypt_attachment(plaintext: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| "Attachment encryption could not be initialized.")?;
    let mut nonce_bytes = [0; NONCE_BYTES];
    fill_random(&mut nonce_bytes)
        .map_err(|error| format!("Secure random generation failed: {error}"))?;
    let nonce = Nonce::try_from(nonce_bytes.as_slice())
        .map_err(|_| "Attachment encryption nonce is invalid.")?;
    let ciphertext = cipher
        .encrypt(&nonce, plaintext)
        .map_err(|_| "The attachment could not be encrypted.")?;
    let mut stored =
        Vec::with_capacity(ENCRYPTED_ATTACHMENT_MAGIC.len() + NONCE_BYTES + ciphertext.len());
    stored.extend_from_slice(ENCRYPTED_ATTACHMENT_MAGIC);
    stored.extend_from_slice(&nonce_bytes);
    stored.extend_from_slice(&ciphertext);
    Ok(stored)
}

pub(crate) fn decrypt_attachment(stored: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, String> {
    let minimum_size = ENCRYPTED_ATTACHMENT_MAGIC.len() + NONCE_BYTES + 16;
    if stored.len() < minimum_size || !stored.starts_with(ENCRYPTED_ATTACHMENT_MAGIC) {
        return Err("The encrypted attachment format is invalid.".into());
    }
    let nonce_start = ENCRYPTED_ATTACHMENT_MAGIC.len();
    let data_start = nonce_start + NONCE_BYTES;
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| "Attachment encryption could not be initialized.")?;
    let nonce = Nonce::try_from(&stored[nonce_start..data_start])
        .map_err(|_| "The encrypted attachment nonce is invalid.")?;
    cipher
        .decrypt(&nonce, &stored[data_start..])
        .map_err(|_| "The attachment could not be decrypted or has been changed.".into())
}

fn validate_plaintext_attachment(
    bytes: &[u8],
    expected_size: i64,
    expected_hash: &str,
    media_type: &str,
) -> Result<(), String> {
    if bytes.len() as i64 != expected_size
        || format!("{:x}", Sha256::digest(bytes)) != expected_hash
        || !matches_content(bytes, media_type)
    {
        return Err("A transaction attachment failed its integrity check.".into());
    }
    Ok(())
}

impl SqliteAttachmentRepository<'_> {
    fn remove_stale_encryption_files(&self) -> Result<(), String> {
        for entry in
            fs::read_dir(&self.root).map_err(|_| "Attachment storage could not be listed.")?
        {
            let entry = entry.map_err(|_| "Attachment storage could not be listed.")?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.')
                && name.ends_with(".encrypting")
                && entry.file_type().is_ok_and(|file_type| file_type.is_file())
            {
                fs::remove_file(entry.path())
                    .map_err(|_| "An incomplete encrypted attachment could not be removed.")?;
            }
        }
        Ok(())
    }
}

fn safe_original_name(path: &Path) -> Result<String, String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("The selected file name is invalid.")?;
    let cleaned: String = name
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect();
    if cleaned.trim().is_empty() {
        return Err("The selected file name is invalid.".into());
    }
    Ok(cleaned)
}

fn allowed_type(path: &Path) -> Result<(&'static str, String), String> {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => Ok(("pdf", "application/pdf".into())),
        "jpg" | "jpeg" => Ok(("jpg", "image/jpeg".into())),
        "png" => Ok(("png", "image/png".into())),
        _ => Err("Choose a PDF, JPEG, or PNG file.".into()),
    }
}

fn matches_content(bytes: &[u8], media_type: &str) -> bool {
    match media_type {
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        _ => false,
    }
}

fn valid_stored_name(value: &str) -> bool {
    let Some((stem, extension)) = value.rsplit_once('.') else {
        return false;
    };
    Uuid::parse_str(stem).is_ok() && matches!(extension, "pdf" | "jpg" | "png")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::{database::Database, security::DataKey};

    fn encrypted_database() -> Database {
        Database::initialize_encrypted(Path::new(":memory:"), DataKey::from_bytes([31; 32]))
            .unwrap()
    }

    fn insert_attachment_transaction(database: &Database) {
        database.connection().lock().unwrap().execute(
            "INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('attachment-transaction','expense',100,'KES','2026-11-08T10:00:00Z','manual')",
            [],
        ).unwrap();
    }

    #[test]
    fn encrypted_attachments_round_trip_and_are_only_decrypted_in_temp_storage() {
        let root = std::env::temp_dir().join(format!("etracker-attachments-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source = root.with_extension("source.png");
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(b"private receipt bytes");
        fs::write(&source, &png).unwrap();

        let database = encrypted_database();
        insert_attachment_transaction(&database);
        let repository = SqliteAttachmentRepository::new(&database, &root);
        let attachment = repository.add("attachment-transaction", &source).unwrap();
        let stored_path = root
            .join("attachments")
            .join(format!("{}.png", attachment.id));
        let stored_bytes = fs::read(&stored_path).unwrap();
        assert!(stored_bytes.starts_with(ENCRYPTED_ATTACHMENT_MAGIC));
        assert!(
            !stored_bytes
                .windows(b"private receipt bytes".len())
                .any(|window| window == b"private receipt bytes")
        );
        assert!(!repository.list("attachment-transaction").unwrap()[0].damaged);

        let opened_path = repository.file_path(&attachment.id).unwrap();
        assert_eq!(fs::read(&opened_path).unwrap(), png);
        assert!(opened_path.starts_with(&root.join(TEMP_ATTACHMENT_DIR)));
        repository.cleanup_temporary_decrypted_files().unwrap();
        assert!(!opened_path.exists());

        fs::remove_dir_all(&root).unwrap();
        fs::remove_file(source).unwrap();
    }

    #[test]
    fn migrates_existing_plaintext_attachments_without_changing_metadata() {
        let root = std::env::temp_dir().join(format!("etracker-attachments-{}", Uuid::new_v4()));
        let attachment_root = root.join("attachments");
        fs::create_dir_all(&attachment_root).unwrap();
        let source = root.with_extension("source.png");
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(b"legacy receipt bytes");
        fs::write(&source, &png).unwrap();

        let database = encrypted_database();
        insert_attachment_transaction(&database);
        let id = Uuid::new_v4().to_string();
        let digest = format!("{:x}", Sha256::digest(&png));
        database.connection().lock().unwrap().execute(
            "INSERT INTO transaction_attachments(id,transaction_id,stored_name,original_name,media_type,size_bytes,sha256) VALUES (?1,'attachment-transaction',?2,'receipt.png','image/png',?3,?4)",
            params![id, format!("{id}.png"), png.len() as i64, digest],
        ).unwrap();
        let legacy_path = attachment_root.join(format!("{id}.png"));
        fs::write(&legacy_path, &png).unwrap();

        let repository = SqliteAttachmentRepository::new(&database, &root);
        repository.migrate_to_encrypted().unwrap();

        let encrypted = fs::read(&legacy_path).unwrap();
        assert!(encrypted.starts_with(ENCRYPTED_ATTACHMENT_MAGIC));
        assert_eq!(repository.file_path(&id).unwrap().exists(), true);
        assert!(!repository.list("attachment-transaction").unwrap()[0].damaged);

        fs::remove_dir_all(&root).unwrap();
        fs::remove_file(source).unwrap();
    }

    #[test]
    fn stores_and_lists_only_supported_matching_documents() {
        let root = std::env::temp_dir().join(format!("etracker-attachments-{}", Uuid::new_v4()));
        let source = root.with_extension("source.png");
        fs::create_dir_all(&root).unwrap();
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(b"minimal image fixture");
        fs::write(&source, &png).unwrap();
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        database.connection().lock().unwrap().execute(
            "INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('attachment-transaction','expense',100,'KES','2026-11-08T10:00:00Z','manual')",
            [],
        ).unwrap();
        let repository = SqliteAttachmentRepository::new(&database, &root);
        let attachment = repository.add("attachment-transaction", &source).unwrap();
        assert_eq!(attachment.media_type, "image/png");
        assert_eq!(repository.list("attachment-transaction").unwrap().len(), 1);
        let stored_path = repository.file_path(&attachment.id).unwrap();
        assert!(stored_path.is_file());
        fs::write(&stored_path, b"corrupted").unwrap();
        let listed = repository.list("attachment-transaction").unwrap();
        assert!(listed[0].damaged);
        assert!(repository.file_path(&attachment.id).is_err());
        fs::write(&stored_path, &png).unwrap();
        repository.remove(&attachment.id).unwrap();
        assert!(
            repository
                .list("attachment-transaction")
                .unwrap()
                .is_empty()
        );
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(source).unwrap();
    }

    #[test]
    fn reports_missing_attachment_files_and_rejects_mismatched_content() {
        let root = std::env::temp_dir().join(format!("etracker-attachments-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source = root.with_extension("source.png");
        fs::write(&source, b"not a png").unwrap();
        let database = Database::initialize(Path::new(":memory:")).unwrap();
        database.connection().lock().unwrap().execute(
            "INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('attachment-transaction','expense',100,'KES','2026-11-08T10:00:00Z','manual')",
            [],
        ).unwrap();
        let repository = SqliteAttachmentRepository::new(&database, &root);
        assert!(repository.add("attachment-transaction", &source).is_err());
        database.connection().lock().unwrap().execute(
            "INSERT INTO transaction_attachments(id,transaction_id,stored_name,original_name,media_type,size_bytes,sha256) VALUES ('missing','attachment-transaction',?1,'missing.png','image/png',8,printf('%064d',0))",
            [format!("{}.png", Uuid::new_v4())],
        ).unwrap();
        let rows = repository.list("attachment-transaction").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].missing);
        fs::remove_dir_all(root).unwrap();
        fs::remove_file(source).unwrap();
    }
}
