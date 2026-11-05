use crate::{
    application::accounts::{AccountRepository, AccountRepositoryError},
    domain::accounts::{Account, AccountType, NewAccount, UpdateAccount},
    infrastructure::database::Database,
};

use rusqlite::{OptionalExtension, Row, params};
use uuid::Uuid;

pub struct SqliteAccountRepository<'a> {
    database: &'a Database,
}

impl<'a> SqliteAccountRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    fn map_row(row: &Row<'_>) -> rusqlite::Result<Account> {
        let account_type_value: String = row.get(2)?;
        let account_type = AccountType::from_str(&account_type_value).ok_or_else(|| {
            rusqlite::Error::FromSqlConversionFailure(
                2,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unknown account type: {account_type_value}"),
                )),
            )
        })?;
        Ok(Account {
            id: row.get(0)?,
            name: row.get(1)?,
            account_type,
            institution_name: row.get(3)?,
            currency_code: row.get(4)?,
            notes: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
            archived_at: row.get(8)?,
        })
    }

    fn map_database_error(error: rusqlite::Error) -> AccountRepositoryError {
        match error {
            rusqlite::Error::FromSqlConversionFailure(_, _, source) => {
                AccountRepositoryError::InvalidStoredData(source.to_string())
            }
            other => AccountRepositoryError::Database(other.to_string()),
        }
    }
}

impl AccountRepository for SqliteAccountRepository<'_> {
    fn create(&self, account: &NewAccount) -> Result<Account, AccountRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| AccountRepositoryError::ConnectionPoisoned)?;

        let account_id = Uuid::new_v4().to_string();
        let transaction = connection.transaction().map_err(Self::map_database_error)?;

        transaction
            .execute(
                "
        INSERT INTO accounts ( id, name, account_type, institution_name, currency_code, notes)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ",
                params![
                    account_id,
                    account.name,
                    account.account_type.as_str(),
                    account.institution_name,
                    account.currency_code,
                    account.notes,
                ],
            )
            .map_err(Self::map_database_error)?;

        if let Some(opening_balance) = &account.opening_balance {
            let transaction_id = Uuid::new_v4().to_string();

            transaction
                .execute(
                    "
            INSERT INTO financial_transactions (id, transaction_type, amount_minor, currency_code, occurred_at, description, source_type)
            VALUES (?1, 'opening_balance', ?2, ?3, ?4, ?5, 'opening_balance')
            ",
                    params![
                        transaction_id,
                        opening_balance.amount_minor,
                        account.currency_code,
                        opening_balance.occurred_at,
                        "Opening balance",
                    ],
                )
                .map_err(Self::map_database_error)?;

            transaction
                .execute(
                    "
            INSERT INTO account_entries (id, transaction_id, account_id, amount_minor, entry_type)
            VALUES (?1, ?2, ?3, ?4, 'opening_balance')
            ",
                    params![
                        Uuid::new_v4().to_string(),
                        transaction_id,
                        account_id,
                        opening_balance.amount_minor,
                    ],
                )
                .map_err(Self::map_database_error)?;
        }

        transaction.commit().map_err(Self::map_database_error)?;

        let mut statement = connection.prepare(
        "
        SELECT
            id, name, account_type, institution_name, currency_code, notes, created_at, updated_at, archived_at
        FROM accounts
        WHERE id = ?1
        ",)
        .map_err(Self::map_database_error)?;

        statement
            .query_row([account_id], Self::map_row)
            .map_err(Self::map_database_error)
    }

    fn find_by_id(&self, id: &str) -> Result<Option<Account>, AccountRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| AccountRepositoryError::ConnectionPoisoned)?;

        let mut statement = connection
            .prepare("
            SELECT
                id, name, account_type, institution_name, currency_code, notes, created_at, updated_at, archived_at
            FROM accounts
            WHERE id = ?1
            ",)
            .map_err(Self::map_database_error)?;

        statement
            .query_row([id], Self::map_row)
            .optional()
            .map_err(Self::map_database_error)
    }

    fn list_active(&self) -> Result<Vec<Account>, AccountRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| AccountRepositoryError::ConnectionPoisoned)?;

        let mut statement = connection
            .prepare("
            SELECT
                id, name, account_type, institution_name, currency_code, notes, created_at, updated_at, archived_at
            FROM accounts
            WHERE archived_at IS NULL
            ORDER BY name COLLATE NOCASE
            ",)
            .map_err(Self::map_database_error)?;

        let rows = statement
            .query_map([], Self::map_row)
            .map_err(Self::map_database_error)?;

        rows.map(|row| row.map_err(Self::map_database_error))
            .collect()
    }
    fn update(
        &self,
        id: &str,
        account: &UpdateAccount,
    ) -> Result<Option<Account>, AccountRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| AccountRepositoryError::ConnectionPoisoned)?;

        let affected = connection
            .execute(
                "
        UPDATE accounts
        SET
            name = ?1,
            institution_name = ?2,
            notes = ?3,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?4
          AND archived_at IS NULL
        ",
                params![account.name, account.institution_name, account.notes, id,],
            )
            .map_err(Self::map_database_error)?;

        if affected == 0 {
            return Ok(None);
        }

        let mut statement = connection.prepare("
        SELECT
            id, name, account_type, institution_name, currency_code, notes, created_at, updated_at, archived_at
        FROM accounts
        WHERE id = ?1
        ",)
    .map_err(Self::map_database_error)?;

        statement
            .query_row([id], Self::map_row)
            .map(Some)
            .map_err(Self::map_database_error)
    }
    fn archive(&self, id: &str) -> Result<Option<Account>, AccountRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| AccountRepositoryError::ConnectionPoisoned)?;

        let affected = connection
            .execute(
                "
        UPDATE accounts
        SET
            archived_at = CURRENT_TIMESTAMP,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?1
          AND archived_at IS NULL
        ",
                [id],
            )
            .map_err(Self::map_database_error)?;

        if affected == 0 {
            return Ok(None);
        }

        let mut statement = connection.prepare(
        "
        SELECT
            id, name, account_type, institution_name, currency_code, notes, created_at, updated_at, archived_at
        FROM accounts
        WHERE id = ?1
        ",)
    .map_err(Self::map_database_error)?;

        statement
            .query_row([id], Self::map_row)
            .map(Some)
            .map_err(Self::map_database_error)
    }
    fn reactivate(&self, id: &str) -> Result<Option<Account>, AccountRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| AccountRepositoryError::ConnectionPoisoned)?;

        let affected = connection
            .execute(
                "
        UPDATE accounts
        SET
            archived_at = NULL,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?1
          AND archived_at IS NOT NULL
        ",
                [id],
            )
            .map_err(Self::map_database_error)?;

        if affected == 0 {
            return Ok(None);
        }

        let mut statement = connection
            .prepare("
        SELECT
            id, name, account_type, institution_name, currency_code, notes, created_at, updated_at, archived_at
        FROM accounts
        WHERE id = ?1
        ", )
            .map_err(Self::map_database_error)?;

        statement
            .query_row([id], Self::map_row)
            .map(Some)
            .map_err(Self::map_database_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::accounts::OpeningBalance, infrastructure::database::Database};
    use std::path::Path;

    fn setup() -> Database {
        Database::initialize(Path::new(":memory:")).expect("Failed to initialize test database")
    }
    fn new_account() -> NewAccount {
        NewAccount::new(
            "Safaricom M-Pesa".to_string(),
            AccountType::MobileMoney,
            Some("Safaricom".to_string()),
            None,
        )
        .expect("Failed to create test account")
    }

    #[test]
    fn creates_account_without_opening_balance() {
        let database = setup();
        let repository = SqliteAccountRepository::new(&database);
        let account = new_account();
        let created = repository
            .create(&account)
            .expect("Account creation failed");

        assert_eq!(created.name, "Safaricom M-Pesa");

        let connection = database.connection().lock().unwrap();
        let transaction_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |row| {
                row.get(0)
            })
            .unwrap();
        let entry_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM account_entries", [], |row| row.get(0))
            .unwrap();

        assert_eq!(transaction_count, 0);
        assert_eq!(entry_count, 0);
    }

    #[test]
    fn creates_account_with_opening_balance() {
        let database = setup();
        let repository = SqliteAccountRepository::new(&database);
        let account = new_account().with_opening_balance(
            OpeningBalance::new(250_000, "2026-11-05T10:00:00+03:00".to_string()).unwrap(),
        );
        let created = repository
            .create(&account)
            .expect("Account creation failed");
        let connection = database.connection().lock().unwrap();
        let transaction_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |row| {
                row.get(0)
            })
            .unwrap();
        let entry: (i64, String) = connection
            .query_row(
                "
                SELECT amount_minor, entry_type
                FROM account_entries
                WHERE account_id = ?1
                ",
                [&created.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();

        assert_eq!(transaction_count, 1);
        assert_eq!(entry.0, 250_000);
        assert_eq!(entry.1, "opening_balance");
    }

    #[test]
    fn rolls_back_account_when_opening_entry_fails() {
        let database = setup();
        {
            let connection = database.connection().lock().unwrap();

            connection
                .execute_batch(
                    "
                    CREATE TRIGGER reject_opening_entry
                    BEFORE INSERT ON account_entries
                    BEGIN
                        SELECT RAISE(ABORT, 'Simulated entry failure');
                    END;
                    ",
                )
                .unwrap();
        }

        let repository = SqliteAccountRepository::new(&database);
        let account = new_account().with_opening_balance(
            OpeningBalance::new(250_000, "2026-11-05T10:00:00+03:00".to_string()).unwrap(),
        );
        let result = repository.create(&account);

        assert!(result.is_err());

        let connection = database.connection().lock().unwrap();
        let account_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM accounts", [], |row| row.get(0))
            .unwrap();
        let transaction_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(account_count, 0);
        assert_eq!(transaction_count, 0);
    }
    #[test]
fn updates_archives_and_reactivates_account() {
    let database = setup();
    let repository = SqliteAccountRepository::new(&database);
    let account = new_account();
    let created = repository
        .create(&account)
        .expect("Account creation failed");
    let update = UpdateAccount::new(
        "Personal M-Pesa".to_string(),
        Some("Safaricom".to_string()),
        Some("Primary mobile money account".to_string()),
    )
    .unwrap();
    let updated = repository
        .update(&created.id, &update)
        .expect("Account update failed")
        .expect("Account should exist");

    assert_eq!(updated.name, "Personal M-Pesa");
    assert_eq!(
        updated.notes.as_deref(),
        Some("Primary mobile money account")
    );
    assert!(!updated.is_archived());

    let archived = repository
        .archive(&created.id)
        .expect("Archiving failed")
        .expect("Account should exist");

    assert!(archived.is_archived());

    let active_accounts = repository
        .list_active()
        .expect("Listing active accounts failed");

    assert!(active_accounts.is_empty());
    assert!(
        repository.update(&created.id, &update).unwrap().is_none()
    );

    let reactivated = repository
        .reactivate(&created.id)
        .expect("Reactivation failed")
        .expect("Account should exist");

    assert!(!reactivated.is_archived());

    let active_accounts = repository
        .list_active()
        .expect("Listing active accounts failed");

    assert_eq!(active_accounts.len(), 1);
    assert_eq!(active_accounts[0].id, created.id);
}
}
