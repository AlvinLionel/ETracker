use crate::{
    application::transactions::{
        Allocation, TransactionRepositoryError, TransactionService, TransactionServiceError,
        TransactionType,
    },
    domain::transactions::{NewTransaction, TransactionValidationError},
    infrastructure::database::Database,
    persistence::transactions::{SqliteTransactionRepository, TransactionRecord},
};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct AllocationRequest {
    pub category_id: String,
    pub amount_minor: i64,
}
#[derive(Deserialize)]
pub struct CreateTransactionRequest {
    pub account_id: String,
    pub transaction_type: String,
    pub amount_minor: i64,
    pub currency_code: String,
    pub occurred_at: String,
    pub description: Option<String>,
    pub notes: Option<String>,
    #[serde(default)]
    pub allocations: Vec<AllocationRequest>,
}
#[tauri::command]
pub fn create_transaction(
    state: State<'_, Database>,
    request: CreateTransactionRequest,
) -> Result<String, String> {
    let transaction_type =
        TransactionType::parse(&request.transaction_type).ok_or("Select income or expense.")?;
    let item = NewTransaction {
        account_id: request.account_id,
        transaction_type,
        amount_minor: request.amount_minor,
        currency_code: request.currency_code,
        occurred_at: request.occurred_at,
        description: request.description,
        notes: request.notes,
        allocations: request
            .allocations
            .into_iter()
            .map(|a| Allocation {
                category_id: a.category_id,
                amount_minor: a.amount_minor,
            })
            .collect(),
    };
    let repository = SqliteTransactionRepository::new(state.inner());
    TransactionService::new(repository)
        .create(&item)
        .map_err(map_error)
}
#[tauri::command]
pub fn list_transactions(
    state: State<'_, Database>,
    account_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
) -> Result<Vec<TransactionRecord>, String> {
    if from
        .as_deref()
        .is_some_and(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err())
        || to
            .as_deref()
            .is_some_and(|v| chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_err())
    {
        return Err("Enter dates in YYYY-MM-DD format.".into());
    }
    SqliteTransactionRepository::new(state.inner())
        .list(account_id.as_deref(), from.as_deref(), to.as_deref())
        .map_err(|_| "Transactions could not be loaded.".into())
}
fn map_error(error: TransactionServiceError) -> String {
    match error {
        TransactionServiceError::Validation(e) => match e {
            TransactionValidationError::EmptyAccount => "Select an account.",
            TransactionValidationError::InvalidAmount => "Amount must be greater than zero.",
            TransactionValidationError::InvalidCurrency => "Currency code is invalid.",
            TransactionValidationError::InvalidDate => "Enter a valid date and time with timezone.",
            TransactionValidationError::InvalidAllocation => {
                "Category allocations must be positive and add up to the transaction amount."
            }
        }
        .into(),
        TransactionServiceError::Repository(e) => match e {
            TransactionRepositoryError::AccountUnavailable => {
                "The account is unavailable or archived."
            }
            TransactionRepositoryError::CurrencyMismatch => {
                "Transaction currency must match the account currency."
            }
            TransactionRepositoryError::InvalidCategory => {
                "Choose an active category that matches the transaction type."
            }
            _ => "The transaction could not be saved.",
        }
        .into(),
    }
}
