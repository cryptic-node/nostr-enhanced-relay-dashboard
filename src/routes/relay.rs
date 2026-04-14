use std::{path::PathBuf, sync::Arc};

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;

use crate::{
    app_state::AppState,
    db::relay_rules,
    relay::{bridge, manager, schema_guard, status, types::CompatStatus, types::CreateRuleRequest},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(relay_status))
        .route("/config", get(relay_config).post(relay_config_update))
        .route("/compat", get(relay_compat))
        .route("/summary", get(relay_summary))
        .route("/export-rules", get(export_rules).post(export_rules_create))
        .route(
            "/export-rules/:id",
            put(export_rules_update).delete(export_rules_delete),
        )
        .route("/export/preview", get(export_preview))
        .route("/export/run", post(export_run))
        .route("/observe/run", post(observe_run))
        .route("/restart", post(restart_nrr))
        .route("/writers", post(writer_add))
        .route("/writers/:pubkey", delete(writer_remove))
        .route("/prune", post(prune))
        .route("/logs", get(logs))
}

fn ensure_admin(headers: &HeaderMap, token: &str) -> Result<(), (StatusCode, &'static str)> {
    let provided = headers.get("x-admin-token").and_then(|h| h.to_str().ok());
    if provided == Some(token) {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "missing/invalid admin token"))
    }
}

async fn relay_status(State(app): State<AppState>) -> impl IntoResponse {
    manager::wait_for_nrr(&app.config.local_relay_http_url).await;
    Json(manager::fetch_nip11(&app.config.local_relay_http_url).await)
}

async fn relay_config(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    match manager::load_nrr_config(&app.config.nrr_config_path) {
        Ok(content) => Json(serde_json::json!({"config": content})).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct ConfigUpdate {
    content: String,
}

async fn relay_config_update(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ConfigUpdate>,
) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    match manager::write_nrr_config(&app.config.nrr_config_path, &body.content) {
        Ok(hash) => {
            Json(serde_json::json!({"ok": true, "config_hash": hash, "restart_required": true}))
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn relay_compat(State(app): State<AppState>) -> impl IntoResponse {
    match schema_guard::check_nrr_schema(app.config.nrr_db_path.to_string_lossy().as_ref()) {
        Ok((compatible, version)) => Json(CompatStatus {
            bridge_compatible: compatible,
            schema_version: version,
            supported_range: format!(
                "{}-{}",
                schema_guard::NRR_SCHEMA_MIN,
                schema_guard::NRR_SCHEMA_MAX
            ),
        })
        .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn relay_summary(State(app): State<AppState>) -> impl IntoResponse {
    let row1 = sqlx::query_scalar::<_, Option<String>>(
        "SELECT value FROM relay_runtime_state WHERE key='last_export_run_at'",
    )
    .fetch_optional(&app.pool)
    .await
    .ok()
    .flatten();
    let row2 = sqlx::query_scalar::<_, Option<String>>(
        "SELECT value FROM relay_runtime_state WHERE key='last_export_count'",
    )
    .fetch_optional(&app.pool)
    .await
    .ok()
    .flatten();

    let nrd_path = PathBuf::from(app.config.database_url.trim_start_matches("sqlite://"));
    let summary = status::build_summary(&nrd_path, &app.config.nrr_db_path, row1, row2);
    Json(summary).into_response()
}

async fn export_rules(State(app): State<AppState>) -> impl IntoResponse {
    match relay_rules::list(&app.pool).await {
        Ok(rules) => Json(serde_json::json!({"rules": rules})).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn export_rules_create(
    State(app): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    let enabled = body.enabled.unwrap_or(true);
    let backfill_mode = body
        .backfill_mode
        .unwrap_or_else(|| "future_only".to_string());
    match sqlx::query(
        r#"INSERT INTO relay_publish_rules(enabled, rule_type, rule_value, kinds_json, backfill_mode)
           VALUES(?1, ?2, ?3, ?4, ?5)"#,
    )
    .bind(if enabled { 1 } else { 0 })
    .bind(body.rule_type)
    .bind(body.rule_value)
    .bind(body.kinds_json)
    .bind(backfill_mode)
    .execute(&app.pool)
    .await
    {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": err.to_string()}))).into_response(),
    }
}

async fn export_rules_update(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(body): Json<CreateRuleRequest>,
) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    match sqlx::query(
        r#"UPDATE relay_publish_rules SET
             enabled=?1, rule_type=?2, rule_value=?3, kinds_json=?4, backfill_mode=?5, updated_at=unixepoch()
            WHERE id=?6"#,
    )
    .bind(if body.enabled.unwrap_or(true) { 1 } else { 0 })
    .bind(body.rule_type)
    .bind(body.rule_value)
    .bind(body.kinds_json)
    .bind(body.backfill_mode.unwrap_or_else(|| "future_only".to_string()))
    .bind(id)
    .execute(&app.pool)
    .await
    {
        Ok(_) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(err) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": err.to_string()}))).into_response(),
    }
}

async fn export_rules_delete(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    let _ = sqlx::query("DELETE FROM relay_publish_rules WHERE id=?1")
        .bind(id)
        .execute(&app.pool)
        .await;
    Json(serde_json::json!({"ok": true}))
}

async fn export_preview(State(app): State<AppState>) -> impl IntoResponse {
    let count = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM events e
           INNER JOIN relay_publish_rules r
           ON ((r.rule_type = 'author' AND e.pubkey = r.rule_value)
           OR (r.rule_type = 'event' AND e.id = r.rule_value)
           OR (r.rule_type = 'kind' AND CAST(e.kind AS TEXT) = r.rule_value))
           WHERE r.enabled=1 AND e.id NOT IN (SELECT event_id FROM relay_export_ledger);"#,
    )
    .fetch_one(&app.pool)
    .await
    .unwrap_or(0);

    Json(serde_json::json!({"candidate_count": count}))
}

async fn export_run(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }

    let nrd_path = app
        .config
        .database_url
        .trim_start_matches("sqlite://")
        .to_string();
    match bridge::export_batch(
        &app.pool,
        &nrd_path,
        app.config.nrr_db_path.to_string_lossy().as_ref(),
    )
    .await
    {
        Ok(count) => Json(serde_json::json!({"ok": true, "exported": count})).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn observe_run(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    let _ = bridge::upsert_runtime_state(
        &app.pool,
        "last_local_observe_sync_at",
        &chrono::Utc::now().timestamp().to_string(),
    )
    .await;
    Json(serde_json::json!({"ok": true, "message": "placeholder: hook existing relay sync worker"}))
}

async fn restart_nrr(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    let _ = app;
    Json(
        serde_json::json!({"ok": true, "message": "placeholder: run docker compose restart nostr-rs-relay"}),
    )
}

async fn writer_add(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    Json(
        serde_json::json!({"ok": true, "message": "placeholder: parse and modify nrr-config.toml whitelist"}),
    )
}

async fn writer_remove(
    State(app): State<AppState>,
    headers: HeaderMap,
    Path(pubkey): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    Json(serde_json::json!({"ok": true, "message": format!("placeholder remove {pubkey}")}))
}

async fn prune(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    let _ = app;
    Json(
        serde_json::json!({"ok": true, "message": "placeholder: delete only ledger tracked rows from NRR event table"}),
    )
}

async fn logs(State(app): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = ensure_admin(&headers, &app.config.admin_token) {
        return e.into_response();
    }
    Json(serde_json::json!({"logs": "placeholder: mount and tail NRR log file"}))
}
