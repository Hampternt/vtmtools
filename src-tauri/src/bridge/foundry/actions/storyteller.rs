//! Foundry `storyteller.*` helper builders. World-level operations not tied
//! to a single actor.
//!
//! v1 milestone-4 ships exactly one helper: `storyteller.create_world_item` —
//! creates a Foundry-world-level Item doc (feature type) with the
//! given featuretype (merit / flaw / background / boon).

use serde_json::{json, Value};

/// Build a `storyteller.create_world_item { name, featuretype, description, points }`
/// envelope. Validates featuretype against the same enum
/// `actor.create_feature` uses.
pub fn build_create_world_item(
    name: &str,
    featuretype: &str,
    description: &str,
    points: i32,
) -> Result<Value, String> {
    match featuretype {
        "merit" | "flaw" | "background" | "boon" => {}
        other => {
            return Err(format!(
                "foundry/storyteller.create_world_item: invalid featuretype: {other}"
            ));
        }
    }
    Ok(json!({
        "type": "storyteller.create_world_item",
        "name": name,
        "featuretype": featuretype,
        "description": description,
        "points": points,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_world_item_envelope_shape() {
        let out = build_create_world_item("Iron Gullet", "merit", "rancid blood ok", 3)
            .expect("merit is a valid featuretype");
        assert_eq!(out["type"], "storyteller.create_world_item");
        assert_eq!(out["name"], "Iron Gullet");
        assert_eq!(out["featuretype"], "merit");
        assert_eq!(out["description"], "rancid blood ok");
        assert_eq!(out["points"], 3);
    }

    #[test]
    fn create_world_item_invalid_featuretype_returns_err() {
        let err = build_create_world_item("X", "discipline", "", 0)
            .expect_err("discipline is not a valid featuretype");
        assert!(
            err.starts_with("foundry/storyteller.create_world_item: invalid featuretype:"),
            "got: {err}"
        );
    }
}
