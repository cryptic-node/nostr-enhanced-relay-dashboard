// auth_policy.rs - Privacy and Authentication Policy Engine
// Implements NIP-42 enhanced AUTH with configurable per-kind protection

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::collections::HashSet;

/// Authentication policy configuration loaded from database
#[derive(Clone, Debug)]
pub struct AuthPolicy {
    /// Require AUTH for all operations
    pub global_auth_required: bool,
    /// Only allow whitelisted pubkeys to write
    pub whitelist_only: bool,
    /// Require AUTH even for reading/subscribing
    pub auth_read_required: bool,
    /// Event kinds that require AUTH
    pub protected_kinds: HashSet<u16>,
    /// Whitelisted pubkey hex values
    pub whitelisted_pubkeys: HashSet<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AuthPolicyUpdate {
    pub global_auth_required: bool,
    pub whitelist_only: bool,
    pub auth_read_required: bool,
    pub auth_kind_0: bool,  // Metadata
    pub auth_kind_1: bool,  // Text notes
    // Kinds 4 and 1059 are always protected
}

impl AuthPolicy {
    /// Load current policy from database settings
    pub async fn load_from_db(pool: &SqlitePool) -> Self {
        let global_auth = get_setting_bool(pool, "global_auth_required").await;
        let whitelist_only = get_setting_bool(pool, "whitelist_only").await;
        let auth_read = get_setting_bool(pool, "auth_read_required").await;

        let mut protected_kinds = HashSet::new();
        
        // ALWAYS protected - privacy-critical kinds
        protected_kinds.insert(4);    // DMs (NIP-04)
        protected_kinds.insert(1059); // Gift Wrap (NIP-59)
        protected_kinds.insert(17);   // Private DMs (NIP-17)

        // Configurable protection for other kinds
        if get_setting_bool(pool, "auth_kind_1").await {
            protected_kinds.insert(1);  // Text notes
        }
        if get_setting_bool(pool, "auth_kind_0").await {
            protected_kinds.insert(0);  // Metadata
        }

        let whitelisted_pubkeys = load_whitelist(pool).await;

        AuthPolicy {
            global_auth_required: global_auth,
            whitelist_only,
            auth_read_required: auth_read,
            protected_kinds,
            whitelisted_pubkeys,
        }
    }

    /// Check if a specific event kind requires authentication
    pub fn requires_auth_for_kind(&self, kind: u16) -> bool {
        self.global_auth_required || self.protected_kinds.contains(&kind)
    }

    /// Check if reading/subscribing requires authentication
    pub fn requires_auth_for_read(&self) -> bool {
        self.auth_read_required
    }

    /// Check if a pubkey is whitelisted (only matters if whitelist_only=true)
    pub fn is_whitelisted(&self, pubkey: &str) -> bool {
        !self.whitelist_only || self.whitelisted_pubkeys.contains(pubkey)
    }

    /// Check if a write operation should be allowed
    pub fn can_write(&self, pubkey: &str, kind: u16, is_authenticated: bool) -> bool {
        // If global auth required and not authenticated, reject
        if self.global_auth_required && !is_authenticated {
            return false;
        }

        // If this kind requires auth and not authenticated, reject
        if self.requires_auth_for_kind(kind) && !is_authenticated {
            return false;
        }

        // If whitelist mode enabled and not whitelisted, reject
        if !self.is_whitelisted(pubkey) {
            return false;
        }

        true
    }
}

/// Load whitelisted pubkeys from monitored_npubs table
async fn load_whitelist(pool: &SqlitePool) -> HashSet<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT pubkey_hex FROM monitored_npubs WHERE whitelisted = 1 AND pubkey_hex IS NOT NULL",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter().map(|(pk,)| pk).collect()
}

/// Get boolean setting from database
async fn get_setting_bool(pool: &SqlitePool, key: &str) -> bool {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
        .unwrap_or(false)
}

/// Log an AUTH-related event
pub async fn log_auth_event(
    pool: &SqlitePool,
    pubkey: &str,
    action: &str,
    result: &str,
) {
    let _ = sqlx::query(
        "INSERT INTO auth_log (pubkey_hex, action, result) VALUES (?, ?, ?)",
    )
    .bind(pubkey)
    .bind(action)
    .bind(result)
    .execute(pool)
    .await;
}

/// Track a new connection
pub async fn track_connection(
    pool: &SqlitePool,
    pubkey: Option<&str>,
    ip: &str,
    authenticated: bool,
) -> i64 {
    let result = sqlx::query(
        r#"
        INSERT INTO relay_connections (pubkey_hex, ip_address, authenticated)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(pubkey)
    .bind(ip)
    .bind(if authenticated { 1 } else { 0 })
    .execute(pool)
    .await;

    result.map(|r| r.last_insert_rowid()).unwrap_or(0)
}

/// Mark connection as authenticated
pub async fn mark_authenticated(pool: &SqlitePool, connection_id: i64, pubkey: &str) {
    let _ = sqlx::query(
        r#"
        UPDATE relay_connections 
        SET authenticated = 1, pubkey_hex = ?
        WHERE id = ?
        "#,
    )
    .bind(pubkey)
    .bind(connection_id)
    .execute(pool)
    .await;
}

/// Mark connection as disconnected
pub async fn mark_disconnected(pool: &SqlitePool, connection_id: i64) {
    let _ = sqlx::query(
        "UPDATE relay_connections SET disconnected_at = datetime('now') WHERE id = ?",
    )
    .bind(connection_id)
    .execute(pool)
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_always_protected_kinds() {
        let mut policy = AuthPolicy {
            global_auth_required: false,
            whitelist_only: false,
            auth_read_required: false,
            protected_kinds: HashSet::new(),
            whitelisted_pubkeys: HashSet::new(),
        };

        // Add always-protected kinds
        policy.protected_kinds.insert(4);
        policy.protected_kinds.insert(1059);
        policy.protected_kinds.insert(17);

        assert!(policy.requires_auth_for_kind(4), "DMs should always require AUTH");
        assert!(policy.requires_auth_for_kind(1059), "Gift Wrap should always require AUTH");
        assert!(policy.requires_auth_for_kind(17), "Private DMs should always require AUTH");
        assert!(!policy.requires_auth_for_kind(1), "Text notes should not require AUTH by default");
    }

    #[test]
    fn test_whitelist_mode() {
        let mut policy = AuthPolicy {
            global_auth_required: false,
            whitelist_only: true,
            auth_read_required: false,
            protected_kinds: HashSet::new(),
            whitelisted_pubkeys: HashSet::new(),
        };

        policy.whitelisted_pubkeys.insert("test_pubkey".to_string());

        assert!(policy.is_whitelisted("test_pubkey"), "Whitelisted pubkey should pass");
        assert!(!policy.is_whitelisted("other_pubkey"), "Non-whitelisted pubkey should fail");
    }

    #[test]
    fn test_can_write() {
        let mut policy = AuthPolicy {
            global_auth_required: false,
            whitelist_only: false,
            auth_read_required: false,
            protected_kinds: HashSet::new(),
            whitelisted_pubkeys: HashSet::new(),
        };

        policy.protected_kinds.insert(4);

        // Should allow writing kind 1 without auth
        assert!(policy.can_write("test_pubkey", 1, false));

        // Should deny writing kind 4 without auth
        assert!(!policy.can_write("test_pubkey", 4, false));

        // Should allow writing kind 4 with auth
        assert!(policy.can_write("test_pubkey", 4, true));
    }
}
