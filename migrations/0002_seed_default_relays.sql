-- NERD seed data
-- Migration: 0002_seed_default_relays.sql
--
-- Seeds a default list of well-known public relays.
-- Uses INSERT OR IGNORE so re-running this is safe and users
-- can delete entries without them coming back.
--
-- The "first run" semantics are achieved by the UNIQUE constraint
-- on relays.url combined with INSERT OR IGNORE.

INSERT OR IGNORE INTO relays (url, notes) VALUES
    ('wss://relay.damus.io',          'Damus default relay'),
    ('wss://nos.lol',                 'nos.lol public relay'),
    ('wss://relay.snort.social',      'Snort default relay'),
    ('wss://nostr.wine',              'nostr.wine paid relay (free reads)'),
    ('wss://relay.primal.net',        'Primal public relay'),
    ('wss://relay.nostr.band',        'Nostr.band aggregator relay'),
    ('wss://nostr-pub.wellorder.net', 'Wellorder public relay');
