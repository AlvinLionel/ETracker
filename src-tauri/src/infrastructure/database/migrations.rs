use rusqlite::Connection;

struct Migration {
    version: i64,
    sql: &'static str,
}

pub(super) const LATEST_VERSION: i64 = 21;

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
    Migration {
        version: 17,
        sql: include_str!("../../../migrations/0017_transaction_attachments.sql"),
    },
    Migration {
        version: 18,
        sql: include_str!("../../../migrations/0018_saved_analytics_reports.sql"),
    },
    Migration {
        version: 19,
        sql: include_str!("../../../migrations/0019_flexible_categories_and_subscriptions.sql"),
    },
    Migration {
        version: 20,
        sql: include_str!("../../../migrations/0020_savings_withdrawals.sql"),
    },
    Migration {
        version: 21,
        sql: include_str!("../../../migrations/0021_diagnostic_events.sql"),
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

        let rebuilds_foreign_key_tables = migration.version == 19;
        if rebuilds_foreign_key_tables {
            connection.pragma_update(None, "foreign_keys", "OFF")?;
        }

        let result = (|| {
            let transaction = connection.transaction()?;

            transaction.execute_batch(migration.sql)?;

            transaction.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                [migration.version],
            )?;

            transaction.commit()
        })();

        if rebuilds_foreign_key_tables {
            connection.pragma_update(None, "foreign_keys", "ON")?;
        }
        result?;

        if rebuilds_foreign_key_tables {
            let violations: i64 = connection.query_row(
                "SELECT COUNT(*) FROM pragma_foreign_key_check",
                [],
                |row| row.get(0),
            )?;
            if violations > 0 {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY),
                    Some(format!(
                        "Migration {} left {violations} foreign-key violations",
                        migration.version
                    )),
                ));
            }
        }
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

    #[test]
    fn flexible_category_and_frequency_migration_preserves_references() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        connection.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);").unwrap();
        for migration in MIGRATIONS.iter().filter(|migration| migration.version < 19) {
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
                "INSERT INTO accounts(id,name,account_type) VALUES ('cash','Cash','cash')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO categories(id,name,category_type) VALUES ('food','Food','expense')",
                [],
            )
            .unwrap();
        connection.execute("INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('expense','expense',1000,'KES','2026-11-01T00:00:00Z','manual')", []).unwrap();
        connection.execute("INSERT INTO category_allocations(id,transaction_id,category_id,amount_minor) VALUES ('allocation','expense','food',1000)", []).unwrap();
        connection.execute("INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('subscription','Music',500,'KES','monthly','2026-11-20')", []).unwrap();
        connection.execute("INSERT INTO subscription_history(id,subscription_id,previous_values,new_values) VALUES ('history','subscription','{}','{}')", []).unwrap();

        run(&mut connection).unwrap();

        let preserved: i64 = connection.query_row("SELECT COUNT(*) FROM category_allocations WHERE id='allocation' AND category_id='food'", [], |row| row.get(0)).unwrap();
        assert_eq!(preserved, 1);
        let history: i64 = connection.query_row("SELECT COUNT(*) FROM subscription_history WHERE id='history' AND subscription_id='subscription'", [], |row| row.get(0)).unwrap();
        assert_eq!(history, 1);
        connection.execute("INSERT INTO categories(id,name,category_type) VALUES ('emergency','Emergency','emergency')", []).unwrap();
        connection.execute("INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('custom','Custom',500,'KES','custom:3:months','2026-11-20')", []).unwrap();
        assert!(connection
            .execute(
                "INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('invalid-custom','Invalid',500,'KES','custom:3x:months','2026-11-20')",
                [],
            )
            .is_err());
        assert!(connection
            .execute(
                "INSERT INTO subscriptions(id,name,amount_minor,currency_code,frequency,next_due_date) VALUES ('oversized-custom','Invalid',500,'KES','custom:366:months','2026-11-20')",
                [],
            )
            .is_err());
        let violations: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
    }

    #[test]
    fn savings_and_diagnostics_migrations_preserve_legacy_records_and_enforce_links() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        connection.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);").unwrap();
        for migration in MIGRATIONS.iter().filter(|migration| migration.version < 20) {
            connection.execute_batch(migration.sql).unwrap();
            connection
                .execute(
                    "INSERT INTO schema_migrations(version) VALUES (?1)",
                    [migration.version],
                )
                .unwrap();
        }
        connection.execute(
            "INSERT INTO accounts(id,name,account_type,currency_code) VALUES ('savings','Savings','bank','KES')",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO savings_goals(id,name,target_amount_minor,currency_code,account_id) VALUES ('goal','Reserve',10000,'KES','savings')",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO savings_contributions(id,goal_id,amount_minor,contribution_date) VALUES ('contribution','goal',5000,'2026-11-01')",
            [],
        ).unwrap();

        run(&mut connection).unwrap();

        connection.execute(
            "INSERT INTO financial_transactions(id,transaction_type,amount_minor,currency_code,occurred_at,source_type) VALUES ('withdrawal','expense',1000,'KES','2026-11-02T12:00:00Z','manual')",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO account_entries(id,transaction_id,account_id,amount_minor) VALUES ('withdrawal-entry','withdrawal','savings',-1000)",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO savings_withdrawals(id,goal_id,transaction_id,amount_minor,withdrawal_date) VALUES ('withdrawal-link','goal','withdrawal',1000,'2026-11-02')",
            [],
        ).unwrap();
        connection.execute(
            "INSERT INTO diagnostic_events(id,event_code) VALUES ('startup-event','app_started')",
            [],
        ).unwrap();
        let preserved: i64 = connection
            .query_row(
                "SELECT amount_minor FROM savings_contributions WHERE id='contribution'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let violations: i64 = connection
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(preserved, 5000);
        assert_eq!(violations, 0);
    }
}
