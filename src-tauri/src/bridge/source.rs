use crate::bridge::types::{CanonicalCharacter, SourceKind};
use async_trait::async_trait;
use serde_json::Value;

/// A source-specific protocol adapter — one implementation per VTT.
///
/// Sources are stateless transformers: shared connection state
/// (characters map, outbound channel, connected flag) lives in
/// `BridgeState`, so a single `Arc<dyn BridgeSource>` is shared by
/// every connection on its port.
#[async_trait]
pub trait BridgeSource: Send + Sync {
    fn kind(&self) -> SourceKind;

    /// Parse an inbound JSON message from this source's wire protocol
    /// and return the canonical character snapshot the frontend should
    /// mirror. Sources that send non-character messages (chat, etc.)
    /// can return an empty Vec.
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<CanonicalCharacter>, String>;

    /// Build an outbound "set attribute" message in this source's wire
    /// format. The `name` and `value` semantics are source-specific —
    /// the frontend treats them as opaque strings.
    fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Value;

    /// Build an outbound "refresh" / "resend everything" message.
    fn build_refresh(&self) -> Value;
}
