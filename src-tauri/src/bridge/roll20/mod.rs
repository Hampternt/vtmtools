pub mod translate;
pub mod types;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::bridge::roll20::types::InboundMsg;
use crate::bridge::source::{BridgeSource, InboundEvent};

/// Stateless adapter — parses Roll20 wire messages into canonical characters
/// and builds the outbound counterparts. Shared connection state lives in
/// BridgeState (see `bridge/mod.rs`), not in this struct.
pub struct Roll20Source;

#[async_trait]
impl BridgeSource for Roll20Source {
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String> {
        let parsed: InboundMsg = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        match parsed {
            InboundMsg::Characters { characters } => {
                let canonical: Vec<_> = characters.iter().map(translate::to_canonical).collect();
                Ok(vec![InboundEvent::CharactersSnapshot {
                    source: crate::bridge::types::SourceKind::Roll20,
                    characters: canonical,
                }])
            }
            InboundMsg::CharacterUpdate { character } => {
                let canonical = translate::to_canonical(&character);
                Ok(vec![InboundEvent::CharacterUpdated(canonical)])
            }
        }
    }

    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String> {
        Ok(json!({
            "type": "set_attribute",
            "character_id": source_id,
            "name": name,
            "value": value,
        }))
    }

    fn build_refresh(&self) -> Value {
        json!({ "type": "refresh" })
    }
}
