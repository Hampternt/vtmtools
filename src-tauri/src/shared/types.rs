use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Temperament {
    Negligible,
    Fleeting,
    Intense,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResonanceType {
    Phlegmatic,
    Melancholy,
    Choleric,
    Sanguine,
}

/// 7-step slider level for resonance type weighting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SliderLevel {
    Impossible,
    ExtremelyUnlikely,
    Unlikely,
    Neutral,
    Likely,
    ExtremelyLikely,
    Guaranteed,
}

impl SliderLevel {
    /// Maps slider level to a weight multiplier applied against the base probability
    pub fn multiplier(&self) -> f64 {
        match self {
            SliderLevel::Impossible => 0.0,
            SliderLevel::ExtremelyUnlikely => 0.1,
            SliderLevel::Unlikely => 0.5,
            SliderLevel::Neutral => 1.0,
            SliderLevel::Likely => 2.0,
            SliderLevel::ExtremelyLikely => 4.0,
            SliderLevel::Guaranteed => f64::INFINITY,
        }
    }
}

/// GM-configurable temperament roll options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemperamentConfig {
    /// How many d10s to roll (1–5). Result is best or worst of the pool.
    pub dice_count: u8,
    /// true = take highest die (biases toward Intense), false = take lowest
    pub take_highest: bool,
    /// Upper bound (inclusive) for Negligible result. Default 5.
    pub negligible_max: u8,
    /// Upper bound (inclusive) for Fleeting result. Default 8. Intense = above this.
    pub fleeting_max: u8,
}

impl Default for TemperamentConfig {
    fn default() -> Self {
        Self {
            dice_count: 1,
            take_highest: true,
            negligible_max: 5,
            fleeting_max: 8,
        }
    }
}

/// GM-configurable weighting for resonance type selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResonanceWeights {
    pub phlegmatic: SliderLevel,
    pub melancholy: SliderLevel,
    pub choleric: SliderLevel,
    pub sanguine: SliderLevel,
}

impl Default for ResonanceWeights {
    fn default() -> Self {
        Self {
            phlegmatic: SliderLevel::Neutral,
            melancholy: SliderLevel::Neutral,
            choleric: SliderLevel::Neutral,
            sanguine: SliderLevel::Neutral,
        }
    }
}

/// Full GM config passed to a roll
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollConfig {
    pub temperament: TemperamentConfig,
    pub weights: ResonanceWeights,
}

impl Default for RollConfig {
    fn default() -> Self {
        Self {
            temperament: TemperamentConfig::default(),
            weights: ResonanceWeights::default(),
        }
    }
}

/// A Dyscrasia entry from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DyscrasiaEntry {
    pub id: i64,
    pub resonance_type: ResonanceType,
    pub name: String,
    pub description: String,
    pub bonus: String,
    pub is_custom: bool,
}

/// Full result of one resonance roll sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResonanceRollResult {
    /// All dice rolled for temperament (for display)
    pub temperament_dice: Vec<u8>,
    /// The die value that determined temperament
    pub temperament_die: u8,
    pub temperament: Temperament,
    /// Set if temperament is Fleeting or Intense
    pub resonance_type: Option<ResonanceType>,
    /// d10 rolled for resonance (display only — weighted pick determines actual result)
    pub resonance_die: Option<u8>,
    /// Set if temperament is Intense
    pub acute_die: Option<u8>,
    pub is_acute: bool,
    /// Populated after GM rolls/picks Dyscrasia (not auto-populated here)
    pub dyscrasia: Option<DyscrasiaEntry>,
}
