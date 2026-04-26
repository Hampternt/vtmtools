// Bridge layer: source-agnostic WS server, BridgeSource trait dispatch,
// canonical character type. Roll20 and Foundry slot in as source impls.
//
// Routing is port-based — :7423 dispatches to Roll20, :7424 to Foundry.
// Each source impl is a stateless transformer; shared connection state
// (per-source connected flag, outbound tx, merged characters map) lives
// in BridgeState.

pub mod roll20;
pub mod source;
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
use crate::bridge::types::{CanonicalCharacter, SourceKind};

pub struct ConnectionInfo {
    pub connected: bool,
    pub outbound_tx: Option<mpsc::Sender<String>>,
}

pub struct BridgeState {
    pub characters: Mutex<HashMap<String, CanonicalCharacter>>,
    pub connections: Mutex<HashMap<SourceKind, ConnectionInfo>>,
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
        tokio::spawn(accept_loop(
            state.clone(),
            handle.clone(),
            7424,
            SourceKind::Foundry,
            foundry_tls,
        ));
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
    let _ = handle.emit(&format!("bridge://{}/disconnected", kind.as_str()), ());
}
