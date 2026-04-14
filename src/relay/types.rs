use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct CompatStatus {
    pub bridge_compatible: bool,
    pub schema_version: i64,
    pub supported_range: String,
}

#[derive(Debug, Serialize)]
pub struct RelaySummary {
    pub nrd_db_size_bytes: u64,
    pub nrr_db_size_bytes: u64,
    pub nrr_wal_size_bytes: u64,
    pub last_export_run_at: Option<String>,
    pub last_export_count: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRuleRequest {
    pub enabled: Option<bool>,
    pub rule_type: String,
    pub rule_value: String,
    pub kinds_json: Option<String>,
    pub backfill_mode: Option<String>,
}
