use rusqlite::Connection;

struct Migration {
    version: i64,
    sql: &'static str,
}

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
