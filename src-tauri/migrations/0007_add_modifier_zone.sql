-- Adds the `zone` column to character_modifiers. Two values:
--   'character'   = merits/flaws/items/character-flavored modifiers (default)
--   'situational' = scene/world modifiers (slippery, dark, cursed, etc.)
--
-- Backfill: any existing row with origin_template_id IS NOT NULL came from a
-- status-template application — those are semantically situational. Without
-- the backfill, upgrade users would have to manually drag every template-
-- applied card from Character into Situational on first run after upgrade.

ALTER TABLE character_modifiers
    ADD COLUMN zone TEXT NOT NULL DEFAULT 'character'
    CHECK(zone IN ('character', 'situational'));

UPDATE character_modifiers
   SET zone = 'situational'
 WHERE origin_template_id IS NOT NULL;
