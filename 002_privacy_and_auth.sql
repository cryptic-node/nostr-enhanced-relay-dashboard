-- 002_privacy_and_auth.sql
-- Migration for Nostr Enhanced Relay Dashboard v0.0.1
-- Adds privacy controls, AUTH policy, and relay connection tracking

-- ============================================================================
-- AUTH Policy Settings Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS auth_policy (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Default privacy settings (conservative defaults)
INSERT OR IGNORE INTO auth_policy (key, value) VALUES 
    ('global_auth_required', 'false'),
    ('whitelist_only', 'false'),
    ('auth_read_required', 'false'),
    ('auth_kind_0', 'false'),  -- Metadata
    ('auth_kind_1', 'false');  -- Text notes
    -- Note: kinds 4, 17, and 1059 are ALWAYS protected in code

-- ============================================================================
-- Whitelist Management
-- ============================================================================
-- Add whitelist column to monitored_npubs
-- When whitelist_only=true, only these pubkeys can write events
ALTER TABLE monitored_npubs ADD COLUMN whitelisted INTEGER DEFAULT 0;

-- Create index for fast whitelist lookups
CREATE INDEX IF NOT EXISTS idx_monitored_npubs_whitelisted 
    ON monitored_npubs(whitelisted, pubkey_hex);

-- ============================================================================
-- Relay Connection Tracking
-- ============================================================================
CREATE TABLE IF NOT EXISTS relay_connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hex TEXT,                -- NULL if not authenticated
    ip_address TEXT NOT NULL,
    authenticated INTEGER DEFAULT 0,
    connected_at TEXT DEFAULT (datetime('now')),
    disconnected_at TEXT,
    user_agent TEXT                 -- Future: track client info
);

-- Index for active connections query
CREATE INDEX IF NOT EXISTS idx_relay_connections_active 
    ON relay_connections(disconnected_at, authenticated);

-- Index for pubkey connection history
CREATE INDEX IF NOT EXISTS idx_relay_connections_pubkey 
    ON relay_connections(pubkey_hex);

-- ============================================================================
-- AUTH Activity Log
-- ============================================================================
CREATE TABLE IF NOT EXISTS auth_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hex TEXT NOT NULL,
    action TEXT NOT NULL,           -- 'AUTH', 'DENIED', 'CHALLENGE'
    result TEXT NOT NULL,           -- 'success', 'invalid_sig', 'expired', etc.
    ip_address TEXT,
    timestamp TEXT DEFAULT (datetime('now'))
);

-- Index for searching auth logs by pubkey
CREATE INDEX IF NOT EXISTS idx_auth_log_pubkey 
    ON auth_log(pubkey_hex, timestamp DESC);

-- Index for recent auth activity
CREATE INDEX IF NOT EXISTS idx_auth_log_recent 
    ON auth_log(timestamp DESC);

-- ============================================================================
-- Relay Statistics Table (Optional)
-- ============================================================================
CREATE TABLE IF NOT EXISTS relay_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp TEXT DEFAULT (datetime('now')),
    total_events INTEGER DEFAULT 0,
    active_connections INTEGER DEFAULT 0,
    authenticated_connections INTEGER DEFAULT 0,
    events_per_second REAL DEFAULT 0.0,
    subscriptions_active INTEGER DEFAULT 0
);

-- ============================================================================
-- Event Source Tracking Enhancement
-- ============================================================================
-- Add relay_connection_id to events to track which connection submitted them
ALTER TABLE events ADD COLUMN relay_connection_id INTEGER REFERENCES relay_connections(id);

-- Index for finding events by connection
CREATE INDEX IF NOT EXISTS idx_events_relay_connection 
    ON events(relay_connection_id);

-- ============================================================================
-- Views for Dashboard
-- ============================================================================

-- Active connections summary
CREATE VIEW IF NOT EXISTS v_active_connections AS
SELECT 
    COUNT(*) as total,
    SUM(CASE WHEN authenticated = 1 THEN 1 ELSE 0 END) as authenticated,
    SUM(CASE WHEN authenticated = 0 THEN 1 ELSE 0 END) as unauthenticated
FROM relay_connections
WHERE disconnected_at IS NULL;

-- Recent AUTH activity
CREATE VIEW IF NOT EXISTS v_recent_auth_activity AS
SELECT 
    pubkey_hex,
    substr(pubkey_hex, 1, 16) || '...' as pubkey_short,
    action,
    result,
    timestamp
FROM auth_log
ORDER BY timestamp DESC
LIMIT 100;

-- Connection statistics by pubkey
CREATE VIEW IF NOT EXISTS v_connection_stats_by_pubkey AS
SELECT 
    pubkey_hex,
    COUNT(*) as total_connections,
    SUM(CASE WHEN authenticated = 1 THEN 1 ELSE 0 END) as authenticated_count,
    MAX(connected_at) as last_seen,
    AVG(julianday(disconnected_at) - julianday(connected_at)) * 24 * 60 as avg_duration_minutes
FROM relay_connections
WHERE pubkey_hex IS NOT NULL
GROUP BY pubkey_hex;

-- ============================================================================
-- Privacy Settings Helpers
-- ============================================================================

-- Function-like query to check if AUTH is required for a kind
-- Usage: SELECT * FROM fn_auth_required_for_kind(1)
CREATE VIEW IF NOT EXISTS fn_auth_required_for_kind AS
SELECT 
    CASE 
        WHEN key = 'global_auth_required' AND value = 'true' THEN 1
        WHEN key = 'auth_kind_0' AND value = 'true' THEN 1
        WHEN key = 'auth_kind_1' AND value = 'true' THEN 1
        ELSE 0
    END as required
FROM auth_policy;

-- ============================================================================
-- Data Retention Policies
-- ============================================================================

-- Clean up old disconnected connections (keep last 30 days)
CREATE TRIGGER IF NOT EXISTS cleanup_old_connections
AFTER INSERT ON relay_connections
BEGIN
    DELETE FROM relay_connections
    WHERE disconnected_at IS NOT NULL
    AND datetime(disconnected_at) < datetime('now', '-30 days');
END;

-- Clean up old auth logs (keep last 90 days)
CREATE TRIGGER IF NOT EXISTS cleanup_old_auth_logs
AFTER INSERT ON auth_log
BEGIN
    DELETE FROM auth_log
    WHERE datetime(timestamp) < datetime('now', '-90 days');
END;

-- ============================================================================
-- Monitoring Queries (for debugging)
-- ============================================================================

-- Check current policy status
-- SELECT * FROM auth_policy ORDER BY key;

-- See active connections
-- SELECT * FROM v_active_connections;

-- Recent AUTH attempts
-- SELECT * FROM v_recent_auth_activity;

-- Most active pubkeys
-- SELECT * FROM v_connection_stats_by_pubkey ORDER BY total_connections DESC LIMIT 20;

-- ============================================================================
-- Backwards Compatibility
-- ============================================================================

-- Ensure settings table has auth_policy entries
INSERT OR IGNORE INTO settings (key, value)
SELECT key, value FROM auth_policy;

-- ============================================================================
-- Migration Complete
-- ============================================================================
-- This migration adds:
-- 1. AUTH policy configuration (auth_policy table)
-- 2. Whitelist support (monitored_npubs.whitelisted column)
-- 3. Connection tracking (relay_connections table)
-- 4. AUTH activity logging (auth_log table)
-- 5. Statistics tracking (relay_stats table)
-- 6. Helpful views and automatic cleanup triggers
--
-- All features are opt-in with conservative defaults:
-- - No AUTH required by default
-- - Whitelist mode disabled
-- - DMs and Gift Wraps always protected (enforced in code)
-- ============================================================================
