use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// Open the SQLite database, creating it (and any parent dirs) if needed,
/// and run all pending migrations.
pub async fn init(db_path: &str) -> Result<SqlitePool> {
    // Ensure parent directory exists. SQLite won't create it for us.
    if let Some(parent) = Path::new(db_path).parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create database directory {}", parent.display()))?;
            info!(path = %parent.display(), "created database directory");
        }
    }

    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
        .with_context(|| format!("failed to open database at {}", db_path))?;

    info!(path = %db_path, "database connection pool established");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .context("failed to run database migrations")?;

    info!("migrations applied");

    Ok(pool)
}
