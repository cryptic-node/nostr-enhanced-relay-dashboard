use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PublishRule {
    pub id: i64,
    pub enabled: bool,
    pub rule_type: String,
    pub rule_value: String,
    pub kinds_json: Option<String>,
    pub backfill_mode: String,
}

pub async fn init(pool: &SqlitePool) -> anyhow::Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS relay_publish_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            enabled INTEGER NOT NULL DEFAULT 1,
            rule_type TEXT NOT NULL,
            rule_value TEXT NOT NULL,
            kinds_json TEXT,
            backfill_mode TEXT NOT NULL DEFAULT 'future_only',
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        );"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list(pool: &SqlitePool) -> anyhow::Result<Vec<PublishRule>> {
    let rows = sqlx::query_as::<_, (i64, i64, String, String, Option<String>, String)>(
        r#"SELECT id, enabled, rule_type, rule_value, kinds_json, backfill_mode
           FROM relay_publish_rules ORDER BY id DESC"#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| PublishRule {
            id: r.0,
            enabled: r.1 != 0,
            rule_type: r.2,
            rule_value: r.3,
            kinds_json: r.4,
            backfill_mode: r.5,
        })
        .collect())
}
