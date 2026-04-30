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
