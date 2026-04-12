use crate::shared::resonance::execute_roll;
use crate::shared::types::{ResonanceRollResult, RollConfig};

/// Executes the full resonance roll sequence.
/// The dyscrasia field in the result is always None — the GM fetches it
/// separately via roll_random_dyscrasia or picks manually.
#[tauri::command]
pub fn roll_resonance(config: RollConfig) -> ResonanceRollResult {
    execute_roll(&config)
}
