# Integration Guide: NRD v1.0.5 → NERD v0.0.1

This document outlines the steps to transform Nostr Relay Dashboard v1.0.5 into Nostr Enhanced Relay Dashboard v0.0.1 with nostr-rs-relay integration and privacy features.

---

## 📦 Files Created

### Core Modules
1. **auth_policy.rs** - Privacy and AUTH policy engine
2. **relay_proxy.rs** - WebSocket proxy with NIP-42 inspection
3. **002_privacy_and_auth.sql** - Database migration

### Configuration
4. **relay-config.toml** - nostr-rs-relay configuration
5. **docker-compose-nerd.yml** - Updated compose with relay backend
6. **Cargo-nerd.toml** - Updated dependencies

### Documentation
7. **NERD_v0.0.1_DESIGN.md** - Complete architecture design
8. **README_NERD_v0.0.1.md** - User documentation

---

## 🔧 Integration Steps

### Step 1: File Structure

```
nostr-enhanced-relay-dashboard/
├── src/
│   ├── main.rs                 # UPDATE: Add relay proxy routes
│   ├── lib.rs                  # UPDATE: Export new modules
│   ├── auth_policy.rs          # NEW
│   ├── relay_proxy.rs          # NEW
│   ├── db.rs                   # KEEP
│   └── sync.rs                 # KEEP
├── public/
│   └── index.html              # UPDATE: Add privacy controls UI
├── migrations/
│   ├── 001_initial.sql         # KEEP
│   └── 002_privacy_and_auth.sql # NEW
├── Cargo.toml                  # REPLACE with Cargo-nerd.toml
├── docker-compose.yml          # REPLACE with docker-compose-nerd.yml
├── relay-config.toml           # NEW
├── Dockerfile                  # UPDATE: Multi-service setup
└── README.md                   # REPLACE with README_NERD_v0.0.1.md
```

---

## 📝 Code Changes Required

### 1. Update `src/lib.rs`

```rust
pub mod auth_policy;
pub mod relay_proxy;
pub mod sync;
```

### 2. Update `src/main.rs`

Add to imports:
```rust
use crate::{auth_policy::*, relay_proxy::*};
```

Add to router (before `.nest_service`):
```rust
.route("/ws", get(relay_ws_handler))
.route("/api/relay/status", get(get_relay_status))
.route("/api/relay/connections", get(get_active_connections))
.route("/api/relay/auth-policy", get(get_auth_policy).post(update_auth_policy))
.route("/api/relay/whitelist/:npub_id/toggle", post(toggle_whitelist))
```

Add handler functions:
```rust
async fn get_relay_status(State(state): State<Arc<AppState>>) -> Json<RelayStatus> {
    let total_events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM events")
        .fetch_one(&state.pool).await.unwrap_or(0);
    
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM relay_connections WHERE disconnected_at IS NULL"
    ).fetch_one(&state.pool).await.unwrap_or(0);
    
    let authenticated: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM relay_connections 
         WHERE authenticated = 1 AND disconnected_at IS NULL"
    ).fetch_one(&state.pool).await.unwrap_or(0);
    
    Json(RelayStatus {
        total_events,
        active_connections: active,
        authenticated_connections: authenticated,
    })
}

async fn get_active_connections(State(state): State<Arc<AppState>>) -> Json<Vec<ConnectionInfo>> {
    let connections = sqlx::query_as::<_, ConnectionInfo>(
        r#"
        SELECT id, pubkey_hex, ip_address, authenticated, connected_at
        FROM relay_connections
        WHERE disconnected_at IS NULL
        ORDER BY connected_at DESC
        "#
    )
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default();
    
    Json(connections)
}

async fn get_auth_policy(State(state): State<Arc<AppState>>) -> Json<AuthPolicyResponse> {
    let policy = AuthPolicy::load_from_db(&state.pool).await;
    
    Json(AuthPolicyResponse {
        global_auth_required: policy.global_auth_required,
        whitelist_only: policy.whitelist_only,
        auth_read_required: policy.auth_read_required,
        protected_kinds: policy.protected_kinds.into_iter().collect(),
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
    
    // Update database settings
    upsert_setting(&state.pool, "global_auth_required", 
                   &payload.global_auth_required.to_string()).await;
    upsert_setting(&state.pool, "whitelist_only", 
                   &payload.whitelist_only.to_string()).await;
    upsert_setting(&state.pool, "auth_read_required", 
                   &payload.auth_read_required.to_string()).await;
    upsert_setting(&state.pool, "auth_kind_0", 
                   &payload.auth_kind_0.to_string()).await;
    upsert_setting(&state.pool, "auth_kind_1", 
                   &payload.auth_kind_1.to_string()).await;
    
    log_message("AUTH policy updated via dashboard");
    json_response(StatusCode::OK, true, "AUTH policy updated successfully")
}

async fn toggle_whitelist(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(npub_id): Path<i64>,
    Json(payload): Json<ToggleWhitelistRequest>,
) -> Response {
    if let Some(response) = require_admin(&headers) {
        return response;
    }
    
    let result = sqlx::query("UPDATE monitored_npubs SET whitelisted = ? WHERE id = ?")
        .bind(if payload.whitelisted { 1 } else { 0 })
        .bind(npub_id)
        .execute(&state.pool)
        .await;
    
    match result {
        Ok(done) if done.rows_affected() > 0 => {
            let action = if payload.whitelisted { "whitelisted" } else { "removed from whitelist" };
            log_message(&format!("Npub ID {} {}", npub_id, action));
            json_response(StatusCode::OK, true, format!("Npub {}", action))
        }
        Ok(_) => json_response(StatusCode::NOT_FOUND, false, "Npub not found"),
        Err(e) => json_response(StatusCode::INTERNAL_SERVER_ERROR, false, 
                               format!("Failed to update whitelist: {}", e)),
    }
}
```

Add types:
```rust
#[derive(Serialize)]
struct RelayStatus {
    total_events: i64,
    active_connections: i64,
    authenticated_connections: i64,
}

#[derive(Serialize, sqlx::FromRow)]
struct ConnectionInfo {
    id: i64,
    pubkey_hex: Option<String>,
    ip_address: String,
    authenticated: i64,
    connected_at: String,
}

#[derive(Serialize)]
struct AuthPolicyResponse {
    global_auth_required: bool,
    whitelist_only: bool,
    auth_read_required: bool,
    protected_kinds: Vec<u16>,
}

#[derive(Deserialize)]
struct ToggleWhitelistRequest {
    whitelisted: bool,
}
```

Update `ensure_tables()` to run new migration:
```rust
async fn ensure_tables(pool: &SqlitePool) {
    // ... existing code ...
    
    // Run privacy migration
    let migration_sql = include_str!("../migrations/002_privacy_and_auth.sql");
    for statement in migration_sql.split(";") {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            let _ = sqlx::query(trimmed).execute(pool).await;
        }
    }
    
    log_message("Database ready — NERD v0.0.1");
}
```

### 3. Update `public/index.html`

Add Privacy Controls panel after Monitored Npubs panel:

```html
<!-- Privacy & AUTH Controls Panel -->
<div class="panel bg-zinc-900 rounded-3xl p-6 border border-zinc-800 flex flex-col h-[680px]">
    <h2 class="text-lg font-medium mb-4 flex items-center gap-x-2">
        <i class="fa-solid fa-shield-halved"></i> Privacy & AUTH Controls
    </h2>
    
    <div class="flex-1 min-h-0 space-y-4 overflow-y-auto pr-2">
        <!-- Global AUTH -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <label class="flex items-center justify-between">
                <span class="text-sm font-medium">Require AUTH for all connections</span>
                <input type="checkbox" id="global-auth-required" 
                       onchange="updateAuthPolicy()"
                       class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
            </label>
            <p class="text-xs text-zinc-400 mt-2">
                Force all clients to authenticate via NIP-42 before any operations
            </p>
        </div>

        <!-- Per-Kind AUTH -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <h3 class="text-sm font-medium mb-3">Event Kind Protection</h3>
            
            <div class="space-y-2">
                <label class="flex items-center justify-between text-xs opacity-50">
                    <span>Kind 4 (DMs) - Always Protected 🔒</span>
                    <input type="checkbox" checked disabled
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-emerald-500">
                </label>
                <label class="flex items-center justify-between text-xs opacity-50">
                    <span>Kind 1059 (Gift Wrap) - Always Protected 🔒</span>
                    <input type="checkbox" checked disabled
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-emerald-500">
                </label>
                <label class="flex items-center justify-between text-xs">
                    <span>Kind 1 (Text Notes)</span>
                    <input type="checkbox" id="auth-kind-1" onchange="updateAuthPolicy()"
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
                </label>
                <label class="flex items-center justify-between text-xs">
                    <span>Kind 0 (Metadata)</span>
                    <input type="checkbox" id="auth-kind-0" onchange="updateAuthPolicy()"
                           class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
                </label>
            </div>
            
            <p class="text-xs text-zinc-400 mt-3">
                <i class="fa-solid fa-lock text-emerald-400"></i> 
                DMs and Gift Wraps are always protected for privacy
            </p>
        </div>

        <!-- Whitelist Mode -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <label class="flex items-center justify-between">
                <span class="text-sm font-medium">Whitelist-only mode</span>
                <input type="checkbox" id="whitelist-mode" onchange="updateAuthPolicy()"
                       class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
            </label>
            <p class="text-xs text-zinc-400 mt-2">
                Only allow writes from whitelisted pubkeys (toggle in Monitored Npubs)
            </p>
        </div>

        <!-- Read Protection -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <label class="flex items-center justify-between">
                <span class="text-sm font-medium">Require AUTH for REQ</span>
                <input type="checkbox" id="auth-read-required" onchange="updateAuthPolicy()"
                       class="h-4 w-4 rounded border-zinc-600 bg-zinc-900 text-purple-500">
            </label>
            <p class="text-xs text-zinc-400 mt-2">
                Require authentication even to read/subscribe to events
            </p>
        </div>

        <!-- Connection Stats -->
        <div class="bg-zinc-800 rounded-2xl p-4">
            <h3 class="text-sm font-medium mb-2">Active Connections</h3>
            <div id="connection-stats" class="grid grid-cols-2 gap-3 text-xs">
                <div>
                    <div class="text-zinc-400">Total</div>
                    <div class="text-xl font-bold text-purple-400" id="total-connections">0</div>
                </div>
                <div>
                    <div class="text-zinc-400">Authenticated</div>
                    <div class="text-xl font-bold text-emerald-400" id="auth-connections">0</div>
                </div>
            </div>
        </div>
    </div>
</div>
```

Add JavaScript functions:
```javascript
async function fetchAuthPolicy() {
    const response = await fetch('/api/relay/auth-policy');
    const policy = await response.json();
    
    document.getElementById('global-auth-required').checked = policy.global_auth_required;
    document.getElementById('whitelist-mode').checked = policy.whitelist_only;
    document.getElementById('auth-read-required').checked = policy.auth_read_required;
    document.getElementById('auth-kind-0').checked = policy.protected_kinds.includes(0);
    document.getElementById('auth-kind-1').checked = policy.protected_kinds.includes(1);
}

async function updateAuthPolicy() {
    const payload = {
        global_auth_required: document.getElementById('global-auth-required').checked,
        whitelist_only: document.getElementById('whitelist-mode').checked,
        auth_read_required: document.getElementById('auth-read-required').checked,
        auth_kind_0: document.getElementById('auth-kind-0').checked,
        auth_kind_1: document.getElementById('auth-kind-1').checked,
    };
    
    const response = await fetchWithAdminRetry('/api/relay/auth-policy', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
    }, 'Could not update AUTH policy');
    
    if (!response) return;
    const data = await response.json();
    showStatus(data.message, !response.ok || !data.success);
}

async function fetchConnectionStats() {
    const response = await fetch('/api/relay/status');
    const stats = await response.json();
    
    document.getElementById('total-connections').textContent = stats.active_connections;
    document.getElementById('auth-connections').textContent = stats.authenticated_connections;
}

async function toggleNpubWhitelist(npubId) {
    const npub = npubsCache.find(n => n.id === npubId);
    const newState = !npub.whitelisted;
    
    const response = await fetchWithAdminRetry(`/api/relay/whitelist/${npubId}/toggle`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ whitelisted: newState })
    }, 'Could not toggle whitelist');
    
    if (!response) return;
    const data = await response.json();
    showStatus(data.message, !response.ok || !data.success);
    
    if (data.success) {
        await fetchNpubs();
    }
}

// Update window.onload
window.onload = async () => {
    await fetchRelays();
    await fetchNpubs();
    await fetchSettings();
    await fetchAuthPolicy();
    await fetchConnectionStats();
    
    // Poll connection stats every 5 seconds
    setInterval(fetchConnectionStats, 5000);
    
    console.log('%c✅ NERD v0.0.1 loaded', 'color:#10b981;font-weight:bold;');
};
```

Update npub card rendering to show whitelist toggle:
```javascript
function renderNpubCard(npub) {
    const displayNpub = truncate(npub.npub, 28);
    const isSelected = npub.id === selectedNpubId ? 'npub-selected' : '';
    const whitelistIcon = npub.whitelisted 
        ? '<i class="fa-solid fa-shield-check text-emerald-400"></i>'
        : '<i class="fa-solid fa-shield text-zinc-500"></i>';

    return `
        <div onclick="selectNpub(${npub.id})" class="bg-zinc-800 rounded-2xl px-4 py-3 text-xs cursor-pointer ${isSelected}">
            <div class="flex items-start justify-between gap-3">
                <div class="min-w-0 flex-1">
                    <div class="font-medium flex items-center gap-2">
                        <span>${escapeHtml(npub.label || 'No label')}</span>
                        ${whitelistIcon}
                    </div>
                    <div class="text-zinc-400 font-mono">${escapeHtml(displayNpub)}</div>
                </div>
                <div class="flex gap-2 shrink-0">
                    <button onclick="event.stopPropagation(); toggleNpubWhitelist(${npub.id})" 
                            class="w-8 h-8 rounded-full bg-zinc-700 hover:bg-purple-500 text-zinc-200 hover:text-white"
                            title="Toggle whitelist">
                        <i class="fa-solid fa-shield-halved"></i>
                    </button>
                    <button onclick="event.stopPropagation(); deleteNpub(${npub.id})" 
                            class="w-8 h-8 rounded-full bg-zinc-700 hover:bg-red-500 text-zinc-200 hover:text-white">
                        <i class="fa-solid fa-trash"></i>
                    </button>
                </div>
            </div>
            <div class="mt-3 flex items-end justify-between gap-3">
                <div class="text-emerald-400">${npub.notes_stored} stored notes</div>
                <div class="text-zinc-400 text-[10px]">${escapeHtml(npub.last_synced || 'never')}</div>
            </div>
        </div>
    `;
}
```

### 4. Update `Dockerfile`

No major changes needed, but ensure:
```dockerfile
# Make sure we copy the new modules
COPY src ./src
COPY public ./public
COPY migrations ./migrations
```

---

## 🧪 Testing Checklist

### Basic Functionality
- [ ] Dashboard loads at localhost:8080
- [ ] Can add/remove relays
- [ ] Can add/remove npubs
- [ ] Sync works (recent/deep/full modes)
- [ ] Backup/restore works

### Privacy Features
- [ ] AUTH policy settings persist
- [ ] Connection stats update in real-time
- [ ] Whitelist toggle works
- [ ] Can connect to relay at ws://localhost:7447

### Relay Integration
- [ ] nostr-rs-relay starts and stays running
- [ ] Dashboard proxy intercepts messages
- [ ] AUTH protection enforced for protected kinds
- [ ] DM (kind 4) always requires AUTH
- [ ] Gift Wrap (kind 1059) always requires AUTH

### Client Testing
Test with real Nostr clients:
- [ ] Damus/nos (iOS)
- [ ] Amethyst (Android)
- [ ] Iris/Snort (web)
- [ ] nostrudel (web)

Verify:
- [ ] Can connect without AUTH (if not required)
- [ ] Blocked from DMs without AUTH
- [ ] AUTH challenge works
- [ ] Can post after AUTH
- [ ] Subscriptions work

---

## 📊 Performance Targets

- Dashboard load time: < 2s
- WebSocket proxy latency: < 10ms
- Database queries: < 100ms
- Memory usage: < 512MB (dashboard + relay)
- Concurrent connections: 1000+

---

## 🚀 Deployment Checklist

Before going to production:
- [ ] Set strong ADMIN_TOKEN
- [ ] Configure relay_url in relay-config.toml
- [ ] Set up HTTPS (Caddy recommended)
- [ ] Enable nightly backups
- [ ] Configure retention limits
- [ ] Test backup/restore procedure
- [ ] Document custom policies
- [ ] Set up monitoring/alerts

---

## 📝 Migration Notes

### Breaking Changes
None! NERD v0.0.1 is fully backwards compatible with NRD v1.0.5 data.

### New Database Tables
- `auth_policy` - Privacy settings
- `relay_connections` - Connection tracking
- `auth_log` - AUTH activity log
- `relay_stats` - Performance metrics

### New Columns
- `monitored_npubs.whitelisted` - Whitelist flag
- `events.relay_connection_id` - Source tracking

### Data Preservation
All existing data (relays, npubs, events, settings, sync state) is preserved and migrated automatically.

---

## ✅ Completion Criteria

NERD v0.0.1 is complete when:
- [x] All core modules implemented
- [x] Database migration created
- [x] Dashboard UI updated
- [x] Docker setup configured
- [x] Documentation written
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Client compatibility verified
- [ ] Performance targets met
- [ ] Security audit passed

---

## 🎯 Next Steps

1. **Copy files** from /home/claude/ to your project
2. **Run migration** on existing database
3. **Update imports** in main.rs and lib.rs
4. **Test locally** with docker-compose
5. **Verify AUTH** with Nostr client
6. **Deploy** to production
7. **Monitor** logs and stats
8. **Iterate** based on feedback

---

**Ready to transform your relay into NERD v0.0.1!** 🚀
