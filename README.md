# NERD

**A personal archival relay for Nostr.**

NERD is a self-hosted Nostr relay scoped to the npubs you care about. Point any
Nostr client at it over `wss://` and you'll get back your own data — durably
stored, locally owned, and always available.

Your data. Your relay. Your client.

---

## What it is

NERD syncs events from upstream relays for a configured set of npubs and stores
them locally in a SQLite database. It then serves those events back over the
standard Nostr relay protocol (NIP-01), so any Nostr client — Damus, Amethyst,
Primal, Nostrudel, whatever you already use — can connect to it just like any
other relay.

Use it to:

- Back up your own Nostr history so it survives upstream relays going dark
- Pin a friend, family member, or project npub so their notes don't disappear
- Run a private relay you control, with no public write traffic
- Serve a filtered, archival view of your feed to your own clients over Tailscale or LAN

## What it isn't

**NERD is not a public relay.** It's not designed to serve strangers, handle
high write volume, or compete with community relays like `nos.lol` or
`relay.damus.io`.

It's a personal relay you run for yourself, scoped to the npubs you choose,
optimized for durability and archival rather than throughput. If you're looking
for a relay to publish your notes to the wider network, run a public relay
instead — NERD is the thing you point your client at *after* you want a copy
that's yours.

## Status

**v0.0.1 — early development.** 

## Related projects

NERD is part of the [`cryptic-node`](https://github.com/cryptic-node) family of
small, self-hostable Nostr tools:

- **[NRD](https://github.com/cryptic-node/nrd)** — Nostr Relay Downloader. Pulls
  notes from public relays and archives them locally. If NERD is the relay you
  serve your data from, NRD is one of the tools that helps you collect it in
  the first place.

## License

Apache-2.0

## Support development

If NERD is useful to you, Lightning tips are welcome.

*Lightning address orchidcheetah29@primal.net and QR is QR.png*

<p align="center">
  <img src="QR.png" alt="Lightning donation QR code" width="200">
</p>
