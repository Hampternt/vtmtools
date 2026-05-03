use serde::{Deserialize, Serialize};
use crate::bridge::types::SourceKind;

/// One row in the `character_modifiers` table, hydrated from JSON columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterModifier {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub is_active: bool,
    pub is_hidden: bool,
    pub origin_template_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Tagged enum describing what the modifier is bound to. New variants
/// (Room, FoundryEffect) can be added without a migration — just deserialize
/// a new shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ModifierBinding {
    Free,
    Advantage { item_id: String },
    // Future variants intentionally unstubbed (per ARCH §11 plan packaging
    // — strict additivity preserved without dead-code stubs):
    //   Room { room_id: i64 }              — rooms/bundles future
    //   FoundryEffect { effect_id: String } — Phase 4 mirror
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModifierEffect {
    pub kind: ModifierKind,
    pub scope: Option<String>,
    pub delta: Option<i32>,
    pub note: Option<String>,
    /// Foundry-bonus dot-paths (e.g. ["attributes.strength", "skills.subterfuge"]).
    /// Mirrors `FoundryItemBonus.paths`. Empty vec = pathless. Only used by the
    /// push-to-Foundry command on `pool`-kind effects; ignored for other kinds.
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind { Pool, Difficulty, Note }

/// Argument shape for `add_character_modifier`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCharacterModifier {
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub origin_template_id: Option<i64>,
}

/// Patch shape for `update_character_modifier`. Active/hidden have dedicated
/// setters; binding cannot be changed once set.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub effects: Option<Vec<ModifierEffect>>,
    pub tags: Option<Vec<String>>,
}
