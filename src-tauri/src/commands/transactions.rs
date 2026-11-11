use crate::{
    application::transactions::{
        Allocation, TransactionRepositoryError, TransactionService, TransactionServiceError,
        TransactionType,
    },
    domain::transactions::{NewTransaction, TransactionValidationError},
    infrastructure::database::Database,
    persistence::transactions::{
        CategoryRecord, SqliteTransactionRepository, TransactionAuditRecord, TransactionRecord,
    },
};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize, serde::Serialize)]
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
    #[serde(default)]
    pub refund_of_transaction_id: Option<String>,
}
#[tauri::command]
pub fn create_transaction(
    state: State<'_, Database>,
    request: CreateTransactionRequest,
) -> Result<String, String> {
    if request.transaction_type == "refund" {
        let item = NewTransaction {
            account_id: request.account_id,
            transaction_type: TransactionType::Income,
            amount_minor: request.amount_minor,
            currency_code: request.currency_code,
            occurred_at: request.occurred_at,
            description: request.description,
            notes: request.notes,
            allocations: vec![],
        };
        item.validate().map_err(|e| match e {
            TransactionValidationError::InvalidAmount => "Amount must be greater than zero.",
            _ => "Enter valid refund details.",
        })?;
        return SqliteTransactionRepository::new(state.inner())
            .create_refund(&item, request.refund_of_transaction_id.as_deref())
            .map_err(map_repository_error);
    }
    let transaction_type = TransactionType::parse(&request.transaction_type)
        .ok_or("Select income, expense, or refund.")?;
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
pub fn update_transaction(
    state: State<'_, Database>,
    id: String,
    request: CreateTransactionRequest,
) -> Result<(), String> {
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
        .update(&id, &item)
        .map_err(map_error)
}
#[tauri::command]
pub fn list_eligible_refunds(
    state: State<'_, Database>,
    account_id: String,
) -> Result<Vec<crate::persistence::transactions::EligibleRefund>, String> {
    SqliteTransactionRepository::new(state.inner())
        .eligible_refunds(&account_id)
        .map_err(|_| "Eligible refunds could not be loaded.".into())
}
#[tauri::command]
pub fn void_transaction(
    state: State<'_, Database>,
    id: String,
    reason: String,
) -> Result<(), String> {
    let repository = SqliteTransactionRepository::new(state.inner());
    TransactionService::new(repository)
        .void(&id, &reason)
        .map_err(map_error)
}
#[tauri::command]
pub fn reverse_transaction(state: State<'_, Database>, id: String) -> Result<String, String> {
    let repository = SqliteTransactionRepository::new(state.inner());
    TransactionService::new(repository)
        .reverse(&id)
        .map_err(map_error)
}
#[derive(Deserialize)]
pub struct TransferRequest {
    pub from_account_id: String,
    pub to_account_id: String,
    pub amount_minor: i64,
    #[serde(default)]
    pub fee_minor: i64,
    pub occurred_at: String,
    pub description: Option<String>,
}
#[tauri::command]
pub fn create_transfer(
    state: State<'_, Database>,
    request: TransferRequest,
) -> Result<String, String> {
    let repository = SqliteTransactionRepository::new(state.inner());
    TransactionService::new(repository)
        .transfer(
            &request.from_account_id,
            &request.to_account_id,
            request.amount_minor,
            request.fee_minor,
            &request.occurred_at,
            request.description.as_deref(),
        )
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

#[tauri::command]
pub fn search_transactions(
    state: State<'_, Database>,
    account_id: Option<String>,
    from: Option<String>,
    to: Option<String>,
    query: Option<String>,
    status: Option<String>,
    flow: Option<String>,
    offset: Option<usize>,
    limit: Option<usize>,
) -> Result<crate::persistence::transactions::TransactionPage, String> {
    for date in [&from, &to].into_iter().flatten() {
        if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            return Err("Enter dates in YYYY-MM-DD format.".into());
        }
    }
    let query = query
        .map(|s| s.trim().chars().take(100).collect::<String>())
        .filter(|s| !s.is_empty());
    let status = status.filter(|s| ["posted", "voided"].contains(&s.as_str()));
    let flow = flow.filter(|s| ["income", "expense", "transfers"].contains(&s.as_str()));
    SqliteTransactionRepository::new(state.inner())
        .list_page(
            account_id.as_deref(),
            from.as_deref(),
            to.as_deref(),
            query.as_deref(),
            status.as_deref(),
            flow.as_deref(),
            offset.unwrap_or(0),
            limit.unwrap_or(50).clamp(1, 200),
        )
        .map_err(|_| "Transactions could not be searched.".into())
}
#[tauri::command]
pub fn list_active_categories(
    state: State<'_, Database>,
    category_type: Option<String>,
) -> Result<Vec<CategoryRecord>, String> {
    if category_type
        .as_deref()
        .is_some_and(|kind| !matches!(kind, "income" | "expense"))
    {
        return Err("Choose income or expense categories.".into());
    }
    SqliteTransactionRepository::new(state.inner())
        .list_active_categories(category_type.as_deref())
        .map_err(|_| "Categories could not be loaded.".into())
}
#[tauri::command]
pub fn create_category(
    state: State<'_, Database>,
    name: String,
    category_type: String,
) -> Result<CategoryRecord, String> {
    if !matches!(category_type.as_str(), "income" | "expense") {
        return Err("Choose income or expense for the category.".into());
    }
    SqliteTransactionRepository::new(state.inner())
        .create_category(&name, &category_type)
        .map_err(|error| match error {
            TransactionRepositoryError::InvalidCategory => {
                "Enter a category name up to 100 characters.".into()
            }
            TransactionRepositoryError::Database(message)
                if message.contains("idx_categories_unique_active_name") =>
            {
                "An active category with that name and type already exists.".into()
            }
            _ => "The category could not be created.".into(),
        })
}
#[tauri::command]
pub fn archive_category(state: State<'_, Database>, id: String) -> Result<(), String> {
    SqliteTransactionRepository::new(state.inner())
        .archive_category(&id)
        .map_err(|error| match error {
            TransactionRepositoryError::InvalidCategory => {
                "This category cannot be archived or is already archived.".into()
            }
            _ => "The category could not be archived.".into(),
        })
}
#[tauri::command]
pub fn get_transaction_audit_history(
    state: State<'_, Database>,
    id: String,
) -> Result<Vec<TransactionAuditRecord>, String> {
    SqliteTransactionRepository::new(state.inner())
        .audit_history(&id)
        .map_err(|_| "Transaction history could not be loaded.".into())
}
#[tauri::command]
pub fn get_transaction_allocations(
    state: State<'_, Database>,
    id: String,
) -> Result<Vec<AllocationRequest>, String> {
    SqliteTransactionRepository::new(state.inner())
        .allocations(&id)
        .map(|rows| {
            rows.into_iter()
                .map(|a| AllocationRequest {
                    category_id: a.category_id,
                    amount_minor: a.amount_minor,
                })
                .collect()
        })
        .map_err(|_| "Transaction allocations could not be loaded.".into())
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
        TransactionServiceError::Repository(e) => map_repository_error(e),
    }
}
fn map_repository_error(e: TransactionRepositoryError) -> String {
    match e {
        TransactionRepositoryError::AccountUnavailable => "The account is unavailable or archived.",
        TransactionRepositoryError::CurrencyMismatch => {
            "Transaction currency must match the account currency."
        }
        TransactionRepositoryError::InvalidCategory => {
            "Choose an active category that matches the transaction type."
        }
        TransactionRepositoryError::NotEditable => "This transaction cannot be edited.",
        TransactionRepositoryError::NotReversible => {
            "This transaction cannot be reversed or has already been reversed."
        }
        TransactionRepositoryError::RefundLimitExceeded => {
            "Refund amount exceeds the remaining refundable amount."
        }
        _ => "The transaction could not be saved.",
    }
    .into()
}
