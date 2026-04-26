use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub current: String,
    pub max: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub controlled_by: String,
    pub attributes: Vec<Attribute>,
}

/// Inbound messages from the browser extension.
/// `#[serde(tag = "type", rename_all = "snake_case")]` means the JSON field
/// `"type": "characters"` deserialises to the `Characters` variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMsg {
    Characters { characters: Vec<Character> },
    CharacterUpdate { character: Character },
}
