//! Tauri command wrapping shared::v5::skill_check::skill_check with thread_rng().
//! Synchronous — no I/O.

use crate::shared::v5::skill_check::skill_check;
use crate::shared::v5::types::{SkillCheckInput, SkillCheckResult};

#[tauri::command]
pub fn roll_skill_check(input: SkillCheckInput) -> SkillCheckResult {
    let mut rng = rand::thread_rng();
    skill_check(&input, &mut rng)
}
