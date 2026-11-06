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
            "INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense'),('rent','Rent','expense')",
            [],
        )
        .unwrap();
        c.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type,status,voided_at,reversal_of_transaction_id) VALUES ('e1','expense',1000,'KES','2026-10-01T10:00:00Z','manual','posted',NULL,NULL),('e2','expense',300,'KES','2026-10-02T10:00:00Z','manual','posted',NULL,NULL),('e-split','expense',1200,'KES','2026-10-03T10:00:00Z','manual','posted',NULL,NULL),('i1','income',2000,'KES','2026-10-03T10:00:00Z','manual','posted',NULL,NULL),('e3','expense',500,'KES','2026-11-01T10:00:00Z','manual','posted',NULL,NULL),('transfer','transfer',4000,'KES','2026-10-03T10:00:00Z','manual','posted',NULL,NULL),('void','expense',700,'KES','2026-10-04T10:00:00Z','manual','voided',CURRENT_TIMESTAMP,NULL),('reversal','reversal',1000,'KES','2026-10-05T10:00:00Z','reversal','posted',NULL,'e1')",[]).unwrap();
        c.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('a1','e1','cash',-1000),('a2','e2','bank',-300),('a-split','e-split','cash',-1200),('a3','i1','cash',2000),('a4','e3','cash',-500),('a5','transfer','cash',-4000),('a6','transfer','bank',4000),('a7','void','cash',-700),('a8','reversal','cash',1000)",[]).unwrap();
        c.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES ('ca1','e1','food',1000),('ca2','e2','food',300),('ca3','e-split','food',300),('ca4','e-split','rent',900)",[]).unwrap();
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
    }
}
