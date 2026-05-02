use sqlx::{Row, SqlitePool};
use crate::bridge::types::{CanonicalCharacter, SourceKind};

/// A locally-saved snapshot of a bridged character. The `(source, source_id)`
/// pair matches the live `CanonicalCharacter`, enabling drift detection when
/// the same character is live AND saved.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCharacter {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub foundry_world: Option<String>,
    pub name: String,
    pub canonical: CanonicalCharacter,
    pub saved_at: String,
    pub last_updated_at: String,
}

fn source_to_str(s: &SourceKind) -> &'static str {
    match s {
        SourceKind::Roll20 => "roll20",
        SourceKind::Foundry => "foundry",
    }
}

fn str_to_source(s: &str) -> Option<SourceKind> {
    match s {
        "roll20" => Some(SourceKind::Roll20),
        "foundry" => Some(SourceKind::Foundry),
        _ => None,
    }
}

async fn db_save(
    pool: &SqlitePool,
    canonical: &CanonicalCharacter,
    foundry_world: Option<String>,
) -> Result<i64, String> {
    let canonical_json = serde_json::to_string(canonical)
        .map_err(|e| format!("db/saved_character.save: serialize failed: {e}"))?;
    let result = sqlx::query(
        "INSERT INTO saved_characters
         (source, source_id, foundry_world, name, canonical_json)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&canonical.source))
    .bind(&canonical.source_id)
    .bind(&foundry_world)
    .bind(&canonical.name)
    .bind(&canonical_json)
    .execute(pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            "db/saved_character.save: already saved; use update".to_string()
        } else {
            format!("db/saved_character.save: {msg}")
        }
    })?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn save_character(
    pool: tauri::State<'_, crate::DbState>,
    canonical: CanonicalCharacter,
    foundry_world: Option<String>,
) -> Result<i64, String> {
    db_save(&pool.0, &canonical, foundry_world).await
}

async fn db_list(pool: &SqlitePool) -> Result<Vec<SavedCharacter>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, foundry_world, name, canonical_json,
                saved_at, last_updated_at
         FROM saved_characters
         ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/saved_character.list: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let source_str: String = r.get("source");
        let source = str_to_source(&source_str)
            .ok_or_else(|| format!("db/saved_character.list: unknown source '{source_str}'"))?;
        let canonical_json: String = r.get("canonical_json");
        let canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
            .map_err(|e| format!("db/saved_character.list: deserialize failed: {e}"))?;
        out.push(SavedCharacter {
            id: r.get("id"),
            source,
            source_id: r.get("source_id"),
            foundry_world: r.get("foundry_world"),
            name: r.get("name"),
            canonical,
            saved_at: r.get("saved_at"),
            last_updated_at: r.get("last_updated_at"),
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_saved_characters(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<SavedCharacter>, String> {
    db_list(&pool.0).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    canonical: &CanonicalCharacter,
) -> Result<(), String> {
    let canonical_json = serde_json::to_string(canonical)
        .map_err(|e| format!("db/saved_character.update: serialize failed: {e}"))?;
    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, name = ?, last_updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&canonical_json)
    .bind(&canonical.name)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.update: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/saved_character.update: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_saved_character(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    canonical: CanonicalCharacter,
) -> Result<(), String> {
    db_update(&pool.0, id, &canonical).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM saved_characters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/saved_character.delete: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/saved_character.delete: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_saved_character(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

pub(crate) async fn db_patch_field(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.patch_field: {e}"))?
        .ok_or_else(|| "db/saved_character.patch_field: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.patch_field: deserialize failed: {e}"))?;
    crate::shared::canonical_fields::apply_canonical_field(&mut canonical, name, value)?;
    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.patch_field: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, name = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(&canonical.name)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.patch_field: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.patch_field: not found".to_string());
    }
    Ok(())
}

/// Append a feature item to canonical.raw.items[]. Item shape matches what
/// Foundry's actor.create_feature executor produces (type=feature,
/// system.featuretype/description/points). The synthesized `_id` uses the
/// `local-<uuid>` convention (router spec §2.3) — survives until the next
/// "Update saved" replaces the blob with the live one.
pub(crate) async fn db_add_advantage(
    pool: &SqlitePool,
    id: i64,
    featuretype: &str,
    name: &str,
    description: &str,
    points: i32,
) -> Result<(), String> {
    match featuretype {
        "merit" | "flaw" | "background" | "boon" => {}
        other => return Err(format!(
            "db/saved_character.add_advantage: invalid featuretype: {other}"
        )),
    }

    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.add_advantage: {e}"))?
        .ok_or_else(|| "db/saved_character.add_advantage: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.add_advantage: deserialize failed: {e}"))?;

    let new_item = serde_json::json!({
        "_id": format!("local-{}", uuid::Uuid::new_v4()),
        "type": "feature",
        "name": name,
        "system": {
            "featuretype": featuretype,
            "description": description,
            "points": points,
        },
        "effects": [],
    });

    let raw = canonical.raw.as_object_mut().ok_or_else(||
        "db/saved_character.add_advantage: canonical.raw is not an object".to_string()
    )?;
    let items = raw.entry("items".to_string())
        .or_insert_with(|| serde_json::Value::Array(vec![]));
    let arr = items.as_array_mut().ok_or_else(||
        "db/saved_character.add_advantage: canonical.raw.items is not an array".to_string()
    )?;
    arr.push(new_item);

    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.add_advantage: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.add_advantage: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.add_advantage: not found".to_string());
    }
    Ok(())
}

/// Remove a feature item by `_id` AND `featuretype` (defense-in-depth so a
/// UI bug can't accidentally delete a discipline document via a matching id).
pub(crate) async fn db_remove_advantage(
    pool: &SqlitePool,
    id: i64,
    featuretype: &str,
    item_id: &str,
) -> Result<(), String> {
    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.remove_advantage: {e}"))?
        .ok_or_else(|| "db/saved_character.remove_advantage: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.remove_advantage: deserialize failed: {e}"))?;

    let raw = canonical.raw.as_object_mut().ok_or_else(||
        "db/saved_character.remove_advantage: canonical.raw is not an object".to_string()
    )?;
    let Some(items) = raw.get_mut("items").and_then(|v| v.as_array_mut()) else {
        return Err(format!(
            "db/saved_character.remove_advantage: no item with id '{item_id}'"
        ));
    };
    let original_len = items.len();
    items.retain(|item| {
        let id_match = item.get("_id").and_then(|v| v.as_str()) == Some(item_id);
        let ft_match = item
            .get("system")
            .and_then(|s| s.get("featuretype"))
            .and_then(|v| v.as_str())
            == Some(featuretype);
        !(id_match && ft_match)
    });
    if items.len() == original_len {
        return Err(format!(
            "db/saved_character.remove_advantage: no {featuretype} with id '{item_id}'"
        ));
    }

    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.remove_advantage: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.remove_advantage: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.remove_advantage: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn patch_saved_field(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    db_patch_field(&pool.0, id, &name, &value).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_url() -> &'static str { "sqlite::memory:" }

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect(pool_url()).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[allow(dead_code)]
    fn sample_canonical() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "abc123".to_string(),
            name: "Charlotte Reine".to_string(),
            controlled_by: None,
            hunger: Some(2),
            health: None,
            willpower: None,
            humanity: Some(7),
            humanity_stains: Some(0),
            blood_potency: Some(2),
            raw: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn migrations_apply_cleanly() {
        let _pool = fresh_pool().await;
    }

    #[tokio::test]
    async fn save_inserts_and_returns_id() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, Some("Chronicles of Chicago".into())).await.unwrap();
        assert!(id > 0);
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM saved_characters")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn save_twice_for_same_source_pair_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        db_save(&pool, &canonical, None).await.unwrap();
        let err = db_save(&pool, &canonical, None).await.unwrap_err();
        assert!(err.contains("already saved"), "expected 'already saved' in: {err}");
    }

    #[tokio::test]
    async fn list_returns_rows_ordered_by_id() {
        let pool = fresh_pool().await;
        // Insert two rows directly to avoid coupling this test to db_save.
        for sid in &["a", "b"] {
            sqlx::query(
                "INSERT INTO saved_characters
                 (source, source_id, foundry_world, name, canonical_json)
                 VALUES ('foundry', ?, NULL, 'X', '{\"source\":\"foundry\",\"source_id\":\"x\",\"name\":\"X\",\"controlled_by\":null,\"hunger\":null,\"health\":null,\"willpower\":null,\"humanity\":null,\"humanity_stains\":null,\"blood_potency\":null,\"raw\":{}}')"
            )
            .bind(sid)
            .execute(&pool).await.unwrap();
        }
        let list = db_list(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].id < list[1].id);
    }

    #[tokio::test]
    async fn update_overwrites_canonical_and_bumps_last_updated() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        let mut new_canonical = canonical.clone();
        new_canonical.hunger = Some(5);
        db_update(&pool, id, &new_canonical).await.unwrap();

        let list = db_list(&pool).await.unwrap();
        assert_eq!(list[0].canonical.hunger, Some(5));
        // saved_at should be unchanged; last_updated_at should be present (bumped).
        assert!(!list[0].last_updated_at.is_empty());
    }

    #[tokio::test]
    async fn update_missing_id_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let err = db_update(&pool, 9999, &canonical).await.unwrap_err();
        assert!(err.contains("not found"), "expected 'not found' in: {err}");
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();
        db_delete(&pool, id).await.unwrap();
        let list = db_list(&pool).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_delete(&pool, 9999).await.unwrap_err();
        assert!(err.contains("not found"), "expected 'not found' in: {err}");
    }

    #[tokio::test]
    async fn patch_field_updates_canonical_and_bumps_last_updated() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        db_patch_field(&pool, id, "hunger", &serde_json::json!(4))
            .await
            .unwrap();

        let list = db_list(&pool).await.unwrap();
        assert_eq!(list[0].canonical.hunger, Some(4));
        assert!(!list[0].last_updated_at.is_empty());
    }

    #[tokio::test]
    async fn patch_field_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_patch_field(&pool, 9999, "hunger", &serde_json::json!(0))
            .await
            .unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn patch_field_unknown_name_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        let err = db_patch_field(&pool, id, "xyzzy", &serde_json::json!(0))
            .await
            .unwrap_err();
        assert!(err.contains("unknown field 'xyzzy'"), "got: {err}");
    }

    #[tokio::test]
    async fn patch_field_type_mismatch_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        let err = db_patch_field(&pool, id, "hunger", &serde_json::json!("oops"))
            .await
            .unwrap_err();
        assert!(err.contains("expects integer"), "got: {err}");
    }

    // ── #8 advantage editor tests ────────────────────────────────────────

    fn sample_canonical_with_items(items: serde_json::Value) -> CanonicalCharacter {
        let mut c = sample_canonical();
        c.raw = serde_json::json!({ "items": items });
        c
    }

    async fn seed_with_canonical(pool: &SqlitePool, c: &CanonicalCharacter) -> i64 {
        db_save(pool, c, None).await.unwrap()
    }

    #[tokio::test]
    async fn add_advantage_happy_path_appends_item_with_local_uuid() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        db_add_advantage(&pool, id, "merit", "Iron Will", "Strong-minded.", 2)
            .await
            .unwrap();

        let list = db_list(&pool).await.unwrap();
        let items = list[0].canonical.raw.get("items")
            .and_then(|v| v.as_array())
            .expect("items array");
        assert_eq!(items.len(), 1);
        let item = &items[0];
        let item_id = item.get("_id").and_then(|v| v.as_str()).unwrap();
        assert!(item_id.starts_with("local-"), "got id: {item_id}");
        assert_eq!(item.get("type").and_then(|v| v.as_str()), Some("feature"));
        assert_eq!(item.get("name").and_then(|v| v.as_str()), Some("Iron Will"));
        let sys = item.get("system").unwrap();
        assert_eq!(sys.get("featuretype").and_then(|v| v.as_str()), Some("merit"));
        assert_eq!(sys.get("description").and_then(|v| v.as_str()), Some("Strong-minded."));
        assert_eq!(sys.get("points").and_then(|v| v.as_i64()), Some(2));
    }

    #[tokio::test]
    async fn add_advantage_invalid_featuretype_errors() {
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();
        let err = db_add_advantage(&pool, id, "discipline", "X", "y", 1)
            .await
            .unwrap_err();
        assert!(err.contains("invalid featuretype"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_add_advantage(&pool, 9999, "merit", "X", "y", 1)
            .await
            .unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_materializes_items_array_if_absent() {
        // sample_canonical() has raw = json!({}), no items key. Must work.
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();
        db_add_advantage(&pool, id, "boon", "Owed Favor", "From Camarilla.", 3)
            .await
            .unwrap();
        let list = db_list(&pool).await.unwrap();
        let items = list[0].canonical.raw.get("items")
            .and_then(|v| v.as_array())
            .expect("items array");
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn remove_advantage_happy_path() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical_with_items(serde_json::json!([
            { "_id": "item-keep",   "type": "feature", "name": "Keep",
              "system": { "featuretype": "merit", "description": "k", "points": 1 },
              "effects": [] },
            { "_id": "item-remove", "type": "feature", "name": "Remove",
              "system": { "featuretype": "merit", "description": "r", "points": 1 },
              "effects": [] },
        ]));
        let id = seed_with_canonical(&pool, &canonical).await;

        db_remove_advantage(&pool, id, "merit", "item-remove").await.unwrap();

        let list = db_list(&pool).await.unwrap();
        let items = list[0].canonical.raw.get("items")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("_id").and_then(|v| v.as_str()), Some("item-keep"));
        assert!(!list[0].last_updated_at.is_empty());
    }

    #[tokio::test]
    async fn remove_advantage_missing_id_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical_with_items(serde_json::json!([
            { "_id": "item-1", "type": "feature", "name": "X",
              "system": { "featuretype": "merit" }, "effects": [] },
        ]));
        let id = seed_with_canonical(&pool, &canonical).await;
        let err = db_remove_advantage(&pool, id, "merit", "nonexistent").await.unwrap_err();
        assert!(err.contains("no merit with id 'nonexistent'"), "got: {err}");
    }

    #[tokio::test]
    async fn remove_advantage_featuretype_mismatch_errors() {
        // Defense-in-depth: if the UI passes the wrong featuretype, the id-match
        // alone isn't enough — both must agree.
        let pool = fresh_pool().await;
        let canonical = sample_canonical_with_items(serde_json::json!([
            { "_id": "item-1", "type": "feature", "name": "X",
              "system": { "featuretype": "merit" }, "effects": [] },
        ]));
        let id = seed_with_canonical(&pool, &canonical).await;
        let err = db_remove_advantage(&pool, id, "flaw", "item-1").await.unwrap_err();
        assert!(err.contains("no flaw with id 'item-1'"), "got: {err}");
    }

    #[tokio::test]
    async fn remove_advantage_no_items_key_errors() {
        // sample_canonical()'s raw is {} — no items key at all.
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();
        let err = db_remove_advantage(&pool, id, "merit", "item-1").await.unwrap_err();
        assert!(err.contains("no item with id 'item-1'"), "got: {err}");
    }
}
