use rand::seq::SliceRandom;
use sqlx::{Row, SqlitePool};
use crate::shared::types::{Advantage, Field};

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
// Internal helpers (testable — take &SqlitePool directly)
// --------------------------------------------------------------------------

async fn db_list(pool: &SqlitePool) -> Result<Vec<Advantage>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, tags_json, properties_json, is_custom
         FROM advantages ORDER BY is_custom ASC, id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/advantage.list: {}", e))?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        let tags_json: String = r.get("tags_json");
        let properties_json: String = r.get("properties_json");
        out.push(Advantage {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            tags: deserialize_tags(&tags_json)?,
            properties: deserialize_properties(&properties_json)?,
            is_custom: r.get::<bool, _>("is_custom"),
        });
    }
    Ok(out)
}

async fn db_insert(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<Advantage, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;

    let result = sqlx::query(
        "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
         VALUES (?, ?, ?, ?, 1)"
    )
    .bind(name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.insert: {}", e))?;

    Ok(Advantage {
        id: result.last_insert_rowid(),
        name: name.to_string(),
        description: description.to_string(),
        tags: tags.to_vec(),
        properties: properties.to_vec(),
        is_custom: true,
    })
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<(), String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;

    let result = sqlx::query(
        "UPDATE advantages
         SET name = ?, description = ?, tags_json = ?, properties_json = ?
         WHERE id = ? AND is_custom = 1"
    )
    .bind(name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
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
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Advantage, String> {
    db_insert(&pool.0, &name, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn update_advantage(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<(), String> {
    db_update(&pool.0, id, &name, &description, &tags, &properties).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{Field, FieldValue, NumberFieldValue};
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE advantages (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                tags_json       TEXT NOT NULL DEFAULT '[]',
                properties_json TEXT NOT NULL DEFAULT '[]',
                is_custom       INTEGER NOT NULL DEFAULT 0
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

        let inserted = db_insert(&pool, "Iron Gullet", "Can drink rancid blood", &tags, &props)
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
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
             VALUES ('Allies', '', '[]', '[]', 0)"
        ).execute(&pool).await.unwrap();

        let err = db_update(&pool, 1, "X", "", &[], &[]).await.unwrap_err();
        assert!(err.contains("not editable"));
    }

    #[tokio::test]
    async fn delete_rejects_builtin_row() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
             VALUES ('Allies', '', '[]', '[]', 0)"
        ).execute(&pool).await.unwrap();

        let err = db_delete(&pool, 1).await.unwrap_err();
        assert!(err.contains("not deletable"));
    }

    #[tokio::test]
    async fn update_succeeds_on_custom_row() {
        let pool = test_pool().await;
        let inserted = db_insert(&pool, "Old Name", "", &[], &[]).await.unwrap();
        db_update(&pool, inserted.id, "New Name", "desc", &[], &[]).await.unwrap();
        let rows = db_list(&pool).await.unwrap();
        assert_eq!(rows[0].name, "New Name");
    }

    #[tokio::test]
    async fn delete_succeeds_on_custom_row() {
        let pool = test_pool().await;
        let inserted = db_insert(&pool, "To Delete", "", &[], &[]).await.unwrap();
        db_delete(&pool, inserted.id).await.unwrap();
        assert!(db_list(&pool).await.unwrap().is_empty());
    }

    async fn seed_three(pool: &SqlitePool) {
        db_insert(pool, "M1", "", &vec!["Merit".to_string()],     &[]).await.unwrap();
        db_insert(pool, "B1", "", &vec!["Background".to_string()], &[]).await.unwrap();
        db_insert(pool, "F1", "", &vec!["Flaw".to_string()],       &[]).await.unwrap();
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
}
