use sqlx::{Row, SqlitePool};
use crate::bridge::types::{CanonicalCharacter, SourceKind};

/// A locally-saved snapshot of a bridged character. The `(source, source_id)`
/// pair matches the live `CanonicalCharacter`, enabling drift detection when
/// the same character is live AND saved.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCharacter {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub foundry_world: Option<String>,
    pub name: String,
    pub canonical: CanonicalCharacter,
    pub saved_at: String,
    pub last_updated_at: String,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_url() -> &'static str { "sqlite::memory:" }

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect(pool_url()).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[allow(dead_code)]
    fn sample_canonical() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "abc123".to_string(),
            name: "Charlotte Reine".to_string(),
            controlled_by: None,
            hunger: Some(2),
            health: None,
            willpower: None,
            humanity: Some(7),
            humanity_stains: Some(0),
            blood_potency: Some(2),
            raw: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn migrations_apply_cleanly() {
        let _pool = fresh_pool().await;
    }
}
