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
    pub is_system: bool,
}
#[derive(serde::Serialize)]
pub struct TransactionAuditRecord {
    pub action: String,
    pub previous_values: Option<String>,
    pub new_values: Option<String>,
    pub reason: Option<String>,
    pub created_at: String,
}
#[derive(serde::Serialize)]
pub struct EligibleRefund {
    pub transaction_id: String,
    pub amount_minor: i64,
    pub refundable_minor: i64,
    pub occurred_at: String,
    pub description: Option<String>,
}
impl SqliteTransactionRepository<'_> {
    pub fn eligible_refunds(
        &self,
        account_id: &str,
    ) -> Result<Vec<EligibleRefund>, TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let mut statement = connection.prepare("SELECT t.id,t.amount_minor,t.occurred_at,t.description,MAX(0,t.amount_minor-COALESCE((SELECT SUM(r.amount_minor) FROM refund_transactions rf JOIN financial_transactions r ON r.id=rf.transaction_id WHERE rf.original_expense_transaction_id=t.id AND r.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=r.id)),0)) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.transaction_type='expense' AND t.status='posted' AND e.account_id=?1 AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=t.id) ORDER BY datetime(t.occurred_at) DESC").map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map([account_id], |r| {
                Ok(EligibleRefund {
                    transaction_id: r.get(0)?,
                    amount_minor: r.get(1)?,
                    occurred_at: r.get(2)?,
                    description: r.get(3)?,
                    refundable_minor: r.get(4)?,
                })
            })
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
    }

    pub fn create_refund(
        &self,
        item: &NewTransaction,
        original_id: Option<&str>,
    ) -> Result<String, TransactionRepositoryError> {
        if item.transaction_type != crate::domain::transactions::TransactionType::Income
            || item.amount_minor <= 0
        {
            return Err(TransactionRepositoryError::InvalidCategory);
        }
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
        let mut allocations: Vec<(String, i64)> = Vec::new();
        if let Some(original_id) = original_id {
            let original: Option<(i64,String,String,String)> = tx.query_row("SELECT t.amount_minor,t.currency_code,e.account_id,t.transaction_type FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.id=?1 AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=t.id AND rv.status='posted')", [original_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
            let Some((original_amount, original_currency, original_account, kind)) = original
            else {
                return Err(TransactionRepositoryError::NotReversible);
            };
            if kind != "expense"
                || original_account != item.account_id
                || original_currency != item.currency_code
            {
                return Err(TransactionRepositoryError::NotReversible);
            }
            let refunded: i64 = tx.query_row("SELECT COALESCE(SUM(r.amount_minor),0) FROM refund_transactions rf JOIN financial_transactions r ON r.id=rf.transaction_id WHERE rf.original_expense_transaction_id=?1 AND r.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=r.id)", [original_id], |r| r.get(0)).map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
            if item.amount_minor > original_amount - refunded {
                return Err(TransactionRepositoryError::RefundLimitExceeded);
            }
            let parts: Vec<(String, i64)> = {
                let mut s=tx.prepare("SELECT category_id,amount_minor FROM category_allocations WHERE transaction_id=?1 ORDER BY id").map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
                s.query_map([original_id], |r| Ok((r.get(0)?, r.get(1)?)))
                    .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
                    .collect::<rusqlite::Result<Vec<_>>>()
                    .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            };
            let total: i64 = parts.iter().map(|p| p.1).sum();
            if total > 0 {
                let mut assigned = 0;
                for (idx, (category, amount)) in parts.iter().enumerate() {
                    let share = if idx + 1 == parts.len() {
                        item.amount_minor - assigned
                    } else {
                        ((item.amount_minor as i128 * *amount as i128) / total as i128) as i64
                    };
                    assigned += share;
                    if share > 0 {
                        allocations.push((category.clone(), share));
                    }
                }
            }
        }
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,notes,source_type) VALUES (?1,'income',?2,?3,?4,?5,?6,'manual')",params![id,item.amount_minor,item.currency_code,item.occurred_at,item.description,item.notes]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,item.account_id,item.amount_minor]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO refund_transactions(transaction_id,original_expense_transaction_id) VALUES (?1,?2)",params![id,original_id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        for (category, amount) in allocations {
            tx.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES (?1,?2,?3,?4)",params![Uuid::new_v4().to_string(),id,category,amount]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        }
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'created',?3)",params![Uuid::new_v4().to_string(),id,serde_json::json!({"type":"refund","amount_minor":item.amount_minor,"original_expense_transaction_id":original_id}).to_string()]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if let Some(original_id) = original_id {
            tx.execute(
                "INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'refund_received',?3)",
                params![Uuid::new_v4().to_string(), original_id, serde_json::json!({"refund_transaction_id": id, "amount_minor": item.amount_minor, "description": item.description.as_deref()}).to_string()],
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        }
        tx.commit()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(id)
    }

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
            "SELECT id,name,category_type,is_system FROM categories WHERE archived_at IS NULL AND (?1 IS NULL OR category_type=?1) ORDER BY is_system DESC, lower(name)"
        ).map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        statement
            .query_map([category_type], |row| {
                Ok(CategoryRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category_type: row.get(2)?,
                    is_system: row.get(3)?,
                })
            })
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))
    }

    pub fn create_category(
        &self,
        name: &str,
        category_type: &str,
    ) -> Result<CategoryRecord, TransactionRepositoryError> {
        let name = name.trim();
        if name.is_empty()
            || name.chars().count() > 100
            || !matches!(category_type, "income" | "expense")
        {
            return Err(TransactionRepositoryError::InvalidCategory);
        }
        let id = Uuid::new_v4().to_string();
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        connection
            .execute(
                "INSERT INTO categories(id,name,category_type,is_system) VALUES (?1,?2,?3,0)",
                params![id, name, category_type],
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(CategoryRecord {
            id,
            name: name.to_owned(),
            category_type: category_type.to_owned(),
            is_system: false,
        })
    }

    pub fn archive_category(&self, id: &str) -> Result<(), TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let changed = connection
            .execute(
                "UPDATE categories SET archived_at=CURRENT_TIMESTAMP,updated_at=CURRENT_TIMESTAMP WHERE id=?1 AND archived_at IS NULL AND is_system=0",
                [id],
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        if changed == 0 {
            return Err(TransactionRepositoryError::InvalidCategory);
        }
        Ok(())
    }

    pub fn list(
        &self,
        account_id: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
    ) -> Result<Vec<TransactionRecord>, TransactionRepositoryError> {
        self.list_page(account_id, from, to, None, None, None, 0, 1000)
            .map(|page| page.items)
    }

    pub fn list_page(
        &self,
        account_id: Option<&str>,
        from: Option<&str>,
        to: Option<&str>,
        query: Option<&str>,
        status: Option<&str>,
        flow: Option<&str>,
        offset: usize,
        limit: usize,
    ) -> Result<TransactionPage, TransactionRepositoryError> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| TransactionRepositoryError::ConnectionPoisoned)?;
        let pattern =
            query.map(|value| format!("%{}%", value.replace('%', "\\%").replace('_', "\\_")));
        let where_clause = "FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN accounts a ON a.id=e.account_id WHERE (?1 IS NULL OR e.account_id=?1) AND (?2 IS NULL OR date(t.occurred_at)>=date(?2)) AND (?3 IS NULL OR date(t.occurred_at)<=date(?3)) AND (?4 IS NULL OR t.status=?4) AND (?5 IS NULL OR t.description LIKE ?5 ESCAPE '\\' OR a.name LIKE ?5 ESCAPE '\\' OR t.source_reference LIKE ?5 ESCAPE '\\') AND (?6 IS NULL OR (?6='income' AND t.transaction_type='income') OR (?6='expense' AND (t.transaction_type='expense' OR EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id))) OR (?6='transfers' AND t.transaction_type='transfer'))";
        let count_sql = format!("SELECT COUNT(*) {where_clause}");
        let total: i64 = connection
            .query_row(
                &count_sql,
                params![account_id, from, to, status, pattern, flow],
                |r| r.get(0),
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let sql = format!(
            "SELECT t.id,e.account_id,CASE WHEN EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id) THEN 'refund' ELSE t.transaction_type END,t.amount_minor,e.amount_minor,t.currency_code,t.occurred_at,t.description,t.status {where_clause} GROUP BY t.id,e.account_id ORDER BY datetime(t.occurred_at) DESC,t.created_at DESC LIMIT ?7 OFFSET ?8"
        );
        let mut statement = connection
            .prepare(&sql)
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        let items = statement
            .query_map(
                params![
                    account_id,
                    from,
                    to,
                    status,
                    pattern,
                    flow,
                    limit.clamp(1, 1000) as i64,
                    offset.min(1_000_000) as i64
                ],
                |r| {
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
                },
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        Ok(TransactionPage {
            items,
            total: total.max(0) as usize,
        })
    }
}

#[derive(serde::Serialize)]
pub struct TransactionPage {
    pub items: Vec<TransactionRecord>,
    pub total: usize,
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
        let has_refunds: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM refund_transactions rf JOIN financial_transactions r ON r.id=rf.transaction_id WHERE rf.original_expense_transaction_id=?1 AND r.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=r.id AND rv.status='posted'))", [id], |r| r.get(0)).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if has_refunds {
            return Err(TransactionRepositoryError::NotEditable);
        }
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
        let refund_original: Option<String> = tx
            .query_row(
                "SELECT original_expense_transaction_id FROM refund_transactions WHERE transaction_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .flatten();
        if source.as_deref() != Some("manual") {
            return Err(TransactionRepositoryError::NotEditable);
        }
        let has_refunds: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM refund_transactions rf JOIN financial_transactions r ON r.id=rf.transaction_id WHERE rf.original_expense_transaction_id=?1 AND r.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=r.id AND rv.status='posted'))", [id], |r| r.get(0)).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if has_refunds {
            return Err(TransactionRepositoryError::NotEditable);
        }
        let changed=tx.execute("UPDATE financial_transactions SET status='voided',voided_at=CURRENT_TIMESTAMP,void_reason=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2 AND status='posted'",params![reason,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if changed != 1 {
            return Err(TransactionRepositoryError::NotEditable);
        }
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,reason) VALUES (?1,?2,'voided',?3)",params![Uuid::new_v4().to_string(),id,reason]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if let Some(original_id) = refund_original {
            tx.execute(
                "INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'refund_voided',?3)",
                params![Uuid::new_v4().to_string(), original_id, serde_json::json!({"refund_transaction_id": id}).to_string()],
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        }
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
        let refund_original: Option<String> = tx
            .query_row(
                "SELECT original_expense_transaction_id FROM refund_transactions WHERE transaction_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?
            .flatten();
        if !matches!(kind.as_str(), "income" | "expense" | "transfer") || source != "manual" {
            return Err(TransactionRepositoryError::NotReversible);
        }
        let has_refunds: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM refund_transactions rf JOIN financial_transactions r ON r.id=rf.transaction_id WHERE rf.original_expense_transaction_id=?1 AND r.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions rv WHERE rv.reversal_of_transaction_id=r.id AND rv.status='posted'))", [id], |r| r.get(0)).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if has_refunds {
            return Err(TransactionRepositoryError::NotReversible);
        }
        let reversal_id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,description,source_type,reversal_of_transaction_id) SELECT ?1,'reversal',amount_minor,currency_code,datetime('now'),description,'reversal',id FROM financial_transactions WHERE id=?2",params![reversal_id,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) SELECT ?1,?2,account_id,-amount_minor FROM account_entries WHERE transaction_id=?3",params![Uuid::new_v4().to_string(),reversal_id,id]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        tx.execute("INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'reversed',?3)",params![Uuid::new_v4().to_string(),id,serde_json::json!({"reversal_id":reversal_id}).to_string()]).map_err(|e|TransactionRepositoryError::Database(e.to_string()))?;
        if let Some(original_id) = refund_original {
            tx.execute(
                "INSERT INTO transaction_audit_log(id,transaction_id,action,new_values) VALUES (?1,?2,'refund_reversed',?3)",
                params![Uuid::new_v4().to_string(), original_id, serde_json::json!({"refund_transaction_id": id, "reversal_id": reversal_id}).to_string()],
            )
            .map_err(|e| TransactionRepositoryError::Database(e.to_string()))?;
        }
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
    fn creates_trimmed_typed_categories_and_rejects_empty_or_duplicate_names() {
        let db = db();
        let repository = SqliteTransactionRepository::new(&db);
        let category = repository.create_category("  Food  ", "expense").unwrap();
        assert_eq!(category.name, "Food");
        assert_eq!(category.category_type, "expense");
        assert!(!category.is_system);
        assert!(matches!(
            repository.create_category("  ", "expense"),
            Err(TransactionRepositoryError::InvalidCategory)
        ));
        assert!(matches!(
            repository.create_category("Salary", "transfer"),
            Err(TransactionRepositoryError::InvalidCategory)
        ));
        assert!(matches!(
            repository.create_category("food", "expense"),
            Err(TransactionRepositoryError::Database(_))
        ));
    }

    #[test]
    fn archives_user_categories_without_deleting_history_or_system_categories() {
        let db = db();
        db.connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO categories(id,name,category_type,is_system) VALUES ('system','System','expense',1)",
                [],
            )
            .unwrap();
        let repository = SqliteTransactionRepository::new(&db);
        let category = repository.create_category("Travel", "expense").unwrap();
        repository.archive_category(&category.id).unwrap();
        let active = repository.list_active_categories(Some("expense")).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "system");
        let connection = db.connection().lock().unwrap();
        let remaining: (i64, i64) = connection
            .query_row(
                "SELECT COUNT(*),SUM(CASE WHEN archived_at IS NOT NULL THEN 1 ELSE 0 END) FROM categories WHERE id=?1",
                [&category.id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(remaining, (1, 1));
        drop(connection);
        assert!(matches!(
            repository.archive_category("system"),
            Err(TransactionRepositoryError::InvalidCategory)
        ));
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

    #[test]
    fn refunds_are_distinct_proportional_and_cannot_exceed_original() {
        let db = db();
        account(&db, "a");
        db.connection().lock().unwrap().execute("INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense'),('home','Home','expense')",[]).unwrap();
        let repository = SqliteTransactionRepository::new(&db);
        let mut expense = input("a", TransactionType::Expense);
        expense.allocations = vec![
            Allocation {
                category_id: "food".into(),
                amount_minor: 700,
            },
            Allocation {
                category_id: "home".into(),
                amount_minor: 500,
            },
        ];
        let original = repository.create(&expense).unwrap();
        let mut refund = input("a", TransactionType::Income);
        refund.amount_minor = 600;
        let refund_id = repository.create_refund(&refund, Some(&original)).unwrap();
        let listed = repository.list(Some("a"), None, None).unwrap();
        assert_eq!(
            listed
                .iter()
                .find(|record| record.id == refund_id)
                .unwrap()
                .transaction_type,
            "refund"
        );
        let allocations = repository.allocations(&refund_id).unwrap();
        assert_eq!(allocations.iter().map(|a| a.amount_minor).sum::<i64>(), 600);
        assert!(matches!(
            repository.create_refund(
                &{
                    let mut r = refund.clone();
                    r.amount_minor = 601;
                    r
                },
                Some(&original)
            ),
            Err(TransactionRepositoryError::RefundLimitExceeded)
        ));
        assert!(matches!(
            repository.reverse(&original),
            Err(TransactionRepositoryError::NotReversible)
        ));
        assert!(matches!(
            repository.update(&original, &expense),
            Err(TransactionRepositoryError::NotEditable)
        ));
        assert_eq!(
            repository
                .eligible_refunds("a")
                .unwrap()
                .iter()
                .find(|r| r.transaction_id == original)
                .unwrap()
                .refundable_minor,
            600
        );
    }

    #[test]
    fn linked_refund_audit_is_visible_from_original_and_tracks_voids_and_reversals() {
        let db = db();
        account(&db, "a");
        let repository = SqliteTransactionRepository::new(&db);
        let mut expense = input("a", TransactionType::Expense);
        expense.amount_minor = 1_000;
        let original = repository.create(&expense).unwrap();

        let mut refund = input("a", TransactionType::Income);
        refund.amount_minor = 300;
        refund.description = Some("Linked refund".into());
        let reversed_refund = repository.create_refund(&refund, Some(&original)).unwrap();
        let received_event = repository
            .audit_history(&original)
            .unwrap()
            .into_iter()
            .find(|event| event.action == "refund_received")
            .unwrap();
        let received_values = received_event.new_values.unwrap();
        assert!(received_values.contains(&reversed_refund));
        assert!(received_values.contains("Linked refund"));

        repository.reverse(&reversed_refund).unwrap();
        let mut second_refund = refund.clone();
        second_refund.amount_minor = 200;
        let voided_refund = repository
            .create_refund(&second_refund, Some(&original))
            .unwrap();
        repository.void(&voided_refund, "Duplicate refund").unwrap();

        let events = repository.audit_history(&original).unwrap();
        assert!(events.iter().any(|event| event.action == "refund_reversed"));
        assert!(events.iter().any(|event| event.action == "refund_voided"));
    }
}
