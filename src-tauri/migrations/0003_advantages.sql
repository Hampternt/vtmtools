-- Local library of VTM 5e Merits, Backgrounds, and Flaws (collectively Advantages).
-- Per-row attributes (level, min_level, max_level, source, prereq, …) live inside
-- properties_json, mirroring nodes.properties_json so the future character builder
-- can consume advantages natively.
CREATE TABLE IF NOT EXISTS advantages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    is_custom       INTEGER NOT NULL DEFAULT 0
);
