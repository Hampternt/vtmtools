// Verified 2026-05-10 against foundry-vtm5e-roll-sample.json: terms[] is flat,
// difficulty at roll.options.difficulty (absent → None), timestamp is ms-epoch number,
// message ID at _id field.
//
// Foundry WoD5e roll → CanonicalRoll.
//
// Foundry does NOT persist rollMessageData (totals, criticals, messy, bestial)
// on the ChatMessage — see docs/reference/foundry-vtm5e-rolls.md §"Chat message shape".
// The translator recomputes those classifications from basic_results + advanced_results.

use crate::bridge::foundry::types::FoundryRollMessage;
use crate::bridge::types::{CanonicalRoll, RollSplat, SourceKind};

pub fn to_canonical_roll(raw: &FoundryRollMessage) -> CanonicalRoll {
    let splat = parse_splat(&raw.splat, &raw.formula);
    let criticals = count_criticals(&raw.basic_results, &raw.advanced_results, splat);
    let messy = matches!(splat, RollSplat::Vampire) && raw.advanced_results.iter().any(|&d| d == 10);
    let bestial = matches!(splat, RollSplat::Vampire) && raw.advanced_results.iter().any(|&d| d == 1);
    let brutal = matches!(splat, RollSplat::Werewolf) && raw.advanced_results.iter().any(|&d| d == 1);

    CanonicalRoll {
        source: SourceKind::Foundry,
        source_id: raw.message_id.clone(),
        actor_id: raw.actor_id.clone(),
        actor_name: raw.actor_name.clone(),
        timestamp: raw.timestamp.clone(),
        splat,
        flavor: raw.flavor.clone(),
        formula: raw.formula.clone(),
        basic_results: raw.basic_results.clone(),
        advanced_results: raw.advanced_results.clone(),
        total: raw.total,
        difficulty: raw.difficulty,
        criticals,
        messy,
        bestial,
        brutal,
        raw: raw.raw.clone(),
    }
}

/// Defensive: trust the JS-side splat string when it's a known value, else
/// re-detect from formula via regex per docs/reference/foundry-vtm5e-rolls.md
/// §"Splat detection". `roll.system` is unreliable post-rehydration so the
/// formula is the robust signal.
fn parse_splat(js_splat: &str, formula: &str) -> RollSplat {
    match js_splat {
        "mortal" => RollSplat::Mortal,
        "vampire" => RollSplat::Vampire,
        "werewolf" => RollSplat::Werewolf,
        "hunter" => RollSplat::Hunter,
        _ => detect_splat_from_formula(formula),
    }
}

fn detect_splat_from_formula(formula: &str) -> RollSplat {
    // Order matters: vampire and werewolf both have advanced dice (g/r),
    // so check the more specific basic+advanced pairings first.
    if has_die(formula, 'v') || has_die(formula, 'g') {
        RollSplat::Vampire
    } else if has_die(formula, 'w') || has_die(formula, 'r') {
        RollSplat::Werewolf
    } else if has_die(formula, 'h') || has_die(formula, 's') {
        RollSplat::Hunter
    } else if has_die(formula, 'm') {
        RollSplat::Mortal
    } else {
        RollSplat::Unknown
    }
}

/// Match an `Ndx` pattern in a Foundry roll formula. `N` is a digit count;
/// `x` is the die-class letter. The formula structure is what we trust:
/// real formulas always have a digit-count immediately before the `d`. Whatever
/// follows the denom (whitespace, `cs>5`, `+`, end-of-string) doesn't matter
/// because the digit-prefix is the load-bearing invariant — denominations are
/// always single lowercase letters in WoD5e.
fn has_die(formula: &str, denom: char) -> bool {
    let pat: [u8; 2] = [b'd', denom as u8];
    let bytes = formula.as_bytes();
    if bytes.len() < 2 { return false; }
    for i in 0..=bytes.len() - 2 {
        if bytes[i] == pat[0] && bytes[i + 1] == pat[1] {
            // The digit-prefix invariant: byte before `d` must be a digit.
            // (Or the formula is malformed and we don't accept it.)
            if i > 0 && bytes[i - 1].is_ascii_digit() {
                return true;
            }
        }
    }
    false
}

/// Count critical successes — natural 10s on basic dice plus natural 10s on
/// hunger dice (latter are messy crits, counted in both `criticals` and `messy`).
/// Werewolf rage 10s and hunter desperation 10s are counted as criticals too.
fn count_criticals(basic: &[u8], advanced: &[u8], splat: RollSplat) -> u32 {
    let basic_tens = basic.iter().filter(|&&d| d == 10).count() as u32;
    let advanced_tens = match splat {
        RollSplat::Vampire | RollSplat::Werewolf | RollSplat::Hunter => {
            advanced.iter().filter(|&&d| d == 10).count() as u32
        }
        _ => 0,
    };
    basic_tens + advanced_tens
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_msg(splat: &str, formula: &str, basic: Vec<u8>, advanced: Vec<u8>) -> FoundryRollMessage {
        FoundryRollMessage {
            message_id: "msg_test".into(),
            actor_id: Some("actor_test".into()),
            actor_name: Some("Tester".into()),
            timestamp: Some("2026-05-10T00:00:00Z".into()),
            flavor: "Test".into(),
            formula: formula.into(),
            splat: splat.into(),
            basic_results: basic,
            advanced_results: advanced,
            total: 0,
            difficulty: None,
            raw: json!({}),
        }
    }

    #[test]
    fn vampire_clean_no_messy_no_bestial() {
        let m = make_msg("vampire", "5dv cs>5 + 0dg cs>5", vec![3, 7, 9, 10, 6], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Vampire);
        assert_eq!(c.criticals, 1);
        assert!(!c.messy);
        assert!(!c.bestial);
        assert!(!c.brutal);
    }

    #[test]
    fn vampire_messy_when_hunger_ten() {
        let m = make_msg("vampire", "5dv cs>5 + 3dg cs>5", vec![3, 7, 9, 6, 6], vec![2, 8, 10]);
        let c = to_canonical_roll(&m);
        assert!(c.messy);
        assert_eq!(c.criticals, 1, "the 10 on hunger counts as a crit");
    }

    #[test]
    fn vampire_bestial_when_hunger_one() {
        let m = make_msg("vampire", "5dv cs>5 + 3dg cs>5", vec![3, 7, 9, 6, 6], vec![1, 8, 4]);
        let c = to_canonical_roll(&m);
        assert!(c.bestial);
        assert!(!c.messy);
    }

    #[test]
    fn werewolf_brutal_when_rage_one() {
        let m = make_msg("werewolf", "5dw cs>5 + 2dr cs>5", vec![3, 7, 9], vec![1, 8]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Werewolf);
        assert!(c.brutal);
    }

    #[test]
    fn double_basic_tens_count_two_criticals() {
        let m = make_msg("vampire", "4dv cs>5", vec![10, 10, 5, 7], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.criticals, 2);
    }

    #[test]
    fn splat_detection_from_formula_when_js_splat_unknown() {
        let m = make_msg("unknown", "5dv cs>5 + 2dg cs>5", vec![6, 7], vec![3]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Vampire);
    }

    #[test]
    fn empty_advanced_no_panic() {
        let m = make_msg("mortal", "3dm cs>5", vec![6, 7, 8], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Mortal);
        assert!(!c.messy);
        assert!(!c.bestial);
        assert!(!c.brutal);
    }

    #[test]
    fn vampire_unspaced_formula_from_live_sample() {
        // Real Foundry formulas don't put a space between denom and "cs>5".
        // Captured live in docs/reference/foundry-vtm5e-roll-sample.json.
        let m = make_msg("unknown", "12dvcs>5 + 0dgcs>5", vec![3, 7, 9, 10, 6, 4, 8, 6, 5, 9, 2, 7], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Vampire,
            "splat detection must work on real (unspaced) Foundry formulas");
    }

    #[test]
    fn has_die_requires_digit_prefix() {
        // The structural invariant: real formulas have a digit count before d<x>.
        assert!(has_die("12dvcs>5", 'v'));
        assert!(has_die("5dv cs>5", 'v'));
        assert!(has_die("0dg cs>5", 'g'));
        assert!(has_die("3dh+2ds", 'h'));
        assert!(has_die("3dh+2ds", 's'));
        // Without a digit prefix, do not match (defensive against false positives
        // in arbitrary text).
        assert!(!has_die("dv cs>5", 'v'));
        assert!(!has_die("addv", 'v'));
    }
}
