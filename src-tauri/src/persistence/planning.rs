use crate::infrastructure::database::Database;
use chrono::Datelike;
use rusqlite::{OptionalExtension, params};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, serde::Deserialize, Serialize)]
pub struct PlannedItem {
    pub item_type: String,
    pub category_id: Option<String>,
    pub label: String,
    pub amount_minor: i64,
}

#[derive(Serialize)]
pub struct BudgetTemplate {
    pub id: String,
    pub name: String,
    pub currency_code: String,
}

#[derive(Serialize)]
pub struct BudgetPeriod {
    pub id: String,
    pub name: String,
    pub currency_code: String,
    pub starts_on: String,
    pub ends_on: String,
}

#[derive(Serialize)]
pub struct BudgetLine {
    pub id: String,
    pub item_type: String,
    pub category_id: Option<String>,
    pub label: String,
    pub planned_minor: i64,
    pub actual_minor: i64,
}

#[derive(Serialize)]
pub struct BudgetTotals {
    pub actual_income_minor: i64,
    pub actual_expense_minor: i64,
    pub actual_savings_minor: i64,
    pub unallocated_minor: i64,
}

#[derive(Serialize)]
pub struct SavingsGoal {
    pub id: String,
    pub name: String,
    pub target_amount_minor: i64,
    pub contributed_minor: i64,
    pub currency_code: String,
    pub target_date: Option<String>,
    pub status: String,
    pub account_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SavingsContributionRecord {
    pub id: String,
    pub goal_id: String,
    pub goal_name: String,
    pub amount_minor: i64,
    pub currency_code: String,
    pub contribution_date: String,
    pub description: Option<String>,
    pub source: String,
}

#[derive(Serialize)]
pub struct EligibleSavingsTransfer {
    pub transaction_id: String,
    pub amount_minor: i64,
    pub occurred_at: String,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub amount_minor: i64,
    pub currency_code: String,
    pub frequency: String,
    pub next_due_date: String,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub status: String,
    pub payment_count: i64,
    pub paid_minor: i64,
}

#[derive(Serialize)]
pub struct EligibleSubscriptionPayment {
    pub transaction_id: String,
    pub amount_minor: i64,
    pub occurred_at: String,
    pub description: Option<String>,
}

pub struct SqlitePlanningRepository<'a> {
    database: &'a Database,
}

impl<'a> SqlitePlanningRepository<'a> {
    pub fn new(database: &'a Database) -> Self {
        Self { database }
    }

    pub fn list_templates(&self) -> Result<Vec<BudgetTemplate>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut statement = connection
            .prepare(
                "SELECT id,name,currency_code FROM budget_templates ORDER BY name COLLATE NOCASE",
            )
            .map_err(|e| e.to_string())?;
        statement
            .query_map([], |row| {
                Ok(BudgetTemplate {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    currency_code: row.get(2)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn template_items(&self, template_id: &str) -> Result<Vec<PlannedItem>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut statement = connection.prepare("SELECT item_type,category_id,label,amount_minor FROM budget_template_items WHERE template_id=?1 ORDER BY item_type,label COLLATE NOCASE").map_err(|e| e.to_string())?;
        statement
            .query_map([template_id], |r| {
                Ok(PlannedItem {
                    item_type: r.get(0)?,
                    category_id: r.get(1)?,
                    label: r.get(2)?,
                    amount_minor: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn create_template(
        &self,
        name: &str,
        currency: &str,
        items: &[PlannedItem],
    ) -> Result<String, String> {
        let name = name.trim();
        let currency = currency.trim().to_ascii_uppercase();
        if name.is_empty() || currency != "KES" {
            return Err("Budget templates in the initial release use KES.".into());
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        validate_items(&connection, items)?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO budget_templates(id,name,currency_code) VALUES (?1,?2,?3)",
            params![id, name, currency],
        )
        .map_err(|e| e.to_string())?;
        insert_items(&tx, "budget_template_items", "template_id", &id, items)?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn update_template(
        &self,
        template_id: &str,
        name: &str,
        items: &[PlannedItem],
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Template name is required.".into());
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        validate_items(&connection, items)?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let changed = tx
            .execute(
                "UPDATE budget_templates SET name=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                params![name, template_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("The budget template no longer exists.".into());
        }
        tx.execute(
            "DELETE FROM budget_template_items WHERE template_id=?1",
            [template_id],
        )
        .map_err(|e| e.to_string())?;
        insert_items(
            &tx,
            "budget_template_items",
            "template_id",
            template_id,
            items,
        )?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn create_period(&self, template_id: &str, month: &str) -> Result<String, String> {
        let (year, month_num) = month.split_once('-').ok_or("Choose a valid month.")?;
        if year.len() != 4 || month_num.len() != 2 {
            return Err("Choose a valid month.".into());
        }
        let year_num: i32 = year.parse().map_err(|_| "Choose a valid month.")?;
        let month_num: u32 = month_num.parse().map_err(|_| "Choose a valid month.")?;
        if !(1..=12).contains(&month_num) || year_num < 1 {
            return Err("Choose a valid month.".into());
        }
        let starts = format!("{month}-01");
        let days = chrono::NaiveDate::from_ymd_opt(year_num, month_num, 1)
            .ok_or("Choose a valid month.")?
            .with_day(1)
            .and_then(|d| d.checked_add_months(chrono::Months::new(1)))
            .and_then(|d| d.pred_opt())
            .ok_or("Choose a valid month.")?
            .day();
        let ends = format!("{month}-{days:02}");
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let (name, currency): (String, String) = tx
            .query_row(
                "SELECT name,currency_code FROM budget_templates WHERE id=?1",
                [template_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The selected budget template no longer exists.")?;
        let id = Uuid::new_v4().to_string();
        tx.execute("INSERT INTO budget_periods(id,template_id,name,currency_code,starts_on,ends_on) VALUES (?1,?2,?3,?4,?5,?6)", params![id,template_id,format!("{name} — {month}"),currency,starts,ends]).map_err(|e| if e.to_string().contains("UNIQUE") { "A budget already exists for this month.".into() } else { e.to_string() })?;
        tx.execute("INSERT INTO budget_period_items(id,budget_id,item_type,category_id,label,amount_minor) SELECT lower(hex(randomblob(16))),?1,item_type,category_id,label,amount_minor FROM budget_template_items WHERE template_id=?2", params![id,template_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn list_periods(&self) -> Result<Vec<BudgetPeriod>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut statement = connection.prepare("SELECT id,name,currency_code,starts_on,ends_on FROM budget_periods ORDER BY starts_on DESC").map_err(|e| e.to_string())?;
        statement
            .query_map([], |row| {
                Ok(BudgetPeriod {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    currency_code: row.get(2)?,
                    starts_on: row.get(3)?,
                    ends_on: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn period_items(&self, budget_id: &str) -> Result<Vec<BudgetLine>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let (starts, ends, currency): (String, String, String) = connection
            .query_row(
                "SELECT starts_on,ends_on,currency_code FROM budget_periods WHERE id=?1",
                [budget_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The selected budget no longer exists.")?;
        let mut statement = connection.prepare("SELECT i.id,i.item_type,i.category_id,i.label,i.amount_minor,CASE WHEN i.item_type IN ('expense','income') AND i.category_id IS NOT NULL THEN COALESCE((SELECT SUM(ca.amount_minor) FROM category_allocations ca JOIN financial_transactions t ON t.id=ca.transaction_id WHERE ca.category_id=i.category_id AND t.transaction_type=i.item_type AND t.currency_code=?4 AND t.status='posted' AND date(t.occurred_at) BETWEEN ?2 AND ?3 AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted')),0) ELSE 0 END FROM budget_period_items i WHERE i.budget_id=?1 ORDER BY i.item_type,i.label COLLATE NOCASE").map_err(|e| e.to_string())?;
        statement
            .query_map(params![budget_id, starts, ends, currency], |r| {
                Ok(BudgetLine {
                    id: r.get(0)?,
                    item_type: r.get(1)?,
                    category_id: r.get(2)?,
                    label: r.get(3)?,
                    planned_minor: r.get(4)?,
                    actual_minor: r.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn period_totals(&self, budget_id: &str) -> Result<BudgetTotals, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let (starts, ends, currency): (String, String, String) = connection
            .query_row(
                "SELECT starts_on,ends_on,currency_code FROM budget_periods WHERE id=?1",
                [budget_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The selected budget no longer exists.")?;
        let (income, expense, savings): (i64, i64, i64) = connection.query_row("SELECT COALESCE(SUM(CASE WHEN transaction_type='income' THEN amount_minor ELSE 0 END),0),COALESCE(SUM(CASE WHEN transaction_type='expense' THEN amount_minor ELSE 0 END),0),COALESCE((SELECT SUM(c.amount_minor) FROM savings_contributions c JOIN savings_goals g ON g.id=c.goal_id AND g.currency_code=?3 WHERE c.contribution_date BETWEEN ?1 AND ?2 AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions linked WHERE linked.id=c.transaction_id AND linked.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=linked.id AND reversal.status='posted')))),0) FROM financial_transactions WHERE status='posted' AND currency_code=?3 AND date(occurred_at) BETWEEN ?1 AND ?2 AND transaction_type IN ('income','expense') AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=financial_transactions.id AND reversal.status='posted')", params![starts,ends,currency], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(|e| e.to_string())?;
        Ok(BudgetTotals {
            actual_income_minor: income,
            actual_expense_minor: expense,
            actual_savings_minor: savings,
            unallocated_minor: income - expense - savings,
        })
    }

    pub fn update_period_items(
        &self,
        budget_id: &str,
        items: &[PlannedItem],
    ) -> Result<(), String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        validate_items(&connection, items)?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM budget_periods WHERE id=?1 AND currency_code='KES')",
                [budget_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if !exists {
            return Err("The budget period no longer exists.".into());
        }
        tx.execute(
            "DELETE FROM budget_period_items WHERE budget_id=?1",
            [budget_id],
        )
        .map_err(|e| e.to_string())?;
        insert_items(&tx, "budget_period_items", "budget_id", budget_id, items)?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn list_savings_goals(&self) -> Result<Vec<SavingsGoal>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut statement = connection.prepare("SELECT g.id,g.name,g.target_amount_minor,COALESCE(SUM(c.amount_minor),0),g.currency_code,g.target_date,g.status,g.account_id FROM savings_goals g LEFT JOIN savings_contributions c ON c.goal_id=g.id AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions t WHERE t.id=c.transaction_id AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted'))) GROUP BY g.id ORDER BY CASE g.status WHEN 'active' THEN 0 WHEN 'completed' THEN 1 ELSE 2 END,g.created_at DESC").map_err(|e| e.to_string())?;
        statement
            .query_map([], |r| {
                Ok(SavingsGoal {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    target_amount_minor: r.get(2)?,
                    contributed_minor: r.get(3)?,
                    currency_code: r.get(4)?,
                    target_date: r.get(5)?,
                    status: r.get(6)?,
                    account_id: r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn list_savings_contributions(
        &self,
        goal_id: Option<&str>,
    ) -> Result<Vec<SavingsContributionRecord>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut statement = connection.prepare("SELECT c.id,c.goal_id,g.name,c.amount_minor,g.currency_code,c.contribution_date,c.description,CASE WHEN c.transaction_id IS NULL THEN 'manual' ELSE 'linked transfer' END FROM savings_contributions c JOIN savings_goals g ON g.id=c.goal_id WHERE (?1 IS NULL OR c.goal_id=?1) AND (c.transaction_id IS NULL OR EXISTS(SELECT 1 FROM financial_transactions t WHERE t.id=c.transaction_id AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted'))) ORDER BY date(c.contribution_date) DESC,c.created_at DESC LIMIT 1000").map_err(|e| e.to_string())?;
        statement
            .query_map([goal_id], |r| {
                Ok(SavingsContributionRecord {
                    id: r.get(0)?,
                    goal_id: r.get(1)?,
                    goal_name: r.get(2)?,
                    amount_minor: r.get(3)?,
                    currency_code: r.get(4)?,
                    contribution_date: r.get(5)?,
                    description: r.get(6)?,
                    source: r.get(7)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn create_savings_goal(
        &self,
        name: &str,
        target_minor: i64,
        currency: &str,
        target_date: Option<&str>,
        description: Option<&str>,
        account_id: Option<&str>,
    ) -> Result<String, String> {
        let name = name.trim();
        let currency = currency.trim().to_ascii_uppercase();
        if name.is_empty() || target_minor <= 0 || currency != "KES" {
            return Err("Enter a goal name, positive target and KES currency.".into());
        }
        if let Some(date) = target_date {
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| "Enter a valid target date.")?;
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        if let Some(account_id) = account_id {
            let valid: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL AND currency_code=?2)", params![account_id,currency], |r| r.get(0)).map_err(|e| e.to_string())?;
            if !valid {
                return Err("Choose an active savings account using the goal currency.".into());
            }
        }
        let id = Uuid::new_v4().to_string();
        connection.execute("INSERT INTO savings_goals(id,name,description,target_amount_minor,currency_code,target_date,account_id) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![id,name,description.map(str::trim).filter(|v| !v.is_empty()),target_minor,currency,target_date,account_id]).map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn eligible_savings_transfers(
        &self,
        goal_id: &str,
    ) -> Result<Vec<EligibleSavingsTransfer>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let account_id: Option<String> = connection
            .query_row(
                "SELECT account_id FROM savings_goals WHERE id=?1 AND status='active'",
                [goal_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("Choose an active savings goal.")?;
        let Some(account_id) = account_id else {
            return Ok(Vec::new());
        };
        let mut statement = connection.prepare("SELECT t.id,e.amount_minor,t.occurred_at,t.description FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.transaction_type='transfer' AND t.status='posted' AND e.account_id=?1 AND e.amount_minor>0 AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted') AND NOT EXISTS(SELECT 1 FROM savings_contributions c WHERE c.transaction_id=t.id) ORDER BY t.occurred_at DESC").map_err(|e| e.to_string())?;
        statement
            .query_map([account_id], |r| {
                Ok(EligibleSavingsTransfer {
                    transaction_id: r.get(0)?,
                    amount_minor: r.get(1)?,
                    occurred_at: r.get(2)?,
                    description: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn add_savings_transfer(&self, goal_id: &str, transaction_id: &str) -> Result<(), String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let (currency, account_id, status): (String, Option<String>, String) = tx
            .query_row(
                "SELECT currency_code,account_id,status FROM savings_goals WHERE id=?1",
                [goal_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The savings goal no longer exists.")?;
        if status != "active" {
            return Err("Choose an active savings goal.".into());
        }
        let account_id =
            account_id.ok_or("Choose a linked savings account before recording a transfer.")?;
        let (amount, date): (i64, String) = tx.query_row("SELECT e.amount_minor,date(t.occurred_at) FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN accounts a ON a.id=e.account_id WHERE t.id=?1 AND t.transaction_type='transfer' AND t.status='posted' AND e.account_id=?2 AND e.amount_minor>0 AND t.currency_code=?3 AND a.currency_code=?3 AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted') AND NOT EXISTS(SELECT 1 FROM savings_contributions c WHERE c.transaction_id=t.id)", params![transaction_id,account_id,currency], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e| e.to_string())?.ok_or("That transfer is not an eligible incoming transfer to this savings account.")?;
        tx.execute("INSERT INTO savings_contributions(id,goal_id,amount_minor,contribution_date,description,transaction_id) VALUES (?1,?2,?3,?4,'Linked incoming account transfer',?5)", params![Uuid::new_v4().to_string(),goal_id,amount,date,transaction_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn add_savings_contribution(
        &self,
        goal_id: &str,
        amount_minor: i64,
        date: &str,
        description: Option<&str>,
    ) -> Result<(), String> {
        if amount_minor <= 0 {
            return Err("Savings contribution must be positive.".into());
        }
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| "Enter a valid contribution date.")?;
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let goal: Option<(String, Option<String>)> = connection
            .query_row(
                "SELECT status,account_id FROM savings_goals WHERE id=?1",
                [goal_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let Some((status, account_id)) = goal else {
            return Err("Choose an active savings goal.".into());
        };
        if status != "active" {
            return Err("Choose an active savings goal.".into());
        }
        if let Some(account_id) = account_id {
            let duplicate_transfer: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id WHERE t.transaction_type='transfer' AND t.status='posted' AND date(t.occurred_at)=?1 AND e.account_id=?2 AND e.amount_minor=?3 AND e.amount_minor>0 AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted'))", params![date,account_id,amount_minor], |r| r.get(0)).map_err(|e| e.to_string())?;
            if duplicate_transfer {
                return Err("An incoming transfer matches this amount and date. Link it instead of adding it twice.".into());
            }
        }
        connection.execute("INSERT INTO savings_contributions(id,goal_id,amount_minor,contribution_date,description) VALUES (?1,?2,?3,?4,?5)", params![Uuid::new_v4().to_string(),goal_id,amount_minor,date,description.map(str::trim).filter(|v| !v.is_empty())]).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn set_savings_goal_status(&self, goal_id: &str, status: &str) -> Result<(), String> {
        if !["active", "completed", "archived"].contains(&status) {
            return Err("Choose a valid savings goal status.".into());
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let changed = connection
            .execute(
                "UPDATE savings_goals SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
                params![status, goal_id],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("The savings goal no longer exists.".into());
        }
        Ok(())
    }

    pub fn list_subscriptions(&self) -> Result<Vec<Subscription>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let mut statement = connection.prepare("SELECT s.id,s.name,s.description,s.amount_minor,s.currency_code,s.frequency,s.next_due_date,s.account_id,s.category_id,s.status,COUNT(t.id),COALESCE(SUM(t.amount_minor),0) FROM subscriptions s LEFT JOIN subscription_payments p ON p.subscription_id=s.id LEFT JOIN financial_transactions t ON t.id=p.transaction_id AND t.status='posted' AND NOT EXISTS(SELECT 1 FROM financial_transactions r WHERE r.reversal_of_transaction_id=t.id AND r.status='posted') GROUP BY s.id ORDER BY CASE s.status WHEN 'active' THEN 0 WHEN 'paused' THEN 1 ELSE 2 END,s.next_due_date,s.name COLLATE NOCASE").map_err(|e| e.to_string())?;
        statement
            .query_map([], |r| {
                Ok(Subscription {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    description: r.get(2)?,
                    amount_minor: r.get(3)?,
                    currency_code: r.get(4)?,
                    frequency: r.get(5)?,
                    next_due_date: r.get(6)?,
                    account_id: r.get(7)?,
                    category_id: r.get(8)?,
                    status: r.get(9)?,
                    payment_count: r.get(10)?,
                    paid_minor: r.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn eligible_subscription_payments(
        &self,
        subscription_id: &str,
    ) -> Result<Vec<EligibleSubscriptionPayment>, String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let (currency, account, category): (String, Option<String>, Option<String>) = connection
            .query_row(
                "SELECT currency_code,account_id,category_id FROM subscriptions WHERE id=?1",
                [subscription_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The subscription no longer exists.")?;
        let mut statement = connection.prepare("SELECT t.id,t.amount_minor,t.occurred_at,t.description FROM financial_transactions t WHERE t.transaction_type='expense' AND t.status='posted' AND t.currency_code=?1 AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted') AND NOT EXISTS(SELECT 1 FROM subscription_payments p WHERE p.transaction_id=t.id) AND (?2 IS NULL OR EXISTS(SELECT 1 FROM account_entries e WHERE e.transaction_id=t.id AND e.account_id=?2 AND e.amount_minor<0)) AND (?3 IS NULL OR EXISTS(SELECT 1 FROM category_allocations a WHERE a.transaction_id=t.id AND a.category_id=?3)) ORDER BY t.occurred_at DESC").map_err(|e| e.to_string())?;
        statement
            .query_map(params![currency, account, category], |r| {
                Ok(EligibleSubscriptionPayment {
                    transaction_id: r.get(0)?,
                    amount_minor: r.get(1)?,
                    occurred_at: r.get(2)?,
                    description: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
    }

    pub fn link_subscription_payment(
        &self,
        subscription_id: &str,
        transaction_id: &str,
    ) -> Result<(), String> {
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let (currency, account, category): (String, Option<String>, Option<String>) = tx
            .query_row(
                "SELECT currency_code,account_id,category_id FROM subscriptions WHERE id=?1",
                [subscription_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The subscription no longer exists.")?;
        let valid: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM financial_transactions t WHERE t.id=?1 AND t.transaction_type='expense' AND t.status='posted' AND t.currency_code=?2 AND NOT EXISTS(SELECT 1 FROM financial_transactions reversal WHERE reversal.reversal_of_transaction_id=t.id AND reversal.status='posted') AND (?3 IS NULL OR EXISTS(SELECT 1 FROM account_entries e WHERE e.transaction_id=t.id AND e.account_id=?3 AND e.amount_minor<0)) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM category_allocations a WHERE a.transaction_id=t.id AND a.category_id=?4)) AND NOT EXISTS(SELECT 1 FROM subscription_payments p WHERE p.transaction_id=t.id))", params![transaction_id,currency,account,category], |r| r.get(0)).map_err(|e| e.to_string())?;
        if !valid {
            return Err(
                "Choose a posted expense matching this subscription and not linked elsewhere."
                    .into(),
            );
        }
        tx.execute("INSERT INTO subscription_payments(id,subscription_id,transaction_id) VALUES (?1,?2,?3)", params![Uuid::new_v4().to_string(),subscription_id,transaction_id]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn create_subscription(&self, item: &SubscriptionInput) -> Result<String, String> {
        validate_subscription(item)?;
        if item.currency_code != "KES" {
            return Err("New subscriptions in the initial release use KES.".into());
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        if let Some(account_id) = &item.account_id {
            let valid: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL AND currency_code=?2)", params![account_id,item.currency_code], |r| r.get(0)).map_err(|e| e.to_string())?;
            if !valid {
                return Err(
                    "Select an active payment account using the subscription currency.".into(),
                );
            }
        }
        if let Some(category_id) = &item.category_id {
            let valid: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND category_type='expense' AND archived_at IS NULL)", [category_id], |r| r.get(0)).map_err(|e| e.to_string())?;
            if !valid {
                return Err("Select an active expense category.".into());
            }
        }
        let id = Uuid::new_v4().to_string();
        connection.execute("INSERT INTO subscriptions(id,name,description,amount_minor,currency_code,frequency,next_due_date,account_id,category_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![id,item.name.trim(),item.description.as_deref().map(str::trim).filter(|v| !v.is_empty()),item.amount_minor,item.currency_code,item.frequency,item.next_due_date,item.account_id,item.category_id]).map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn update_subscription(&self, id: &str, item: &SubscriptionInput) -> Result<(), String> {
        validate_subscription(item)?;
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        if let Some(account_id) = &item.account_id {
            let valid: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM accounts WHERE id=?1 AND archived_at IS NULL AND currency_code=?2)", params![account_id,item.currency_code], |r| r.get(0)).map_err(|e| e.to_string())?;
            if !valid {
                return Err(
                    "Select an active payment account using the subscription currency.".into(),
                );
            }
        }
        if let Some(category_id) = &item.category_id {
            let valid: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND category_type='expense' AND archived_at IS NULL)", [category_id], |r| r.get(0)).map_err(|e| e.to_string())?;
            if !valid {
                return Err("Select an active expense category.".into());
            }
        }
        let (previous,old_currency):(String,String) = connection.query_row("SELECT json_object('name',name,'amount_minor',amount_minor,'currency_code',currency_code,'frequency',frequency,'next_due_date',next_due_date,'account_id',account_id,'category_id',category_id,'status',status),currency_code FROM subscriptions WHERE id=?1", [id], |r| Ok((r.get(0)?,r.get(1)?))).optional().map_err(|e| e.to_string())?.ok_or("The subscription no longer exists.")?;
        if item.currency_code != old_currency {
            return Err("A subscription's currency cannot be changed.".into());
        }
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute("UPDATE subscriptions SET name=?1,description=?2,amount_minor=?3,currency_code=?4,frequency=?5,next_due_date=?6,account_id=?7,category_id=?8,updated_at=CURRENT_TIMESTAMP WHERE id=?9", params![item.name.trim(),item.description.as_deref().map(str::trim).filter(|v| !v.is_empty()),item.amount_minor,item.currency_code,item.frequency,item.next_due_date,item.account_id,item.category_id,id]).map_err(|e| e.to_string())?;
        let current = serde_json::json!({"name":item.name.trim(),"description":item.description,"amount_minor":item.amount_minor,"currency_code":item.currency_code,"frequency":item.frequency,"next_due_date":item.next_due_date,"account_id":item.account_id,"category_id":item.category_id});
        tx.execute("INSERT INTO subscription_history(id,subscription_id,previous_values,new_values) VALUES (?1,?2,?3,?4)", params![Uuid::new_v4().to_string(),id,previous,current.to_string()]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }

    pub fn set_subscription_status(&self, id: &str, status: &str) -> Result<(), String> {
        if !["active", "paused", "cancelled"].contains(&status) {
            return Err("Choose active, paused or cancelled.".into());
        }
        let connection = self
            .database
            .connection()
            .lock()
            .map_err(|_| "Database is unavailable")?;
        let previous: String = connection
            .query_row(
                "SELECT json_object('status',status) FROM subscriptions WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The subscription no longer exists.")?;
        let tx = connection
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE subscriptions SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            params![status, id],
        )
        .map_err(|e| e.to_string())?;
        tx.execute("INSERT INTO subscription_history(id,subscription_id,previous_values,new_values) VALUES (?1,?2,?3,?4)", params![Uuid::new_v4().to_string(),id,previous,serde_json::json!({"status":status}).to_string()]).map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
}

#[derive(Clone, serde::Deserialize)]
pub struct SubscriptionInput {
    pub name: String,
    pub description: Option<String>,
    pub amount_minor: i64,
    pub currency_code: String,
    pub frequency: String,
    pub next_due_date: String,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
}

fn validate_subscription(item: &SubscriptionInput) -> Result<(), String> {
    if item.name.trim().is_empty() || item.amount_minor <= 0 {
        return Err("Enter a name and a positive subscription amount.".into());
    }
    if item.currency_code.len() != 3 || !item.currency_code.chars().all(|c| c.is_ascii_uppercase())
    {
        return Err("Enter a three-letter currency code.".into());
    }
    if !["weekly", "monthly", "quarterly", "yearly"].contains(&item.frequency.as_str()) {
        return Err("Choose a supported billing frequency.".into());
    }
    chrono::NaiveDate::parse_from_str(&item.next_due_date, "%Y-%m-%d")
        .map_err(|_| "Enter a valid next due date.")?;
    Ok(())
}

fn validate_items(connection: &rusqlite::Connection, items: &[PlannedItem]) -> Result<(), String> {
    if items.is_empty() {
        return Err("Add at least one planned item.".into());
    }
    for item in items {
        let expected = match item.item_type.as_str() {
            "income" => Some("income"),
            "expense" => Some("expense"),
            "savings" => None,
            _ => return Err("Choose income, expense or savings for each item.".into()),
        };
        if item.amount_minor <= 0 || item.label.trim().is_empty() {
            return Err("Each planned item needs a label and a positive amount.".into());
        }
        match (&item.category_id, expected) {
            (Some(id), Some(kind)) => {
                let valid: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM categories WHERE id=?1 AND category_type=?2 AND archived_at IS NULL)", params![id,kind], |r| r.get(0)).map_err(|e| e.to_string())?;
                if !valid {
                    return Err("Select an active category matching the planned item type.".into());
                }
            }
            (Some(_), None) => {
                return Err("Savings plans do not use transaction categories.".into());
            }
            (None, Some(_)) => return Err("Income and expense plans require a category.".into()),
            (None, None) => {}
        }
    }
    Ok(())
}

fn insert_items(
    tx: &rusqlite::Transaction<'_>,
    table: &str,
    foreign_key: &str,
    id: &str,
    items: &[PlannedItem],
) -> Result<(), String> {
    let sql = format!(
        "INSERT INTO {table}(id,{foreign_key},item_type,category_id,label,amount_minor) VALUES (?1,?2,?3,?4,?5,?6)"
    );
    for item in items {
        tx.execute(
            &sql,
            params![
                Uuid::new_v4().to_string(),
                id,
                item.item_type,
                item.category_id,
                item.label.trim(),
                item.amount_minor
            ],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn database() -> Database {
        Database::initialize(Path::new(":memory:")).unwrap()
    }

    #[test]
    fn new_planning_records_reject_non_kes_while_legacy_currency_is_preserved() {
        let database = database();
        let repository = SqlitePlanningRepository::new(&database);
        let item = PlannedItem {
            item_type: "expense".into(),
            category_id: None,
            label: "Test".into(),
            amount_minor: 100,
        };
        assert!(
            repository
                .create_template("Foreign plan", "USD", &[item])
                .is_err()
        );
        assert!(
            repository
                .create_savings_goal("Foreign goal", 100, "USD", None, None, None)
                .is_err()
        );
        let subscription = SubscriptionInput {
            name: "Foreign sub".into(),
            description: None,
            amount_minor: 100,
            currency_code: "USD".into(),
            frequency: "monthly".into(),
            next_due_date: "2026-12-01".into(),
            account_id: None,
            category_id: None,
        };
        assert!(repository.create_subscription(&subscription).is_err());
        database.connection().lock().unwrap().execute("INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('legacy','Legacy USD',100,'USD','monthly','2026-12-01')",[]).unwrap();
        assert_eq!(
            repository.list_subscriptions().unwrap()[0].currency_code,
            "USD"
        );
    }

    #[test]
    fn new_planning_records_reject_non_kes_but_do_not_rewrite_existing_data() {
        let database = database();
        let repository = SqlitePlanningRepository::new(&database);
        assert!(
            repository
                .create_template(
                    "Foreign plan",
                    "USD",
                    &[PlannedItem {
                        item_type: "expense".into(),
                        category_id: None,
                        label: "Test".into(),
                        amount_minor: 100
                    }]
                )
                .is_err()
        );
        assert!(
            repository
                .create_savings_goal("Foreign goal", 100, "USD", None, None, None)
                .is_err()
        );
        let subscription = SubscriptionInput {
            name: "Foreign sub".into(),
            description: None,
            amount_minor: 100,
            currency_code: "USD".into(),
            frequency: "monthly".into(),
            next_due_date: "2026-12-01".into(),
            account_id: None,
            category_id: None,
        };
        assert!(repository.create_subscription(&subscription).is_err());
        database.connection().lock().unwrap().execute("INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('legacy','Legacy USD',100,'USD','monthly','2026-12-01')",[]).unwrap();
        assert_eq!(
            repository.list_subscriptions().unwrap()[0].currency_code,
            "USD"
        );
    }

    #[test]
    fn budget_instance_copies_template_and_remains_independent() {
        let database = database();
        database
            .connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense')",
                [],
            )
            .unwrap();
        let repository = SqlitePlanningRepository::new(&database);
        let template_id = repository
            .create_template(
                "Household",
                "KES",
                &[PlannedItem {
                    item_type: "expense".into(),
                    category_id: Some("food".into()),
                    label: "Food".into(),
                    amount_minor: 25000,
                }],
            )
            .unwrap();
        let budget_id = repository.create_period(&template_id, "2026-02").unwrap();
        repository
            .update_template(
                &template_id,
                "Updated household",
                &[PlannedItem {
                    item_type: "expense".into(),
                    category_id: Some("food".into()),
                    label: "Food".into(),
                    amount_minor: 50_000,
                }],
            )
            .unwrap();
        let lines = repository.period_items(&budget_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].planned_minor, 25000);
        assert_eq!(lines[0].actual_minor, 0);
        let next_period = repository.create_period(&template_id, "2026-03").unwrap();
        assert_eq!(
            repository.period_items(&next_period).unwrap()[0].planned_minor,
            50_000
        );
        repository
            .update_period_items(
                &budget_id,
                &[PlannedItem {
                    item_type: "expense".into(),
                    category_id: Some("food".into()),
                    label: "Food".into(),
                    amount_minor: 30_000,
                }],
            )
            .unwrap();
        assert_eq!(
            repository.period_items(&budget_id).unwrap()[0].planned_minor,
            30_000
        );
        assert_eq!(
            repository.period_items(&next_period).unwrap()[0].planned_minor,
            50_000
        );
        assert!(repository.create_period(&template_id, "2026-02").is_err());
        assert!(repository.create_period(&template_id, "2026-13").is_err());
    }

    #[test]
    fn rejects_inactive_or_mismatched_planned_categories() {
        let database = database();
        database.connection().lock().unwrap().execute("INSERT INTO categories(id,name,category_type,archived_at) VALUES ('food','Food','expense',CURRENT_TIMESTAMP)", []).unwrap();
        let repository = SqlitePlanningRepository::new(&database);
        let result = repository.create_template(
            "Household",
            "KES",
            &[PlannedItem {
                item_type: "expense".into(),
                category_id: Some("food".into()),
                label: "Food".into(),
                amount_minor: 25000,
            }],
        );
        assert!(result.is_err());
        let count: i64 = database
            .connection()
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM budget_templates", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn actual_budget_spending_uses_only_posted_transactions_in_the_period() {
        let database = database();
        database
            .connection()
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense')",
                [],
            )
            .unwrap();
        let template_id = SqlitePlanningRepository::new(&database)
            .create_template(
                "Household",
                "KES",
                &[PlannedItem {
                    item_type: "expense".into(),
                    category_id: Some("food".into()),
                    label: "Food".into(),
                    amount_minor: 25000,
                }],
            )
            .unwrap();
        let repository = SqlitePlanningRepository::new(&database);
        let budget_id = repository.create_period(&template_id, "2026-02").unwrap();
        let connection = database.connection().lock().unwrap();
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('posted','expense',8000,'KES','2026-02-10T12:00:00Z','manual'),('voided','expense',3000,'KES','2026-02-12T12:00:00Z','manual'),('outside','expense',5000,'KES','2026-03-01T12:00:00Z','manual'),('income','income',20000,'KES','2026-02-15T12:00:00Z','manual')", []).unwrap();
        connection.execute("UPDATE financial_transactions SET status='voided',voided_at=CURRENT_TIMESTAMP WHERE id='voided'", []).unwrap();
        connection.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES ('a1','posted','food',8000),('a2','voided','food',3000),('a3','outside','food',5000)", []).unwrap();
        drop(connection);
        assert_eq!(
            repository.period_items(&budget_id).unwrap()[0].actual_minor,
            8000
        );
        let goal = repository
            .create_savings_goal("Reserve", 50_000, "KES", None, None, None)
            .unwrap();
        repository
            .add_savings_contribution(&goal, 3_000, "2026-02-20", None)
            .unwrap();
        let totals = repository.period_totals(&budget_id).unwrap();
        assert_eq!(
            (
                totals.actual_income_minor,
                totals.actual_expense_minor,
                totals.actual_savings_minor,
                totals.unallocated_minor
            ),
            (20_000, 8_000, 3_000, 9_000)
        );
    }

    #[test]
    fn savings_contributions_are_tracked_without_ledger_movement() {
        let database = database();
        let repository = SqlitePlanningRepository::new(&database);
        let goal_id = repository
            .create_savings_goal(
                "Emergency fund",
                100_000,
                "KES",
                Some("2027-12-31"),
                None,
                None,
            )
            .unwrap();
        repository
            .add_savings_contribution(&goal_id, 12_500, "2026-11-06", Some("Monthly saving"))
            .unwrap();
        let goals = repository.list_savings_goals().unwrap();
        assert_eq!(goals[0].contributed_minor, 12_500);
        repository
            .set_savings_goal_status(&goal_id, "completed")
            .unwrap();
        assert!(
            repository
                .add_savings_contribution(&goal_id, 100, "2026-11-07", None)
                .is_err()
        );
        let connection = database.connection().lock().unwrap();
        let transactions: i64 = connection
            .query_row("SELECT COUNT(*) FROM financial_transactions", [], |r| {
                r.get(0)
            })
            .unwrap();
        let entries: i64 = connection
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!((transactions, entries), (0, 0));
    }

    #[test]
    fn qualifying_savings_transfer_is_linked_once_without_duplicate_ledger_entries() {
        let database = database();
        database.connection().lock().unwrap().execute("INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('checking','Checking','cash','KES'),('savings','Savings','cash','KES')", []).unwrap();
        database.connection().lock().unwrap().execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('transfer','transfer',5000,'KES','2026-11-06T12:00:00Z','manual')", []).unwrap();
        database.connection().lock().unwrap().execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('out','transfer','checking',-5000),('in','transfer','savings',5000)", []).unwrap();
        let repository = SqlitePlanningRepository::new(&database);
        let goal = repository
            .create_savings_goal("Reserve", 100_000, "KES", None, None, Some("savings"))
            .unwrap();
        assert_eq!(
            repository.eligible_savings_transfers(&goal).unwrap().len(),
            1
        );
        repository.add_savings_transfer(&goal, "transfer").unwrap();
        assert!(
            repository
                .eligible_savings_transfers(&goal)
                .unwrap()
                .is_empty()
        );
        assert!(
            repository
                .add_savings_contribution(&goal, 5000, "2026-11-06", None)
                .is_err()
        );
        assert_eq!(
            repository.list_savings_goals().unwrap()[0].contributed_minor,
            5000
        );
        let connection = database.connection().lock().unwrap();
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type,reversal_of_transaction_id) VALUES ('reversal','reversal',5000,'KES','2026-11-07T12:00:00Z','reversal','transfer')", []).unwrap();
        connection.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('reverse-out','reversal','checking',5000),('reverse-in','reversal','savings',-5000)", []).unwrap();
        drop(connection);
        assert_eq!(
            repository.list_savings_goals().unwrap()[0].contributed_minor,
            0
        );
        let connection = database.connection().lock().unwrap();
        let entries: i64 = connection
            .query_row("SELECT COUNT(*) FROM account_entries", [], |r| r.get(0))
            .unwrap();
        assert_eq!(entries, 4);
    }

    #[test]
    fn subscription_changes_are_historical_and_never_create_actual_payments() {
        let database = database();
        let repository = SqlitePlanningRepository::new(&database);
        let subscription = SubscriptionInput {
            name: "Music".into(),
            description: None,
            amount_minor: 1_000,
            currency_code: "KES".into(),
            frequency: "monthly".into(),
            next_due_date: "2026-11-20".into(),
            account_id: None,
            category_id: None,
        };
        let id = repository.create_subscription(&subscription).unwrap();
        database.connection().lock().unwrap().execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('payment','expense',1600,'KES','2026-11-20T08:00:00Z','manual')", []).unwrap();
        database.connection().lock().unwrap().execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type,status,voided_at) VALUES ('wrong-currency','expense',1600,'USD','2026-11-20T08:00:00Z','manual','posted',NULL),('voided-payment','expense',1600,'KES','2026-11-20T08:00:00Z','manual','voided',CURRENT_TIMESTAMP)", []).unwrap();
        assert_eq!(
            repository
                .eligible_subscription_payments(&id)
                .unwrap()
                .len(),
            1
        );
        assert!(
            repository
                .link_subscription_payment(&id, "wrong-currency")
                .is_err()
        );
        assert!(
            repository
                .link_subscription_payment(&id, "voided-payment")
                .is_err()
        );
        repository
            .link_subscription_payment(&id, "payment")
            .unwrap();
        let listed = repository.list_subscriptions().unwrap();
        assert_eq!((listed[0].payment_count, listed[0].paid_minor), (1, 1600));
        assert!(
            repository
                .link_subscription_payment(&id, "payment")
                .is_err()
        );
        let updated = SubscriptionInput {
            amount_minor: 1_500,
            ..subscription
        };
        repository.update_subscription(&id, &updated).unwrap();
        repository.set_subscription_status(&id, "paused").unwrap();
        assert_eq!(repository.list_subscriptions().unwrap()[0].status, "paused");
        let connection = database.connection().lock().unwrap();
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type,reversal_of_transaction_id) VALUES ('payment-reversal','reversal',1600,'KES','2026-11-21T08:00:00Z','reversal','payment')", []).unwrap();
        drop(connection);
        assert_eq!(repository.list_subscriptions().unwrap()[0].payment_count, 0);
        let connection = database.connection().lock().unwrap();
        let changes: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM subscription_history WHERE subscription_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        let transactions: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM financial_transactions WHERE transaction_type='expense'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let linked: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM subscription_payments WHERE subscription_id=?1",
                [&id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(changes, 2);
        assert_eq!(transactions, 3);
        assert_eq!(linked, 1);
    }
}
