use rusqlite::Connection;

struct Migration {
    version: i64,
    sql: &'static str,
}

pub(super) const LATEST_VERSION: i64 = 16;

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../../../migrations/0001_initial_schema.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../../../migrations/0002_import_staging.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("../../../migrations/0003_import_reconciliation.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("../../../migrations/0004_import_category_allocations.sql"),
    },
    Migration {
        version: 5,
        sql: include_str!("../../../migrations/0005_import_source_files.sql"),
    },
    Migration {
        version: 6,
        sql: include_str!("../../../migrations/0006_import_revisions.sql"),
    },
    Migration {
        version: 7,
        sql: include_str!("../../../migrations/0007_financial_planning.sql"),
    },
    Migration {
        version: 8,
        sql: include_str!("../../../migrations/0008_savings_account_links.sql"),
    },
    Migration {
        version: 9,
        sql: include_str!("../../../migrations/0009_subscription_payments.sql"),
    },
    Migration {
        version: 10,
        sql: include_str!("../../../migrations/0010_forecasting.sql"),
    },
    Migration {
        version: 11,
        sql: include_str!("../../../migrations/0011_refund_records.sql"),
    },
    Migration {
        version: 12,
        sql: include_str!("../../../migrations/0012_forecast_schedule.sql"),
    },
    Migration {
        version: 13,
        sql: include_str!("../../../migrations/0013_user_preferences.sql"),
    },
    Migration {
        version: 14,
        sql: include_str!("../../../migrations/0014_theme_preference.sql"),
    },
    Migration {
        version: 15,
        sql: include_str!("../../../migrations/0015_refund_audit_events.sql"),
    },
    Migration {
        version: 16,
        sql: include_str!("../../../migrations/0016_discard_import_batches.sql"),
    },
];

pub(super) fn run(connection: &mut Connection) -> rusqlite::Result<()> {
    let applied_versions = {
        let mut statement = connection.prepare("SELECT version FROM schema_migrations")?;

        let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;

        rows.collect::<rusqlite::Result<Vec<i64>>>()?
    };

    for migration in MIGRATIONS {
        if applied_versions.contains(&migration.version) {
            continue;
        }

        let transaction = connection.transaction()?;

        transaction.execute_batch(migration.sql)?;

        transaction.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [migration.version],
        )?;

        transaction.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MIGRATIONS, run};
    use rusqlite::Connection;

    #[test]
    fn refund_audit_migration_preserves_existing_history_and_accepts_link_events() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);",
            )
            .unwrap();
        for migration in MIGRATIONS.iter().filter(|migration| migration.version < 15) {
            connection.execute_batch(migration.sql).unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    [migration.version],
                )
                .unwrap();
        }

        connection
            .execute(
                "INSERT INTO financial_transactions(id, transaction_type, amount_minor, currency_code, occurred_at, source_type) VALUES ('expense-1', 'expense', 1000, 'KES', '2026-11-01T00:00:00Z', 'manual')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO transaction_audit_log(id, transaction_id, action, new_values) VALUES ('audit-1', 'expense-1', 'created', '{\"type\":\"expense\"}')",
                [],
            )
            .unwrap();

        run(&mut connection).unwrap();

        let preserved: String = connection
            .query_row(
                "SELECT new_values FROM transaction_audit_log WHERE id='audit-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(preserved, "{\"type\":\"expense\"}");
        connection
            .execute(
                "INSERT INTO transaction_audit_log(id, transaction_id, action) VALUES ('audit-2', 'expense-1', 'refund_received')",
                [],
            )
            .unwrap();
    }
}
