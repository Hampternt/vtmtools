use rand::Rng;
use crate::shared::types::{ResonanceType, ResonanceWeights};

/// Rolls a single d10. Returns 1–10 (0 on the die = 10).
pub fn roll_d10() -> u8 {
    let n: u8 = rand::thread_rng().gen_range(0..10);
    if n == 0 { 10 } else { n }
}

/// Rolls `count` d10s. Returns (selected_value, all_rolls).
/// If take_highest=true, returns the max; otherwise returns the min.
pub fn advantage_roll(count: u8, take_highest: bool) -> (u8, Vec<u8>) {
    let count = count.max(1).min(5);
    let rolls: Vec<u8> = (0..count).map(|_| roll_d10()).collect();
    let selected = if take_highest {
        *rolls.iter().max().unwrap()
    } else {
        *rolls.iter().min().unwrap()
    };
    (selected, rolls)
}

/// Selects a ResonanceType using weighted probability.
/// Base probabilities: Phlegmatic 30%, Melancholy 30%, Choleric 20%, Sanguine 20%.
/// Each type's base probability is multiplied by its slider level multiplier,
/// then results are normalised. "Guaranteed" bypasses normalisation entirely.
pub fn weighted_resonance_pick(weights: &ResonanceWeights) -> ResonanceType {
    // Base probabilities (must sum to 1.0)
    // Equal by default — all neutral means truly equal odds; sliders skew from there.
    let base = [
        (ResonanceType::Phlegmatic, 0.25_f64),
        (ResonanceType::Melancholy, 0.25_f64),
        (ResonanceType::Choleric,   0.25_f64),
        (ResonanceType::Sanguine,   0.25_f64),
    ];

    let multipliers = [
        weights.phlegmatic.multiplier(),
        weights.melancholy.multiplier(),
        weights.choleric.multiplier(),
        weights.sanguine.multiplier(),
    ];

    // Guaranteed: return immediately without normalising
    if let Some(i) = multipliers.iter().position(|m| m.is_infinite()) {
        return base[i].0.clone();
    }

    // Apply multipliers to base probabilities
    let weighted: Vec<f64> = base.iter().zip(&multipliers)
        .map(|((_, b), m)| b * m)
        .collect();

    let total: f64 = weighted.iter().sum();

    // All weights are zero — fall back to uniform
    if total == 0.0 {
        let i = rand::thread_rng().gen_range(0..4);
        return base[i].0.clone();
    }

    let pick: f64 = rand::thread_rng().gen_range(0.0..total);
    let mut cumulative = 0.0;
    for (i, w) in weighted.iter().enumerate() {
        cumulative += w;
        if pick < cumulative {
            return base[i].0.clone();
        }
    }
    base[3].0.clone() // Sanguine fallback (floating-point edge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_d10_stays_in_range() {
        for _ in 0..1000 {
            let r = roll_d10();
            assert!(r >= 1 && r <= 10, "roll_d10 returned {r}");
        }
    }

    #[test]
    fn advantage_roll_single_die_matches_roll_d10_range() {
        let (val, rolls) = advantage_roll(1, true);
        assert_eq!(rolls.len(), 1);
        assert!(val >= 1 && val <= 10);
        assert_eq!(val, rolls[0]);
    }

    #[test]
    fn advantage_roll_take_highest_returns_max() {
        for _ in 0..100 {
            let (val, rolls) = advantage_roll(3, true);
            assert_eq!(val, *rolls.iter().max().unwrap());
        }
    }

    #[test]
    fn advantage_roll_take_lowest_returns_min() {
        for _ in 0..100 {
            let (val, rolls) = advantage_roll(3, false);
            assert_eq!(val, *rolls.iter().min().unwrap());
        }
    }

    #[test]
    fn weighted_resonance_guaranteed_always_returns_that_type() {
        let weights = ResonanceWeights {
            phlegmatic: SliderLevel::Guaranteed,
            melancholy: SliderLevel::Neutral,
            choleric: SliderLevel::Neutral,
            sanguine: SliderLevel::Neutral,
        };
        for _ in 0..50 {
            assert_eq!(weighted_resonance_pick(&weights), ResonanceType::Phlegmatic);
        }
    }

    #[test]
    fn weighted_resonance_impossible_never_returns_that_type() {
        let weights = ResonanceWeights {
            phlegmatic: SliderLevel::Impossible,
            melancholy: SliderLevel::Neutral,
            choleric: SliderLevel::Neutral,
            sanguine: SliderLevel::Neutral,
        };
        for _ in 0..200 {
            assert_ne!(weighted_resonance_pick(&weights), ResonanceType::Phlegmatic);
        }
    }

    #[test]
    fn weighted_resonance_all_neutral_returns_all_types_over_many_rolls() {
        let weights = ResonanceWeights::default();
        let mut counts = std::collections::HashMap::new();
        for _ in 0..2000 {
            let t = weighted_resonance_pick(&weights);
            *counts.entry(format!("{t:?}")).or_insert(0u32) += 1;
        }
        assert!(counts.contains_key("Phlegmatic"), "Phlegmatic never appeared");
        assert!(counts.contains_key("Melancholy"), "Melancholy never appeared");
        assert!(counts.contains_key("Choleric"), "Choleric never appeared");
        assert!(counts.contains_key("Sanguine"), "Sanguine never appeared");
    }
}
