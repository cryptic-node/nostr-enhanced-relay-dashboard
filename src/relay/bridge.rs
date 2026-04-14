use anyhow::Context;
use rusqlite::Connection;
use sqlx::SqlitePool;
use tokio::time::{interval, Duration};
use tracing::{error, info, warn};

use super::schema_guard;

pub async fn run_export_bridge(
    pool: SqlitePool,
    nrd_db: String,
    nrr_db: String,
    interval_secs: u64,
) {
    match schema_guard::check_nrr_schema(&nrr_db) {
        Ok((true, version)) => {
            let _ = upsert_runtime_state(&pool, "bridge_enabled", "true").await;
            let _ = upsert_runtime_state(&pool, "nrr_schema_version", &version.to_string()).await;
        }
        Ok((false, version)) => {
            let _ = upsert_runtime_state(&pool, "bridge_enabled", "false").await;
            let _ = upsert_runtime_state(&pool, "nrr_schema_version", &version.to_string()).await;
            warn!("Export bridge disabled due to schema mismatch.");
            return;
        }
        Err(err) => {
            let _ = upsert_runtime_state(&pool, "bridge_enabled", "false").await;
            error!("Bridge startup schema check failed: {err}");
            return;
        }
    }

    let mut ticker = interval(Duration::from_secs(interval_secs.max(5)));
    loop {
        ticker.tick().await;
        if let Err(err) = export_batch(&pool, &nrd_db, &nrr_db).await {
            error!("Bridge export batch failed: {err}");
        }
    }
}

pub async fn export_batch(pool: &SqlitePool, nrd_db: &str, nrr_db: &str) -> anyhow::Result<usize> {
    let nrr = Connection::open(nrr_db).context("open NRR db")?;
    nrr.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    nrr.execute_batch(&format!("ATTACH DATABASE 'file:{nrd_db}?mode=ro' AS nrd;"))?;

    let exported = nrr.execute(
        r#"INSERT OR IGNORE INTO event (id, pubkey, created_at, kind, tags, content, sig)
        SELECT e.id, e.pubkey, e.created_at, e.kind, e.tags_json, e.content, e.sig
        FROM nrd.events e
        INNER JOIN nrd.relay_publish_rules r
            ON ((r.rule_type = 'author' AND e.pubkey = r.rule_value)
            OR (r.rule_type = 'event' AND e.id = r.rule_value)
            OR (r.rule_type = 'kind' AND CAST(e.kind AS TEXT) = r.rule_value))
        WHERE r.enabled = 1
        LIMIT 500;"#,
        [],
    )?;

    nrr.execute(
        r#"INSERT OR IGNORE INTO nrd.relay_export_ledger(event_id, source_rule_id, export_origin)
           SELECT e.id, r.id, 'nrd_export'
           FROM nrd.events e
           INNER JOIN nrd.relay_publish_rules r
             ON ((r.rule_type = 'author' AND e.pubkey = r.rule_value)
             OR (r.rule_type = 'event' AND e.id = r.rule_value)
             OR (r.rule_type = 'kind' AND CAST(e.kind AS TEXT) = r.rule_value))
           WHERE r.enabled = 1
           LIMIT 500;"#,
        [],
    )?;

    nrr.execute_batch("DETACH DATABASE nrd;")?;

    upsert_runtime_state(
        pool,
        "last_export_run_at",
        &chrono::Utc::now().timestamp().to_string(),
    )
    .await?;
    upsert_runtime_state(pool, "last_export_count", &exported.to_string()).await?;

    if exported > 0 {
        info!("Bridge exported {exported} events to NRR");
    }

    Ok(exported)
}

pub async fn upsert_runtime_state(pool: &SqlitePool, key: &str, value: &str) -> anyhow::Result<()> {
    sqlx::query(
        r#"INSERT INTO relay_runtime_state(key, value, updated_at)
           VALUES(?1, ?2, unixepoch())
           ON CONFLICT(key)
           DO UPDATE SET value=excluded.value, updated_at=unixepoch();"#,
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}
