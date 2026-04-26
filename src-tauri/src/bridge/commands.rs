use std::collections::HashMap;

use tauri::State;

use crate::bridge::types::{CanonicalCharacter, SourceKind};
use crate::bridge::BridgeConn;

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
    let source_impl = conn
        .0
        .sources
        .get(&source)
        .cloned()
        .ok_or_else(|| format!("source {} not registered", source.as_str()))?;
    let payload = source_impl.build_set_attribute(&source_id, &name, &value);
    let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    send_to_source(&conn, source, text).await
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

async fn send_to_source(
    conn: &State<'_, BridgeConn>,
    kind: SourceKind,
    text: String,
) -> Result<(), String> {
    let tx = {
        let conns = conn.0.connections.lock().await;
        conns
            .get(&kind)
            .and_then(|c| c.outbound_tx.clone())
    };
    if let Some(tx) = tx {
        tx.send(text).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
