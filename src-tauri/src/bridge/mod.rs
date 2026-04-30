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

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio_rustls::TlsAcceptor;
use tokio_tungstenite::tungstenite::Message;

use crate::bridge::source::BridgeSource;
use crate::bridge::types::{CanonicalCharacter, SourceInfo, SourceKind};

pub struct ConnectionInfo {
    pub connected: bool,
    pub outbound_tx: Option<mpsc::Sender<String>>,
}

pub struct BridgeState {
    pub characters: Mutex<HashMap<String, CanonicalCharacter>>,
    pub connections: Mutex<HashMap<SourceKind, ConnectionInfo>>,
    pub source_info: Mutex<HashMap<SourceKind, SourceInfo>>,
    pub sources: HashMap<SourceKind, Arc<dyn BridgeSource>>,
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
        }
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
        if kind == SourceKind::Foundry {
            if let Some("hello") = parsed.get("type").and_then(|t| t.as_str()) {
                let info = SourceInfo {
                    world_id: parsed.get("world_id").and_then(|v| v.as_str()).map(String::from),
                    world_title: parsed.get("world_title").and_then(|v| v.as_str()).map(String::from),
                    system_id: parsed.get("system_id").and_then(|v| v.as_str()).map(String::from),
                    system_version: parsed.get("system_version").and_then(|v| v.as_str()).map(String::from),
                    protocol_version: parsed
                        .get("protocol_version")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as u32,
                    capabilities: parsed
                        .get("capabilities")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_else(|| vec!["actors".to_string()]),
                };
                let mut store = state.source_info.lock().await;
                store.insert(kind, info);
            }
            if let Some("error") = parsed.get("type").and_then(|t| t.as_str()) {
                let payload = serde_json::json!({
                    "refers_to": parsed.get("refers_to").and_then(|v| v.as_str()).unwrap_or(""),
                    "code":      parsed.get("code").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "message":   parsed.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                });
                let _ = handle.emit("bridge://foundry/error", payload);
                continue;
            }
        }

        match source.handle_inbound(parsed).await {
            Ok(updated) if !updated.is_empty() => {
                let mut chars = state.characters.lock().await;
                for c in updated {
                    chars.insert(c.key(), c);
                }
                let snapshot: Vec<CanonicalCharacter> = chars.values().cloned().collect();
                drop(chars);
                let _ = handle.emit("bridge://characters-updated", snapshot);
            }
            Ok(_) => {}
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
