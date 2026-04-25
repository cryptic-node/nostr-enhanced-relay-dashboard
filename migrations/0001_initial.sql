-- NERD initial schema
-- Migration: 0001_initial.sql

-- Relays NERD syncs FROM
CREATE TABLE IF NOT EXISTS relays (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    url         TEXT    NOT NULL UNIQUE,
    enabled     INTEGER NOT NULL DEFAULT 1,
    -- Encryption scaffolding (not used in v0.0.1, but reserved for future)
    encrypted   INTEGER NOT NULL DEFAULT 0,
    -- Bookkeeping
    added_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    last_seen   INTEGER,
    notes       TEXT
);

CREATE INDEX IF NOT EXISTS idx_relays_enabled ON relays(enabled);

-- Npubs NERD syncs FOR (i.e., subscribes to events authored by these pubkeys)
CREATE TABLE IF NOT EXISTS npubs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hex  TEXT    NOT NULL UNIQUE,  -- 64-char hex, not bech32
    label       TEXT,                       -- optional human-friendly name
    enabled     INTEGER NOT NULL DEFAULT 1,
    -- Encryption scaffolding
    encrypted   INTEGER NOT NULL DEFAULT 0,
    -- Bookkeeping
    added_at    INTEGER NOT NULL DEFAULT (unixepoch()),
    notes       TEXT
);

CREATE INDEX IF NOT EXISTS idx_npubs_enabled ON npubs(enabled);

-- Per-(relay, npub) sync state - tracks last successful sync watermark
CREATE TABLE IF NOT EXISTS sync_state (
    relay_id        INTEGER NOT NULL REFERENCES relays(id) ON DELETE CASCADE,
    npub_id         INTEGER NOT NULL REFERENCES npubs(id)  ON DELETE CASCADE,
    last_event_at   INTEGER,    -- unix timestamp of latest event seen
    last_synced_at  INTEGER,    -- when we last completed a sync pass
    PRIMARY KEY (relay_id, npub_id)
);

-- Nostr events archived locally
CREATE TABLE IF NOT EXISTS events (
    id          TEXT    PRIMARY KEY,        -- 64-char hex event id
    pubkey_hex  TEXT    NOT NULL,            -- author pubkey (64-char hex)
    created_at  INTEGER NOT NULL,            -- unix timestamp from event
    kind        INTEGER NOT NULL,
    tags        TEXT    NOT NULL,            -- JSON array
    content     TEXT    NOT NULL,
    sig         TEXT    NOT NULL,
    -- Bookkeeping
    archived_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_events_pubkey      ON events(pubkey_hex);
CREATE INDEX IF NOT EXISTS idx_events_kind        ON events(kind);
CREATE INDEX IF NOT EXISTS idx_events_created_at  ON events(created_at);
CREATE INDEX IF NOT EXISTS idx_events_pubkey_kind ON events(pubkey_hex, kind);
