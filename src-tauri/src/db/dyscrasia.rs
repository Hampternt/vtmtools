use crate::shared::types::{DyscrasiaEntry, ResonanceType};

#[tauri::command]
pub async fn list_dyscrasias(
    _pool: tauri::State<'_, crate::DbState>,
    _resonance_type: ResonanceType,
) -> Result<Vec<DyscrasiaEntry>, String> {
    todo!()
}

#[tauri::command]
pub async fn add_dyscrasia(
    _pool: tauri::State<'_, crate::DbState>,
    _resonance_type: ResonanceType,
    _name: String,
    _description: String,
    _bonus: String,
) -> Result<DyscrasiaEntry, String> {
    todo!()
}

#[tauri::command]
pub async fn update_dyscrasia(
    _pool: tauri::State<'_, crate::DbState>,
    _id: i64,
    _name: String,
    _description: String,
    _bonus: String,
) -> Result<(), String> {
    todo!()
}

#[tauri::command]
pub async fn delete_dyscrasia(
    _pool: tauri::State<'_, crate::DbState>,
    _id: i64,
) -> Result<(), String> {
    todo!()
}

#[tauri::command]
pub async fn roll_random_dyscrasia(
    _pool: tauri::State<'_, crate::DbState>,
    _resonance_type: ResonanceType,
) -> Result<Option<DyscrasiaEntry>, String> {
    todo!()
}
