# NERD — Nostr Enhanced Relay Dashboard

Rust-based successor to [NRD](https://github.com/cryptic-node/nostr-relay-dashboard).
A read-only Nostr relay dashboard and reader-archiver for backing up `npub` data
across multiple relays.

> **Status:** v0.0.1 — pre-release / under active development.
> NRD parity only. Write relay, NIP-42 auth, and embedded `nostr-rs-relay`
> are deferred to v0.1.0+.

---

## Features (v0.0.1)

- Background sync worker — subscribes to configured relays for configured npubs
- Read-only NIP-01 relay endpoint — query archived events via WebSocket
- Minimal admin UI — manage npubs and relays at runtime
- Single static binary or Docker image

---

## Quick start

### From source

```bash
git clone https://github.com/cryptic-node/nostr-enhanced-relay-dashboard
cd nostr-enhanced-relay-dashboard
cargo run --release
```

NERD will:
1. Read `nerd.toml` (create one from the bundled default if missing)
2. Open `./data/nerd.db` (creating it on first run)
3. Apply migrations and seed a default relay list
4. Bind to `0.0.0.0:8082`

Visit <http://localhost:8082/health> to confirm it's running.

### Docker

_(Coming in v0.0.1 final — Session 5 deliverable.)_

---

## Configuration

NERD reads `nerd.toml` at startup. Any value can be overridden by an
environment variable prefixed with `NERD_`, using `__` as a section
separator:

```bash
NERD_SERVER__PORT=8083 cargo run
NERD_DATABASE__PATH=/var/lib/nerd/nerd.db cargo run
```

Set `RUST_LOG` to override `logging.filter` at runtime:

```bash
RUST_LOG=debug cargo run
```

---

## Data and security

NERD stores all relay URLs and npubs in its SQLite database. The schema
includes an `encrypted` column reserved for future use, but **app-layer
encryption is not implemented in v0.0.1**.

If your relay list contains sensitive endpoints (e.g. private Tailscale
subdomains, home-IP-exposing URLs), use **full-disk encryption on the
host**. This is a deliberate choice: app-layer encryption requires key
management that conflicts with headless deployment.

---

## Coexisting with NRD

NERD is designed to run alongside NRD on the same host without conflict:

- Default port: `8082` (NRD uses `8080`)
- Default data dir: `./data/` (NRD has its own, untouched)
- No shared state, no shared database

You can run both indefinitely. A migration tool to import NRD's archive
into NERD is planned post-v0.0.1.

---

## License

Apache-2.0
