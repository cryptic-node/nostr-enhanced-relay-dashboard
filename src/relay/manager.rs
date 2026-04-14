use std::{fs, time::Duration};

use anyhow::Context;
use reqwest::Client;
use sha2::{Digest, Sha256};

pub async fn wait_for_nrr(url: &str) {
    for attempt in 0u32..10 {
        if reqwest::get(url).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(5)))).await;
    }
    tracing::warn!("NRR did not become healthy after retries.");
}

pub async fn fetch_nip11(url: &str) -> serde_json::Value {
    let client = Client::builder().timeout(Duration::from_secs(5)).build();
    let Ok(client) = client else {
        return serde_json::json!({"online": false, "error": "client init failed"});
    };

    match client
        .get(url)
        .header("Accept", "application/nostr+json")
        .send()
        .await
    {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(v) => v,
            Err(_) => serde_json::json!({"online": false, "error": "parse failed"}),
        },
        Err(err) => serde_json::json!({"online": false, "error": err.to_string()}),
    }
}

pub fn load_nrr_config(path: &std::path::Path) -> anyhow::Result<String> {
    fs::read_to_string(path).context("read nrr config")
}

pub fn write_nrr_config(path: &std::path::Path, body: &str) -> anyhow::Result<String> {
    fs::write(path, body).context("write nrr config")?;
    config_hash(body)
}

pub fn config_hash(content: &str) -> anyhow::Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    Ok(hex::encode(hasher.finalize()))
}
