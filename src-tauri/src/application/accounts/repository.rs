use crate::domain::accounts::{Account, NewAccount, UpdateAccount};

#[derive(Debug)]
pub enum AccountRepositoryError {
    Database(String),
    ConnectionPoisoned,
    InvalidStoredData(String),
    BalanceChanged {
        expected_balance_minor: i64,
        actual_balance_minor: i64,
    },
}

pub trait AccountRepository {
    fn create(&self, account: &NewAccount) -> Result<Account, AccountRepositoryError>;
    fn find_by_id(&self, id: &str) -> Result<Option<Account>, AccountRepositoryError>;
    fn list_active(&self) -> Result<Vec<Account>, AccountRepositoryError>;
    fn update(
        &self,
        id: &str,
        account: &UpdateAccount,
    ) -> Result<Option<Account>, AccountRepositoryError>;
    fn archive(
        &self,
        id: &str,
        expected_balance_minor: i64,
    ) -> Result<Option<Account>, AccountRepositoryError>;
    fn reactivate(&self, id: &str) -> Result<Option<Account>, AccountRepositoryError>;
    fn list_archived(&self) -> Result<Vec<Account>, AccountRepositoryError>;
    fn get_balance_minor(&self, account_id: &str) -> Result<i64, AccountRepositoryError>;
}