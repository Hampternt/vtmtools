// Bridge layer: source-agnostic WS server, BridgeSource trait dispatch,
// canonical character type. Roll20 and Foundry slot in as source impls.
//
// Routing is port-based — :7423 dispatches to Roll20, :7424 to Foundry.
// Each source impl is a stateless transformer; shared connection state
// (per-source connected flag, outbound tx, merged characters map) lives
// in BridgeState.

pub mod commands;
pub mod foundry;
pub mod roll20;
pub mod source;
pub mod tls;
pub mod types;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

use crate::bridge::source::{BridgeSource, InboundEvent};
use crate::bridge::types::{CanonicalCharacter, CanonicalRoll, SourceInfo, SourceKind};

/// Capacity of the in-memory roll-history ring. ~80 rolls per typical 4-hour
/// session × 2.5 → 200 covers a session-and-a-half comfortably. Per
/// docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §15
/// open question 4: revisit only if user feedback shows entries dropping
/// mid-session.
const ROLL_HISTORY_CAPACITY: usize = 200;

pub struct ConnectionInfo {
    pub connected: bool,
    pub outbound_tx: Option<mpsc::Sender<String>>,
}

pub struct BridgeState {
    pub characters: Mutex<HashMap<String, CanonicalCharacter>>,
    pub connections: Mutex<HashMap<SourceKind, ConnectionInfo>>,
    pub source_info: Mutex<HashMap<SourceKind, SourceInfo>>,
    pub sources: HashMap<SourceKind, Arc<dyn BridgeSource>>,
    // roll_history uses tokio::sync::Mutex to match the existing in-memory
    // caches (characters, connections, source_info). tokio's Mutex doesn't
    // poison; push_roll / get_rolls are async because acquisition awaits.
    pub roll_history: Mutex<VecDeque<CanonicalRoll>>,
}

impl BridgeState {
    pub fn new(sources: HashMap<SourceKind, Arc<dyn BridgeSource>>) -> Self {
        let mut connections = HashMap::new();
        for &kind in sources.keys() {
            connections.insert(
                kind,
                ConnectionInfo { connected: false, outbound_tx: None },
            );
        }
        Self {
            characters: Mutex::new(HashMap::new()),
            connections: Mutex::new(connections),
            source_info: Mutex::new(HashMap::new()),
            sources,
            roll_history: Mutex::new(VecDeque::with_capacity(ROLL_HISTORY_CAPACITY)),
        }
    }

    /// Push a roll into the bounded ring. Newest-first ordering. Dedup by
    /// `source_id` — Foundry occasionally re-fires createChatMessage for the
    /// same message across sockets; pre-removing any existing entry collapses
    /// dupes without losing chronology.
    pub async fn push_roll(&self, roll: CanonicalRoll) {
        let mut ring = self.roll_history.lock().await;
        ring.retain(|r| r.source_id != roll.source_id);
        ring.push_front(roll);
        while ring.len() > ROLL_HISTORY_CAPACITY {
            ring.pop_back();
        }
    }

    /// Snapshot of the ring, newest-first. Cheap clone — capacity 200 of small
    /// structs (Vec<u8> dice arrays + an opaque JSON blob).
    pub async fn get_rolls(&self) -> Vec<CanonicalRoll> {
        self.roll_history.lock().await.iter().cloned().collect()
    }
}

/// Newtype wrapper so Tauri's `.manage()` / `State<>` can hold the Arc.
pub struct BridgeConn(pub Arc<BridgeState>);

/// Spawn one accept loop per registered source.
pub async fn start_servers(
    state: Arc<BridgeState>,
    handle: AppHandle,
    foundry_tls: Option<TlsAcceptor>,
) {
    if state.sources.contains_key(&SourceKind::Roll20) {
        tokio::spawn(accept_loop(
            state.clone(),
            handle.clone(),
            7423,
            SourceKind::Roll20,
            None,
        ));
    }
    if state.sources.contains_key(&SourceKind::Foundry) {
        // Refuse to spawn a plain-ws listener on :7424 when TLS init fails —
        // the Foundry module dials wss://, so plain ws would handshake-fail
        // and look exactly like a missing-cert error to the user. Better to
        // log loudly and disable the path than silently mislead them.
        if let Some(tls) = foundry_tls {
            tokio::spawn(accept_loop(
                state.clone(),
                handle.clone(),
                7424,
                SourceKind::Foundry,
                Some(tls),
            ));
        } else {
            eprintln!(
                "[bridge] Foundry source registered but TLS cert init failed — \
                 wss://localhost:7424 will NOT be served. Foundry connections \
                 disabled this session."
            );
        }
    }
}

async fn accept_loop(
    state: Arc<BridgeState>,
    handle: AppHandle,
    port: u16,
    kind: SourceKind,
    tls: Option<TlsAcceptor>,
) {
    let listener = match TcpListener::bind(("127.0.0.1", port)).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[bridge:{}] failed to bind 127.0.0.1:{port}: {e}", kind.as_str());
            return;
        }
    };

    loop {
        let (tcp, _addr) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[bridge:{}] TCP accept error: {e}", kind.as_str());
                continue;
            }
        };

        match &tls {
            Some(acceptor) => match acceptor.accept(tcp).await {
                Ok(tls_stream) => {
                    handle_connection(tls_stream, state.clone(), handle.clone(), kind).await;
                }
                Err(e) => eprintln!("[bridge:{}] TLS handshake error: {e}", kind.as_str()),
            },
            None => handle_connection(tcp, state.clone(), handle.clone(), kind).await,
        }
    }
}

async fn handle_connection<S>(
    stream: S,
    state: Arc<BridgeState>,
    handle: AppHandle,
    kind: SourceKind,
) where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let ws_stream = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("[bridge:{}] WS handshake error: {e}", kind.as_str());
            return;
        }
    };
    let (mut ws_sink, mut ws_source) = ws_stream.split();

    let source = match state.sources.get(&kind) {
        Some(s) => s.clone(),
        None => {
            eprintln!("[bridge:{}] no source registered", kind.as_str());
            return;
        }
    };

    // Channel: Tauri commands → WebSocket outbound
    let (tx, mut rx) = mpsc::channel::<String>(32);
    {
        let mut conns = state.connections.lock().await;
        conns.insert(
            kind,
            ConnectionInfo { connected: true, outbound_tx: Some(tx) },
        );
    }
    let _ = handle.emit(&format!("bridge://{}/connected", kind.as_str()), ());

    // Forward channel messages to the WS sink in a background task.
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sink.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Process inbound messages.
    while let Some(msg_result) = ws_source.next().await {
        let text = match msg_result {
            Ok(Message::Text(t)) => t,
            Ok(Message::Close(_)) | Err(_) => break,
            _ => continue,
        };
        let parsed: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[bridge:{}] parse error: {e}  raw: {text}", kind.as_str());
                continue;
            }
        };

        // Foundry-only: capture Hello metadata into BridgeState::source_info
        // before delegating to the (stateless) BridgeSource trait, and route
        // Error envelopes as Tauri events without trait dispatch.
        //
        // Typed deserialization: the field shapes are pinned by FoundryInbound
        // — a future rename in that enum becomes a compile error here, not a
        // silent runtime drop. The clone is negligible (Hello/Error frames
        // are infrequent and small); the trait handler will deserialize the
        // same value again in the unhandled branches, which is fine.
        if kind == SourceKind::Foundry {
            match serde_json::from_value::<crate::bridge::foundry::types::FoundryInbound>(parsed.clone()) {
                Ok(crate::bridge::foundry::types::FoundryInbound::Hello {
                    protocol_version, world_id, world_title,
                    system_id, system_version, capabilities,
                }) => {
                    let info = SourceInfo {
                        world_id,
                        world_title,
                        system_id,
                        system_version,
                        protocol_version: protocol_version.unwrap_or(0),
                        capabilities: capabilities.unwrap_or_else(|| vec!["actors".to_string()]),
                    };
                    let mut store = state.source_info.lock().await;
                    store.insert(kind, info);
                }
                Ok(crate::bridge::foundry::types::FoundryInbound::Error {
                    refers_to, code, message, ..
                }) => {
                    let payload = serde_json::json!({
                        "refers_to": refers_to,
                        "code":      code,
                        "message":   message,
                    });
                    let _ = handle.emit("bridge://foundry/error", payload);
                    continue;
                }
                Ok(_) | Err(_) => {} // Actors/ActorUpdate handled below by handle_inbound; deserialization errors logged there.
            }
        }

        match source.handle_inbound(parsed).await {
            Ok(events) => {
                for event in events {
                    match event {
                        InboundEvent::CharactersUpdated(updated) if !updated.is_empty() => {
                            let mut chars = state.characters.lock().await;
                            for c in updated {
                                chars.insert(c.key(), c);
                            }
                            let snapshot: Vec<CanonicalCharacter> =
                                chars.values().cloned().collect();
                            drop(chars);
                            let _ = handle.emit("bridge://characters-updated", snapshot);
                        }
                        InboundEvent::CharactersUpdated(_) => {}
                        InboundEvent::RollReceived(roll) => {
                            state.push_roll(roll.clone()).await;
                            let _ = handle.emit("bridge://roll-received", &roll);
                        }
                        InboundEvent::ItemDeleted { source, source_id, item_id } => {
                            // Lookup the pool via Tauri's managed-state map.
                            // DbState is registered in lib.rs::run setup,
                            // before bridge servers are spawned, so this
                            // never fails in practice. If it ever did
                            // (no pool managed), skip the reap silently —
                            // a stale orphan card is preferable to a panic.
                            let pool = match handle.try_state::<crate::DbState>() {
                                Some(s) => Arc::clone(&s.0),
                                None => {
                                    eprintln!(
                                        "[bridge:{}] ItemDeleted: no DbState managed, skipping reap",
                                        kind.as_str()
                                    );
                                    continue;
                                }
                            };
                            match crate::db::modifier::db_delete_by_advantage_binding(
                                &pool, &source, &source_id, &item_id,
                            ).await {
                                Ok(ids) if !ids.is_empty() => {
                                    let _ = handle.emit(
                                        "modifiers://rows-reaped",
                                        serde_json::json!({ "ids": ids }),
                                    );
                                }
                                Ok(_) => {} // idempotent — no rows matched, nothing to emit
                                Err(e) => {
                                    eprintln!(
                                        "[bridge:{}] ItemDeleted reap failed: {e}",
                                        kind.as_str()
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => eprintln!("[bridge:{}] handler error: {e}", kind.as_str()),
        }
    }

    // Disconnect cleanup.
    {
        let mut conns = state.connections.lock().await;
        conns.insert(
            kind,
            ConnectionInfo { connected: false, outbound_tx: None },
        );
    }
    {
        let mut info = state.source_info.lock().await;
        info.remove(&kind);
    }
    let _ = handle.emit(&format!("bridge://{}/disconnected", kind.as_str()), ());
}

#[cfg(test)]
mod ring_tests {
    use super::*;
    use crate::bridge::types::{CanonicalRoll, RollSplat, SourceKind};
    use serde_json::json;

    fn make_roll(id: &str) -> CanonicalRoll {
        CanonicalRoll {
            source: SourceKind::Foundry,
            source_id: id.into(),
            actor_id: None,
            actor_name: None,
            timestamp: None,
            splat: RollSplat::Mortal,
            flavor: String::new(),
            formula: String::new(),
            basic_results: vec![],
            advanced_results: vec![],
            total: 0,
            difficulty: None,
            criticals: 0,
            messy: false,
            bestial: false,
            brutal: false,
            raw: json!({}),
        }
    }

    #[tokio::test]
    async fn ring_dedups_by_source_id() {
        let state = BridgeState::new(HashMap::new());
        state.push_roll(make_roll("a")).await;
        state.push_roll(make_roll("b")).await;
        state.push_roll(make_roll("a")).await; // dup of first
        let rolls = state.get_rolls().await;
        assert_eq!(rolls.len(), 2);
        assert_eq!(rolls[0].source_id, "a", "newest-first; re-pushed 'a' is newest");
        assert_eq!(rolls[1].source_id, "b");
    }

    #[tokio::test]
    async fn ring_caps_at_capacity() {
        let state = BridgeState::new(HashMap::new());
        for i in 0..(ROLL_HISTORY_CAPACITY + 50) {
            state.push_roll(make_roll(&format!("id_{i}"))).await;
        }
        assert_eq!(state.get_rolls().await.len(), ROLL_HISTORY_CAPACITY);
    }

    #[tokio::test]
    async fn ring_newest_first_ordering() {
        let state = BridgeState::new(HashMap::new());
        state.push_roll(make_roll("first")).await;
        state.push_roll(make_roll("second")).await;
        state.push_roll(make_roll("third")).await;
        let rolls = state.get_rolls().await;
        assert_eq!(rolls.len(), 3);
        assert_eq!(rolls[0].source_id, "third");
        assert_eq!(rolls[1].source_id, "second");
        assert_eq!(rolls[2].source_id, "first");
    }
}
