use rand::seq::SliceRandom;
use sqlx::{Row, SqlitePool};
use crate::bridge::{BridgeConn, BridgeState, types::SourceKind};
use crate::shared::types::{
    Advantage, AdvantageKind, Field, FieldValue, ImportOutcome, NumberFieldValue,
};

// --------------------------------------------------------------------------
// JSON serde helpers for tags_json and properties_json columns
// --------------------------------------------------------------------------

fn serialize_tags(tags: &[String]) -> Result<String, String> {
    serde_json::to_string(tags).map_err(|e| format!("db/advantage.serialize_tags: {}", e))
}

fn deserialize_tags(s: &str) -> Result<Vec<String>, String> {
    serde_json::from_str(s).map_err(|e| format!("db/advantage.deserialize_tags: {}", e))
}

fn serialize_properties(props: &[Field]) -> Result<String, String> {
    serde_json::to_string(props).map_err(|e| format!("db/advantage.serialize_properties: {}", e))
}

fn deserialize_properties(s: &str) -> Result<Vec<Field>, String> {
    serde_json::from_str(s).map_err(|e| format!("db/advantage.deserialize_properties: {}", e))
}

// --------------------------------------------------------------------------
// Kind <-> SQL string helpers (kept inline; YAGNI — don't extract).
// Mirrors the CHECK constraint in 0009_advantages_kind_and_source.sql.
// --------------------------------------------------------------------------

fn kind_to_str(k: AdvantageKind) -> &'static str {
    match k {
        AdvantageKind::Merit      => "merit",
        AdvantageKind::Flaw       => "flaw",
        AdvantageKind::Background => "background",
        AdvantageKind::Boon       => "boon",
    }
}

fn str_to_kind(s: &str) -> Result<AdvantageKind, String> {
    match s {
        "merit"      => Ok(AdvantageKind::Merit),
        "flaw"       => Ok(AdvantageKind::Flaw),
        "background" => Ok(AdvantageKind::Background),
        "boon"       => Ok(AdvantageKind::Boon),
        other        => Err(format!("db/advantage: unknown kind: {other}")),
    }
}

// --------------------------------------------------------------------------
// Internal helpers (testable — take &SqlitePool directly)
// --------------------------------------------------------------------------

fn row_to_advantage(r: &sqlx::sqlite::SqliteRow) -> Result<Advantage, String> {
    let tags_json: String = r.get("tags_json");
    let properties_json: String = r.get("properties_json");
    let kind_str: String = r.get("kind");
    let source_attribution_str: Option<String> = r.get("source_attribution");

    let source_attribution = match source_attribution_str {
        Some(s) => Some(
            serde_json::from_str::<serde_json::Value>(&s)
                .map_err(|e| format!("db/advantage.deserialize_source_attribution: {}", e))?,
        ),
        None => None,
    };

    Ok(Advantage {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        kind: str_to_kind(&kind_str)?,
        tags: deserialize_tags(&tags_json)?,
        properties: deserialize_properties(&properties_json)?,
        is_custom: r.get::<bool, _>("is_custom"),
        source_attribution,
    })
}

pub(crate) async fn db_list(pool: &SqlitePool) -> Result<Vec<Advantage>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, tags_json, properties_json, is_custom, kind, source_attribution
         FROM advantages ORDER BY is_custom ASC, id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/advantage.list: {}", e))?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        out.push(row_to_advantage(r)?);
    }
    Ok(out)
}

async fn db_list_by_kind(
    pool: &SqlitePool,
    kind: AdvantageKind,
) -> Result<Vec<Advantage>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, tags_json, properties_json, is_custom, kind, source_attribution
         FROM advantages WHERE kind = ? ORDER BY is_custom ASC, id ASC"
    )
    .bind(kind_to_str(kind))
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/advantage.list_by_kind: {}", e))?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        out.push(row_to_advantage(r)?);
    }
    Ok(out)
}

async fn db_insert(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    kind: AdvantageKind,
    source_attribution: Option<&serde_json::Value>,
    tags: &[String],
    properties: &[Field],
) -> Result<Advantage, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;
    let source_attribution_str: Option<String> = match source_attribution {
        Some(v) => Some(
            serde_json::to_string(v)
                .map_err(|e| format!("db/advantage.serialize_source_attribution: {}", e))?,
        ),
        None => None,
    };

    let result = sqlx::query(
        "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom, kind, source_attribution)
         VALUES (?, ?, ?, ?, 1, ?, ?)"
    )
    .bind(name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .bind(kind_to_str(kind))
    .bind(&source_attribution_str)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.insert: {}", e))?;

    Ok(Advantage {
        id: result.last_insert_rowid(),
        name: name.to_string(),
        description: description.to_string(),
        kind,
        tags: tags.to_vec(),
        properties: properties.to_vec(),
        is_custom: true,
        source_attribution: source_attribution.cloned(),
    })
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    description: &str,
    kind: AdvantageKind,
    source_attribution: Option<&serde_json::Value>,
    tags: &[String],
    properties: &[Field],
) -> Result<(), String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;
    let source_attribution_str: Option<String> = match source_attribution {
        Some(v) => Some(
            serde_json::to_string(v)
                .map_err(|e| format!("db/advantage.serialize_source_attribution: {}", e))?,
        ),
        None => None,
    };

    let result = sqlx::query(
        "UPDATE advantages
         SET name = ?, description = ?, tags_json = ?, properties_json = ?, kind = ?, source_attribution = ?
         WHERE id = ? AND is_custom = 1"
    )
    .bind(name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .bind(kind_to_str(kind))
    .bind(&source_attribution_str)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.update: {}", e))?;

    if result.rows_affected() == 0 {
        return Err("db/advantage.update: row not found or not editable".to_string());
    }
    Ok(())
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM advantages WHERE id = ? AND is_custom = 1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/advantage.delete: {}", e))?;

    if result.rows_affected() == 0 {
        return Err("db/advantage.delete: row not found or not deletable".to_string());
    }
    Ok(())
}

async fn db_roll_random(
    pool: &SqlitePool,
    tags: &[String],
) -> Result<Option<Advantage>, String> {
    let all = db_list(pool).await?;

    let pool_of_matches: Vec<Advantage> = if tags.is_empty() {
        all
    } else {
        all.into_iter()
            .filter(|row| row.tags.iter().any(|t| tags.contains(t)))
            .collect()
    };

    if pool_of_matches.is_empty() {
        return Ok(None);
    }
    Ok(pool_of_matches.choose(&mut rand::thread_rng()).cloned())
}

// --------------------------------------------------------------------------
// FVTT import path (parallel to db_insert / db_update — preserves dedup
// identity on the immutable Foundry document _id carried via
// source_attribution.foundryId, NOT on the mutable display name).
// --------------------------------------------------------------------------

/// Identity lookup keyed on the immutable Foundry document id (carried via
/// `source_attribution.foundryId`) scoped by `worldTitle`. Returns None for
/// any row whose `source_attribution` is NULL (i.e. local rows never match).
pub(crate) async fn db_find_by_foundry_id(
    pool: &SqlitePool,
    foundry_id: &str,
    world_title: &str,
) -> Result<Option<Advantage>, String> {
    let row = sqlx::query(
        "SELECT id, name, description, tags_json, properties_json, is_custom,
                kind, source_attribution
         FROM advantages
         WHERE json_extract(source_attribution, '$.foundryId')  = ?
           AND json_extract(source_attribution, '$.worldTitle') = ?
         LIMIT 1"
    )
    .bind(foundry_id)
    .bind(world_title)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/advantage.find_by_foundry_id: {}", e))?;

    match row {
        Some(r) => Ok(Some(row_to_advantage(&r)?)),
        None => Ok(None),
    }
}

/// True iff any row matches (name, kind) regardless of attribution. Used by
/// the import path to decide when to auto-suffix imported names so a
/// hand-authored "Iron Gullet" can coexist with the FVTT-imported one.
pub(crate) async fn db_collides_locally(
    pool: &SqlitePool,
    name: &str,
    kind: AdvantageKind,
) -> Result<bool, String> {
    let row = sqlx::query("SELECT COUNT(*) AS c FROM advantages WHERE name = ? AND kind = ?")
        .bind(name)
        .bind(kind_to_str(kind))
        .fetch_one(pool)
        .await
        .map_err(|e| format!("db/advantage.collides_locally: {}", e))?;
    let c: i64 = row.get("c");
    Ok(c > 0)
}

/// Import-flow workhorse. Resolves the dedup decision tree:
///   - Same (foundryId, worldTitle) already imported → UPDATE in place
///     (description, properties, attribution), preserving the row id and
///     the stored name (which may already carry a suffix).
///   - Different attribution but (name, kind) collides with any local row
///     → auto-suffix "(FVTT — <worldTitle>)" and INSERT.
///   - No collision → straight INSERT.
///
/// Imported rows are always `is_custom = 1` (survives destructive reseed,
/// per ARCHITECTURE.md §6 tri-state) and start with empty tags.
pub(crate) async fn db_upsert_imported(
    pool: &SqlitePool,
    name: &str,
    kind: AdvantageKind,
    description: &str,
    properties: &[Field],
    source_attribution: &serde_json::Value,
) -> Result<ImportOutcome, String> {
    let world_title = source_attribution
        .get("worldTitle")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "db/advantage.upsert_imported: source_attribution missing worldTitle".to_string()
        })?;
    let foundry_id = source_attribution
        .get("foundryId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            "db/advantage.upsert_imported: source_attribution missing foundryId".to_string()
        })?;

    // Case 1: same (foundryId, worldTitle) → UPDATE in place.
    if let Some(existing) = db_find_by_foundry_id(pool, foundry_id, world_title).await? {
        let properties_json = serialize_properties(properties)?;
        let attribution_str = serde_json::to_string(source_attribution)
            .map_err(|e| format!("db/advantage.upsert_imported: serialize attribution: {}", e))?;

        sqlx::query(
            "UPDATE advantages
             SET description = ?, properties_json = ?, source_attribution = ?
             WHERE id = ?"
        )
        .bind(description)
        .bind(&properties_json)
        .bind(&attribution_str)
        .bind(existing.id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/advantage.upsert_imported: update: {}", e))?;

        return Ok(ImportOutcome::Updated {
            id: existing.id,
            name: existing.name,
            kind: existing.kind,
        });
    }

    // Case 2 / 3: INSERT. Suffix name iff (name, kind) collides with any
    // existing row (local hand-authored OR a different-world import).
    let final_name = if db_collides_locally(pool, name, kind).await? {
        format!("{} (FVTT — {})", name, world_title)
    } else {
        name.to_string()
    };

    let tags_json = serialize_tags(&[])?;
    let properties_json = serialize_properties(properties)?;
    let attribution_str = serde_json::to_string(source_attribution)
        .map_err(|e| format!("db/advantage.upsert_imported: serialize attribution: {}", e))?;

    let result = sqlx::query(
        "INSERT INTO advantages
            (name, description, tags_json, properties_json, is_custom, kind, source_attribution)
         VALUES (?, ?, ?, ?, 1, ?, ?)"
    )
    .bind(&final_name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .bind(kind_to_str(kind))
    .bind(&attribution_str)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.upsert_imported: insert: {}", e))?;

    Ok(ImportOutcome::Inserted {
        id: result.last_insert_rowid(),
        name: final_name,
        kind,
    })
}

// --------------------------------------------------------------------------
// FVTT import orchestration (inner helper + Tauri wrapper)
//
// Split into an inner helper that takes `&SqlitePool` + `&BridgeState`
// (unit-testable without a Tauri runtime) and a thin `#[tauri::command]`
// wrapper that extracts the State references and delegates. Mirrors the
// `db_list` / `list_advantages` pattern used throughout this file.
// --------------------------------------------------------------------------

/// Snapshot the active Foundry world info + cached world items, filter to
/// feature-type rows with a known featuretype, and per-row delegate to
/// `db_upsert_imported`. Non-feature items and unknown featuretypes are
/// returned as `ImportOutcome::Skipped` for the frontend's summary toast.
///
/// Error prefix `db/advantage.import_from_world:` per ARCH §7.
pub(crate) async fn db_import_from_world(
    pool: &SqlitePool,
    bridge: &BridgeState,
) -> Result<Vec<ImportOutcome>, String> {
    // Snapshot Foundry source info under one lock acquisition. world_title
    // is required (it's the dedup-scope key); world_id and system_version
    // are optional metadata stamped into source_attribution.
    let (world_title, world_id, system_version) = {
        let info = bridge.source_info.lock().await;
        let i = info.get(&SourceKind::Foundry).ok_or_else(|| {
            "db/advantage.import_from_world: no active Foundry connection".to_string()
        })?;
        let world_title = i.world_title.clone().ok_or_else(|| {
            "db/advantage.import_from_world: no active Foundry world (connect first?)"
                .to_string()
        })?;
        (world_title, i.world_id.clone(), i.system_version.clone())
    };

    let items: Vec<crate::bridge::types::CanonicalWorldItem> = {
        let store = bridge.world_items.lock().await;
        store
            .get(&SourceKind::Foundry)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    };

    let now = chrono::Utc::now().to_rfc3339();

    let mut outcomes = Vec::with_capacity(items.len());
    for item in items {
        if item.kind != "feature" {
            outcomes.push(ImportOutcome::Skipped {
                reason: format!("non-feature item kind: {}", item.kind),
                name: item.name,
            });
            continue;
        }
        let ft = match item.featuretype.as_deref() {
            Some("merit")      => AdvantageKind::Merit,
            Some("flaw")       => AdvantageKind::Flaw,
            Some("background") => AdvantageKind::Background,
            Some("boon")       => AdvantageKind::Boon,
            other => {
                outcomes.push(ImportOutcome::Skipped {
                    reason: format!("unknown featuretype: {:?}", other),
                    name: item.name,
                });
                continue;
            }
        };

        let description = item
            .system
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let points = item
            .system
            .get("points")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        let properties: Vec<Field> = if points > 0 {
            vec![Field {
                name: "level".into(),
                value: FieldValue::Number {
                    value: NumberFieldValue::Single(points as f64),
                },
            }]
        } else {
            vec![]
        };

        let attribution = serde_json::json!({
            "source":        "foundry",
            "worldTitle":    world_title,
            "worldId":       world_id,
            "systemVersion": system_version,
            "foundryId":     item.id,
            "importedAt":    now,
        });

        let outcome = db_upsert_imported(
            pool,
            &item.name,
            ft,
            &description,
            &properties,
            &attribution,
        )
        .await?;
        outcomes.push(outcome);
    }

    Ok(outcomes)
}

// --------------------------------------------------------------------------
// Tauri command handlers (thin wrappers around the helpers above)
// --------------------------------------------------------------------------

#[tauri::command]
pub async fn list_advantages(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<Advantage>, String> {
    db_list(&pool.0).await
}

#[tauri::command]
pub async fn add_advantage(
    pool: tauri::State<'_, crate::DbState>,
    name: String,
    description: String,
    kind: AdvantageKind,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Advantage, String> {
    db_insert(&pool.0, &name, &description, kind, None, &tags, &properties).await
}

#[tauri::command]
pub async fn update_advantage(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
    kind: AdvantageKind,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<(), String> {
    db_update(&pool.0, id, &name, &description, kind, None, &tags, &properties).await
}

#[tauri::command]
pub async fn delete_advantage(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

#[tauri::command]
pub async fn roll_random_advantage(
    pool: tauri::State<'_, crate::DbState>,
    tags: Vec<String>,
) -> Result<Option<Advantage>, String> {
    db_roll_random(&pool.0, &tags).await
}

/// Import feature-type world items from the active Foundry world into the
/// local advantages library. Pulls from `BridgeState.world_items` — the
/// frontend must have subscribed to the `item` collection beforehand
/// (Task 6 wires the subscription).
///
/// Filter: only `kind == "feature"` items with featuretype ∈ {merit,
/// flaw, background, boon}. Other item kinds (speciality, power, etc.)
/// and unknown featuretypes are returned as `ImportOutcome::Skipped` for
/// the frontend's summary toast.
#[tauri::command]
pub async fn import_advantages_from_world(
    db: tauri::State<'_, crate::DbState>,
    bridge: tauri::State<'_, BridgeConn>,
) -> Result<Vec<ImportOutcome>, String> {
    db_import_from_world(&db.0, &bridge.0).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{Field, FieldValue, NumberFieldValue};
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE advantages (
                id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                name               TEXT NOT NULL,
                description        TEXT NOT NULL DEFAULT '',
                tags_json          TEXT NOT NULL DEFAULT '[]',
                properties_json    TEXT NOT NULL DEFAULT '[]',
                is_custom          INTEGER NOT NULL DEFAULT 0,
                kind               TEXT NOT NULL DEFAULT 'merit'
                    CHECK(kind IN ('merit','flaw','background','boon')),
                source_attribution TEXT
            )"
        ).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = test_pool().await;
        let result = db_list(&pool).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn insert_and_list_round_trips_tags_and_properties() {
        let pool = test_pool().await;
        let tags = vec!["VTM 5e".to_string(), "Merit".to_string()];
        let props = vec![Field {
            name: "level".to_string(),
            value: FieldValue::Number { value: NumberFieldValue::Single(3.0) },
        }];

        let inserted = db_insert(
            &pool,
            "Iron Gullet",
            "Can drink rancid blood",
            AdvantageKind::Merit,
            None,
            &tags,
            &props,
        )
        .await
        .unwrap();
        assert_eq!(inserted.name, "Iron Gullet");
        assert!(inserted.is_custom);

        let entries = db_list(&pool).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tags, tags);
        assert_eq!(entries[0].properties, props);
    }

    #[tokio::test]
    async fn update_rejects_builtin_row() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom, kind)
             VALUES ('Allies', '', '[]', '[]', 0, 'background')"
        ).execute(&pool).await.unwrap();

        let err = db_update(&pool, 1, "X", "", AdvantageKind::Background, None, &[], &[])
            .await
            .unwrap_err();
        assert!(err.contains("not editable"));
    }

    #[tokio::test]
    async fn delete_rejects_builtin_row() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom, kind)
             VALUES ('Allies', '', '[]', '[]', 0, 'background')"
        ).execute(&pool).await.unwrap();

        let err = db_delete(&pool, 1).await.unwrap_err();
        assert!(err.contains("not deletable"));
    }

    #[tokio::test]
    async fn update_succeeds_on_custom_row() {
        let pool = test_pool().await;
        let inserted = db_insert(&pool, "Old Name", "", AdvantageKind::Merit, None, &[], &[])
            .await
            .unwrap();
        db_update(&pool, inserted.id, "New Name", "desc", AdvantageKind::Merit, None, &[], &[])
            .await
            .unwrap();
        let rows = db_list(&pool).await.unwrap();
        assert_eq!(rows[0].name, "New Name");
    }

    #[tokio::test]
    async fn delete_succeeds_on_custom_row() {
        let pool = test_pool().await;
        let inserted = db_insert(&pool, "To Delete", "", AdvantageKind::Merit, None, &[], &[])
            .await
            .unwrap();
        db_delete(&pool, inserted.id).await.unwrap();
        assert!(db_list(&pool).await.unwrap().is_empty());
    }

    async fn seed_three(pool: &SqlitePool) {
        db_insert(pool, "M1", "", AdvantageKind::Merit,      None, &vec!["Merit".to_string()],      &[]).await.unwrap();
        db_insert(pool, "B1", "", AdvantageKind::Background, None, &vec!["Background".to_string()], &[]).await.unwrap();
        db_insert(pool, "F1", "", AdvantageKind::Flaw,       None, &vec!["Flaw".to_string()],       &[]).await.unwrap();
    }

    #[tokio::test]
    async fn roll_random_empty_tags_returns_any_row() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        let picked = db_roll_random(&pool, &[]).await.unwrap();
        assert!(picked.is_some(), "expected Some(row), got None");
    }

    #[tokio::test]
    async fn roll_random_single_tag_returns_only_matching() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        for _ in 0..20 {
            let picked = db_roll_random(&pool, &["Merit".to_string()]).await.unwrap().unwrap();
            assert!(picked.tags.contains(&"Merit".to_string()));
        }
    }

    #[tokio::test]
    async fn roll_random_multi_tag_is_or_match() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        let filter = vec!["Merit".to_string(), "Background".to_string()];
        for _ in 0..20 {
            let picked = db_roll_random(&pool, &filter).await.unwrap().unwrap();
            assert!(picked.tags.iter().any(|t| filter.contains(t)));
            assert!(!picked.tags.contains(&"Flaw".to_string()));
        }
    }

    #[tokio::test]
    async fn roll_random_no_match_returns_none() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        let picked = db_roll_random(&pool, &["NonexistentTag".to_string()]).await.unwrap();
        assert!(picked.is_none());
    }

    #[tokio::test]
    async fn roll_random_empty_table_returns_none() {
        let pool = test_pool().await;
        let picked = db_roll_random(&pool, &[]).await.unwrap();
        assert!(picked.is_none());
    }

    // ─── New tests: kind / source_attribution / list_by_kind ────────────

    #[tokio::test]
    async fn kind_round_trips_through_insert_and_list() {
        let pool = test_pool().await;
        db_insert(&pool, "F1", "", AdvantageKind::Flaw,       None, &[], &[]).await.unwrap();
        db_insert(&pool, "B1", "", AdvantageKind::Background, None, &[], &[]).await.unwrap();
        db_insert(&pool, "M1", "", AdvantageKind::Merit,      None, &[], &[]).await.unwrap();
        let rows = db_list(&pool).await.unwrap();
        let by_name: std::collections::HashMap<_, _> =
            rows.iter().map(|r| (r.name.clone(), r.kind)).collect();
        assert_eq!(by_name["F1"], AdvantageKind::Flaw);
        assert_eq!(by_name["B1"], AdvantageKind::Background);
        assert_eq!(by_name["M1"], AdvantageKind::Merit);
    }

    #[tokio::test]
    async fn source_attribution_round_trips_through_insert_and_list() {
        let pool = test_pool().await;
        let attribution = serde_json::json!({
            "source": "foundry",
            "world_title": "Chronicles of Chicago",
            "imported_at": "2026-05-14T12:00:00Z",
        });
        db_insert(
            &pool,
            "Imported Merit",
            "",
            AdvantageKind::Merit,
            Some(&attribution),
            &[],
            &[],
        )
        .await
        .unwrap();
        db_insert(&pool, "Local Merit", "", AdvantageKind::Merit, None, &[], &[])
            .await
            .unwrap();
        let rows = db_list(&pool).await.unwrap();
        let by_name: std::collections::HashMap<_, _> = rows
            .iter()
            .map(|r| (r.name.clone(), r.source_attribution.clone()))
            .collect();
        assert_eq!(
            by_name["Imported Merit"].as_ref().unwrap()["world_title"],
            "Chronicles of Chicago"
        );
        assert!(by_name["Local Merit"].is_none());
    }

    #[tokio::test]
    async fn list_by_kind_filters_correctly() {
        let pool = test_pool().await;
        db_insert(&pool, "F1", "", AdvantageKind::Flaw,  None, &[], &[]).await.unwrap();
        db_insert(&pool, "F2", "", AdvantageKind::Flaw,  None, &[], &[]).await.unwrap();
        db_insert(&pool, "M1", "", AdvantageKind::Merit, None, &[], &[]).await.unwrap();
        let flaws = db_list_by_kind(&pool, AdvantageKind::Flaw).await.unwrap();
        assert_eq!(flaws.len(), 2);
        assert!(flaws.iter().all(|r| r.kind == AdvantageKind::Flaw));
    }

    // ─── New tests: FVTT import dedup helpers ───────────────────────────

    fn world_attribution(world: &str, foundry_id: &str) -> serde_json::Value {
        serde_json::json!({
            "source": "foundry",
            "worldTitle": world,
            "foundryId": foundry_id,
            "importedAt": "2026-05-14T12:00:00Z",
        })
    }

    #[tokio::test]
    async fn upsert_imported_new_row_inserts() {
        let pool = test_pool().await;
        let out = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "desc",
            &[],
            &world_attribution("Chicago", "chi_iron"),
        )
        .await
        .unwrap();
        assert!(matches!(out, ImportOutcome::Inserted { ref name, .. } if name == "Iron Gullet"));
    }

    #[tokio::test]
    async fn upsert_imported_same_world_updates_in_place() {
        let pool = test_pool().await;
        let first = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "desc1",
            &[],
            &world_attribution("Chicago", "chi_iron"),
        )
        .await
        .unwrap();
        let first_id = match first {
            ImportOutcome::Inserted { id, .. } => id,
            other => panic!("expected Inserted on first import, got {other:?}"),
        };

        let second = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "desc2 (revised)",
            &[],
            &world_attribution("Chicago", "chi_iron"),
        )
        .await
        .unwrap();

        match second {
            ImportOutcome::Updated { id, .. } => assert_eq!(id, first_id),
            other => panic!("expected Updated, got {other:?}"),
        }
        let rows = db_list(&pool).await.unwrap();
        assert_eq!(rows.iter().filter(|r| r.name == "Iron Gullet").count(), 1);
        assert_eq!(
            rows.iter().find(|r| r.id == first_id).unwrap().description,
            "desc2 (revised)"
        );
    }

    #[tokio::test]
    async fn upsert_imported_different_world_suffixes_name() {
        let pool = test_pool().await;
        db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "d",
            &[],
            &world_attribution("Chicago", "chi_iron"),
        )
        .await
        .unwrap();

        let second = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "d",
            &[],
            &world_attribution("Berlin", "ber_iron"),
        )
        .await
        .unwrap();

        match second {
            ImportOutcome::Inserted { name, .. } => {
                assert_eq!(name, "Iron Gullet (FVTT — Berlin)");
            }
            other => panic!("expected Inserted with suffix, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn upsert_imported_suffixes_against_local_row() {
        let pool = test_pool().await;
        db_insert(&pool, "Iron Gullet", "local desc", AdvantageKind::Merit, None, &[], &[])
            .await
            .unwrap();

        let imported = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "fvtt desc",
            &[],
            &world_attribution("Chicago", "chi_iron"),
        )
        .await
        .unwrap();

        match imported {
            ImportOutcome::Inserted { name, .. } => {
                assert_eq!(name, "Iron Gullet (FVTT — Chicago)");
            }
            other => panic!("expected Inserted with suffix, got {other:?}"),
        }
        let rows = db_list(&pool).await.unwrap();
        let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"Iron Gullet"));
        assert!(names.contains(&"Iron Gullet (FVTT — Chicago)"));
    }

    #[tokio::test]
    async fn upsert_imported_repull_of_secondary_world_updates_in_place() {
        let pool = test_pool().await;

        let chi = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "v1",
            &[],
            &world_attribution("Chicago", "chi_iron"),
        )
        .await
        .unwrap();
        assert!(matches!(chi, ImportOutcome::Inserted { ref name, .. } if name == "Iron Gullet"));

        let ber1 = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "v1",
            &[],
            &world_attribution("Berlin", "ber_iron"),
        )
        .await
        .unwrap();
        let ber_id = match ber1 {
            ImportOutcome::Inserted { id, name, .. } => {
                assert_eq!(name, "Iron Gullet (FVTT — Berlin)");
                id
            }
            other => panic!("expected suffixed Inserted, got {other:?}"),
        };

        let ber2 = db_upsert_imported(
            &pool,
            "Iron Gullet",
            AdvantageKind::Merit,
            "v2",
            &[],
            &world_attribution("Berlin", "ber_iron"),
        )
        .await
        .unwrap();
        match ber2 {
            ImportOutcome::Updated { id, .. } => assert_eq!(id, ber_id),
            other => panic!("expected Updated on re-pull of secondary world, got {other:?}"),
        }
        let rows = db_list(&pool).await.unwrap();
        let berlin_rows = rows
            .iter()
            .filter(|r| r.name == "Iron Gullet (FVTT — Berlin)")
            .count();
        assert_eq!(
            berlin_rows, 1,
            "re-pull of secondary-world item must update in place, not duplicate"
        );
        let chicago_rows = rows.iter().filter(|r| r.name == "Iron Gullet").count();
        assert_eq!(
            chicago_rows, 1,
            "Chicago row must be untouched by Berlin re-pull"
        );
    }

    #[tokio::test]
    async fn find_by_foundry_id_returns_none_for_local_row() {
        let pool = test_pool().await;
        db_insert(&pool, "Allies", "local", AdvantageKind::Background, None, &[], &[])
            .await
            .unwrap();
        let found = db_find_by_foundry_id(&pool, "any_id", "Chicago").await.unwrap();
        assert!(
            found.is_none(),
            "local row (NULL attribution) must never match a foundryId lookup"
        );
    }

    // ─── Integration test: db_import_from_world filter + orchestration ──

    #[tokio::test]
    async fn import_from_world_filters_non_feature_and_unknown_featuretype() {
        use crate::bridge::types::{CanonicalWorldItem, SourceInfo};
        use std::collections::HashMap;

        let pool = test_pool().await;

        // Construct a real BridgeState with empty sources map (the import
        // path only reads source_info + world_items — never invokes the
        // BridgeSource trait — so an empty sources HashMap is fine).
        let bridge = BridgeState::new(HashMap::new());
        {
            let mut info = bridge.source_info.lock().await;
            info.insert(
                SourceKind::Foundry,
                SourceInfo {
                    world_id:         Some("world_chi".into()),
                    world_title:      Some("Chicago".into()),
                    system_id:        Some("vtm5e".into()),
                    system_version:   Some("0.9.0".into()),
                    protocol_version: 1,
                    capabilities:     vec!["actors".into(), "items".into()],
                },
            );
        }
        {
            let mut store = bridge.world_items.lock().await;
            let slot = store.entry(SourceKind::Foundry).or_default();
            slot.insert(
                "merit_id".into(),
                CanonicalWorldItem {
                    source: SourceKind::Foundry,
                    id:     "merit_id".into(),
                    name:   "Iron Gullet".into(),
                    kind:   "feature".into(),
                    featuretype: Some("merit".into()),
                    system: serde_json::json!({
                        "description": "Can drink rancid blood",
                        "points":      1,
                    }),
                },
            );
            slot.insert(
                "spec_id".into(),
                CanonicalWorldItem {
                    source: SourceKind::Foundry,
                    id:     "spec_id".into(),
                    name:   "Survival: Urban".into(),
                    kind:   "speciality".into(),
                    featuretype: None,
                    system: serde_json::json!({}),
                },
            );
            slot.insert(
                "unknown_ft_id".into(),
                CanonicalWorldItem {
                    source: SourceKind::Foundry,
                    id:     "unknown_ft_id".into(),
                    name:   "Mystery Feature".into(),
                    kind:   "feature".into(),
                    featuretype: Some("discipline".into()),
                    system: serde_json::json!({}),
                },
            );
        }

        let outcomes = db_import_from_world(&pool, &bridge).await.unwrap();
        assert_eq!(outcomes.len(), 3, "one outcome per snapshot item");

        // HashMap iteration order is unspecified — partition by variant.
        let mut inserted = 0;
        let mut skipped_non_feature = 0;
        let mut skipped_unknown_ft = 0;
        for o in &outcomes {
            match o {
                ImportOutcome::Inserted { name, kind, .. } => {
                    inserted += 1;
                    assert_eq!(name, "Iron Gullet");
                    assert_eq!(*kind, AdvantageKind::Merit);
                }
                ImportOutcome::Skipped { reason, .. } => {
                    if reason.contains("non-feature") {
                        skipped_non_feature += 1;
                    } else if reason.contains("unknown featuretype") {
                        skipped_unknown_ft += 1;
                    } else {
                        panic!("unexpected Skipped reason: {reason}");
                    }
                }
                ImportOutcome::Updated { .. } => {
                    panic!("expected Inserted, got Updated on a fresh import")
                }
            }
        }
        assert_eq!(inserted, 1, "merit feature should Insert");
        assert_eq!(skipped_non_feature, 1, "speciality should Skip (non-feature)");
        assert_eq!(skipped_unknown_ft, 1, "unknown featuretype should Skip");

        // Verify the inserted row landed in the DB with correct attribution.
        let rows = db_list(&pool).await.unwrap();
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.name, "Iron Gullet");
        assert_eq!(r.kind, AdvantageKind::Merit);
        let attrib = r.source_attribution.as_ref().unwrap();
        assert_eq!(attrib["worldTitle"], "Chicago");
        assert_eq!(attrib["worldId"], "world_chi");
        assert_eq!(attrib["systemVersion"], "0.9.0");
        assert_eq!(attrib["foundryId"], "merit_id");
        assert_eq!(attrib["source"], "foundry");
        // Properties should carry a level=1 field since points > 0.
        assert_eq!(r.properties.len(), 1);
        assert_eq!(r.properties[0].name, "level");
    }

    #[tokio::test]
    async fn import_from_world_errors_when_no_foundry_connected() {
        use std::collections::HashMap;
        let pool = test_pool().await;
        let bridge = BridgeState::new(HashMap::new());
        let err = db_import_from_world(&pool, &bridge).await.unwrap_err();
        assert!(
            err.contains("db/advantage.import_from_world"),
            "error message missing prefix: {err}"
        );
        assert!(err.contains("no active Foundry"), "unexpected error: {err}");
    }
}
