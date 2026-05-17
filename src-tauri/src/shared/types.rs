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

/// Kind discriminator for the polymorphic `advantages` library table.
/// Mirrors the SQL CHECK constraint in `0009_advantages_kind_and_source.sql`.
///
/// Partitioning rule (ARCHITECTURE.md §9): same row shape → same table with
/// kind; different row shape → own table. The four variants here share the
/// Advantage row shape AND the `actor.create_feature` push contract
/// (foundry helper roadmap §5). Dyscrasias and (future) disciplines have
/// different row shapes and get their own tables.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdvantageKind {
    Merit,
    Flaw,
    Background,
    Boon,
}

/// A library entry for a VTM 5e Merit, Background, Flaw, or Boon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Advantage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub kind: AdvantageKind,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,
    pub is_custom: bool,
    /// FVTT-import provenance. None = hand-authored locally (corebook or
    /// GM custom). Some = imported from a Foundry world; JSON shape
    /// described in the migration comment. Promoted to a tagged enum
    /// when a second source materializes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_attribution: Option<serde_json::Value>,
}

/// Outcome of one import attempt for a single world item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ImportOutcome {
    /// Row didn't exist locally. Inserted as-is (may include world-suffix
    /// in the name if a same-name local row already existed).
    Inserted { id: i64, name: String, kind: AdvantageKind },
    /// Same (foundryId, worldTitle) already imported here. Updated description
    /// + bumped imported_at; row id preserved.
    Updated  { id: i64, name: String, kind: AdvantageKind },
    /// Filtered out (non-feature item.kind, or unrecognized featuretype).
    Skipped  { reason: String, name: String },
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

// ---------------------------------------------------------------------------
// Domains Manager / Chronicle graph types
// ---------------------------------------------------------------------------

/// A running game. Contains nodes and edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chronicle {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Single-or-multi string value. Serialized untagged: a raw string for single,
/// an array for multi.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringFieldValue {
    Single(String),
    Multi(Vec<String>),
}

/// Single-or-multi number value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum NumberFieldValue {
    Single(f64),
    Multi(Vec<f64>),
}

/// A typed field value. `#[serde(tag = "type")]` means the JSON discriminator field
/// `"type"` chooses which variant is parsed; a value of the wrong type fails to
/// deserialize — no manual validation code needed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FieldValue {
    String    { value: StringFieldValue },
    Text      { value: String           },
    Number    { value: NumberFieldValue },
    Date      { value: String           },
    Url       { value: String           },
    Email     { value: String           },
    Bool      { value: bool             },
    Reference { value: i64              },
}

/// A named, typed field. JSON shape example:
///   {"name": "influence_rating", "type": "number", "value": 3}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    #[serde(flatten)]
    pub value: FieldValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub chronicle_id: i64,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub id: i64,
    pub chronicle_id: i64,
    pub from_node_id: i64,
    pub to_node_id: i64,
    pub edge_type: String,
    pub description: String,
    pub properties: Vec<Field>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeDirection {
    In,
    Out,
    Both,
}
