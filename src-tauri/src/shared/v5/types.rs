//! All V5 helper types in one place. Cross-leaf shared shapes.

use serde::{Deserialize, Serialize};

/// One named contributor to a V5 dice pool — typically an Attribute or a Skill.
/// Specialty is represented as a synthetic `PoolPart` named "Specialty: <name>"
/// with `level: 1` so the dice-rolling step is uniform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolPart {
    pub name: String,
    pub level: u8,   // 0..=5 in V5; specialty contributions are always 1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DieKind { Regular, Hunger }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Die {
    pub kind: DieKind,
    pub value: u8,   // 1..=10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolSpec {
    pub parts: Vec<PoolPart>,
    pub regular_count: u8,
    pub hunger_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollResult {
    pub parts: Vec<PoolPart>,
    /// Pool-order: regulars first, then hunger.
    pub dice: Vec<Die>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub successes: u8,           // dice ≥6 + 2*crit_pairs
    pub crit_pairs: u8,          // tens / 2
    pub is_critical: bool,       // crit_pairs ≥ 1
    pub is_messy_critical: bool, // critical AND ≥1 hunger 10
    pub has_hunger_one: bool,    // ≥1 hunger 1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeFlags {
    pub critical: bool,
    pub messy: bool,
    pub bestial_failure: bool,   // !passed AND has_hunger_one
    pub total_failure: bool,     // successes == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub successes: u8,
    pub difficulty: u8,
    pub margin: i32,             // successes - difficulty
    pub passed: bool,
    pub flags: OutcomeFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCheckInput {
    pub character_name: Option<String>,   // for message formatting
    pub attribute: PoolPart,
    pub skill: PoolPart,
    pub hunger: u8,                       // 0..=5; 0 = mortal/non-vampire
    pub specialty: Option<String>,        // Some(name) → +1 die
    pub difficulty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCheckResult {
    pub spec: PoolSpec,
    pub roll: RollResult,
    pub tally: Tally,
    pub outcome: Outcome,
    pub message: String,
}
