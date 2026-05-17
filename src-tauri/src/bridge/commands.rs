use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;

use crate::bridge::foundry::actions::bridge as bridge_actions;
use crate::bridge::types::{
    CanonicalCharacter, CanonicalRoll, CanonicalWorldItem, SourceInfo, SourceKind,
};
use crate::bridge::BridgeConn;
use crate::bridge::BridgeState;

/// Returns per-source connection state. Sources without a connected
/// client report `false`. The frontend uses this to render per-source
/// status pips.
#[tauri::command]
pub async fn bridge_get_status(
    conn: State<'_, BridgeConn>,
) -> Result<HashMap<SourceKind, bool>, String> {
    let conns = conn.0.connections.lock().await;
    Ok(conns.iter().map(|(k, v)| (*k, v.connected)).collect())
}

/// Returns every character known across every source, in canonical form.
#[tauri::command]
pub async fn bridge_get_characters(
    conn: State<'_, BridgeConn>,
) -> Result<Vec<CanonicalCharacter>, String> {
    let chars = conn.0.characters.lock().await;
    Ok(chars.values().cloned().collect())
}

/// Returns a newest-first snapshot of the in-memory roll-history ring
/// (capacity 200, dedup by `source_id`). The frontend rolls store calls
/// this once on mount, then subscribes to `bridge://roll-received` for
/// incremental updates. Per
/// docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §7.
#[tauri::command]
pub async fn bridge_get_rolls(
    conn: State<'_, BridgeConn>,
) -> Result<Vec<CanonicalRoll>, String> {
    Ok(conn.0.get_rolls().await)
}

/// Returns every world-level item known across every source. Used by
/// the frontend on initial load; live updates flow through the
/// `bridge://foundry/items-updated` event.
#[tauri::command]
pub async fn bridge_get_world_items(
    conn: State<'_, BridgeConn>,
) -> Result<Vec<CanonicalWorldItem>, String> {
    let store = conn.0.world_items.lock().await;
    Ok(store.values().flat_map(|m| m.values().cloned()).collect())
}

/// Inner logic shared by the Tauri command and any non-IPC caller (the new
/// character_set_field router). Operates directly on `Arc<BridgeState>` so
/// callers don't need to hold a `State<'_, BridgeConn>`.
pub(crate) async fn do_set_attribute(
    state: &Arc<BridgeState>,
    source: SourceKind,
    source_id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    let source_impl = state
        .sources
        .get(&source)
        .cloned()
        .ok_or_else(|| format!("source {} not registered", source.as_str()))?;
    let payload = source_impl
        .build_set_attribute(&source_id, &name, &value)
        .map_err(|e| format!("bridge/set_attribute: {e}"))?;
    let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    send_to_source_inner(state, source, text).await
}

/// Asks the named source to push attribute `name` = `value` for the given
/// `source_id` (Roll20 → set_attribute on a sheet; Foundry → actor.update
/// or item create depending on translation). No-op if the source isn't
/// connected.
#[tauri::command]
pub async fn bridge_set_attribute(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    source_id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    do_set_attribute(&conn.0, source, source_id, name, value).await
}

pub(crate) async fn send_to_source_inner(
    state: &Arc<BridgeState>,
    kind: SourceKind,
    text: String,
) -> Result<(), String> {
    let tx = {
        let conns = state.connections.lock().await;
        conns.get(&kind).and_then(|c| c.outbound_tx.clone())
    };
    if let Some(tx) = tx {
        tx.send(text).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) async fn send_to_source(
    conn: &State<'_, BridgeConn>,
    kind: SourceKind,
    text: String,
) -> Result<(), String> {
    send_to_source_inner(&conn.0, kind, text).await
}

/// Asks one source — or all sources if `source` is `None` — to resend
/// their full character snapshot.
#[tauri::command]
pub async fn bridge_refresh(
    conn: State<'_, BridgeConn>,
    source: Option<SourceKind>,
) -> Result<(), String> {
    let kinds: Vec<SourceKind> = match source {
        Some(k) => vec![k],
        None => conn.0.sources.keys().copied().collect(),
    };
    for kind in kinds {
        if let Some(impl_) = conn.0.sources.get(&kind) {
            let payload = impl_.build_refresh();
            let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
            // Best-effort — disconnected sources silently skip.
            let _ = send_to_source(&conn, kind, text).await;
        }
    }
    Ok(())
}

/// Returns the captured Hello metadata for a connected source, or None if
/// the source is not currently connected. Async to match the existing
/// bridge command surface (none of these have I/O — the async signature
/// is consistency, not necessity).
#[tauri::command]
pub async fn bridge_get_source_info(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
) -> Result<Option<SourceInfo>, String> {
    let info = conn.0.source_info.lock().await;
    Ok(info.get(&source).cloned())
}

/// Send `bridge.subscribe { collection }` to the named source. No-op
/// if the source isn't connected. Per-source dispatch: in v1 only
/// Foundry implements bridge.* subscriptions; Roll20 returns an error
/// string rather than silently ignoring — gives the frontend a clearer
/// signal.
#[tauri::command]
pub async fn bridge_subscribe(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    collection: String,
) -> Result<(), String> {
    if source != SourceKind::Foundry {
        return Err(format!(
            "bridge/subscribe: source {source:?} does not support subscriptions"
        ));
    }
    let payload = bridge_actions::build_subscribe(&collection);
    let text = serde_json::to_string(&payload)
        .map_err(|e| format!("bridge/subscribe: serialize: {e}"))?;
    send_to_source_inner(&conn.0, source, text).await
}

/// Send `bridge.unsubscribe { collection }` to the named source. No-op
/// if the source isn't connected. v1 only Foundry implements bridge.*
/// subscriptions; Roll20 returns an error string.
#[tauri::command]
pub async fn bridge_unsubscribe(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    collection: String,
) -> Result<(), String> {
    if source != SourceKind::Foundry {
        return Err(format!(
            "bridge/unsubscribe: source {source:?} does not support subscriptions"
        ));
    }
    let payload = bridge_actions::build_unsubscribe(&collection);
    let text = serde_json::to_string(&payload)
        .map_err(|e| format!("bridge/unsubscribe: serialize: {e}"))?;
    send_to_source_inner(&conn.0, source, text).await
}
