use serde::{Deserialize, Serialize};

/// Inbound messages from the Foundry module.
///
/// Hello fields are all `Option<…>` for backward compatibility with 0.1.0
/// modules that send `{ "type": "hello" }` with no payload. Missing
/// `protocol_version` is treated by the desktop as `0` (legacy); missing
/// `capabilities` defaults to `["actors"]` (preserves always-send-actors).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    Actors { actors: Vec<FoundryActor> },
    ActorUpdate { actor: FoundryActor },
    Hello {
        #[serde(default)] protocol_version: Option<u32>,
        #[serde(default)] world_id: Option<String>,
        #[serde(default)] world_title: Option<String>,
        #[serde(default)] system_id: Option<String>,
        #[serde(default)] system_version: Option<String>,
        #[serde(default)] capabilities: Option<Vec<String>>,
    },
    /// Module-side handler threw; surfaced to the GM via toast.
    Error {
        refers_to: String,
        #[serde(default)] request_id: Option<String>,
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundryActor {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    /// Raw `actor.system` blob — translate.rs picks paths verified in
    /// docs/reference/foundry-vtm5e-paths.md.
    pub system: serde_json::Value,
    /// Embedded item documents (`actor.items.contents.map(i => i.toObject())`).
    /// Each item is a full Foundry Document source, including its own
    /// `system` and embedded `effects`. Defaults to `Value::Null` for
    /// payloads from pre-0.4.0 modules.
    #[serde(default)]
    pub items: serde_json::Value,
    /// Actor-level ActiveEffect documents (`actor.effects.contents.map(e => e.toObject())`).
    /// Item-attached effects are nested inside `items[].effects` and arrive
    /// via `items` instead. Defaults to `Value::Null` for pre-0.4.0 payloads.
    #[serde(default)]
    pub effects: serde_json::Value,
}

/// Frontend → Tauri payload for applying a dyscrasia to a Foundry
/// actor. Sent JSON-encoded as the `value: String` arg of
/// `bridge_set_attribute` when `name == "dyscrasia"`. The Foundry
/// source impl parses this back into the typed struct, stamps the
/// timestamp, renders the merit description HTML, and emits the
/// `actor.apply_dyscrasia` wire shape.
#[derive(Debug, Deserialize)]
pub struct ApplyDyscrasiaPayload {
    pub dyscrasia_name: String,
    pub resonance_type: String,
    pub description: String,
    pub bonus: String,
}

/// Payload for actor.append_private_notes_line wire message.
/// Used at feature-time when a frontend tool wants to append a notes line.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AppendPrivateNotesLinePayload {
    pub line: String,
}

/// Payload for actor.replace_private_notes wire message.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ReplacePrivateNotesPayload {
    pub full_text: String,
}

/// Payload for actor.create_feature wire message.
/// `featuretype` must be one of "merit", "flaw", "background", "boon".
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CreateFeaturePayload {
    pub featuretype: String,
    pub name: String,
    pub description: String,
    pub points: i32,
}

/// Payload for actor.delete_items_by_prefix wire message.
/// `featuretype` is optional — when None, only `item_type` and `name_prefix`
/// filter the deletion set.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct DeleteItemsByPrefixPayload {
    pub item_type: String,
    pub featuretype: Option<String>,
    pub name_prefix: String,
}

/// Payload for actor.delete_item_by_id wire message.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct DeleteItemByIdPayload {
    pub item_id: String,
}

/// Payload for actor.update_item_field wire message. `path` is a
/// dot-namespaced field path on the embedded Item document (e.g.
/// "system.points", "name", "system.featuretype"). Frontend tools
/// use this to edit merits, flaws, etc. in place.
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UpdateItemFieldPayload {
    pub item_id: String,
    pub path: String,
    pub value: serde_json::Value,
}

/// Input for the `trigger_foundry_roll` Tauri command (frontend → Rust).
/// Becomes the source of the outbound `game.roll_v5_pool` envelope.
/// Empty `value_paths` is allowed — `[]` + `advanced_dice: 1` is a rouse check.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollV5PoolInput {
    pub actor_id: String,
    pub value_paths: Vec<String>,
    pub difficulty: u8,
    pub flavor: Option<String>,
    pub advanced_dice: Option<u8>,
    pub selectors: Option<Vec<String>>,
    /// One of "roll" / "gmroll" / "blindroll" / "selfroll". None = "roll".
    /// Mirrors the field on PostChatAsActorInput.
    pub roll_mode: Option<String>,
    /// Net pool modifier from the GM Screen popover (sum of active card pool
    /// deltas). i32 because penalties are negative. None = 0. When non-zero,
    /// the JS executor switches to direct WOD5E.api.Roll to bypass selectors-
    /// based situational-bonus auto-apply (avoids double-counting modifier
    /// cards that have been pushed to the sheet via GM Screen Plan C).
    pub pool_modifier: Option<i32>,
}

/// Input for the `post_foundry_chat` Tauri command (frontend → Rust).
/// Becomes the source of the outbound `game.post_chat_as_actor` envelope.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostChatAsActorInput {
    pub actor_id: String,
    pub content: String,
    pub flavor: Option<String>,
    pub roll_mode: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundry_actor_deserializes_with_items_and_effects() {
        let wire = r#"{
            "id": "abc",
            "name": "Jesper",
            "owner": null,
            "system": { "hunger": { "value": 1 } },
            "items": [
                { "_id": "i1", "type": "feature", "name": "Iron Will",
                  "system": { "featuretype": "merit", "points": 2 },
                  "effects": [] }
            ],
            "effects": [
                { "_id": "e1", "name": "Bonus", "disabled": false, "changes": [] }
            ]
        }"#;

        let actor: FoundryActor = serde_json::from_str(wire).expect("parses");
        assert_eq!(actor.id, "abc");
        assert!(actor.items.is_array(), "items must be an array");
        assert_eq!(actor.items.as_array().unwrap().len(), 1);
        assert_eq!(actor.items[0]["system"]["featuretype"], "merit");
        assert_eq!(actor.effects.as_array().unwrap().len(), 1);
    }

    #[test]
    fn foundry_actor_deserializes_without_items_and_effects_legacy() {
        // Legacy 0.3.0 module sends only id/name/owner/system. After the
        // wire bump, those payloads must still deserialize — items and
        // effects default to Value::Null via #[serde(default)].
        let wire = r#"{
            "id": "abc", "name": "Jesper", "owner": null,
            "system": {}
        }"#;
        let actor: FoundryActor = serde_json::from_str(wire).expect("legacy parses");
        assert_eq!(actor.id, "abc");
    }
}
