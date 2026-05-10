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
use crate::shared::canonical_fields::{canonical_to_roll20_attr, is_allowed_name};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteTarget {
    Live,
    Saved,
    Both,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum FeatureType {
    Merit,
    Flaw,
    Background,
    Boon,
}

impl FeatureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureType::Merit      => "merit",
            FeatureType::Flaw       => "flaw",
            FeatureType::Background => "background",
            FeatureType::Boon       => "boon",
        }
    }
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
    if !is_allowed_name(&name) {
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

#[tauri::command]
pub async fn character_add_advantage(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    name: String,
    description: String,
    points: i32,
) -> Result<(), String> {
    do_add_advantage(
        &db.0, &bridge.0, target, source, source_id,
        featuretype, name, description, points,
    ).await
}

pub(crate) async fn do_add_advantage(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    name: String,
    description: String,
    points: i32,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("character/add_advantage: empty name".to_string());
    }
    if !(0..=10).contains(&points) {
        return Err(format!(
            "character/add_advantage: points {points} out of range 0..=10"
        ));
    }

    if target != WriteTarget::Saved && source == SourceKind::Roll20 {
        return Err(
            "character/add_advantage: Roll20 live editing of advantages not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    let do_saved = || async {
        crate::db::saved_character::db_add_advantage(
            pool,
            saved_id.unwrap(),
            featuretype.as_str(),
            &name,
            &description,
            points,
        )
        .await
    };

    let do_live = || async {
        let payload = crate::bridge::foundry::actions::actor::build_create_feature(
            &source_id,
            featuretype.as_str(),
            &name,
            &description,
            points,
        )
        .map_err(|e| format!("character/add_advantage: {e}"))?;
        let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        crate::bridge::commands::send_to_source_inner(bridge_state, source, text).await
    };

    match target {
        WriteTarget::Saved => do_saved().await,
        WriteTarget::Live  => do_live().await,
        WriteTarget::Both  => {
            do_saved().await
                .map_err(|e| format!("character/add_advantage: saved write failed: {e}"))?;
            do_live().await
                .map_err(|e| format!(
                    "character/add_advantage: saved updated, live failed: {e}"
                ))
        }
    }
}

#[tauri::command]
pub async fn character_remove_advantage(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    item_id: String,
) -> Result<(), String> {
    do_remove_advantage(
        &db.0, &bridge.0, target, source, source_id, featuretype, item_id,
    ).await
}

pub(crate) async fn do_remove_advantage(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    item_id: String,
) -> Result<(), String> {
    if item_id.trim().is_empty() {
        return Err("character/remove_advantage: empty item_id".to_string());
    }

    if target != WriteTarget::Saved && source == SourceKind::Roll20 {
        return Err(
            "character/remove_advantage: Roll20 live editing of advantages not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    let do_saved = || async {
        crate::db::saved_character::db_remove_advantage(
            pool, saved_id.unwrap(), featuretype.as_str(), &item_id,
        )
        .await
    };

    let do_live = || async {
        let payload = crate::bridge::foundry::actions::actor::build_delete_item_by_id(
            &source_id, &item_id,
        );
        let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        crate::bridge::commands::send_to_source_inner(bridge_state, source, text).await
    };

    match target {
        WriteTarget::Saved => do_saved().await,
        WriteTarget::Live  => do_live().await,
        WriteTarget::Both  => {
            do_saved().await
                .map_err(|e| format!("character/remove_advantage: saved write failed: {e}"))?;
            do_live().await
                .map_err(|e| format!(
                    "character/remove_advantage: saved updated, live failed: {e}"
                ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::source::{BridgeSource, InboundEvent};
    use crate::bridge::types::CanonicalCharacter;
    use crate::bridge::ConnectionInfo;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    struct StubFoundrySource;

    #[async_trait]
    impl BridgeSource for StubFoundrySource {
        async fn handle_inbound(&self, _msg: Value) -> Result<Vec<InboundEvent>, String> {
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
            roll_history: Mutex::new(std::collections::VecDeque::new()),
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
        async fn handle_inbound(&self, _msg: Value) -> Result<Vec<InboundEvent>, String> {
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
    async fn live_foundry_attribute_strength_routes_outbound() {
        let pool = fresh_pool().await;
        // Use the real FoundrySource so canonical → Foundry path translation
        // (Task B3) actually runs; StubFoundrySource returns an opaque blob.
        let (state, rx) = make_bridge_state_with_source(
            true,
            Arc::new(crate::bridge::foundry::FoundrySource),
        );
        let mut rx = rx.expect("connected state must yield a receiver");

        do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "attribute.strength".to_string(),
            serde_json::json!(4),
        )
        .await
        .expect("happy path");

        let sent = rx.recv().await.expect("router must send outbound message");
        assert!(
            sent.contains("system.attributes.strength.value"),
            "outbound payload should contain the translated path; got: {sent}"
        );
        // Value flows: do_set_field stringifies the JSON value to "4",
        // FoundrySource::build_set_attribute parses it back via parse_value()
        // to the number 4, which serializes unquoted into the wire JSON as
        // `"value":4`.
        assert!(
            sent.contains("\"value\":4"),
            "value should appear as numeric 4; got: {sent}"
        );
    }

    #[tokio::test]
    async fn live_roll20_attribute_strength_fast_fails() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Roll20,
            "abc".to_string(),
            "attribute.strength".to_string(),
            serde_json::json!(3),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("Roll20 live editing of canonical names not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn live_roll20_skill_brawl_fast_fails() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Roll20,
            "abc".to_string(),
            "skill.brawl".to_string(),
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
    async fn unknown_attribute_key_errors_at_router() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "attribute.foo".to_string(),
            serde_json::json!(0),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("unknown field 'attribute.foo'"),
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

    // ── #8 advantage editor tests ────────────────────────────────────────

    async fn seed_saved_row_with_item(pool: &SqlitePool, source_id: &str, item_id: &str, ft: &str) {
        let mut c = sample_canonical();
        c.raw = serde_json::json!({
            "items": [
                { "_id": item_id, "type": "feature", "name": "Pre-existing",
                  "system": { "featuretype": ft, "description": "x", "points": 1 },
                  "effects": [] }
            ]
        });
        let canonical_json = serde_json::to_string(&c).unwrap();
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

    // ─── add_advantage ─────────────────────────────────────────────────

    #[tokio::test]
    async fn add_advantage_empty_name_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "   ".to_string(), "desc".to_string(), 2,
        ).await.unwrap_err();
        assert!(err.contains("empty name"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_points_out_of_range_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 11,
        ).await.unwrap_err();
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_roll20_live_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Roll20,
            "abc".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 1,
        ).await.unwrap_err();
        assert!(
            err.contains("Roll20 live editing of advantages not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn add_advantage_roll20_saved_succeeds() {
        // Saved-side editing works for any source — Roll20 fast-fail only on Live.
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        // Seed a roll20 saved row.
        let mut c = sample_canonical();
        c.source = SourceKind::Roll20;
        let canonical_json = serde_json::to_string(&c).unwrap();
        sqlx::query(
            "INSERT INTO saved_characters
             (source, source_id, foundry_world, name, canonical_json)
             VALUES ('roll20', 'r20-1', NULL, 'R20', ?)",
        )
        .bind(&canonical_json)
        .execute(&pool)
        .await
        .unwrap();

        do_add_advantage(
            &pool, &state, WriteTarget::Saved, SourceKind::Roll20,
            "r20-1".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 1,
        ).await.unwrap();
    }

    #[tokio::test]
    async fn add_advantage_target_saved_writes_db() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row(&pool, "abc").await;

        do_add_advantage(
            &pool, &state, WriteTarget::Saved, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "Iron Will".to_string(), "Strong-minded.".to_string(), 2,
        ).await.unwrap();

        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        let items = c.raw.get("items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("name").and_then(|v| v.as_str()), Some("Iron Will"));
    }

    #[tokio::test]
    async fn add_advantage_target_live_sends_payload() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");

        do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "actor-xyz".to_string(), FeatureType::Flaw,
            "Bad Sight".to_string(), "Squints.".to_string(), 1,
        ).await.unwrap();

        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.create_feature"));
        assert_eq!(payload.get("actor_id").and_then(|v| v.as_str()), Some("actor-xyz"));
        assert_eq!(payload.get("featuretype").and_then(|v| v.as_str()), Some("flaw"));
        assert_eq!(payload.get("name").and_then(|v| v.as_str()), Some("Bad Sight"));
    }

    #[tokio::test]
    async fn add_advantage_target_both_writes_both() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");
        seed_saved_row(&pool, "abc").await;

        do_add_advantage(
            &pool, &state, WriteTarget::Both, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Boon,
            "Owed Favor".to_string(), "From Camarilla.".to_string(), 3,
        ).await.unwrap();

        // Saved write landed.
        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(c.raw.get("items").and_then(|v| v.as_array()).unwrap().len(), 1);

        // Live wire payload sent.
        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.create_feature"));
    }

    #[tokio::test]
    async fn add_advantage_both_partial_success_when_live_fails() {
        // Force tx.send() to fail by dropping the receiver while keeping the
        // sender alive in BridgeState.connections. tokio mpsc::Sender::send
        // returns SendError when the receiver has been dropped, which
        // send_to_source_inner surfaces as Err — triggering the partial-success
        // path (saved-first ordering means the saved write already landed).
        let pool = fresh_pool().await;
        let (state, rx) = make_bridge_state(true);
        drop(rx);
        seed_saved_row(&pool, "abc").await;

        let err = do_add_advantage(
            &pool, &state, WriteTarget::Both, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 1,
        ).await.unwrap_err();

        assert!(
            err.starts_with("character/add_advantage: saved updated, live failed:"),
            "got: {err}"
        );

        // Saved row was still written (saved-first ordering).
        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(c.raw.get("items").and_then(|v| v.as_array()).unwrap().len(), 1);
    }

    // ─── remove_advantage ─────────────────────────────────────────────

    #[tokio::test]
    async fn remove_advantage_empty_item_id_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_remove_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit, "  ".to_string(),
        ).await.unwrap_err();
        assert!(err.contains("empty item_id"), "got: {err}");
    }

    #[tokio::test]
    async fn remove_advantage_roll20_live_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_remove_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Roll20,
            "abc".to_string(), FeatureType::Merit, "item-1".to_string(),
        ).await.unwrap_err();
        assert!(
            err.contains("Roll20 live editing of advantages not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn remove_advantage_target_saved_writes_db() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row_with_item(&pool, "abc", "item-1", "merit").await;

        do_remove_advantage(
            &pool, &state, WriteTarget::Saved, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit, "item-1".to_string(),
        ).await.unwrap();

        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        let items = c.raw.get("items").and_then(|v| v.as_array()).unwrap();
        assert!(items.is_empty(), "items should be empty after remove");
    }

    #[tokio::test]
    async fn remove_advantage_target_live_sends_payload() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");

        do_remove_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "actor-xyz".to_string(), FeatureType::Background, "item-bg".to_string(),
        ).await.unwrap();

        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.delete_item_by_id"));
        assert_eq!(payload.get("actor_id").and_then(|v| v.as_str()), Some("actor-xyz"));
        assert_eq!(payload.get("item_id").and_then(|v| v.as_str()), Some("item-bg"));
    }

    #[tokio::test]
    async fn remove_advantage_target_both_writes_both() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");
        seed_saved_row_with_item(&pool, "abc", "item-1", "merit").await;

        do_remove_advantage(
            &pool, &state, WriteTarget::Both, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit, "item-1".to_string(),
        ).await.unwrap();

        // Saved row updated.
        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert!(c.raw.get("items").and_then(|v| v.as_array()).unwrap().is_empty());

        // Live payload sent.
        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.delete_item_by_id"));
    }
}
