pub mod translate;
pub mod types;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::bridge::foundry::types::FoundryInbound;
use crate::bridge::source::BridgeSource;
use crate::bridge::types::{CanonicalCharacter, SourceKind};

/// Stateless adapter for the FoundryVTT WoD5e module. Translates
/// Foundry actor data into the canonical bridge shape and builds
/// outbound messages the module knows how to apply via actor.update().
pub struct FoundrySource;

#[async_trait]
impl BridgeSource for FoundrySource {
    fn kind(&self) -> SourceKind {
        SourceKind::Foundry
    }

    async fn handle_inbound(&self, msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
        let parsed: FoundryInbound = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        let actors = match parsed {
            FoundryInbound::Actors { actors } => actors,
            FoundryInbound::ActorUpdate { actor } => vec![actor],
            FoundryInbound::Hello => return Ok(vec![]),
        };
        Ok(actors.iter().map(translate::to_canonical).collect())
    }

    fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Value {
        // Maps the canonical attribute name to a Foundry operation.
        // Most fields are simple actor.update() calls; resonance is special
        // because WoD5e stores it as an Item document, not a system field.
        // See docs/reference/foundry-vtm5e-paths.md.
        match name {
            "resonance" => json!({
                "type": "create_item",
                "actor_id": source_id,
                "item_type": "resonance",
                "item_name": value,
                "replace_existing": true,
            }),
            _ => {
                let path = canonical_to_path(name);
                json!({
                    "type": "update_actor",
                    "actor_id": source_id,
                    "path": path,
                    "value": parse_value(value),
                })
            }
        }
    }

    fn build_refresh(&self) -> Value {
        json!({ "type": "refresh" })
    }
}

fn canonical_to_path(name: &str) -> String {
    match name {
        "hunger" => "system.hunger.value",
        "humanity" => "system.humanity.value",
        "humanity_stains" => "system.humanity.stains",
        "blood_potency" => "system.blood.potency",
        "health_superficial" => "system.health.superficial",
        "health_aggravated" => "system.health.aggravated",
        "willpower_superficial" => "system.willpower.superficial",
        "willpower_aggravated" => "system.willpower.aggravated",
        // Pass-through for already-qualified paths (e.g. when a tool wants
        // to write a niche field the canonical mapping doesn't cover).
        other if other.starts_with("system.") => other,
        other => other,
    }
    .to_string()
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
