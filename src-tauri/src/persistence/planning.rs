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

    pub fn create_template(
        &self,
        name: &str,
        currency: &str,
        items: &[PlannedItem],
    ) -> Result<String, String> {
        let name = name.trim();
        let currency = currency.trim().to_ascii_uppercase();
        if name.is_empty()
            || currency.len() != 3
            || !currency.chars().all(|c| c.is_ascii_uppercase())
        {
            return Err("Enter a template name and three-letter currency code.".into());
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
        let (starts, ends): (String, String) = connection
            .query_row(
                "SELECT starts_on,ends_on FROM budget_periods WHERE id=?1",
                [budget_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .ok_or("The selected budget no longer exists.")?;
        let mut statement = connection.prepare("SELECT i.id,i.item_type,i.category_id,i.label,i.amount_minor,CASE WHEN i.item_type='expense' AND i.category_id IS NOT NULL THEN COALESCE((SELECT SUM(ca.amount_minor) FROM category_allocations ca JOIN financial_transactions t ON t.id=ca.transaction_id WHERE ca.category_id=i.category_id AND t.transaction_type='expense' AND t.status='posted' AND date(t.occurred_at) BETWEEN ?2 AND ?3),0) ELSE 0 END FROM budget_period_items i WHERE i.budget_id=?1 ORDER BY i.item_type,i.label COLLATE NOCASE").map_err(|e| e.to_string())?;
        statement
            .query_map(params![budget_id, starts, ends], |r| {
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
        database
            .connection()
            .lock()
            .unwrap()
            .execute(
                "UPDATE budget_template_items SET amount_minor=50000 WHERE template_id=?1",
                [&template_id],
            )
            .unwrap();
        let lines = repository.period_items(&budget_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].planned_minor, 25000);
        assert_eq!(lines[0].actual_minor, 0);
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
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('posted','expense',8000,'KES','2026-02-10T12:00:00Z','manual'),('voided','expense',3000,'KES','2026-02-12T12:00:00Z','manual'),('outside','expense',5000,'KES','2026-03-01T12:00:00Z','manual')", []).unwrap();
        connection.execute("UPDATE financial_transactions SET status='voided',voided_at=CURRENT_TIMESTAMP WHERE id='voided'", []).unwrap();
        connection.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES ('a1','posted','food',8000),('a2','voided','food',3000),('a3','outside','food',5000)", []).unwrap();
        drop(connection);
        assert_eq!(
            repository.period_items(&budget_id).unwrap()[0].actual_minor,
            8000
        );
    }
}
