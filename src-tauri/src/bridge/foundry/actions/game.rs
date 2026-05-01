// Foundry game.* helper builders.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

use serde_json::{json, Value};

use crate::bridge::foundry::types::{PostChatAsActorInput, RollV5PoolInput};

const VALID_ROLL_MODES: &[&str] = &["roll", "gmroll", "blindroll", "selfroll"];

pub fn build_roll_v5_pool(input: &RollV5PoolInput) -> Result<Value, String> {
    if input.actor_id.is_empty() {
        return Err("foundry/game.roll_v5_pool: actor_id is required".into());
    }
    // Note: value_paths may be empty — empty paths + advanced_dice=1 is a
    // rouse check (basic pool = 0, one hunger die). No emptiness check.
    Ok(json!({
        "type": "game.roll_v5_pool",
        "actor_id": input.actor_id,
        "value_paths": input.value_paths,
        "difficulty": input.difficulty,
        "flavor": input.flavor,
        "advanced_dice": input.advanced_dice,
        "selectors": input.selectors.clone().unwrap_or_default(),
    }))
}

pub fn build_post_chat_as_actor(input: &PostChatAsActorInput) -> Result<Value, String> {
    if input.actor_id.is_empty() {
        return Err("foundry/game.post_chat_as_actor: actor_id is required".into());
    }
    if input.content.is_empty() {
        return Err("foundry/game.post_chat_as_actor: content is required".into());
    }
    if let Some(rm) = &input.roll_mode {
        if !VALID_ROLL_MODES.contains(&rm.as_str()) {
            return Err(format!(
                "foundry/game.post_chat_as_actor: invalid roll_mode: {rm}"
            ));
        }
    }
    Ok(json!({
        "type": "game.post_chat_as_actor",
        "actor_id": input.actor_id,
        "content": input.content,
        "flavor": input.flavor,
        "roll_mode": input.roll_mode.as_deref().unwrap_or("roll"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_roll_input() -> RollV5PoolInput {
        RollV5PoolInput {
            actor_id: "abc".into(),
            value_paths: vec!["attributes.strength.value".into(), "skills.brawl.value".into()],
            difficulty: 3,
            flavor: Some("Strength + Brawl".into()),
            advanced_dice: None,
            selectors: None,
        }
    }

    #[test]
    fn roll_v5_pool_envelope_shape() {
        let v = build_roll_v5_pool(&sample_roll_input()).expect("ok");
        assert_eq!(v["type"], "game.roll_v5_pool");
        assert_eq!(v["actor_id"], "abc");
        assert_eq!(
            v["value_paths"],
            json!(["attributes.strength.value", "skills.brawl.value"])
        );
        assert_eq!(v["difficulty"], 3);
        assert_eq!(v["flavor"], "Strength + Brawl");
    }

    #[test]
    fn roll_v5_pool_empty_actor_id_errors() {
        let mut input = sample_roll_input();
        input.actor_id = "".into();
        let err = build_roll_v5_pool(&input).expect_err("must reject empty actor_id");
        assert!(err.contains("actor_id"), "{err}");
    }

    #[test]
    fn roll_v5_pool_empty_value_paths_allowed_for_rouse() {
        // [] + advanced_dice=1 is the rouse-check pattern. Builder must permit it.
        let mut input = sample_roll_input();
        input.value_paths = vec![];
        input.advanced_dice = Some(1);
        let v = build_roll_v5_pool(&input).expect("rouse-shape input must build");
        assert_eq!(v["value_paths"], json!([]));
        assert_eq!(v["advanced_dice"], 1);
    }

    #[test]
    fn roll_v5_pool_selectors_default_empty_array() {
        let v = build_roll_v5_pool(&sample_roll_input()).expect("ok");
        assert_eq!(v["selectors"], json!([]));
    }

    #[test]
    fn roll_v5_pool_advanced_dice_passes_through() {
        let mut input = sample_roll_input();
        input.advanced_dice = Some(2);
        let v = build_roll_v5_pool(&input).expect("ok");
        assert_eq!(v["advanced_dice"], 2);
    }

    fn sample_chat_input() -> PostChatAsActorInput {
        PostChatAsActorInput {
            actor_id: "abc".into(),
            content: "<p>Hello world</p>".into(),
            flavor: Some("Narration".into()),
            roll_mode: None,
        }
    }

    #[test]
    fn post_chat_as_actor_envelope_shape() {
        let v = build_post_chat_as_actor(&sample_chat_input()).expect("ok");
        assert_eq!(v["type"], "game.post_chat_as_actor");
        assert_eq!(v["actor_id"], "abc");
        assert_eq!(v["content"], "<p>Hello world</p>");
        assert_eq!(v["flavor"], "Narration");
    }

    #[test]
    fn post_chat_as_actor_empty_content_errors() {
        let mut input = sample_chat_input();
        input.content = "".into();
        let err =
            build_post_chat_as_actor(&input).expect_err("must reject empty content");
        assert!(err.contains("content"), "{err}");
    }

    #[test]
    fn post_chat_as_actor_invalid_roll_mode_errors() {
        let mut input = sample_chat_input();
        input.roll_mode = Some("shouting".into());
        let err = build_post_chat_as_actor(&input)
            .expect_err("must reject invalid roll_mode");
        assert!(err.contains("roll_mode"), "{err}");
    }

    #[test]
    fn post_chat_as_actor_default_roll_mode_is_roll() {
        let v = build_post_chat_as_actor(&sample_chat_input()).expect("ok");
        assert_eq!(v["roll_mode"], "roll");
    }
}
