use sqlx::SqlitePool;

pub async fn init(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS relay_export_ledger (
            event_id TEXT PRIMARY KEY,
            exported_at INTEGER NOT NULL DEFAULT (unixepoch()),
            source_rule_id INTEGER REFERENCES relay_publish_rules(id),
            desired_state TEXT NOT NULL DEFAULT 'present',
            last_error TEXT,
            last_checked_at INTEGER,
            export_origin TEXT NOT NULL DEFAULT 'nrd_export'
        );"#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS relay_runtime_state (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        );"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
