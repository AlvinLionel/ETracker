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
    pub entry_amount_minor: i64,
    pub currency_code: String,
    pub occurred_at: String,
    pub description: Option<String>,
    pub status: String,
}
#[derive(serde::Serialize)]
pub struct CategoryRecord {
    pub id: String,
    pub name: String,
    pub category_type: String,
}
#[derive(serde::Serialize)]
pub struct TransactionAuditRecord {
    pub action: String,
    pub previous_values: Option<String>,
    pub new_values: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}
impl SqliteTransactionRepository<'_> {
    pub fn allocations(
        &self,
        transaction_id: &str,
    ) -> Result<Vec<crate::domain::transactions::Allocation>, TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let mut statement=connection.prepare("SELECT category_id,amount_minor FROM category_allocations WHERE transaction_id=?1 ORDER BY created_at,id").map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map([transaction_id], |r| {
                Ok(crate::domain::transactions::Allocation {
                    category_id: r.get(0)?,
                    amount_minor: r.get(1)?,
                })
            })
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
    }
    pub fn audit_history(
        &self,
        id: &str,
    ) -> Result<Vec<TransactionAuditRecord>, TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let mut statement=connection.prepare("SELECT action,previous_values,new_values,reason,created_at FROM transaction_audit_log WHERE transaction_id=?1 ORDER BY created_at DESC,id DESC").map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map([id], |r| {
                Ok(TransactionAuditRecord {
                    action: r.get(0)?,
                    previous_values: r.get(1)?,
                    new_values: r.get(2)?,
                    reason: r.get(3)?,
                    created_at: r.get(4)?,
                })
            })
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
    }
    pub fn list_active_categories(
        &self,
        category_type: Option<&str>,
    ) -> Result<Vec<CategoryRecord>, TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT id,name,category_type FROM categories WHERE archived_at IS NULL AND (?1 IS NULL OR category_type=?1) ORDER BY is_system DESC, lower(name)"
        ).map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map([category_type], |row| {
                Ok(CategoryRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category_type: row.get(2)?,
                })
            })
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
    }

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
        let mut statement=connection.prepare("SELECT t.id,e.account_id,t.transaction_type,t.amount_minor,e.amount_minor,t.currency_code,t.occurred_at,t.description,t.status FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE (?1 IS NULL OR e.account_id=?1) AND (?2 IS NULL OR date(t.occurred_at)>=date(?2)) AND (?3 IS NULL OR date(t.occurred_at)<=date(?3)) ORDER BY datetime(t.occurred_at) DESC,t.created_at DESC").map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map(params![account_id, from, to], |r| {
                Ok(TransactionRecord {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    transaction_type: r.get(2)?,
                    amount_minor: r.get(3)?,
                    entry_amount_minor: r.get(4)?,
                    currency_code: r.get(5)?,
                    occurred_at: r.get(6)?,
                    description: r.get(7)?,
                    status: r.get(8)?,
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

    fn update(&self, id: &str, item: &NewTransaction) -> Result<(), TransactionRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let existing: Option<(String, String, i64, String, Option<String>, Option<String>, String, String)> = tx.query_row(
            "SELECT t.transaction_type,t.source_type,t.amount_minor,t.currency_code,t.description,t.notes,t.occurred_at,e.account_id FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.id=?1 AND t.status='posted'",
            [id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))
        ).optional().map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let Some((
            kind,
            source,
            old_amount,
            old_currency,
            old_description,
            old_notes,
            old_date,
            old_account,
        )) = existing
        else {
            return Err(TransactionRepositoryError::NotEditable);
        };
        if source != "manual" || kind != item.transaction_type.as_str() {
            return Err(TransactionRepositoryError::NotEditable);
        }
        let account: Option<(String, Option<String>)> = tx
            .query_row(
                "SELECT currency_code,archived_at FROM accounts WHERE id=?1",
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
        let old_allocations: Vec<(String, i64)> = {
            let mut stmt=tx.prepare("SELECT category_id,amount_minor FROM category_allocations WHERE transaction_id=?1").map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
            stmt.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))
                .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
        };
        for allocation in &item.allocations {
            let valid: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM categories c WHERE c.id=?1 AND c.category_type=?2 AND (c.archived_at IS NULL OR EXISTS(SELECT 1 FROM category_allocations ca WHERE ca.transaction_id=?3 AND ca.category_id=c.id AND ca.amount_minor=?4)))",
                params![allocation.category_id, item.transaction_type.as_str(), id, allocation.amount_minor],
                |r| r.get(0),
            ).map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
            if !valid {
                return Err(TransactionRepositoryError::InvalidCategory);
            }
        }
        let previous = serde_json::json!({"account_id":old_account,"amount_minor":old_amount,"currency_code":old_currency,"description":old_description,"notes":old_notes,"occurred_at":old_date,"allocations":old_allocations});
        let new_values = serde_json::json!({"account_id":item.account_id,"amount_minor":item.amount_minor,"currency_code":item.currency_code,"description":item.description,"notes":item.notes,"occurred_at":item.occurred_at,"allocations":item.allocations.iter().map(|a|serde_json::json!({"category_id":a.category_id,"amount_minor":a.amount_minor})).collect::<Vec<_>>()});
        tx.execute("UPDATE financial_transactions SET amount_minor=?1,currency_code=?2,occurred_at=?3,description=?4,notes=?5,updated_at=CURRENT_TIMESTAMP WHERE id=?6",params![item.amount_minor,item.currency_code,item.occurred_at,item.description,item.notes,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute(
            "UPDATE account_entries SET account_id=?1,amount_minor=?2 WHERE transaction_id=?3",
            params![
                item.account_id,
                item.transaction_type.entry_amount(item.amount_minor),
                id
            ],
        )
        .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute(
            "DELETE FROM category_allocations WHERE transaction_id=?1",
            [id],
        )
        .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        for allocation in &item.allocations {
            tx.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,allocation.category_id,allocation.amount_minor]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        }
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,previous_values,new_values) VALUES (?1,?2,'updated',?3,?4)",params![Uuid::new_v4().to_string(),id,previous.to_string(),new_values.to_string()]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    fn void(&self, id: &str, reason: &str) -> Result<(), TransactionRepositoryError> {
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(TransactionRepositoryError::NotEditable);
        }
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let source: Option<String> = tx
            .query_row(
                "SELECT source_type FROM financial_transactions WHERE id=?1 AND status='posted'",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        if source.as_deref() != Some("manual") {
            return Err(TransactionRepositoryError::NotEditable);
        }
        let changed=tx.execute("UPDATE financial_transactions SET status='voided',voided_at=CURRENT_TIMESTAMP,void_reason=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND status='posted'",params![reason,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(TransactionRepositoryError::NotEditable);
        }
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,reason) VALUES (?1,?2,'voided',?3)",params![Uuid::new_v4().to_string(),id,reason]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(())
    }

    fn reverse(&self, id: &str) -> Result<String, TransactionRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let original:Option<(String,i64,String,String)>=tx.query_row("SELECT transaction_type,amount_minor,currency_code,source_type FROM financial_transactions WHERE id=?1 AND status='posted'",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        let Some((kind, amount, currency, source)) = original else {
            return Err(TransactionRepositoryError::NotReversible);
        };
        if !matches!(kind.as_str(), "income" | "expense" | "transfer") || source != "manual" {
            return Err(TransactionRepositoryError::NotReversible);
        }
        let reversal_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type,reversal_of_transaction_id) SELECT ?1,'reversal',amount_minor,currency_code,datetime('now'),description,'reversal',id FROM financial_transactions WHERE id=?2",params![reversal_id,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) SELECT ?1,?2,account_id,-amount_minor FROM account_entries WHERE transaction_id=?3",params![Uuid::new_v4().to_string(),reversal_id,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'reversed',?3)",params![Uuid::new_v4().to_string(),id,serde_json::json!({"reversal_id":reversal_id}).to_string()]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let _ = (amount, currency);
        Ok(reversal_id)
    }

    fn transfer(
        &self,
        from: &str,
        to: &str,
        amount: i64,
        fee: i64,
        date: &str,
        description: Option<&str>,
    ) -> Result<String, TransactionRepositoryError> {
        let mut connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let get_currency = |account: &str| {
            tx.query_row(
                "SELECT currency_code FROM accounts WHERE id=?1 AND archived_at IS NULL",
                [account],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
        };
        let from_currency =
            get_currency(from)?.ok_or(TransactionRepositoryError::AccountUnavailable)?;
        let to_currency =
            get_currency(to)?.ok_or(TransactionRepositoryError::AccountUnavailable)?;
        if from_currency != to_currency {
            return Err(TransactionRepositoryError::CurrencyMismatch);
        }
        let transfer_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type) VALUES (?1,'transfer',?2,?3,?4,?5,'manual')",params![transfer_id,amount,from_currency,date,description]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),transfer_id,from,-amount]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),transfer_id,to,amount]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        let mut fee_id = None;
        if fee > 0 {
            let id = Uuid::new_v4().to_string();
            tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type) VALUES (?1,'expense',?2,?3,?4,'Transfer fee','manual')",params![id,fee,from_currency,date]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
            tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,from,-fee]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
            fee_id = Some(id);
        }
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'created',?3)",params![Uuid::new_v4().to_string(),transfer_id,serde_json::json!({"from_account_id":from,"to_account_id":to,"amount_minor":amount,"fee_transaction_id":fee_id}).to_string()]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.commit()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(transfer_id)
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

    #[test]
    fn active_categories_filter_by_type_and_exclude_archived() {
        let db = db();
        {
            let c = db.connection().lock().unwrap();
            c.execute("INSERT INTO categories(id,name,category_type,is_system) VALUES ('food','Food','expense',1)", []).unwrap();
            c.execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('salary','Salary','income')",
                [],
            )
            .unwrap();
            c.execute("INSERT INTO categories(id,name,category_type,archived_at) VALUES ('old','Old','expense',CURRENT_TIMESTAMP)", []).unwrap();
        }
        let repository = SqliteTransactionRepository::new(&db);
        let expenses = repository.list_active_categories(Some("expense")).unwrap();
        assert_eq!(expenses.len(), 1);
        assert_eq!(expenses[0].id, "food");
        let all = repository.list_active_categories(None).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn update_changes_ledger_and_records_before_after_audit() {
        let db = db();
        account(&db, "a");
        account(&db, "b");
        let repository = SqliteTransactionRepository::new(&db);
        let id = repository
            .create(&input("a", TransactionType::Expense))
            .unwrap();
        let mut edited = input("b", TransactionType::Expense);
        edited.amount_minor = 2500;
        edited.description = Some("Corrected".into());
        repository.update(&id, &edited).unwrap();
        let c = db.connection().lock().unwrap();
        let (account_id, amount): (String, i64) = c
            .query_row(
                "SELECT account_id,amount_minor FROM account_entries WHERE transaction_id=?1",
                [&id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let (action, previous, next): (String,String,String) = c.query_row(
            "SELECT action,previous_values,new_values FROM transaction_audit_log WHERE transaction_id=?1 AND action='updated'", [&id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))
        ).unwrap();
        assert_eq!((account_id, amount), ("b".into(), -2500));
        assert_eq!(action, "updated");
        assert!(previous.contains("1200"));
        assert!(next.contains("2500"));
    }

    #[test]
    fn update_rejects_non_manual_records_without_changing_them() {
        let db = db();
        account(&db, "a");
        let repository = SqliteTransactionRepository::new(&db);
        let id = repository
            .create(&input("a", TransactionType::Expense))
            .unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE financial_transactions SET source_type='import' WHERE id=?1",
                [&id],
            )
            .unwrap();
        assert!(matches!(
            repository.update(&id, &input("a", TransactionType::Expense)),
            Err(TransactionRepositoryError::NotEditable)
        ));
        let amount: i64 = db
            .connection()
            .lock()
            .unwrap()
            .query_row(
                "SELECT amount_minor FROM financial_transactions WHERE id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(amount, 1200);
    }

    #[test]
    fn edit_preserves_unchanged_archived_category_but_cannot_reassign_it() {
        let db = db();
        account(&db, "a");
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense')",
                [],
            )
            .unwrap();
        let repository = SqliteTransactionRepository::new(&db);
        let mut original = input("a", TransactionType::Expense);
        original.allocations = vec![Allocation {
            category_id: "food".into(),
            amount_minor: 1200,
        }];
        let id = repository.create(&original).unwrap();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE categories SET archived_at=CURRENT_TIMESTAMP WHERE id='food'",
                [],
            )
            .unwrap();
        let mut metadata_edit = original.clone();
        metadata_edit.description = Some("Updated memo".into());
        repository.update(&id, &metadata_edit).unwrap();
        let mut changed_allocation = original;
        changed_allocation.allocations[0].amount_minor = 1000;
        changed_allocation.allocations.push(Allocation {
            category_id: "new".into(),
            amount_minor: 200,
        });
        assert!(matches!(
            repository.update(&id, &changed_allocation),
            Err(TransactionRepositoryError::InvalidCategory)
        ));
        let amount: i64 = db.connection().lock().unwrap().query_row("SELECT amount_minor FROM category_allocations WHERE transaction_id=?1 AND category_id='food'", [&id], |r|r.get(0)).unwrap();
        assert_eq!(amount, 1200);
    }

    #[test]
    fn voiding_marks_record_and_excludes_it_from_account_balance() {
        let db = db();
        account(&db, "a");
        let repository = SqliteTransactionRepository::new(&db);
        let id = repository
            .create(&input("a", TransactionType::Expense))
            .unwrap();
        repository.void(&id, "Entered twice").unwrap();
        let c = db.connection().lock().unwrap();
        let status: String = c
            .query_row(
                "SELECT status FROM financial_transactions WHERE id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        let audit:i64=c.query_row("SELECT COUNT(*) FROM transaction_audit_log WHERE transaction_id=?1 AND action='voided'",[&id],|r|r.get(0)).unwrap();
        let balance:i64=c.query_row("SELECT COALESCE(SUM(e.amount_minor),0) FROM account_entries e JOIN financial_transactions t ON t.id=e.transaction_id WHERE e.account_id='a' AND t.status='posted'",[],|r|r.get(0)).unwrap();
        assert_eq!(status, "voided");
        assert_eq!(audit, 1);
        assert_eq!(balance, 0);
    }

    #[test]
    fn reversal_offsets_original_and_duplicate_reversal_is_rejected() {
        let db = db();
        account(&db, "a");
        let repository = SqliteTransactionRepository::new(&db);
        let id = repository
            .create(&input("a", TransactionType::Expense))
            .unwrap();
        let reversal = repository.reverse(&id).unwrap();
        let c = db.connection().lock().unwrap();
        let movements:i64=c.query_row("SELECT SUM(e.amount_minor) FROM account_entries e WHERE e.transaction_id IN (?1,?2)",params![id,reversal],|r|r.get(0)).unwrap();
        assert_eq!(movements, 0);
        drop(c);
        assert!(matches!(
            repository.reverse(&id),
            Err(TransactionRepositoryError::Database(_))
        ));
    }

    #[test]
    fn transfer_offsets_accounts_and_records_fee_as_expense() {
        let db = db();
        account(&db, "a");
        account(&db, "b");
        let repository = SqliteTransactionRepository::new(&db);
        repository
            .transfer(
                "a",
                "b",
                5000,
                150,
                "2026-11-06T10:00:00Z",
                Some("Rent transfer"),
            )
            .unwrap();
        let c = db.connection().lock().unwrap();
        let movements:i64=c.query_row("SELECT SUM(amount_minor) FROM account_entries WHERE transaction_id IN (SELECT id FROM financial_transactions WHERE transaction_type='transfer')",[],|r|r.get(0)).unwrap();
        let fee:i64=c.query_row("SELECT amount_minor FROM account_entries WHERE account_id='a' AND transaction_id IN (SELECT id FROM financial_transactions WHERE description='Transfer fee')",[],|r|r.get(0)).unwrap();
        assert_eq!(movements, 0);
        assert_eq!(fee, -150);
    }
}
