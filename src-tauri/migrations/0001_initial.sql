CREATE TABLE IF NOT EXISTS dyscrasias (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resonance_type  TEXT    NOT NULL CHECK(resonance_type IN ('Phlegmatic','Melancholy','Choleric','Sanguine')),
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL,
    bonus           TEXT    NOT NULL,
    is_custom       INTEGER NOT NULL DEFAULT 0
);
