use crate::infrastructure::database::Database;
use serde::Serialize;

#[derive(Serialize)]
pub struct DashboardAccount {
    pub id: String,
    pub name: String,
    pub currency_code: String,
    pub balance_minor: i64,
}
#[derive(Serialize)]
pub struct DashboardTransaction {
    pub id: String,
    pub account_id: String,
    pub account_name: String,
    pub transaction_type: String,
    pub amount_minor: i64,
    pub entry_amount_minor: i64,
    pub currency_code: String,
    pub occurred_at: String,
    pub description: Option<String>,
}
#[derive(Serialize)]
pub struct DashboardBudget {
    pub id: String,
    pub name: String,
    pub currency_code: String,
    pub ends_on: String,
    pub planned_expense_minor: i64,
    pub actual_expense_minor: i64,
}
#[derive(Serialize)]
pub struct DashboardGoal {
    pub id: String,
    pub name: String,
    pub currency_code: String,
    pub target_amount_minor: i64,
    pub contributed_minor: i64,
}
#[derive(Serialize)]
pub struct DashboardSubscription {
    pub id: String,
    pub name: String,
    pub currency_code: String,
    pub amount_minor: i64,
    pub next_due_date: String,
}
#[derive(Serialize)]
pub struct DashboardSummary {
    pub currency_code: String,
    pub balance_minor: i64,
    pub income_minor: i64,
    pub expense_minor: i64,
    pub refund_minor: i64,
    pub net_spending_minor: i64,
    pub accounts: Vec<DashboardAccount>,
    pub recent_transactions: Vec<DashboardTransaction>,
    pub budgets: Vec<DashboardBudget>,
    pub savings_goals: Vec<DashboardGoal>,
    pub upcoming_subscriptions: Vec<DashboardSubscription>,
    pub import_batches_pending_review: i64,
    pub import_batches_with_discrepancy: i64,
}

pub struct SqliteDashboardRepository<'a> {
    database: &'a Database,
}
impl<'a> SqliteDashboardRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }
    pub fn summary(&self) -> Result<DashboardSummary, String> {
        let c = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database unavailable")?;
        let accounts = {
            let mut s=c.prepare("SELECT a.id,a.name,a.currency_code,COALESCE(SUM(CASE WHEN t.status='posted' THEN e.amount_minor ELSE 0 END),0) FROM accounts a LEFT JOIN account_entries e ON e.account_id=a.id LEFT JOIN financial_transactions t ON t.id=e.transaction_id GROUP BY a.id ORDER BY a.archived_at IS NOT NULL,a.name COLLATE NOCASE LIMIT 8").map_err(|e|e.to_string())?;
            s.query_map([], |r| {
                Ok(DashboardAccount {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    currency_code: r.get(2)?,
                    balance_minor: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
        };
        let (income,expense,refund):(i64,i64,i64)=c.query_row("SELECT COALESCE(SUM(CASE WHEN t.transaction_type='income' AND NOT EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id) THEN t.amount_minor ELSE 0 END),0),COALESCE(SUM(CASE WHEN t.transaction_type='expense' THEN t.amount_minor ELSE 0 END),0),COALESCE(SUM(CASE WHEN EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id) THEN t.amount_minor ELSE 0 END),0) FROM financial_transactions t WHERE t.status='posted' AND t.currency_code='KES' AND date(t.occurred_at)>=date('now','start of month') AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted')",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e|e.to_string())?;
        let recent_transactions = {
            let mut s=c.prepare("SELECT t.id,e.account_id,a.name,CASE WHEN EXISTS(SELECT 1 FROM refund_transactions rf WHERE rf.transaction_id=t.id) THEN 'refund' ELSE t.transaction_type END,t.amount_minor,e.amount_minor,t.currency_code,t.occurred_at,t.description FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN accounts a ON a.id=e.account_id WHERE t.status='posted' AND t.transaction_type IN ('income','expense','transfer') ORDER BY datetime(t.occurred_at) DESC LIMIT 8").map_err(|e|e.to_string())?;
            s.query_map([], |r| {
                Ok(DashboardTransaction {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    account_name: r.get(2)?,
                    transaction_type: r.get(3)?,
                    amount_minor: r.get(4)?,
                    entry_amount_minor: r.get(5)?,
                    currency_code: r.get(6)?,
                    occurred_at: r.get(7)?,
                    description: r.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
        };
        let budgets = {
            let mut s=c.prepare("SELECT bp.id,bp.name,bp.currency_code,bp.ends_on,COALESCE((SELECT SUM(amount_minor) FROM budget_period_items WHERE budget_id=bp.id AND item_type='expense'),0),COALESCE((SELECT SUM(t.amount_minor) FROM financial_transactions t WHERE t.currency_code=bp.currency_code AND t.transaction_type='expense' AND t.status='posted' AND date(t.occurred_at) BETWEEN bp.starts_on AND bp.ends_on AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted')),0) FROM budget_periods bp WHERE bp.ends_on>=date('now') ORDER BY bp.starts_on LIMIT 4").map_err(|e|e.to_string())?;
            s.query_map([], |r| {
                Ok(DashboardBudget {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    currency_code: r.get(2)?,
                    ends_on: r.get(3)?,
                    planned_expense_minor: r.get(4)?,
                    actual_expense_minor: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
        };
        let savings_goals = {
            let mut s=c.prepare("SELECT g.id,g.name,g.currency_code,g.target_amount_minor,COALESCE(SUM(c.amount_minor),0) FROM savings_goals g LEFT JOIN savings_contributions c ON c.goal_id=g.id AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions t WHERE t.id=c.transaction_id AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted'))) WHERE g.status='active' GROUP BY g.id ORDER BY g.name LIMIT 4").map_err(|e|e.to_string())?;
            s.query_map([], |r| {
                Ok(DashboardGoal {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    currency_code: r.get(2)?,
                    target_amount_minor: r.get(3)?,
                    contributed_minor: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
        };
        let upcoming_subscriptions = {
            let mut s=c.prepare("SELECT id,name,currency_code,amount_minor,next_due_date FROM subscriptions WHERE status='active' AND date(next_due_date) BETWEEN date('now') AND date('now','+30 days') ORDER BY next_due_date LIMIT 8").map_err(|e|e.to_string())?;
            s.query_map([], |r| {
                Ok(DashboardSubscription {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    currency_code: r.get(2)?,
                    amount_minor: r.get(3)?,
                    next_due_date: r.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
        };
        let (import_batches_pending_review, import_batches_with_discrepancy): (i64, i64) = c
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(CASE WHEN reconciliation_status='discrepancy' THEN 1 ELSE 0 END),0) FROM import_batches WHERE status='review' AND discarded_at IS NULL",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let balance: i64 = c
            .query_row("SELECT COALESCE(SUM(e.amount_minor),0) FROM account_entries e JOIN accounts a ON a.id=e.account_id JOIN financial_transactions t ON t.id=e.transaction_id WHERE a.currency_code='KES' AND t.status='posted'", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        Ok(DashboardSummary {
            currency_code: "KES".into(),
            balance_minor: balance,
            income_minor: income,
            expense_minor: expense,
            refund_minor: refund,
            net_spending_minor: expense - refund,
            accounts,
            recent_transactions,
            budgets,
            savings_goals,
            upcoming_subscriptions,
            import_batches_pending_review,
            import_batches_with_discrepancy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn dashboard_uses_posted_ledger_and_separates_refunds() {
        let db = Database::initialize(Path::new(":memory:")).unwrap();
        let c = db.connection().lock().unwrap();
        c.execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('cash','Cash','cash','KES')", []).unwrap();
        c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('expense','expense',5000,'KES',datetime('now'),'manual'),('refund','income',2000,'KES',datetime('now'),'manual'),('income','income',10000,'KES',datetime('now'),'manual')", []).unwrap();
        c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('e1','expense','cash',-5000),('e2','refund','cash',2000),('e3','income','cash',10000)", []).unwrap();
        c.execute(
            "INSERT INTO refund_transactions(transaction_id) VALUES ('refund')",
            [],
        )
        .unwrap();
        drop(c);

        let summary = SqliteDashboardRepository::new(&db).summary().unwrap();
        assert_eq!(summary.balance_minor, 7000);
        assert_eq!(
            (
                summary.income_minor,
                summary.expense_minor,
                summary.refund_minor,
                summary.net_spending_minor
            ),
            (10000, 5000, 2000, 3000)
        );
        assert!(
            summary
                .recent_transactions
                .iter()
                .any(|t| t.id == "refund" && t.transaction_type == "refund")
        );
    }

    #[test]
    fn dashboard_counts_only_active_import_workspaces_and_reconciliation_discrepancies() {
        let db = Database::initialize(Path::new(":memory:")).unwrap();
        let c = db.connection().lock().unwrap();
        c.execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('cash','Cash','cash','KES')", []).unwrap();
        c.execute("INSERT INTO import_batches(id,account_id,source_file_name,source_format,status,reconciliation_status) VALUES ('open','cash','open.csv','csv','review','unavailable'),('mismatch','cash','mismatch.csv','csv','review','discrepancy'),('done','cash','done.csv','csv','finalized','reconciled'),('discarded','cash','discarded.csv','csv','review','discrepancy')", []).unwrap();
        c.execute(
            "UPDATE import_batches SET discarded_at=CURRENT_TIMESTAMP WHERE id='discarded'",
            [],
        )
        .unwrap();
        drop(c);

        let summary = SqliteDashboardRepository::new(&db).summary().unwrap();
        assert_eq!(summary.import_batches_pending_review, 2);
        assert_eq!(summary.import_batches_with_discrepancy, 1);
    }
}
