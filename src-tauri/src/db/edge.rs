use sqlx::{Row, SqlitePool};
use crate::shared::types::{Edge, Field, EdgeDirection};
use crate::db::node::would_create_cycle;

// -------- Serialization helper (mirrors node.rs for properties) --------

fn serialize_properties(props: &[Field]) -> Result<String, String> {
    serde_json::to_string(props).map_err(|e| e.to_string())
}

fn deserialize_properties(s: &str) -> Result<Vec<Field>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

fn row_to_edge(r: &sqlx::sqlite::SqliteRow) -> Result<Edge, String> {
    let properties_json: String = r.get("properties_json");
    Ok(Edge {
        id:              r.get("id"),
        chronicle_id:    r.get("chronicle_id"),
        from_node_id:    r.get("from_node_id"),
        to_node_id:      r.get("to_node_id"),
        edge_type:       r.get("edge_type"),
        description:     r.get("description"),
        properties:      deserialize_properties(&properties_json)?,
        created_at:      r.get("created_at"),
        updated_at:      r.get("updated_at"),
    })
}

// -------- Pure CRUD helpers --------

async fn db_list(
    pool: &SqlitePool,
    chronicle_id: i64,
    edge_type_filter: Option<&str>,
) -> Result<Vec<Edge>, String> {
    let rows = match edge_type_filter {
        Some(t) => sqlx::query(
            "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at
             FROM edges WHERE chronicle_id = ? AND edge_type = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).bind(t).fetch_all(pool).await,
        None => sqlx::query(
            "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at
             FROM edges WHERE chronicle_id = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).fetch_all(pool).await,
    }.map_err(|e| e.to_string())?;

    rows.iter().map(row_to_edge).collect()
}

async fn db_list_for_node(
    pool: &SqlitePool,
    node_id: i64,
    direction: &EdgeDirection,
    edge_type_filter: Option<&str>,
) -> Result<Vec<Edge>, String> {
    const COLS: &str = "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at FROM edges";

    let rows = match (direction, edge_type_filter) {
        (EdgeDirection::Out,  Some(t)) => sqlx::query(&format!("{COLS} WHERE from_node_id = ? AND edge_type = ? ORDER BY id ASC"))
            .bind(node_id).bind(t).fetch_all(pool).await,
        (EdgeDirection::Out,  None)    => sqlx::query(&format!("{COLS} WHERE from_node_id = ? ORDER BY id ASC"))
            .bind(node_id).fetch_all(pool).await,
        (EdgeDirection::In,   Some(t)) => sqlx::query(&format!("{COLS} WHERE to_node_id = ? AND edge_type = ? ORDER BY id ASC"))
            .bind(node_id).bind(t).fetch_all(pool).await,
        (EdgeDirection::In,   None)    => sqlx::query(&format!("{COLS} WHERE to_node_id = ? ORDER BY id ASC"))
            .bind(node_id).fetch_all(pool).await,
        (EdgeDirection::Both, Some(t)) => sqlx::query(&format!("{COLS} WHERE (from_node_id = ? OR to_node_id = ?) AND edge_type = ? ORDER BY id ASC"))
            .bind(node_id).bind(node_id).bind(t).fetch_all(pool).await,
        (EdgeDirection::Both, None)    => sqlx::query(&format!("{COLS} WHERE from_node_id = ? OR to_node_id = ? ORDER BY id ASC"))
            .bind(node_id).bind(node_id).fetch_all(pool).await,
    }.map_err(|e| e.to_string())?;

    rows.iter().map(row_to_edge).collect()
}

async fn db_get(pool: &SqlitePool, id: i64) -> Result<Edge, String> {
    let r = sqlx::query(
        "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at
         FROM edges WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    row_to_edge(&r)
}

async fn db_create(
    pool: &SqlitePool,
    chronicle_id: i64,
    from_node_id: i64,
    to_node_id: i64,
    edge_type: &str,
    description: &str,
    properties: &[Field],
) -> Result<Edge, String> {
    if edge_type == "contains"
        && would_create_cycle(pool, from_node_id, to_node_id).await?
    {
        return Err("cycle detected: creating this edge would form a cycle in the contains graph".to_string());
    }

    let properties_json = serialize_properties(properties)?;
    let result = sqlx::query(
        "INSERT INTO edges (chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(chronicle_id)
    .bind(from_node_id)
    .bind(to_node_id)
    .bind(edge_type)
    .bind(description)
    .bind(&properties_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, result.last_insert_rowid()).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    edge_type: &str,
    description: &str,
    properties: &[Field],
) -> Result<Edge, String> {
    let properties_json = serialize_properties(properties)?;
    sqlx::query(
        "UPDATE edges SET edge_type = ?, description = ?, properties_json = ?
         WHERE id = ?"
    )
    .bind(edge_type)
    .bind(description)
    .bind(&properties_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, id).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM edges WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// -------- Tauri commands --------

#[tauri::command]
pub async fn list_edges(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    edge_type_filter: Option<String>,
) -> Result<Vec<Edge>, String> {
    db_list(&pool.0, chronicle_id, edge_type_filter.as_deref()).await
}

#[tauri::command]
pub async fn list_edges_for_node(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
    direction: EdgeDirection,
    edge_type_filter: Option<String>,
) -> Result<Vec<Edge>, String> {
    db_list_for_node(&pool.0, node_id, &direction, edge_type_filter.as_deref()).await
}

#[tauri::command]
pub async fn create_edge(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    from_node_id: i64,
    to_node_id: i64,
    edge_type: String,
    description: String,
    properties: Vec<Field>,
) -> Result<Edge, String> {
    db_create(&pool.0, chronicle_id, from_node_id, to_node_id, &edge_type, &description, &properties).await
}

#[tauri::command]
pub async fn update_edge(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    edge_type: String,
    description: String,
    properties: Vec<Field>,
) -> Result<Edge, String> {
    db_update(&pool.0, id, &edge_type, &description, &properties).await
}

#[tauri::command]
pub async fn delete_edge(
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

    async fn mk_node(pool: &SqlitePool, cid: i64, label: &str) -> i64 {
        let r = sqlx::query("INSERT INTO nodes (chronicle_id, type, label) VALUES (?, 'area', ?)")
            .bind(cid).bind(label).execute(pool).await.unwrap();
        r.last_insert_rowid()
    }

    async fn setup(pool: &SqlitePool) -> (i64, i64, i64, i64) {
        let r = sqlx::query("INSERT INTO chronicles (name) VALUES ('T')")
            .execute(pool).await.unwrap();
        let cid = r.last_insert_rowid();
        let a = mk_node(pool, cid, "A").await;
        let b = mk_node(pool, cid, "B").await;
        let c = mk_node(pool, cid, "C").await;
        (cid, a, b, c)
    }

    #[tokio::test]
    async fn create_edge_happy_path() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        let e = db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        assert_eq!(e.from_node_id, a);
        assert_eq!(e.to_node_id, b);
        assert_eq!(e.edge_type, "contains");
    }

    #[tokio::test]
    async fn self_loop_rejected() {
        let pool = test_pool().await;
        let (cid, a, _, _) = setup(&pool).await;
        let result = db_create(&pool, cid, a, a, "contains", "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn duplicate_edge_same_type_rejected() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        let result = db_create(&pool, cid, a, b, "contains", "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn duplicate_edge_different_type_allowed() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains",    "", &[]).await.unwrap();
        db_create(&pool, cid, a, b, "adjacent-to", "", &[]).await.unwrap();
    }

    #[tokio::test]
    async fn second_contains_parent_rejected() {
        let pool = test_pool().await;
        let (cid, a, b, c) = setup(&pool).await;
        db_create(&pool, cid, a, c, "contains", "", &[]).await.unwrap();
        let result = db_create(&pool, cid, b, c, "contains", "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cycle_rejected() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        let result = db_create(&pool, cid, b, a, "contains", "", &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycle"));
    }

    #[tokio::test]
    async fn cycle_check_only_applies_to_contains() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains",     "", &[]).await.unwrap();
        db_create(&pool, cid, b, a, "allied-with", "", &[]).await.unwrap();
    }

    #[tokio::test]
    async fn list_for_node_filters_direction() {
        let pool = test_pool().await;
        let (cid, a, b, c) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        db_create(&pool, cid, c, a, "contains", "", &[]).await.unwrap();

        let out  = db_list_for_node(&pool, a, &EdgeDirection::Out,  None).await.unwrap();
        let inc  = db_list_for_node(&pool, a, &EdgeDirection::In,   None).await.unwrap();
        let both = db_list_for_node(&pool, a, &EdgeDirection::Both, None).await.unwrap();
        assert_eq!(out.len(),  1);
        assert_eq!(inc.len(),  1);
        assert_eq!(both.len(), 2);
    }

    #[tokio::test]
    async fn node_delete_cascades_to_edges() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        sqlx::query("DELETE FROM nodes WHERE id = ?").bind(a).execute(&pool).await.unwrap();

        let left = db_list(&pool, cid, None).await.unwrap();
        assert!(left.is_empty(), "edge should have cascaded");
    }
}
