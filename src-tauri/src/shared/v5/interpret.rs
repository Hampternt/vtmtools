//! Convert RollResult into a Tally (successes, crits, hunger flags).

use crate::shared::v5::types::{Die, DieKind, PoolPart, RollResult, Tally};

pub fn interpret(result: &RollResult) -> Tally {
    let mut tens: u8 = 0;
    let mut hunger_tens: u8 = 0;
    let mut hunger_ones: u8 = 0;
    let mut nontens_six_plus: u8 = 0;

    for d in &result.dice {
        if d.value == 10 {
            tens += 1;
            if d.kind == DieKind::Hunger { hunger_tens += 1; }
        } else if d.value >= 6 {
            nontens_six_plus += 1;
        }
        if d.kind == DieKind::Hunger && d.value == 1 {
            hunger_ones += 1;
        }
    }

    let crit_pairs = tens / 2;
    let successes = nontens_six_plus + tens + crit_pairs * 2;
    let is_critical = crit_pairs >= 1;
    let is_messy_critical = is_critical && hunger_tens >= 1;

    Tally {
        successes,
        crit_pairs,
        is_critical,
        is_messy_critical,
        has_hunger_one: hunger_ones > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn die(kind: DieKind, value: u8) -> Die { Die { kind, value } }

    fn result_from(values: &[(DieKind, u8)]) -> RollResult {
        RollResult {
            parts: vec![PoolPart { name: "X".into(), level: values.len() as u8 }],
            dice: values.iter().map(|(k, v)| die(*k, *v)).collect(),
        }
    }

    #[test]
    fn no_dice_no_successes() {
        let r = result_from(&[]);
        let t = interpret(&r);
        assert_eq!(t.successes, 0);
        assert_eq!(t.crit_pairs, 0);
        assert!(!t.is_critical);
    }

    #[test]
    fn dice_six_through_nine_each_count_one_success() {
        let r = result_from(&[
            (DieKind::Regular, 6), (DieKind::Regular, 7),
            (DieKind::Regular, 8), (DieKind::Regular, 9),
            (DieKind::Regular, 5), (DieKind::Regular, 1),
        ]);
        let t = interpret(&r);
        assert_eq!(t.successes, 4);
        assert_eq!(t.crit_pairs, 0);
        assert!(!t.is_critical);
    }

    #[test]
    fn two_tens_yield_four_successes_one_crit_pair() {
        let r = result_from(&[(DieKind::Regular, 10), (DieKind::Regular, 10)]);
        let t = interpret(&r);
        assert_eq!(t.successes, 4);  // 2 from tens + 2 bonus
        assert_eq!(t.crit_pairs, 1);
        assert!(t.is_critical);
        assert!(!t.is_messy_critical);
    }

    #[test]
    fn four_tens_yield_eight_successes_two_crit_pairs() {
        let r = result_from(&[
            (DieKind::Regular, 10), (DieKind::Regular, 10),
            (DieKind::Regular, 10), (DieKind::Regular, 10),
        ]);
        let t = interpret(&r);
        assert_eq!(t.successes, 8);  // 4 + 2*2 bonus
        assert_eq!(t.crit_pairs, 2);
    }

    #[test]
    fn three_tens_yield_five_successes_one_pair_plus_lone() {
        let r = result_from(&[
            (DieKind::Regular, 10), (DieKind::Regular, 10),
            (DieKind::Regular, 10),
        ]);
        let t = interpret(&r);
        assert_eq!(t.successes, 5);  // 3 from tens + 2 bonus from one pair
        assert_eq!(t.crit_pairs, 1);
    }

    #[test]
    fn hunger_ten_in_pair_marks_messy() {
        let r = result_from(&[(DieKind::Hunger, 10), (DieKind::Regular, 10)]);
        let t = interpret(&r);
        assert!(t.is_critical);
        assert!(t.is_messy_critical);
    }

    #[test]
    fn hunger_ten_alone_is_not_messy_without_pair() {
        let r = result_from(&[(DieKind::Hunger, 10), (DieKind::Regular, 7)]);
        let t = interpret(&r);
        assert!(!t.is_critical);
        assert!(!t.is_messy_critical);
        assert_eq!(t.successes, 2);  // one ten + one ≥6 = 2
    }

    #[test]
    fn hunger_one_recorded_in_tally() {
        let r = result_from(&[(DieKind::Hunger, 1), (DieKind::Regular, 7)]);
        let t = interpret(&r);
        assert!(t.has_hunger_one);
    }

    #[test]
    fn regular_one_does_not_set_hunger_one() {
        let r = result_from(&[(DieKind::Regular, 1), (DieKind::Hunger, 5)]);
        let t = interpret(&r);
        assert!(!t.has_hunger_one);
    }
}
