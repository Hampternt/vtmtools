//! Orchestrator: assembles → rolls → interprets → compares → formats.

use crate::shared::v5::pool::build_pool;
use crate::shared::v5::dice::roll_pool;
use crate::shared::v5::interpret::interpret;
use crate::shared::v5::difficulty::compare;
use crate::shared::v5::message::format_skill_check;
use crate::shared::v5::types::{SkillCheckInput, SkillCheckResult};
use rand::Rng;

pub fn skill_check<R: Rng + ?Sized>(
    input: &SkillCheckInput,
    rng: &mut R,
) -> SkillCheckResult {
    let spec    = build_pool(input);
    let roll    = roll_pool(&spec, rng);
    let tally   = interpret(&roll);
    let outcome = compare(&tally, input.difficulty);
    let message = format_skill_check(input, &roll, &outcome);
    SkillCheckResult { spec, roll, tally, outcome, message }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::v5::types::PoolPart;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn sample_input() -> SkillCheckInput {
        SkillCheckInput {
            character_name: Some("Charlotte".into()),
            attribute: PoolPart { name: "Strength".into(), level: 4 },
            skill: PoolPart { name: "Brawl".into(), level: 3 },
            hunger: 2,
            specialty: None,
            difficulty: 2,
        }
    }

    #[test]
    fn deterministic_with_seed() {
        let mut rng_a = StdRng::seed_from_u64(42);
        let mut rng_b = StdRng::seed_from_u64(42);
        let a = skill_check(&sample_input(), &mut rng_a);
        let b = skill_check(&sample_input(), &mut rng_b);
        assert_eq!(a.outcome.successes, b.outcome.successes);
        assert_eq!(a.outcome.margin, b.outcome.margin);
        assert_eq!(a.message, b.message);
    }

    #[test]
    fn pipeline_produces_message_referencing_character() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = skill_check(&sample_input(), &mut rng);
        assert!(r.message.contains("Charlotte"));
        assert!(r.message.contains("Strength"));
        assert!(r.message.contains("Brawl"));
    }

    #[test]
    fn pool_size_matches_attribute_plus_skill_plus_hunger_clamp() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = skill_check(&sample_input(), &mut rng);
        // attribute 4 + skill 3 = pool 7; hunger 2 → 5 regular + 2 hunger.
        assert_eq!(r.spec.regular_count, 5);
        assert_eq!(r.spec.hunger_count, 2);
        assert_eq!(r.roll.dice.len(), 7);
    }
}
