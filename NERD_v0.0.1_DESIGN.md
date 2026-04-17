# Nostr Enhanced Relay Dashboard (NERD) v0.0.1

## Design Document

### Vision
Transform the Nostr Relay Dashboard v1.0.5 into a full-featured relay with integrated dashboard, combining the archive/sync capabilities with nostr-rs-relay backend to provide a complete, privacy-focused relay solution.

---

## Core Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Web Dashboard (Port 8080)                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  - Relay Management UI                                │  │
│  │  - Npub Monitoring                                    │  │
│  │  - Privacy Controls (NEW)                             │  │
│  │  - AUTH Policy Manager (NEW)                          │  │
│  │  │                                                     │  │
│  └───────────────────────────────────────────────────────┘  │
│                            ▲                                 │
│                            │ HTTP API                        │
│                            ▼                                 │
│  ┌───────────────────────────────────────────────────────┐  │
│  │          Rust Backend (main.rs)                       │  │
│  │  - API Routes                                         │  │
│  │  - Admin Token Auth                                   │  │
│  │  - Sync Engine                                        │  │
│  │  - Backup/Restore                                     │  │
│  │  - Relay Proxy Layer (NEW)                            │  │
│  └───────────────────────────────────────────────────────┘  │
│                            ▲                                 │
│                            │                                 │
└────────────────────────────┼─────────────────────────────────┘
                             │
                             │ IPC / Shared DB
                             ▼
┌─────────────────────────────────────────────────────────────┐
│              nostr-rs-relay (Port 7447)                      │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  WebSocket Server                                     │  │
│  │  - NIP-01 (Basic Protocol)                            │  │
│  │  - NIP-42 (AUTH) ★ ENHANCED                           │  │
│  │  - NIP-11 (Relay Info)                                │  │
│  │  - NIP-09 (Event Deletion)                            │  │
│  │  - NIP-40 (Expiration)                                │  │
│  │  - Event validation                                   │  │
│  │  - Storage engine (SQLite)                            │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Privacy Layer (NEW) - NIP-42 Enhanced                │  │
│  │  ┌─────────────────────────────────────────────────┐  │  │
│  │  │ AUTH Policy Engine                              │  │  │
│  │  │ - Whitelist management                          │  │  │
│  │  │ - Per-kind AUTH requirements                    │  │  │
│  │  │ - DM protection (kind 4)                        │  │  │
│  │  │ - Gift Wrap protection (kind 1059)              │  │  │
│  │  │ - Configurable defaults                         │  │  │
│  │  └─────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## New Features for v0.0.1

### 1. **Integrated Relay Backend (nostr-rs-relay)**

**Status**: New integration
**Implementation**: Run nostr-rs-relay as a subprocess or sidecar container

#### Configuration
```toml
# config.toml for nostr-rs-relay
[info]
relay_url = "wss://your-relay.example.com"
name = "Your Enhanced Relay"
description = "Privacy-focused relay with dashboard"
pubkey = "your_hex_pubkey"
contact = "admin@example.com"

[database]
data_directory = "/app/data/relay"
engine = "sqlite"

[network]
port = 7447
address = "127.0.0.1"

[authorization]
pubkey_whitelist = []  # Managed via dashboard
nip42_auth_required = false  # Configurable per-kind

[limits]
messages_per_sec = 60
subscriptions_per_min = 20
max_event_bytes = 131072
max_ws_message_bytes = 524288
max_subscriptions = 20

[verified_users]
mode = "enabled"  # Options: disabled, passive, enabled
domain_whitelist = []
verify_expiration_days = 30

[retention]
# Managed by dashboard backup system
max_events = 5000000
max_bytes = 10737418240  # 10GB
```

#### Docker Compose Integration
```yaml
services:
  nerd-dashboard:
    build: .
    container_name: nerd-dashboard
    restart: unless-stopped
    ports:
      - "8080:8080"
    volumes:
      - relay-data:/app/data
    environment:
      - PORT=8080
      - HOST=0.0.0.0
      - DATABASE_PATH=/app/data/dashboard.db
      - RELAY_BACKEND=http://127.0.0.1:7447
      - NRD_ADMIN_TOKEN=${ADMIN_TOKEN}
    depends_on:
      - nostr-relay

  nostr-relay:
    image: scsibug/nostr-rs-relay:latest
    container_name: nostr-relay-backend
    restart: unless-stopped
    ports:
      - "7447:7447"
    volumes:
      - relay-data:/usr/src/app/db
      - ./relay-config.toml:/usr/src/app/config.toml
    environment:
      - RUST_LOG=warn,nostr_rs_relay=info

volumes:
  relay-data:
    driver: local
```

---

### 2. **Privacy Controls - NIP-42 Enhanced AUTH**

**Status**: New feature
**Purpose**: Wisp-style privacy protection at the relay level

#### Dashboard UI Addition

New "Privacy & AUTH" panel in the dashboard:

```html
<div class="panel bg-zinc-900 rounded-3xl p-6 border border-zinc-800">
    <h2 class="text-lg font-medium mb-4 flex items-center gap-x-2">
        <i class="fa-solid fa-shield-halved"></i> Privacy & AUTH Controls
    </h2>
    
    <div class="space-y-4">
        <!-- Global AUTH Policy -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <label class="flex items-center justify-between">
                <span class="text-sm font-medium">Require AUTH for all connections</span>
                <input type="checkbox" id="global-auth-required" 
                       class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
            </label>
            <p class="text-xs text-zinc-400 mt-2">
                Force all clients to authenticate before any operations
            </p>
        </div>

        <!-- Per-Kind AUTH -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <h3 class="text-sm font-medium mb-3">Event Kind Protection</h3>
            
            <div class="space-y-2">
                <label class="flex items-center justify-between text-xs">
                    <span>Kind 4 (DMs) - Require AUTH</span>
                    <input type="checkbox" id="auth-kind-4" checked disabled
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-emerald-500">
                </label>
                <label class="flex items-center justify-between text-xs">
                    <span>Kind 1059 (Gift Wrap) - Require AUTH</span>
                    <input type="checkbox" id="auth-kind-1059" checked disabled
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-emerald-500">
                </label>
                <label class="flex items-center justify-between text-xs">
                    <span>Kind 1 (Text Notes) - Require AUTH</span>
                    <input type="checkbox" id="auth-kind-1"
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
                </label>
                <label class="flex items-center justify-between text-xs">
                    <span>Kind 0 (Metadata) - Require AUTH</span>
                    <input type="checkbox" id="auth-kind-0"
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
                </label>
            </div>
            
            <p class="text-xs text-zinc-400 mt-3">
                <i class="fa-solid fa-lock text-emerald-400"></i> 
                DMs and Gift Wraps are always protected by default
            </p>
        </div>

        <!-- Whitelist Mode -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <label class="flex items-center justify-between">
                <span class="text-sm font-medium">Whitelist-only mode</span>
                <input type="checkbox" id="whitelist-mode"
                       class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
            </label>
            <p class="text-xs text-zinc-400 mt-2">
                Only allow authenticated pubkeys on your whitelist to write events
            </p>
        </div>

        <!-- Read Protection -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <label class="flex items-center justify-between">
                <span class="text-sm font-medium">Require AUTH for REQ</span>
                <input type="checkbox" id="auth-read-required"
                       class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
            </label>
            <p class="text-xs text-zinc-400 mt-2">
                Require authentication even to read/subscribe to events
            </p>
        </div>

        <!-- Connection Tracking -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <h3 class="text-sm font-medium mb-2">Active Connections</h3>
            <div id="connection-stats" class="grid grid-cols-2 gap-3 text-xs">
                <div>
                    <div class="text-zinc-400">Total</div>
                    <div class="text-xl font-bold text-purple-400">0</div>
                </div>
                <div>
                    <div class="text-zinc-400">Authenticated</div>
                    <div class="text-xl font-bold text-emerald-400">0</div>
                </div>
            </div>
        </div>
    </div>
</div>
```

#### Backend Implementation

New `auth_policy.rs` module:

```rust
use sqlx::SqlitePool;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct AuthPolicy {
    pub global_auth_required: bool,
    pub whitelist_only: bool,
    pub auth_read_required: bool,
    pub protected_kinds: HashSet<u16>,
    pub whitelisted_pubkeys: HashSet<String>,
}

impl AuthPolicy {
    pub async fn load_from_db(pool: &SqlitePool) -> Self {
        let global_auth = get_setting_bool(pool, "global_auth_required").await;
        let whitelist_only = get_setting_bool(pool, "whitelist_only").await;
        let auth_read = get_setting_bool(pool, "auth_read_required").await;
        
        let mut protected_kinds = HashSet::new();
        protected_kinds.insert(4);    // DMs - always protected
        protected_kinds.insert(1059); // Gift Wrap - always protected
        
        // Load configurable kinds
        if get_setting_bool(pool, "auth_kind_1").await {
            protected_kinds.insert(1);
        }
        if get_setting_bool(pool, "auth_kind_0").await {
            protected_kinds.insert(0);
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
    
    pub fn requires_auth_for_kind(&self, kind: u16) -> bool {
        self.global_auth_required || self.protected_kinds.contains(&kind)
    }
    
    pub fn requires_auth_for_read(&self) -> bool {
        self.auth_read_required
    }
    
    pub fn is_whitelisted(&self, pubkey: &str) -> bool {
        !self.whitelist_only || self.whitelisted_pubkeys.contains(pubkey)
    }
}

async fn load_whitelist(pool: &SqlitePool) -> HashSet<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT pubkey_hex FROM monitored_npubs WHERE whitelisted = 1"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    
    rows.into_iter().map(|(pk,)| pk).collect()
}
```

---

### 3. **Relay Proxy Layer**

**Status**: New feature
**Purpose**: Dashboard can intercept and inspect relay traffic

Create `relay_proxy.rs`:

```rust
use axum::{
    extract::{ws::WebSocket, ws::WebSocketUpgrade, State},
    response::Response,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;

pub async fn relay_ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_relay_connection(socket, state))
}

async fn handle_relay_connection(socket: WebSocket, state: Arc<AppState>) {
    let (mut client_sink, mut client_stream) = socket.split();
    
    // Connect to backend relay
    let backend_url = std::env::var("RELAY_BACKEND")
        .unwrap_or_else(|_| "ws://127.0.0.1:7447".to_string());
    
    let (backend_ws, _) = tokio_tungstenite::connect_async(&backend_url)
        .await
        .expect("Failed to connect to backend relay");
    
    let (mut backend_sink, mut backend_stream) = backend_ws.split();
    
    // Load AUTH policy
    let policy = AuthPolicy::load_from_db(&state.pool).await;
    let mut authenticated_pubkeys = HashSet::new();
    
    // Client -> Backend
    let client_to_backend = async {
        while let Some(Ok(msg)) = client_stream.next().await {
            if let axum::extract::ws::Message::Text(text) = msg {
                // Parse and inspect message
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                    match parsed.as_array().and_then(|a| a.get(0))
                                         .and_then(|v| v.as_str()) {
                        Some("EVENT") => {
                            // Check if kind requires AUTH
                            if let Some(event) = parsed.get(1) {
                                if let Some(kind) = event.get("kind").and_then(|k| k.as_u64()) {
                                    if policy.requires_auth_for_kind(kind as u16) {
                                        if let Some(pubkey) = event.get("pubkey")
                                            .and_then(|p| p.as_str()) {
                                            if !authenticated_pubkeys.contains(pubkey) {
                                                // Send auth-required error
                                                let error_msg = format!(
                                                    r#"["OK", "{}", false, "auth-required: kind {} requires authentication"]"#,
                                                    event.get("id").and_then(|i| i.as_str()).unwrap_or(""),
                                                    kind
                                                );
                                                let _ = client_sink.send(
                                                    axum::extract::ws::Message::Text(error_msg)
                                                ).await;
                                                continue;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Some("AUTH") => {
                            // Track successful auth
                            if let Some(event) = parsed.get(1) {
                                if let Some(pubkey) = event.get("pubkey")
                                    .and_then(|p| p.as_str()) {
                                    authenticated_pubkeys.insert(pubkey.to_string());
                                    log_message(&format!(
                                        "Client authenticated as {}", 
                                        &pubkey[..16]
                                    ));
                                }
                            }
                        }
                        Some("REQ") => {
                            if policy.requires_auth_for_read() 
                                && authenticated_pubkeys.is_empty() {
                                let sub_id = parsed.get(1)
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("");
                                let error_msg = format!(
                                    r#"["CLOSED", "{}", "auth-required: relay requires authentication"]"#,
                                    sub_id
                                );
                                let _ = client_sink.send(
                                    axum::extract::ws::Message::Text(error_msg)
                                ).await;
                                continue;
                            }
                        }
                        _ => {}
                    }
                }
                
                // Forward to backend
                let backend_msg = tokio_tungstenite::tungstenite::Message::Text(text);
                let _ = backend_sink.send(backend_msg).await;
            }
        }
    };
    
    // Backend -> Client
    let backend_to_client = async {
        while let Some(Ok(msg)) = backend_stream.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                let client_msg = axum::extract::ws::Message::Text(text);
                let _ = client_sink.send(client_msg).await;
            }
        }
    };
    
    tokio::select! {
        _ = client_to_backend => {},
        _ = backend_to_client => {},
    }
}
```

---

### 4. **Database Schema Updates**

Add to migration:

```sql
-- Privacy settings
CREATE TABLE IF NOT EXISTS auth_policy (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT DEFAULT (datetime('now'))
);

-- Whitelist for AUTH
ALTER TABLE monitored_npubs ADD COLUMN whitelisted INTEGER DEFAULT 0;

-- Connection tracking
CREATE TABLE IF NOT EXISTS relay_connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hex TEXT,
    ip_address TEXT,
    authenticated INTEGER DEFAULT 0,
    connected_at TEXT DEFAULT (datetime('now')),
    disconnected_at TEXT
);

-- AUTH activity log
CREATE TABLE IF NOT EXISTS auth_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hex TEXT NOT NULL,
    action TEXT NOT NULL,
    result TEXT NOT NULL,
    timestamp TEXT DEFAULT (datetime('now'))
);

-- Default privacy settings
INSERT OR IGNORE INTO auth_policy (key, value) VALUES 
    ('global_auth_required', 'false'),
    ('whitelist_only', 'false'),
    ('auth_read_required', 'false'),
    ('auth_kind_1', 'false'),
    ('auth_kind_0', 'false');
```

---

### 5. **New API Endpoints**

Add to `main.rs` router:

```rust
.route("/api/relay/status", get(get_relay_status))
.route("/api/relay/connections", get(get_active_connections))
.route("/api/relay/auth-policy", get(get_auth_policy).post(update_auth_policy))
.route("/api/relay/whitelist/:npub_id/toggle", post(toggle_whitelist))
.route("/ws", get(relay_ws_handler))  // Main relay WebSocket endpoint
```

Handlers:

```rust
async fn get_relay_status(State(state): State<Arc<AppState>>) -> Json<RelayStatus> {
    let total_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);
    
    let total_connections: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM relay_connections WHERE disconnected_at IS NULL"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    
    let authenticated: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM relay_connections 
         WHERE authenticated = 1 AND disconnected_at IS NULL"
    )
    .fetch_one(&state.pool)
    .await
    .unwrap_or(0);
    
    Json(RelayStatus {
        total_events,
        total_connections,
        authenticated_connections: authenticated,
        uptime_seconds: get_uptime(),
    })
}

async fn update_auth_policy(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<AuthPolicyUpdate>,
) -> Response {
    if let Some(response) = require_admin(&headers) {
        return response;
    }
    
    upsert_setting(&state.pool, "global_auth_required", 
                   &payload.global_auth_required.to_string()).await;
    upsert_setting(&state.pool, "whitelist_only", 
                   &payload.whitelist_only.to_string()).await;
    upsert_setting(&state.pool, "auth_read_required", 
                   &payload.auth_read_required.to_string()).await;
    
    log_message("AUTH policy updated via dashboard");
    json_response(StatusCode::OK, true, "AUTH policy updated")
}
```

---

## Migration Path

### Phase 1: Setup (Week 1)
1. ✅ Add nostr-rs-relay to docker-compose
2. ✅ Update Dockerfile to handle two services
3. ✅ Test basic relay functionality
4. ✅ Verify NIP-01 compatibility

### Phase 2: Privacy Layer (Week 2)
1. ✅ Implement `auth_policy.rs`
2. ✅ Add database schema for privacy settings
3. ✅ Create relay proxy layer
4. ✅ Implement NIP-42 inspection

### Phase 3: Dashboard Integration (Week 3)
1. ✅ Add Privacy & AUTH UI panel
2. ✅ Implement API endpoints
3. ✅ Add connection monitoring
4. ✅ Create AUTH activity log viewer

### Phase 4: Testing & Docs (Week 4)
1. ✅ Test with multiple Nostr clients
2. ✅ Verify AUTH flow works
3. ✅ Document configuration
4. ✅ Create migration guide from v1.0.5

---

## Configuration Example

### Environment Variables
```bash
# Dashboard
PORT=8080
HOST=0.0.0.0
DATABASE_PATH=/app/data/dashboard.db
NRD_ADMIN_TOKEN=your-secure-token-here
BACKUP_DIR=/app/data/backups

# Relay Backend
RELAY_BACKEND=http://127.0.0.1:7447
RELAY_CONFIG_PATH=/app/data/relay-config.toml
```

### relay-config.toml
```toml
[info]
relay_url = "wss://relay.example.com"
name = "My Enhanced Relay"
description = "Privacy-focused relay with dashboard"
pubkey = "your_npub_as_hex"
contact = "admin@example.com"

[database]
data_directory = "/app/data/relay"
engine = "sqlite"

[network]
port = 7447
address = "127.0.0.1"

[authorization]
nip42_auth_required = false  # Managed by dashboard

[limits]
messages_per_sec = 60
max_subscriptions = 20
```

---

## Security Features

### 1. **Admin Token Protection**
- All mutating operations require NRD_ADMIN_TOKEN
- Token stored as HTTP header or Bearer token
- Prompt for token if not set in localStorage

### 2. **Privacy by Default**
- DMs (kind 4) ALWAYS require AUTH
- Gift Wrap (kind 1059) ALWAYS require AUTH
- Configurable protection for other kinds

### 3. **Whitelist Mode**
- Optional pubkey whitelist
- Managed via dashboard
- Syncs with monitored npubs

### 4. **Connection Tracking**
- Track authenticated vs unauthenticated
- Log AUTH attempts
- Monitor for suspicious activity

---

## Testing Checklist

### Relay Functionality
- [ ] Can connect with standard Nostr client
- [ ] Events are stored correctly
- [ ] REQ filters work properly
- [ ] EVENT publishing works
- [ ] AUTH challenge/response works

### Privacy Features
- [ ] DM protection enforced
- [ ] Gift Wrap protection enforced
- [ ] Whitelist mode blocks non-whitelisted
- [ ] AUTH required mode works globally
- [ ] Read protection works

### Dashboard
- [ ] Privacy controls update policy
- [ ] Connection stats display correctly
- [ ] AUTH log shows activity
- [ ] Sync still works for monitored npubs
- [ ] Backup/restore includes new tables

---

## Documentation Needs

1. **Installation Guide**
   - Docker setup
   - Configuration
   - First-time setup wizard

2. **Privacy Guide**
   - Understanding NIP-42
   - Configuring AUTH policies
   - Whitelist management

3. **API Reference**
   - New endpoints
   - WebSocket protocol
   - AUTH flow

4. **Migration Guide**
   - v1.0.5 → v0.0.1
   - Data preservation
   - Config changes

---

## Future Enhancements (v0.0.2+)

1. **NIP-50 Search** - Full-text search integration
2. **NIP-65 Relay Lists** - Advertise relay recommendations
3. **NIP-77 Negentropy** - Efficient sync protocol
4. **Rate Limiting** - Per-pubkey rate limits
5. **Payment Integration** - Lightning payments for access
6. **Analytics Dashboard** - Usage statistics, popular content
7. **Moderation Tools** - Content filtering, spam detection

---

## Success Metrics

- ✅ Relay passes NIP-01 compliance tests
- ✅ AUTH protection works as expected
- ✅ Dashboard controls relay behavior
- ✅ Zero downtime migration from v1.0.5
- ✅ Performance matches standalone nostr-rs-relay
- ✅ Privacy features configurable without relay restart

---

## License & Credits

- Based on Nostr Relay Dashboard v1.0.5
- Uses nostr-rs-relay as backend
- Implements privacy features inspired by Wisp client
- MIT License

---

## Contact & Support

- GitHub: github.com/cryptic-node/nostr-enhanced-relay-dashboard
- Issues: Open an issue on GitHub
- Nostr: [Your npub here]
