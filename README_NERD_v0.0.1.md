# Nostr Enhanced Relay Dashboard (NERD) v0.0.1

**Privacy-focused Nostr relay with integrated dashboard management**

Transform your Nostr relay into a feature-rich, privacy-focused platform with powerful monitoring, authentication controls, and archival capabilities.

---

## 🌟 Features

### Core Functionality
- ✅ **Full Nostr Relay** - Built on battle-tested nostr-rs-relay
- ✅ **Web Dashboard** - Beautiful, intuitive management interface
- ✅ **Npub Monitoring** - Track and archive specific accounts
- ✅ **Multi-Relay Sync** - Pull events from multiple upstream relays
- ✅ **Backup & Restore** - NDJSON format for data portability

### Privacy & Security (NEW in v0.0.1)
- 🔐 **NIP-42 AUTH** - Enhanced authentication with per-kind control
- 🛡️ **Privacy Protection** - DMs and Gift Wraps always require AUTH
- 📋 **Whitelist Mode** - Restrict writes to approved pubkeys
- 🔒 **Read Protection** - Optional AUTH requirement for subscriptions
- 📊 **Connection Tracking** - Monitor authenticated vs anonymous users
- 📝 **AUTH Activity Log** - Audit trail of authentication events

### Management
- ⚙️ **Admin Token Protection** - Secure access to sensitive operations
- 📈 **Real-time Stats** - Connection counts, event totals, sync status
- 🗄️ **Automated Backups** - Nightly backup scheduler with retention
- 🔄 **Flexible Sync Modes** - Recent, deep, and full backfill options

---

## 🚀 Quick Start

### Prerequisites
- Docker & Docker Compose
- 2GB RAM minimum
- 10GB disk space for event storage

### Installation

1. **Clone the repository**
```bash
git clone https://github.com/cryptic-node/nostr-enhanced-relay-dashboard
cd nostr-enhanced-relay-dashboard
```

2. **Configure environment**
```bash
cp .env.example .env
nano .env
```

Set your admin token:
```bash
ADMIN_TOKEN=your-secure-random-token-here
DOMAIN=relay.example.com  # Optional: for HTTPS
```

3. **Update relay config**
```bash
nano relay-config.toml
```

Set your relay info:
```toml
[info]
relay_url = "wss://relay.example.com"
name = "My Enhanced Relay"
pubkey = "your_npub_as_hex"
contact = "admin@example.com"
```

4. **Start the services**
```bash
docker-compose up -d
```

5. **Access dashboard**
```
http://localhost:8080
```

---

## 📋 Configuration

### Dashboard Settings

#### Environment Variables
```bash
PORT=8080                          # Dashboard HTTP port
HOST=0.0.0.0                       # Bind address
DATABASE_PATH=/app/data/dashboard.db
BACKUP_DIR=/app/data/backups
NRD_ADMIN_TOKEN=your-token         # Required for admin operations
RELAY_BACKEND=ws://nostr-relay:7447
RUST_LOG=info                      # Logging level
```

#### Relay Settings (relay-config.toml)
```toml
[network]
port = 7447
address = "0.0.0.0"

[limits]
messages_per_sec = 60
max_subscriptions = 20
max_event_bytes = 131072

[retention]
max_events = 5000000
max_bytes = 10737418240  # 10GB
```

---

## 🔐 Privacy Controls

### Dashboard UI

The **Privacy & AUTH Controls** panel lets you configure:

1. **Global AUTH Requirement**
   - Force all clients to authenticate before any operations

2. **Per-Kind Protection**
   - 🔒 **Always Protected** (enforced in code):
     - Kind 4: Direct Messages (NIP-04)
     - Kind 17: Private DMs (NIP-17)
     - Kind 1059: Gift Wrap (NIP-59)
   - ⚙️ **Configurable**:
     - Kind 0: Metadata
     - Kind 1: Text Notes
     - Any other event kinds

3. **Whitelist Mode**
   - Only allow writes from approved pubkeys
   - Managed via monitored npubs list
   - Toggle whitelist status per npub

4. **Read Protection**
   - Require AUTH even for REQ (subscriptions)
   - Prevents anonymous event queries

### How AUTH Works

```
Client                 Dashboard Proxy            Backend Relay
  |                           |                          |
  |-------- EVENT (kind 4) -->|                          |
  |                           |-- Check AUTH policy      |
  |                           |-- Not authenticated      |
  |<--- OK [false] "auth-required: kind 4 requires ..." -|
  |                           |                          |
  |-------- AUTH event ------>|                          |
  |                           |-- Validate & track       |
  |                           |------------ AUTH ------->|
  |<------------------------- OK [true] -----------------|
  |                           |                          |
  |-------- EVENT (kind 4) -->|                          |
  |                           |-- Check: authenticated ✓ |
  |                           |-------- EVENT ---------->|
  |<------------------------- OK [true] -----------------|
```

---

## 🎯 Usage Examples

### Adding Upstream Relays

1. Navigate to **Upstream Relays** panel
2. Enter relay URL (e.g., `wss://relay.damus.io`)
3. Optional: Give it a friendly name
4. Click **+ Add Relay**

### Monitoring an Npub

1. Go to **Monitored Npubs** panel
2. Enter npub (e.g., `npub1abc...`)
3. Optional: Add a label/nickname
4. Click **+ Add Npub**
5. Toggle **whitelist** if you want this pubkey to have write access in whitelist mode

### Syncing Events

1. Choose sync mode:
   - **Recent**: Updates since last sync (fast)
   - **Deep**: Last N days (thorough)
   - **Full**: All available history (slow)

2. Optional: Toggle "Sync selected npub only"

3. Click **Sync Now**

4. Watch real-time progress in logs

### Viewing Stored Notes

1. Click on an npub in the **Monitored Npubs** panel
2. View stored notes in the **Notes Archive** panel
3. Click **Load older notes** for pagination

### Enabling Privacy Protection

1. Open **Privacy & AUTH Controls** panel
2. Toggle desired protections:
   - ✅ Require AUTH for text notes (kind 1)
   - ✅ Require AUTH for metadata (kind 0)
   - ✅ Enable whitelist-only mode
   - ✅ Require AUTH for reading

3. Changes apply immediately to new connections

### Backing Up Data

**Manual Backup:**
```
Dashboard → Backup button → Downloads NDJSON file
```

**Automated Backup:**
```
Settings → Toggle "Nightly backup 00:05 local"
Stored in: /app/data/backups/backup-YYYY-MM-DD.ndjson
Retention: Last 7 days kept automatically
```

### Restoring Data

1. Click **Restore** button
2. Select your NDJSON backup file
3. Dashboard imports:
   - Relay configurations
   - Monitored npubs
   - Settings
   - Events
   - Sync state

---

## 🔌 Connecting Clients

### As a Relay

Point your Nostr client to:
```
ws://localhost:7447
```

Or with HTTPS (see below):
```
wss://relay.example.com
```

### Supported NIPs

- ✅ NIP-01: Basic protocol flow
- ✅ NIP-09: Event deletion
- ✅ NIP-11: Relay information document
- ✅ NIP-40: Expiration timestamp
- ✅ NIP-42: Authentication of clients to relays (enhanced)
- 🔜 NIP-50: Search capability (planned)
- 🔜 NIP-65: Relay list metadata (planned)

---

## 🌐 HTTPS Setup

### Option 1: Caddy (Recommended)

1. **Uncomment Caddy service in docker-compose.yml**

2. **Create Caddyfile:**
```
relay.example.com {
    encode zstd gzip
    reverse_proxy nerd-dashboard:8080
}

wss://relay.example.com {
    reverse_proxy nostr-relay:7447
}
```

3. **Update .env:**
```bash
DOMAIN=relay.example.com
```

4. **Restart:**
```bash
docker-compose up -d
```

Caddy automatically obtains and renews SSL certificates!

### Option 2: Manual nginx

```nginx
server {
    listen 443 ssl http2;
    server_name relay.example.com;
    
    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;
    
    # Dashboard
    location / {
        proxy_pass http://localhost:8080;
    }
    
    # Relay WebSocket
    location /ws {
        proxy_pass http://localhost:7447;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

---

## 📊 Monitoring

### Dashboard Stats

- **Active Connections**: Total and authenticated counts
- **Total Events**: Events stored in your relay
- **Relay Status**: Uptime, sync status, last errors

### Logs

**View in dashboard:**
```
Dashboard → Download Logs button
```

**View in Docker:**
```bash
docker-compose logs -f nerd-dashboard
docker-compose logs -f nostr-relay
```

**View on disk:**
```bash
tail -f data/dashboard.log
```

### Database Queries

```bash
# Connect to dashboard database
docker exec -it nerd-dashboard sqlite3 /app/data/dashboard.db

# View active connections
SELECT * FROM v_active_connections;

# Recent AUTH activity
SELECT * FROM v_recent_auth_activity LIMIT 20;

# Most active pubkeys
SELECT * FROM v_connection_stats_by_pubkey ORDER BY total_connections DESC LIMIT 10;

# Current AUTH policy
SELECT * FROM auth_policy ORDER BY key;
```

---

## 🛠️ Troubleshooting

### Port Already in Use

**Error:** `Port 8080 is already in use`

**Solution:** Change port in .env:
```bash
PORT=8081
```

### Backend Connection Failed

**Error:** `Failed to connect to backend relay`

**Solution:** Check nostr-relay is running:
```bash
docker-compose ps nostr-relay
docker-compose logs nostr-relay
```

### Admin Token Not Working

**Symptom:** "Admin token required" errors

**Solution:** 
1. Check NRD_ADMIN_TOKEN is set in .env
2. Clear browser localStorage:
   ```javascript
   localStorage.removeItem('nrdAdminToken')
   ```
3. Re-enter token when prompted

### Sync Not Working

**Check:**
1. Are relays enabled? (toggle On/Off)
2. Are npubs added correctly?
3. Check logs for connection errors:
   ```bash
   docker-compose logs -f nerd-dashboard | grep "SYNC"
   ```

### AUTH Not Enforced

**Verify:**
1. Dashboard policy is enabled
2. Backend relay restarted after config changes
3. Clients support NIP-42

---

## 🔄 Migration from v1.0.5

### Data Preservation

1. **Backup your v1.0.5 data:**
```bash
curl -H "X-Admin-Token: your-token" http://localhost:8080/api/backup > backup-v1.0.5.ndjson
```

2. **Stop v1.0.5:**
```bash
docker-compose down
```

3. **Deploy v0.0.1** (see Quick Start)

4. **Restore backup:**
```
Dashboard → Restore → Select backup-v1.0.5.ndjson
```

All your relays, npubs, settings, and events will be preserved!

### New Features to Configure

After migration, configure privacy features:
- [ ] Set AUTH policies
- [ ] Enable whitelist for monitored npubs
- [ ] Test AUTH flow with a client
- [ ] Enable nightly backups

---

## 📚 Advanced Topics

### Custom Relay Policy

Edit `relay_proxy.rs` to add custom logic:
- IP-based rate limiting
- Content filtering
- Custom AUTH challenges
- Event validation rules

### Metrics & Analytics

Query `relay_stats` table:
```sql
SELECT 
    DATE(timestamp) as date,
    AVG(active_connections) as avg_connections,
    SUM(total_events) as daily_events
FROM relay_stats
GROUP BY date
ORDER BY date DESC
LIMIT 30;
```

### Horizontal Scaling

For high-traffic relays:
1. Run multiple relay instances
2. Use load balancer (HAProxy, nginx)
3. Share PostgreSQL backend (future)
4. Coordinate AUTH policy via dashboard API

---

## 🐛 Bug Reports

Found a bug? Please open an issue:

**Include:**
- Dashboard version (v0.0.1)
- Error logs
- Steps to reproduce
- Expected vs actual behavior

---

## 🤝 Contributing

Contributions welcome!

**Priority areas:**
- NIP-50 search integration
- Performance optimizations
- UI/UX improvements
- Documentation

---

## 📄 License

MIT License - See LICENSE file

---

## 🙏 Credits

- **nostr-rs-relay**: [@scsibug](https://github.com/scsibug/nostr-rs-relay)
- **Nostr Relay Dashboard v1.0.5**: Foundation for this project
- **Wisp**: Inspiration for privacy-first AUTH controls
- **Nostr Community**: For the amazing protocol

---

## 📞 Support

- **GitHub Issues**: Bug reports and feature requests
- **Nostr**: [Your contact npub]
- **Email**: [Your contact email]

---

## 🗺️ Roadmap

### v0.0.2 (Next)
- [ ] NIP-50 full-text search
- [ ] Enhanced analytics dashboard
- [ ] Per-kind rate limiting
- [ ] Content moderation tools

### v0.1.0 (Future)
- [ ] Lightning payment integration
- [ ] NIP-65 relay list recommendations
- [ ] NIP-77 Negentropy sync
- [ ] Multi-user access control

### v1.0.0 (Long-term)
- [ ] Federation protocol
- [ ] Distributed storage
- [ ] AI-powered spam detection
- [ ] Mobile app

---

**Built with ❤️ for the Nostr community**
