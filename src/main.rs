use anyhow::{Context, Result};
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod config;
mod db;
mod routes;

use config::Config;
use routes::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load config (TOML + env overrides)
    let cfg = Config::load("nerd.toml").context("failed to load configuration")?;

    // 2. Initialize tracing subscriber using configured filter.
    //    RUST_LOG env var, if set, overrides the config-file filter.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&cfg.logging.filter));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_level(true))
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "starting nerd-server"
    );

    // 3. Open DB and run migrations
    let pool = db::init(&cfg.database.path).await?;

    // 4. Build axum app
    let state = AppState { db: pool };
    let app = routes::build(state);

    // 5. Bind and serve
    let addr = cfg.bind_addr();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {}", addr))?;

    info!(addr = %addr, "NERD ready");

    axum::serve(listener, app)
        .await
        .context("server error")?;

    Ok(())
}
