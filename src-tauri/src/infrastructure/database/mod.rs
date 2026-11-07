mod connection;
mod migrations;

pub use connection::Database;

pub(crate) fn run_migrations(connection: &mut rusqlite::Connection) -> rusqlite::Result<()> {
    migrations::run(connection)
}

pub(crate) const LATEST_SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;
