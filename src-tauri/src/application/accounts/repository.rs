use crate::domain::accounts::{Account, NewAccount, UpdateAccount};

#[derive(Debug)]
pub enum AccountRepositoryError {
    Database(String),
    ConnectionPoisoned,
    InvalidStoredData(String),
}
pub trait AccountRepository {
    fn create(
        &self,
        account: &NewAccount,
    ) -> Result<Account, AccountRepositoryError>;

    fn find_by_id(
        &self,
        id: &str,
    ) -> Result<Option<Account>, AccountRepositoryError>;

    fn list_active(
        &self,
    ) -> Result<Vec<Account>, AccountRepositoryError>;

    fn update(
        &self,
        id: &str,
        account: &UpdateAccount,
    ) -> Result<Option<Account>, AccountRepositoryError>;

    fn archive(
        &self,
        id: &str,
    ) -> Result<Option<Account>, AccountRepositoryError>;

    fn reactivate(
        &self,
        id: &str,
    ) -> Result<Option<Account>, AccountRepositoryError>;
}