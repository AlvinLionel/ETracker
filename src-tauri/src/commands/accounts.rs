use crate::{
    application::accounts::{AccountService, AccountServiceError},
    domain::accounts::{
        Account, AccountType, AccountValidationError, NewAccount, OpeningBalance, UpdateAccount,
    },
    infrastructure::database::Database,
    persistence::accounts::SqliteAccountRepository,
};

use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize)]
pub struct AccountResponse {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub institution_name: Option<String>,
    pub currency_code: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
}

impl From<Account> for AccountResponse {
    fn from(account: Account) -> Self {
        Self {
            id: account.id,
            name: account.name,
            account_type: account.account_type.as_str().to_string(),
            institution_name: account.institution_name,
            currency_code: account.currency_code,
            notes: account.notes,
            created_at: account.created_at,
            updated_at: account.updated_at,
            archived_at: account.archived_at,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateAccountRequest {
    pub name: String,
    pub account_type: String,
    pub institution_name: Option<String>,
    pub notes: Option<String>,
    pub opening_balance: Option<OpeningBalanceRequest>,
}

#[derive(Debug, Deserialize)]
pub struct OpeningBalanceRequest {
    pub amount_minor: i64,
    pub occurred_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAccountRequest {
    pub name: String,
    pub institution_name: Option<String>,
    pub notes: Option<String>,
}

fn map_validation_error(error: AccountValidationError) -> String {
    match error {
        AccountValidationError::EmptyName => "Account name is required.".to_string(),

        AccountValidationError::UnsupportedCurrency => {
            "The selected currency is not supported.".to_string()
        }

        AccountValidationError::InvalidOpeningBalanceAmount => {
            "Opening balance must be greater than zero.".to_string()
        }

        AccountValidationError::MissingOpeningBalanceDate => {
            "Opening balance date is required.".to_string()
        }
        AccountValidationError::InvalidOpeningBalanceDate => {
            "Enter a valid date and time with a timezone.".to_string()
        }
    }
}

fn map_service_error(error: AccountServiceError) -> String {
    match error {
        AccountServiceError::Validation(error) => map_validation_error(error),

        AccountServiceError::Repository(_) => {
            "The account operation could not be completed.".to_string()
        }
    }
}

#[tauri::command]
pub fn create_account(
    state: State<'_, Database>,
    request: CreateAccountRequest,
) -> Result<AccountResponse, String> {
    let account_type = AccountType::from_str(&request.account_type)
        .ok_or_else(|| "Select a valid account type.".to_string())?;

    let mut account = NewAccount::new(
        request.name,
        account_type,
        request.institution_name,
        request.notes,
    )
    .map_err(map_validation_error)?;

    if let Some(opening_balance) = request.opening_balance {
        let opening_balance =
            OpeningBalance::new(opening_balance.amount_minor, opening_balance.occurred_at)
                .map_err(map_validation_error)?;

        account = account.with_opening_balance(opening_balance);
    }

    let repository = SqliteAccountRepository::new(state.inner());
    let service = AccountService::new(repository);

    service
        .create_account(&account)
        .map(AccountResponse::from)
        .map_err(map_service_error)
}

#[tauri::command]
pub fn get_account(
    state: State<'_, Database>,
    id: String,
) -> Result<Option<AccountResponse>, String> {
    let repository = SqliteAccountRepository::new(state.inner());
    let service = AccountService::new(repository);

    service
        .get_account(&id)
        .map(|account| account.map(AccountResponse::from))
        .map_err(map_service_error)
}

#[tauri::command]
pub fn list_active_accounts(state: State<'_, Database>) -> Result<Vec<AccountResponse>, String> {
    let repository = SqliteAccountRepository::new(state.inner());
    let service = AccountService::new(repository);

    service
        .list_active_accounts()
        .map(|accounts| accounts.into_iter().map(AccountResponse::from).collect())
        .map_err(map_service_error)
}

#[tauri::command]
pub fn update_account(
    state: State<'_, Database>,
    id: String,
    request: UpdateAccountRequest,
) -> Result<Option<AccountResponse>, String> {
    let account = UpdateAccount::new(request.name, request.institution_name, request.notes)
        .map_err(map_validation_error)?;

    let repository = SqliteAccountRepository::new(state.inner());
    let service = AccountService::new(repository);

    service
        .update_account(&id, &account)
        .map(|account| account.map(AccountResponse::from))
        .map_err(map_service_error)
}

#[tauri::command]
pub fn archive_account(
    state: State<'_, Database>,
    id: String,
) -> Result<Option<AccountResponse>, String> {
    let repository = SqliteAccountRepository::new(state.inner());
    let service = AccountService::new(repository);

    service
        .archive_account(&id)
        .map(|account| account.map(AccountResponse::from))
        .map_err(map_service_error)
}

#[tauri::command]
pub fn reactivate_account(
    state: State<'_, Database>,
    id: String,
) -> Result<Option<AccountResponse>, String> {
    let repository = SqliteAccountRepository::new(state.inner());
    let service = AccountService::new(repository);

    service
        .reactivate_account(&id)
        .map(|account| account.map(AccountResponse::from))
        .map_err(map_service_error)
}
