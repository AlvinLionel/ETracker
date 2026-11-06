use crate::infrastructure::database::Database;
use rusqlite::params;
use serde::Serialize;

#[derive(Serialize)]
pub struct AnalyticsFilterOption {
    pub id: String,
    pub name: String,
    pub currency_code: Option<String>,
}

#[derive(Serialize)]
pub struct AnalyticsPoint {
    pub period: String,
    pub income_minor: i64,
    pub expense_minor: i64,
}

#[derive(Serialize)]
pub struct AnalyticsCategory {
    pub category_id: Option<String>,
    pub name: String,
    pub transaction_type: String,
    pub amount_minor: i64,
}

#[derive(Serialize)]
pub struct AnalyticsAccount {
    pub account_id: String,
    pub name: String,
    pub income_minor: i64,
    pub expense_minor: i64,
}

#[derive(Serialize)]
pub struct UnusualExpense {
    pub transaction_id: String,
    pub occurred_at: String,
    pub description: Option<String>,
    pub category_name: String,
    pub amount_minor: i64,
    pub recent_average_minor: i64,
    pub comparison_count: i64,
}

#[derive(Serialize)]
pub struct BudgetPerformance {
    pub budget_id: String,
    pub name: String,
    pub starts_on: String,
    pub ends_on: String,
    pub planned_income_minor: i64,
    pub planned_expense_minor: i64,
    pub planned_savings_minor: i64,
    pub actual_income_minor: i64,
    pub actual_expense_minor: i64,
    pub actual_savings_minor: i64,
}

#[derive(Serialize)]
pub struct SavingsProgress {
    pub goal_id: String,
    pub name: String,
    pub target_amount_minor: i64,
    pub contributed_minor: i64,
    pub status: String,
}

#[derive(Serialize)]
pub struct AnalyticsSummary {
    pub currencies: Vec<String>,
    pub accounts: Vec<AnalyticsFilterOption>,
    pub categories: Vec<AnalyticsFilterOption>,
    pub total_income_minor: i64,
    pub total_expense_minor: i64,
    pub trends: Vec<AnalyticsPoint>,
    pub category_breakdown: Vec<AnalyticsCategory>,
    pub account_comparison: Vec<AnalyticsAccount>,
    pub recurring_expenses: Vec<AnalyticsCategory>,
    pub unusual_expenses: Vec<UnusualExpense>,
    pub budget_performance: Vec<BudgetPerformance>,
    pub savings_progress: Vec<SavingsProgress>,
}

pub struct SqliteAnalyticsRepository<'a> {
    database: &'a Database,
}

impl<'a> SqliteAnalyticsRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn summarize(
        &self,
        from: Option<&str>,
        to: Option<&str>,
        account_id: Option<&str>,
        category_id: Option<&str>,
        currency_code: Option<&str>,
    ) -> Result<AnalyticsSummary, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut currencies_statement = connection.prepare("SELECT DISTINCT t.currency_code FROM financial_transactions t WHERE t.status='posted' AND t.transaction_type IN ('income','expense') AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted') ORDER BY t.currency_code").map_err(|e| e.to_string())?;
        let currencies = currencies_statement
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<String>>>()
            .map_err(|e| e.to_string())?;
        let effective_currency = currency_code.or(currencies.first().map(String::as_str));
        let mut accounts_statement = connection.prepare("SELECT id,name,currency_code FROM accounts ORDER BY archived_at IS NOT NULL,name COLLATE NOCASE").map_err(|e| e.to_string())?;
        let accounts = accounts_statement
            .query_map([], |r| {
                Ok(AnalyticsFilterOption {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    currency_code: Some(r.get(2)?),
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let mut categories_statement = connection.prepare("SELECT id,name,NULL FROM categories ORDER BY archived_at IS NOT NULL,name COLLATE NOCASE").map_err(|e| e.to_string())?;
        let categories = categories_statement
            .query_map([], |r| {
                Ok(AnalyticsFilterOption {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    currency_code: r.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;

        let filter = "t.status='posted' AND t.transaction_type IN ('income','expense') AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted') AND (?1 IS NULL OR date(t.occurred_at)>=date(?1)) AND (?2 IS NULL OR date(t.occurred_at)<=date(?2)) AND (?3 IS NULL OR e.account_id=?3) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM category_allocations ca WHERE ca.transaction_id=t.id AND ca.category_id=?4)) AND (?5 IS NULL OR t.currency_code=?5)";
        let selected_amount = "CASE WHEN ?4 IS NULL THEN t.amount_minor ELSE COALESCE((SELECT SUM(selected.amount_minor) FROM category_allocations selected WHERE selected.transaction_id=t.id AND selected.category_id=?4),0) END";
        let totals_sql = format!(
            "SELECT COALESCE(SUM(CASE WHEN t.transaction_type='income' THEN {selected_amount} ELSE 0 END),0),COALESCE(SUM(CASE WHEN t.transaction_type='expense' THEN {selected_amount} ELSE 0 END),0) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE {filter}"
        );
        let (total_income_minor, total_expense_minor) = connection
            .query_row(
                &totals_sql,
                params![from, to, account_id, category_id, effective_currency],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let trends_sql = format!(
            "SELECT strftime('%Y-%m',t.occurred_at),COALESCE(SUM(CASE WHEN t.transaction_type='income' THEN {selected_amount} ELSE 0 END),0),COALESCE(SUM(CASE WHEN t.transaction_type='expense' THEN {selected_amount} ELSE 0 END),0) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE {filter} GROUP BY strftime('%Y-%m',t.occurred_at) ORDER BY 1"
        );
        let mut statement = connection.prepare(&trends_sql).map_err(|e| e.to_string())?;
        let trends = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(AnalyticsPoint {
                        period: r.get(0)?,
                        income_minor: r.get(1)?,
                        expense_minor: r.get(2)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let breakdown_sql = format!(
            "SELECT a.category_id,COALESCE(c.name,'Uncategorized'),t.transaction_type,SUM(COALESCE(a.amount_minor,t.amount_minor)) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id LEFT JOIN category_allocations a ON a.transaction_id=t.id AND (?4 IS NULL OR a.category_id=?4) LEFT JOIN categories c ON c.id=a.category_id WHERE {filter} GROUP BY a.category_id,t.transaction_type ORDER BY t.transaction_type,SUM(COALESCE(a.amount_minor,t.amount_minor)) DESC"
        );
        let mut statement = connection
            .prepare(&breakdown_sql)
            .map_err(|e| e.to_string())?;
        let category_breakdown = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(AnalyticsCategory {
                        category_id: r.get(0)?,
                        name: r.get(1)?,
                        transaction_type: r.get(2)?,
                        amount_minor: r.get(3)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let accounts_sql = format!(
            "SELECT e.account_id,a.name,SUM(CASE WHEN t.transaction_type='income' THEN {selected_amount} ELSE 0 END),SUM(CASE WHEN t.transaction_type='expense' THEN {selected_amount} ELSE 0 END) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN accounts a ON a.id=e.account_id WHERE {filter} GROUP BY e.account_id ORDER BY a.name COLLATE NOCASE"
        );
        let mut statement = connection
            .prepare(&accounts_sql)
            .map_err(|e| e.to_string())?;
        let account_comparison = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(AnalyticsAccount {
                        account_id: r.get(0)?,
                        name: r.get(1)?,
                        income_minor: r.get(2)?,
                        expense_minor: r.get(3)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let recurring_sql = format!(
            "SELECT NULL,COALESCE(NULLIF(trim(t.description),''),'Unnamed expense'),t.transaction_type,SUM({selected_amount}) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE {filter} AND t.transaction_type='expense' AND t.description IS NOT NULL AND length(trim(t.description))>0 GROUP BY lower(trim(t.description)) HAVING COUNT(*)>=3 ORDER BY SUM({selected_amount}) DESC LIMIT 10"
        );
        let mut statement = connection
            .prepare(&recurring_sql)
            .map_err(|e| e.to_string())?;
        let recurring_expenses = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(AnalyticsCategory {
                        category_id: r.get(0)?,
                        name: r.get(1)?,
                        transaction_type: r.get(2)?,
                        amount_minor: r.get(3)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let unusual_sql = format!(
            "SELECT t.id,t.occurred_at,t.description,c.name,a.amount_minor,baseline.average_minor,baseline.sample_count FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN category_allocations a ON a.transaction_id=t.id JOIN categories c ON c.id=a.category_id JOIN (SELECT current.id AS transaction_id,current_allocation.category_id AS category_id,AVG(prior_allocation.amount_minor) AS average_minor,COUNT(*) AS sample_count FROM financial_transactions current JOIN category_allocations current_allocation ON current_allocation.transaction_id=current.id JOIN account_entries current_entry ON current_entry.transaction_id=current.id LEFT JOIN category_allocations prior_allocation ON prior_allocation.category_id=current_allocation.category_id JOIN financial_transactions prior ON prior.id=prior_allocation.transaction_id AND prior.transaction_type='expense' AND prior.currency_code=current.currency_code AND date(prior.occurred_at)<date(current.occurred_at) AND date(prior.occurred_at)>=date(current.occurred_at,'-6 months') AND prior.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions prior_reversal WHERE prior_reversal.reversal_of_transaction_id=prior.id AND prior_reversal.status='posted') LEFT JOIN account_entries prior_entry ON prior_entry.transaction_id=prior.id AND (?3 IS NULL OR prior_entry.account_id=?3) WHERE current.transaction_type='expense' AND current.status='posted' AND (?3 IS NULL OR current_entry.account_id=?3) GROUP BY current.id,current_allocation.category_id HAVING COUNT(*)>=3) baseline ON baseline.transaction_id=t.id AND baseline.category_id=a.category_id AND baseline.sample_count>=3 AND a.amount_minor>=baseline.average_minor*2 WHERE {filter} AND t.transaction_type='expense' ORDER BY a.amount_minor DESC LIMIT 20"
        );
        let mut statement = connection
            .prepare(&unusual_sql)
            .map_err(|e| e.to_string())?;
        let unusual_expenses = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(UnusualExpense {
                        transaction_id: r.get(0)?,
                        occurred_at: r.get(1)?,
                        description: r.get(2)?,
                        category_name: r.get(3)?,
                        amount_minor: r.get(4)?,
                        recent_average_minor: r.get::<_, f64>(5)? as i64,
                        comparison_count: r.get(6)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let budgets_sql = "SELECT bp.id,bp.name,bp.starts_on,bp.ends_on,COALESCE((SELECT SUM(i.amount_minor) FROM budget_period_items i WHERE i.budget_id=bp.id AND i.item_type='income' AND (?4 IS NULL OR i.category_id=?4)),0),COALESCE((SELECT SUM(i.amount_minor) FROM budget_period_items i WHERE i.budget_id=bp.id AND i.item_type='expense' AND (?4 IS NULL OR i.category_id=?4)),0),CASE WHEN ?4 IS NULL THEN COALESCE((SELECT SUM(i.amount_minor) FROM budget_period_items i WHERE i.budget_id=bp.id AND i.item_type='savings'),0) ELSE 0 END,COALESCE((SELECT SUM(CASE WHEN ?4 IS NULL THEN t.amount_minor ELSE COALESCE((SELECT SUM(ca.amount_minor) FROM category_allocations ca WHERE ca.transaction_id=t.id AND ca.category_id=?4),0) END) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.currency_code=bp.currency_code AND t.transaction_type='income' AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted') AND date(t.occurred_at) BETWEEN max(date(bp.starts_on),COALESCE(date(?1),date(bp.starts_on))) AND min(date(bp.ends_on),COALESCE(date(?2),date(bp.ends_on))) AND (?3 IS NULL OR e.account_id=?3) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM category_allocations x WHERE x.transaction_id=t.id AND x.category_id=?4))),0),COALESCE((SELECT SUM(CASE WHEN ?4 IS NULL THEN t.amount_minor ELSE COALESCE((SELECT SUM(ca.amount_minor) FROM category_allocations ca WHERE ca.transaction_id=t.id AND ca.category_id=?4),0) END) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.currency_code=bp.currency_code AND t.transaction_type='expense' AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted') AND date(t.occurred_at) BETWEEN max(date(bp.starts_on),COALESCE(date(?1),date(bp.starts_on))) AND min(date(bp.ends_on),COALESCE(date(?2),date(bp.ends_on))) AND (?3 IS NULL OR e.account_id=?3) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM category_allocations x WHERE x.transaction_id=t.id AND x.category_id=?4))),0),CASE WHEN ?4 IS NULL THEN COALESCE((SELECT SUM(c.amount_minor) FROM savings_contributions c JOIN savings_goals g ON g.id=c.goal_id AND g.currency_code=bp.currency_code WHERE date(c.contribution_date) BETWEEN max(date(bp.starts_on),COALESCE(date(?1),date(bp.starts_on))) AND min(date(bp.ends_on),COALESCE(date(?2),date(bp.ends_on))) AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions t WHERE t.id=c.transaction_id AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted'))) AND (?3 IS NULL OR (c.transaction_id IS NOT NULL AND EXISTS(SELECT 1 FROM account_entries e WHERE e.transaction_id=c.transaction_id AND e.account_id=?3)))),0) ELSE 0 END FROM budget_periods bp WHERE bp.currency_code=?5 AND (?1 IS NULL OR bp.ends_on>=?1) AND (?2 IS NULL OR bp.starts_on<=?2) ORDER BY bp.starts_on DESC";
        let mut statement = connection.prepare(budgets_sql).map_err(|e| e.to_string())?;
        let budget_performance = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(BudgetPerformance {
                        budget_id: r.get(0)?,
                        name: r.get(1)?,
                        starts_on: r.get(2)?,
                        ends_on: r.get(3)?,
                        planned_income_minor: r.get(4)?,
                        planned_expense_minor: r.get(5)?,
                        planned_savings_minor: r.get(6)?,
                        actual_income_minor: r.get(7)?,
                        actual_expense_minor: r.get(8)?,
                        actual_savings_minor: r.get(9)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        let savings_sql = "SELECT g.id,g.name,g.target_amount_minor,COALESCE(SUM(c.amount_minor),0),g.status FROM savings_goals g LEFT JOIN savings_contributions c ON c.goal_id=g.id AND date(c.contribution_date)>=COALESCE(date(?1),date('0001-01-01')) AND date(c.contribution_date)<=COALESCE(date(?2),date('9999-12-31')) AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions t WHERE t.id=c.transaction_id AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted'))) AND (?3 IS NULL OR (c.transaction_id IS NOT NULL AND EXISTS(SELECT 1 FROM account_entries e WHERE e.transaction_id=c.transaction_id AND e.account_id=?3))) WHERE g.currency_code=?5 AND ?4 IS NULL GROUP BY g.id ORDER BY CASE g.status WHEN 'active' THEN 0 WHEN 'completed' THEN 1 ELSE 2 END,g.name COLLATE NOCASE";
        let mut statement = connection.prepare(savings_sql).map_err(|e| e.to_string())?;
        let savings_progress = statement
            .query_map(
                params![from, to, account_id, category_id, effective_currency],
                |r| {
                    Ok(SavingsProgress {
                        goal_id: r.get(0)?,
                        name: r.get(1)?,
                        target_amount_minor: r.get(2)?,
                        contributed_minor: r.get(3)?,
                        status: r.get(4)?,
                    })
                },
            )
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        Ok(AnalyticsSummary {
            currencies,
            accounts,
            categories,
            total_income_minor,
            total_expense_minor,
            trends,
            category_breakdown,
            account_comparison,
            recurring_expenses,
            unusual_expenses,
            budget_performance,
            savings_progress,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn filters_and_aggregates_posted_ledger_without_transfers_or_reversed_activity() {
        let db = Database::initialize(Path::new(":memory:")).unwrap();
        let c = db.connection().lock().unwrap();
        c.execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('cash','Cash','cash','KES'),('bank','Bank','bank','KES')",[]).unwrap();
        c.execute(
            "INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense'),('rent','Rent','expense'),('utilities','Utilities','expense'),('salary','Salary','income')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type,status,voided_at,reversal_of_transaction_id) VALUES ('e1','expense',1000,'KES','2026-10-01T10:00:00Z','manual','posted',NULL,NULL),('e2','expense',300,'KES','2026-10-02T10:00:00Z','manual','posted',NULL,NULL),('e-split','expense',1200,'KES','2026-10-03T10:00:00Z','manual','posted',NULL,NULL),('i1','income',2000,'KES','2026-10-03T10:00:00Z','manual','posted',NULL,NULL),('e3','expense',500,'KES','2026-11-01T10:00:00Z','manual','posted',NULL,NULL),('transfer','transfer',4000,'KES','2026-10-03T10:00:00Z','manual','posted',NULL,NULL),('void','expense',700,'KES','2026-10-04T10:00:00Z','manual','voided',CURRENT_TIMESTAMP,NULL),('reversal','reversal',1000,'KES','2026-10-05T10:00:00Z','reversal','posted',NULL,'e1')",[]).unwrap();
        c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('a1','e1','cash',-1000),('a2','e2','bank',-300),('a-split','e-split','cash',-1200),('a3','i1','cash',2000),('a4','e3','cash',-500),('a5','transfer','cash',-4000),('a6','transfer','bank',4000),('a7','void','cash',-700),('a8','reversal','cash',1000)",[]).unwrap();
        c.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES ('ca1','e1','food',1000),('ca2','e2','food',300),('ca3','e-split','food',300),('ca4','e-split','rent',900)",[]).unwrap();
        c.execute("INSERT INTO budget_templates(id,name,currency_code) VALUES ('template','October plan','KES')",[]).unwrap();
        c.execute("INSERT INTO budget_periods(id,template_id,name,currency_code,starts_on,ends_on) VALUES ('budget','template','October plan — 2026-10','KES','2026-10-01','2026-10-31')",[]).unwrap();
        c.execute("INSERT INTO budget_period_items(id,budget_id,item_type,category_id,label,amount_minor) VALUES ('planned-income','budget','income','salary','Salary',5000),('planned-expense','budget','expense','food','Food',600),('planned-savings','budget','savings',NULL,'Reserve',2000)",[]).unwrap();
        c.execute("INSERT INTO savings_goals(id,name,target_amount_minor,currency_code) VALUES ('goal','Reserve',2000,'KES')",[]).unwrap();
        c.execute("INSERT INTO savings_contributions(id,goal_id,amount_minor,contribution_date) VALUES ('contribution','goal',500,'2026-10-12')",[]).unwrap();
        drop(c);
        let summary = SqliteAnalyticsRepository::new(&db)
            .summarize(
                Some("2026-10-01"),
                Some("2026-10-31"),
                None,
                None,
                Some("KES"),
            )
            .unwrap();
        assert_eq!(
            (summary.total_income_minor, summary.total_expense_minor),
            (2000, 1500)
        );
        assert_eq!(summary.trends.len(), 1);
        assert_eq!(summary.category_breakdown[0].amount_minor, 900);
        assert_eq!(summary.account_comparison.len(), 2);
        assert_eq!(summary.budget_performance.len(), 1);
        assert_eq!(summary.budget_performance[0].planned_expense_minor, 600);
        assert_eq!(summary.budget_performance[0].actual_expense_minor, 1500);
        assert_eq!(summary.budget_performance[0].actual_savings_minor, 500);
        assert_eq!(summary.savings_progress[0].contributed_minor, 500);
        let filtered = SqliteAnalyticsRepository::new(&db)
            .summarize(None, None, None, Some("food"), Some("KES"))
            .unwrap();
        assert_eq!(
            (filtered.total_income_minor, filtered.total_expense_minor),
            (0, 600)
        );
        assert_eq!(filtered.category_breakdown.len(), 1);
        assert_eq!(filtered.category_breakdown[0].name, "Food");
        let account_filtered = SqliteAnalyticsRepository::new(&db)
            .summarize(None, None, Some("bank"), None, Some("KES"))
            .unwrap();
        assert_eq!(account_filtered.total_expense_minor, 300);
        let connection = db.connection().lock().unwrap();
        for (index, date) in ["2026-05-01", "2026-06-01", "2026-07-01"]
            .iter()
            .enumerate()
        {
            let tx = format!("utility-history-{index}");
            connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES (?1,'expense',100,'KES',?2,'manual')", params![tx,date]).unwrap();
            connection.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES (?1,?2,'cash',-100)", params![format!("utility-entry-{index}"),tx]).unwrap();
            connection.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES (?1,?2,'utilities',100)", params![format!("utility-category-{index}"),tx]).unwrap();
        }
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('unusual','expense',5000,'KES','2026-10-20T10:00:00Z','manual')",[]).unwrap();
        connection.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('unusual-entry','unusual','cash',-5000)",[]).unwrap();
        connection.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES ('unusual-category','unusual','utilities',5000)",[]).unwrap();
        drop(connection);
        let unusual = SqliteAnalyticsRepository::new(&db)
            .summarize(
                Some("2026-10-01"),
                Some("2026-10-31"),
                None,
                Some("utilities"),
                Some("KES"),
            )
            .unwrap();
        assert_eq!(unusual.unusual_expenses.len(), 1);
        assert_eq!(unusual.unusual_expenses[0].comparison_count, 3);
    }
}
