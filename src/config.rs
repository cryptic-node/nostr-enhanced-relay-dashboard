use std::{env, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    pub admin_token: String,
    pub local_relay_http_url: String,
    pub local_relay_ws_url: String,
    pub nrr_db_path: PathBuf,
    pub nrr_config_path: PathBuf,
    pub bridge_interval_secs: u64,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:///app/data/nrd.db".to_string()),
            admin_token: env::var("NRD_ADMIN_TOKEN").unwrap_or_else(|_| "dev-token".to_string()),
            local_relay_http_url: env::var("LOCAL_RELAY_HTTP_URL")
                .unwrap_or_else(|_| "http://nostr-rs-relay:8080".to_string()),
            local_relay_ws_url: env::var("LOCAL_RELAY_WS_URL")
                .unwrap_or_else(|_| "ws://nostr-rs-relay:8080".to_string()),
            nrr_db_path: env::var("NRR_DB_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/app/relay-db/nostr.db")),
            nrr_config_path: env::var("NRR_CONFIG_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/app/config/nrr-config.toml")),
            bridge_interval_secs: env::var("BRIDGE_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(30),
        }
    }
}
