use crate::bridge::types::{CanonicalCharacter, CanonicalRoll};
use async_trait::async_trait;
use serde_json::Value;

/// One event emitted from a single inbound frame. A frame may yield zero,
/// one, or many events — e.g. `actors` snapshot is one CharactersUpdated;
/// a `hello` frame yields nothing.
#[derive(Debug, Clone)]
pub enum InboundEvent {
    /// Source pushed an updated set of characters. Frontend re-renders
    /// from the merged cache; an empty Vec is a legal "no characters
    /// attached to this message" — not a clear signal.
    CharactersUpdated(Vec<CanonicalCharacter>),
    /// Source pushed a roll result.
    RollReceived(CanonicalRoll),
    /// Foundry-side item deletion — frontend modifier rows tied to this
    /// item must be reaped. Caller in `bridge::mod` runs the DB delete and
    /// emits `modifiers://rows-reaped`. Spec §5.2.
    ItemDeleted {
        source: crate::bridge::types::SourceKind,
        source_id: String,
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
