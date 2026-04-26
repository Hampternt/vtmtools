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
