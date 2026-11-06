#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewImportBatch {
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub source_file_hash: Option<String>,
}
#[derive(Clone, Debug)]
pub struct CsvImportMapping {
    pub date_column: String,
    pub description_column: Option<String>,
    pub amount_column: String,
    pub positive_amount_type: String,
    pub date_format: String,
}
#[derive(Debug)]
pub enum ImportError {
    Database(String),
    ConnectionPoisoned,
    InvalidBatch,
}
pub trait ImportRepository {
    fn create_batch(&self, batch: &NewImportBatch) -> Result<String, ImportError>;
    fn correct_record(
        &self,
        record_id: &str,
        date: &str,
        amount_minor: i64,
        transaction_type: &str,
        description: Option<&str>,
    ) -> Result<(), ImportError>;
    fn review_record(&self, record_id: &str, keep: bool) -> Result<(), ImportError>;
    fn finalize(&self, batch_id: &str) -> Result<usize, ImportError>;
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
    pub fn correct_record(
        &self,
        id: &str,
        date: &str,
        amount: i64,
        kind: &str,
        description: Option<&str>,
    ) -> Result<(), ImportError> {
        if amount <= 0
            || !matches!(kind, "income" | "expense")
            || chrono::DateTime::parse_from_rfc3339(date).is_err()
        {
            return Err(ImportError::InvalidBatch);
        }
        self.repository
            .correct_record(id, date, amount, kind, description)
    }
    pub fn review_record(&self, id: &str, keep: bool) -> Result<(), ImportError> {
        self.repository.review_record(id, keep)
    }
    pub fn finalize(&self, id: &str) -> Result<usize, ImportError> {
        self.repository.finalize(id)
    }
}
