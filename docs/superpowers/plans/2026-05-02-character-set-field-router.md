# `character::set_field` router — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship issue [#6](https://github.com/Hampternt/vtmtools/issues/6) — a single Tauri command (`character_set_field`) that mutates one canonical-named field on a character, routing to the saved DB row, the live bridge, or both per an explicit `WriteTarget`.

**Architecture:** New shared module `src-tauri/src/shared/canonical_fields.rs` owns the canonical-name namespace as a typed contract. New tools module `src-tauri/src/tools/character.rs` is a thin composer over (a) the existing `bridge_set_attribute` (live side), and (b) a new `db_patch_field` helper on `db/saved_character.rs` (saved side). Two small bridge refactors expose seams the router needs without touching wire shape.

**Tech Stack:** Rust 2021 (Tauri 2, sqlx, tokio, serde_json), TypeScript (Svelte 5 runes mode), SQLite.

**Spec:** `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md`

**Hard rules** (from `CLAUDE.md`):
- Every task ending in a commit MUST run `./scripts/verify.sh` first and produce a green result.
- Frontend components NEVER call `invoke(...)` directly — go through `src/lib/**/api.ts`.
- Never run `git status -uall` (memory issues on large trees).
- Tauri command request/response shape is the stable contract; do not bypass.

---

## Fresh-session bootstrap

If you are picking this plan up in a new session, here's everything you need:

**Read first (in order):**
1. `CLAUDE.md` — auto-loaded from project root; defines the verify gate, frontend wrapper rule, and other invariants.
2. **This plan** — has full code for every task. The spec is referenced but **not strictly required** for execution.
3. (Optional, only if a step surprises you) `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md` — adds rationale ("why saved-first?", "why deferred Roll20?").

**Recommended dispatch shape (subagent-driven):**

```
Task 0 (pre-flight)              ← single subagent, blocking
  │
  ▼
[Task 1, Task 2]                  ← parallel batch (independent files)
  │
  ▼
[Task 3, Task 4]                  ← parallel batch (both depend on Task 1; touch different files)
  │
  ▼
Task 5 → Task 6 → Task 7 → Task 8 ← sequential
```

Tasks 1+2 and 3+4 touch disjoint files (verify against the per-task `Files:` blocks), so concurrent edits don't collide. Anti-scope (per task body) enforces the boundary if a subagent strays.

**Suggested first message in the new session:**

> "Execute the plan at `docs/superpowers/plans/2026-05-02-character-set-field-router.md` using `superpowers:subagent-driven-development`. Run Task 0 first; then dispatch Tasks 1+2 in parallel; then 3+4; then 5, 6, 7, 8 sequentially. Final commit footer: `Closes #6`."

That's enough to bootstrap a fresh session into productive execution; the plan's per-task code blocks carry the rest of the context.

---

## File map

| Action | Path | Purpose | Task |
|---|---|---|---|
| Create | `src-tauri/src/shared/canonical_fields.rs` | Namespace SSoT — `ALLOWED_NAMES`, `apply_canonical_field`, per-source path mappers | 1 |
| Modify | `src-tauri/src/shared/mod.rs` | Add `pub mod canonical_fields;` | 1 |
| Modify | `src-tauri/src/bridge/commands.rs` | Extract `do_set_attribute` + `send_to_source_inner` | 2 |
| Modify | `src-tauri/src/bridge/foundry/mod.rs` | Delegate `canonical_to_path` to shared helper | 3 |
| Modify | `src-tauri/src/db/saved_character.rs` | Add `db_patch_field` + `patch_saved_field` Tauri command + tests | 4 |
| Create | `src-tauri/src/tools/character.rs` | Router (`character_set_field` Tauri command) | 5 |
| Modify | `src-tauri/src/tools/mod.rs` | Add `pub mod character;` | 5 |
| Modify | `src-tauri/src/lib.rs` | Register `character_set_field` and `patch_saved_field` in `invoke_handler!` | 6 |
| Modify | `src/types.ts` | Mirror `WriteTarget` and `CanonicalFieldName` | 7 |
| Create | `src/lib/character/api.ts` | Typed frontend wrapper | 7 |
| n/a | (final verification) | `./scripts/verify.sh` + manual smoke note | 8 |

Total: 3 created Rust files (one is a directory's `mod.rs` line — already exists), 1 created TS file, 7 modifications. No SQL migrations. No new wire variants. No new Tauri events.

---

### Task 0: Pre-flight green build

Verifies the workspace is green before starting; surfaces any unrelated issues before they get attributed to this work.

**Files:** none

- [ ] **Step 1: Run the aggregate gate.**

```bash
./scripts/verify.sh
```

Expected: green. If it fails, stop and resolve before starting Task 1.

---

### Task 1: `shared/canonical_fields.rs` — namespace SSoT

The pure helper module that owns the canonical-name list, the saved-side mutator, and per-source path translators.

**Files:**
- Create: `src-tauri/src/shared/canonical_fields.rs`
- Modify: `src-tauri/src/shared/mod.rs` (add one line)

- [ ] **Step 1: Write the failing tests first (TDD).**

Create `src-tauri/src/shared/canonical_fields.rs` with the **tests only** plus stub function signatures:

```rust
//! Canonical-name namespace for character field updates.
//!
//! Single source of truth for the set of names accepted by
//! `character::set_field`. Per-source translators MUST cover ALLOWED_NAMES;
//! `cargo test` enforces coverage.

use crate::bridge::types::{CanonicalCharacter, HealthTrack};
use serde_json::Value;

pub const ALLOWED_NAMES: &[&str] = &[];

pub fn apply_canonical_field(
    _c: &mut CanonicalCharacter,
    _name: &str,
    _value: &Value,
) -> Result<(), String> {
    Err("not implemented".to_string())
}

pub fn canonical_to_foundry_path(_name: &str) -> Option<&'static str> {
    None
}

pub fn canonical_to_roll20_attr(_name: &str) -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::types::SourceKind;

    fn sample() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "x".to_string(),
            name: "T".to_string(),
            controlled_by: None,
            hunger: None,
            health: None,
            willpower: None,
            humanity: None,
            humanity_stains: None,
            blood_potency: None,
            raw: serde_json::json!({}),
        }
    }

    #[test]
    fn apply_hunger_happy_path() {
        let mut c = sample();
        apply_canonical_field(&mut c, "hunger", &serde_json::json!(3)).unwrap();
        assert_eq!(c.hunger, Some(3));
    }

    #[test]
    fn apply_hunger_out_of_range_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "hunger", &serde_json::json!(7))
            .unwrap_err();
        assert!(err.contains("expects integer 0..=5"), "got: {err}");
    }

    #[test]
    fn apply_hunger_wrong_type_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "hunger", &serde_json::json!("3"))
            .unwrap_err();
        assert!(err.contains("got string"), "got: {err}");
    }

    #[test]
    fn apply_unknown_name_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "xyzzy", &serde_json::json!(0))
            .unwrap_err();
        assert!(err.contains("unknown field 'xyzzy'"), "got: {err}");
    }

    #[test]
    fn apply_health_creates_default_track_if_missing() {
        let mut c = sample();
        apply_canonical_field(&mut c, "health_superficial", &serde_json::json!(2))
            .unwrap();
        let t = c.health.unwrap();
        assert_eq!(t.superficial, 2);
        assert_eq!(t.aggravated, 0);
        assert_eq!(t.max, 0);
    }

    #[test]
    fn apply_humanity_stains_happy_path() {
        let mut c = sample();
        apply_canonical_field(&mut c, "humanity_stains", &serde_json::json!(2))
            .unwrap();
        assert_eq!(c.humanity_stains, Some(2));
    }

    #[test]
    fn every_allowed_name_has_foundry_path() {
        for n in ALLOWED_NAMES {
            assert!(
                canonical_to_foundry_path(n).is_some(),
                "missing Foundry path for {n}"
            );
        }
    }

    #[test]
    fn every_allowed_name_applies_via_apply_canonical_field() {
        for n in ALLOWED_NAMES {
            let mut c = sample();
            let v = serde_json::json!(0);
            let res = apply_canonical_field(&mut c, n, &v);
            assert!(
                res.is_ok(),
                "apply_canonical_field rejected '{n}': {:?}",
                res.err()
            );
        }
    }

    #[test]
    fn roll20_attr_stub_returns_none_for_all_names() {
        for n in ALLOWED_NAMES {
            assert!(
                canonical_to_roll20_attr(n).is_none(),
                "v1 stub should return None for {n}"
            );
        }
    }
}
```

Add `pub mod canonical_fields;` to `src-tauri/src/shared/mod.rs`. The current file has:

```rust
pub mod types;
pub mod dice;
pub mod resonance;
pub mod v5;
```

Add the new line so it reads:

```rust
pub mod types;
pub mod dice;
pub mod resonance;
pub mod v5;
pub mod canonical_fields;
```

- [ ] **Step 2: Run tests — verify they fail.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields
```

Expected: FAIL. The stub returns `Err("not implemented")`, so `apply_hunger_happy_path`, `apply_hunger_out_of_range_errors`, `apply_hunger_wrong_type_errors`, `apply_health_creates_default_track_if_missing`, and `apply_humanity_stains_happy_path` should fail. `apply_unknown_name_errors` will pass coincidentally (the stub error contains "not implemented", but the assertion checks for "unknown field 'xyzzy'", so it should also fail). The two coverage tests (`every_allowed_name_*`) pass *vacuously* on an empty ALLOWED_NAMES — that's expected; they only have teeth once Step 3 populates the list.

- [ ] **Step 3: Implement.**

Replace the file body (everything except the test module) with:

```rust
//! Canonical-name namespace for character field updates.
//!
//! Single source of truth for the set of names accepted by
//! `character::set_field`. Per-source translators MUST cover ALLOWED_NAMES;
//! `cargo test` enforces coverage.

use crate::bridge::types::{CanonicalCharacter, HealthTrack};
use serde_json::Value;

pub const ALLOWED_NAMES: &[&str] = &[
    "hunger",
    "humanity",
    "humanity_stains",
    "blood_potency",
    "health_superficial",
    "health_aggravated",
    "willpower_superficial",
    "willpower_aggravated",
];

/// Apply a canonical-named field to a typed CanonicalCharacter.
/// Returns Err on unknown name, wrong value type, or out-of-range integer.
pub fn apply_canonical_field(
    c: &mut CanonicalCharacter,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    match name {
        "hunger" => {
            let n = expect_u8_in_range(value, name, 0, 5)?;
            c.hunger = Some(n);
        }
        "humanity" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity = Some(n);
        }
        "humanity_stains" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity_stains = Some(n);
        }
        "blood_potency" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.blood_potency = Some(n);
        }
        "health_superficial" | "health_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.health, name, n);
        }
        "willpower_superficial" | "willpower_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.willpower, name, n);
        }
        other => return Err(format!("character/set_field: unknown field '{other}'")),
    }
    Ok(())
}

fn apply_track_field(track: &mut Option<HealthTrack>, name: &str, n: u8) {
    let t = track.get_or_insert(HealthTrack {
        max: 0,
        superficial: 0,
        aggravated: 0,
    });
    if name.ends_with("_superficial") {
        t.superficial = n;
    } else if name.ends_with("_aggravated") {
        t.aggravated = n;
    }
}

fn expect_u8_in_range(v: &Value, name: &str, lo: u8, hi: u8) -> Result<u8, String> {
    let n = v.as_u64().ok_or_else(|| {
        format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {}",
            type_label(v),
        )
    })?;
    if n > hi as u64 {
        return Err(format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {n}"
        ));
    }
    let n = n as u8;
    if n < lo {
        return Err(format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {n}"
        ));
    }
    Ok(n)
}

fn type_label(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

/// Foundry system-path mapping. Replaces the inline match in
/// `bridge/foundry/mod.rs::canonical_to_path` (delegated in Task 3).
pub fn canonical_to_foundry_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "hunger" => "system.hunger.value",
        "humanity" => "system.humanity.value",
        "humanity_stains" => "system.humanity.stains",
        "blood_potency" => "system.blood.potency",
        "health_superficial" => "system.health.superficial",
        "health_aggravated" => "system.health.aggravated",
        "willpower_superficial" => "system.willpower.superficial",
        "willpower_aggravated" => "system.willpower.aggravated",
        _ => return None,
    })
}

/// Roll20 attribute mapping. v1 returns None for every canonical name —
/// Roll20 live editing of canonical names is deferred to Phase 2.5.
/// Roll20 saved-side editing is unaffected (mutates the typed struct).
pub fn canonical_to_roll20_attr(_name: &str) -> Option<&'static str> {
    None
}
```

- [ ] **Step 4: Run tests — verify they pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields
```

Expected: PASS, all 9 tests green.

- [ ] **Step 5: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS (`cargo test`, `npm run check`, `npm run build` all green).

- [ ] **Step 6: Commit.**

```bash
git add src-tauri/src/shared/canonical_fields.rs src-tauri/src/shared/mod.rs
git commit -m "feat(shared/canonical_fields): namespace SSoT + saved-side mutator"
```

---

### Task 2: Extract `do_set_attribute` from `bridge/commands.rs`

Refactor only — no behavior change. Exposes a non-IPC entry point so the future router can call the same logic without round-tripping through `State`.

**Test discipline note:** This task does **not** add a new test — the safety net is the existing test suite (which exercises `bridge_set_attribute` indirectly via the bridge accept loop and the stub source impls). A green `./scripts/verify.sh` is the proof of correctness for this refactor. Do not invent new tests.

**Files:**
- Modify: `src-tauri/src/bridge/commands.rs`

- [ ] **Step 1: Apply the extraction.**

The current file (read via `Read` tool first) has `bridge_set_attribute` and `send_to_source` defined. Replace those two functions with the four-function shape below, and add the imports.

Add to the imports at the top of the file:

```rust
use std::sync::Arc;
use crate::bridge::BridgeState;
```

Replace the body of `bridge_set_attribute` and `send_to_source` with:

```rust
/// Inner logic shared by the Tauri command and any non-IPC caller (the new
/// character_set_field router). Operates directly on `Arc<BridgeState>` so
/// callers don't need to hold a `State<'_, BridgeConn>`.
pub(crate) async fn do_set_attribute(
    state: &Arc<BridgeState>,
    source: SourceKind,
    source_id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    let source_impl = state
        .sources
        .get(&source)
        .cloned()
        .ok_or_else(|| format!("source {} not registered", source.as_str()))?;
    let payload = source_impl
        .build_set_attribute(&source_id, &name, &value)
        .map_err(|e| format!("bridge/set_attribute: {e}"))?;
    let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    send_to_source_inner(state, source, text).await
}

/// Asks the named source to push attribute `name` = `value` for the given
/// `source_id` (Roll20 → set_attribute on a sheet; Foundry → actor.update
/// or item create depending on translation). No-op if the source isn't
/// connected.
#[tauri::command]
pub async fn bridge_set_attribute(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    source_id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    do_set_attribute(&conn.0, source, source_id, name, value).await
}

pub(crate) async fn send_to_source_inner(
    state: &Arc<BridgeState>,
    kind: SourceKind,
    text: String,
) -> Result<(), String> {
    let tx = {
        let conns = state.connections.lock().await;
        conns.get(&kind).and_then(|c| c.outbound_tx.clone())
    };
    if let Some(tx) = tx {
        tx.send(text).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(crate) async fn send_to_source(
    conn: &State<'_, BridgeConn>,
    kind: SourceKind,
    text: String,
) -> Result<(), String> {
    send_to_source_inner(&conn.0, kind, text).await
}
```

(`bridge_get_status`, `bridge_get_characters`, `bridge_refresh`, `bridge_get_source_info` are unchanged — leave them in place.)

- [ ] **Step 2: Run cargo check + tests — verify nothing breaks.**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml bridge
```

Expected: PASS (refactor is behavior-preserving; existing tests still pass).

- [ ] **Step 3: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit.**

```bash
git add src-tauri/src/bridge/commands.rs
git commit -m "refactor(bridge/commands): extract do_set_attribute for non-IPC callers"
```

---

### Task 3: Delegate Foundry `canonical_to_path` to the shared helper

Tiny refactor that makes `bridge/foundry/mod.rs` source the canonical-name → Foundry-path mapping from the shared module instead of an inline match.

**Files:**
- Modify: `src-tauri/src/bridge/foundry/mod.rs`

- [ ] **Step 1: Replace the function body.**

Find the `fn canonical_to_path(name: &str) -> String {` function (currently a `match` block) and replace its body with:

```rust
fn canonical_to_path(name: &str) -> String {
    if let Some(p) = crate::shared::canonical_fields::canonical_to_foundry_path(name) {
        return p.to_string();
    }
    if name.starts_with("system.") {
        return name.to_string();
    }
    name.to_string()
}
```

The function signature is unchanged. The passthrough branch (`other if other.starts_with("system.")`) and the final fallback (`other => other`) are preserved by the two `if`/return arms. Wire shape unchanged.

- [ ] **Step 2: Run cargo check + tests.**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry
```

Expected: PASS.

- [ ] **Step 3: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit.**

```bash
git add src-tauri/src/bridge/foundry/mod.rs
git commit -m "refactor(bridge/foundry): canonical_to_path delegates to shared/canonical_fields"
```

---

### Task 4: `db/saved_character.rs` — `db_patch_field` + `patch_saved_field` Tauri command

The genuinely new primitive: path-level patch on a saved row (vs. the existing whole-blob `update_saved_character`).

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs`

- [ ] **Step 1: Add tests first (TDD).**

Inside the existing `#[cfg(test)] mod tests { … }` block in `src-tauri/src/db/saved_character.rs`, add the following tests (after the existing tests, before the closing `}`):

```rust
#[tokio::test]
async fn patch_field_updates_canonical_and_bumps_last_updated() {
    let pool = fresh_pool().await;
    let canonical = sample_canonical();
    let id = db_save(&pool, &canonical, None).await.unwrap();

    db_patch_field(&pool, id, "hunger", &serde_json::json!(4))
        .await
        .unwrap();

    let list = db_list(&pool).await.unwrap();
    assert_eq!(list[0].canonical.hunger, Some(4));
    assert!(!list[0].last_updated_at.is_empty());
}

#[tokio::test]
async fn patch_field_missing_id_errors() {
    let pool = fresh_pool().await;
    let err = db_patch_field(&pool, 9999, "hunger", &serde_json::json!(0))
        .await
        .unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}

#[tokio::test]
async fn patch_field_unknown_name_errors() {
    let pool = fresh_pool().await;
    let canonical = sample_canonical();
    let id = db_save(&pool, &canonical, None).await.unwrap();

    let err = db_patch_field(&pool, id, "xyzzy", &serde_json::json!(0))
        .await
        .unwrap_err();
    assert!(err.contains("unknown field 'xyzzy'"), "got: {err}");
}

#[tokio::test]
async fn patch_field_type_mismatch_errors() {
    let pool = fresh_pool().await;
    let canonical = sample_canonical();
    let id = db_save(&pool, &canonical, None).await.unwrap();

    let err = db_patch_field(&pool, id, "hunger", &serde_json::json!("oops"))
        .await
        .unwrap_err();
    assert!(err.contains("expects integer"), "got: {err}");
}
```

- [ ] **Step 2: Run tests — verify they fail.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::patch_field
```

Expected: FAIL with "cannot find function `db_patch_field`".

- [ ] **Step 3: Implement.**

Add the following two functions at the end of `src-tauri/src/db/saved_character.rs`, **before** the `#[cfg(test)] mod tests` block:

```rust
async fn db_patch_field(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.patch_field: {e}"))?
        .ok_or_else(|| "db/saved_character.patch_field: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.patch_field: deserialize failed: {e}"))?;
    crate::shared::canonical_fields::apply_canonical_field(&mut canonical, name, value)?;
    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.patch_field: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, name = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(&canonical.name)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.patch_field: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.patch_field: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn patch_saved_field(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    db_patch_field(&pool.0, id, &name, &value).await
}
```

Also expose `db_patch_field` to the future router. Add `pub(crate)` to its signature so `crate::tools::character` can call it:

```rust
pub(crate) async fn db_patch_field(
```

- [ ] **Step 4: Run tests — verify they pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::saved_character
```

Expected: PASS, including the four new `patch_field` tests and all pre-existing tests.

- [ ] **Step 5: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "feat(db/saved_character): add db_patch_field + patch_saved_field command"
```

---

### Task 5: `tools/character.rs` — the router

Thin composer over Task 2's `do_set_attribute` and Task 4's `db_patch_field`.

**Files:**
- Create: `src-tauri/src/tools/character.rs`
- Modify: `src-tauri/src/tools/mod.rs`

- [ ] **Step 1: Create the file with stubs + tests (TDD).**

Create `src-tauri/src/tools/character.rs`:

```rust
//! Field-level character editing router. Composes the existing live-write
//! pipeline (`bridge::commands::do_set_attribute`) and the new saved-side
//! patcher (`db::saved_character::db_patch_field`) under an explicit
//! `WriteTarget`.
//!
//! See `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md`.

use serde::Deserialize;
use sqlx::Row;
use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::State;

use crate::bridge::types::SourceKind;
use crate::bridge::BridgeState;
use crate::shared::canonical_fields::{canonical_to_roll20_attr, ALLOWED_NAMES};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WriteTarget {
    Live,
    Saved,
    Both,
}

#[tauri::command]
pub async fn character_set_field(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    do_set_field(&db.0, &bridge.0, target, source, source_id, name, value).await
}

/// Inner implementation taking owned `Arc<BridgeState>` and `&SqlitePool` so
/// it's testable without a Tauri runtime.
pub(crate) async fn do_set_field(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    if !ALLOWED_NAMES.contains(&name.as_str()) {
        return Err(format!("character/set_field: unknown field '{name}'"));
    }

    if target != WriteTarget::Saved
        && source == SourceKind::Roll20
        && canonical_to_roll20_attr(&name).is_none()
    {
        return Err(
            "character/set_field: Roll20 live editing of canonical names not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    match target {
        WriteTarget::Saved => {
            crate::db::saved_character::db_patch_field(
                pool,
                saved_id.unwrap(),
                &name,
                &value,
            )
            .await
        }
        WriteTarget::Live => forward_live(bridge_state, source, &source_id, &name, &value).await,
        WriteTarget::Both => {
            crate::db::saved_character::db_patch_field(
                pool,
                saved_id.unwrap(),
                &name,
                &value,
            )
            .await
            .map_err(|e| format!("character/set_field: saved write failed: {e}"))?;
            forward_live(bridge_state, source, &source_id, &name, &value)
                .await
                .map_err(|e| format!("character/set_field: saved updated, live failed: {e}"))
        }
    }
}

async fn lookup_saved_id(
    pool: &SqlitePool,
    source: SourceKind,
    source_id: &str,
) -> Result<i64, String> {
    let row = sqlx::query(
        "SELECT id FROM saved_characters WHERE source = ? AND source_id = ?",
    )
    .bind(source.as_str())
    .bind(source_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("character/set_field: {e}"))?
    .ok_or_else(|| {
        format!(
            "character/set_field: no saved row for {}/{}",
            source.as_str(),
            source_id
        )
    })?;
    Ok(row.get("id"))
}

async fn forward_live(
    state: &Arc<BridgeState>,
    source: SourceKind,
    source_id: &str,
    name: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    let s = match value {
        serde_json::Value::String(s) => s.clone(),
        v => v.to_string(),
    };
    crate::bridge::commands::do_set_attribute(
        state,
        source,
        source_id.to_string(),
        name.to_string(),
        s,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::source::BridgeSource;
    use crate::bridge::types::CanonicalCharacter;
    use crate::bridge::ConnectionInfo;
    use async_trait::async_trait;
    use serde_json::Value;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    struct StubFoundrySource;

    #[async_trait]
    impl BridgeSource for StubFoundrySource {
        async fn handle_inbound(&self, _msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
            Ok(vec![])
        }
        fn build_set_attribute(
            &self,
            _source_id: &str,
            _name: &str,
            _value: &str,
        ) -> Result<Value, String> {
            Ok(serde_json::json!({"type": "stub"}))
        }
        fn build_refresh(&self) -> Value {
            serde_json::json!({"type": "refresh"})
        }
    }

    /// Builds a stub bridge state. Returns the channel receiver too — the
    /// caller MUST bind it (e.g. `let (state, _rx) = ...`) so the channel
    /// stays open for the test's lifetime. If we dropped the receiver here,
    /// `tx.send()` in `do_set_attribute` would fail with `SendError`, breaking
    /// the no-op semantics the live-write path relies on.
    fn make_bridge_state(
        connected: bool,
    ) -> (Arc<BridgeState>, Option<tokio::sync::mpsc::Receiver<String>>) {
        make_bridge_state_with_source(connected, Arc::new(StubFoundrySource))
    }

    fn make_bridge_state_with_source(
        connected: bool,
        source_impl: Arc<dyn BridgeSource>,
    ) -> (Arc<BridgeState>, Option<tokio::sync::mpsc::Receiver<String>>) {
        let mut sources: HashMap<SourceKind, Arc<dyn BridgeSource>> = HashMap::new();
        sources.insert(SourceKind::Foundry, source_impl);

        let mut connections = HashMap::new();
        let rx_opt = if connected {
            let (tx, rx) = tokio::sync::mpsc::channel::<String>(8);
            connections.insert(
                SourceKind::Foundry,
                ConnectionInfo {
                    connected: true,
                    outbound_tx: Some(tx),
                },
            );
            Some(rx)
        } else {
            None
        };

        let state = Arc::new(BridgeState {
            connections: Mutex::new(connections),
            source_info: Mutex::new(HashMap::new()),
            sources,
        });
        (state, rx_opt)
    }

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    fn sample_canonical() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "abc".to_string(),
            name: "Test".to_string(),
            controlled_by: None,
            hunger: Some(2),
            health: None,
            willpower: None,
            humanity: None,
            humanity_stains: None,
            blood_potency: None,
            raw: serde_json::json!({}),
        }
    }

    /// Stub source whose build_set_attribute always errs — used to exercise
    /// the partial-success path on Both targets.
    struct AlwaysErrSource;

    #[async_trait]
    impl BridgeSource for AlwaysErrSource {
        async fn handle_inbound(&self, _msg: Value) -> Result<Vec<CanonicalCharacter>, String> {
            Ok(vec![])
        }
        fn build_set_attribute(
            &self,
            _source_id: &str,
            _name: &str,
            _value: &str,
        ) -> Result<Value, String> {
            Err("stub forced failure".to_string())
        }
        fn build_refresh(&self) -> Value {
            serde_json::json!({"type": "refresh"})
        }
    }

    async fn seed_saved_row(pool: &SqlitePool, source_id: &str) {
        let canonical = sample_canonical();
        let canonical_json = serde_json::to_string(&canonical).unwrap();
        sqlx::query(
            "INSERT INTO saved_characters
             (source, source_id, foundry_world, name, canonical_json)
             VALUES ('foundry', ?, NULL, 'Test', ?)",
        )
        .bind(source_id)
        .bind(&canonical_json)
        .execute(pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unknown_name_returns_err_immediately() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "xyzzy".to_string(),
            serde_json::json!(0),
        )
        .await
        .unwrap_err();
        assert!(err.contains("unknown field 'xyzzy'"), "got: {err}");
    }

    #[tokio::test]
    async fn roll20_live_canonical_returns_unsupported_err() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Roll20,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(2),
        )
        .await
        .unwrap_err();
        assert!(
            err.contains("Roll20 live editing of canonical names not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn saved_target_no_row_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Saved,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(2),
        )
        .await
        .unwrap_err();
        assert!(
            err.contains("no saved row for foundry/abc"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn saved_target_happy_path_writes_db() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row(&pool, "abc").await;

        do_set_field(
            &pool,
            &state,
            WriteTarget::Saved,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(5),
        )
        .await
        .unwrap();

        let row =
            sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
                .fetch_one(&pool)
                .await
                .unwrap();
        let json: String = row.get("canonical_json");
        let updated: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.hunger, Some(5));
    }

    #[tokio::test]
    async fn live_target_disconnected_source_no_op_succeeds() {
        // do_set_attribute is no-op when the source has no outbound channel.
        // Mirrors existing bridge_set_attribute semantics.
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(false); // disconnected
        let res = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(3),
        )
        .await;
        assert!(res.is_ok(), "live no-op should be Ok, got: {:?}", res);
    }

    #[tokio::test]
    async fn both_target_saved_succeeds_then_live_succeeds() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row(&pool, "abc").await;

        do_set_field(
            &pool,
            &state,
            WriteTarget::Both,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(4),
        )
        .await
        .unwrap();

        let row = sqlx::query(
            "SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let json: String = row.get("canonical_json");
        let updated: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.hunger, Some(4));
    }

    #[tokio::test]
    async fn both_partial_success_when_live_fails() {
        // Spec §6: when target=Both, saved succeeds, live fails, we get the
        // partial-success error string AND the saved row reflects the change.
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state_with_source(true, Arc::new(AlwaysErrSource));
        seed_saved_row(&pool, "abc").await;

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Both,
            SourceKind::Foundry,
            "abc".to_string(),
            "hunger".to_string(),
            serde_json::json!(4),
        )
        .await
        .unwrap_err();

        assert!(
            err.starts_with("character/set_field: saved updated, live failed:"),
            "got: {err}"
        );

        // Saved row was still patched (saved-first ordering).
        let row = sqlx::query(
            "SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        let json: String = row.get("canonical_json");
        let updated: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(updated.hunger, Some(4));
    }
}
```

Add `pub mod character;` to `src-tauri/src/tools/mod.rs`. The current file has:

```rust
pub mod resonance;
pub mod skill_check;
pub mod export;
pub mod foundry_chat;
```

Add the new line so it reads:

```rust
pub mod resonance;
pub mod skill_check;
pub mod export;
pub mod foundry_chat;
pub mod character;
```

- [ ] **Step 2: Run tests — verify they pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml tools::character
```

Expected: PASS. All seven tests green.

Important detail: `make_bridge_state` returns `(Arc<BridgeState>, Option<Receiver<String>>)`. Each test must bind the receiver (e.g. `let (state, _rx) = make_bridge_state(true);`) so the mpsc channel stays open for the test's lifetime — dropping `_rx` would close the channel and cause `tx.send()` in `do_set_attribute` to fail.

If a test about `BridgeState` or `ConnectionInfo` field visibility fails to compile, check that those types are `pub`/`pub(crate)` in `src-tauri/src/bridge/mod.rs`. From the spec, both already are; no change needed.

- [ ] **Step 3: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit.**

```bash
git add src-tauri/src/tools/character.rs src-tauri/src/tools/mod.rs
git commit -m "feat(tools/character): add character_set_field router"
```

---

### Task 6: Register Tauri commands in `lib.rs`

Wire the two new commands into the `invoke_handler!` so the frontend can call them.

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Read the current `invoke_handler!` block.**

Use the `Read` tool on `src-tauri/src/lib.rs` to see the current command list (the block starting around line 66 with `tauri::generate_handler![`).

- [ ] **Step 2: Add `patch_saved_field` after the saved-character commands.**

Find this exact line:

```rust
            db::saved_character::delete_saved_character,
```

Insert one new line directly below it so the saved-character cluster reads:

```rust
            db::saved_character::save_character,
            db::saved_character::list_saved_characters,
            db::saved_character::update_saved_character,
            db::saved_character::delete_saved_character,
            db::saved_character::patch_saved_field,
```

- [ ] **Step 3: Add `character_set_field` to the `tools::*` cluster.**

The `tools::*` cluster currently has these entries (search for `tools::` to locate):

```rust
            tools::resonance::roll_resonance,
            tools::skill_check::roll_skill_check,
            ...
            tools::export::export_result_to_md,
            tools::foundry_chat::trigger_foundry_roll,
            tools::foundry_chat::post_foundry_chat,
```

Find this exact line:

```rust
            tools::foundry_chat::post_foundry_chat,
```

Insert one new line directly below it:

```rust
            tools::foundry_chat::post_foundry_chat,
            tools::character::character_set_field,
```

- [ ] **Step 4: Run cargo check.**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS. If a "function not found" error appears, double-check the module path matches the file structure (`tools::character::character_set_field` and `db::saved_character::patch_saved_field`).

- [ ] **Step 5: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(lib): register character_set_field + patch_saved_field commands"
```

---

### Task 7: Frontend mirror + typed wrapper

Type-safe surface so components can call the router without `invoke()`.

**Files:**
- Modify: `src/types.ts`
- Create: `src/lib/character/api.ts`

- [ ] **Step 1: Add the mirror types to `src/types.ts`.**

`src/types.ts` line 60 has:

```ts
export type SourceKind = 'roll20' | 'foundry';
```

Insert the following two type exports immediately after that line (before the empty line and the `HealthTrack` interface that follows):

```ts
export type SourceKind = 'roll20' | 'foundry';

/** Mirrors src-tauri/src/tools/character.rs::WriteTarget. */
export type WriteTarget = 'live' | 'saved' | 'both';

/**
 * Mirrors src-tauri/src/shared/canonical_fields.rs::ALLOWED_NAMES.
 * Adding a name = update both ends in the same commit.
 */
export type CanonicalFieldName =
  | 'hunger'
  | 'humanity'
  | 'humanity_stains'
  | 'blood_potency'
  | 'health_superficial'
  | 'health_aggravated'
  | 'willpower_superficial'
  | 'willpower_aggravated';
```

- [ ] **Step 2: Create the typed wrapper `src/lib/character/api.ts`.**

```ts
// Typed wrapper around character_set_field. Per CLAUDE.md, components must
// NOT call invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type { SourceKind, WriteTarget, CanonicalFieldName } from '../../types';

export type { WriteTarget, CanonicalFieldName } from '../../types';

export const characterSetField = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('character_set_field', { target, source, sourceId, name, value });

export const patchSavedField = (
  id: number,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('patch_saved_field', { id, name, value });
```

- [ ] **Step 3: Run npm check.**

```bash
npm run check
```

Expected: PASS, no TypeScript errors.

If `SourceKind` isn't already exported from `src/types.ts`, the check will report an unresolved import. Confirm `SourceKind` is exported (it should be — Plan 0/1 added it). If it's named differently (e.g., `BridgeSourceKind`), update the import accordingly.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add src/types.ts src/lib/character/api.ts
git commit -m "feat(lib/character): add typed wrapper for character_set_field"
```

---

### Task 8: Final verification + manual smoke

Closes the loop with the manual verification path from spec §7.

**Files:** none

- [ ] **Step 1: Final aggregate verification.**

```bash
./scripts/verify.sh
```

Expected: PASS — `cargo test`, `npm run check`, `npm run build` all green.

- [ ] **Step 2: Confirm test counts.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields 2>&1 | tail -3
cargo test --manifest-path src-tauri/Cargo.toml db::saved_character 2>&1 | tail -3
cargo test --manifest-path src-tauri/Cargo.toml tools::character 2>&1 | tail -3
```

Expected:
- `shared::canonical_fields`: ≥ 9 tests pass
- `db::saved_character`: ≥ 11 tests pass (7 pre-existing + 4 new)
- `tools::character`: ≥ 7 tests pass

- [ ] **Step 3: Manual smoke (optional but recommended before announcing done).**

The Phase 2 stat-editor UI (#7) hasn't shipped yet, so there's no GUI button that calls `characterSetField`. To smoke-test from a Foundry-connected dev session:

1. Start the dev app:
   ```bash
   npm run tauri dev
   ```
2. Connect the Foundry world (browser, accept cert, GM login).
3. Open browser devtools on the Foundry world page (where the bridge runs).
4. From the Tauri devtools console (Ctrl+Shift+I in the desktop app):
   ```js
   const { invoke } = window.__TAURI_INTERNALS__;
   await invoke('character_set_field', {
     target: 'live',
     source: 'foundry',
     sourceId: '<actor-id-from-Campaign-tool>',
     name: 'hunger',
     value: 3,
   });
   ```
5. Verify the live card hunger value re-renders.
6. If a saved counterpart exists, verify the drift badge appears.

Skipping the manual smoke is acceptable if the cargo tests cover the path; flag it in the PR description so the user knows.

- [ ] **Step 4: Issue closure.**

When the final commit lands, propose this commit-message footer (per CLAUDE.md GitHub conventions):

```
Closes #6
```

The user closes #6 (or merges the PR if one is opened).

---

## Dependency graph

```
Task 0 (pre-flight)
  │
  ▼
Task 1 (canonical_fields.rs)
  │
  ├──► Task 3 (Foundry delegation)        ┐
  │                                        ├── parallel-safe after Task 1
  └──► Task 4 (db_patch_field) ────────────┘
                                  │
Task 2 (do_set_attribute extract) │       ── independent of Task 1; can run anytime before Task 5
                                  │
                                  ▼
                          Task 5 (router)
                                  │
                                  ▼
                          Task 6 (lib.rs registration)
                                  │
                                  ▼
                          Task 7 (frontend mirror + wrapper)
                                  │
                                  ▼
                          Task 8 (final verify)
```

Tasks 1, 2, 3, 4 can be parallelized across worktrees (`superpowers:using-git-worktrees`). Sequential execution is fine too — each task takes ~5 minutes.

---

## Anti-scope (sub-agents must not touch these files)

| Anti-scope file/area | Why |
|---|---|
| Any `vtmtools-bridge/` JS file | No new wire variants; module unchanged |
| `src-tauri/migrations/*.sql` | No schema change |
| `src-tauri/src/db/saved_character.rs::update_saved_character` | Whole-blob path is unchanged; do not touch |
| `src-tauri/src/bridge/foundry/types.rs` | No new wire variants |
| `src-tauri/src/bridge/roll20/mod.rs` | Roll20 stays passthrough; v1 stub returns None |
| Any `.svelte` component | Phase 2 UI is #7's scope, not #6's |
| `src/store/savedCharacters.svelte.ts` | Existing store unchanged; no new methods needed |
| `docs/superpowers/specs/**` | Spec is frozen for plan execution |

---

## Verification gate summary

Per CLAUDE.md hard rule: every task ending in a commit runs `./scripts/verify.sh` first.

| Task | `cargo test` adds | `npm run check` impact | `npm run build` impact |
|---|---|---|---|
| 1 | +9 tests in `shared::canonical_fields` | none | none |
| 2 | refactor — existing tests still pass | none | none |
| 3 | refactor — existing tests still pass | none | none |
| 4 | +4 tests in `db::saved_character` | none | none |
| 5 | +7 tests in `tools::character` | none | none |
| 6 | none (registration) | none | command surface grows |
| 7 | none | mirror types must match | new module compiles |
| 8 | aggregate green | aggregate green | aggregate green |

Final command-surface count: 37 → 39 (`character_set_field`, `patch_saved_field`).

---

## Pointers

- Spec: `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md`
- Roadmap: `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 2
- ARCHITECTURE.md §4 (IPC + bridge protocol), §7 (errors), §9 (Add a Tauri command), §11 (plan conventions)
- Foundry helper roadmap: `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`
- Issue: https://github.com/Hampternt/vtmtools/issues/6
