//! SQLite connection pool and migrations.

use std::time::Duration;

use sqlx::{
    SqlitePool,
    migrate::{MigrateError, Migrator},
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

/// Migrations embedded from `server/migrations`.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Opens (creating if missing) the SQLite database at `path`.
pub async fn connect(path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
}

/// Applies any pending migrations.
pub async fn migrate(pool: &SqlitePool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}
