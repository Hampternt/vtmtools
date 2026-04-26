use serde::{Deserialize, Serialize};

/// Inbound messages from the Foundry module. Module sends `actors` on
/// initial connect (from `pushAllActors`) and `actor_update` on
/// updateActor / createActor / deleteActor hooks.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    Actors { actors: Vec<FoundryActor> },
    ActorUpdate { actor: FoundryActor },
    /// Module hello — no character data, just confirms it connected
    /// and registered the GM gate. Translated to an empty Vec.
    Hello,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundryActor {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    /// Raw `actor.system` blob — translate.rs picks paths verified in
    /// docs/reference/foundry-vtm5e-paths.md.
    pub system: serde_json::Value,
}

/// Frontend → Tauri payload for applying a dyscrasia to a Foundry
/// actor. Sent JSON-encoded as the `value: String` arg of
/// `bridge_set_attribute` when `name == "dyscrasia"`. The Foundry
/// source impl parses this back into the typed struct, stamps the
/// timestamp, renders the merit description HTML, and emits the
/// `apply_dyscrasia` wire shape.
#[derive(Debug, Deserialize)]
pub struct ApplyDyscrasiaPayload {
    pub dyscrasia_name: String,
    pub resonance_type: String,
    pub description: String,
    pub bonus: String,
}
