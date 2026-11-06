pub use crate::domain::transactions::{Allocation, TransactionType};
use crate::domain::transactions::{NewTransaction, TransactionValidationError};
#[derive(Debug)]
pub enum TransactionRepositoryError {
    Database(String),
    ConnectionPoisoned,
    AccountUnavailable,
    CurrencyMismatch,
    InvalidCategory,
}
pub trait TransactionRepository {
    fn create(&self, transaction: &NewTransaction) -> Result<String, TransactionRepositoryError>;
}
#[derive(Debug)]
pub enum TransactionServiceError {
    Validation(TransactionValidationError),
    Repository(TransactionRepositoryError),
}
pub struct TransactionService<R: TransactionRepository> {
    repository: R,
}
impl<R: TransactionRepository> TransactionService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    pub fn create(&self, t: &NewTransaction) -> Result<String, TransactionServiceError> {
        t.validate().map_err(TransactionServiceError::Validation)?;
        self.repository
            .create(t)
            .map_err(TransactionServiceError::Repository)
    }
}
