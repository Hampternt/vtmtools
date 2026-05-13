//! GM Screen IPC commands. Plan C: per-card "Push to Foundry" mirrors a
//! card's pool effects to the bound merit's `system.bonuses[]` on the live
//! Foundry actor via the bridge. Opt-in, per-button-press; never automatic.
//!
//! See `docs/superpowers/plans/2026-05-03-gm-screen-plan-c-foundry-pushback.md`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tauri::State;

use crate::bridge::types::SourceKind;
use crate::bridge::BridgeState;
use crate::shared::modifier::{ModifierBinding, ModifierEffect, ModifierKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkippedEffect {
    pub effect_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PushReport {
    pub pushed: usize,
    pub skipped: Vec<SkippedEffect>,
}

/// Build the `source` field that tags one of our pushed bonuses.
/// Format: `"GM Screen #<id>: <name>"`. The `"GM Screen #<id>"` prefix
/// (followed by `:` or end-of-string) is what we filter on for re-push.
fn source_tag(modifier_id: i64, modifier_name: &str) -> String {
    format!("GM Screen #{modifier_id}: {modifier_name}")
}

/// True iff the bonus's `source` was pushed by THIS modifier.
/// Matches `"GM Screen #<id>"` followed by exactly `:` or end-of-string,
/// so id 5 doesn't match id 50.
fn is_ours(bonus: &Value, modifier_id: i64) -> bool {
    let Some(source) = bonus.get("source").and_then(|v| v.as_str()) else {
        return false;
    };
    let prefix = format!("GM Screen #{modifier_id}");
    if !source.starts_with(&prefix) {
        return false;
    }
    let rest = &source[prefix.len()..];
    rest.is_empty() || rest.starts_with(':')
}

/// Translate one ModifierEffect to a Foundry bonus value. Returns None for
/// non-pool kinds (those are reported as skipped in the PushReport).
fn effect_to_bonus(
    effect: &ModifierEffect,
    modifier_id: i64,
    modifier_name: &str,
) -> Option<Value> {
    if effect.kind != ModifierKind::Pool {
        return None;
    }
    let value = effect.delta.unwrap_or(0);
    let paths: Vec<String> = if effect.paths.is_empty() {
        vec!["".to_string()]
    } else {
        effect.paths.clone()
    };
    Some(json!({
        "source": source_tag(modifier_id, modifier_name),
        "value": value,
        "paths": paths,
        "activeWhen": { "check": "always", "path": "", "value": "" },
        "displayWhenInactive": true,
        "unless": "",
    }))
}

/// Filter existing bonuses (drop ones tagged as ours), then append `ours`.
/// Player-added bonuses and bonuses from other modifiers are preserved.
fn merge_bonuses(existing: &[Value], modifier_id: i64, ours: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = existing
        .iter()
        .filter(|b| !is_ours(b, modifier_id))
        .cloned()
        .collect();
    out.extend(ours);
    out
}

/// Inner logic: load modifier, validate binding, read cached actor + item,
/// merge, send actor.update_item_field via the bridge.
pub(crate) async fn do_push_to_foundry(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    modifier_id: i64,
) -> Result<PushReport, String> {
    // Guard: refuse if Foundry isn't actually connected. BridgeState.characters
    // persists across disconnects (see bridge/mod.rs cleanup block), so a stale
    // cached actor would otherwise let send_to_source_inner silently no-op
    // (no outbound_tx) — the UI would show a fake "Pushed N bonuses" toast.
    let connected = bridge_state
        .connections
        .lock()
        .await
        .get(&SourceKind::Foundry)
        .map(|c| c.connected)
        .unwrap_or(false);
    if !connected {
        return Err("gm_screen/push: Foundry is not connected".to_string());
    }

    // 1. Load the modifier (helper added in Step 0).
    let m = crate::db::modifier::get_modifier_by_id(pool, modifier_id)
        .await
        .map_err(|e| format!("gm_screen/push: load modifier {modifier_id}: {e}"))?;

    // 2. Validate source + binding.
    if m.source != SourceKind::Foundry {
        return Err(format!(
            "gm_screen/push: modifier {} is not a Foundry-source modifier (source={:?})",
            modifier_id, m.source
        ));
    }
    let item_id = match &m.binding {
        ModifierBinding::Advantage { item_id } => item_id.clone(),
        ModifierBinding::Free => {
            return Err(format!(
                "gm_screen/push: modifier {modifier_id} has free binding; only advantage-bound modifiers can push"
            ));
        }
    };

    // 3. Build the new bonuses and the skipped report.
    let mut new_bonuses = Vec::new();
    let mut skipped = Vec::new();
    for (i, effect) in m.effects.iter().enumerate() {
        match effect_to_bonus(effect, m.id, &m.name) {
            Some(b) => new_bonuses.push(b),
            None => {
                let reason = match effect.kind {
                    ModifierKind::Difficulty => "difficulty: no Foundry bonus equivalent",
                    ModifierKind::Note => "note: descriptive only",
                    ModifierKind::Stat => "stat: render-time card delta only, no Foundry bonus",
                    ModifierKind::Pool => unreachable!("Pool always translates"),
                };
                skipped.push(SkippedEffect {
                    effect_index: i,
                    reason: reason.to_string(),
                });
            }
        }
    }

    // 4. Read cached actor + locate the item, then read existing bonuses.
    //    We use the BridgeState's character cache (already populated by the
    //    Foundry bridge on Hello/refresh). The cache is a HashMap<String, _>
    //    keyed by `CanonicalCharacter::key()` = `format!("{source}:{source_id}")`.
    //    TOCTOU note: if the player edits bonuses in Foundry between our read
    //    and our write, edits to OUR-tagged bonuses can be lost. Player-added
    //    bonuses are safe (filtered out of `is_ours`). Acceptable for v1.
    let key = format!("{}:{}", SourceKind::Foundry.as_str(), m.source_id);
    let chars = bridge_state.characters.lock().await;
    let actor = chars.get(&key).cloned().ok_or_else(|| {
        format!(
            "gm_screen/push: actor {} not in bridge cache (is Foundry connected?)",
            m.source_id
        )
    })?;
    drop(chars);

    // CanonicalCharacter.raw is a serde_json::Value (NOT Option<Value>).
    let items = actor
        .raw
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            format!(
                "gm_screen/push: actor {} raw has no items[] array",
                m.source_id
            )
        })?;
    let item = items
        .iter()
        .find(|it| it.get("_id").and_then(|v| v.as_str()) == Some(item_id.as_str()))
        .ok_or_else(|| {
            format!(
                "gm_screen/push: item {} not found on actor {} (was the merit deleted?)",
                item_id, m.source_id
            )
        })?;
    let existing: Vec<Value> = item
        .get("system")
        .and_then(|s| s.get("bonuses"))
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    // 5. Merge and send.
    let merged = merge_bonuses(&existing, m.id, new_bonuses.clone());
    let payload = crate::bridge::foundry::actions::actor::build_update_item_field(
        &m.source_id,
        &item_id,
        "system.bonuses",
        Value::Array(merged),
    );
    let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    crate::bridge::commands::send_to_source_inner(bridge_state, SourceKind::Foundry, text)
        .await
        .map_err(|e| format!("gm_screen/push: bridge send failed: {e}"))?;

    Ok(PushReport {
        pushed: new_bonuses.len(),
        skipped,
    })
}

#[tauri::command]
pub async fn gm_screen_push_to_foundry(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    modifier_id: i64,
) -> Result<PushReport, String> {
    do_push_to_foundry(&db.0, &bridge.0, modifier_id).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::source::{BridgeSource, InboundEvent};
    use crate::bridge::types::CanonicalCharacter;
    use crate::bridge::ConnectionInfo;
    use crate::shared::modifier::{ModifierBinding, NewCharacterModifier};
    use async_trait::async_trait;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    fn pool_effect(delta: i32, paths: Vec<&str>) -> ModifierEffect {
        ModifierEffect {
            kind: ModifierKind::Pool,
            scope: None,
            delta: Some(delta),
            note: None,
            paths: paths.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn translation_pool_with_paths_emits_bonus() {
        let e = pool_effect(2, vec!["attributes.strength", "skills.brawl"]);
        let b = effect_to_bonus(&e, 7, "Brawl Buff").expect("pool kind translates");
        assert_eq!(b["value"], 2);
        assert_eq!(b["paths"], json!(["attributes.strength", "skills.brawl"]));
        assert_eq!(b["source"], "GM Screen #7: Brawl Buff");
        assert_eq!(b["activeWhen"]["check"], "always");
        assert_eq!(b["displayWhenInactive"], true);
        assert_eq!(b["unless"], "");
    }

    #[test]
    fn translation_pool_with_no_paths_emits_pathless_bonus() {
        let e = pool_effect(3, vec![]);
        let b = effect_to_bonus(&e, 1, "X").expect("pool kind translates");
        assert_eq!(
            b["paths"],
            json!([""]),
            "empty paths becomes [\"\"] per Foundry sample"
        );
    }

    #[test]
    fn translation_difficulty_returns_none() {
        let e = ModifierEffect {
            kind: ModifierKind::Difficulty,
            scope: None,
            delta: Some(-1),
            note: None,
            paths: vec!["attributes.strength".into()],
        };
        assert!(
            effect_to_bonus(&e, 1, "X").is_none(),
            "difficulty must skip"
        );
    }

    #[test]
    fn translation_note_returns_none() {
        let e = ModifierEffect {
            kind: ModifierKind::Note,
            scope: None,
            delta: None,
            note: Some("careful".into()),
            paths: vec![],
        };
        assert!(effect_to_bonus(&e, 1, "X").is_none(), "note must skip");
    }

    #[test]
    fn merge_filters_only_our_modifier_id() {
        let existing = vec![
            json!({"source": "Player Buff", "value": 1, "paths": ["x"]}),
            json!({"source": "GM Screen #5: A", "value": 2, "paths": ["y"]}), // ours
            json!({"source": "GM Screen #50: B", "value": 3, "paths": ["z"]}), // NOT ours (id 50)
            json!({"source": "GM Screen #6: C", "value": 4, "paths": ["w"]}), // NOT ours (id 6)
            json!({"source": "GM Screen #5", "value": 9, "paths": []}),       // ours (no name suffix)
        ];
        let ours = vec![json!({"source": "GM Screen #5: A", "value": 99, "paths": ["new"]})];
        let merged = merge_bonuses(&existing, 5, ours);
        assert_eq!(merged.len(), 4, "kept 3 non-ours + 1 new");
        assert!(merged.iter().any(|b| b["source"] == "Player Buff"));
        assert!(merged.iter().any(|b| b["source"] == "GM Screen #50: B"));
        assert!(merged.iter().any(|b| b["source"] == "GM Screen #6: C"));
        let ours_new = merged
            .iter()
            .find(|b| b["source"] == "GM Screen #5: A")
            .unwrap();
        assert_eq!(ours_new["value"], 99);
    }

    #[test]
    fn merge_with_no_existing_bonuses_just_appends() {
        let merged = merge_bonuses(&[], 1, vec![json!({"source": "GM Screen #1: X"})]);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn merge_idempotent_under_repeated_push() {
        let initial: Vec<Value> = vec![];
        let ours_v1 = vec![json!({"source": "GM Screen #2: X", "value": 1, "paths": ["a"]})];
        let after_first = merge_bonuses(&initial, 2, ours_v1);
        let ours_v2 = vec![json!({"source": "GM Screen #2: X", "value": 1, "paths": ["a"]})];
        let after_second = merge_bonuses(&after_first, 2, ours_v2);
        assert_eq!(after_first, after_second, "re-push yields the same array");
    }

    // --- Disconnect-guard integration test --------------------------------
    // Verifies the precondition check inside do_push_to_foundry: if the Foundry
    // connection is marked disconnected (even when the bridge's character cache
    // still holds a valid actor from a prior session), the push must error out
    // BEFORE any work runs — otherwise send_to_source_inner would silently
    // no-op via a missing outbound_tx and the UI would show a fake success.

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
            Ok(json!({"type": "stub"}))
        }
        fn build_refresh(&self) -> Value {
            json!({"type": "refresh"})
        }
    }

    async fn fresh_pool() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    /// Build a BridgeState for the disconnect path: Foundry connection present
    /// but `connected=false` (mirroring the bridge/mod.rs disconnect cleanup).
    fn make_disconnected_bridge_state() -> Arc<BridgeState> {
        let mut sources: HashMap<SourceKind, Arc<dyn BridgeSource>> = HashMap::new();
        sources.insert(SourceKind::Foundry, Arc::new(StubFoundrySource));

        let mut connections = HashMap::new();
        connections.insert(
            SourceKind::Foundry,
            ConnectionInfo {
                connected: false,
                outbound_tx: None,
            },
        );

        Arc::new(BridgeState {
            characters: Mutex::new(HashMap::new()),
            connections: Mutex::new(connections),
            source_info: Mutex::new(HashMap::new()),
            sources,
            roll_history: Mutex::new(std::collections::VecDeque::new()),
        })
    }

    fn cached_actor_with_item(item_id: &str) -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "actor-x".to_string(),
            name: "Stale Cached Actor".to_string(),
            controlled_by: None,
            hunger: None,
            health: None,
            willpower: None,
            humanity: None,
            humanity_stains: None,
            blood_potency: None,
            raw: json!({
                "items": [
                    { "_id": item_id, "system": { "bonuses": [] } }
                ]
            }),
        }
    }

    #[tokio::test]
    async fn push_errors_when_foundry_disconnected_even_with_stale_cache() {
        let pool = fresh_pool().await;

        // Insert a Foundry+advantage modifier. The connection check fires
        // BEFORE the modifier load, but seeding the row makes the test more
        // representative of the real call shape and guards against future
        // regressions that reorder the checks.
        let new = NewCharacterModifier {
            source: SourceKind::Foundry,
            source_id: "actor-x".to_string(),
            name: "Stale Buff".to_string(),
            description: String::new(),
            effects: vec![pool_effect(2, vec!["attributes.strength"])],
            binding: ModifierBinding::Advantage {
                item_id: "merit-1".to_string(),
            },
            tags: vec![],
            origin_template_id: None,
            foundry_captured_labels: vec![],
        };
        let added = crate::db::modifier::db_add(&pool, new).await.unwrap();

        let bridge_state = make_disconnected_bridge_state();

        // Populate the character cache to simulate the post-disconnect-but-
        // cache-still-warm scenario the guard exists to defend against.
        bridge_state.characters.lock().await.insert(
            format!("{}:{}", SourceKind::Foundry.as_str(), "actor-x"),
            cached_actor_with_item("merit-1"),
        );

        let err = do_push_to_foundry(&pool, &bridge_state, added.id)
            .await
            .expect_err("disconnected Foundry must error");
        assert!(
            err.to_lowercase().contains("not connected"),
            "error must mention disconnect, got: {err}"
        );
    }
}
