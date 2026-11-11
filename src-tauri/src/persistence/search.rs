use crate::infrastructure::database::Database;
use rusqlite::params;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SearchResult {
    pub kind: String,
    pub id: String,
    pub title: String,
    pub detail: String,
    pub page: String,
}

/// Search only user-visible metadata and cap work/results to keep the local query predictable.
pub fn search(database: &Database, query: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
    let query = query.trim();
    if query.chars().count() < 2 {
        return Ok(Vec::new());
    }
    let query: String = query.chars().take(80).collect();
    let pattern = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
    let limit = limit.clamp(1, 30) as i64;
    let connection = database.connection().lock().map_err(|e| e.to_string())?;
    let mut results = Vec::new();

    let mut statement = connection.prepare(
        "SELECT 'transaction', t.id, COALESCE(NULLIF(t.description,''), t.transaction_type), a.name || ' · ' || t.occurred_at || ' · ' || t.currency_code || ' ' || printf('%.2f', t.amount_minor / 100.0), 'Transactions'
         FROM financial_transactions t JOIN account_entries e ON e.transaction_id=t.id JOIN accounts a ON a.id=e.account_id
         WHERE t.status='posted' AND (t.description LIKE ?1 ESCAPE '\\' OR t.source_reference LIKE ?1 ESCAPE '\\' OR a.name LIKE ?1 ESCAPE '\\')
         GROUP BY t.id ORDER BY t.occurred_at DESC LIMIT ?2",
    ).map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![pattern, limit], |row| {
            Ok(SearchResult {
                kind: row.get(0)?,
                id: row.get(1)?,
                title: row.get(2)?,
                detail: row.get(3)?,
                page: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    drop(statement);

    for (kind, table, page, detail_col) in [
        (
            "account",
            "accounts",
            "Accounts",
            "COALESCE(institution_name, account_type)",
        ),
        ("category", "categories", "Categories", "category_type"),
    ] {
        let sql = format!(
            "SELECT '{kind}', id, name, {detail_col}, '{page}' FROM {table} WHERE archived_at IS NULL AND name LIKE ?1 ESCAPE '\\' ORDER BY name LIMIT ?2"
        );
        let mut statement = connection.prepare(&sql).map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![pattern, limit], |row| {
                Ok(SearchResult {
                    kind: row.get(0)?,
                    id: row.get(1)?,
                    title: row.get(2)?,
                    detail: row.get(3)?,
                    page: row.get(4)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
    }

    let mut statement = connection.prepare("SELECT 'budget', id, name, currency_code, 'Budgeting' FROM budget_templates WHERE name LIKE ?1 ESCAPE '\\' ORDER BY name LIMIT ?2").map_err(|e| e.to_string())?;
    let rows = statement
        .query_map(params![pattern, limit], |row| {
            Ok(SearchResult {
                kind: row.get(0)?,
                id: row.get(1)?,
                title: row.get(2)?,
                detail: row.get(3)?,
                page: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?;
    for row in rows {
        results.push(row.map_err(|e| e.to_string())?);
    }
    results.truncate(limit as usize);
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::search;
    use crate::infrastructure::database::Database;
    use std::{env, fs};
    use uuid::Uuid;

    #[test]
    fn searches_transactions_accounts_categories_and_templates_with_a_bounded_result() {
        let path = env::temp_dir().join(format!("etracker-search-{}.sqlite", Uuid::new_v4()));
        let db = Database::initialize(&path).unwrap();
        let connection = db.connection().lock().unwrap();
        connection
            .execute(
                "INSERT INTO accounts(id,name,account_type) VALUES('a','Market wallet','cash')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES('c','Market food','expense')",
                [],
            )
            .unwrap();
        connection.execute("INSERT INTO budget_templates(id,name,currency_code) VALUES('b','Market month','KES')", []).unwrap();
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,occurred_at,description,source_type) VALUES('t','expense',1200,'2026-01-01','Market groceries','manual')", []).unwrap();
        connection.execute("INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES('e','t','a',-1200)", []).unwrap();
        drop(connection);
        let found = search(&db, "Market", 10).unwrap();
        assert!(found.iter().any(|item| item.kind == "transaction"));
        assert!(found.iter().any(|item| item.kind == "account"));
        assert!(found.iter().any(|item| item.kind == "category"));
        assert!(found.iter().any(|item| item.kind == "budget"));
        assert!(search(&db, "Market", 1).unwrap().len() <= 1);
        assert!(search(&db, "x", 10).unwrap().is_empty());
        drop(db);
        let _ = fs::remove_file(path);
    }
}
