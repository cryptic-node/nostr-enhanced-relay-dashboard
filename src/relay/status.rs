use std::{fs, path::Path};

use crate::relay::types::RelaySummary;

pub fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn build_summary(
    nrd_db: &Path,
    nrr_db: &Path,
    last_export_run_at: Option<String>,
    last_export_count: Option<String>,
) -> RelaySummary {
    let wal = nrr_db.with_extension("db-wal");
    RelaySummary {
        nrd_db_size_bytes: file_size(nrd_db),
        nrr_db_size_bytes: file_size(nrr_db),
        nrr_wal_size_bytes: file_size(&wal),
        last_export_run_at,
        last_export_count,
    }
}
