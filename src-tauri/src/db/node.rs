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

// -------- Derived tree queries (all follow edge_type = 'contains') --------

async fn db_get_parent(pool: &SqlitePool, node_id: i64) -> Result<Option<Node>, String> {
    let r = sqlx::query(
        "SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at
         FROM nodes n
         JOIN edges e ON e.from_node_id = n.id
         WHERE e.to_node_id = ? AND e.edge_type = 'contains'
         LIMIT 1"
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    match r {
        Some(row) => Ok(Some(row_to_node(&row)?)),
        None => Ok(None),
    }
}

async fn db_get_children(pool: &SqlitePool, node_id: i64) -> Result<Vec<Node>, String> {
    let rows = sqlx::query(
        "SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at
         FROM nodes n
         JOIN edges e ON e.to_node_id = n.id
         WHERE e.from_node_id = ? AND e.edge_type = 'contains'
         ORDER BY n.id ASC"
    )
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get_siblings(pool: &SqlitePool, node_id: i64) -> Result<Vec<Node>, String> {
    let rows = sqlx::query(
        "SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at
         FROM nodes n
         JOIN edges e ON e.to_node_id = n.id
         WHERE e.edge_type = 'contains'
           AND e.from_node_id = (
               SELECT from_node_id FROM edges
               WHERE to_node_id = ? AND edge_type = 'contains'
               LIMIT 1
           )
           AND n.id != ?
         ORDER BY n.id ASC"
    )
    .bind(node_id)
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get_path_to_root(pool: &SqlitePool, node_id: i64) -> Result<Vec<Node>, String> {
    let rows = sqlx::query(
        "WITH RECURSIVE ancestors(id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at, depth) AS (
            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, 0
            FROM nodes n WHERE n.id = ?

            UNION ALL

            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, a.depth + 1
            FROM nodes n
            JOIN edges e ON e.from_node_id = n.id
            JOIN ancestors a ON a.id = e.to_node_id
            WHERE e.edge_type = 'contains' AND a.depth < 32
        )
        SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
        FROM ancestors WHERE depth > 0 ORDER BY depth ASC"
    )
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get_subtree(
    pool: &SqlitePool,
    node_id: i64,
    max_depth: Option<i32>,
) -> Result<Vec<Node>, String> {
    let cap = max_depth.unwrap_or(32);
    let rows = sqlx::query(
        "WITH RECURSIVE descendants(id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at, depth) AS (
            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, 0
            FROM nodes n WHERE n.id = ?

            UNION ALL

            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, d.depth + 1
            FROM nodes n
            JOIN edges e ON e.to_node_id = n.id
            JOIN descendants d ON d.id = e.from_node_id
            WHERE e.edge_type = 'contains' AND d.depth < ?
        )
        SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
        FROM descendants WHERE depth > 0 ORDER BY depth ASC, id ASC"
    )
    .bind(node_id)
    .bind(cap)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

/// Returns true if creating a contains edge `from -> to` would produce a cycle.
/// Cycle exists if `from` is already in `to`'s descendant set (via contains).
pub(crate) async fn would_create_cycle(
    pool: &SqlitePool,
    from_node_id: i64,
    to_node_id: i64,
) -> Result<bool, String> {
    let row = sqlx::query(
        "WITH RECURSIVE descendants(id, depth) AS (
            SELECT ?, 0
            UNION ALL
            SELECT e.to_node_id, d.depth + 1
            FROM edges e
            JOIN descendants d ON e.from_node_id = d.id
            WHERE e.edge_type = 'contains' AND d.depth < 32
        )
        SELECT COUNT(*) AS cnt FROM descendants WHERE id = ?"
    )
    .bind(to_node_id)
    .bind(from_node_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let cnt: i64 = row.get("cnt");
    Ok(cnt > 0)
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

// -------- Derived-query Tauri commands --------

#[tauri::command]
pub async fn get_parent_of(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Option<Node>, String> {
    db_get_parent(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_children_of(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Vec<Node>, String> {
    db_get_children(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_siblings_of(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Vec<Node>, String> {
    db_get_siblings(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_path_to_root(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Vec<Node>, String> {
    db_get_path_to_root(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_subtree(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
    max_depth: Option<i32>,
) -> Result<Vec<Node>, String> {
    db_get_subtree(&pool.0, node_id, max_depth).await
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

    /// Helper: insert a raw contains edge for test scaffolding, bypassing edge.rs logic.
    async fn insert_contains(pool: &SqlitePool, chronicle_id: i64, from: i64, to: i64) {
        sqlx::query(
            "INSERT INTO edges (chronicle_id, from_node_id, to_node_id, edge_type)
             VALUES (?, ?, ?, 'contains')"
        )
        .bind(chronicle_id).bind(from).bind(to)
        .execute(pool).await.unwrap();
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

    #[tokio::test]
    async fn get_parent_returns_some_when_parent_exists() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let parent = db_create(&pool, cid, "area", "Parent", "", &[], &[]).await.unwrap();
        let child  = db_create(&pool, cid, "area", "Child",  "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, parent.id, child.id).await;

        let p = db_get_parent(&pool, child.id).await.unwrap();
        assert!(p.is_some());
        assert_eq!(p.unwrap().id, parent.id);
    }

    #[tokio::test]
    async fn get_parent_returns_none_for_root() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let root = db_create(&pool, cid, "area", "Root", "", &[], &[]).await.unwrap();
        assert!(db_get_parent(&pool, root.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_children_returns_all_direct_children() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let parent = db_create(&pool, cid, "area", "Parent", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "C", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, parent.id, a.id).await;
        insert_contains(&pool, cid, parent.id, b.id).await;
        insert_contains(&pool, cid, a.id,      c.id).await;

        let kids = db_get_children(&pool, parent.id).await.unwrap();
        let ids: Vec<i64> = kids.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }

    #[tokio::test]
    async fn get_siblings_returns_peers() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let parent = db_create(&pool, cid, "area", "P", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "C", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, parent.id, a.id).await;
        insert_contains(&pool, cid, parent.id, b.id).await;
        insert_contains(&pool, cid, parent.id, c.id).await;

        let sibs = db_get_siblings(&pool, a.id).await.unwrap();
        let ids: Vec<i64> = sibs.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![b.id, c.id]);
    }

    #[tokio::test]
    async fn get_siblings_empty_for_root() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let n = db_create(&pool, cid, "area", "Solo", "", &[], &[]).await.unwrap();
        assert!(db_get_siblings(&pool, n.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_path_to_root_returns_ancestors_bottom_up() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let g = db_create(&pool, cid, "area", "State",     "", &[], &[]).await.unwrap();
        let p = db_create(&pool, cid, "area", "City",      "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "Borough",   "", &[], &[]).await.unwrap();
        let l = db_create(&pool, cid, "area", "Neighbor",  "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, g.id, p.id).await;
        insert_contains(&pool, cid, p.id, c.id).await;
        insert_contains(&pool, cid, c.id, l.id).await;

        let path = db_get_path_to_root(&pool, l.id).await.unwrap();
        let ids: Vec<i64> = path.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![c.id, p.id, g.id]);
    }

    #[tokio::test]
    async fn get_subtree_returns_descendants() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let r = db_create(&pool, cid, "area", "Root", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A",    "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B",    "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "C",    "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, r.id, a.id).await;
        insert_contains(&pool, cid, r.id, b.id).await;
        insert_contains(&pool, cid, a.id, c.id).await;

        let sub = db_get_subtree(&pool, r.id, None).await.unwrap();
        let ids: Vec<i64> = sub.iter().map(|n| n.id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&a.id));
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
    }

    #[tokio::test]
    async fn get_subtree_respects_max_depth() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let r = db_create(&pool, cid, "area", "R", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, r.id, a.id).await;
        insert_contains(&pool, cid, a.id, b.id).await;

        let sub = db_get_subtree(&pool, r.id, Some(1)).await.unwrap();
        let ids: Vec<i64> = sub.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![a.id]);
    }

    #[tokio::test]
    async fn would_create_cycle_true_for_back_edge() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, a.id, b.id).await;

        assert!(would_create_cycle(&pool, b.id, a.id).await.unwrap());
    }

    #[tokio::test]
    async fn would_create_cycle_false_for_safe_edge() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();

        assert!(!would_create_cycle(&pool, a.id, b.id).await.unwrap());
    }
}
