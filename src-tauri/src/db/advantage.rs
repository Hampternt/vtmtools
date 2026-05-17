use rand::seq::SliceRandom;
use sqlx::{Row, SqlitePool};
use crate::shared::types::{Advantage, AdvantageKind, Field};

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

async fn db_list(pool: &SqlitePool) -> Result<Vec<Advantage>, String> {
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
}
