pub use crate::domain::transactions::{Allocation, TransactionType};
use crate::domain::transactions::{NewTransaction, TransactionValidationError};
#[derive(Debug)]
pub enum TransactionRepositoryError {
    Database(String),
    ConnectionPoisoned,
    AccountUnavailable,
    CurrencyMismatch,
    InvalidCategory,
    NotEditable,
    NotReversible,
    RefundLimitExceeded,
}
pub trait TransactionRepository {
    fn create(&self, transaction: &NewTransaction) -> Result<String, TransactionRepositoryError>;
    fn update(
        &self,
        id: &str,
        transaction: &NewTransaction,
    ) -> Result<(), TransactionRepositoryError>;
    fn void(&self, id: &str, reason: &str) -> Result<(), TransactionRepositoryError>;
    fn reverse(&self, id: &str) -> Result<String, TransactionRepositoryError>;
    fn transfer(
        &self,
        from_account_id: &str,
        to_account_id: &str,
        amount_minor: i64,
        fee_minor: i64,
        occurred_at: &str,
        description: Option<&str>,
    ) -> Result<String, TransactionRepositoryError>;
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
    pub fn update(&self, id: &str, t: &NewTransaction) -> Result<(), TransactionServiceError> {
        t.validate().map_err(TransactionServiceError::Validation)?;
        self.repository
            .update(id, t)
            .map_err(TransactionServiceError::Repository)
    }
    pub fn void(&self, id: &str, reason: &str) -> Result<(), TransactionServiceError> {
        self.repository
            .void(id, reason)
            .map_err(TransactionServiceError::Repository)
    }
    pub fn reverse(&self, id: &str) -> Result<String, TransactionServiceError> {
        self.repository
            .reverse(id)
            .map_err(TransactionServiceError::Repository)
    }
    pub fn transfer(
        &self,
        from: &str,
        to: &str,
        amount: i64,
        fee: i64,
        date: &str,
        description: Option<&str>,
    ) -> Result<String, TransactionServiceError> {
        if from == to
            || amount <= 0
            || fee < 0
            || chrono::DateTime::parse_from_rfc3339(date).is_err()
        {
            return Err(TransactionServiceError::Validation(
                TransactionValidationError::InvalidAmount,
            ));
        }
        self.repository
            .transfer(from, to, amount, fee, date, description)
            .map_err(TransactionServiceError::Repository)
    }
}
