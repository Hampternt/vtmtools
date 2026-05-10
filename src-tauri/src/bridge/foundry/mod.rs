pub mod actions;
pub mod translate;
mod translate_roll;
pub mod types;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::bridge::foundry::actions::actor;
use crate::bridge::foundry::types::FoundryInbound;
use crate::bridge::source::{BridgeSource, InboundEvent};

/// Stateless adapter for the FoundryVTT WoD5e module. Translates
/// Foundry actor data into the canonical bridge shape and builds
/// outbound messages the module knows how to apply via actor.update().
pub struct FoundrySource;

#[async_trait]
impl BridgeSource for FoundrySource {
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String> {
        let parsed: FoundryInbound = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        let actors = match parsed {
            FoundryInbound::Actors { actors } => actors,
            FoundryInbound::ActorUpdate { actor } => vec![actor],
            // Hello metadata is captured pre-trait in bridge::handle_connection;
            // the trait method just returns no characters. Same for Error: the
            // pre-trait layer routes errors as Tauri events; this arm is
            // exhaustiveness completeness only.
            FoundryInbound::Hello { .. } => return Ok(vec![]),
            FoundryInbound::Error { .. } => return Ok(vec![]),
            FoundryInbound::RollResult { message } => {
                let canonical = translate_roll::to_canonical_roll(&message);
                return Ok(vec![InboundEvent::RollReceived(canonical)]);
            }
        };
        let canonical: Vec<_> = actors.iter().map(translate::to_canonical).collect();
        Ok(vec![InboundEvent::CharactersUpdated(canonical)])
    }

    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String> {
        match name {
            "resonance" => Ok(actor::build_create_item_simple(source_id, "resonance", value)),
            "dyscrasia" => actor::build_apply_dyscrasia(source_id, value),
            _ => {
                let path = canonical_to_path(name);
                Ok(actor::build_update_field(source_id, &path, parse_value(value)))
            }
        }
    }

    fn build_refresh(&self) -> Value {
        json!({ "type": "refresh" })
    }
}

fn canonical_to_path(name: &str) -> String {
    if let Some(p) = crate::shared::canonical_fields::canonical_to_foundry_path(name) {
        return p;
    }
    if name.starts_with("system.") {
        return name.to_string();
    }
    name.to_string()
}

fn parse_value(s: &str) -> Value {
    if let Ok(n) = s.parse::<i64>() {
        Value::from(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::from(f)
    } else if s == "true" {
        Value::from(true)
    } else if s == "false" {
        Value::from(false)
    } else {
        Value::from(s)
    }
}
