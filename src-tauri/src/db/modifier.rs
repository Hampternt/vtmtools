use sqlx::{Row, SqlitePool};
use crate::bridge::types::SourceKind;
use crate::shared::modifier::{
    CharacterModifier, ModifierBinding, ModifierEffect, ModifierKind,
};

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

fn row_to_modifier(r: &sqlx::sqlite::SqliteRow) -> Result<CharacterModifier, String> {
    let source_str: String = r.get("source");
    let source = str_to_source(&source_str)
        .ok_or_else(|| format!("db/modifier.list: unknown source '{source_str}'"))?;
    let effects_json: String = r.get("effects_json");
    let effects: Vec<ModifierEffect> = serde_json::from_str(&effects_json)
        .map_err(|e| format!("db/modifier.list: effects deserialize: {e}"))?;
    let binding_json: String = r.get("binding_json");
    let binding: ModifierBinding = serde_json::from_str(&binding_json)
        .map_err(|e| format!("db/modifier.list: binding deserialize: {e}"))?;
    let tags_json: String = r.get("tags_json");
    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| format!("db/modifier.list: tags deserialize: {e}"))?;
    let captured_json: String = r.try_get("foundry_captured_labels_json").unwrap_or_else(|_| "[]".to_string());
    let foundry_captured_labels: Vec<String> = serde_json::from_str(&captured_json)
        .map_err(|e| format!("db/modifier.list: captured labels deserialize: {e}"))?;
    Ok(CharacterModifier {
        id: r.get("id"),
        source,
        source_id: r.get("source_id"),
        name: r.get("name"),
        description: r.get("description"),
        effects,
        binding,
        tags,
        is_active: r.get::<bool, _>("is_active"),
        is_hidden: r.get::<bool, _>("is_hidden"),
        origin_template_id: r.get("origin_template_id"),
        foundry_captured_labels,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub(crate) async fn db_list(
    pool: &SqlitePool,
    source: &SourceKind,
    source_id: &str,
) -> Result<Vec<CharacterModifier>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, foundry_captured_labels_json, created_at, updated_at
         FROM character_modifiers
         WHERE source = ? AND source_id = ?
         ORDER BY id ASC"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/modifier.list: {e}"))?;
    rows.iter().map(row_to_modifier).collect()
}

pub(crate) async fn db_list_all(pool: &SqlitePool) -> Result<Vec<CharacterModifier>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, foundry_captured_labels_json, created_at, updated_at
         FROM character_modifiers
         ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/modifier.list_all: {e}"))?;
    rows.iter().map(row_to_modifier).collect()
}

#[tauri::command]
pub async fn list_character_modifiers(
    pool: tauri::State<'_, crate::DbState>,
    source: SourceKind,
    source_id: String,
) -> Result<Vec<CharacterModifier>, String> {
    db_list(&pool.0, &source, &source_id).await
}

#[tauri::command]
pub async fn list_all_character_modifiers(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<CharacterModifier>, String> {
    db_list_all(&pool.0).await
}

use crate::shared::modifier::{NewCharacterModifier, ModifierPatch};

pub(crate) async fn db_add(
    pool: &SqlitePool,
    input: NewCharacterModifier,
) -> Result<CharacterModifier, String> {
    if input.name.trim().is_empty() {
        return Err("db/modifier.add: empty name".to_string());
    }
    let effects_json = serde_json::to_string(&input.effects)
        .map_err(|e| format!("db/modifier.add: serialize effects: {e}"))?;
    let binding_json = serde_json::to_string(&input.binding)
        .map_err(|e| format!("db/modifier.add: serialize binding: {e}"))?;
    let tags_json = serde_json::to_string(&input.tags)
        .map_err(|e| format!("db/modifier.add: serialize tags: {e}"))?;
    let captured_labels_json = serde_json::to_string(&input.foundry_captured_labels)
        .map_err(|e| format!("db/modifier.add: serialize captured labels: {e}"))?;

    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          origin_template_id, foundry_captured_labels_json)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&input.source))
    .bind(&input.source_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&binding_json)
    .bind(&tags_json)
    .bind(input.origin_template_id)
    .bind(&captured_labels_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.add: {e}"))?;
    let id = result.last_insert_rowid();
    db_get(pool, id).await
}

/// Public single-id loader. Thin shim over the crate-private `db_get` so
/// non-IPC callers (e.g. `tools::gm_screen::do_push_to_foundry`) can fetch
/// a modifier by its row id without taking a Tauri `State`.
pub async fn get_modifier_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<CharacterModifier, String> {
    db_get(pool, id).await
}

pub(crate) async fn db_get(pool: &SqlitePool, id: i64) -> Result<CharacterModifier, String> {
    let row = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, foundry_captured_labels_json, created_at, updated_at
         FROM character_modifiers WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/modifier.get: {e}"))?
    .ok_or_else(|| "db/modifier.get: not found".to_string())?;
    row_to_modifier(&row)
}

#[tauri::command]
pub async fn add_character_modifier(
    pool: tauri::State<'_, crate::DbState>,
    input: NewCharacterModifier,
) -> Result<CharacterModifier, String> {
    db_add(&pool.0, input).await
}

pub(crate) async fn db_update(
    pool: &SqlitePool,
    id: i64,
    patch: ModifierPatch,
) -> Result<CharacterModifier, String> {
    // Load existing, apply patch in memory, write back. Simpler than dynamic SQL
    // and avoids COALESCE-with-JSON gymnastics.
    let mut current = db_get(pool, id).await
        .map_err(|e| if e.contains("not found") { "db/modifier.update: not found".to_string() } else { format!("db/modifier.update: {e}") })?;

    if let Some(name) = patch.name {
        if name.trim().is_empty() {
            return Err("db/modifier.update: empty name".to_string());
        }
        current.name = name;
    }
    if let Some(desc) = patch.description { current.description = desc; }
    if let Some(effects) = patch.effects   { current.effects = effects; }
    if let Some(tags) = patch.tags         { current.tags = tags; }

    let effects_json = serde_json::to_string(&current.effects)
        .map_err(|e| format!("db/modifier.update: serialize effects: {e}"))?;
    let tags_json = serde_json::to_string(&current.tags)
        .map_err(|e| format!("db/modifier.update: serialize tags: {e}"))?;

    let result = sqlx::query(
        "UPDATE character_modifiers
         SET name = ?, description = ?, effects_json = ?, tags_json = ?,
             updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&current.name)
    .bind(&current.description)
    .bind(&effects_json)
    .bind(&tags_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.update: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/modifier.update: not found".to_string());
    }
    db_get(pool, id).await
}

#[tauri::command]
pub async fn update_character_modifier(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    patch: ModifierPatch,
) -> Result<CharacterModifier, String> {
    db_update(&pool.0, id, patch).await
}

pub(crate) async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM character_modifiers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/modifier.delete: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/modifier.delete: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_character_modifier(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

pub(crate) async fn db_set_active(pool: &SqlitePool, id: i64, value: bool) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE character_modifiers SET is_active = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(value as i64)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.set_active: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/modifier.set_active: not found".to_string());
    }
    Ok(())
}

pub(crate) async fn db_set_hidden(pool: &SqlitePool, id: i64, value: bool) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE character_modifiers SET is_hidden = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(value as i64)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.set_hidden: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/modifier.set_hidden: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn set_modifier_active(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    is_active: bool,
) -> Result<(), String> {
    db_set_active(&pool.0, id, is_active).await
}

#[tauri::command]
pub async fn set_modifier_hidden(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    is_hidden: bool,
) -> Result<(), String> {
    db_set_hidden(&pool.0, id, is_hidden).await
}

/// Idempotent upsert. If a row exists for (source, source_id, binding=Advantage{item_id}),
/// returns it unchanged. Otherwise inserts with empty effects, is_active=false,
/// is_hidden=false. Spec §8.2.
pub(crate) async fn db_materialize_advantage(
    pool: &SqlitePool,
    source: &SourceKind,
    source_id: &str,
    item_id: &str,
    name: &str,
    description: &str,
) -> Result<CharacterModifier, String> {
    if name.trim().is_empty() {
        return Err("db/modifier.materialize: empty name".to_string());
    }

    let existing = sqlx::query(
        "SELECT id FROM character_modifiers
         WHERE source = ? AND source_id = ?
           AND json_extract(binding_json, '$.kind') = 'advantage'
           AND json_extract(binding_json, '$.item_id') = ?
         LIMIT 1"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/modifier.materialize: {e}"))?;

    if let Some(row) = existing {
        let id: i64 = row.get("id");
        return db_get(pool, id).await;
    }

    // Build the binding JSON inline (the enum's tag/snake_case variant rename is
    // the source of truth for the literal we write here).
    let binding_json = format!("{{\"kind\":\"advantage\",\"item_id\":{}}}",
        serde_json::to_string(item_id).map_err(|e| format!("db/modifier.materialize: encode item_id: {e}"))?);

    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          foundry_captured_labels_json)
         VALUES (?, ?, ?, ?, '[]', ?, '[]', '[]')"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .bind(name)
    .bind(description)
    .bind(&binding_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.materialize: {e}"))?;
    db_get(pool, result.last_insert_rowid()).await
}

#[tauri::command]
pub async fn materialize_advantage_modifier(
    pool: tauri::State<'_, crate::DbState>,
    source: SourceKind,
    source_id: String,
    item_id: String,
    name: String,
    description: String,
) -> Result<CharacterModifier, String> {
    db_materialize_advantage(&pool.0, &source, &source_id, &item_id, &name, &description).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = fresh_pool().await;
        let result = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn round_trip_preserves_effects_binding_tags() {
        let pool = fresh_pool().await;
        // Insert directly (db_add doesn't exist yet — round-trip-only test).
        sqlx::query(
            "INSERT INTO character_modifiers
             (source, source_id, name, description, effects_json, binding_json, tags_json, is_active, is_hidden)
             VALUES ('foundry', 'abc', 'Beautiful', 'desc',
                     '[{\"kind\":\"pool\",\"scope\":\"Social\",\"delta\":1,\"note\":null}]',
                     '{\"kind\":\"advantage\",\"item_id\":\"item-xyz\"}',
                     '[\"Social\",\"Looks\"]',
                     1, 0)"
        )
        .execute(&pool).await.unwrap();

        let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert_eq!(list.len(), 1);
        let m = &list[0];
        assert_eq!(m.name, "Beautiful");
        assert_eq!(m.effects.len(), 1);
        assert_eq!(m.effects[0].kind, ModifierKind::Pool);
        assert_eq!(m.effects[0].scope.as_deref(), Some("Social"));
        assert_eq!(m.effects[0].delta, Some(1));
        match &m.binding {
            ModifierBinding::Advantage { item_id } => assert_eq!(item_id, "item-xyz"),
            other => panic!("expected Advantage binding, got {other:?}"),
        }
        assert_eq!(m.tags, vec!["Social".to_string(), "Looks".to_string()]);
        assert!(m.is_active);
        assert!(!m.is_hidden);
    }

    #[tokio::test]
    async fn list_all_returns_rows_across_characters() {
        let pool = fresh_pool().await;
        for sid in &["a", "b", "c"] {
            sqlx::query(
                "INSERT INTO character_modifiers (source, source_id, name) VALUES ('foundry', ?, 'X')"
            )
            .bind(sid)
            .execute(&pool).await.unwrap();
        }
        let list = db_list_all(&pool).await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn list_filters_by_source_and_source_id() {
        let pool = fresh_pool().await;
        sqlx::query("INSERT INTO character_modifiers (source, source_id, name) VALUES ('foundry', 'abc', 'X')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO character_modifiers (source, source_id, name) VALUES ('foundry', 'def', 'Y')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO character_modifiers (source, source_id, name) VALUES ('roll20', 'abc', 'Z')")
            .execute(&pool).await.unwrap();
        let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "X");
    }

    fn sample_new(source_id: &str) -> crate::shared::modifier::NewCharacterModifier {
        use crate::shared::modifier::*;
        NewCharacterModifier {
            source: SourceKind::Foundry,
            source_id: source_id.to_string(),
            name: "Beautiful".to_string(),
            description: "Looks bonus".to_string(),
            effects: vec![ModifierEffect {
                kind: ModifierKind::Pool,
                scope: Some("Social".to_string()),
                delta: Some(1),
                note: None,
                paths: Vec::new(),
            }],
            binding: ModifierBinding::Free,
            tags: vec!["Social".to_string()],
            origin_template_id: None,
            foundry_captured_labels: vec![],
        }
    }

    #[tokio::test]
    async fn add_inserts_and_returns_full_record() {
        let pool = fresh_pool().await;
        let m = db_add(&pool, sample_new("abc")).await.unwrap();
        assert!(m.id > 0);
        assert_eq!(m.name, "Beautiful");
        assert_eq!(m.effects.len(), 1);
        assert!(matches!(m.binding, ModifierBinding::Free));
        assert!(!m.is_active);
        assert!(!m.is_hidden);
        let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn add_rejects_empty_name() {
        let pool = fresh_pool().await;
        let mut new = sample_new("abc");
        new.name = String::new();
        let err = db_add(&pool, new).await.unwrap_err();
        assert!(err.contains("empty name"), "got: {err}");
    }

    #[tokio::test]
    async fn update_applies_partial_patch_and_preserves_untouched_fields() {
        let pool = fresh_pool().await;
        let m = db_add(&pool, sample_new("abc")).await.unwrap();
        let original_desc = m.description.clone();

        let patch = ModifierPatch {
            name: Some("Renamed".to_string()),
            description: None,
            effects: None,
            tags: None,
        };
        let updated = db_update(&pool, m.id, patch).await.unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.description, original_desc);
        assert_eq!(updated.effects.len(), 1); // untouched
        assert_eq!(updated.tags, vec!["Social".to_string()]); // untouched
    }

    #[tokio::test]
    async fn update_missing_id_returns_not_found() {
        let pool = fresh_pool().await;
        let patch = ModifierPatch { name: Some("X".into()), description: None, effects: None, tags: None };
        let err = db_update(&pool, 9999, patch).await.unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn delete_removes_row() {
        let pool = fresh_pool().await;
        let m = db_add(&pool, sample_new("abc")).await.unwrap();
        db_delete(&pool, m.id).await.unwrap();
        let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_id_returns_not_found() {
        let pool = fresh_pool().await;
        let err = db_delete(&pool, 9999).await.unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn set_active_flips_flag() {
        let pool = fresh_pool().await;
        let m = db_add(&pool, sample_new("abc")).await.unwrap();
        assert!(!m.is_active);
        db_set_active(&pool, m.id, true).await.unwrap();
        let after = db_get(&pool, m.id).await.unwrap();
        assert!(after.is_active);
        db_set_active(&pool, m.id, false).await.unwrap();
        let after = db_get(&pool, m.id).await.unwrap();
        assert!(!after.is_active);
    }

    #[tokio::test]
    async fn set_active_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_set_active(&pool, 9999, true).await.unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn set_hidden_flips_flag() {
        let pool = fresh_pool().await;
        let m = db_add(&pool, sample_new("abc")).await.unwrap();
        assert!(!m.is_hidden);
        db_set_hidden(&pool, m.id, true).await.unwrap();
        let after = db_get(&pool, m.id).await.unwrap();
        assert!(after.is_hidden);
    }

    #[tokio::test]
    async fn set_hidden_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_set_hidden(&pool, 9999, true).await.unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn materialize_inserts_when_absent() {
        let pool = fresh_pool().await;
        let m = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-merit-1",
            "Beautiful", "Looks bonus",
        ).await.unwrap();
        assert!(m.id > 0);
        assert_eq!(m.name, "Beautiful");
        assert_eq!(m.description, "Looks bonus");
        assert!(m.effects.is_empty());
        assert!(!m.is_active);
        assert!(!m.is_hidden);
        match &m.binding {
            ModifierBinding::Advantage { item_id } => assert_eq!(item_id, "item-merit-1"),
            other => panic!("expected Advantage binding, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn materialize_returns_existing_unchanged_when_present() {
        let pool = fresh_pool().await;
        let first = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-merit-1",
            "Beautiful", "Looks bonus",
        ).await.unwrap();

        // Mutate the row to verify the second call does NOT overwrite it.
        db_set_active(&pool, first.id, true).await.unwrap();

        // Calling materialize again with different name/description must NOT change the row.
        let second = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-merit-1",
            "Different name", "Different description",
        ).await.unwrap();

        assert_eq!(second.id, first.id);
        assert_eq!(second.name, "Beautiful");           // original preserved
        assert_eq!(second.description, "Looks bonus");  // original preserved
        assert!(second.is_active);                      // mutation preserved
    }

    #[tokio::test]
    async fn materialize_distinguishes_by_character_and_by_item_id() {
        let pool = fresh_pool().await;
        let a = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-x", "X", "x",
        ).await.unwrap();
        // Same item_id, different character → distinct row.
        let b = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-2", "item-x", "X", "x",
        ).await.unwrap();
        // Same character, different item_id → distinct row.
        let c = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-y", "Y", "y",
        ).await.unwrap();
        assert_ne!(a.id, b.id);
        assert_ne!(a.id, c.id);
        assert_ne!(b.id, c.id);
    }

    #[tokio::test]
    async fn materialize_allows_existing_free_modifiers_for_same_character() {
        // No unique constraint per spec §5 rationale — a free-floating modifier
        // and an advantage-bound modifier coexist freely.
        let pool = fresh_pool().await;
        let _free = db_add(&pool, sample_new("char-1")).await.unwrap();
        let bound = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-1", "Bound", "b",
        ).await.unwrap();
        let list = db_list(&pool, &SourceKind::Foundry, "char-1").await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|m| m.id == bound.id));
    }

    #[tokio::test]
    async fn materialize_rejects_empty_name() {
        let pool = fresh_pool().await;
        let err = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "char-1", "item-1", "", "desc",
        ).await.unwrap_err();
        assert!(err.contains("empty name"), "got: {err}");
    }

    #[tokio::test]
    async fn get_by_id_returns_inserted_row() {
        use crate::shared::modifier::*;
        let pool = fresh_pool().await;
        let new = NewCharacterModifier {
            source: SourceKind::Foundry,
            source_id: "actor-x".into(),
            name: "Test Mod".into(),
            description: String::new(),
            effects: vec![ModifierEffect {
                kind: ModifierKind::Pool,
                scope: None,
                delta: Some(2),
                note: None,
                paths: vec!["attributes.strength".into()],
            }],
            binding: ModifierBinding::Advantage { item_id: "item-y".into() },
            tags: vec!["combat".into()],
            origin_template_id: None,
            foundry_captured_labels: vec![],
        };
        let added = db_add(&pool, new).await.unwrap();
        let loaded = get_modifier_by_id(&pool, added.id).await.unwrap();
        assert_eq!(loaded.id, added.id);
        assert_eq!(loaded.name, "Test Mod");
        assert_eq!(loaded.effects.len(), 1);
        assert_eq!(loaded.effects[0].paths, vec!["attributes.strength".to_string()]);
        assert!(matches!(loaded.binding, ModifierBinding::Advantage { ref item_id } if item_id == "item-y"));
    }

    #[tokio::test]
    async fn get_by_id_unknown_returns_err() {
        let pool = fresh_pool().await;
        let err = get_modifier_by_id(&pool, 99999).await.expect_err("unknown id must err");
        assert!(err.contains("99999") || err.to_lowercase().contains("not found"), "got: {err}");
    }

    #[test]
    fn modifier_effect_serde_back_compat_and_roundtrip() {
        // Legacy effects_json (no `paths` field) deserializes with empty paths.
        let legacy = r#"{"kind":"pool","scope":"Strength","delta":2,"note":null}"#;
        let parsed: crate::shared::modifier::ModifierEffect =
            serde_json::from_str(legacy).expect("legacy shape parses");
        assert_eq!(parsed.paths, Vec::<String>::new());
        assert_eq!(parsed.delta, Some(2));

        // New shape round-trips cleanly.
        let new_shape = crate::shared::modifier::ModifierEffect {
            kind: crate::shared::modifier::ModifierKind::Pool,
            scope: Some("Strength rolls".into()),
            delta: Some(3),
            note: None,
            paths: vec!["attributes.strength".into(), "skills.brawl".into()],
        };
        let json = serde_json::to_string(&new_shape).unwrap();
        let back: crate::shared::modifier::ModifierEffect =
            serde_json::from_str(&json).unwrap();
        assert_eq!(back, new_shape);
    }
}
