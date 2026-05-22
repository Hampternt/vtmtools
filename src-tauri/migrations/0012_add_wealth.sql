-- Wealth tracking subsystem (V1).
-- Per-character opt-in. Row presence in character_wallets == opted in.
-- See docs/superpowers/specs/2026-05-20-wealth-tracking-design.md

CREATE TABLE IF NOT EXISTS character_wallets (
    character_id          INTEGER PRIMARY KEY
                                  REFERENCES characters(id) ON DELETE CASCADE,
    liquid                REAL    NOT NULL DEFAULT 0,
    banked                REAL    NOT NULL DEFAULT 0,
    currency_name         TEXT    NOT NULL DEFAULT '$',
    transfer_delay_json   TEXT,
    created_at            TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS character_assets (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id          INTEGER NOT NULL
                                  REFERENCES character_wallets(character_id) ON DELETE CASCADE,
    name                  TEXT    NOT NULL,
    category              TEXT    CHECK(category IN ('weapon','vehicle','property','investment','misc')),
    class                 TEXT,
    purchase_price        REAL,
    current_value         REAL    NOT NULL DEFAULT 0,
    condition             TEXT    CHECK(condition IN ('new','used','very_used','custom')),
    custom_modifier_json  TEXT,
    notes                 TEXT,
    created_at            TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_character_assets_character_id
    ON character_assets(character_id);

CREATE TABLE IF NOT EXISTS item_slots (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    category              TEXT    NOT NULL
                                  CHECK(category IN ('weapon','vehicle','property','investment','misc')),
    class                 TEXT    NOT NULL,
    label                 TEXT    NOT NULL,
    base_price_min        REAL    NOT NULL,
    base_price_max        REAL    NOT NULL,
    examples              TEXT,
    is_custom             INTEGER NOT NULL DEFAULT 0,
    source                TEXT,
    UNIQUE(category, class)
);
