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
    /// Source labels (`bonus.source` strings) that this modifier "captured"
    /// when it was created via "Save as local override". Non-empty marks
    /// this row as a Foundry override → push becomes surgical-replace
    /// (drops bonuses whose source ∈ this list before appending ours).
    /// Empty = hand-rolled modifier with additive push semantics.
    #[serde(default)]
    pub foundry_captured_labels: Vec<String>,
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
pub enum ModifierKind {
    Pool,
    Difficulty,
    Note,
    Stat, // NEW — render-time visual stat delta on the character card.
}

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
    #[serde(default)]
    pub foundry_captured_labels: Vec<String>,
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

/// One row in the status_templates table — a GM-authored reusable bundle of
/// effects + tags. Templates have no character anchor; they're applied as
/// independent copies via add_character_modifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTemplate {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewStatusTemplate {
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTemplatePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub effects: Option<Vec<ModifierEffect>>,
    pub tags: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_effect_stat_round_trips_through_json() {
        let original = ModifierEffect {
            kind: ModifierKind::Stat,
            scope: Some("vs social rolls".to_string()),
            delta: Some(1),
            note: None,
            paths: vec!["attributes.charisma".to_string(), "attributes.manipulation".to_string()],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let round_trip: ModifierEffect = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip.kind, ModifierKind::Stat);
        assert_eq!(round_trip.scope.as_deref(), Some("vs social rolls"));
        assert_eq!(round_trip.delta, Some(1));
        assert_eq!(round_trip.note, None);
        assert_eq!(
            round_trip.paths,
            vec!["attributes.charisma".to_string(), "attributes.manipulation".to_string()],
        );
    }

    #[test]
    fn modifier_kind_serializes_as_snake_case() {
        // Regression — guards against accidentally renaming the variant.
        let stat_json = serde_json::to_string(&ModifierKind::Stat).expect("ser stat");
        assert_eq!(stat_json, r#""stat""#);
        let pool_json = serde_json::to_string(&ModifierKind::Pool).expect("ser pool");
        assert_eq!(pool_json, r#""pool""#);
    }

    #[test]
    fn effects_json_blob_with_all_four_kinds_round_trips() {
        // Mirrors the actual `effects_json` TEXT column shape.
        let blob = vec![
            ModifierEffect { kind: ModifierKind::Pool,       scope: Some("Social".into()),  delta: Some(1),  note: None,                paths: vec![] },
            ModifierEffect { kind: ModifierKind::Difficulty, scope: None,                   delta: Some(-1), note: None,                paths: vec![] },
            ModifierEffect { kind: ModifierKind::Note,       scope: None,                   delta: None,     note: Some("blinded".into()), paths: vec![] },
            ModifierEffect { kind: ModifierKind::Stat,       scope: Some("Beautiful".into()), delta: Some(1),  note: None,                paths: vec!["attributes.charisma".into()] },
        ];
        let json = serde_json::to_string(&blob).expect("serialize");
        let round_trip: Vec<ModifierEffect> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip.len(), 4);
        assert_eq!(round_trip[3].kind, ModifierKind::Stat);
        assert_eq!(round_trip[3].paths, vec!["attributes.charisma".to_string()]);
    }

    #[test]
    fn character_modifier_captured_labels_round_trip_json() {
        let json = serde_json::json!({
            "id": 7,
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Resilience",
            "description": "",
            "effects": [],
            "binding": { "kind": "advantage", "item_id": "merit-1" },
            "tags": [],
            "isActive": true,
            "isHidden": false,
            "originTemplateId": null,
            "foundryCapturedLabels": ["Buff Modifier", "Fortify"],
            "createdAt": "2026-05-12 00:00:00",
            "updatedAt": "2026-05-12 00:00:00",
        });
        let m: CharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert_eq!(m.foundry_captured_labels, vec!["Buff Modifier", "Fortify"]);
        let round_trip = serde_json::to_value(&m).expect("serialize");
        assert_eq!(round_trip["foundryCapturedLabels"], serde_json::json!(["Buff Modifier", "Fortify"]));
    }

    #[test]
    fn character_modifier_missing_captured_labels_defaults_to_empty() {
        // Legacy rows from before the migration / IPC payloads without the field.
        let json = serde_json::json!({
            "id": 1,
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Legacy",
            "description": "",
            "effects": [],
            "binding": { "kind": "free" },
            "tags": [],
            "isActive": false,
            "isHidden": false,
            "originTemplateId": null,
            "createdAt": "2026-05-12 00:00:00",
            "updatedAt": "2026-05-12 00:00:00",
        });
        let m: CharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert!(m.foundry_captured_labels.is_empty());
    }

    #[test]
    fn new_character_modifier_captured_labels_round_trip_json() {
        let json = serde_json::json!({
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Resilience",
            "description": "",
            "effects": [],
            "binding": { "kind": "advantage", "item_id": "merit-1" },
            "tags": [],
            "originTemplateId": null,
            "foundryCapturedLabels": ["Buff Modifier"],
        });
        let n: NewCharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert_eq!(n.foundry_captured_labels, vec!["Buff Modifier"]);
    }
}
