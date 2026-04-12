use crate::shared::types::{ResonanceRollResult, RollConfig};

#[tauri::command]
pub fn roll_resonance(_config: RollConfig) -> ResonanceRollResult {
    todo!()
}
