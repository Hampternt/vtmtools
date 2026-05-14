# Foundry Helper Library — Phase 2: Game-roll helpers

> **Status:** implementable spec. Activates Phase 2 of the [Foundry Helper Library Roadmap](./2026-04-26-foundry-helper-library-roadmap.md). Produces one plan.
>
> **Audience:** anyone implementing the `game.*` umbrella helpers in vtmtools.

---

## §1 What this is

Two outbound bridge helpers that let vtmtools trigger in-Foundry actions:

- `game.roll_v5_pool` — issue a V5 dice roll into Foundry chat using the WoD5e system's roll API; the roll appears in Foundry's chat log just as if a player clicked their sheet's dice button. Result mirroring back into vtmtools is **deliberately out of scope** here — it lives in Character Tooling Phase 3 (Roll mirroring) which will activate the `chat.*` umbrella.
- `game.post_chat_as_actor` — post a non-roll chat message attributed to a given actor, useful for narration, declaration of intent, or future tools that want vtmtools-authored narrative beats to appear in the Foundry log.

Both ship per the [Foundry Helper Library Roadmap §7 Phase 2](./2026-04-26-foundry-helper-library-roadmap.md#7-build-order) deliverable list. `game.rouse_check` is **not** included — the [research spike findings in `docs/reference/foundry-vtm5e-rolls.md`](../../reference/foundry-vtm5e-rolls.md#triggering-a-roll--two-shapes) confirmed that a rouse check is `roll_v5_pool` with `value_paths: []` and `advanced_dice: 1`, so it doesn't need its own helper.

## §2 Why now

The research spike for this phase was effectively completed at file-creation time of `foundry-vtm5e-rolls.md` (commit `d16b5d960a` of WoD5e v5.3.17 was pinned; full API surface, dice mechanics, ChatMessage shape, hook behaviors, and a wire-integration sketch are documented). Phase 2's "research first" gate is satisfied.

The natural near-term consumer is Character Tooling Phase 3's roll-source toggle ("roll here" vs. "roll in Foundry"), but per the project's planning-early-for-future-features philosophy, the helpers ship now without waiting for that phase's plan to land.

## §3 File layout

The directory scaffolding already exists from FHL Phase 0; this phase fills stubs and adds frontend access plumbing.

```
src-tauri/src/bridge/foundry/
├── actions/
│   └── game.rs                    [FILL: build_roll_v5_pool, build_post_chat_as_actor]
└── types.rs                       [MODIFY: add RollV5PoolInput, PostChatAsActorInput]

src-tauri/src/tools/
├── foundry_chat.rs                [NEW: trigger_foundry_roll, post_foundry_chat Tauri commands]
└── mod.rs                         [MODIFY: pub mod foundry_chat;]

src-tauri/src/lib.rs               [MODIFY: register both Tauri commands in invoke_handler!]

src/lib/foundry-chat/
└── api.ts                         [NEW: triggerFoundryRoll, postFoundryChat typed wrappers]

vtmtools-bridge/scripts/foundry-actions/
└── game.js                        [FILL: rollV5Pool + postChatAsActor + handlers map]

vtmtools-bridge/module.json        [MODIFY: bump 0.2.0 → 0.3.0]
```

`actions/mod.rs` and `foundry-actions/index.js` already declare/import `game` — no change needed there.

## §4 Wire payloads (outbound, snake_case)

### `game.roll_v5_pool`

```json
{
  "type": "game.roll_v5_pool",
  "actor_id": "ObCGftjZjCvpPBdN",
  "value_paths": ["attributes.strength.value", "skills.brawl.value"],
  "difficulty": 4,
  "flavor": "Strength + Brawl",
  "advanced_dice": null,
  "selectors": ["attributes.strength", "skills.brawl", "physical"]
}
```

| Field | Type | Required | Semantics |
|---|---|---|---|
| `actor_id` | string | yes | Foundry actor `_id`. Validation: non-empty. |
| `value_paths` | string[] | yes | Foundry actor system dot-paths (e.g. `attributes.strength.value`). JS executor joins with spaces (WoD5e API expects space-separated). May be empty — `[]` produces a basic-pool of 0, which combined with `advanced_dice: 1` is exactly a rouse check. No validation on emptiness. |
| `difficulty` | u8 | yes | DV for the roll. `0` = no DV check. |
| `flavor` | string \| null | optional | Chat-message title (WoD5e calls this `label`). If null, JS executor derives from `value_paths`. |
| `advanced_dice` | u8 \| null | optional | If null, JS executor calls `WOD5E.api.getAdvancedDice({ actor })` to auto-derive (hunger for vampire, rage for werewolf, 0 for mortal). If set, overrides for custom-pool rolls. |
| `selectors` | string[] \| null | optional | Foundry situational-modifier selectors. If null, executor passes `[]`. Drives `actor.system.bonuses[*].selectors` matching. |

### `game.post_chat_as_actor`

```json
{
  "type": "game.post_chat_as_actor",
  "actor_id": "ObCGftjZjCvpPBdN",
  "content": "<p>Charlotte feels her humanity slipping…</p>",
  "flavor": "Narration",
  "roll_mode": "roll"
}
```

| Field | Type | Required | Semantics |
|---|---|---|---|
| `actor_id` | string | yes | Speaker attribution. JS executor calls `ChatMessage.getSpeaker({ actor })`. Validation: non-empty. |
| `content` | string | yes | Message body (HTML-rendered). Plain-text callers should HTML-escape upstream. Validation: non-empty. |
| `flavor` | string \| null | optional | Optional title/header above the message body. |
| `roll_mode` | string \| null | optional | One of `"roll"`, `"gmroll"`, `"blindroll"`, `"selfroll"`. Default `"roll"`. Validation: must match enum if present. |

## §5 Tauri command surface

Two new commands. Total command surface grows from 37 → 39.

```rust
// src-tauri/src/tools/foundry_chat.rs

#[tauri::command]
async fn trigger_foundry_roll(
    state: State<'_, Arc<BridgeState>>,
    input: RollV5PoolInput,
) -> Result<(), String>;

#[tauri::command]
async fn post_foundry_chat(
    state: State<'_, Arc<BridgeState>>,
    input: PostChatAsActorInput,
) -> Result<(), String>;
```

Each command:
1. Calls the Rust builder (`build_roll_v5_pool` or `build_post_chat_as_actor`) which validates the input and produces a `serde_json::Value` envelope. Validation failure → `Err("foundry/game.<verb>: <reason>")`.
2. Serializes the envelope via `serde_json::to_string(&envelope)`.
3. Calls the existing `bridge::commands::send_to_source(&conn, SourceKind::Foundry, text).await` (free function — must be promoted from private to `pub(crate)` so it's callable from `tools/foundry_chat.rs`). The function silently no-ops if Foundry isn't connected (matches `bridge_refresh`'s best-effort semantics); a per-call disconnect signal is left to a future iteration.
4. Returns `Ok(())`. The roll/chat-post is fire-and-forget at the wire level — actual roll outcome lands in Foundry chat. Result-mirroring is CT Phase 3.

The Tauri command handlers take `State<'_, BridgeConn>` (matching the existing `bridge_*` command signatures), NOT `State<'_, Arc<BridgeState>>`. `BridgeConn` is a `Arc`-wrapped tuple-struct around the bridge state; access goes through `conn.0.<field>`.

### Rust input types (in `src-tauri/src/bridge/foundry/types.rs`)

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollV5PoolInput {
    pub actor_id: String,
    pub value_paths: Vec<String>,
    pub difficulty: u8,
    pub flavor: Option<String>,
    pub advanced_dice: Option<u8>,
    pub selectors: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostChatAsActorInput {
    pub actor_id: String,
    pub content: String,
    pub flavor: Option<String>,
    pub roll_mode: Option<String>,
}
```

### Frontend wrappers (in `src/lib/foundry-chat/api.ts`)

```ts
import { invoke } from '@tauri-apps/api/core';

export interface RollV5PoolInput {
  actorId: string;
  valuePaths: string[];
  difficulty: number;
  flavor?: string | null;
  advancedDice?: number | null;
  selectors?: string[] | null;
}

export interface PostChatAsActorInput {
  actorId: string;
  content: string;
  flavor?: string | null;
  rollMode?: 'roll' | 'gmroll' | 'blindroll' | 'selfroll' | null;
}

export async function triggerFoundryRoll(input: RollV5PoolInput): Promise<void> {
  await invoke('trigger_foundry_roll', { input });
}

export async function postFoundryChat(input: PostChatAsActorInput): Promise<void> {
  await invoke('post_foundry_chat', { input });
}
```

Components import these — never call `invoke()` directly (CLAUDE.md hard rule).

## §6 Rust builder shape (`actions/game.rs`)

```rust
use serde_json::{json, Value};
use crate::bridge::foundry::types::{RollV5PoolInput, PostChatAsActorInput};

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
```

## §7 JS executor (`foundry-actions/game.js`)

```js
// Foundry game.* helper executors.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md

const MODULE_ID = "vtmtools-bridge";

async function rollV5Pool(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[${MODULE_ID}] game.roll_v5_pool: actor not found: ${msg.actor_id}`);
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  const paths = msg.value_paths ?? [];
  const advancedDice = msg.advanced_dice
    ?? WOD5E.api.getAdvancedDice({ actor });
  const label = msg.flavor ?? deriveFlavorFromPaths(paths);

  if (paths.length === 0) {
    // Rouse-style: zero basic dice + caller-supplied advanced dice. Use the
    // direct Roll API since RollFromDataset can't represent an empty pool.
    await WOD5E.api.Roll({
      basicDice: 0,
      advancedDice,
      actor,
      difficulty: msg.difficulty,
      flavor: label,
      quickRoll: true,
    });
    return;
  }

  await WOD5E.api.RollFromDataset({
    dataset: {
      valuePaths: paths.join(" "),
      label,
      difficulty: msg.difficulty,
      selectDialog: false,            // never pop the GM dialog from outside Foundry
      advancedDice,
      selectors: msg.selectors ?? [],
    },
    actor,
  });
}

async function postChatAsActor(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[${MODULE_ID}] game.post_chat_as_actor: actor not found: ${msg.actor_id}`);
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  await ChatMessage.create({
    speaker: ChatMessage.getSpeaker({ actor }),
    content: msg.content,
    flavor: msg.flavor ?? null,
    rollMode: msg.roll_mode ?? "roll",
  });
}

function deriveFlavorFromPaths(paths) {
  if (!paths || paths.length === 0) return "Roll";
  return paths
    .map((p) => p.split(".").slice(-2)[0])  // "skills.brawl.value" → "brawl"
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join(" + ");
}

export const handlers = {
  "game.roll_v5_pool": rollV5Pool,
  "game.post_chat_as_actor": postChatAsActor,
};
```

Note: these don't use `wireExecutor(fn)` (the actor.* wrapper) because they manage the actor lookup directly so they can `throw` on miss — `bridge.js` Plan 0 wraps handler exceptions into the error envelope returned to the desktop. The `wireExecutor` wrapper merely warns; for game.* the desktop should know about the failure (toast).

## §8 Error handling

| Failure | Where caught | Surfaces as |
|---|---|---|
| `actor_id` empty (both helpers) / `content` empty (post_chat_as_actor) | Rust builder | `Err("foundry/game.<verb>: <reason>")` from Tauri command → caller in `api.ts` propagates rejection |
| Invalid `roll_mode` | Rust builder | `Err("foundry/game.post_chat_as_actor: invalid roll_mode: <value>")` |
| No Foundry connection | `BridgeState::send_to` | `Err("foundry/game.<verb>: bridge not connected")` |
| Module-side actor not found | JS executor throws | Plan 0's `try/catch` in `bridge.js` produces an `error` envelope → desktop emits `bridge://foundry/error` event → frontend toast: `actor not found: <id>` |
| Module is pre-0.3.0 (doesn't know `game.*`) | `bridge.js::handleInbound` handler-map miss | Plan 0's error envelope: `{ type: "error", refers_to: "game.roll_v5_pool", code: "unknown_message_type", message: "..." }` → desktop toast |

**No new desktop event.** Existing `bridge://foundry/error` from Plan 0 covers all module-side errors.

## §9 Backward compatibility

Module 0.3.0 introduces new wire types but no protocol-version bump (Plan 0's `protocol_version: 1` still applies). The change is purely additive within protocol v1.

**Old desktop (post-0.2.0, pre-this-phase) ↔ new module (0.3.0):** old desktop never sends `game.*` envelopes, so no compatibility issue.

**New desktop (this phase) ↔ old module (0.2.0):** new desktop sends `game.*` envelopes; old module's handler-map lookup fails; Plan 0's error envelope returns; desktop toast surfaces "unknown message type" — graceful failure mode.

**Capability gating** is *not* added in this phase. The Plan 0 `Hello.capabilities` array exists and could be checked (e.g., refuse to issue `game.*` envelopes if `capabilities` does not contain `"game"`), but doing so would require coordinating a `capabilities` list addition in the module's Hello payload, which expands scope. Phase 2 keeps capability gating as a future tightening — current behavior (old module → error envelope) is acceptable.

If a future phase adds capability gating, the new module's Hello will report `capabilities: ["actors", "game"]` and the desktop's `bridge_get_source_info` consumer can surface a "this Foundry world does not support in-tool rolls" affordance pre-emptively.

## §10 Testing

### Rust unit tests (in `src-tauri/src/bridge/foundry/actions/game.rs::tests`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::foundry::types::{RollV5PoolInput, PostChatAsActorInput};

    fn roll_input() -> RollV5PoolInput {
        RollV5PoolInput {
            actor_id: "abc".into(),
            value_paths: vec!["attributes.strength.value".into()],
            difficulty: 3,
            flavor: None,
            advanced_dice: None,
            selectors: None,
        }
    }

    #[test] fn roll_v5_pool_envelope_shape() { /* type, actor_id, value_paths echoed */ }
    #[test] fn roll_v5_pool_empty_actor_id_errors() { /* */ }
    #[test] fn roll_v5_pool_empty_value_paths_allowed_for_rouse() { /* []+advanced=1 is a rouse */ }
    #[test] fn roll_v5_pool_selectors_default_empty_array() { /* selectors: [] when None */ }
    #[test] fn roll_v5_pool_advanced_dice_passes_through() { /* */ }

    #[test] fn post_chat_as_actor_envelope_shape() { /* */ }
    #[test] fn post_chat_as_actor_empty_content_errors() { /* */ }
    #[test] fn post_chat_as_actor_invalid_roll_mode_errors() { /* */ }
    #[test] fn post_chat_as_actor_default_roll_mode_is_roll() { /* */ }
}
```

Per ARCHITECTURE.md §10 testing convention: tests live next to the code under `#[cfg(test)] mod tests`.

### Manual smoke (after the implementation plan ships)

With `npm run tauri dev` running and a Foundry world with module 0.3.0 connected:

1. Open desktop dev-tools console.
2. Get a known actor id from the bridge store: `(await bridgeStore.characters)[0].sourceId` or similar.
3. Call:
   ```js
   await window.__TAURI_INTERNALS__.invoke('trigger_foundry_roll', {
     input: {
       actorId: '<id>',
       valuePaths: ['attributes.strength.value', 'skills.brawl.value'],
       difficulty: 3,
       flavor: 'Smoke test',
     },
   });
   ```
   Expected: a Strength + Brawl roll appears in Foundry chat with the right actor as speaker.
4. Call:
   ```js
   await window.__TAURI_INTERNALS__.invoke('post_foundry_chat', {
     input: {
       actorId: '<id>',
       content: '<p>Smoke test message</p>',
       flavor: 'Smoke',
     },
   });
   ```
   Expected: a chat message appears with the actor as speaker, "Smoke" header.
5. Call with invalid actorId → expect a toast surfacing `actor not found: <id>`.

### Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first. Final task of the implementation plan: run `./scripts/verify.sh` + manual smoke above + module version-bump commit.

## §11 Anti-scope

This phase MUST NOT touch:

- `src-tauri/src/bridge/foundry/actions/actor.rs` (Phase 1 territory, frozen)
- `src-tauri/src/bridge/foundry/actions/bridge.rs` (Plan 0 territory, frozen)
- `src-tauri/src/shared/v5/` (Plan 3 territory, frozen)
- Any `db/` module — game.* helpers are stateless wire passers
- Any inbound mirroring / `chat.*` umbrella — that is Character Tooling Phase 3

## §12 Plan dependencies

This phase depends on:

- **Plan 0 (bridge protocol consolidation)** — landed. Provides the `error` envelope and `bridge://foundry/error` event for failure surfacing.
- **FHL Phase 0 (wire-protocol scaffolding)** — landed. Provides the empty `actions/game.rs` and `foundry-actions/game.js` stubs and the handler-map dispatch in `bridge.js`.

This phase has zero file overlap with any other in-flight or planned phase. It can be executed standalone.

## §13 Open questions

None. The research spike resolved the WoD5e API surface; the wire shapes are derived from the spike + the rolls.md sketch; the Tauri command shape follows the Plan 3 v5-helpers precedent.

Phase 4 (Library sync) and Character Tooling Phase 3 (Roll mirroring) will each compose one or both of these helpers when they land.

## §14 Roadmap link

This phase is tracked under the "vtmtools roadmap" GitHub Project board. The implementation plan should propose `Closes #N` in the final commit's footer if a corresponding issue exists. (Per CLAUDE.md: never auto-create issues.)
