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

/// Legacy 8 flat canonical names — duplicates ALLOWED_NAMES under a clearer
/// name so the three arrays (FLAT_NAMES + ATTRIBUTE_NAMES + SKILL_NAMES) form
/// the full v2 surface. ALLOWED_NAMES is kept for backward compatibility with
/// any existing callers that iterate it.
pub const FLAT_NAMES: &[&str] = ALLOWED_NAMES;

/// WoD5e v5.3.17 attribute keys (system.attributes.<key>.value).
/// Mirrors src/lib/foundry/canonical-names.ts::FOUNDRY_ATTRIBUTE_NAMES.
/// When changing this list, update the TS array in the same commit.
pub const ATTRIBUTE_NAMES: &[&str] = &[
    "charisma",
    "composure",
    "dexterity",
    "intelligence",
    "manipulation",
    "resolve",
    "stamina",
    "strength",
    "wits",
];

/// WoD5e v5.3.17 skill keys (system.skills.<key>.value).
/// Mirrors src/lib/foundry/canonical-names.ts::FOUNDRY_SKILL_NAMES.
pub const SKILL_NAMES: &[&str] = &[
    "academics",
    "animalken",
    "athletics",
    "awareness",
    "brawl",
    "craft",
    "drive",
    "etiquette",
    "finance",
    "firearms",
    "insight",
    "intimidation",
    "investigation",
    "larceny",
    "leadership",
    "medicine",
    "melee",
    "occult",
    "performance",
    "persuasion",
    "politics",
    "science",
    "stealth",
    "streetwise",
    "subterfuge",
    "survival",
    "technology",
];

/// Returns true if `name` is in the v2 canonical-name surface:
///   - one of the legacy 8 flat names (FLAT_NAMES), OR
///   - `attribute.<key>` where `<key>` is in ATTRIBUTE_NAMES, OR
///   - `skill.<key>` where `<key>` is in SKILL_NAMES.
///
/// Use this at the router instead of `ALLOWED_NAMES.contains(...)` — the
/// const can't grow inline (no const-fn array concat in stable Rust).
pub fn is_allowed_name(name: &str) -> bool {
    if FLAT_NAMES.contains(&name) {
        return true;
    }
    if let Some(rest) = name.strip_prefix("attribute.") {
        return ATTRIBUTE_NAMES.contains(&rest);
    }
    if let Some(rest) = name.strip_prefix("skill.") {
        return SKILL_NAMES.contains(&rest);
    }
    false
}

/// Apply a canonical-named field to a typed CanonicalCharacter.
/// Returns Err on unknown name, wrong value type, or out-of-range integer.
pub fn apply_canonical_field(
    c: &mut CanonicalCharacter,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    // Existing 8 flat-name arms.
    match name {
        "hunger" => {
            let n = expect_u8_in_range(value, name, 0, 5)?;
            c.hunger = Some(n);
            return Ok(());
        }
        "humanity" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity = Some(n);
            return Ok(());
        }
        "humanity_stains" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity_stains = Some(n);
            return Ok(());
        }
        "blood_potency" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.blood_potency = Some(n);
            return Ok(());
        }
        "health_superficial" | "health_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.health, name, n);
            return Ok(());
        }
        "willpower_superficial" | "willpower_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.willpower, name, n);
            return Ok(());
        }
        _ => {}
    }

    // New namespaced arms: attribute.<key>, skill.<key>.
    if let Some(key) = name.strip_prefix("attribute.") {
        return apply_attribute(c, key, value);
    }
    if let Some(key) = name.strip_prefix("skill.") {
        return apply_skill(c, key, value);
    }

    Err(format!("character/set_field: unknown field '{name}'"))
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

/// Walk `raw` by JSON pointer-path segments and overwrite the leaf with `n`.
/// Creates intermediate objects as needed so a saved-side write succeeds even
/// for actors whose raw blob hasn't seen this skill/attribute before.
///
/// `pointer` MUST start with '/' and use '/' as the segment delimiter
/// (RFC 6901 JSON Pointer syntax — same as serde_json's Value::pointer).
///
/// Returns Err if a non-leaf segment exists but is not a JSON object
/// (e.g. trying to walk into a string at /system/attributes when the actor's
/// raw has system.attributes as "broken" — defensive; should never happen
/// with valid Foundry payloads).
fn set_raw_u8(raw: &mut Value, pointer: &str, n: u8) -> Result<(), String> {
    if !pointer.starts_with('/') {
        return Err(format!(
            "character/set_field: invalid pointer '{pointer}' (must start with '/')"
        ));
    }
    let segments: Vec<&str> = pointer[1..].split('/').collect();
    if segments.is_empty() {
        return Err("character/set_field: empty pointer".into());
    }

    // Ensure root is an object.
    if !raw.is_object() {
        *raw = Value::Object(serde_json::Map::new());
    }

    // Walk all but the last segment, creating empty objects as we go.
    let mut cur = raw;
    for seg in &segments[..segments.len() - 1] {
        let obj = cur.as_object_mut().ok_or_else(|| {
            format!("character/set_field: pointer '{pointer}' walks into non-object")
        })?;
        if !obj.contains_key(*seg) {
            obj.insert(seg.to_string(), Value::Object(serde_json::Map::new()));
        }
        // Re-borrow for the next iteration. Unwrap is safe — we just inserted
        // (or it already existed); object_mut may still fail if existing was
        // not an object, which is the defensive Err above on next iteration.
        cur = obj.get_mut(*seg).unwrap();
    }

    // Set the leaf.
    let leaf_obj = cur.as_object_mut().ok_or_else(|| {
        format!("character/set_field: pointer '{pointer}' walks into non-object at leaf parent")
    })?;
    leaf_obj.insert(segments.last().unwrap().to_string(), Value::from(n as u64));
    Ok(())
}

/// Apply `attribute.<key>` write — validates `key` against ATTRIBUTE_NAMES,
/// range-checks the value 0..=5 (WoD5e dot rating), then writes via JSON
/// pointer to /system/attributes/<key>/value.
fn apply_attribute(c: &mut CanonicalCharacter, key: &str, value: &Value) -> Result<(), String> {
    if !ATTRIBUTE_NAMES.contains(&key) {
        return Err(format!("character/set_field: unknown attribute '{key}'"));
    }
    let display_name = format!("attribute.{key}");
    let n = expect_u8_in_range(value, &display_name, 0, 5)?;
    let pointer = format!("/system/attributes/{key}/value");
    set_raw_u8(&mut c.raw, &pointer, n)
}

/// Apply `skill.<key>` write — same shape as apply_attribute but for
/// /system/skills/<key>/value.
fn apply_skill(c: &mut CanonicalCharacter, key: &str, value: &Value) -> Result<(), String> {
    if !SKILL_NAMES.contains(&key) {
        return Err(format!("character/set_field: unknown skill '{key}'"));
    }
    let display_name = format!("skill.{key}");
    let n = expect_u8_in_range(value, &display_name, 0, 5)?;
    let pointer = format!("/system/skills/{key}/value");
    set_raw_u8(&mut c.raw, &pointer, n)
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

    #[test]
    fn is_allowed_name_accepts_legacy_flat_names() {
        for n in FLAT_NAMES {
            assert!(is_allowed_name(n), "should accept legacy flat name '{n}'");
        }
    }

    #[test]
    fn is_allowed_name_accepts_namespaced_attributes() {
        for n in ATTRIBUTE_NAMES {
            let full = format!("attribute.{n}");
            assert!(is_allowed_name(&full), "should accept '{full}'");
        }
    }

    #[test]
    fn is_allowed_name_accepts_namespaced_skills() {
        for n in SKILL_NAMES {
            let full = format!("skill.{n}");
            assert!(is_allowed_name(&full), "should accept '{full}'");
        }
    }

    #[test]
    fn is_allowed_name_rejects_unknown_attribute_key() {
        assert!(!is_allowed_name("attribute.foo"));
        assert!(!is_allowed_name("attribute."));
    }

    #[test]
    fn is_allowed_name_rejects_unknown_skill_key() {
        assert!(!is_allowed_name("skill.bar"));
        assert!(!is_allowed_name("skill."));
    }

    #[test]
    fn is_allowed_name_rejects_unknown_flat_name() {
        assert!(!is_allowed_name("xyzzy"));
        assert!(!is_allowed_name(""));
    }

    #[test]
    fn flat_names_match_legacy_allowed_names() {
        // FLAT_NAMES must equal the existing ALLOWED_NAMES (legacy 8 names) —
        // this test pins the equivalence so future-you doesn't accidentally
        // diverge them.
        assert_eq!(FLAT_NAMES, ALLOWED_NAMES);
    }

    #[test]
    fn apply_attribute_strength_writes_into_raw() {
        let mut c = sample();
        apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!(3))
            .expect("happy path");
        let v = c
            .raw
            .pointer("/system/attributes/strength/value")
            .expect("raw pointer exists");
        assert_eq!(v, &serde_json::json!(3));
    }

    #[test]
    fn apply_skill_brawl_writes_into_raw() {
        let mut c = sample();
        apply_canonical_field(&mut c, "skill.brawl", &serde_json::json!(2))
            .expect("happy path");
        let v = c
            .raw
            .pointer("/system/skills/brawl/value")
            .expect("raw pointer exists");
        assert_eq!(v, &serde_json::json!(2));
    }

    #[test]
    fn apply_attribute_unknown_key_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "attribute.foo", &serde_json::json!(1))
            .unwrap_err();
        assert!(
            err.contains("unknown attribute 'foo'"),
            "got: {err}"
        );
    }

    #[test]
    fn apply_skill_unknown_key_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "skill.bar", &serde_json::json!(1))
            .unwrap_err();
        assert!(err.contains("unknown skill 'bar'"), "got: {err}");
    }

    #[test]
    fn apply_attribute_out_of_range_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!(6))
            .unwrap_err();
        assert!(err.contains("expects integer 0..=5"), "got: {err}");
    }

    #[test]
    fn apply_attribute_wrong_type_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!("3"))
            .unwrap_err();
        assert!(err.contains("got string"), "got: {err}");
    }

    #[test]
    fn apply_attribute_overwrites_existing_raw_value() {
        let mut c = sample();
        c.raw = serde_json::json!({
            "system": { "attributes": { "strength": { "value": 1 } } }
        });
        apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!(4))
            .expect("happy path");
        assert_eq!(
            c.raw.pointer("/system/attributes/strength/value"),
            Some(&serde_json::json!(4))
        );
    }

    #[test]
    fn apply_skill_creates_intermediate_objects_when_missing() {
        // sample() raw is `{}` — fully missing system/skills/<key>/value path.
        // set_raw_u8 must create intermediate objects without erroring.
        let mut c = sample();
        apply_canonical_field(&mut c, "skill.occult", &serde_json::json!(5))
            .expect("must create intermediate objects");
        assert_eq!(
            c.raw.pointer("/system/skills/occult/value"),
            Some(&serde_json::json!(5))
        );
    }
}
