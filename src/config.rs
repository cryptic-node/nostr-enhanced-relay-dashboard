use anyhow::{Context, Result};
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub filter: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8082,
            },
            database: DatabaseConfig {
                path: "./data/nerd.db".to_string(),
            },
            logging: LoggingConfig {
                filter: "info,sqlx::query=warn".to_string(),
            },
        }
    }
}

impl Config {
    /// Load config from `nerd.toml` (if present) overlaid with `NERD_*` env vars.
    /// Missing files are tolerated; missing values fall back to `Default`.
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut figment = Figment::from(figment::providers::Serialized::defaults(Self::default()));

        if path.exists() {
            figment = figment.merge(Toml::file(path));
        }

        // Env vars: NERD_SERVER__PORT=8083 -> server.port = 8083
        figment = figment.merge(Env::prefixed("NERD_").split("__"));

        figment
            .extract()
            .with_context(|| format!("failed to load config from {}", path.display()))
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}
