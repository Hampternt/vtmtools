# Foundry Helper Library Phase 2 — Game-Roll Helpers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the FHL Phase 2 outbound `game.*` helpers — `game.roll_v5_pool` (V5 dice rolls into Foundry chat) and `game.post_chat_as_actor` (actor-attributed non-roll chat posts) — with Tauri commands, frontend wrappers, JS executors, and a module version bump to 0.3.0.

**Architecture:** Wire-protocol-typed-per-helper (per FHL §3 Approach A). Each helper has a Rust input struct in `bridge/foundry/types.rs`, a Rust builder in `actions/game.rs` producing a `serde_json::Value` envelope, a Tauri command in `tools/foundry_chat.rs` that calls the builder + `BridgeState::send_to`, a typed frontend wrapper in `src/lib/foundry-chat/api.ts`, and a JS executor in `vtmtools-bridge/scripts/foundry-actions/game.js`. Outbound only — result mirroring is Character Tooling Phase 3.

**Tech Stack:** Rust + Tauri 2 (desktop), SvelteKit + TypeScript (frontend), vanilla JS (Foundry module), `serde_json`, WoD5e `WOD5E.api.RollFromDataset` / `WOD5E.api.Roll` / `ChatMessage.create`.

**Spec:** `docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md`.

**Total tasks: 8.** All tasks except the final verification gate end with a commit; per the project's CLAUDE.md hard rule, every commit-ending task runs `./scripts/verify.sh` before committing.

---

## Task overview

| # | Title | Files | Tests added |
|---|---|---|---|
| 1 | Add `game.*` input types | `types.rs` | none (compile-only) |
| 2 | `build_roll_v5_pool` + tests | `actions/game.rs` | 5 |
| 3 | `build_post_chat_as_actor` + tests | `actions/game.rs` | 4 |
| 4 | Tauri commands + register in `lib.rs` | `tools/foundry_chat.rs` (new), `tools/mod.rs`, `lib.rs` | none |
| 5 | Frontend typed wrappers | `src/lib/foundry-chat/api.ts` (new) | none (TS compile) |
| 6 | JS executors `rollV5Pool` + `postChatAsActor` | `foundry-actions/game.js` | none (manual smoke) |
| 7 | Module version bump 0.2.0 → 0.3.0 | `vtmtools-bridge/module.json` | none |
| 8 | Final verification gate | none — verify only | none |

---

## Task 1: Add `game.*` input types

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs`

**Anti-scope:** Do NOT touch `actions/game.rs` yet. Do NOT add other unrelated types.

**Depends on:** none.

**Invariants cited:** ARCHITECTURE.md §4 (Tauri IPC types), spec §5 (Tauri command input types).

- [ ] **Step 1: Open `src-tauri/src/bridge/foundry/types.rs`** and read the existing structure (imports, existing payload structs).

- [ ] **Step 2: Append the two input structs near the bottom of the file (after the existing `*Payload` structs, before any tests)**

```rust
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
```

If `Deserialize` is not already imported at the top of the file, add `use serde::Deserialize;` to the imports. (Other structs in the file likely already import it — check first.)

- [ ] **Step 3: Run `cargo check`**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: clean. (Warnings about unused types are expected and acceptable until Task 2 consumes them — see memory `feedback_dead_code_acceptable.md`.)

- [ ] **Step 4: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/bridge/foundry/types.rs
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add game.* input types

RollV5PoolInput + PostChatAsActorInput, used as Tauri command input
shapes. Outbound wire envelopes are produced by the builders in
actions/game.rs (next task).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Implement `build_roll_v5_pool` + tests

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/game.rs` (currently a single comment-only stub from FHL Phase 0).

**Anti-scope:** Do NOT add `build_post_chat_as_actor` yet (Task 3). Do NOT touch any other file.

**Depends on:** Task 1.

**Invariants cited:** ARCHITECTURE.md §10 (`#[cfg(test)] mod tests` per file), spec §6 (builder shape), spec §10 (test list for roll_v5_pool).

- [ ] **Step 1: Replace `actions/game.rs` contents with module header + imports + 5 failing tests**

Open `src-tauri/src/bridge/foundry/actions/game.rs` and replace its contents entirely with:

```rust
// Foundry game.* helper builders.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

use serde_json::{json, Value};

use crate::bridge::foundry::types::{PostChatAsActorInput, RollV5PoolInput};

const VALID_ROLL_MODES: &[&str] = &["roll", "gmroll", "blindroll", "selfroll"];

pub fn build_roll_v5_pool(_input: &RollV5PoolInput) -> Result<Value, String> {
    todo!("implemented in Task 2 step 3")
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
        // advanced_dice + selectors covered by dedicated tests below.
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
        // selectors: None on input → [] on the wire (never null) for cleaner JS-side handling.
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
}
```

- [ ] **Step 2: Run the new tests to confirm they all fail (panic on `todo!()`)**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p vtmtools bridge::foundry::actions::game::tests::roll_v5_pool 2>&1 | tail -20
```

Expected: 5 tests, all FAIL (panicked at `not yet implemented`).

- [ ] **Step 3: Replace the `todo!()` body of `build_roll_v5_pool`**

```rust
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
```

- [ ] **Step 4: Run the tests again to confirm all 5 pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p vtmtools bridge::foundry::actions::game::tests::roll_v5_pool 2>&1 | tail -10
```

Expected: 5 passed; 0 failed.

- [ ] **Step 5: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/game.rs
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add build_roll_v5_pool primitive + tests

Validates actor_id; allows empty value_paths (rouse-check pattern);
defaults selectors to [] on the wire when None. 5 tests cover envelope
shape, validation, rouse case, defaults, and pass-through.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Implement `build_post_chat_as_actor` + tests

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/game.rs`

**Anti-scope:** Do NOT touch any other file.

**Depends on:** Task 2.

**Invariants cited:** spec §6 (builder shape), spec §10 (test list for post_chat_as_actor).

- [ ] **Step 1: Add a `todo!()` stub for `build_post_chat_as_actor` and 4 failing tests**

Append to `src-tauri/src/bridge/foundry/actions/game.rs` (after the existing `build_roll_v5_pool`, before `#[cfg(test)] mod tests`):

```rust
pub fn build_post_chat_as_actor(_input: &PostChatAsActorInput) -> Result<Value, String> {
    todo!("implemented in Task 3 step 3")
}
```

Inside the existing `mod tests { ... }` block, append (after the existing roll_v5_pool tests):

```rust
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
```

- [ ] **Step 2: Run the new tests to confirm they all fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p vtmtools bridge::foundry::actions::game::tests::post_chat_as_actor 2>&1 | tail -20
```

Expected: 4 tests, all FAIL (panic on `todo!()`).

- [ ] **Step 3: Replace the `todo!()` body of `build_post_chat_as_actor`**

```rust
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

- [ ] **Step 4: Run all `bridge::foundry::actions::game::tests` to confirm all 9 pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -p vtmtools bridge::foundry::actions::game::tests 2>&1 | tail -15
```

Expected: 9 passed; 0 failed.

- [ ] **Step 5: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/game.rs
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add build_post_chat_as_actor primitive + tests

Validates actor_id + content; rejects roll_mode outside the
{roll, gmroll, blindroll, selfroll} enum; defaults roll_mode to "roll"
on the wire when None. 4 tests cover envelope shape, validation, and
default.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Tauri commands + register in `lib.rs`

**Files:**
- Modify: `src-tauri/src/bridge/commands.rs` (promote `send_to_source` from private to `pub(crate)`).
- Create: `src-tauri/src/tools/foundry_chat.rs`
- Modify: `src-tauri/src/tools/mod.rs`
- Modify: `src-tauri/src/lib.rs` (only the `invoke_handler!` line list — anti-scope below)

**Anti-scope:** Do NOT change `BridgeConn` itself. Do NOT change any existing `bridge_*` command. The two new commands should be the only additions to `invoke_handler!`. The `send_to_source` change is *visibility-only* — keep its body unchanged.

**Depends on:** Task 3.

**Invariants cited:** ARCHITECTURE.md §4 (Tauri command surface), spec §5 (Tauri command surface — note that the spec was updated to specify the `send_to_source` call path and `BridgeConn` State type).

**Codebase ground truth (verified before writing this task):**
- `BridgeState`-style state is wrapped in a tuple-struct `BridgeConn(Arc<...>)` exposed as `tauri::State<'_, BridgeConn>`. Existing `bridge_*` commands in `src-tauri/src/bridge/commands.rs` show the canonical signature.
- `send_to_source` is a free async fn at `src-tauri/src/bridge/commands.rs:88` with signature `async fn send_to_source(conn: &State<'_, BridgeConn>, kind: SourceKind, text: String) -> Result<(), String>` — takes a `String`, not a `Value`. JSON envelopes must be serialized via `serde_json::to_string(&envelope)` before calling.
- `send_to_source` no-ops on a disconnected source (best-effort), matching `bridge_refresh` semantics.

- [ ] **Step 1: Promote `send_to_source` to `pub(crate)`**

In `src-tauri/src/bridge/commands.rs` line 88, change:

```rust
async fn send_to_source(
```

to:

```rust
pub(crate) async fn send_to_source(
```

Body unchanged.

- [ ] **Step 2: Create `src-tauri/src/tools/foundry_chat.rs` with both commands**

```rust
// Tauri commands for outbound Foundry game.* helpers.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

use tauri::State;

use crate::bridge::{
    commands::send_to_source,
    foundry::{
        actions::game::{build_post_chat_as_actor, build_roll_v5_pool},
        types::{PostChatAsActorInput, RollV5PoolInput},
    },
    types::SourceKind,
    BridgeConn,
};

#[tauri::command]
pub async fn trigger_foundry_roll(
    conn: State<'_, BridgeConn>,
    input: RollV5PoolInput,
) -> Result<(), String> {
    let envelope = build_roll_v5_pool(&input)?;
    let text = serde_json::to_string(&envelope)
        .map_err(|e| format!("foundry/game.roll_v5_pool: serialize: {e}"))?;
    send_to_source(&conn, SourceKind::Foundry, text).await
}

#[tauri::command]
pub async fn post_foundry_chat(
    conn: State<'_, BridgeConn>,
    input: PostChatAsActorInput,
) -> Result<(), String> {
    let envelope = build_post_chat_as_actor(&input)?;
    let text = serde_json::to_string(&envelope)
        .map_err(|e| format!("foundry/game.post_chat_as_actor: serialize: {e}"))?;
    send_to_source(&conn, SourceKind::Foundry, text).await
}
```

**Note on the import paths:** if `BridgeConn` lives at `crate::bridge::BridgeConn` (re-exported from `bridge/mod.rs`), the import path above is correct. Verify with `grep -n "pub use\|pub struct BridgeConn" src-tauri/src/bridge/mod.rs`. If `SourceKind` is re-exported at `crate::bridge::SourceKind`, prefer that over the deeper `crate::bridge::types::SourceKind` path. Adjust to match whichever is the canonical re-export.

- [ ] **Step 3: Add `pub mod foundry_chat;` to `src-tauri/src/tools/mod.rs`**

Insert as a new line in alphabetical order with the existing `pub mod` declarations.

- [ ] **Step 4: Register both commands in `src-tauri/src/lib.rs`'s `invoke_handler!`**

In the `tauri::generate_handler![...]` macro, add at the end of the existing list:

```rust
            tools::foundry_chat::trigger_foundry_roll,
            tools::foundry_chat::post_foundry_chat,
```

Match the indentation and trailing-comma style of the existing entries.

- [ ] **Step 5: Run `cargo check`**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: clean (no errors).

- [ ] **Step 6: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/bridge/commands.rs src-tauri/src/tools/foundry_chat.rs src-tauri/src/tools/mod.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(tools/foundry_chat): add trigger_foundry_roll + post_foundry_chat commands

Two Tauri commands wrap the actions/game.rs builders, serialize the
JSON envelope, and dispatch via the existing send_to_source path
(promoted from private to pub(crate)). Both registered in
generate_handler!. Total command surface: 37 → 39.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Frontend typed wrappers (`src/lib/foundry-chat/api.ts`)

**Files:**
- Create: `src/lib/foundry-chat/api.ts`

**Anti-scope:** Do NOT add a runes store, components, or any consumer wiring (those belong to a future feature plan, not this primitive). Do NOT touch any existing component.

**Depends on:** Task 4.

**Invariants cited:** CLAUDE.md "Never call `invoke(...)` directly from a Svelte component — use the typed wrapper in `src/lib/**/api.ts`", spec §5 (frontend wrapper section).

- [ ] **Step 1: Create the directory**

```bash
mkdir -p src/lib/foundry-chat
```

- [ ] **Step 2: Create `src/lib/foundry-chat/api.ts`**

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

- [ ] **Step 3: Run `npm run check`**

```bash
npm run check
```

Expected: 0 errors. Existing `AdvantageForm.svelte` warnings (6) are documented expected non-regression noise per ARCHITECTURE.md §10 and persist.

- [ ] **Step 4: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/foundry-chat/api.ts
git commit -m "$(cat <<'EOF'
feat(foundry-chat): add triggerFoundryRoll + postFoundryChat typed wrappers

Frontend access path for the FHL Phase 2 game.* helpers. Components
must import these (not invoke() directly) per CLAUDE.md §4. No
consumer yet — Character Tooling Phase 3's roll-source toggle will
be the first.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: JS executors `rollV5Pool` + `postChatAsActor`

**Files:**
- Modify: `vtmtools-bridge/scripts/foundry-actions/game.js` (currently `export const handlers = {};`)

**Anti-scope:** Do NOT touch `bridge.js`, `actor.js`, `index.js`, or `translate.js`. The handler-map dispatch is already wired via FHL Phase 0 — `index.js` already imports `gameHandlers`.

**Depends on:** Task 5 (logical only — JS module is independent of Rust).

**Invariants cited:** spec §7 (JS executor shape), Plan 0 error envelope conventions (handler exceptions become outbound `error` envelopes).

- [ ] **Step 1: Replace the contents of `vtmtools-bridge/scripts/foundry-actions/game.js`**

```js
// Foundry game.* helper executors.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

const MODULE_ID = "vtmtools-bridge";

async function rollV5Pool(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[${MODULE_ID}] game.roll_v5_pool: actor not found: ${msg.actor_id}`);
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  const paths = msg.value_paths ?? [];
  const advancedDice =
    msg.advanced_dice ?? WOD5E.api.getAdvancedDice({ actor });
  const label = msg.flavor ?? deriveFlavorFromPaths(paths);

  if (paths.length === 0) {
    // Rouse-style: zero basic dice + caller-supplied advanced dice. Use the
    // direct Roll API since RollFromDataset cannot represent an empty pool.
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
      selectDialog: false, // never pop the GM dialog from outside Foundry
      advancedDice,
      selectors: msg.selectors ?? [],
    },
    actor,
  });
}

async function postChatAsActor(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(
      `[${MODULE_ID}] game.post_chat_as_actor: actor not found: ${msg.actor_id}`,
    );
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
    .map((p) => p.split(".").slice(-2)[0]) // "skills.brawl.value" → "brawl"
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join(" + ");
}

export const handlers = {
  "game.roll_v5_pool": rollV5Pool,
  "game.post_chat_as_actor": postChatAsActor,
};
```

- [ ] **Step 2: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. (No JS test runner — Foundry-side JS is verified manually in Task 8 smoke.)

- [ ] **Step 3: Commit**

```bash
git add vtmtools-bridge/scripts/foundry-actions/game.js
git commit -m "$(cat <<'EOF'
feat(foundry-actions/game): add rollV5Pool + postChatAsActor executors

Two outbound handlers registered under game.roll_v5_pool +
game.post_chat_as_actor. Empty value_paths routes to WOD5E.api.Roll
(direct) for the rouse-check pattern; non-empty routes to
RollFromDataset. Actor-not-found throws — Plan 0's bridge.js wraps the
exception into an outbound error envelope visible to the desktop.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Bump module version 0.2.0 → 0.3.0

**Files:**
- Modify: `vtmtools-bridge/module.json`

**Anti-scope:** Only change the `version` field. Do NOT modify any other field (`manifest`, `download`, etc. are out of scope for this phase).

**Depends on:** Task 6.

**Invariants cited:** FHL roadmap §3 versioning discipline (additive new wire types → minor bump).

- [ ] **Step 1: Read the existing `vtmtools-bridge/module.json`**

```bash
cat vtmtools-bridge/module.json
```

Locate the `"version": "0.2.0"` line.

- [ ] **Step 2: Change `"version": "0.2.0"` to `"version": "0.3.0"`**

Use a single targeted edit; do not reformat the rest of the file.

- [ ] **Step 3: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 4: Commit**

```bash
git add vtmtools-bridge/module.json
git commit -m "$(cat <<'EOF'
chore(bridge-module): bump to 0.3.0

Adds the game.* umbrella (roll_v5_pool + post_chat_as_actor) over
the existing protocol_version: 1 wire (additive — no protocol bump
needed). Old desktops never send game.* envelopes; new desktops
talking to a 0.2.0 module receive Plan 0's error envelope on the
unknown handler-map miss.

Closes #17

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Final verification gate

**Files:** none — verification only.

**Depends on:** all previous.

- [ ] **Step 1: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. `cargo test` should report 9 new passing tests under `bridge::foundry::actions::game::tests::*`.

- [ ] **Step 2: Manual end-to-end smoke**

Run `npm run tauri dev` with a Foundry world running and the bridge module 0.3.0 installed and connected.

In the desktop dev-tools console:

```js
// Get an actor id from the bridge store (adjust the access path to match
// the project's actual store API — likely bridgeStore.characters or similar):
const actorId = (window.__VTMTOOLS_DEBUG_BRIDGE_STORE__ ?? bridgeStore).characters[0].sourceId;
```

Then test each helper:

**Roll a basic check:**
```js
await window.__TAURI_INTERNALS__.invoke('trigger_foundry_roll', {
  input: {
    actorId,
    valuePaths: ['attributes.strength.value', 'skills.brawl.value'],
    difficulty: 3,
    flavor: 'Smoke test — Strength + Brawl',
  },
});
```
Expected: a Strength + Brawl roll appears in Foundry chat with the right actor as speaker.

**Roll a rouse check (empty pool + 1 hunger die):**
```js
await window.__TAURI_INTERNALS__.invoke('trigger_foundry_roll', {
  input: {
    actorId,
    valuePaths: [],
    difficulty: 0,
    advancedDice: 1,
    flavor: 'Smoke test — Rouse',
  },
});
```
Expected: a single hunger-die roll appears in Foundry chat.

**Post a chat message:**
```js
await window.__TAURI_INTERNALS__.invoke('post_foundry_chat', {
  input: {
    actorId,
    content: '<p>Smoke test message</p>',
    flavor: 'Smoke',
  },
});
```
Expected: a chat message with the actor as speaker, "Smoke" header.

**Error path (actor not found):**
```js
await window.__TAURI_INTERNALS__.invoke('trigger_foundry_roll', {
  input: {
    actorId: 'no-such-actor',
    valuePaths: ['attributes.strength.value'],
    difficulty: 0,
  },
});
```
Expected: the desktop emits a `bridge://foundry/error` event surfacing `actor not found: no-such-actor` (visible in console or via toast if a toast handler is wired).

**Validation error (empty content):**
```js
try {
  await window.__TAURI_INTERNALS__.invoke('post_foundry_chat', {
    input: { actorId, content: '' },
  });
} catch (e) {
  console.log('expected error:', e);
}
```
Expected: the promise rejects with `"foundry/game.post_chat_as_actor: content is required"`.

- [ ] **Step 3: Commit any fixups**

```bash
git status --short
```

If clean, no commit needed. If any small fixes were made during smoke, commit:

```bash
git add -A
git commit -m "$(cat <<'EOF'
chore: FHL Phase 2 verification fixups

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Note: the `Closes #17` footer is on Task 7's commit (the last code commit). Task 8 verification has no `Closes` footer — if the smoke passes clean and there's no fixup commit, the parent #17 closes via Task 7's footer when that PR/commit lands on master.

---

## Self-review checklist

- [x] Spec §1 — both helpers shipped (Tasks 2, 3); rouse covered via empty `value_paths` (Task 2 + Task 6 routing).
- [x] Spec §3 — every file in the file inventory is touched in the right task.
- [x] Spec §4 — wire payloads exact (`type`, `actor_id`, `value_paths`, `difficulty`, `flavor`, `advanced_dice`, `selectors` for roll; `type`, `actor_id`, `content`, `flavor`, `roll_mode` for chat). Field names snake_case on the wire confirmed in Task 2/3 builders.
- [x] Spec §5 — Tauri commands `trigger_foundry_roll` + `post_foundry_chat` (Task 4); typed wrappers `triggerFoundryRoll` + `postFoundryChat` (Task 5).
- [x] Spec §6 — Rust builder shape including the `roll_mode` enum check (Task 3).
- [x] Spec §7 — JS executor including empty-paths Shape A routing + `deriveFlavorFromPaths` helper (Task 6).
- [x] Spec §8 — error surfaces: builder validation `Err`, `BridgeState::send_to` failure, JS executor `throw` on actor-not-found (Plan 0 envelope).
- [x] Spec §9 — module bump 0.2.0 → 0.3.0 (Task 7); pre-0.3.0 module behavior documented in commit message.
- [x] Spec §10 — 9 unit tests across roll_v5_pool (5) + post_chat_as_actor (4); manual smoke procedure (Task 8) covers happy path + error paths.
- [x] Spec §11 — anti-scope respected on every task.
- [x] No placeholders / TBDs / "implement appropriate error handling" hand-waving.
- [x] Every commit-ending task runs `./scripts/verify.sh` first per CLAUDE.md hard rule (memory: `feedback_plans_must_include_verify.md`).
- [x] Type and method names consistent across tasks: `RollV5PoolInput`/`PostChatAsActorInput` (defined Task 1, used 2/3/4); `build_roll_v5_pool`/`build_post_chat_as_actor` (defined 2/3, used 4); `trigger_foundry_roll`/`post_foundry_chat` (defined 4, used 5+8); `triggerFoundryRoll`/`postFoundryChat` (defined 5, used 8).
- [x] Plan/spec NOT committed to git per `project_specs_plans_not_committed.md` — only implementation commits enter the repo.
- [x] `BridgeState::send_to` method-name verification step included in Task 4 (codebase grep guard) since the method name was inferred from convention rather than read directly.
