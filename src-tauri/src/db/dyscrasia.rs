use rand::Rng;
use sqlx::{Row, SqlitePool};
use crate::shared::types::{DyscrasiaEntry, ResonanceType};

fn rtype_to_str(r: &ResonanceType) -> &'static str {
    match r {
        ResonanceType::Phlegmatic => "Phlegmatic",
        ResonanceType::Melancholy => "Melancholy",
        ResonanceType::Choleric   => "Choleric",
        ResonanceType::Sanguine   => "Sanguine",
    }
}

fn str_to_rtype(s: &str) -> ResonanceType {
    match s {
        "Phlegmatic" => ResonanceType::Phlegmatic,
        "Melancholy" => ResonanceType::Melancholy,
        "Choleric"   => ResonanceType::Choleric,
        _            => ResonanceType::Sanguine,
    }
}

async fn db_list(pool: &SqlitePool, rtype: &str) -> Result<Vec<DyscrasiaEntry>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, resonance_type, name, description, bonus, is_custom
         FROM dyscrasias WHERE resonance_type = ? ORDER BY is_custom ASC, id ASC"
    )
    .bind(rtype)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| DyscrasiaEntry {
        id: r.get("id"),
        resonance_type: str_to_rtype(r.get("resonance_type")),
        name: r.get("name"),
        description: r.get("description"),
        bonus: r.get("bonus"),
        is_custom: r.get::<bool, _>("is_custom"),
    }).collect())
}

#[tauri::command]
pub async fn list_dyscrasias(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
) -> Result<Vec<DyscrasiaEntry>, String> {
    db_list(&pool.0, rtype_to_str(&resonance_type)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
    name: String,
    description: String,
    bonus: String,
) -> Result<DyscrasiaEntry, String> {
    let rtype = rtype_to_str(&resonance_type);
    let result = sqlx::query(
        "INSERT INTO dyscrasias (resonance_type, name, description, bonus, is_custom)
         VALUES (?, ?, ?, ?, 1)"
    )
    .bind(rtype).bind(&name).bind(&description).bind(&bonus)
    .execute(&*pool.0)
    .await
    .map_err(|e| e.to_string())?;

    Ok(DyscrasiaEntry {
        id: result.last_insert_rowid(),
        resonance_type,
        name,
        description,
        bonus,
        is_custom: true,
    })
}

#[tauri::command]
pub async fn update_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
    bonus: String,
) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE dyscrasias SET name = ?, description = ?, bonus = ? WHERE id = ? AND is_custom = 1"
    )
    .bind(&name).bind(&description).bind(&bonus).bind(id)
    .execute(&*pool.0)
    .await
    .map_err(|e| e.to_string())?;

    if result.rows_affected() == 0 {
        return Err("Dyscrasia not found or cannot be edited".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM dyscrasias WHERE id = ? AND is_custom = 1")
        .bind(id)
        .execute(&*pool.0)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn roll_random_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
) -> Result<Option<DyscrasiaEntry>, String> {
    let entries = db_list(&pool.0, rtype_to_str(&resonance_type))
        .await
        .map_err(|e| e.to_string())?;
    if entries.is_empty() {
        return Ok(None);
    }
    let idx = rand::thread_rng().gen_range(0..entries.len());
    Ok(Some(entries[idx].clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE dyscrasias (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                resonance_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                bonus TEXT NOT NULL,
                is_custom INTEGER NOT NULL DEFAULT 0
            )"
        ).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = test_pool().await;
        let result = db_list(&pool, "Choleric").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn insert_and_list_round_trips() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO dyscrasias (resonance_type, name, description, bonus, is_custom)
             VALUES ('Choleric', 'Rage', 'Pure anger', '+1 Potence', 1)"
        ).execute(&pool).await.unwrap();

        let entries = db_list(&pool, "Choleric").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Rage");
        assert_eq!(entries[0].resonance_type, ResonanceType::Choleric);
        assert!(entries[0].is_custom);
    }
}
