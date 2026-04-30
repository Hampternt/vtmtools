//! Pure dice rolling. RNG-injected for deterministic tests.

use crate::shared::v5::types::{Die, DieKind, PoolPart, PoolSpec, RollResult};
use rand::Rng;

pub fn roll_pool<R: Rng + ?Sized>(spec: &PoolSpec, rng: &mut R) -> RollResult {
    let total = (spec.regular_count + spec.hunger_count) as usize;
    let mut dice = Vec::with_capacity(total);
    for _ in 0..spec.regular_count {
        dice.push(Die { kind: DieKind::Regular, value: rng.gen_range(1..=10) });
    }
    for _ in 0..spec.hunger_count {
        dice.push(Die { kind: DieKind::Hunger, value: rng.gen_range(1..=10) });
    }
    RollResult { parts: spec.parts.clone(), dice }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn spec(reg: u8, hung: u8) -> PoolSpec {
        PoolSpec {
            parts: vec![
                PoolPart { name: "X".into(), level: reg + hung },
            ],
            regular_count: reg,
            hunger_count: hung,
        }
    }

    #[test]
    fn roll_count_matches_spec() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = roll_pool(&spec(5, 2), &mut rng);
        assert_eq!(r.dice.len(), 7);
    }

    #[test]
    fn regulars_first_then_hunger_in_pool_order() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = roll_pool(&spec(3, 2), &mut rng);
        assert_eq!(r.dice.len(), 5);
        assert_eq!(r.dice[0].kind, DieKind::Regular);
        assert_eq!(r.dice[1].kind, DieKind::Regular);
        assert_eq!(r.dice[2].kind, DieKind::Regular);
        assert_eq!(r.dice[3].kind, DieKind::Hunger);
        assert_eq!(r.dice[4].kind, DieKind::Hunger);
    }

    #[test]
    fn dice_values_are_one_through_ten() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = roll_pool(&spec(10, 0), &mut rng);
        for d in &r.dice {
            assert!(d.value >= 1 && d.value <= 10, "die value out of range: {}", d.value);
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);
        let a = roll_pool(&spec(5, 2), &mut rng_a);
        let b = roll_pool(&spec(5, 2), &mut rng_b);
        let av: Vec<_> = a.dice.iter().map(|d| d.value).collect();
        let bv: Vec<_> = b.dice.iter().map(|d| d.value).collect();
        assert_eq!(av, bv);
    }

    #[test]
    fn parts_are_copied_into_result() {
        let s = spec(3, 0);
        let parts_in = s.parts.clone();
        let mut rng = StdRng::seed_from_u64(1);
        let r = roll_pool(&s, &mut rng);
        assert_eq!(r.parts.len(), parts_in.len());
        assert_eq!(r.parts[0].name, parts_in[0].name);
    }
}
