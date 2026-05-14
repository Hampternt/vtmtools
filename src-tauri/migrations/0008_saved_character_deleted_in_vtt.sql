-- Adds deleted_in_vtt_at column to saved_characters. ISO-8601 timestamp
-- (matches saved_at / last_updated_at format) recorded when the bridge
-- layer observed the live actor disappearing from its source VTT. NULL
-- means "not known to be deleted" — either never deleted, or the
-- deletion happened before this column existed.
--
-- Set by: bridge::accept_loop on CharacterRemoved events AND by
-- snapshot reconciliation when a saved row's source_id is absent from
-- a fresh CharactersSnapshot of the same Foundry world.
-- Cleared by: an explicit CharacterUpdated event for the same key, or
-- by snapshot reconciliation seeing the source_id reappear.
-- Owned entirely by the bridge layer; save_character /
-- update_saved_character do not touch this column.

ALTER TABLE saved_characters
    ADD COLUMN deleted_in_vtt_at TEXT;
