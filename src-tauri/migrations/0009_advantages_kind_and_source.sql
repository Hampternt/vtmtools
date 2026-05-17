-- Adds kind and source_attribution columns to advantages.
--
-- kind disambiguates the polymorphic table (Phase 4 storage decision); was
-- previously inferred from tags_json string-matching. tags_json stays for
-- free-form taxonomy ("Feeding", "Social", "Supernatural", "VTM 5e", etc.).
--
-- source_attribution carries FVTT-import provenance (Phase 4 issue #15).
-- NULL = hand-authored locally (whether corebook seed or GM custom).
-- Non-null = imported from a Foundry world; JSON shape:
--   { "source": "foundry", "world_title": "...", "world_id": "...",
--     "system_version": "...", "imported_at": "ISO-8601" }
-- The exact shape is enforced at the application layer, not by SQLite.

ALTER TABLE advantages
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'merit'
    CHECK(kind IN ('merit', 'flaw', 'background', 'boon'));

ALTER TABLE advantages
    ADD COLUMN source_attribution TEXT;

-- Backfill kind from tags_json. Priority cascade: merit > flaw > background
-- > boon. Highest priority wins via CASE WHEN ordering (first match returns).
-- Rows tagged with none of the four kind-strings stay at the default 'merit'.
UPDATE advantages
   SET kind = CASE
       WHEN tags_json LIKE '%"Merit"%'      THEN 'merit'
       WHEN tags_json LIKE '%"Flaw"%'       THEN 'flaw'
       WHEN tags_json LIKE '%"Background"%' THEN 'background'
       WHEN tags_json LIKE '%"Boon"%'       THEN 'boon'
       ELSE 'merit'
   END;
