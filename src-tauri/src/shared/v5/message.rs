//! Format the human-facing summary string for a skill check.

use crate::shared::v5::types::{Outcome, PoolPart, RollResult, SkillCheckInput};

pub fn format_skill_check(
    input: &SkillCheckInput,
    _result: &RollResult,
    outcome: &Outcome,
) -> String {
    // Owner prefix: "Charlotte's Strength + Brawl check" or just "Strength + Brawl check".
    let head = match input.character_name.as_deref() {
        Some(name) if !name.is_empty() => format!(
            "{}'s {} + {} check",
            name, input.attribute.name, input.skill.name
        ),
        _ => format!("{} + {} check", input.attribute.name, input.skill.name),
    };

    // Embellishment: pick the most specific descriptor; flags ordered by priority.
    let descriptor = if outcome.flags.messy {
        "Messy Critical"
    } else if outcome.flags.critical && outcome.passed {
        "Critical"
    } else if outcome.flags.bestial_failure && outcome.flags.total_failure {
        "Bestial Total Failure"
    } else if outcome.flags.bestial_failure {
        "Bestial Failure"
    } else if outcome.flags.total_failure {
        "Total Failure"
    } else if outcome.passed {
        "Success"
    } else {
        "Failure"
    };

    // Margin sign: "+3" / "-2".
    let margin_str = if outcome.margin >= 0 {
        format!("+{}", outcome.margin)
    } else {
        format!("{}", outcome.margin)
    };

    format!(
        "{} · {} · {} ({} vs DV {})",
        head, descriptor, margin_str, outcome.successes, outcome.difficulty
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::v5::types::{Die, DieKind, OutcomeFlags};

    fn input_with(name: Option<&str>) -> SkillCheckInput {
        SkillCheckInput {
            character_name: name.map(String::from),
            attribute: PoolPart { name: "Strength".into(), level: 4 },
            skill: PoolPart { name: "Brawl".into(), level: 3 },
            hunger: 2,
            specialty: None,
            difficulty: 2,
        }
    }

    fn empty_result() -> RollResult {
        RollResult { parts: vec![], dice: vec![] }
    }

    fn outcome(successes: u8, difficulty: u8, passed: bool, flags: OutcomeFlags) -> Outcome {
        Outcome {
            successes,
            difficulty,
            margin: (successes as i32) - (difficulty as i32),
            passed,
            flags,
        }
    }

    fn flags(critical: bool, messy: bool, bestial: bool, total: bool) -> OutcomeFlags {
        OutcomeFlags { critical, messy, bestial_failure: bestial, total_failure: total }
    }

    #[test]
    fn includes_character_name_when_set() {
        let i = input_with(Some("Charlotte"));
        let o = outcome(5, 2, true, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("Charlotte"), "expected Charlotte in: {s}");
    }

    #[test]
    fn omits_owner_when_name_is_none() {
        let i = input_with(None);
        let o = outcome(5, 2, true, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(!s.starts_with("'s"));
        assert!(s.starts_with("Strength + Brawl"));
    }

    #[test]
    fn marks_messy_critical() {
        let i = input_with(Some("Charlotte"));
        let o = outcome(5, 2, true, flags(true, true, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("Messy Critical"));
    }

    #[test]
    fn marks_bestial_failure() {
        let i = input_with(Some("Tessa"));
        let o = outcome(1, 3, false, flags(false, false, true, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("Bestial Failure"));
    }

    #[test]
    fn shows_margin_and_dv() {
        let i = input_with(None);
        let o = outcome(5, 2, true, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("+3"), "expected +3 in: {s}");
        assert!(s.contains("DV 2"), "expected DV 2 in: {s}");
    }

    #[test]
    fn shows_negative_margin_for_failure() {
        let i = input_with(None);
        let o = outcome(1, 3, false, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("-2"), "expected -2 in: {s}");
    }
}
