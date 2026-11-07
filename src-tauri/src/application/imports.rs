pub const MAX_SOURCE_FILE_BYTES: usize = 25 * 1024 * 1024;

fn source_file_is_too_large(size: usize) -> bool {
    size > MAX_SOURCE_FILE_BYTES
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewImportBatch {
    pub account_id: String,
    pub source_file_name: String,
    pub source_format: String,
    pub revision_of_batch_id: Option<String>,
    pub source_file_contents: Vec<u8>,
}
#[derive(Clone, Debug)]
pub struct CsvImportMapping {
    pub date_column: String,
    pub description_column: Option<String>,
    pub amount_column: String,
    pub positive_amount_type: String,
    pub date_format: String,
}
#[derive(Clone, Debug, serde::Serialize)]
pub struct ImportReconciliation {
    pub opening_balance_minor: Option<i64>,
    pub closing_balance_minor: Option<i64>,
    pub expected_closing_balance_minor: Option<i64>,
    pub difference_minor: Option<i64>,
    pub outcome: String,
    pub explanation: String,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StagedCategoryAllocation {
    pub category_id: String,
    pub amount_minor: i64,
}
#[derive(Debug)]
pub enum ImportError {
    Database(String),
    ConnectionPoisoned,
    InvalidBatch,
    SourceFileTooLarge,
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
    fn record_failure(
        &self,
        batch_id: &str,
        record_id: Option<&str>,
        stage: &str,
        message: &str,
        corrective_action: &str,
    ) -> Result<(), ImportError>;
    fn reconcile(
        &self,
        batch_id: &str,
        opening: Option<i64>,
        closing: Option<i64>,
    ) -> Result<ImportReconciliation, ImportError>;
    fn set_record_allocations(
        &self,
        record_id: &str,
        allocations: &[StagedCategoryAllocation],
    ) -> Result<(), ImportError>;
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
            || batch.source_file_contents.is_empty()
            || !matches!(
                batch.source_format.as_str(),
                "csv" | "xlsx" | "pdf" | "unknown"
            )
        {
            return Err(ImportError::InvalidBatch);
        }
        if source_file_is_too_large(batch.source_file_contents.len()) {
            return Err(ImportError::SourceFileTooLarge);
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
    pub fn record_failure(
        &self,
        batch_id: &str,
        record_id: Option<&str>,
        stage: &str,
        message: &str,
        corrective_action: &str,
    ) -> Result<(), ImportError> {
        self.repository
            .record_failure(batch_id, record_id, stage, message, corrective_action)
    }
    pub fn reconcile(
        &self,
        batch_id: &str,
        opening: Option<i64>,
        closing: Option<i64>,
    ) -> Result<ImportReconciliation, ImportError> {
        if opening.is_some() != closing.is_some() {
            return Err(ImportError::InvalidBatch);
        }
        self.repository.reconcile(batch_id, opening, closing)
    }
    pub fn set_record_allocations(
        &self,
        record_id: &str,
        allocations: &[StagedCategoryAllocation],
    ) -> Result<(), ImportError> {
        let mut categories = std::collections::HashSet::new();
        if allocations.iter().any(|a| {
            a.category_id.trim().is_empty()
                || a.amount_minor <= 0
                || !categories.insert(a.category_id.as_str())
        }) {
            return Err(ImportError::InvalidBatch);
        }
        self.repository
            .set_record_allocations(record_id, allocations)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_SOURCE_FILE_BYTES, source_file_is_too_large};

    #[test]
    fn source_file_size_limit_accepts_the_limit_and_rejects_larger_files() {
        assert!(!source_file_is_too_large(MAX_SOURCE_FILE_BYTES));
        assert!(source_file_is_too_large(MAX_SOURCE_FILE_BYTES + 1));
    }
}
