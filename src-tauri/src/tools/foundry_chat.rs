// Tauri commands for outbound Foundry game.* helpers.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

use tauri::State;

use crate::bridge::{
    commands::send_to_source,
    foundry::{
        actions::game::{build_post_chat_as_actor, build_roll_v5_pool},
        types::{PostChatAsActorInput, RollV5PoolInput},
    },
    types::SourceKind,
    BridgeConn,
};

#[tauri::command]
pub async fn trigger_foundry_roll(
    conn: State<'_, BridgeConn>,
    input: RollV5PoolInput,
) -> Result<(), String> {
    let envelope = build_roll_v5_pool(&input)?;
    let text = serde_json::to_string(&envelope)
        .map_err(|e| format!("foundry/game.roll_v5_pool: serialize: {e}"))?;
    send_to_source(&conn, SourceKind::Foundry, text).await
}

#[tauri::command]
pub async fn post_foundry_chat(
    conn: State<'_, BridgeConn>,
    input: PostChatAsActorInput,
) -> Result<(), String> {
    let envelope = build_post_chat_as_actor(&input)?;
    let text = serde_json::to_string(&envelope)
        .map_err(|e| format!("foundry/game.post_chat_as_actor: serialize: {e}"))?;
    send_to_source(&conn, SourceKind::Foundry, text).await
}
