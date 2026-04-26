use crate::bridge::types::CanonicalCharacter;
use async_trait::async_trait;
use serde_json::Value;

/// A source-specific protocol adapter — one implementation per VTT.
///
/// Sources are stateless transformers: shared connection state
/// (characters map, outbound channel, connected flag) lives in
/// `BridgeState`, so a single `Arc<dyn BridgeSource>` is shared by
/// every connection on its port.
///
/// The `SourceKind` discriminator is stored externally (in the
/// `BridgeState.sources` map key, the `accept_loop` argument, and the
/// canonical characters this source emits) — sources don't need to
/// report it themselves.
#[async_trait]
pub trait BridgeSource: Send + Sync {
    /// Parse an inbound JSON message from this source's wire protocol
    /// and return the canonical character snapshot the frontend should
    /// mirror. Sources that send non-character messages (chat, etc.)
    /// can return an empty Vec.
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<CanonicalCharacter>, String>;

    /// Build an outbound "set attribute" message in this source's wire
    /// format. The `name` and `value` semantics are source-specific —
    /// the frontend treats them as opaque strings. Returns Err if the
    /// source can't translate the (name, value) pair into its wire
    /// shape (e.g. because `value` is a structured payload that fails
    /// to parse for this source).
    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String>;

    /// Build an outbound "refresh" / "resend everything" message.
    fn build_refresh(&self) -> Value;
}
