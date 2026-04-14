# NERD (Nostr Enhanced Relay Dashboard) v0.0.1

NERD v0.0.1 is a practical baseline for operating a **curated nostr-rs-relay sidecar** behind one hostname.

## What this release delivers

- Separate-process architecture:
  - **NRD/NERD** (operator dashboard + policy engine)
  - **NRR** (`scsibug/nostr-rs-relay`) for client-facing relay traffic
- Single-hostname edge routing via Caddy (`/` WS + NIP-11 to NRR; everything else to NERD).
- New relay control API namespace under `/api/relay/*`.
- Rule-driven export bridge (`nrd.db` -> `nostr.db`) using SQLite `ATTACH` + `INSERT OR IGNORE`.
- Schema guard for NRR SQLite schema range checks (fail-closed bridge behavior).
- Runtime state + export ledger tables for observability and safe future pruning.
- Basic operator UI (`public/index.html`) for relay status and export-rule creation.

## Repo layout

```text
.
├── docker-compose.yml
├── deploy/
│   ├── Caddyfile
│   └── nrr-config.toml
├── public/
│   └── index.html
└── src/
    ├── main.rs
    ├── config.rs
    ├── app_state.rs
    ├── db/
    ├── relay/
    └── routes/
```

## Quick start

1. Copy `.env.example` to `.env` and set `NRD_ADMIN_TOKEN`.
2. Ensure Caddy runs on host with `deploy/Caddyfile` semantics.
3. Start services:
   ```bash
   docker compose up --build -d
   ```
4. Open dashboard via `https://relay.swallow-liberty.ts.net`.

## Implemented APIs (`/api/relay`)

Read-only:
- `GET /status`
- `GET /compat`
- `GET /summary`
- `GET /export-rules`
- `GET /export/preview`

Admin-token required:
- `GET /config`
- `POST /config`
- `POST /export/run`
- `POST /observe/run`
- `POST /export-rules`
- `PUT /export-rules/:id`
- `DELETE /export-rules/:id`
- `POST /restart` (stub for controlled restart integration)
- `POST /writers` and `DELETE /writers/:pubkey` (stubs for whitelist management)
- `POST /prune` (stub with ledger-safe intent)
- `GET /logs` (stub)

## Notes on v0.0.1 scope

This release provides a complete Phase 0-2 skeleton with working bridge primitives, rule CRUD, and compatibility checks. Some control-plane actions are intentionally explicit placeholders (restart, whitelist mutation, prune execution) so they can be wired to your preferred security model (Docker socket proxy vs companion restarter) without hidden privilege escalation.

## Ops checks

```bash
curl -H "Accept: application/nostr+json" https://relay.swallow-liberty.ts.net
curl -s -o /dev/null -w "%{http_code}" https://relay.swallow-liberty.ts.net
echo '["REQ","check",{"limit":1}]' | websocat wss://relay.swallow-liberty.ts.net
```
