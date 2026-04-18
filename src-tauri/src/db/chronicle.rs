use sqlx::{Row, SqlitePool};
use crate::shared::types::Chronicle;

// -------- Pure CRUD helpers (testable without Tauri state) -------------

async fn db_list(pool: &SqlitePool) -> Result<Vec<Chronicle>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, description, created_at, updated_at
         FROM chronicles
         ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| Chronicle {
        id:          r.get("id"),
        name:        r.get("name"),
        description: r.get("description"),
        created_at:  r.get("created_at"),
        updated_at:  r.get("updated_at"),
    }).collect())
}

async fn db_get(pool: &SqlitePool, id: i64) -> Result<Chronicle, sqlx::Error> {
    let r = sqlx::query(
        "SELECT id, name, description, created_at, updated_at
         FROM chronicles WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(Chronicle {
        id:          r.get("id"),
        name:        r.get("name"),
        description: r.get("description"),
        created_at:  r.get("created_at"),
        updated_at:  r.get("updated_at"),
    })
}

async fn db_create(pool: &SqlitePool, name: &str, description: &str) -> Result<Chronicle, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO chronicles (name, description) VALUES (?, ?)"
    )
    .bind(name)
    .bind(description)
    .execute(pool)
    .await?;
    db_get(pool, result.last_insert_rowid()).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    description: &str,
) -> Result<Chronicle, sqlx::Error> {
    sqlx::query(
        "UPDATE chronicles SET name = ?, description = ? WHERE id = ?"
    )
    .bind(name)
    .bind(description)
    .bind(id)
    .execute(pool)
    .await?;
    db_get(pool, id).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM chronicles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// -------- Tauri commands (thin wrappers; map errors to Strings) --------

#[tauri::command]
pub async fn list_chronicles(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<Chronicle>, String> {
    db_list(&pool.0).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<Chronicle, String> {
    db_get(&pool.0, id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    name: String,
    description: String,
) -> Result<Chronicle, String> {
    db_create(&pool.0, &name, &description).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
) -> Result<Chronicle, String> {
    db_update(&pool.0, id, &name, &description).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await.map_err(|e| e.to_string())
}

// -------- Test helper (top-level so other db modules' tests can reach it) ------

/// Build an in-memory pool with the migration applied and foreign keys enabled.
/// Declared at the file top level (rather than inside `mod tests`) so that
/// sibling test modules (e.g. `db::node::tests`, `db::edge::tests`) can
/// import it as `crate::db::chronicle::test_pool`.
#[cfg(test)]
pub(crate) async fn test_pool() -> SqlitePool {
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(opts).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

// -------- Unit tests --------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = test_pool().await;
        assert!(db_list(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_then_list_round_trips() {
        let pool = test_pool().await;
        let c = db_create(&pool, "Anarch Nights", "test chronicle").await.unwrap();
        assert_eq!(c.name, "Anarch Nights");
        assert_eq!(c.description, "test chronicle");

        let all = db_list(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, c.id);
    }

    #[tokio::test]
    async fn update_changes_name() {
        let pool = test_pool().await;
        let c = db_create(&pool, "Old Name", "").await.unwrap();
        let updated = db_update(&pool, c.id, "New Name", "desc").await.unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.description, "desc");
    }

    #[tokio::test]
    async fn delete_removes_chronicle() {
        let pool = test_pool().await;
        let c = db_create(&pool, "X", "").await.unwrap();
        db_delete(&pool, c.id).await.unwrap();
        assert!(db_list(&pool).await.unwrap().is_empty());
    }
}
