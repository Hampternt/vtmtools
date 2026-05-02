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
}
