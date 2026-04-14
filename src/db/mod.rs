pub mod relay_ledger;
pub mod relay_rules;

use sqlx::SqlitePool;

pub async fn init(pool: &SqlitePool, local_relay_ws_url: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS relays (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1,
            internal INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        );"#,
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT OR IGNORE INTO relays (url, enabled, internal) VALUES (?1, 1, 1)")
        .bind(local_relay_ws_url)
        .execute(pool)
        .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            pubkey TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            kind INTEGER NOT NULL,
            tags_json TEXT NOT NULL,
            content TEXT NOT NULL,
            sig TEXT NOT NULL
        );"#,
    )
    .execute(pool)
    .await?;

    relay_rules::init(pool).await?;
    relay_ledger::init(pool).await?;

    Ok(())
}
