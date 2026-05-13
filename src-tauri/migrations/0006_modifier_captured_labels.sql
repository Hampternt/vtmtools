-- Adds the foundry_captured_labels JSON column to character_modifiers.
-- Default '[]' = empty array, meaning "hand-rolled modifier, additive push".
-- Non-empty = "saved override from a Foundry bonus, surgical push".
ALTER TABLE character_modifiers
    ADD COLUMN foundry_captured_labels_json TEXT NOT NULL DEFAULT '[]';
