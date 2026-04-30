//! V5 dice pool assembly.

use crate::shared::v5::types::{PoolPart, PoolSpec, SkillCheckInput};

pub fn build_pool(input: &SkillCheckInput) -> PoolSpec {
    let mut parts = Vec::with_capacity(3);
    parts.push(input.attribute.clone());
    parts.push(input.skill.clone());
    if let Some(name) = &input.specialty {
        parts.push(PoolPart {
            name: format!("Specialty: {}", name),
            level: 1,
        });
    }

    let pool_size: u8 = parts.iter().map(|p| p.level).sum();
    let hunger_count = input.hunger.min(pool_size);
    let regular_count = pool_size - hunger_count;

    PoolSpec { parts, regular_count, hunger_count }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(attr: u8, skill: u8, hunger: u8, specialty: Option<&str>) -> SkillCheckInput {
        SkillCheckInput {
            character_name: None,
            attribute: PoolPart { name: "Strength".into(), level: attr },
            skill: PoolPart { name: "Brawl".into(), level: skill },
            hunger,
            specialty: specialty.map(String::from),
            difficulty: 0,
        }
    }

    #[test]
    fn pool_size_is_attribute_plus_skill() {
        let spec = build_pool(&input(3, 4, 0, None));
        assert_eq!(spec.regular_count + spec.hunger_count, 7);
        assert_eq!(spec.parts.len(), 2);
    }

    #[test]
    fn specialty_adds_one_die_with_labeled_part() {
        let spec = build_pool(&input(3, 4, 0, Some("bare-knuckle")));
        assert_eq!(spec.regular_count + spec.hunger_count, 8);
        assert_eq!(spec.parts.len(), 3);
        assert!(spec.parts[2].name.contains("bare-knuckle"));
        assert_eq!(spec.parts[2].level, 1);
    }

    #[test]
    fn hunger_replaces_regular_dice_one_for_one() {
        let spec = build_pool(&input(3, 4, 2, None));
        assert_eq!(spec.regular_count, 5);
        assert_eq!(spec.hunger_count, 2);
    }

    #[test]
    fn hunger_capped_at_pool_size() {
        let spec = build_pool(&input(2, 1, 5, None)); // pool 3, hunger 5
        assert_eq!(spec.regular_count, 0);
        assert_eq!(spec.hunger_count, 3);
    }

    #[test]
    fn zero_pool_is_zero_dice() {
        let spec = build_pool(&input(0, 0, 0, None));
        assert_eq!(spec.regular_count + spec.hunger_count, 0);
    }
}
