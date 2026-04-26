pub mod types;
pub mod commands;
pub mod translate;

pub use types::{Roll20Conn, Roll20State};

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::bridge::roll20::types::{Character, InboundMsg};

/// Starts the WebSocket server on 127.0.0.1:7423.
/// Accepts one connection at a time. Emits Tauri events to the frontend:
///   - "roll20://connected"
///   - "roll20://disconnected"
///   - "roll20://characters-updated" with payload Vec<Character>
///
/// Spawn this with `tauri::async_runtime::spawn()` during app setup.
pub async fn start_ws_server(state: Arc<Roll20State>, handle: AppHandle) {
    let listener = TcpListener::bind("127.0.0.1:7423")
        .await
        .expect("Failed to bind WebSocket server on port 7423");

    loop {
        let (tcp_stream, _addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[roll20] TCP accept error: {e}");
                continue;
            }
        };

        let ws_stream = match tokio_tungstenite::accept_async(tcp_stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("[roll20] WebSocket handshake error: {e}");
                continue;
            }
        };

        let (mut ws_sink, mut ws_source) = ws_stream.split();

        // Channel: Tauri commands → WebSocket outbound
        let (tx, mut rx) = mpsc::channel::<String>(32);
        *state.outbound_tx.lock().await = Some(tx);
        *state.connected.lock().await = true;
        let _ = handle.emit("roll20://connected", ());

        // Spawn a task to forward channel messages to the WebSocket sink.
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sink.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Process inbound messages from the extension.
        while let Some(msg_result) = ws_source.next().await {
            let text = match msg_result {
                Ok(Message::Text(t)) => t,
                Ok(Message::Close(_)) | Err(_) => break,
                _ => continue,
            };

            match serde_json::from_str::<InboundMsg>(&text) {
                Ok(InboundMsg::Characters { characters }) => {
                    let map: HashMap<String, Character> = characters
                        .into_iter()
                        .map(|c| (c.id.clone(), c))
                        .collect();
                    *state.characters.lock().await = map;
                    let all = all_chars(&state).await;
                    let _ = handle.emit("roll20://characters-updated", all);
                }
                Ok(InboundMsg::CharacterUpdate { character }) => {
                    state
                        .characters
                        .lock()
                        .await
                        .insert(character.id.clone(), character);
                    let all = all_chars(&state).await;
                    let _ = handle.emit("roll20://characters-updated", all);
                }
                Err(e) => eprintln!("[roll20] Parse error: {e}  raw: {text}"),
            }
        }

        // Extension disconnected — clean up.
        *state.connected.lock().await = false;
        *state.outbound_tx.lock().await = None;
        let _ = handle.emit("roll20://disconnected", ());
    }
}

async fn all_chars(state: &Roll20State) -> Vec<Character> {
    state.characters.lock().await.values().cloned().collect()
}

// ── BridgeSource impl ───────────────────────────────────────────────────────

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::bridge::source::BridgeSource;
use crate::bridge::types::{CanonicalCharacter, SourceKind};

/// Stateless adapter — parses Roll20 wire messages into canonical characters
/// and builds the outbound counterparts. Shared connection state lives in
/// BridgeState (the new generic server), not in this struct.
pub struct Roll20Source;

#[async_trait]
impl BridgeSource for Roll20Source {
    fn kind(&self) -> SourceKind {
        SourceKind::Roll20
    }

    async fn handle_inbound(&self, msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
        let parsed: InboundMsg = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        let chars = match parsed {
            InboundMsg::Characters { characters } => characters,
            InboundMsg::CharacterUpdate { character } => vec![character],
        };
        Ok(chars.iter().map(translate::to_canonical).collect())
    }

    fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Value {
        json!({
            "type": "set_attribute",
            "character_id": source_id,
            "name": name,
            "value": value,
        })
    }

    fn build_refresh(&self) -> Value {
        json!({ "type": "refresh" })
    }
}
