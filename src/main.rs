mod app_state;
mod config;
mod db;
mod relay;
mod routes;

use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use app_state::AppState;
use axum::{response::Html, routing::get, Router};
use config::Config;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = Arc::new(Config::from_env());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .context("connect sqlite")?;

    db::init(&pool, &config.local_relay_ws_url).await?;

    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
    };

    let nrd_db_path = config
        .database_url
        .trim_start_matches("sqlite://")
        .to_string();
    let nrr_db_path = config.nrr_db_path.to_string_lossy().to_string();
    let bridge_pool = pool.clone();
    let interval_secs = config.bridge_interval_secs;
    tokio::spawn(async move {
        relay::bridge::run_export_bridge(bridge_pool, nrd_db_path, nrr_db_path, interval_secs)
            .await;
    });

    let app = Router::new()
        .route("/", get(index))
        .route("/healthz", get(|| async { "ok" }))
        .nest("/api/relay", routes::relay::router())
        .with_state(state);

    let addr: SocketAddr = config.bind_addr.parse().context("parse BIND_ADDR")?;
    info!("NERD v0.0.1 listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut sigterm) = signal(SignalKind::terminate()) {
            sigterm.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../public/index.html"))
}
