use tauri::State;

use crate::roll20::types::{Character, OutboundMsg, Roll20Conn};

/// Returns all characters currently known from the Roll20 session.
/// Returns an empty vec if no extension is connected.
#[tauri::command]
pub async fn get_roll20_characters(
    conn: State<'_, Roll20Conn>,
) -> Result<Vec<Character>, String> {
    let chars = conn.0.characters.lock().await;
    Ok(chars.values().cloned().collect())
}

/// Returns true if the browser extension is currently connected.
#[tauri::command]
pub async fn get_roll20_status(
    conn: State<'_, Roll20Conn>,
) -> Result<bool, String> {
    Ok(*conn.0.connected.lock().await)
}

/// Asks the extension to re-read all characters from Roll20.
/// No-op if no extension is connected.
#[tauri::command]
pub async fn refresh_roll20_data(
    conn: State<'_, Roll20Conn>,
) -> Result<(), String> {
    let tx = conn.0.outbound_tx.lock().await.clone();
    if let Some(tx) = tx {
        let msg = serde_json::to_string(&OutboundMsg::Refresh)
            .map_err(|e| e.to_string())?;
        tx.send(msg).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Sends a chat message into the Roll20 game via the extension.
/// No-op if no extension is connected.
#[tauri::command]
pub async fn send_roll20_chat(
    message: String,
    conn: State<'_, Roll20Conn>,
) -> Result<(), String> {
    let tx = conn.0.outbound_tx.lock().await.clone();
    if let Some(tx) = tx {
        let msg = serde_json::to_string(&OutboundMsg::SendChat { message })
            .map_err(|e| e.to_string())?;
        tx.send(msg).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Writes a single attribute on a Roll20 character sheet via the extension.
/// No-op if no extension is connected.
#[tauri::command]
pub async fn set_roll20_attribute(
    character_id: String,
    name: String,
    value: String,
    conn: State<'_, Roll20Conn>,
) -> Result<(), String> {
    let tx = conn.0.outbound_tx.lock().await.clone();
    if let Some(tx) = tx {
        let msg = serde_json::to_string(&OutboundMsg::SetAttribute {
            character_id,
            name,
            value,
        })
        .map_err(|e| e.to_string())?;
        tx.send(msg).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
