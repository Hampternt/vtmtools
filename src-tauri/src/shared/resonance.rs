use crate::shared::dice::{advantage_roll, roll_d10, weighted_resonance_pick};
use crate::shared::types::*;

/// Rolls temperament according to config. Returns (selected_die, all_dice, Temperament).
pub fn roll_temperament(config: &TemperamentConfig) -> (u8, Vec<u8>, Temperament) {
    debug_assert!(
        config.negligible_max < config.fleeting_max,
        "TemperamentConfig invariant violated: negligible_max ({}) must be < fleeting_max ({})",
        config.negligible_max, config.fleeting_max
    );
    let (die, all_rolls) = advantage_roll(config.dice_count, config.take_highest);
    let temperament = if die <= config.negligible_max {
        Temperament::Negligible
    } else if die <= config.fleeting_max {
        Temperament::Fleeting
    } else {
        Temperament::Intense
    };
    (die, all_rolls, temperament)
}

/// Rolls the resonance type (returns a display die + weighted pick result).
pub fn roll_resonance_type(weights: &ResonanceWeights) -> (u8, ResonanceType) {
    let display_die = roll_d10(); // shown to GM for flavour
    let resonance_type = weighted_resonance_pick(weights);
    (display_die, resonance_type)
}

/// Rolls the Acute check (9–10 = Acute). Returns (die, is_acute).
pub fn check_acute() -> (u8, bool) {
    let die = roll_d10();
    (die, die >= 9)
}

/// Executes the full roll sequence from a RollConfig.
/// Does NOT populate the dyscrasia field — that requires a DB call done in the command layer.
pub fn execute_roll(config: &RollConfig) -> ResonanceRollResult {
    let (temperament_die, temperament_dice, temperament) =
        roll_temperament(&config.temperament);

    match temperament {
        Temperament::Negligible => ResonanceRollResult {
            temperament_dice,
            temperament_die,
            temperament: Temperament::Negligible,
            resonance_type: None,
            resonance_die: None,
            acute_die: None,
            is_acute: false,
            dyscrasia: None,
        },
        Temperament::Fleeting => {
            let (res_die, res_type) = roll_resonance_type(&config.weights);
            ResonanceRollResult {
                temperament_dice,
                temperament_die,
                temperament: Temperament::Fleeting,
                resonance_type: Some(res_type),
                resonance_die: Some(res_die),
                acute_die: None,
                is_acute: false,
                dyscrasia: None,
            }
        }
        Temperament::Intense => {
            let (res_die, res_type) = roll_resonance_type(&config.weights);
            let (acute_die, is_acute) = check_acute();
            ResonanceRollResult {
                temperament_dice,
                temperament_die,
                temperament: Temperament::Intense,
                resonance_type: Some(res_type),
                resonance_die: Some(res_die),
                acute_die: Some(acute_die),
                is_acute,
                dyscrasia: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> RollConfig {
        RollConfig::default()
    }

    #[test]
    fn temperament_negligible_when_die_lte_negligible_max() {
        // negligible_max=10 covers the entire d10 range; fleeting_max=11 satisfies the invariant
        let config = TemperamentConfig {
            dice_count: 1,
            take_highest: true,
            negligible_max: 10,
            fleeting_max: 11,
        };
        for _ in 0..50 {
            let (_, _, t) = roll_temperament(&config);
            assert_eq!(t, Temperament::Negligible);
        }
    }

    #[test]
    fn temperament_intense_when_die_above_fleeting_max() {
        // negligible_max=0 and fleeting_max=1 means die=1 → Fleeting, die 2–10 → Intense.
        // Use negligible_max=0, fleeting_max=0 is not valid; instead force Intense by setting
        // both thresholds below the minimum die value (d10 rolls 1–10), so we set
        // negligible_max=0 and fleeting_max=0 is invalid. Use a config where the
        // temperament can be verified: negligible_max=0, fleeting_max=0 violates invariant.
        // Closest valid config: verify that a die value above fleeting_max yields Intense.
        let config = TemperamentConfig {
            dice_count: 1,
            take_highest: true,
            negligible_max: 0,
            fleeting_max: 1,
        };
        // With fleeting_max=1, any die value > 1 yields Intense. Run many iterations and
        // assert each roll is either Fleeting (die=1) or Intense (die>1) — never Negligible.
        for _ in 0..50 {
            let (die, _, t) = roll_temperament(&config);
            if die > 1 {
                assert_eq!(t, Temperament::Intense, "die={die} should be Intense");
            } else {
                assert_eq!(t, Temperament::Fleeting, "die={die} should be Fleeting");
            }
        }
    }

    #[test]
    fn check_acute_returns_true_for_9_or_10() {
        for _ in 0..500 {
            let (die, is_acute) = check_acute();
            if die >= 9 {
                assert!(is_acute, "die={die} should be acute");
            } else {
                assert!(!is_acute, "die={die} should not be acute");
            }
        }
    }

    #[test]
    fn execute_roll_negligible_has_no_resonance_type() {
        let config = RollConfig {
            temperament: TemperamentConfig {
                dice_count: 1,
                take_highest: true,
                negligible_max: 10,
                fleeting_max: 11,
            },
            weights: ResonanceWeights::default(),
        };
        let result = execute_roll(&config);
        assert_eq!(result.temperament, Temperament::Negligible);
        assert!(result.resonance_type.is_none());
        assert!(result.acute_die.is_none());
        assert!(!result.is_acute);
    }

    #[test]
    fn execute_roll_intense_always_has_resonance_type_and_acute_die() {
        // negligible_max=0, fleeting_max=1: die=1 → Fleeting, die 2–10 → Intense.
        // Run enough iterations that we are statistically certain to hit Intense at least once,
        // and assert that every Intense result carries resonance_type and acute_die.
        let config = RollConfig {
            temperament: TemperamentConfig {
                dice_count: 1,
                take_highest: true,
                negligible_max: 0,
                fleeting_max: 1,
            },
            weights: ResonanceWeights::default(),
        };
        let mut saw_intense = false;
        for _ in 0..100 {
            let result = execute_roll(&config);
            if result.temperament == Temperament::Intense {
                saw_intense = true;
                assert!(result.resonance_type.is_some(), "Intense must have resonance_type");
                assert!(result.acute_die.is_some(), "Intense must have acute_die");
            }
        }
        assert!(saw_intense, "Expected at least one Intense result in 100 rolls");
    }

    #[test]
    fn execute_roll_fleeting_has_resonance_type_but_no_acute() {
        let config = RollConfig {
            temperament: TemperamentConfig {
                dice_count: 1,
                take_highest: true,
                negligible_max: 0,
                fleeting_max: 10,
            },
            weights: ResonanceWeights::default(),
        };
        let result = execute_roll(&config);
        assert_eq!(result.temperament, Temperament::Fleeting);
        assert!(result.resonance_type.is_some(), "Fleeting must have resonance_type");
        assert!(result.resonance_die.is_some(), "Fleeting must have resonance_die");
        assert!(result.acute_die.is_none(), "Fleeting must NOT have acute_die");
        assert!(!result.is_acute, "Fleeting must NOT be acute");
    }
}
