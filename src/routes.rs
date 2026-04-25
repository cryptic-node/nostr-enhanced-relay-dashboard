use axum::{routing::get, Json, Router};
use serde::Serialize;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    #[allow(dead_code)] // used in later sessions
    pub db: SqlitePool,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
