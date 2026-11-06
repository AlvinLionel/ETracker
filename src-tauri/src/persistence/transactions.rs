use crate::{
    application::transactions::{TransactionRepository, TransactionRepositoryError},
    domain::transactions::NewTransaction,
    infrastructure::database::Database,
};
use rusqlite::{TransactionBehavior, params};
use uuid::Uuid;

pub struct SqliteTransactionRepository<'a> {
    database: &'a Database,
}
impl<'a> SqliteTransactionRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }
}
#[derive(serde::Serialize)]
pub struct TransactionRecord {
    pub id: String,
    pub account_id: String,
    pub transaction_type: String,
    pub amount_minor: i64,
    pub currency_code: String,
    pub occurred_at: String,
    pub description: Option<String>,
    pub status: String,
}
impl SqliteTransactionRepository<'_> {
    pub fn list(
        &self,
        account_id: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<TransactionRecord>, TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let mut statement=connection.prepare("SELECT t.id,e.account_id,t.transaction_type,t.amount_minor,t.currency_code,t.occurred_at,t.description,t.status FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE (?1 IS NULL OR e.account_id=?1) AND (?2 IS NULL OR date(t.occurred_at)>=date(?2)) AND (?3 IS NULL OR date(t.occurred_at)<=date(?3)) ORDER BY datetime(t.occurred_at) DESC,t.created_at DESC").map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map(params![account_id, from, to], |r| {
                Ok(TransactionRecord {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    transaction_type: r.get(2)?,
                    amount_minor: r.get(3)?,
                    currency_code: r.get(4)?,
                    occurred_at: r.get(5)?,
                    description: r.get(6)?,
                    status: r.get(7)?,
                })
            })
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
    }
}
impl TransactionRepository for SqliteTransactionRepository<'_> {
    fn create(&self, item: &NewTransaction) -> Result<String, TransactionRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let account: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT currency_code, archived_at FROM accounts WHERE id=?1",
                [&item.account_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let Some((currency, archived)) = account else {
            return Err(TransactionRepositoryError::AccountUnavailable);
        };
        if archived.is_some() {
            return Err(TransactionRepositoryError::AccountUnavailable);
        }
        if currency != item.currency_code {
            return Err(TransactionRepositoryError::CurrencyMismatch);
        }
        for allocation in &item.allocations {
            let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND category_type=?2 AND archived_at IS NULL)",[&allocation.category_id,item.transaction_type.as_str()],|r|r.get(0)).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
            if !valid {
                return Err(TransactionRepositoryError::InvalidCategory);
            }
        }
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO financial_transactions (id,transaction_type,amount_minor,currency_code,occurred_at,description,notes,source_type) VALUES (?1,?2,?3,?4,?5,?6,?7,'manual')",params![id,item.transaction_type.as_str(),item.amount_minor,item.currency_code,item.occurred_at,item.description,item.notes]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO account_entries (id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,item.account_id,item.transaction_type.entry_amount(item.amount_minor)]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        for allocation in &item.allocations {
            tx.execute("INSERT INTO category_allocations (id,transaction_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,allocation.category_id,allocation.amount_minor]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        }
        tx.execute("INSERT INTO transaction_audit_log (id,transaction_id,action,new_values) VALUES (?1,?2,'created',?3)",params![Uuid::new_v4().to_string(),id,format!("{{\"type\":\"{}\",\"amount_minor\":{}}}",item.transaction_type.as_str(),item.amount_minor)]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(id)
    }
}

use rusqlite::OptionalExtension;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::transactions::{Allocation, TransactionType},
        infrastructure::database::Database,
    };
    use std::path::Path;
    fn db() -> Database {
        Database::initialize(Path::new(":memory:")).unwrap()
    }
    fn account(db: &Database, id: &str) {
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO accounts(id,name,account_type) VALUES (?1,'Main','cash')",
                [id],
            )
            .unwrap();
    }
    fn input(account: &str, kind: TransactionType) -> NewTransaction {
        NewTransaction {
            account_id: account.into(),
            transaction_type: kind,
            amount_minor: 1200,
            currency_code: "KES".into(),
            occurred_at: "2026-11-06T10:00:00+03:00".into(),
            description: Some("Test".into()),
            notes: None,
            allocations: vec![],
        }
    }
    #[test]
    fn persists_entry_and_audit_atomically() {
        let db = db();
        account(&db, "a");
        let r = SqliteTransactionRepository::new(&db);
        let id = r.create(&input("a", TransactionType::Expense)).unwrap();
        let c = db.connection().lock().unwrap();
        let amount: i64 = c
            .query_row(
                "SELECT amount_minor FROM account_entries WHERE transaction_id=?1",
                [&id],
                |x| x.get(0),
            )
            .unwrap();
        let audit: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM transaction_audit_log WHERE transaction_id=?1",
                [&id],
                |x| x.get(0),
            )
            .unwrap();
        assert_eq!(amount, -1200);
        assert_eq!(audit, 1);
    }
    #[test]
    fn rejects_archived_account_without_writes() {
        let db = db();
        account(&db, "a");
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE accounts SET archived_at=CURRENT_TIMESTAMP WHERE id='a'",
                [],
            )
            .unwrap();
        let r = SqliteTransactionRepository::new(&db);
        assert!(matches!(
            r.create(&input("a", TransactionType::Income)),
            Err(TransactionRepositoryError::AccountUnavailable)
        ));
        let n: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |x| {
                x.get(0)
            })
            .unwrap();
        assert_eq!(n, 0);
    }
    #[test]
    fn category_must_be_active_and_match_type() {
        let db = db();
        account(&db, "a");
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('c','Food','expense')",
                [],
            )
            .unwrap();
        let mut t = input("a", TransactionType::Income);
        t.allocations = vec![Allocation {
            category_id: "c".into(),
            amount_minor: 1200,
        }];
        let r = SqliteTransactionRepository::new(&db);
        assert!(matches!(
            r.create(&t),
            Err(TransactionRepositoryError::InvalidCategory)
        ));
    }

    #[test]
    fn transaction_failure_rolls_back_header_and_entry() {
        let db = db();
        account(&db, "a");
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('c','Food','expense')",
                [],
            )
            .unwrap();
        let mut t = input("a", TransactionType::Expense);
        t.allocations = vec![
            Allocation {
                category_id: "c".into(),
                amount_minor: 600,
            },
            Allocation {
                category_id: "c".into(),
                amount_minor: 600,
            },
        ];
        let r = SqliteTransactionRepository::new(&db);
        assert!(matches!(
            r.create(&t),
            Err(TransactionRepositoryError::Database(_))
        ));
        let c = db.connection().lock().unwrap();
        let headers: i64 = c
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |x| {
                x.get(0)
            })
            .unwrap();
        let entries: i64 = c
            .query_row("SELECT COUNT(*) FROM account_entries", [], |x| x.get(0))
            .unwrap();
        assert_eq!((headers, entries), (0, 0));
    }

    #[test]
    fn history_filters_by_account_and_effective_date() {
        let db = db();
        account(&db, "a");
        account(&db, "b");
        let r = SqliteTransactionRepository::new(&db);
        r.create(&input("a", TransactionType::Income)).unwrap();
        let mut earlier = input("b", TransactionType::Expense);
        earlier.occurred_at = "2026-11-05T10:00:00+03:00".into();
        r.create(&earlier).unwrap();
        let matches = r.list(Some("a"), Some("2026-11-06"), None).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].account_id, "a");
        assert_eq!(matches[0].transaction_type, "income");
    }
}
