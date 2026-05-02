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
                origin_template_id, created_at, updated_at
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
                origin_template_id, created_at, updated_at
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

    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          origin_template_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&input.source))
    .bind(&input.source_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&binding_json)
    .bind(&tags_json)
    .bind(input.origin_template_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.add: {e}"))?;
    let id = result.last_insert_rowid();
    db_get(pool, id).await
}

pub(crate) async fn db_get(pool: &SqlitePool, id: i64) -> Result<CharacterModifier, String> {
    let row = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, created_at, updated_at
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
            }],
            binding: ModifierBinding::Free,
            tags: vec!["Social".to_string()],
            origin_template_id: None,
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
}
