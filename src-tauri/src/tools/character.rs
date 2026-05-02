//! Field-level character editing router. Composes the existing live-write
//! pipeline (`bridge::commands::do_set_attribute`) and the new saved-side
//! patcher (`db::saved_character::db_patch_field`) under an explicit
//! `WriteTarget`.
//!
//! See `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md`.

use serde::Deserialize;
use sqlx::Row;
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::State;

use crate::bridge::types::SourceKind;
use crate::bridge::BridgeState;
use crate::shared::canonical_fields::{canonical_to_roll20_attr, ALLOWED_NAMES};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteTarget {
    Live,
    Saved,
    Both,
}

#[tauri::command]
pub async fn character_set_field(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    do_set_field(&db.0, &bridge.0, target, source, source_id, name, value).await
}

/// Inner implementation taking owned `Arc<BridgeState>` and `&SqlitePool` so
/// it's testable without a Tauri runtime.
pub(crate) async fn do_set_field(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    if !ALLOWED_NAMES.contains(&name.as_str()) {
        return Err(format!("character/set_field: unknown field '{name}'"));
    }

    if target != WriteTarget::Saved
        && source == SourceKind::Roll20
        && canonical_to_roll20_attr(&name).is_none()
    {
        return Err(
            "character/set_field: Roll20 live editing of canonical names not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    match target {
        WriteTarget::Saved => {
            crate::db::saved_character::db_patch_field(
                pool,
                saved_id.unwrap(),
                &name,
                &value,
            )
            .await
        }
        WriteTarget::Live => forward_live(bridge_state, source, &source_id, &name, &value).await,
        WriteTarget::Both => {
            crate::db::saved_character::db_patch_field(
                pool,
                saved_id.unwrap(),
                &name,
                &value,
            )
            .await
            .map_err(|e| format!("character/set_field: saved write failed: {e}"))?;
            forward_live(bridge_state, source, &source_id, &name, &value)
                .await
                .map_err(|e| format!("character/set_field: saved updated, live failed: {e}"))
        }
    }
}

async fn lookup_saved_id(
    pool: &SqlitePool,
    source: SourceKind,
    source_id: &str,
) -> Result<i64, String> {
    let row = sqlx::query(
        "SELECT id FROM saved_characters WHERE source = ? AND source_id = ?",
    )
    .bind(source.as_str())
    .bind(source_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("character/set_field: {e}"))?
    .ok_or_else(|| {
        format!(
            "character/set_field: no saved row for {}/{}",
            source.as_str(),
            source_id
        )
    })?;
    Ok(row.get("id"))
}

async fn forward_live(
    state: &Arc<BridgeState>,
    source: SourceKind,
    source_id: &str,
    name: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let s = match value {
        serde_json::Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    crate::bridge::commands::do_set_attribute(
        state,
        source,
        source_id.to_string(),
        name.to_string(),
        s,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::source::BridgeSource;
    use crate::bridge::types::CanonicalCharacter;
    use crate::bridge::ConnectionInfo;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    struct StubFoundrySource;

    #[async_trait]
    impl BridgeSource for StubFoundrySource {
        async fn handle_inbound(&self, _msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
            Ok(vec![])
        }
        fn build_set_attribute(
            &self,
            _source_id: &str,
            _name: &str,
            _value: &str,
        ) -> Result<Value, String> {
            Ok(serde_json::json!({"type": "stub"}))
        }
        fn build_refresh(&self) -> Value {
            serde_json::json!({"type": "refresh"})
        }
    }

    /// Builds a stub bridge state. Returns the channel receiver too — the
    /// caller MUST bind it (e.g. `let (state, _rx) = ...`) so the channel
    /// stays open for the test's lifetime. If we dropped the receiver here,
    /// `tx.send()` in `do_set_attribute` would fail with `SendError`, breaking
    /// the no-op semantics the live-write path relies on.
    fn make_bridge_state(
        connected: bool,
    ) -> (Arc<BridgeState>, Option<tokio::sync::mpsc::Receiver<String>>) {
        make_bridge_state_with_source(connected, Arc::new(StubFoundrySource))
    }

    fn make_bridge_state_with_source(
        connected: bool,
        source_impl: Arc<dyn BridgeSource>,
    ) -> (Arc<BridgeState>, Option<tokio::sync::mpsc::Receiver<String>>) {
        let mut sources: HashMap<SourceKind, Arc<dyn BridgeSource>> = HashMap::new();
        sources.insert(SourceKind::Foundry, source_impl);

        let mut connections = HashMap::new();
        let rx_opt = if connected {
            let (tx, rx) = tokio::sync::mpsc::channel::<String>(8);
            connections.insert(
                SourceKind::Foundry,
                ConnectionInfo {
                    connected: true,
                    outbound_tx: Some(tx),
                },
            );
            Some(rx)
        } else {
            None
        };

        let state = Arc::new(BridgeState {
            characters: Mutex::new(HashMap::new()),
            connections: Mutex::new(connections),
            source_info: Mutex::new(HashMap::new()),
            sources,
        });
        (state, rx_opt)
    }

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn sample_canonical() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "abc".to_string(),
            name: "Test".to_string(),
            controlled_by: None,
            hunger: Some(2),
            health: None,
            willpower: None,
            humanity: None,
            humanity_stains: None,
            blood_potency: None,
            raw: serde_json::json!({}),
        }
    }

    /// Stub source whose build_set_attribute always errs — used to exercise
    /// the partial-success path on Both targets.
    struct AlwaysErrSource;

    #[async_trait]
    impl BridgeSource for AlwaysErrSource {
        async fn handle_inbound(&self, _msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
            Ok(vec![])
        }
        fn build_set_attribute(
            &self,
            _source_id: &str,
            _name: &str,
            _value: &str,
        ) -> Result<Value, String> {
            Err("stub forced failure".to_string())
        }
        fn build_refresh(&self) -> Value {
            serde_json::json!({"type": "refresh"})
        }
    }

    async fn seed_saved_row(pool: &SqlitePool, source_id: &str) {
        let canonical = sample_canonical();
        let canonical_json = serde_json::to_string(&canonical).unwrap();
        sqlx::query(
            "INSERT INTO saved_characters
             (source, source_id, foundry_world, name, canonical_json)
             VALUES ('foundry', ?, NULL, 'Test', ?)",
        )
        .bind(source_id)
        .bind(&canonical_json)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unknown_name_returns_err_immediately() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "xyzzy".to_string(),
            serde_json::json!(0),
        )
        .await
        .unwrap_err();
        assert!(err.contains("unknown field 'xyzzy'"), "got: {err}");
    }

    #[tokio::test]
    async fn roll20_live_canonical_returns_unsupported_err() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Roll20,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(2),
        )
        .await
        .unwrap_err();
        assert!(
            err.contains("Roll20 live editing of canonical names not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn saved_target_no_row_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Saved,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(2),
        )
        .await
        .unwrap_err();
        assert!(
            err.contains("no saved row for foundry/abc"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn saved_target_happy_path_writes_db() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row(&pool, "abc").await;

        do_set_field(
            &pool,
            &state,
            WriteTarget::Saved,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(5),
        )
        .await
        .unwrap();

        let row =
            sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let json: String = row.get("canonical_json");
        let updated: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.hunger, Some(5));
    }

    #[tokio::test]
    async fn live_target_disconnected_source_no_op_succeeds() {
        // do_set_attribute is no-op when the source has no outbound channel.
        // Mirrors existing bridge_set_attribute semantics.
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(false); // disconnected
        let res = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(3),
        )
        .await;
        assert!(res.is_ok(), "live no-op should be Ok, got: {:?}", res);
    }

    #[tokio::test]
    async fn both_target_saved_succeeds_then_live_succeeds() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row(&pool, "abc").await;

        do_set_field(
            &pool,
            &state,
            WriteTarget::Both,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(4),
        )
        .await
        .unwrap();

        let row = sqlx::query(
            "SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let json: String = row.get("canonical_json");
        let updated: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.hunger, Some(4));
    }

    #[tokio::test]
    async fn both_partial_success_when_live_fails() {
        // Spec §6: when target=Both, saved succeeds, live fails, we get the
        // partial-success error string AND the saved row reflects the change.
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state_with_source(true, Arc::new(AlwaysErrSource));
        seed_saved_row(&pool, "abc").await;

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Both,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(4),
        )
        .await
        .unwrap_err();

        assert!(
            err.starts_with("character/set_field: saved updated, live failed:"),
            "got: {err}"
        );

        // Saved row was still patched (saved-first ordering).
        let row = sqlx::query(
            "SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let json: String = row.get("canonical_json");
        let updated: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.hunger, Some(4));
    }
}
