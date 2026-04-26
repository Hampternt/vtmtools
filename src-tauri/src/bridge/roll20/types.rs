use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;

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

/// Outbound messages sent to the browser extension.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMsg {
    Refresh,
    SendChat { message: String },
    SetAttribute {
        character_id: String,
        name: String,
        value: String,
    },
}

/// Shared in-memory state for the Roll20 connection.
pub struct Roll20State {
    pub characters: Mutex<HashMap<String, Character>>,
    pub connected: Mutex<bool>,
    /// Sender half of the channel used to push messages to the WebSocket.
    /// None when no extension is connected.
    pub outbound_tx: Mutex<Option<mpsc::Sender<String>>>,
}

impl Roll20State {
    pub fn new() -> Self {
        Self {
            characters: Mutex::new(HashMap::new()),
            connected: Mutex::new(false),
            outbound_tx: Mutex::new(None),
        }
    }
}

/// Newtype wrapper so Tauri's `.manage()` / `State<>` can hold the Arc.
pub struct Roll20Conn(pub Arc<Roll20State>);
