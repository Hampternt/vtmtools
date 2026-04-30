-- Saved characters: local snapshots of bridged characters, durable across
-- sessions. Distinct from the in-memory bridge cache (which holds live
-- character data and is reset on each connect cycle).
CREATE TABLE IF NOT EXISTS saved_characters (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    source             TEXT    NOT NULL CHECK(source IN ('roll20','foundry')),
    source_id          TEXT    NOT NULL,
    foundry_world      TEXT,
    name               TEXT    NOT NULL,
    canonical_json     TEXT    NOT NULL,
    saved_at           TEXT    NOT NULL DEFAULT (datetime('now')),
    last_updated_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE (source, source_id)
);
