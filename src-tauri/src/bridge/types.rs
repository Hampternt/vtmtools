use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Roll20,
    Foundry,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Roll20 => "roll20",
            SourceKind::Foundry => "foundry",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrack {
    pub max: u8,
    pub superficial: u8,
    pub aggravated: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalCharacter {
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub controlled_by: Option<String>,
    pub hunger: Option<u8>,
    pub health: Option<HealthTrack>,
    pub willpower: Option<HealthTrack>,
    pub humanity: Option<u8>,
    pub humanity_stains: Option<u8>,
    pub blood_potency: Option<u8>,
    pub raw: serde_json::Value,
}

impl CanonicalCharacter {
    pub fn key(&self) -> String {
        format!("{}:{}", self.source.as_str(), self.source_id)
    }
}

/// Per-source connection metadata captured from the source's Hello frame.
/// Populated by `bridge::handle_connection` on Hello receipt (pre-trait,
/// because `BridgeSource::handle_inbound` is stateless and cannot write to
/// `BridgeState`); cleared by the same function's disconnect-cleanup block.
/// Not persisted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub world_id: Option<String>,
    pub world_title: Option<String>,
    pub system_id: Option<String>,
    pub system_version: Option<String>,
    pub protocol_version: u32,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RollSplat {
    Mortal,
    Vampire,
    Werewolf,
    Hunter,
    Unknown,
}

/// A source-agnostic roll result. Each source's `BridgeSource` impl
/// decodes its own raw chat-message / roll shape into this canonical form.
///
/// `source_id` reuses the source's stable per-roll ID (Foundry: chat
/// `_id`); the bridge ring dedups by this key.
///
/// `messy` / `bestial` / `brutal` / `criticals` are computed by the
/// translator from `basic_results` + `advanced_results` — Foundry does
/// NOT persist `rollMessageData` on the chat message
/// (see docs/reference/foundry-vtm5e-rolls.md §"Chat message shape").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalRoll {
    pub source: SourceKind,
    pub source_id: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    /// ISO-8601. None if the source's frame didn't carry a timestamp.
    pub timestamp: Option<String>,
    pub splat: RollSplat,
    pub flavor: String,
    pub formula: String,
    pub basic_results: Vec<u8>,
    pub advanced_results: Vec<u8>,
    pub total: u32,
    pub difficulty: Option<u32>,
    pub criticals: u32,
    pub messy: bool,
    pub bestial: bool,
    pub brutal: bool,
    pub raw: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_roll_serde_roundtrip() {
        let roll = CanonicalRoll {
            source: SourceKind::Foundry,
            source_id: "msg_abc123".into(),
            actor_id: Some("actor_xyz".into()),
            actor_name: Some("Doe, John".into()),
            timestamp: Some("2026-05-10T12:00:00Z".into()),
            splat: RollSplat::Vampire,
            flavor: "Strength + Brawl".into(),
            formula: "5dv cs>5 + 2dg cs>5".into(),
            basic_results: vec![3, 7, 9, 10, 6],
            advanced_results: vec![2, 8],
            total: 4,
            difficulty: Some(4),
            criticals: 1,
            messy: false,
            bestial: false,
            brutal: false,
            raw: json!({ "anything": "goes" }),
        };
        let s = serde_json::to_string(&roll).unwrap();
        let back: CanonicalRoll = serde_json::from_str(&s).unwrap();
        assert_eq!(back.source_id, roll.source_id);
        assert_eq!(back.splat, RollSplat::Vampire);
        assert_eq!(back.basic_results, vec![3, 7, 9, 10, 6]);
        assert_eq!(back.criticals, 1);
        assert_eq!(back.raw, json!({ "anything": "goes" }));
    }

    #[test]
    fn roll_splat_serde_snake_case() {
        let s = serde_json::to_string(&RollSplat::Vampire).unwrap();
        assert_eq!(s, "\"vampire\"");
        let back: RollSplat = serde_json::from_str("\"werewolf\"").unwrap();
        assert_eq!(back, RollSplat::Werewolf);
    }
}
