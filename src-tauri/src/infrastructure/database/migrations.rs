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
