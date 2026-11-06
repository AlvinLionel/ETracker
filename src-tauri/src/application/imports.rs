#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewImportBatch {
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub source_file_hash: Option<String>,
}
#[derive(Debug)]
pub enum ImportError {
    Database(String),
    ConnectionPoisoned,
    InvalidBatch,
}
pub trait ImportRepository {
    fn create_batch(&self, batch: &NewImportBatch) -> Result<String, ImportError>;
}
pub struct ImportService<R: ImportRepository> {
    repository: R,
}
impl<R: ImportRepository> ImportService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    pub fn create_batch(&self, batch: &NewImportBatch) -> Result<String, ImportError> {
        if batch.account_id.trim().is_empty()
            || batch.source_file_name.trim().is_empty()
            || !matches!(
                batch.source_format.as_str(),
                "csv" | "xlsx" | "pdf" | "unknown"
            )
        {
            return Err(ImportError::InvalidBatch);
        }
        self.repository.create_batch(batch)
    }
}
