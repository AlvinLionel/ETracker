use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use uuid::Uuid;

const KEY_BYTES: usize = 32;
const KEY_FILE_LIMIT: u64 = 4096;

pub struct DataKey([u8; KEY_BYTES]);

impl DataKey {
    pub fn from_bytes(bytes: [u8; KEY_BYTES]) -> Self {
        Self(bytes)
    }

    pub fn bytes(&self) -> &[u8; KEY_BYTES] {
        &self.0
    }

    pub fn attachment_key(&self) -> [u8; KEY_BYTES] {
        let mut hasher = Sha256::new();
        hasher.update(b"etracker-attachment-encryption-v1");
        hasher.update(self.0);
        hasher.finalize().into()
    }

    pub fn load_or_create(path: &Path, database_path: &Path) -> Result<Self, String> {
        if path.exists() {
            return read_key(path);
        }

        if is_encrypted_database(database_path)? {
            return Err(
                "The local encryption key is missing. Restore an encrypted ETracker backup.".into(),
            );
        }

        let mut bytes = [0; KEY_BYTES];
        getrandom::fill(&mut bytes).map_err(|error| {
            format!("A secure local encryption key could not be generated: {error}")
        })?;
        let key = Self(bytes);
        persist_key(path, &key)?;
        read_key(path)
    }
}

impl Drop for DataKey {
    fn drop(&mut self) {
        self.0.fill(0);
    }
}

fn is_encrypted_database(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    let mut file = fs::File::open(path)
        .map_err(|_| "The local database could not be checked for encryption.")?;
    let mut header = [0u8; 16];
    let count = file
        .read(&mut header)
        .map_err(|_| "The local database could not be checked for encryption.")?;
    Ok(count > 0 && !header.starts_with(b"SQLite format 3\0"))
}

#[cfg(windows)]
fn read_key(path: &Path) -> Result<DataKey, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| "The local encryption key could not be read.")?;
    if !metadata.file_type().is_file() || metadata.len() > KEY_FILE_LIMIT {
        return Err("The local encryption key file is invalid.".into());
    }
    let encrypted = fs::read(path).map_err(|_| "The local encryption key could not be read.")?;
    let mut plaintext = dpapi_unprotect(&encrypted)?;
    if plaintext.len() != KEY_BYTES {
        plaintext.fill(0);
        return Err("The local encryption key file is invalid.".into());
    }
    let mut bytes = [0; KEY_BYTES];
    bytes.copy_from_slice(&plaintext);
    plaintext.fill(0);
    Ok(DataKey::from_bytes(bytes))
}

#[cfg(not(windows))]
fn read_key(_path: &Path) -> Result<DataKey, String> {
    Err("Local data encryption is currently supported only on Windows.".into())
}

#[cfg(windows)]
fn persist_key(path: &Path, key: &DataKey) -> Result<(), String> {
    let protected = dpapi_protect(key.bytes())?;
    let temporary = temporary_key_path(path);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "The local encryption key could not be stored safely.")?;
    let result = file.write_all(&protected).and_then(|_| file.sync_all());
    drop(file);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("The local encryption key could not be stored safely.".into());
    }
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        if path.exists() {
            return Ok(());
        }
        return Err("The local encryption key could not be stored safely.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn persist_key(_path: &Path, _key: &DataKey) -> Result<(), String> {
    Err("Local data encryption is currently supported only on Windows.".into())
}

fn temporary_key_path(path: &Path) -> PathBuf {
    path.with_extension(format!("key-new-{}", Uuid::new_v4()))
}

#[cfg(windows)]
fn dpapi_protect(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::null;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: plaintext.len() as u32,
        pbData: plaintext.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let succeeded = unsafe {
        CryptProtectData(
            &input,
            null(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if succeeded == 0 || output.pbData.is_null() || output.cbData == 0 {
        return Err(
            "The local encryption key could not be protected for this Windows user.".into(),
        );
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        windows_sys::Win32::Foundation::LocalFree(output.pbData.cast());
    }
    Ok(bytes)
}

#[cfg(windows)]
fn dpapi_unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, String> {
    use std::ptr::null;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData,
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: ciphertext.len() as u32,
        pbData: ciphertext.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let succeeded = unsafe {
        CryptUnprotectData(
            &input,
            null::<*mut u16>().cast_mut(),
            null(),
            null(),
            null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if succeeded == 0 || output.pbData.is_null() || output.cbData == 0 {
        return Err("The local encryption key cannot be unlocked for this Windows user.".into());
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    unsafe {
        std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize).fill(0);
        windows_sys::Win32::Foundation::LocalFree(output.pbData.cast());
    }
    Ok(bytes)
}

#[cfg(windows)]
pub fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let succeeded = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded == 0 {
        return Err("The encrypted data file could not safely replace the existing file.".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination)
        .map_err(|_| "The encrypted data file could not safely replace the existing file.".into())
}

#[cfg(all(test, windows))]
mod tests {
    use super::{DataKey, read_key, replace_file};
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn dpapi_protected_key_is_reusable_only_by_the_current_windows_user() {
        let root = std::env::temp_dir().join(format!("etracker-key-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let key_path = root.join("etracker.key");
        let key = DataKey::from_bytes([7; 32]);
        super::persist_key(&key_path, &key).unwrap();
        assert_eq!(read_key(&key_path).unwrap().bytes(), key.bytes());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generated_key_is_reloaded_from_the_protected_key_file() {
        let root = std::env::temp_dir().join(format!("etracker-key-create-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let key_path = root.join("etracker.key");
        let database_path = root.join("etracker.db");

        let key = DataKey::load_or_create(&key_path, &database_path).unwrap();
        let loaded = DataKey::load_or_create(&key_path, &database_path).unwrap();
        assert_eq!(loaded.bytes(), key.bytes());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_key_never_replaces_an_existing_encrypted_database() {
        let root = std::env::temp_dir().join(format!("etracker-missing-key-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let key_path = root.join("etracker.key");
        let database_path = root.join("etracker.db");
        fs::write(&database_path, b"encrypted database fixture").unwrap();

        let error = match DataKey::load_or_create(&key_path, &database_path) {
            Ok(_) => panic!("missing local key should not be recreated"),
            Err(error) => error,
        };
        assert!(error.contains("key is missing"));
        assert!(!key_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_dpapi_key_file_is_rejected() {
        let root = std::env::temp_dir().join(format!("etracker-invalid-key-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let key_path = root.join("etracker.key");
        fs::write(&key_path, b"not a DPAPI protected key").unwrap();

        assert!(read_key(&key_path).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn atomic_replace_replaces_the_destination_file() {
        let root: PathBuf =
            std::env::temp_dir().join(format!("etracker-replace-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("new");
        let destination = root.join("old");
        fs::write(&source, b"encrypted").unwrap();
        fs::write(&destination, b"plaintext").unwrap();
        replace_file(&source, &destination).unwrap();
        assert_eq!(fs::read(destination).unwrap(), b"encrypted");
        fs::remove_dir_all(root).unwrap();
    }
}
