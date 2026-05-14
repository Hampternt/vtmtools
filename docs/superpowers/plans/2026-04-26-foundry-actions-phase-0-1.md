# Foundry Actions Phase 0 + Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold the Foundry helper library on the bridge layer (categorized helper modules under `actor.*` / `game.*` / `storyteller.*` umbrellas), migrate three existing wire types to dot-namespace, and implement five new actor.* primitives — without changing user-visible behavior.

**Architecture:** Phase 0 introduces handler-map dispatch in `bridge.js` and per-umbrella subdirectories on both Rust (`src-tauri/src/bridge/foundry/actions/`) and JS (`vtmtools-bridge/scripts/foundry-actions/`) sides. Phase 1 adds five actor primitives as typed Rust builders + JS executors, then refactors the existing `apply_dyscrasia` JS executor to compose primitives via direct function calls (A1 strategy). No frontend or Tauri-command changes.

**Tech Stack:** Rust (with `serde_json`, `chrono`), JavaScript (Foundry module, no test framework), Tauri 2 bridge layer.

**Spec:** `docs/superpowers/specs/2026-04-26-foundry-actions-phase-0-1-design.md`

**Roadmap:** `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`

---

## File structure

### New files
- `src-tauri/src/bridge/foundry/actions/mod.rs` — re-exports
- `src-tauri/src/bridge/foundry/actions/actor.rs` — 3 migrated builders + 5 new builders + their tests
- `src-tauri/src/bridge/foundry/actions/game.rs` — empty stub (Phase 2 fills)
- `src-tauri/src/bridge/foundry/actions/storyteller.rs` — empty stub
- `vtmtools-bridge/scripts/foundry-actions/index.js` — flattens umbrella handler exports
- `vtmtools-bridge/scripts/foundry-actions/actor.js` — 8 executors
- `vtmtools-bridge/scripts/foundry-actions/game.js` — empty stub
- `vtmtools-bridge/scripts/foundry-actions/storyteller.js` — empty stub

### Modified files
- `src-tauri/src/bridge/foundry/mod.rs` — `build_set_attribute` calls into `actions::actor::build_*`; existing inline builder code is removed; the dyscrasia `#[cfg(test)]` block moves to `actions/actor.rs`
- `src-tauri/src/bridge/foundry/types.rs` — adds 5 new payload structs (forward-looking; not yet used by any builder)
- `vtmtools-bridge/scripts/bridge.js` — `handleInbound` becomes the handler-map dispatch shell; inline branches removed

### Files explicitly NOT touched
- All Svelte components (`src/**/*.svelte`)
- All `src/lib/**/api.ts` typed wrappers
- `src/store/bridge.svelte.ts` and other frontend stores
- Roll20 source (`src-tauri/src/bridge/roll20/`)
- `BridgeState`, `BridgeSource` trait, Tauri command surface
- `Cargo.toml`, `package.json`, `tauri.conf.json`, lock files
- `vtmtools-bridge/scripts/translate.js`

---

## Task overview

| # | Task | Depends on | Phase |
|---|---|---|---|
| 1 | Scaffold actions/ + foundry-actions/ directories with empty modules | none | 0 |
| 2 | Refactor `bridge.js::handleInbound` to handler-map dispatch (still using OLD wire names) | 1 | 0 |
| 3 | Move Rust builders + tests into `actions/actor.rs` (still using OLD wire names) | 1 | 0 |
| 4 | Rename `update_actor` → `actor.update_field` (Rust + JS pair) + add new test | 2, 3 | 0 |
| 5 | Rename `create_item` → `actor.create_item_simple` (Rust + JS pair) + add new test | 4 | 0 |
| 6 | Rename `apply_dyscrasia` → `actor.apply_dyscrasia` (Rust + JS pair) + update 4 existing tests | 5 | 0 |
| 7 | Add 5 new payload structs to `types.rs` | 1 | 1 |
| 8 | Implement `actor.append_private_notes_line` primitive (Rust builder + test + JS executor) | 3, 7 | 1 |
| 9 | Implement `actor.replace_private_notes` primitive | 3, 7 | 1 |
| 10 | Implement `actor.create_feature` primitive (with featuretype validation) | 3, 7 | 1 |
| 11 | Implement `actor.delete_items_by_prefix` primitive (with empty-prefix validation) | 3, 7 | 1 |
| 12 | Implement `actor.delete_item_by_id` primitive | 3, 7 | 1 |
| 13 | Refactor `applyDyscrasia` JS executor to compose primitives (A1) | 6, 8, 10, 11 | 1 |
| 14 | Final verification gate (verify.sh + git grep + manual E2E) | all | — |

Tasks 8-12 are independent of each other (each adds a new builder + test + executor in non-overlapping regions of `actions/actor.rs`, `actor.js`). Subagent-driven execution can dispatch them in parallel after Tasks 3 and 7 complete.

---

## Task 1: Scaffold actions/ and foundry-actions/ directories

**Files:**
- Create: `src-tauri/src/bridge/foundry/actions/mod.rs`
- Create: `src-tauri/src/bridge/foundry/actions/actor.rs`
- Create: `src-tauri/src/bridge/foundry/actions/game.rs`
- Create: `src-tauri/src/bridge/foundry/actions/storyteller.rs`
- Create: `vtmtools-bridge/scripts/foundry-actions/index.js`
- Create: `vtmtools-bridge/scripts/foundry-actions/actor.js`
- Create: `vtmtools-bridge/scripts/foundry-actions/game.js`
- Create: `vtmtools-bridge/scripts/foundry-actions/storyteller.js`
- Modify: `src-tauri/src/bridge/foundry/mod.rs:1-2` (add `pub mod actions;`)

**Anti-scope:** Do NOT modify `bridge.js`, `bridge/foundry/mod.rs::build_set_attribute`, or any other dispatch logic in this task. Stub files only.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §5 (only `bridge/*` talks to WebSocket).

- [ ] **Step 1: Create `src-tauri/src/bridge/foundry/actions/mod.rs`**

```rust
pub mod actor;
pub mod game;
pub mod storyteller;
```

- [ ] **Step 2: Create `src-tauri/src/bridge/foundry/actions/actor.rs` (empty stub)**

```rust
// Foundry actor.* helper builders.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md
// for the umbrella organization and naming conventions.
```

- [ ] **Step 3: Create `src-tauri/src/bridge/foundry/actions/game.rs` (empty stub)**

```rust
// Foundry game.* helper builders. Empty in v1; Phase 2 fills.
```

- [ ] **Step 4: Create `src-tauri/src/bridge/foundry/actions/storyteller.rs` (empty stub)**

```rust
// Foundry storyteller.* helper builders. Reserved umbrella; no helpers in v1.
```

- [ ] **Step 5: Add `pub mod actions;` to `src-tauri/src/bridge/foundry/mod.rs`**

Edit `src-tauri/src/bridge/foundry/mod.rs:1-2` from:

```rust
pub mod translate;
pub mod types;
```

to:

```rust
pub mod actions;
pub mod translate;
pub mod types;
```

- [ ] **Step 6: Create `vtmtools-bridge/scripts/foundry-actions/index.js`**

```js
// Flattens per-umbrella handler exports into one map for bridge.js::handleInbound.
import { handlers as actorHandlers } from "./actor.js";
import { handlers as gameHandlers } from "./game.js";
import { handlers as storytellerHandlers } from "./storyteller.js";

export const handlers = {
  ...actorHandlers,
  ...gameHandlers,
  ...storytellerHandlers,
};
```

- [ ] **Step 7: Create `vtmtools-bridge/scripts/foundry-actions/actor.js` (empty stub)**

```js
// Foundry actor.* helper executors.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md.
export const handlers = {};
```

- [ ] **Step 8: Create `vtmtools-bridge/scripts/foundry-actions/game.js` (empty stub)**

```js
// Foundry game.* helper executors. Empty in v1.
export const handlers = {};
```

- [ ] **Step 9: Create `vtmtools-bridge/scripts/foundry-actions/storyteller.js` (empty stub)**

```js
// Foundry storyteller.* helper executors. Reserved umbrella; no helpers in v1.
export const handlers = {};
```

- [ ] **Step 10: Verify Rust compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean compile (no errors, no new warnings).

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/ src-tauri/src/bridge/foundry/mod.rs vtmtools-bridge/scripts/foundry-actions/
git commit -m "$(cat <<'EOF'
chore(bridge/foundry): scaffold actions/ and foundry-actions/ umbrella dirs

Empty stubs for actor/game/storyteller umbrellas on both Rust and JS
sides of the Foundry bridge. No behavior change — bridge.js does not
yet import from foundry-actions/index.js.

See docs/superpowers/specs/2026-04-26-foundry-actions-phase-0-1-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Refactor `bridge.js::handleInbound` to handler-map dispatch (old names)

**Files:**
- Modify: `vtmtools-bridge/scripts/bridge.js:73-147` (handleInbound rewrite)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (add 3 executors under OLD wire names)

**Anti-scope:** Do NOT change wire-type strings in this task. Do NOT touch any Rust files. Do NOT add new primitives.

**Depends on:** Task 1.

**Invariants cited:** Existing E2E flows (resonance + dyscrasia apply) must still work after this task — verified manually.

- [ ] **Step 1: Rewrite `vtmtools-bridge/scripts/foundry-actions/actor.js` with the 3 inline executors moved out of `bridge.js`**

Replace the entire file contents with:

```js
// Foundry actor.* helper executors.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md.

const wireExecutor = (fn) => async (msg) => {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[vtmtools-bridge] actor not found: ${msg.actor_id}`);
    return;
  }
  await fn(actor, msg);
};

async function updateField(actor, msg) {
  await actor.update({ [msg.path]: msg.value });
}

async function createItemSimple(actor, msg) {
  if (msg.replace_existing) {
    const existing = actor.items.filter((i) => i.type === msg.item_type);
    if (existing.length) {
      await actor.deleteEmbeddedDocuments(
        "Item",
        existing.map((i) => i.id),
      );
    }
  }
  await actor.createEmbeddedDocuments("Item", [
    { type: msg.item_type, name: msg.item_name },
  ]);
}

async function applyDyscrasia(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) return;

  const existing = actor.items.filter(
    (i) =>
      i.type === "feature" &&
      i.system?.featuretype === "merit" &&
      typeof i.name === "string" &&
      i.name.startsWith("Dyscrasia: "),
  );
  if (msg.replace_existing && existing.length) {
    await actor.deleteEmbeddedDocuments(
      "Item",
      existing.map((i) => i.id),
    );
  }

  await actor.createEmbeddedDocuments("Item", [
    {
      type: "feature",
      name: `Dyscrasia: ${msg.dyscrasia_name}`,
      system: {
        featuretype: "merit",
        description: msg.merit_description_html,
        points: 0,
      },
    },
  ]);

  const current = actor.system?.privatenotes ?? "";
  const next =
    current.trim() === ""
      ? msg.notes_line
      : `${current}\n${msg.notes_line}`;
  await actor.update({ "system.privatenotes": next });
}

export const handlers = {
  // OLD wire names — renamed in Tasks 4-6.
  update_actor: wireExecutor(updateField),
  create_item: wireExecutor(createItemSimple),
  apply_dyscrasia: applyDyscrasia,
};
```

- [ ] **Step 2: Rewrite `vtmtools-bridge/scripts/bridge.js::handleInbound` to use the handler map**

Replace `vtmtools-bridge/scripts/bridge.js:73-147` (the entire `handleInbound` function and its inline branches) with:

```js
import { handlers } from "./foundry-actions/index.js";

async function handleInbound(msg) {
  if (msg.type === "refresh") {
    pushAllActors();
    return;
  }
  const handler = handlers[msg.type];
  if (!handler) {
    console.warn(`[${MODULE_ID}] unknown inbound type:`, msg.type);
    return;
  }
  try {
    await handler(msg);
  } catch (err) {
    console.error(`[${MODULE_ID}] handler ${msg.type} threw:`, err);
    ui.notifications?.error(`vtmtools: ${msg.type} failed — ${err.message}`);
  }
}
```

The `import { handlers }` line goes at the top of the file alongside the existing `import { actorToWire, hookActorChanges }` line.

Note: `refresh` stays inline because it doesn't operate on a single actor — it's a connection-level command. All actor-scoped operations move into the handler map.

- [ ] **Step 3: Manual E2E — resonance apply**

1. Start vtmtools (`npm run tauri dev`).
2. Start a Foundry world with `vtmtools-bridge` module installed and active.
3. GM logs in.
4. In vtmtools Resonance tool: roll a resonance, click "Apply to character" on a Foundry-source character.
5. Verify: a `resonance` Item appears on the actor sheet matching the rolled type. Foundry F12 console shows no errors.

- [ ] **Step 4: Manual E2E — dyscrasia apply**

1. With same setup as Step 3.
2. Roll a dyscrasia, confirm, click "Apply to character".
3. Verify: a `Dyscrasia: <name>` merit Item appears on the actor sheet. The character's private notes contains a new line of the form `[YYYY-MM-DD HH:MM] Acquired Dyscrasia: <name> (<resonance>)`. Foundry F12 console shows no errors.

- [ ] **Step 5: Verify Rust still compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean compile.

- [ ] **Step 6: Commit**

```bash
git add vtmtools-bridge/scripts/bridge.js vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
refactor(bridge/foundry): handler-map dispatch in bridge.js

Move the three inline inbound handlers (update_actor, create_item,
apply_dyscrasia) out of bridge.js into foundry-actions/actor.js,
registered under their existing wire-type names. bridge.js becomes a
thin dispatch shell with uniform try/catch error surfacing.

Wire-type strings are unchanged; renames happen in subsequent tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Move Rust builders + tests into `actions/actor.rs` (old names)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/mod.rs` (extract builder bodies; keep `build_set_attribute` calling them; remove tests)
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (add 3 builders + 4 migrated tests)

**Anti-scope:** Do NOT change wire-type strings in this task. Do NOT touch JS. Do NOT add new builders.

**Depends on:** Task 1.

**Invariants cited:** ARCHITECTURE.md §10 (existing tests must continue to pass).

- [ ] **Step 1: Replace `src-tauri/src/bridge/foundry/actions/actor.rs` with the migrated builders + tests**

Replace the entire file contents with:

```rust
// Foundry actor.* helper builders.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md.

use serde_json::{json, Value};

use crate::bridge::foundry::types::ApplyDyscrasiaPayload;

pub fn build_update_field(actor_id: &str, path: &str, value: Value) -> Value {
    json!({
        "type": "update_actor",
        "actor_id": actor_id,
        "path": path,
        "value": value,
    })
}

pub fn build_create_item_simple(actor_id: &str, item_type: &str, item_name: &str) -> Value {
    json!({
        "type": "create_item",
        "actor_id": actor_id,
        "item_type": item_type,
        "item_name": item_name,
        "replace_existing": true,
    })
}

pub fn build_apply_dyscrasia(actor_id: &str, payload: &str) -> Result<Value, String> {
    let payload: ApplyDyscrasiaPayload = serde_json::from_str(payload)
        .map_err(|e| format!("foundry/apply_dyscrasia: invalid payload: {e}"))?;
    let merit_description_html =
        render_merit_description(&payload.description, &payload.bonus);
    let applied_at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let notes_line = format!(
        "[{applied_at}] Acquired Dyscrasia: {} ({})",
        payload.dyscrasia_name, payload.resonance_type
    );
    Ok(json!({
        "type": "apply_dyscrasia",
        "actor_id": actor_id,
        "dyscrasia_name": payload.dyscrasia_name,
        "resonance_type": payload.resonance_type,
        "merit_description_html": merit_description_html,
        "notes_line": notes_line,
        "replace_existing": true,
    }))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_merit_description(description: &str, bonus: &str) -> String {
    let desc_p = format!("<p>{}</p>", html_escape(description));
    if bonus.trim().is_empty() {
        desc_p
    } else {
        format!(
            "{desc_p}<p><em>Mechanical bonus:</em> {}</p>",
            html_escape(bonus)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload_json(name: &str, res: &str, desc: &str, bonus: &str) -> String {
        json!({
            "dyscrasia_name": name,
            "resonance_type": res,
            "description": desc,
            "bonus": bonus,
        })
        .to_string()
    }

    #[test]
    fn dyscrasia_happy_path_shape() {
        let payload = payload_json("Wax", "Choleric", "Crystallized blood.", "+1 Composure");
        let out = build_apply_dyscrasia("actor-abc", &payload).expect("happy path");
        assert_eq!(out["type"], "apply_dyscrasia");
        assert_eq!(out["actor_id"], "actor-abc");
        assert_eq!(out["dyscrasia_name"], "Wax");
        assert_eq!(out["resonance_type"], "Choleric");
        assert_eq!(out["replace_existing"], true);
        let html = out["merit_description_html"].as_str().unwrap();
        assert!(html.contains("<p>Crystallized blood.</p>"));
        assert!(html.contains("<p><em>Mechanical bonus:</em> +1 Composure</p>"));
        let line = out["notes_line"].as_str().unwrap();
        let re = regex::Regex::new(
            r"^\[\d{4}-\d{2}-\d{2} \d{2}:\d{2}\] Acquired Dyscrasia: Wax \(Choleric\)$",
        )
        .unwrap();
        assert!(re.is_match(line), "notes_line did not match: {line}");
    }

    #[test]
    fn dyscrasia_empty_bonus_omits_bonus_block() {
        let payload = payload_json("Custom", "Sanguine", "Some description.", "");
        let out = build_apply_dyscrasia("a", &payload).expect("empty bonus is valid");
        let html = out["merit_description_html"].as_str().unwrap();
        assert_eq!(html, "<p>Some description.</p>");
        assert!(!html.contains("Mechanical bonus"));
    }

    #[test]
    fn dyscrasia_html_escapes_dangerous_chars() {
        let payload = payload_json(
            "Test",
            "Phlegmatic",
            "<script>alert(\"x\")</script>",
            "& > <",
        );
        let out = build_apply_dyscrasia("a", &payload).expect("html-escape happy path");
        let html = out["merit_description_html"].as_str().unwrap();
        assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"));
        assert!(html.contains("&amp; &gt; &lt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn dyscrasia_malformed_payload_returns_err() {
        let result = build_apply_dyscrasia("a", "{not valid json");
        assert!(result.is_err(), "malformed payload must return Err, not panic");
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("foundry/apply_dyscrasia: invalid payload:"),
            "error message must use module-prefixed convention, got: {msg}"
        );
    }
}
```

- [ ] **Step 2: Rewrite `src-tauri/src/bridge/foundry/mod.rs` to delegate to the new builders and remove the `tests` module + helper functions**

Replace the entire `src-tauri/src/bridge/foundry/mod.rs` file contents with:

```rust
pub mod actions;
pub mod translate;
pub mod types;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::bridge::foundry::actions::actor;
use crate::bridge::foundry::types::FoundryInbound;
use crate::bridge::source::BridgeSource;
use crate::bridge::types::CanonicalCharacter;

/// Stateless adapter for the FoundryVTT WoD5e module. Translates
/// Foundry actor data into the canonical bridge shape and builds
/// outbound messages the module knows how to apply via actor.update().
pub struct FoundrySource;

#[async_trait]
impl BridgeSource for FoundrySource {
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
        let parsed: FoundryInbound = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        let actors = match parsed {
            FoundryInbound::Actors { actors } => actors,
            FoundryInbound::ActorUpdate { actor } => vec![actor],
            FoundryInbound::Hello => return Ok(vec![]),
        };
        Ok(actors.iter().map(translate::to_canonical).collect())
    }

    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String> {
        match name {
            "resonance" => Ok(actor::build_create_item_simple(source_id, "resonance", value)),
            "dyscrasia" => actor::build_apply_dyscrasia(source_id, value),
            _ => {
                let path = canonical_to_path(name);
                Ok(actor::build_update_field(source_id, &path, parse_value(value)))
            }
        }
    }

    fn build_refresh(&self) -> Value {
        json!({ "type": "refresh" })
    }
}

fn canonical_to_path(name: &str) -> String {
    match name {
        "hunger" => "system.hunger.value",
        "humanity" => "system.humanity.value",
        "humanity_stains" => "system.humanity.stains",
        "blood_potency" => "system.blood.potency",
        "health_superficial" => "system.health.superficial",
        "health_aggravated" => "system.health.aggravated",
        "willpower_superficial" => "system.willpower.superficial",
        "willpower_aggravated" => "system.willpower.aggravated",
        other if other.starts_with("system.") => other,
        other => other,
    }
    .to_string()
}

fn parse_value(s: &str) -> Value {
    if let Ok(n) = s.parse::<i64>() {
        Value::from(n)
    } else if let Ok(f) = s.parse::<f64>() {
        Value::from(f)
    } else if s == "true" {
        Value::from(true)
    } else if s == "false" {
        Value::from(false)
    } else {
        Value::from(s)
    }
}
```

Note that `html_escape` and `render_merit_description` are removed from `mod.rs` (they live in `actions/actor.rs` now). The `#[cfg(test)] mod tests` block at the bottom is removed entirely (those 4 tests live in `actions/actor.rs::tests` now).

- [ ] **Step 3: Run cargo tests to verify the migrated tests still pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry`
Expected: 4 tests pass — `dyscrasia_happy_path_shape`, `dyscrasia_empty_bonus_omits_bonus_block`, `dyscrasia_html_escapes_dangerous_chars`, `dyscrasia_malformed_payload_returns_err`. All in `bridge::foundry::actions::actor::tests`.

- [ ] **Step 4: Run full verify.sh**

Run: `./scripts/verify.sh`
Expected: green (npm check + cargo test + npm build all pass).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/bridge/foundry/mod.rs src-tauri/src/bridge/foundry/actions/actor.rs
git commit -m "$(cat <<'EOF'
refactor(bridge/foundry): extract builders into actions/actor.rs

Move build_update_field, build_create_item_simple, and
build_apply_dyscrasia from foundry/mod.rs into the new
actions/actor.rs module. The 4 dyscrasia unit tests move with them.
foundry/mod.rs::build_set_attribute now delegates to the action
builders.

Wire-type strings are unchanged; renames happen in subsequent tasks.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Rename `update_actor` → `actor.update_field` (Rust + JS pair)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs:5-12` (build_update_field — change wire type string)
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs::tests` (add new test for update_field)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (rename handler key)

**Anti-scope:** Do NOT touch any other builder or executor in this task. Single rename only.

**Depends on:** Tasks 2, 3.

**Invariants cited:** ARCHITECTURE.md §10 (verify.sh must pass).

- [ ] **Step 1: Write the failing test for the new wire type**

Add to the `#[cfg(test)] mod tests` block in `src-tauri/src/bridge/foundry/actions/actor.rs`:

```rust
    #[test]
    fn update_field_shape() {
        let out = build_update_field("actor-xyz", "system.hunger.value", json!(3));
        assert_eq!(out["type"], "actor.update_field");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["path"], "system.hunger.value");
        assert_eq!(out["value"], 3);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::update_field_shape`
Expected: FAIL — assertion `out["type"] == "actor.update_field"` fails (current value is `"update_actor"`).

- [ ] **Step 3: Update `build_update_field` to emit the new wire type**

In `src-tauri/src/bridge/foundry/actions/actor.rs`, change `build_update_field`:

```rust
pub fn build_update_field(actor_id: &str, path: &str, value: Value) -> Value {
    json!({
        "type": "actor.update_field",
        "actor_id": actor_id,
        "path": path,
        "value": value,
    })
}
```

(Only the `"type"` string changes from `"update_actor"` to `"actor.update_field"`.)

- [ ] **Step 4: Update `vtmtools-bridge/scripts/foundry-actions/actor.js` handler key**

In the `handlers` export at the bottom of `actor.js`, rename the key:

```js
export const handlers = {
  "actor.update_field": wireExecutor(updateField),  // was: update_actor
  create_item: wireExecutor(createItemSimple),
  apply_dyscrasia: applyDyscrasia,
};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::update_field_shape`
Expected: PASS.

- [ ] **Step 6: Manual E2E — update_field**

1. Start vtmtools + Foundry as in Task 2 Step 3.
2. Pick a Foundry-source character. From any vtmtools UI that calls `bridge_set_attribute` with a non-resonance/non-dyscrasia name (e.g., the bridge store's character refresh + any inline attribute edit if available — if no such UI exists, this step is informational only and the test in Step 5 is sufficient).
3. Verify: actor field updates on the Foundry sheet.

If no UI invokes `update_field` directly in this session, document this and rely on the Rust unit test as the verification — there's no path to break in the JS executor without also breaking the Rust dispatch, which the test covers.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
refactor(bridge/foundry): rename update_actor → actor.update_field

Adopt the dot-namespaced wire convention from the helper library
roadmap. Rust builder emits the new type string; JS executor handler
key matches; new unit test asserts the wire shape.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Rename `create_item` → `actor.create_item_simple` (Rust + JS pair)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (build_create_item_simple wire type + new test)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (rename handler key)

**Anti-scope:** Do NOT touch any other builder or executor. Single rename only.

**Depends on:** Task 4.

**Invariants cited:** ARCHITECTURE.md §10 (existing E2E flow for resonance must still work).

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests` in `actor.rs`:

```rust
    #[test]
    fn create_item_simple_shape() {
        let out = build_create_item_simple("actor-xyz", "resonance", "Choleric");
        assert_eq!(out["type"], "actor.create_item_simple");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["item_type"], "resonance");
        assert_eq!(out["item_name"], "Choleric");
        assert_eq!(out["replace_existing"], true);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::create_item_simple_shape`
Expected: FAIL.

- [ ] **Step 3: Update `build_create_item_simple` wire type**

```rust
pub fn build_create_item_simple(actor_id: &str, item_type: &str, item_name: &str) -> Value {
    json!({
        "type": "actor.create_item_simple",
        "actor_id": actor_id,
        "item_type": item_type,
        "item_name": item_name,
        "replace_existing": true,
    })
}
```

- [ ] **Step 4: Update JS handler key**

In `actor.js` handlers export:

```js
export const handlers = {
  "actor.update_field": wireExecutor(updateField),
  "actor.create_item_simple": wireExecutor(createItemSimple),  // was: create_item
  apply_dyscrasia: applyDyscrasia,
};
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::create_item_simple_shape`
Expected: PASS.

- [ ] **Step 6: Manual E2E — resonance apply**

Repeat Task 2 Step 3: roll resonance, click Apply, verify resonance Item appears on actor.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
refactor(bridge/foundry): rename create_item → actor.create_item_simple

Continues the dot-namespaced wire migration. Rust builder + JS handler
key + new unit test all updated together.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Rename `apply_dyscrasia` → `actor.apply_dyscrasia` (Rust + JS pair) + update existing tests

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (build_apply_dyscrasia wire type + error prefix + 4 existing test assertions)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (rename handler key)

**Anti-scope:** Do NOT refactor `applyDyscrasia` JS executor to compose primitives in this task — that's Task 13. Single rename + test update only.

**Depends on:** Task 5.

**Invariants cited:** ARCHITECTURE.md §10 (existing E2E flow for dyscrasia must still work). ARCHITECTURE.md §7 (error prefix convention).

- [ ] **Step 1: Update `build_apply_dyscrasia` wire type and error prefix**

In `src-tauri/src/bridge/foundry/actions/actor.rs`:

```rust
pub fn build_apply_dyscrasia(actor_id: &str, payload: &str) -> Result<Value, String> {
    let payload: ApplyDyscrasiaPayload = serde_json::from_str(payload)
        .map_err(|e| format!("foundry/actor.apply_dyscrasia: invalid payload: {e}"))?;
    let merit_description_html =
        render_merit_description(&payload.description, &payload.bonus);
    let applied_at = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
    let notes_line = format!(
        "[{applied_at}] Acquired Dyscrasia: {} ({})",
        payload.dyscrasia_name, payload.resonance_type
    );
    Ok(json!({
        "type": "actor.apply_dyscrasia",
        "actor_id": actor_id,
        "dyscrasia_name": payload.dyscrasia_name,
        "resonance_type": payload.resonance_type,
        "merit_description_html": merit_description_html,
        "notes_line": notes_line,
        "replace_existing": true,
    }))
}
```

Two changes: `"type"` field is now `"actor.apply_dyscrasia"`; error prefix is now `"foundry/actor.apply_dyscrasia: ..."`.

- [ ] **Step 2: Update the 4 existing tests to assert new strings**

In the `#[cfg(test)] mod tests` block, update the affected assertions:

In `dyscrasia_happy_path_shape`:

```rust
        assert_eq!(out["type"], "actor.apply_dyscrasia");
```

(was `"apply_dyscrasia"`)

In `dyscrasia_malformed_payload_returns_err`:

```rust
        assert!(
            msg.starts_with("foundry/actor.apply_dyscrasia: invalid payload:"),
            "error message must use module-prefixed convention, got: {msg}"
        );
```

(was `"foundry/apply_dyscrasia: invalid payload:"`)

The other two tests (`dyscrasia_empty_bonus_omits_bonus_block`, `dyscrasia_html_escapes_dangerous_chars`) don't assert on the wire type or error message; no changes needed.

- [ ] **Step 3: Update JS handler key**

In `actor.js` handlers export:

```js
export const handlers = {
  "actor.update_field": wireExecutor(updateField),
  "actor.create_item_simple": wireExecutor(createItemSimple),
  "actor.apply_dyscrasia": applyDyscrasia,  // was: apply_dyscrasia
};
```

- [ ] **Step 4: Run all dyscrasia tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests`
Expected: 6 tests pass (4 dyscrasia + 1 update_field + 1 create_item_simple).

- [ ] **Step 5: Manual E2E — dyscrasia apply**

Repeat Task 2 Step 4: roll dyscrasia, apply, verify merit Item + private notes line appear correctly.

- [ ] **Step 6: Verify rename completeness**

Run: `git grep -nE 'update_actor|create_item|apply_dyscrasia' -- ':!docs/'`
Expected: only the new dot-namespaced strings (`actor.update_field`, `actor.create_item_simple`, `actor.apply_dyscrasia`) appear in code and inline comments. No bare `update_actor` / `create_item` / `apply_dyscrasia` strings remain (excluding `docs/` because spec files reference old names historically).

- [ ] **Step 7: Commit (Phase 0 complete after this commit)**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
refactor(bridge/foundry): rename apply_dyscrasia → actor.apply_dyscrasia

Final wire-rename of Phase 0. Rust builder, error prefix, JS handler
key, and 4 existing dyscrasia unit tests all updated together.

Phase 0 (wire-protocol scaffolding) complete: handler-map dispatch
landed in Task 2; builders extracted to actions/actor.rs in Task 3;
all three wire types renamed in Tasks 4-6.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Add 5 new payload structs to `types.rs`

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs` (append 5 struct definitions)

**Anti-scope:** Do NOT change the existing `FoundryInbound`, `FoundryActor`, or `ApplyDyscrasiaPayload` types. Append-only.

**Depends on:** Task 1.

**Invariants cited:** ARCHITECTURE.md §2 (Bridge domain types).

- [ ] **Step 1: Append the 5 payload structs to `src-tauri/src/bridge/foundry/types.rs`**

At the bottom of the file (after `ApplyDyscrasiaPayload`), add:

```rust
/// Payload for actor.append_private_notes_line wire message.
/// Used at feature-time when a frontend tool wants to append a notes line.
#[derive(Debug, Deserialize)]
pub struct AppendPrivateNotesLinePayload {
    pub line: String,
}

/// Payload for actor.replace_private_notes wire message.
#[derive(Debug, Deserialize)]
pub struct ReplacePrivateNotesPayload {
    pub full_text: String,
}

/// Payload for actor.create_feature wire message.
/// `featuretype` must be one of "merit", "flaw", "background", "boon".
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
#[derive(Debug, Deserialize)]
pub struct DeleteItemsByPrefixPayload {
    pub item_type: String,
    pub featuretype: Option<String>,
    pub name_prefix: String,
}

/// Payload for actor.delete_item_by_id wire message.
#[derive(Debug, Deserialize)]
pub struct DeleteItemByIdPayload {
    pub item_id: String,
}
```

- [ ] **Step 2: Verify compile (the new structs are unused — `#[allow(dead_code)]` may be needed if compiler warns)**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean compile. If "struct never used" warnings appear, add `#[allow(dead_code)]` to each new struct (these are forward-looking scaffolding; first consumer materializes at feature-time).

If warnings appear, the struct definitions become:

```rust
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct AppendPrivateNotesLinePayload {
    pub line: String,
}
```

(repeat for all 5).

- [ ] **Step 3: Run cargo test to confirm no regressions**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bridge/foundry/types.rs
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add payload structs for actor.* primitives

Forward-looking scaffolding for the 5 new actor.* primitives landing
in Tasks 8-12. Structs are not yet consumed (no caller deserializes
them); they document the intended wire shape and will be used when
the first feature consumes a primitive via JSON-encoded payload.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Implement `actor.append_private_notes_line` primitive

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (add builder + test)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (add executor + handler entry)

**Anti-scope:** Do NOT modify `applyDyscrasia` to use this primitive (that's Task 13). Add primitive standalone.

**Depends on:** Task 3 (actions/actor.rs exists), Task 7 (payload struct exists).

**Invariants cited:** ARCHITECTURE.md §7 (error prefix convention).

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests` in `actor.rs`:

```rust
    #[test]
    fn append_private_notes_line_shape() {
        let out = build_append_private_notes_line("actor-xyz", "Hello world");
        assert_eq!(out["type"], "actor.append_private_notes_line");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["line"], "Hello world");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::append_private_notes_line_shape`
Expected: FAIL — `build_append_private_notes_line` not defined.

- [ ] **Step 3: Implement the builder**

Add to `src-tauri/src/bridge/foundry/actions/actor.rs` (after `build_apply_dyscrasia`):

```rust
pub fn build_append_private_notes_line(actor_id: &str, line: &str) -> Value {
    json!({
        "type": "actor.append_private_notes_line",
        "actor_id": actor_id,
        "line": line,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::append_private_notes_line_shape`
Expected: PASS.

- [ ] **Step 5: Add JS executor + handler entry**

In `vtmtools-bridge/scripts/foundry-actions/actor.js`, add the executor function (above the `handlers` export) and the handler entry.

Add this function (place between existing `createItemSimple` and `applyDyscrasia` for readability):

```js
async function appendPrivateNotesLine(actor, msg) {
  const current = actor.system?.privatenotes ?? "";
  const next =
    current.trim() === "" ? msg.line : `${current}\n${msg.line}`;
  await actor.update({ "system.privatenotes": next });
}
```

Add this entry to the `handlers` export:

```js
export const handlers = {
  "actor.update_field": wireExecutor(updateField),
  "actor.create_item_simple": wireExecutor(createItemSimple),
  "actor.append_private_notes_line": wireExecutor(appendPrivateNotesLine),
  "actor.apply_dyscrasia": applyDyscrasia,
};
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add actor.append_private_notes_line primitive

Library primitive for appending a single line to actor.system.privatenotes.
Empty notes get the bare line; existing notes get '\n<line>' appended.
Not idempotent — caller dedups if needed.

No frontend caller yet; primitive will be exercised when applyDyscrasia
is refactored to compose primitives in Task 13.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: Implement `actor.replace_private_notes` primitive

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (add builder + test)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (add executor + handler entry)

**Anti-scope:** Single primitive only.

**Depends on:** Task 3, Task 7.

**Invariants cited:** ARCHITECTURE.md §7.

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn replace_private_notes_shape() {
        let out = build_replace_private_notes("actor-xyz", "All new notes");
        assert_eq!(out["type"], "actor.replace_private_notes");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["full_text"], "All new notes");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::replace_private_notes_shape`
Expected: FAIL.

- [ ] **Step 3: Implement the builder**

Add to `actor.rs`:

```rust
pub fn build_replace_private_notes(actor_id: &str, full_text: &str) -> Value {
    json!({
        "type": "actor.replace_private_notes",
        "actor_id": actor_id,
        "full_text": full_text,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::replace_private_notes_shape`
Expected: PASS.

- [ ] **Step 5: Add JS executor + handler entry**

Add executor function to `actor.js`:

```js
async function replacePrivateNotes(actor, msg) {
  await actor.update({ "system.privatenotes": msg.full_text });
}
```

Add to `handlers` export:

```js
  "actor.replace_private_notes": wireExecutor(replacePrivateNotes),
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add actor.replace_private_notes primitive

Library primitive for overwriting actor.system.privatenotes entirely.
Idempotency-safe.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Implement `actor.create_feature` primitive (with featuretype validation)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (add builder + happy-path + error tests)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (add executor + handler entry)

**Anti-scope:** Single primitive only.

**Depends on:** Task 3, Task 7.

**Invariants cited:** ARCHITECTURE.md §7 (error prefix). Spec §2 (featuretype must be one of merit/flaw/background/boon).

- [ ] **Step 1: Write the failing happy-path test**

Add to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn create_feature_happy_path_shape() {
        let out = build_create_feature("actor-xyz", "merit", "Iron Will", "Description.", 2)
            .expect("merit is a valid featuretype");
        assert_eq!(out["type"], "actor.create_feature");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["featuretype"], "merit");
        assert_eq!(out["name"], "Iron Will");
        assert_eq!(out["description"], "Description.");
        assert_eq!(out["points"], 2);
    }

    #[test]
    fn create_feature_invalid_featuretype_returns_err() {
        let result = build_create_feature("a", "discipline", "X", "y", 1);
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("foundry/actor.create_feature: invalid featuretype:"),
            "error must use module-prefixed convention, got: {msg}"
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::create_feature`
Expected: FAIL — function not defined.

- [ ] **Step 3: Implement the builder**

Add to `actor.rs`:

```rust
pub fn build_create_feature(
    actor_id: &str,
    featuretype: &str,
    name: &str,
    description: &str,
    points: i32,
) -> Result<Value, String> {
    match featuretype {
        "merit" | "flaw" | "background" | "boon" => {}
        other => {
            return Err(format!(
                "foundry/actor.create_feature: invalid featuretype: {other}"
            ));
        }
    }
    Ok(json!({
        "type": "actor.create_feature",
        "actor_id": actor_id,
        "featuretype": featuretype,
        "name": name,
        "description": description,
        "points": points,
    }))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::create_feature`
Expected: 2 tests PASS.

- [ ] **Step 5: Add JS executor + handler entry**

Add to `actor.js`:

```js
async function createFeature(actor, msg) {
  await actor.createEmbeddedDocuments("Item", [
    {
      type: "feature",
      name: msg.name,
      system: {
        featuretype: msg.featuretype,
        description: msg.description,
        points: msg.points,
      },
    },
  ]);
}
```

Add to `handlers` export:

```js
  "actor.create_feature": wireExecutor(createFeature),
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add actor.create_feature primitive

Library primitive for creating feature-type Items (merit/flaw/background/boon).
Builder validates featuretype against the allowed set; invalid values
return Err with module-prefixed error.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: Implement `actor.delete_items_by_prefix` primitive (with empty-prefix validation)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (add builder + happy-path + error tests)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (add executor + handler entry)

**Anti-scope:** Single primitive only.

**Depends on:** Task 3, Task 7.

**Invariants cited:** ARCHITECTURE.md §7. Spec §2 (empty name_prefix returns Err to prevent accidental match-all).

- [ ] **Step 1: Write the failing tests**

Add to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn delete_items_by_prefix_with_featuretype_shape() {
        let out = build_delete_items_by_prefix("actor-xyz", "feature", Some("merit"), "Dyscrasia: ")
            .expect("non-empty prefix is valid");
        assert_eq!(out["type"], "actor.delete_items_by_prefix");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["item_type"], "feature");
        assert_eq!(out["featuretype"], "merit");
        assert_eq!(out["name_prefix"], "Dyscrasia: ");
    }

    #[test]
    fn delete_items_by_prefix_without_featuretype_omits_field() {
        let out = build_delete_items_by_prefix("a", "weapon", None, "Stake")
            .expect("featuretype is optional");
        assert_eq!(out["type"], "actor.delete_items_by_prefix");
        assert!(out["featuretype"].is_null(), "featuretype must serialize as null when absent");
    }

    #[test]
    fn delete_items_by_prefix_empty_prefix_returns_err() {
        let result = build_delete_items_by_prefix("a", "feature", None, "");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("foundry/actor.delete_items_by_prefix: empty name_prefix"),
            "error must use module-prefixed convention, got: {msg}"
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::delete_items_by_prefix`
Expected: FAIL — function not defined.

- [ ] **Step 3: Implement the builder**

Add to `actor.rs`:

```rust
pub fn build_delete_items_by_prefix(
    actor_id: &str,
    item_type: &str,
    featuretype: Option<&str>,
    name_prefix: &str,
) -> Result<Value, String> {
    if name_prefix.is_empty() {
        return Err(
            "foundry/actor.delete_items_by_prefix: empty name_prefix is not allowed"
                .to_string(),
        );
    }
    Ok(json!({
        "type": "actor.delete_items_by_prefix",
        "actor_id": actor_id,
        "item_type": item_type,
        "featuretype": featuretype,
        "name_prefix": name_prefix,
    }))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::delete_items_by_prefix`
Expected: 3 tests PASS.

- [ ] **Step 5: Add JS executor + handler entry**

Add to `actor.js`:

```js
async function deleteItemsByPrefix(actor, msg) {
  const matches = actor.items.filter(
    (i) =>
      i.type === msg.item_type &&
      (msg.featuretype === null ||
        msg.featuretype === undefined ||
        i.system?.featuretype === msg.featuretype) &&
      typeof i.name === "string" &&
      i.name.startsWith(msg.name_prefix),
  );
  if (matches.length === 0) return;
  await actor.deleteEmbeddedDocuments(
    "Item",
    matches.map((i) => i.id),
  );
}
```

Add to `handlers` export:

```js
  "actor.delete_items_by_prefix": wireExecutor(deleteItemsByPrefix),
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add actor.delete_items_by_prefix primitive

Library primitive for filter-and-delete embedded Items by item_type +
optional featuretype + name prefix. Case-sensitive prefix match.
Empty name_prefix returns Err to prevent accidental match-all.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 12: Implement `actor.delete_item_by_id` primitive

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/actor.rs` (add builder + test)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (add executor + handler entry)

**Anti-scope:** Single primitive only.

**Depends on:** Task 3, Task 7.

**Invariants cited:** ARCHITECTURE.md §7.

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn delete_item_by_id_shape() {
        let out = build_delete_item_by_id("actor-xyz", "item-abc");
        assert_eq!(out["type"], "actor.delete_item_by_id");
        assert_eq!(out["actor_id"], "actor-xyz");
        assert_eq!(out["item_id"], "item-abc");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::delete_item_by_id_shape`
Expected: FAIL.

- [ ] **Step 3: Implement the builder**

Add to `actor.rs`:

```rust
pub fn build_delete_item_by_id(actor_id: &str, item_id: &str) -> Value {
    json!({
        "type": "actor.delete_item_by_id",
        "actor_id": actor_id,
        "item_id": item_id,
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::actor::tests::delete_item_by_id_shape`
Expected: PASS.

- [ ] **Step 5: Add JS executor + handler entry**

Add to `actor.js`:

```js
async function deleteItemById(actor, msg) {
  await actor.deleteEmbeddedDocuments("Item", [msg.item_id]);
}
```

Add to `handlers` export:

```js
  "actor.delete_item_by_id": wireExecutor(deleteItemById),
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/actor.rs vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): add actor.delete_item_by_id primitive

Library primitive for deleting one embedded Item by Foundry document id.
Foundry no-ops on bad ids; idempotency-safe.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 13: Refactor `applyDyscrasia` JS executor to compose primitives (A1)

**Files:**
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (rewrite `applyDyscrasia` body to call `deleteItemsByPrefix`/`createFeature`/`appendPrivateNotesLine`)

**Anti-scope:** Do NOT change the wire shape (`actor.apply_dyscrasia` stays as one wire message). Do NOT touch any Rust file. Do NOT change any other executor.

**Depends on:** Task 6 (rename), Task 8 (appendPrivateNotesLine), Task 10 (createFeature), Task 11 (deleteItemsByPrefix).

**Invariants cited:** Existing dyscrasia E2E flow must still work — verified manually. Atomicity preserved (three sequential `await`s in one event-loop tick).

- [ ] **Step 1: Rewrite the `applyDyscrasia` function in `vtmtools-bridge/scripts/foundry-actions/actor.js`**

Replace the existing `applyDyscrasia` function body with:

```js
// Composite. Same wire shape as before; executor composes primitives
// (deleteItemsByPrefix, createFeature, appendPrivateNotesLine) via
// direct JS function calls (A1 strategy from spec §1).
async function applyDyscrasia(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) return;

  if (msg.replace_existing) {
    await deleteItemsByPrefix(actor, {
      item_type: "feature",
      featuretype: "merit",
      name_prefix: "Dyscrasia: ",
    });
  }

  await createFeature(actor, {
    featuretype: "merit",
    name: `Dyscrasia: ${msg.dyscrasia_name}`,
    description: msg.merit_description_html,
    points: 0,
  });

  await appendPrivateNotesLine(actor, { line: msg.notes_line });
}
```

Note that `deleteItemsByPrefix`, `createFeature`, and `appendPrivateNotesLine` are called with `actor` as the first argument and a `msg`-shaped object as the second — matching the function signatures defined in Tasks 8, 10, 11. The composite still does its own actor lookup once at the top (per the `wireExecutor` opt-out pattern).

- [ ] **Step 2: Manual E2E — full dyscrasia apply flow (regression check)**

1. Start vtmtools + Foundry as in Task 2 Step 3.
2. Pick a Foundry-source character that already has a dyscrasia merit (`Dyscrasia: <something>`) on the sheet, and existing private notes content.
3. In vtmtools Resonance tool: roll a new dyscrasia (different from the existing one), confirm, click "Apply to character".
4. Verify on the Foundry sheet:
   - Old `Dyscrasia: <something>` merit is gone.
   - New `Dyscrasia: <name>` merit is present, with the description HTML rendered correctly (mechanical bonus block visible if non-empty).
   - Private notes shows the existing content followed by `\n[YYYY-MM-DD HH:MM] Acquired Dyscrasia: <name> (<resonance>)`.
5. Verify Foundry F12 console: no errors.

If any step fails, the composition broke something the inline version did correctly — debug by comparing the new applyDyscrasia function against the spec §1 sketch and the original inline logic from Task 2 (commit `<sha>` from `git log`).

- [ ] **Step 3: Manual E2E — second apply (idempotency check)**

1. Same setup as Step 2.
2. Apply the SAME dyscrasia again (re-confirm + re-apply).
3. Verify: still exactly one `Dyscrasia: <name>` merit (the prior one was deleted and re-created). Private notes has TWO timestamped lines now (append is not idempotent per spec §2).

- [ ] **Step 4: Run cargo tests to confirm Rust side untouched**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all tests still pass (no Rust changes, but verifying no accidental breakage).

- [ ] **Step 5: Commit (Phase 1 complete after this commit)**

```bash
git add vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
refactor(bridge/foundry): apply_dyscrasia composes primitives (A1)

Rewrite applyDyscrasia executor to call deleteItemsByPrefix,
createFeature, and appendPrivateNotesLine via direct JS function
calls instead of inlining the Foundry API operations. Wire shape
is unchanged — composite atomicity preserved within one
event-loop tick.

Phase 1 (actor primitives library) complete: 5 primitives shipped
in Tasks 8-12; the existing apply_dyscrasia composite now exercises
3 of them as the first consumer of the new library.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 14: Final verification gate

**Files:** none modified (verification only).

**Anti-scope:** No code changes. If any check fails, fix the underlying task and re-run from there — do NOT patch in this task.

**Depends on:** all prior tasks.

**Invariants cited:** Spec §5 verification gate.

- [ ] **Step 1: Run full verify.sh**

Run: `./scripts/verify.sh`
Expected: green (npm check + cargo test + npm build all pass). Cargo test count should include 6 dyscrasia + 1 update_field + 1 create_item_simple + 1 append_private_notes_line + 1 replace_private_notes + 2 create_feature + 3 delete_items_by_prefix + 1 delete_item_by_id = 16 tests in `bridge::foundry::actions::actor::tests`.

- [ ] **Step 2: Verify rename completeness**

Run: `git grep -nE 'update_actor|create_item|apply_dyscrasia' -- ':!docs/'`
Expected: only the new dot-namespaced strings (`actor.update_field`, `actor.create_item_simple`, `actor.apply_dyscrasia`) appear in code and inline comments. Acceptable matches: commit messages in `git log` (not searched here), spec/plan/roadmap/ADR files under `docs/` (excluded by the `:!docs/` pathspec).

If any bare old-name matches appear, identify which task should have caught it and re-run that task to fix.

- [ ] **Step 3: Manual E2E full pass — resonance**

1. Start vtmtools (`npm run tauri dev`).
2. Start Foundry world with vtmtools-bridge module installed; GM logs in.
3. In vtmtools Resonance tool: roll resonance, click "Apply to character" on a Foundry-source character.
4. Verify: `resonance` Item appears on the actor sheet matching the rolled type.
5. Verify Foundry F12 console: no errors, no `unknown inbound type` warnings.

- [ ] **Step 4: Manual E2E full pass — dyscrasia (clean apply)**

1. Pick a Foundry-source character with NO existing dyscrasia merit and empty private notes.
2. Roll dyscrasia, apply.
3. Verify: `Dyscrasia: <name>` merit Item appears with full description HTML; private notes contains exactly one `[YYYY-MM-DD HH:MM] Acquired Dyscrasia: <name> (<resonance>)` line.
4. F12 console clean.

- [ ] **Step 5: Manual E2E full pass — dyscrasia (replace existing)**

1. With the character from Step 4 (now has one dyscrasia and one notes line).
2. Roll a different dyscrasia, apply.
3. Verify: old `Dyscrasia: <previous>` merit is gone; new `Dyscrasia: <name>` merit is present; private notes has TWO lines (the old one preserved, the new one appended).
4. F12 console clean.

- [ ] **Step 6: Tag the verification gate completion in the commit log**

No new commit — this is a verification-only task. If all checks above pass, the implementation is complete.

If desired, add a short note to a future PR description summarizing what was verified:

> Phase 0+1 verified: `./scripts/verify.sh` green, `git grep` shows zero leftover old wire-type names in code, manual E2E covers resonance apply + dyscrasia clean apply + dyscrasia replace.
