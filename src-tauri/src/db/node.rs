use sqlx::{Row, SqlitePool};
use crate::shared::types::{Node, Field};

// -------- Serialization helpers --------

fn serialize_tags(tags: &[String]) -> Result<String, String> {
    serde_json::to_string(tags).map_err(|e| e.to_string())
}

fn deserialize_tags(s: &str) -> Result<Vec<String>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

fn serialize_properties(props: &[Field]) -> Result<String, String> {
    serde_json::to_string(props).map_err(|e| e.to_string())
}

fn deserialize_properties(s: &str) -> Result<Vec<Field>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

fn row_to_node(r: &sqlx::sqlite::SqliteRow) -> Result<Node, String> {
    let tags_json: String = r.get("tags_json");
    let properties_json: String = r.get("properties_json");
    Ok(Node {
        id:           r.get("id"),
        chronicle_id: r.get("chronicle_id"),
        node_type:    r.get("type"),
        label:        r.get("label"),
        description:  r.get("description"),
        tags:         deserialize_tags(&tags_json)?,
        properties:   deserialize_properties(&properties_json)?,
        created_at:   r.get("created_at"),
        updated_at:   r.get("updated_at"),
    })
}

// -------- Pure CRUD helpers --------

async fn db_list(
    pool: &SqlitePool,
    chronicle_id: i64,
    type_filter: Option<&str>,
) -> Result<Vec<Node>, String> {
    let rows = match type_filter {
        Some(t) => sqlx::query(
            "SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
             FROM nodes WHERE chronicle_id = ? AND type = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).bind(t).fetch_all(pool).await,
        None => sqlx::query(
            "SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
             FROM nodes WHERE chronicle_id = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).fetch_all(pool).await,
    }.map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get(pool: &SqlitePool, id: i64) -> Result<Node, String> {
    let r = sqlx::query(
        "SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
         FROM nodes WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    row_to_node(&r)
}

async fn db_create(
    pool: &SqlitePool,
    chronicle_id: i64,
    node_type: &str,
    label: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<Node, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;
    let result = sqlx::query(
        "INSERT INTO nodes (chronicle_id, type, label, description, tags_json, properties_json)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(chronicle_id)
    .bind(node_type)
    .bind(label)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, result.last_insert_rowid()).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    node_type: &str,
    label: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<Node, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;
    sqlx::query(
        "UPDATE nodes SET type = ?, label = ?, description = ?, tags_json = ?, properties_json = ?
         WHERE id = ?"
    )
    .bind(node_type)
    .bind(label)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, id).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM nodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// -------- Tauri commands --------

#[tauri::command]
pub async fn list_nodes(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    type_filter: Option<String>,
) -> Result<Vec<Node>, String> {
    db_list(&pool.0, chronicle_id, type_filter.as_deref()).await
}

#[tauri::command]
pub async fn get_node(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<Node, String> {
    db_get(&pool.0, id).await
}

#[tauri::command]
pub async fn create_node(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    node_type: String,
    label: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Node, String> {
    db_create(&pool.0, chronicle_id, &node_type, &label, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn update_node(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    node_type: String,
    label: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Node, String> {
    db_update(&pool.0, id, &node_type, &label, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn delete_node(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

// -------- Unit tests --------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::chronicle::test_pool;
    use crate::shared::types::{FieldValue, StringFieldValue, NumberFieldValue};

    async fn make_chronicle(pool: &SqlitePool) -> i64 {
        let r = sqlx::query("INSERT INTO chronicles (name) VALUES ('Test')")
            .execute(pool).await.unwrap();
        r.last_insert_rowid()
    }

    #[tokio::test]
    async fn create_and_get_round_trips() {
        let pool = test_pool().await;
        let chronicle_id = make_chronicle(&pool).await;

        let props = vec![
            Field {
                name: "influence_rating".into(),
                value: FieldValue::Number { value: NumberFieldValue::Single(3.0) },
            },
            Field {
                name: "aliases".into(),
                value: FieldValue::String {
                    value: StringFieldValue::Multi(vec!["Nightjar".into(), "V".into()]),
                },
            },
        ];

        let created = db_create(
            &pool,
            chronicle_id,
            "area",
            "Manhattan",
            "The big borough",
            &["geographic".into(), "urban".into()],
            &props,
        ).await.unwrap();

        assert_eq!(created.label, "Manhattan");
        assert_eq!(created.node_type, "area");
        assert_eq!(created.tags, vec!["geographic".to_string(), "urban".into()]);
        assert_eq!(created.properties.len(), 2);

        let fetched = db_get(&pool, created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.properties, props);
    }

    #[tokio::test]
    async fn list_filters_by_chronicle_and_type() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;

        db_create(&pool, cid, "area",      "A", "", &[], &[]).await.unwrap();
        db_create(&pool, cid, "area",      "B", "", &[], &[]).await.unwrap();
        db_create(&pool, cid, "character", "C", "", &[], &[]).await.unwrap();

        let areas = db_list(&pool, cid, Some("area")).await.unwrap();
        assert_eq!(areas.len(), 2);
        let all = db_list(&pool, cid, None).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn update_persists_changes() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let n = db_create(&pool, cid, "area", "Old", "", &[], &[]).await.unwrap();

        let updated = db_update(&pool, n.id, "area", "New", "desc", &["tag1".into()], &[]).await.unwrap();
        assert_eq!(updated.label, "New");
        assert_eq!(updated.description, "desc");
        assert_eq!(updated.tags, vec!["tag1".to_string()]);
    }

    #[tokio::test]
    async fn delete_removes_node() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let n = db_create(&pool, cid, "area", "X", "", &[], &[]).await.unwrap();
        db_delete(&pool, n.id).await.unwrap();
        assert!(db_list(&pool, cid, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn chronicle_delete_cascades_to_nodes() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        db_create(&pool, cid, "area", "X", "", &[], &[]).await.unwrap();

        sqlx::query("DELETE FROM chronicles WHERE id = ?")
            .bind(cid).execute(&pool).await.unwrap();

        assert!(db_list(&pool, cid, None).await.unwrap().is_empty());
    }
}
