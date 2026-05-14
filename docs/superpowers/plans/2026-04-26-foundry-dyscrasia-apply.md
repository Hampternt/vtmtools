# Apply Dyscrasia to Foundry Actor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Foundry-only "Apply Dyscrasia" action: clicking the new button on the Resonance Roller pushes a confirmed dyscrasia onto the selected Foundry actor as a `feature` Item with `featuretype = "merit"` named `Dyscrasia: <name>`, AND appends a timestamped audit line to `system.privatenotes`.

**Architecture:** Approach A from the spec — one new outbound wire shape `apply_dyscrasia` bundles both DB writes into a single Foundry-module handler invocation, atomic from the GM's POV. The Tauri-side `BridgeSource::build_set_attribute` trait is widened to `Result<Value, String>` so payload parse failures surface as IPC errors rather than panics (per ARCHITECTURE.md §7). The frontend `ResultCard` lifts its locally-held `confirmedDyscrasia` up to `Resonance.svelte` via a callback prop so the apply button (which lives in `Resonance.svelte` next to the existing apply-resonance button) can see both the dyscrasia and the selected target character.

**Tech Stack:** Rust 2021 (Tauri 2, sqlx, async-trait, chrono, serde_json), Svelte 5 (runes mode), Foundry V14 module API (`Document.createEmbeddedDocuments`, `actor.update`).

**Spec:** [`docs/superpowers/specs/2026-04-26-foundry-dyscrasia-apply-design.md`](../specs/2026-04-26-foundry-dyscrasia-apply-design.md)

---

## File map

| File | Change | Responsibility |
|---|---|---|
| `src-tauri/src/bridge/source.rs` | modify | Widen `build_set_attribute` return type to `Result<Value, String>` |
| `src-tauri/src/bridge/roll20/mod.rs` | modify | Wrap existing return in `Ok(...)` to satisfy widened trait |
| `src-tauri/src/bridge/foundry/types.rs` | modify | Add `ApplyDyscrasiaPayload` Deserialize struct |
| `src-tauri/src/bridge/foundry/mod.rs` | modify | Wrap existing branches in `Ok(...)`; add `"dyscrasia"` branch + `html_escape` / `render_merit_description` helpers + Rust unit tests |
| `src-tauri/src/bridge/commands.rs` | modify | Propagate `build_set_attribute` Result via `?` |
| `src-tauri/Cargo.toml` | modify | Add `regex` as dev-dependency (used by the dyscrasia tests' `notes_line` format check) |
| `vtmtools-bridge/scripts/bridge.js` | modify | Add `apply_dyscrasia` branch in `handleInbound` (filter+delete prior dyscrasia merits, create new merit Item, append privatenotes line) |
| `src/lib/components/ResultCard.svelte` | modify | Add `onDyscrasiaConfirmChange` callback prop, fire on local state change |
| `src/tools/Resonance.svelte` | modify | Add `confirmedDyscrasia` state + `applyDyscrasia` handler + button rendered for Foundry-source characters only |

No files created. No files deleted.

---

## Dependency graph

```
Task 1 (trait widening) ──┐
                          ├─→ Task 2 (Foundry Rust dyscrasia branch) ──┐
                          └─→ (compiles & existing tests pass)         │
                                                                       ├─→ Task 6 (manual end-to-end)
                          Task 3 (Foundry module JS handler) ──────────┤
                                                                       │
Task 4 (lift dyscrasia state) ──→ Task 5 (apply-dyscrasia button) ─────┘
```

Tasks 2, 3, and 4 are independent of each other once Task 1 lands. SDD can dispatch Task 1 alone, then {Task 2, Task 3, Task 4} in parallel, then Task 5, then Task 6 manual verification.

---

### Task 1: Widen `BridgeSource::build_set_attribute` return type

**Files:**
- Modify: `src-tauri/src/bridge/source.rs`
- Modify: `src-tauri/src/bridge/roll20/mod.rs`
- Modify: `src-tauri/src/bridge/foundry/mod.rs`
- Modify: `src-tauri/src/bridge/commands.rs`

**Anti-scope:** Do not touch `foundry/types.rs`, `bridge.js`, or any Svelte file. Do not add a `"dyscrasia"` branch yet — Task 2 owns that. Do not add tests; this task's correctness is type-system-checked.

**Depends on:** none.

**Invariants cited:** ARCHITECTURE.md §5 (only `bridge/*` talks to WS), §7 (no `unwrap()` in command paths — this widening is the idiomatic fix), §4 (Tauri IPC error type is serialized as `String`).

- [ ] **Step 1: Read the current trait definition**

```bash
cat src-tauri/src/bridge/source.rs
```

The current sig (line 27) is:
```rust
fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Value;
```

- [ ] **Step 2: Widen the return type in the trait**

Edit `src-tauri/src/bridge/source.rs`. Change the trait method signature to:

```rust
/// Build an outbound "set attribute" message in this source's wire
/// format. The `name` and `value` semantics are source-specific —
/// the frontend treats them as opaque strings. Returns Err if the
/// source can't translate the (name, value) pair into its wire
/// shape (e.g. because `value` is a structured payload that fails
/// to parse for this source).
fn build_set_attribute(
    &self,
    source_id: &str,
    name: &str,
    value: &str,
) -> Result<Value, String>;
```

- [ ] **Step 3: Wrap Roll20's impl in Ok(...)**

Edit `src-tauri/src/bridge/roll20/mod.rs:27-34`. Replace:
```rust
fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Value {
    json!({
        "type": "set_attribute",
        "character_id": source_id,
        "name": name,
        "value": value,
    })
}
```
with:
```rust
fn build_set_attribute(
    &self,
    source_id: &str,
    name: &str,
    value: &str,
) -> Result<Value, String> {
    Ok(json!({
        "type": "set_attribute",
        "character_id": source_id,
        "name": name,
        "value": value,
    }))
}
```

- [ ] **Step 4: Wrap Foundry's existing branches in Ok(...)**

Edit `src-tauri/src/bridge/foundry/mod.rs:28-51`. Change the signature and wrap each match arm's return value in `Ok(...)`. The `"dyscrasia"` branch is added in Task 2 — do not add it here.

```rust
fn build_set_attribute(
    &self,
    source_id: &str,
    name: &str,
    value: &str,
) -> Result<Value, String> {
    match name {
        "resonance" => Ok(json!({
            "type": "create_item",
            "actor_id": source_id,
            "item_type": "resonance",
            "item_name": value,
            "replace_existing": true,
        })),
        _ => {
            let path = canonical_to_path(name);
            Ok(json!({
                "type": "update_actor",
                "actor_id": source_id,
                "path": path,
                "value": parse_value(value),
            }))
        }
    }
}
```

- [ ] **Step 5: Propagate the Result in `bridge_set_attribute`**

Edit `src-tauri/src/bridge/commands.rs:39-49`. The current line is:
```rust
let payload = source_impl.build_set_attribute(&source_id, &name, &value);
```
Replace with:
```rust
let payload = source_impl
    .build_set_attribute(&source_id, &name, &value)
    .map_err(|e| format!("bridge/set_attribute: {e}"))?;
```

The error prefix matches the per-command identifier convention from ARCHITECTURE.md §7.

- [ ] **Step 6: Compile and run existing tests**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```
Expected: clean compile, all existing tests pass. No new tests added by this task — the type system is the test.

- [ ] **Step 7: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```
Expected: green.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/bridge/source.rs src-tauri/src/bridge/roll20/mod.rs src-tauri/src/bridge/foundry/mod.rs src-tauri/src/bridge/commands.rs
git commit -m "$(cat <<'EOF'
refactor(bridge): Result-typed BridgeSource::build_set_attribute

Prepares the trait surface for sources that take structured payloads
(serde_json::from_str of a JSON-encoded value) and need to surface
parse errors. ARCHITECTURE.md §7 forbids unwrap() in command paths;
widening to Result<Value, String> is the idiomatic fix.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Add `ApplyDyscrasiaPayload` + `"dyscrasia"` branch in `FoundrySource`

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs`
- Modify: `src-tauri/src/bridge/foundry/mod.rs`
- Modify: `src-tauri/Cargo.toml` (add `regex` to `[dev-dependencies]`)

**Anti-scope:** Do not touch `bridge.js`, any Svelte file, or `roll20/mod.rs`. Do not modify `bridge/commands.rs` or `source.rs` — Task 1 owns those.

**Depends on:** Task 1 (needs `Result<Value, String>` return type).

**Invariants cited:** ARCHITECTURE.md §2 (Bridge domain types — wire shape is defined per-source), §5 (only `bridge/*` talks to WS, only `foundry/*` knows Foundry wire format), §7 (no `unwrap`, errors as `String` with module-stable prefix), §10 (Rust unit tests live as `#[cfg(test)] mod tests` inside each source file).

- [ ] **Step 1: Add the payload struct in `foundry/types.rs`**

Edit `src-tauri/src/bridge/foundry/types.rs`. Append after the existing `FoundryActor` struct:

```rust
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
```

`Deserialize` is already imported at the top of the file (`use serde::{Deserialize, Serialize};`) — no new import needed.

- [ ] **Step 2: Add `regex` as a dev-dependency**

Edit `src-tauri/Cargo.toml`. Add (or extend the existing `[dev-dependencies]` section):

```toml
[dev-dependencies]
regex = "1"
```

If a `[dev-dependencies]` section already exists, append the `regex` line to it instead of creating a duplicate section.

- [ ] **Step 3: Write the failing tests**

Edit `src-tauri/src/bridge/foundry/mod.rs`. Append at the end of the file:

```rust
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
        let src = FoundrySource;
        let payload = payload_json(
            "Wax",
            "Choleric",
            "Crystallized blood.",
            "+1 Composure",
        );
        let out = src
            .build_set_attribute("actor-abc", "dyscrasia", &payload)
            .expect("happy path returns Ok");
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
        assert!(
            re.is_match(line),
            "notes_line did not match expected format: {line}"
        );
    }

    #[test]
    fn dyscrasia_empty_bonus_omits_bonus_block() {
        let src = FoundrySource;
        let payload = payload_json("Custom", "Sanguine", "Some description.", "");
        let out = src
            .build_set_attribute("a", "dyscrasia", &payload)
            .expect("empty bonus is valid");
        let html = out["merit_description_html"].as_str().unwrap();
        assert_eq!(html, "<p>Some description.</p>");
        assert!(!html.contains("Mechanical bonus"));
    }

    #[test]
    fn dyscrasia_html_escapes_dangerous_chars() {
        let src = FoundrySource;
        let payload = payload_json(
            "Test",
            "Phlegmatic",
            "<script>alert(\"x\")</script>",
            "& > <",
        );
        let out = src
            .build_set_attribute("a", "dyscrasia", &payload)
            .expect("html-escape happy path");
        let html = out["merit_description_html"].as_str().unwrap();
        assert!(html.contains("&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;"));
        assert!(html.contains("&amp; &gt; &lt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn dyscrasia_malformed_payload_returns_err() {
        let src = FoundrySource;
        let result = src.build_set_attribute("a", "dyscrasia", "{not valid json");
        assert!(result.is_err(), "malformed payload must return Err, not panic");
        let msg = result.unwrap_err();
        assert!(
            msg.starts_with("foundry/dyscrasia: invalid payload:"),
            "error message must use module-prefixed convention, got: {msg}"
        );
    }
}
```

- [ ] **Step 4: Run tests to confirm they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::tests::dyscrasia 2>&1 | tail -40
```
Expected: compile error (`ApplyDyscrasiaPayload` reachable but no `"dyscrasia"` match arm exists yet, so the happy-path test fails — it'll match the `_` catch-all and produce `update_actor` shape, not `apply_dyscrasia`). Or assertion failures because `out["type"] != "apply_dyscrasia"`.

- [ ] **Step 5: Add helper functions in `foundry/mod.rs`**

Edit `src-tauri/src/bridge/foundry/mod.rs`. Add the following private helper functions ABOVE the `#[cfg(test)] mod tests` block (and below the existing `parse_value` function):

```rust
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
```

Note: the `&` substitution must come FIRST in `html_escape` so the literal `&` in `&amp;` (etc.) doesn't get re-escaped by subsequent passes.

- [ ] **Step 6: Add the `"dyscrasia"` match arm in `build_set_attribute`**

Edit `src-tauri/src/bridge/foundry/mod.rs`. First update the `use` statement at the top of the file:
```rust
use crate::bridge::foundry::types::{ApplyDyscrasiaPayload, FoundryInbound};
```

Then add a new arm to the `match name` block in `build_set_attribute` ABOVE the `_` catch-all and below the existing `"resonance"` arm:

```rust
"dyscrasia" => {
    let payload: ApplyDyscrasiaPayload = serde_json::from_str(value)
        .map_err(|e| format!("foundry/dyscrasia: invalid payload: {e}"))?;
    let merit_description_html =
        render_merit_description(&payload.description, &payload.bonus);
    let applied_at = chrono::Local::now()
        .format("%Y-%m-%d %H:%M")
        .to_string();
    let notes_line = format!(
        "[{applied_at}] Acquired Dyscrasia: {} ({})",
        payload.dyscrasia_name, payload.resonance_type
    );
    Ok(json!({
        "type": "apply_dyscrasia",
        "actor_id": source_id,
        "dyscrasia_name": payload.dyscrasia_name,
        "resonance_type": payload.resonance_type,
        "merit_description_html": merit_description_html,
        "notes_line": notes_line,
        "replace_existing": true,
    }))
}
```

The `?` operator works because Task 1 widened the return type to `Result<Value, String>`.

- [ ] **Step 7: Run tests to confirm they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::tests 2>&1 | tail -20
```
Expected: all 4 dyscrasia tests pass; no other tests regress.

- [ ] **Step 8: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```
Expected: green.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/bridge/foundry/types.rs src-tauri/src/bridge/foundry/mod.rs src-tauri/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(bridge/foundry): apply_dyscrasia wire shape

FoundrySource::build_set_attribute("dyscrasia", payload_json) parses
ApplyDyscrasiaPayload, renders HTML-escaped merit description (omitting
the Mechanical bonus block when bonus is empty), stamps applied_at via
chrono::Local::now, and emits the apply_dyscrasia wire shape carrying
both the merit and the privatenotes audit line. Tests cover happy path,
empty bonus, HTML escape, and malformed payload (returns Err, no panic).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Add `apply_dyscrasia` handler in vtmtools-bridge module

**Files:**
- Modify: `vtmtools-bridge/scripts/bridge.js`

**Anti-scope:** Do not modify `vtmtools-bridge/scripts/translate.js` (no schema awareness leaks into the JS module — all field rendering happens Rust-side per spec §1). Do not touch any Rust file. Do not touch `vtmtools-bridge/styles/` or `module.json`.

**Depends on:** Task 2 (the wire shape Task 3 consumes is established in Task 2's commit), but a fresh subagent can implement Task 3 from the spec alone — Task 2 is a logical predecessor, not a code dependency.

**Invariants cited:** ARCHITECTURE.md §4 (Bridge WebSocket protocol — wire shape is defined entirely by the source impl + module pair, no in-message tags), §8 (Foundry module surface: only GM browser, reads `game.actors`, writes via `actor.update` + `createEmbeddedDocuments`).

- [ ] **Step 1: Read the existing `handleInbound` function**

```bash
cat vtmtools-bridge/scripts/bridge.js
```

Locate the `async function handleInbound(msg)` (around line 73). Note the existing `update_actor` and `create_item` branches — the new `apply_dyscrasia` branch follows the same structural pattern.

- [ ] **Step 2: Add the `apply_dyscrasia` branch**

Edit `vtmtools-bridge/scripts/bridge.js`. Inside `handleInbound`, add a new branch BEFORE the final `console.warn(...)` line and AFTER the existing `create_item` branch:

```js
  if (msg.type === "apply_dyscrasia") {
    const actor = game.actors.get(msg.actor_id);
    if (!actor) return;

    // (1) Delete prior dyscrasia merits. Filter is name-prefix-based —
    // any feature Item with featuretype="merit" whose name starts with
    // "Dyscrasia: " is treated as tool-managed and clobbered. Documented
    // limitation in spec §2.
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

    // (2) Create the new dyscrasia merit.
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

    // (3) Append timestamped audit line to private notes. Empty notes →
    // bare line; existing content → newline-prefixed append.
    const current = actor.system?.privatenotes ?? "";
    const next =
      current.trim() === ""
        ? msg.notes_line
        : `${current}\n${msg.notes_line}`;
    await actor.update({ "system.privatenotes": next });
    return;
  }
```

- [ ] **Step 3: Syntax-check the JS file**

vtmtools doesn't have a JS-side test framework or linter for the Foundry module (spec §6, ARCHITECTURE.md §10). The only automated check available is parser validity:

```bash
node --check vtmtools-bridge/scripts/bridge.js
```
Expected: no output (file parses cleanly).

- [ ] **Step 4: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```
Expected: green. (The Foundry module isn't in the verify pipeline — it's sideloaded into a Foundry world separately. verify.sh covers the Tauri side only, which is unaffected by this task.)

- [ ] **Step 5: Commit**

```bash
git add vtmtools-bridge/scripts/bridge.js
git commit -m "$(cat <<'EOF'
feat(foundry-module): apply_dyscrasia handler

handleInbound branch for the apply_dyscrasia wire shape: deletes any
prior feature/merit Items whose name starts with 'Dyscrasia: ',
creates a new merit Item with the rendered HTML description, and
appends the prerendered timestamped audit line to system.privatenotes.

All formatting decisions are Rust-side; this handler is a thin
DB-write executor.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Lift `confirmedDyscrasia` from `ResultCard` via callback prop

**Files:**
- Modify: `src/lib/components/ResultCard.svelte`
- Modify: `src/tools/Resonance.svelte`

**Anti-scope:** Do not touch `AcutePanel.svelte` (the AcutePanel's existing `onconfirm` callback to ResultCard is unchanged — Task 4 only adds a NEW callback going outward from ResultCard). Do not modify any Rust file or `bridge.js`. Do not add an apply button yet — Task 5 owns that.

**Depends on:** none (independent of all backend tasks; can run in parallel with Tasks 1, 2, 3).

**Invariants cited:** ARCHITECTURE.md §6 (Svelte 5 runes mode — callback props are the standard cross-component channel), §5 (frontend components never call `invoke(...)` directly — context only; this task adds no Tauri call).

- [ ] **Step 1: Read both files to understand current state**

```bash
cat src/lib/components/ResultCard.svelte
sed -n '1,150p' src/tools/Resonance.svelte
```

In `ResultCard.svelte` note: `let confirmedDyscrasia: DyscrasiaEntry | null = $state(null)` and the AcutePanel `onconfirm={(d) => { confirmedDyscrasia = d; acuteConfirmed = true; }}` handler. Also note the `$effect` that resets state when `result` changes.

- [ ] **Step 2: Add the callback prop in `ResultCard.svelte`**

Edit `src/lib/components/ResultCard.svelte`. Find the existing `$props()` destructure. Add a new optional prop `onDyscrasiaConfirmChange`:

```ts
let {
  result,
  onDyscrasiaConfirmChange,
}: {
  result: ResonanceRollResult;
  onDyscrasiaConfirmChange?: (d: DyscrasiaEntry | null) => void;
} = $props();
```

If the existing `$props()` uses a different destructure shape, match it — the goal is to add one new optional prop, not to refactor how props are received.

- [ ] **Step 3: Fire the callback whenever `confirmedDyscrasia` changes**

In `src/lib/components/ResultCard.svelte`, add a new `$effect` (next to the existing one that resets state):

```ts
$effect(() => {
  onDyscrasiaConfirmChange?.(confirmedDyscrasia);
});
```

This runs once on initial mount (firing `null`) and on every subsequent change. The reactive dependency on `confirmedDyscrasia` is automatic in runes mode because `$state` reads inside `$effect` are tracked.

- [ ] **Step 4: Capture the callback in `Resonance.svelte`**

Edit `src/tools/Resonance.svelte`. Add `DyscrasiaEntry` to the type imports at line 10:

```ts
import type { RollConfig, ResonanceRollResult, HistoryEntry, BridgeCharacter, Roll20Raw, DyscrasiaEntry } from '../types';
```

Then add new state near the existing `applyState` declaration (around line 38):

```ts
let confirmedDyscrasia = $state<DyscrasiaEntry | null>(null);
```

Update the `<ResultCard {result} />` rendering (around line 241) to pass the callback:

```svelte
<ResultCard
  {result}
  onDyscrasiaConfirmChange={(d) => { confirmedDyscrasia = d; }}
/>
```

- [ ] **Step 5: Type-check the frontend**

```bash
npm run check
```
Expected: no new errors. (Pre-existing warnings documented in ARCHITECTURE.md §10 are still expected.)

- [ ] **Step 6: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```
Expected: green.

- [ ] **Step 7: Manual smoke check (optional, recommended)**

This task should be invisible behavior-wise. If a Tauri dev session is available, quickly verify that the existing apply-resonance flow still works:

```bash
npm run tauri dev
```

In the Tauri window: connect a VTT, roll resonance, confirm dyscrasia (Intense temperament), verify the existing `✓ Apply to <name>` button still appears and apply still succeeds. If broken, the callback wiring needs debugging before proceeding.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/ResultCard.svelte src/tools/Resonance.svelte
git commit -m "$(cat <<'EOF'
refactor(frontend): lift confirmedDyscrasia from ResultCard to Resonance

Adds an optional onDyscrasiaConfirmChange callback prop on ResultCard
that fires whenever local confirmedDyscrasia state changes. Resonance
captures it into its own $state. No behavior change; sets up the
apply-dyscrasia button (next task) to read both the dyscrasia and the
selected target character from one component.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Add apply-dyscrasia button + handler in `Resonance.svelte`

**Files:**
- Modify: `src/tools/Resonance.svelte`

**Anti-scope:** Do not touch `ResultCard.svelte`, `AcutePanel.svelte`, or any Rust file. Do not call `invoke()` directly — go through `setAttribute` from `$lib/bridge/api`.

**Depends on:** Task 4 (needs `confirmedDyscrasia` state in scope). Logically also depends on Tasks 2 + 3 for end-to-end function, but compiles & types check without them — only the live-Foundry test in Task 6 requires the full stack.

**Invariants cited:** ARCHITECTURE.md §4 (frontend never calls `invoke()` directly — uses typed wrapper in `src/lib/**/api.ts`), §6 (CSS uses tokens from `:root`, no hardcoded hex except for transient hover/glow states).

- [ ] **Step 1: Add the apply-dyscrasia state machine**

Edit `src/tools/Resonance.svelte`. Near the existing `let applyState = $state<...>(...)` declaration (around line 38, added in this file before Task 4), add:

```ts
let dyscrasiaApplyState = $state<'idle' | 'applying' | 'applied' | 'error'>('idle');
```

- [ ] **Step 2: Add the `applyDyscrasia` function**

In the `// ── Actions ──` section (after the existing `applyToCharacter` function, around line 137), add:

```ts
async function applyDyscrasia() {
  if (!confirmedDyscrasia || !selectedChar) return;
  if (selectedChar.source !== 'foundry') return; // hard guard; UI also hides the button
  dyscrasiaApplyState = 'applying';
  try {
    const payload = JSON.stringify({
      dyscrasia_name: confirmedDyscrasia.name,
      resonance_type: confirmedDyscrasia.resonanceType,
      description: confirmedDyscrasia.description,
      bonus: confirmedDyscrasia.bonus,
    });
    await setAttribute(
      selectedChar.source,
      selectedChar.source_id,
      'dyscrasia',
      payload,
    );
    dyscrasiaApplyState = 'applied';
    setTimeout(() => { dyscrasiaApplyState = 'idle'; }, 1800);
  } catch {
    dyscrasiaApplyState = 'error';
    setTimeout(() => { dyscrasiaApplyState = 'idle'; }, 1800);
  }
}
```

This assumes `DyscrasiaEntry` has `name`, `resonanceType`, `description`, `bonus` fields. Check `src/types.ts` and adjust the payload field reads if names differ. (`resonanceType` is the camelCase mirror of Rust's `resonance_type` per ARCHITECTURE.md §2.)

- [ ] **Step 3: Render the apply-dyscrasia button**

Edit `src/tools/Resonance.svelte`. Find the existing `.apply-row` block (around lines 242-257). Extend it to include the apply-dyscrasia button alongside the existing apply-resonance button:

```svelte
{#if result}
  <ResultCard
    {result}
    onDyscrasiaConfirmChange={(d) => { confirmedDyscrasia = d; }}
  />
  <div class="apply-row">
    {#if selectedChar && result.resonanceType}
      <button
        class="apply-btn"
        class:applied={applyState === 'applied'}
        class:error={applyState === 'error'}
        onclick={applyToCharacter}
        disabled={applyState !== 'idle'}
      >
        {applyState === 'applying' ? 'Applying…'
         : applyState === 'applied' ? '✓ Applied'
         : applyState === 'error' ? '✗ Failed — retry'
         : `✓ Apply to ${selectedChar.name}`}
      </button>
    {/if}
    {#if selectedChar?.source === 'foundry' && confirmedDyscrasia !== null}
      <button
        class="apply-btn apply-btn--dyscrasia"
        class:applied={dyscrasiaApplyState === 'applied'}
        class:error={dyscrasiaApplyState === 'error'}
        onclick={applyDyscrasia}
        disabled={dyscrasiaApplyState !== 'idle'}
      >
        {dyscrasiaApplyState === 'applying' ? 'Applying Dyscrasia…'
         : dyscrasiaApplyState === 'applied' ? '✓ Dyscrasia Applied'
         : dyscrasiaApplyState === 'error' ? '✗ Failed — retry'
         : `✓ Apply Dyscrasia to ${selectedChar.name}`}
      </button>
    {/if}
  </div>
{/if}
```

- [ ] **Step 4: Update `.apply-row` CSS to wrap on narrow widths**

In the `<style>` block of `src/tools/Resonance.svelte`, replace the existing `.apply-row` rule:
```css
.apply-row { display: flex; justify-content: flex-end; }
```
with:
```css
.apply-row {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  flex-wrap: wrap;
}
```

The existing `.apply-btn` rules already cover sizing/look. The `--dyscrasia` modifier class is added for future visual divergence (no rules under it yet — leave it as a hook). No new color tokens needed; per ARCHITECTURE.md §6 do NOT add hardcoded hex.

- [ ] **Step 5: Type-check**

```bash
npm run check
```
Expected: no new errors.

- [ ] **Step 6: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```
Expected: green.

- [ ] **Step 7: Visual smoke test (no Foundry needed)**

```bash
npm run tauri dev
```
1. Open the Resonance tool. Without connecting any VTT, verify no apply buttons render.
2. (If a Roll20 extension is available) Connect Roll20, select a Roll20 character, roll → confirm dyscrasia. Verify ONLY the apply-resonance button appears, NOT the apply-dyscrasia button.
3. Verify the existing apply-resonance button still works on Roll20.

The full Foundry end-to-end test is in Task 6.

- [ ] **Step 8: Commit**

```bash
git add src/tools/Resonance.svelte
git commit -m "$(cat <<'EOF'
feat(frontend): apply-dyscrasia button on Resonance for Foundry actors

Renders next to the existing apply-resonance button when:
  - selectedChar.source === 'foundry'
  - confirmedDyscrasia !== null
Hidden (not disabled) for Roll20 characters — Roll20 has no
dyscrasia-application path and never will (per VTT-asymmetry posture).

Serializes the dyscrasia entry as a JSON payload and goes through the
existing setAttribute typed wrapper with name='dyscrasia'. Backend
(Foundry source's build_set_attribute) handles the payload parse, HTML
rendering, timestamp stamping, and wire-shape construction.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Manual end-to-end verification

**Files:** none (verification-only).

**Anti-scope:** No code changes. If a bug surfaces during E2E, fix it via a follow-up commit on the appropriate task's files — do not bundle a fix into a "verification" commit.

**Depends on:** Tasks 2, 3, 5 (the full apply-dyscrasia path). Tasks 1, 4 are transitive dependencies.

**Invariants cited:** ARCHITECTURE.md §10 (`./scripts/verify.sh` is the aggregate gate; manual end-to-end is required for bridge features because no JS-side or integration test framework exists).

- [ ] **Step 1: Confirm verify.sh is green**

```bash
./scripts/verify.sh
```
Expected: green. If anything regressed during the per-task work, fix in the originating task's file before proceeding.

- [ ] **Step 2: Sideload the bridge module into a live Foundry world**

If the Foundry module isn't already sideloaded, follow `vtmtools-bridge/README.md`. The directory MUST be named `vtmtools-bridge` inside the world's `Data/modules/` (per the dir-name rule that's documented in the README and was the subject of commit `ed6aec8`). Restart the Foundry world; enable the module in the world's Module Settings.

Open `https://localhost:7424` once in the GM's browser and accept the self-signed cert prompt (per ARCHITECTURE.md §4).

- [ ] **Step 3: Launch vtmtools and connect**

```bash
npm run tauri dev
```
The Foundry pip should turn green within a few seconds of GM-loading the world.

- [ ] **Step 4: Run the spec §8 manual test sequence**

Execute steps 1-9 of spec §8 in order. Each step should pass before moving to the next:

1. Foundry actor visible in Resonance Roller's character strip.
2. Roll resonance → Intense → confirm a built-in dyscrasia in the Acute panel → "✓ Apply Dyscrasia to <actor name>" button appears next to the existing apply-resonance button.
3. After click: actor's Features tab in Foundry shows new `Dyscrasia: <name>` merit, description renders both paragraphs (description text + Mechanical bonus row).
4. After click: actor's `system.privatenotes` (sheet → biography → private notes section) shows the timestamped line `[YYYY-MM-DD HH:MM] Acquired Dyscrasia: <name> (<resonance>)`.
5. Apply a second different dyscrasia → first merit deleted, second created; notes get a second line on its own row, first line preserved.
6. Pre-populate `system.privatenotes` with multi-line GM notes via the sheet → apply → user text preserved verbatim, new line appended at bottom.
7. Apply a custom dyscrasia whose `bonus` is empty (create one in DyscrasiaManager first if needed) → merit description renders only the description `<p>`, no "Mechanical bonus" row.
8. Apply a custom dyscrasia whose `description` contains `<` and `&` characters → the merit displays the literal characters (escaped), no markup leaks into the sheet.
9. (If Roll20 is also connected) Select a Roll20 character → apply-dyscrasia button is NOT rendered (verify by inspecting the rendered DOM, not just by clicking).

- [ ] **Step 5: If everything passes, no further commit needed**

The per-task commits already record the shipped state. If a bug is discovered, fix it in the originating task's file with its own commit; do not amend any of the per-task commits.

- [ ] **Step 6: Update memory if anything surprised you**

If during the end-to-end test you discover a Foundry quirk the spec missed (e.g., a sheet field that renders HTML differently than expected, an Item field shape that diverged from the pinned WoD5e schema), save a small `project_*.md` memory in `/home/hampter/.claude/projects/-home-hampter-projects-vtmtools/memory/` documenting the gotcha. Otherwise skip this step.

---

## Self-review

(Following the writing-plans skill's required self-review checklist.)

**1. Spec coverage:** Walked through each spec section.

- §1 Wire protocol → Task 2 (Rust-side construction), Task 3 (JS-side consumption). ✅
- §2 Foundry module behavior → Task 3. ✅
- §3 Merit description HTML rendering → Task 2 Step 5 (`render_merit_description`, `html_escape`) + tests in Step 3. ✅
- §4 Backend (Rust) changes → Task 1 (trait widening), Task 2 (struct + branch + helpers). ✅
- §5 Frontend (Svelte / TS) changes → Task 4 (callback prop, state lift), Task 5 (handler + button + CSS). ✅
- §6 What does not change → Anti-scope sections of each task enforce this. ✅
- §7 Out of scope → Anti-scopes plus the deliberate choice to leave `roll20/mod.rs`'s existing branches untouched. ✅
- §8 Test plan → Task 2 (Rust unit tests, all four cases), Task 6 (manual end-to-end, all nine cases). ✅

No gaps.

**2. Placeholder scan:** Searched for "TBD", "TODO", "implement later", "fill in details", "Add appropriate", "handle edge cases", "Write tests for the above", "Similar to Task". None present.

**3. Type consistency:**

- `ApplyDyscrasiaPayload` field names (`dyscrasia_name`, `resonance_type`, `description`, `bonus`) match between Task 2 Step 1 (struct definition), Task 2 Step 6 (payload usage in match arm), and Task 5 Step 2 (frontend `JSON.stringify` keys). ✅
- `confirmedDyscrasia` type (`DyscrasiaEntry | null`) consistent across Task 4 (state declaration, callback prop type) and Task 5 (payload reads). ✅
- Wire shape field names (`type`, `actor_id`, `dyscrasia_name`, `resonance_type`, `merit_description_html`, `notes_line`, `replace_existing`) consistent between Task 2 Step 6 (Rust emit) and Task 3 Step 2 (JS read). ✅
- `BridgeSource::build_set_attribute` return type widened in Task 1 and consumed via `?` in Task 1 Step 5 (`commands.rs`) and inside the `?`-using `"dyscrasia"` arm in Task 2 Step 6. ✅
- `dyscrasiaApplyState` literal values (`'idle'`, `'applying'`, `'applied'`, `'error'`) consistent between Task 5 Step 1 (state declaration) and Task 5 Step 3 (button rendering). ✅

No inconsistencies.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-26-foundry-dyscrasia-apply.md` (gitignored, same as the spec — local-only working artifact).

**1. Subagent-Driven (recommended)** — fresh subagent per task, review between tasks. Suits this plan well: Tasks 1, 2, 3, 4 admit two parallel waves — Task 1 alone first, then `{Task 2, Task 3, Task 4}` concurrently after Task 1 lands, then Task 5, then Task 6 manual.

**2. Inline Execution** — execute tasks in this session using executing-plans, batched with checkpoints for review. Lower coordination overhead, no parallelism, and the Task 6 manual end-to-end still requires you to verify in the live Foundry world.

Which approach?
