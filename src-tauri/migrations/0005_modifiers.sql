-- Per-character modifier records. Anchored to (source, source_id) — the same
-- composite key the live bridge cache uses — with no FK to saved_characters
-- so modifiers can attach to live-only OR saved-only OR both characters.
CREATE TABLE IF NOT EXISTS character_modifiers (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    source               TEXT    NOT NULL CHECK(source IN ('roll20','foundry')),
    source_id            TEXT    NOT NULL,
    name                 TEXT    NOT NULL,
    description          TEXT    NOT NULL DEFAULT '',
    effects_json         TEXT    NOT NULL DEFAULT '[]',
    binding_json         TEXT    NOT NULL DEFAULT '{"kind":"free"}',
    tags_json            TEXT    NOT NULL DEFAULT '[]',
    is_active            INTEGER NOT NULL DEFAULT 0,
    is_hidden            INTEGER NOT NULL DEFAULT 0,
    origin_template_id   INTEGER,
    created_at           TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_modifiers_char
    ON character_modifiers(source, source_id);

-- Status templates: GM-authored reusable effect bundles (Slippery, Blind, etc.).
-- Inert in Plan A — Plan B wires the CRUD commands and palette UI.
CREATE TABLE IF NOT EXISTS status_templates (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    effects_json    TEXT    NOT NULL DEFAULT '[]',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);
