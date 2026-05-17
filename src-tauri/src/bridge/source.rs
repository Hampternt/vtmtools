use crate::bridge::types::{CanonicalCharacter, CanonicalRoll};
use async_trait::async_trait;
use serde_json::Value;

/// One event emitted from a single inbound frame. A frame may yield
/// zero, one, or many events.
#[derive(Debug, Clone)]
pub enum InboundEvent {
    /// Source pushed a full bulk snapshot. The bridge cache replaces
    /// this source's slice — every entry whose `source` matches is
    /// dropped, then `characters` are inserted. Empty `characters` is
    /// legal and means "this source now has zero characters".
    CharactersSnapshot {
        source: crate::bridge::types::SourceKind,
        characters: Vec<CanonicalCharacter>,
    },
    /// One character was added or changed. The bridge cache inserts or
    /// overwrites a single entry keyed by `(source, source_id)`.
    CharacterUpdated(CanonicalCharacter),
    /// One character was removed from its source. The bridge cache
    /// evicts the entry keyed by `(source, source_id)`.
    CharacterRemoved {
        source: crate::bridge::types::SourceKind,
        source_id: String,
    },
    /// Source pushed a roll result.
    RollReceived(CanonicalRoll),
    /// Foundry-side item deletion — frontend modifier rows tied to
    /// this item must be reaped. Caller in `bridge::mod` runs the DB
    /// delete and emits `modifiers://rows-reaped`.
    ItemDeleted {
        source: crate::bridge::types::SourceKind,
        source_id: String,
        item_id: String,
    },
    /// Source pushed a full world-level item snapshot. The bridge cache
    /// replaces this source's world-items slice — every entry whose
    /// `source` matches is dropped, then `items` are inserted. Empty
    /// `items` is legal and means "this source now has zero
    /// world-level items".
    WorldItemsSnapshot {
        source: crate::bridge::types::SourceKind,
        items: Vec<crate::bridge::types::CanonicalWorldItem>,
    },
    /// One world-level item was added or changed. The bridge cache
    /// inserts or overwrites a single entry keyed by `(source, id)`.
    WorldItemUpsert {
        source: crate::bridge::types::SourceKind,
        item: crate::bridge::types::CanonicalWorldItem,
    },
    /// One world-level item was removed. The bridge cache evicts the
    /// entry keyed by `(source, id)`.
    WorldItemDeleted {
        source: crate::bridge::types::SourceKind,
        item_id: String,
    },
}

/// Per-source protocol adapter. Sources are stateless transformers;
/// shared state (cache, outbound channel, connected flag, roll history)
/// lives in `BridgeState`.
#[async_trait]
pub trait BridgeSource: Send + Sync {
    /// Parse one inbound JSON frame into zero or more events.
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String>;

    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String>;

    fn build_refresh(&self) -> Value;
}
