use crate::domain::accounts::{Account, AccountValidationError, NewAccount, UpdateAccount};

use super::{AccountRepository, AccountRepositoryError};

#[derive(Debug)]
pub enum AccountServiceError {
    Validation(AccountValidationError),
    Repository(AccountRepositoryError),
}

pub struct AccountService<R: AccountRepository> {
    repository: R,
}

impl<R: AccountRepository> AccountService<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }
    pub fn create_account(&self, account: &NewAccount) -> Result<Account, AccountServiceError> {
        NewAccount::validate_currency(&account.currency_code)
            .map_err(AccountServiceError::Validation)?;

        self.repository
            .create(account)
            .map_err(AccountServiceError::Repository)
    }
    pub fn get_account(&self, id: &str) -> Result<Option<Account>, AccountServiceError> {
        self.repository
            .find_by_id(id)
            .map_err(AccountServiceError::Repository)
    }
    pub fn list_active_accounts(&self) -> Result<Vec<Account>, AccountServiceError> {
        self.repository
            .list_active()
            .map_err(AccountServiceError::Repository)
    }
    pub fn update_account(
        &self,
        id: &str,
        account: &UpdateAccount,
    ) -> Result<Option<Account>, AccountServiceError> {
        if account.name.trim().is_empty() {
            return Err(AccountServiceError::Validation(
                AccountValidationError::EmptyName,
            ));
        }

        self.repository
            .update(id, account)
            .map_err(AccountServiceError::Repository)
    }
    pub fn archive_account(
        &self,
        id: &str,
        expected_balance_minor: i64,
    ) -> Result<Option<Account>, AccountServiceError> {
        self.repository
            .archive(id, expected_balance_minor)
            .map_err(AccountServiceError::Repository)
    }
    pub fn reactivate_account(&self, id: &str) -> Result<Option<Account>, AccountServiceError> {
        self.repository
            .reactivate(id)
            .map_err(AccountServiceError::Repository)
    }
    pub fn list_archived_accounts(&self) -> Result<Vec<Account>, AccountServiceError> {
        self.repository
            .list_archived()
            .map_err(AccountServiceError::Repository)
    }
    pub fn get_account_balance(&self, account_id: &str) -> Result<i64, AccountServiceError> {
        self.repository
            .get_balance_minor(account_id)
            .map_err(AccountServiceError::Repository)
    }
}
