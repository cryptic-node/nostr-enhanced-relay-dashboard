use rusqlite::Connection;
use tracing::{error, info};

pub const NRR_SCHEMA_MIN: i64 = 7;
pub const NRR_SCHEMA_MAX: i64 = 15;

pub fn check_nrr_schema(nrr_db_path: &str) -> anyhow::Result<(bool, i64)> {
    let conn = Connection::open(nrr_db_path)?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;

    if version < NRR_SCHEMA_MIN || version > NRR_SCHEMA_MAX {
        error!(
            "NRR schema version {} is outside tested range {}-{}.",
            version, NRR_SCHEMA_MIN, NRR_SCHEMA_MAX
        );
        return Ok((false, version));
    }

    info!("NRR schema version {} is bridge-compatible.", version);
    Ok((true, version))
}
