//! Canonical-name namespace for character field updates.
//!
//! Single source of truth for the set of names accepted by
//! `character::set_field`. Per-source translators MUST cover ALLOWED_NAMES;
//! `cargo test` enforces coverage.

use crate::bridge::types::{CanonicalCharacter, HealthTrack};
use serde_json::Value;

pub const ALLOWED_NAMES: &[&str] = &[
    "hunger",
    "humanity",
    "humanity_stains",
    "blood_potency",
    "health_superficial",
    "health_aggravated",
    "willpower_superficial",
    "willpower_aggravated",
];

/// Apply a canonical-named field to a typed CanonicalCharacter.
/// Returns Err on unknown name, wrong value type, or out-of-range integer.
pub fn apply_canonical_field(
    c: &mut CanonicalCharacter,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    match name {
        "hunger" => {
            let n = expect_u8_in_range(value, name, 0, 5)?;
            c.hunger = Some(n);
        }
        "humanity" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity = Some(n);
        }
        "humanity_stains" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity_stains = Some(n);
        }
        "blood_potency" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.blood_potency = Some(n);
        }
        "health_superficial" | "health_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.health, name, n);
        }
        "willpower_superficial" | "willpower_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.willpower, name, n);
        }
        other => return Err(format!("character/set_field: unknown field '{other}'")),
    }
    Ok(())
}

fn apply_track_field(track: &mut Option<HealthTrack>, name: &str, n: u8) {
    let t = track.get_or_insert(HealthTrack {
        max: 0,
        superficial: 0,
        aggravated: 0,
    });
    if name.ends_with("_superficial") {
        t.superficial = n;
    } else if name.ends_with("_aggravated") {
        t.aggravated = n;
    }
}

fn expect_u8_in_range(v: &Value, name: &str, lo: u8, hi: u8) -> Result<u8, String> {
    let n = v.as_u64().ok_or_else(|| {
        format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {}",
            type_label(v),
        )
    })?;
    if n > hi as u64 {
        return Err(format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {n}"
        ));
    }
    let n = n as u8;
    if n < lo {
        return Err(format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {n}"
        ));
    }
    Ok(n)
}

fn type_label(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Foundry system-path mapping. Replaces the inline match in
/// `bridge/foundry/mod.rs::canonical_to_path` (delegated in Task 3).
pub fn canonical_to_foundry_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "hunger" => "system.hunger.value",
        "humanity" => "system.humanity.value",
        "humanity_stains" => "system.humanity.stains",
        "blood_potency" => "system.blood.potency",
        "health_superficial" => "system.health.superficial",
        "health_aggravated" => "system.health.aggravated",
        "willpower_superficial" => "system.willpower.superficial",
        "willpower_aggravated" => "system.willpower.aggravated",
        _ => return None,
    })
}

/// Roll20 attribute mapping. v1 returns None for every canonical name —
/// Roll20 live editing of canonical names is deferred to Phase 2.5.
/// Roll20 saved-side editing is unaffected (mutates the typed struct).
pub fn canonical_to_roll20_attr(_name: &str) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::types::SourceKind;

    fn sample() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "x".to_string(),
            name: "T".to_string(),
            controlled_by: None,
            hunger: None,
            health: None,
            willpower: None,
            humanity: None,
            humanity_stains: None,
            blood_potency: None,
            raw: serde_json::json!({}),
        }
    }

    #[test]
    fn apply_hunger_happy_path() {
        let mut c = sample();
        apply_canonical_field(&mut c, "hunger", &serde_json::json!(3)).unwrap();
        assert_eq!(c.hunger, Some(3));
    }

    #[test]
    fn apply_hunger_out_of_range_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "hunger", &serde_json::json!(7))
            .unwrap_err();
        assert!(err.contains("expects integer 0..=5"), "got: {err}");
    }

    #[test]
    fn apply_hunger_wrong_type_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "hunger", &serde_json::json!("3"))
            .unwrap_err();
        assert!(err.contains("got string"), "got: {err}");
    }

    #[test]
    fn apply_unknown_name_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "xyzzy", &serde_json::json!(0))
            .unwrap_err();
        assert!(err.contains("unknown field 'xyzzy'"), "got: {err}");
    }

    #[test]
    fn apply_health_creates_default_track_if_missing() {
        let mut c = sample();
        apply_canonical_field(&mut c, "health_superficial", &serde_json::json!(2))
            .unwrap();
        let t = c.health.unwrap();
        assert_eq!(t.superficial, 2);
        assert_eq!(t.aggravated, 0);
        assert_eq!(t.max, 0);
    }

    #[test]
    fn apply_humanity_stains_happy_path() {
        let mut c = sample();
        apply_canonical_field(&mut c, "humanity_stains", &serde_json::json!(2))
            .unwrap();
        assert_eq!(c.humanity_stains, Some(2));
    }

    #[test]
    fn every_allowed_name_has_foundry_path() {
        for n in ALLOWED_NAMES {
            assert!(
                canonical_to_foundry_path(n).is_some(),
                "missing Foundry path for {n}"
            );
        }
    }

    #[test]
    fn every_allowed_name_applies_via_apply_canonical_field() {
        for n in ALLOWED_NAMES {
            let mut c = sample();
            let v = serde_json::json!(0);
            let res = apply_canonical_field(&mut c, n, &v);
            assert!(
                res.is_ok(),
                "apply_canonical_field rejected '{n}': {:?}",
                res.err()
            );
        }
    }

    #[test]
    fn roll20_attr_stub_returns_none_for_all_names() {
        for n in ALLOWED_NAMES {
            assert!(
                canonical_to_roll20_attr(n).is_none(),
                "v1 stub should return None for {n}"
            );
        }
    }
}
