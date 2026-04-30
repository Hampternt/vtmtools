//! Compare a Tally against a difficulty to produce an Outcome.

use crate::shared::v5::types::{Outcome, OutcomeFlags, Tally};

pub fn compare(tally: &Tally, difficulty: u8) -> Outcome {
    let margin: i32 = (tally.successes as i32) - (difficulty as i32);
    let passed = margin >= 0;
    let total_failure = tally.successes == 0;
    let bestial_failure = !passed && tally.has_hunger_one;

    Outcome {
        successes: tally.successes,
        difficulty,
        margin,
        passed,
        flags: OutcomeFlags {
            critical: tally.is_critical,
            messy: tally.is_messy_critical,
            bestial_failure,
            total_failure,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally(successes: u8, hunger_one: bool, critical: bool, messy: bool) -> Tally {
        Tally {
            successes,
            crit_pairs: if critical { 1 } else { 0 },
            is_critical: critical,
            is_messy_critical: messy,
            has_hunger_one: hunger_one,
        }
    }

    #[test]
    fn passed_with_positive_margin() {
        let o = compare(&tally(5, false, false, false), 2);
        assert!(o.passed);
        assert_eq!(o.margin, 3);
        assert!(!o.flags.bestial_failure);
        assert!(!o.flags.total_failure);
    }

    #[test]
    fn exactly_meeting_difficulty_passes() {
        let o = compare(&tally(2, false, false, false), 2);
        assert!(o.passed);
        assert_eq!(o.margin, 0);
    }

    #[test]
    fn below_difficulty_fails() {
        let o = compare(&tally(1, false, false, false), 3);
        assert!(!o.passed);
        assert_eq!(o.margin, -2);
    }

    #[test]
    fn zero_successes_is_total_failure() {
        let o = compare(&tally(0, false, false, false), 1);
        assert!(o.flags.total_failure);
    }

    #[test]
    fn failure_with_hunger_one_is_bestial() {
        let o = compare(&tally(1, true, false, false), 3);
        assert!(o.flags.bestial_failure);
    }

    #[test]
    fn pass_with_hunger_one_is_not_bestial() {
        let o = compare(&tally(3, true, false, false), 1);
        assert!(o.passed);
        assert!(!o.flags.bestial_failure);
    }

    #[test]
    fn total_failure_with_hunger_one_sets_both_flags() {
        let o = compare(&tally(0, true, false, false), 1);
        assert!(o.flags.total_failure);
        assert!(o.flags.bestial_failure);
    }

    #[test]
    fn critical_flag_propagates_to_outcome_flags() {
        let o = compare(&tally(4, false, true, false), 1);
        assert!(o.passed);
        assert!(o.flags.critical);
    }

    #[test]
    fn messy_flag_propagates() {
        let o = compare(&tally(4, false, true, true), 1);
        assert!(o.flags.messy);
    }
}
