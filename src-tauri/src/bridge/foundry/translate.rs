// Foundry WoD5e actor → CanonicalCharacter.
//
// Path constants below come from docs/reference/foundry-vtm5e-paths.md
// (verified against WoD5e v5.3.17, commit d16b5d960a).

use serde_json::Value;

use crate::bridge::foundry::types::FoundryActor;
use crate::bridge::types::{CanonicalCharacter, HealthTrack, SourceKind};

pub fn to_canonical(raw: &FoundryActor) -> CanonicalCharacter {
    let sys = &raw.system;
    CanonicalCharacter {
        source: SourceKind::Foundry,
        source_id: raw.id.clone(),
        name: raw.name.clone(),
        controlled_by: raw.owner.clone(),
        hunger: get_u8(sys, &["hunger", "value"]),
        health: build_health_track(sys, "health"),
        willpower: build_health_track(sys, "willpower"),
        humanity: get_u8(sys, &["humanity", "value"]),
        humanity_stains: get_u8(sys, &["humanity", "stains"]),
        blood_potency: get_u8(sys, &["blood", "potency"]),
        raw: serde_json::to_value(raw).unwrap_or(Value::Null),
    }
}

fn build_health_track(sys: &Value, base: &str) -> Option<HealthTrack> {
    let track = sys.get(base)?.as_object()?;
    Some(HealthTrack {
        max: track.get("max").and_then(value_to_u8).unwrap_or(5),
        superficial: track.get("superficial").and_then(value_to_u8).unwrap_or(0),
        aggravated: track.get("aggravated").and_then(value_to_u8).unwrap_or(0),
    })
}

fn get_u8(sys: &Value, path: &[&str]) -> Option<u8> {
    let mut cur = sys;
    for seg in path {
        cur = cur.get(*seg)?;
    }
    value_to_u8(cur)
}

fn value_to_u8(v: &Value) -> Option<u8> {
    match v {
        Value::Number(n) => n.as_u64().and_then(|x| u8::try_from(x).ok()),
        Value::String(s) => s.parse::<u8>().ok(),
        _ => None,
    }
}
