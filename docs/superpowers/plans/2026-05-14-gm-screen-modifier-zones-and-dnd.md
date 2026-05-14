# GM Screen — Modifier Zones + Drag-and-Drop Primitive Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Per project `CLAUDE.md` workflow override, dispatch ONE implementer subagent per task with full task text + scene-setting context; do NOT also run per-task spec-compliance / code-quality reviewer subagents. Run `./scripts/verify.sh` after each implementer commits; after ALL tasks are committed, run a SINGLE `code-review:code-review` against the full branch diff.

**Goal:** Add per-modifier zones (`character` | `situational`), split the GM-screen character row into a three-box layout (active effects · character carousel · situational carousel), introduce a hard-delete button on free-bound cards, and ship a pointer-events drag-and-drop primitive (pickup-and-place model with a permission matrix) used in v1 only for same-row free-bound zone reclassification.

**Architecture:** New `zone` enum on `CharacterModifier` (default `character`), persisted via migration `0007` with a backfill on `origin_template_id IS NOT NULL` rows so existing template-applied modifiers land in the Situational box on upgrade. A new `set_modifier_zone` IPC command with a defensive binding-aware reject. Frontend `CharacterRow.svelte` body becomes a three-column flex; each carousel is the existing absolutely-positioned z-stack with its own `--cards` count and per-zone "+ Add" button. New green CSS tokens drive the visual treatment for `data-zone="situational"` cards. A new `src/lib/dnd/` module provides the source/target/action discriminated unions, a `getActionsFor(source, target) → Action[]` matrix function, a pointer-events state-machine store, and four leaf components (`DragSource`, `DropZone`, `DropMenu`, `HeldCardOverlay`).

**Tech Stack:** Rust (tauri 2 + sqlx), TypeScript (Svelte 5 runes mode + SvelteKit static SPA), SQLite, no frontend test framework (manual smoke via `./scripts/verify.sh` + dev server).

**Source spec:** `docs/superpowers/specs/2026-05-14-gm-screen-modifier-zones-and-dnd-design.md`

**Branch suggestion:** `feat/gm-screen-modifier-zones-and-dnd`

---

## Pre-flight (do once, not a task)

```bash
git checkout -b feat/gm-screen-modifier-zones-and-dnd
./scripts/verify.sh   # baseline: must be green before starting Task 1
```

If `verify.sh` is not green from the baseline, stop and fix the existing failure before starting the plan. Do not begin Task 1 against a broken base.

---

## Task 1: Migration — add `zone` column with backfill

**Files:**
- Create: `src-tauri/migrations/0007_add_modifier_zone.sql`

**Anti-scope:** Do not touch `src-tauri/src/shared/modifier.rs` or `src-tauri/src/db/modifier.rs` in this task. This task only adds the schema; the Rust read/write code lands in Task 2.

**Depends on:** none.

**Invariants cited:** `ARCHITECTURE.md` §6 (DB schema lives in `src-tauri/migrations/`); spec §"DB migration".

**Tests:** required — Rust unit tests in `db/modifier.rs` (and `shared/modifier.rs`) run against an in-memory pool that applies all migrations via `sqlx::migrate!("./migrations")`. The migration is exercised every time `cargo test` runs.

- [ ] **Step 1: Create the migration file**

Create `src-tauri/migrations/0007_add_modifier_zone.sql` with this exact content:

```sql
-- Adds the `zone` column to character_modifiers. Two values:
--   'character'   = merits/flaws/items/character-flavored modifiers (default)
--   'situational' = scene/world modifiers (slippery, dark, cursed, etc.)
--
-- Backfill: any existing row with origin_template_id IS NOT NULL came from a
-- status-template application — those are semantically situational. Without
-- the backfill, upgrade users would have to manually drag every template-
-- applied card from Character into Situational on first run after upgrade.

ALTER TABLE character_modifiers
    ADD COLUMN zone TEXT NOT NULL DEFAULT 'character'
    CHECK(zone IN ('character', 'situational'));

UPDATE character_modifiers
   SET zone = 'situational'
 WHERE origin_template_id IS NOT NULL;
```

- [ ] **Step 2: Run verify.sh — migrations execute clean, no new code yet**

Run: `./scripts/verify.sh`
Expected: green. `cargo test` runs all existing modifier tests against an in-memory pool that now applies `0007`. Nothing should regress — the new column is purely additive.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/0007_add_modifier_zone.sql
git commit -m "feat(db): add modifier zone column with origin-template backfill

New column zone TEXT NOT NULL DEFAULT 'character' on character_modifiers.
Backfill: rows with origin_template_id IS NOT NULL flip to 'situational'
so existing template-applied modifiers land in the right box after upgrade."
```

---

## Task 2: Rust — `ModifierZone` enum, struct fields, DB read/write, `db_set_zone`, unit tests

**Files:**
- Modify: `src-tauri/src/shared/modifier.rs` (add enum, add fields on `CharacterModifier` + `NewCharacterModifier`, add round-trip tests)
- Modify: `src-tauri/src/db/modifier.rs` (read/write zone in all CRUD paths; new `db_set_zone`; advantage upsert hard-codes zone; new unit tests)

**Anti-scope:** Do not touch `src-tauri/src/lib.rs` in this task (IPC command registration lands in Task 3). Do not touch `src/types.ts` or any TypeScript (lands in Task 4).

**Depends on:** Task 1 (column must exist for `cargo test` to pass).

**Invariants cited:** `ARCHITECTURE.md` §2 (`CharacterModifier` domain shape — extending, not breaking), §5 (only `db/*` talks to SQLite), §7 (error prefix idiom `db/modifier.set_zone: …`); spec §"Domain shape changes".

**Tests: required** (5 new Rust unit tests per spec).

- [ ] **Step 1: Add `ModifierZone` enum to `shared/modifier.rs`**

Open `src-tauri/src/shared/modifier.rs`. Just above the `CharacterModifier` struct, add:

```rust
/// Per-modifier zone. Drives box placement in the GM screen three-box layout
/// and the green visual treatment for situational cards. Advantage-bound
/// modifiers are zone-locked to `Character` (enforced by `db_set_zone` +
/// `db_materialize_advantage`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModifierZone {
    #[default]
    Character,
    Situational,
}
```

- [ ] **Step 2: Add `zone` field to `CharacterModifier`**

In the same file, in the `CharacterModifier` struct (currently ending around line 28 with `pub updated_at: String,`), add the `zone` field just before `created_at`:

```rust
    #[serde(default)]
    pub zone: ModifierZone,
    pub created_at: String,
    pub updated_at: String,
}
```

`#[serde(default)]` consults the `Default` impl on `ModifierZone` (→ `Character`), matching the existing tolerance pattern for legacy/pre-migration payloads (see `foundry_captured_labels` handling above in the same file).

- [ ] **Step 3: Add `zone` field to `NewCharacterModifier`**

In the same file, in the `NewCharacterModifier` struct, add at the end:

```rust
    #[serde(default)]
    pub zone: ModifierZone,
}
```

- [ ] **Step 4: Add JSON round-trip tests in `shared/modifier.rs`**

In the same file, inside the existing `#[cfg(test)] mod tests` block (after `new_character_modifier_captured_labels_round_trip_json`), append two tests:

```rust
    #[test]
    fn character_modifier_zone_round_trips_json() {
        let json = serde_json::json!({
            "id": 9,
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Slippery Floor",
            "description": "",
            "effects": [],
            "binding": { "kind": "free" },
            "tags": [],
            "isActive": true,
            "isHidden": false,
            "originTemplateId": null,
            "foundryCapturedLabels": [],
            "zone": "situational",
            "createdAt": "2026-05-14 00:00:00",
            "updatedAt": "2026-05-14 00:00:00",
        });
        let m: CharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert_eq!(m.zone, ModifierZone::Situational);
        let round_trip = serde_json::to_value(&m).expect("serialize");
        assert_eq!(round_trip["zone"], serde_json::json!("situational"));
    }

    #[test]
    fn character_modifier_missing_zone_defaults_to_character() {
        // Legacy rows from before the 0007 migration / IPC payloads omitting the field.
        let json = serde_json::json!({
            "id": 1,
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Legacy",
            "description": "",
            "effects": [],
            "binding": { "kind": "free" },
            "tags": [],
            "isActive": false,
            "isHidden": false,
            "originTemplateId": null,
            "foundryCapturedLabels": [],
            "createdAt": "2026-05-14 00:00:00",
            "updatedAt": "2026-05-14 00:00:00",
        });
        let m: CharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert_eq!(m.zone, ModifierZone::Character);
    }
```

- [ ] **Step 5: Update `db/modifier.rs` imports**

Open `src-tauri/src/db/modifier.rs`. At the top, extend the import line that pulls `CharacterModifier, ModifierBinding, …` to also include `ModifierZone`:

```rust
use crate::shared::modifier::{
    CharacterModifier, ModifierBinding, ModifierEffect, ModifierKind, ModifierZone,
};
```

Also add a small helper near `source_to_str` for zone serialization:

```rust
fn zone_to_str(z: &ModifierZone) -> &'static str {
    match z {
        ModifierZone::Character => "character",
        ModifierZone::Situational => "situational",
    }
}

fn str_to_zone(s: &str) -> ModifierZone {
    match s {
        "situational" => ModifierZone::Situational,
        _ => ModifierZone::Character,   // 'character' or unknown → default
    }
}
```

- [ ] **Step 6: Read `zone` in `row_to_modifier`**

In `row_to_modifier` (around line 22), after the captured-labels parsing and before the `Ok(CharacterModifier { … })` literal, read the zone:

```rust
    let zone_str: String = r.try_get("zone").unwrap_or_else(|_| "character".to_string());
    let zone = str_to_zone(&zone_str);
```

Then inside the struct literal, just before `created_at`, add:

```rust
        zone,
        created_at: r.get("created_at"),
```

- [ ] **Step 7: SELECT `zone` in `db_list`, `db_list_all`, `db_get`**

For each of the three queries — `db_list` (line 61), `db_list_all` (line 78), `db_get` (line 158) — add `zone` to the SELECT column list. Example for `db_list`:

```rust
    let rows = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, foundry_captured_labels_json, zone,
                created_at, updated_at
         FROM character_modifiers
         WHERE source = ? AND source_id = ?
         ORDER BY id ASC"
    )
```

Apply the same `, zone,` insertion (between `foundry_captured_labels_json` and `created_at`) in the other two SELECTs.

- [ ] **Step 8: Insert `zone` in `db_add`**

In `db_add` (line 109), update the INSERT to include the zone column and bind the input's zone:

```rust
    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          origin_template_id, foundry_captured_labels_json, zone)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&input.source))
    .bind(&input.source_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&binding_json)
    .bind(&tags_json)
    .bind(input.origin_template_id)
    .bind(&captured_labels_json)
    .bind(zone_to_str(&input.zone))
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.add: {e}"))?;
```

- [ ] **Step 9: Hard-code `zone='character'` in `db_materialize_advantage`**

In `db_materialize_advantage` (line 306), update the INSERT to explicitly write `zone = 'character'`:

```rust
    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          foundry_captured_labels_json, zone)
         VALUES (?, ?, ?, ?, '[]', ?, '[]', '[]', 'character')"
    )
```

(Hard-coding is preferred over relying on the column DEFAULT because the contract is then explicit and survives future DEFAULT changes.)

- [ ] **Step 10: Add `db_set_zone` with binding-aware reject**

After the existing `db_set_hidden` function (line 270) and before the `#[tauri::command] set_modifier_hidden` registration, insert:

```rust
/// Update the zone classification for a modifier row. Returns the updated row.
///
/// Defensive: rejects with a stable-prefix error if the target row's binding
/// is Advantage — advantage-bound modifiers are zone-locked to Character
/// because the box they live in is tied to live Foundry merit/flaw state.
/// The UI matrix (src/lib/dnd/actions.ts) also prevents the call from being
/// issued, but two layers of enforcement is the project's pattern for
/// binding-rule invariants (cf. db_delete_by_advantage_binding).
pub(crate) async fn db_set_zone(
    pool: &SqlitePool,
    id: i64,
    zone: ModifierZone,
) -> Result<CharacterModifier, String> {
    let current = db_get(pool, id).await
        .map_err(|e| if e.contains("not found") {
            "db/modifier.set_zone: not found".to_string()
        } else {
            format!("db/modifier.set_zone: {e}")
        })?;

    if matches!(current.binding, ModifierBinding::Advantage { .. }) {
        return Err("db/modifier.set_zone: cannot reclassify advantage-bound modifier".to_string());
    }

    let result = sqlx::query(
        "UPDATE character_modifiers
            SET zone = ?, updated_at = datetime('now')
          WHERE id = ?"
    )
    .bind(zone_to_str(&zone))
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.set_zone: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/modifier.set_zone: not found".to_string());
    }
    db_get(pool, id).await
}
```

(The Tauri command wrapper for this lands in Task 3 — keeping db-layer and IPC-layer changes in separate commits.)

- [ ] **Step 11: Add db-layer unit tests**

Open `src-tauri/src/db/modifier.rs`. Inside the existing `#[cfg(test)] mod tests` block, append three tests near the end of the block (before the closing `}`):

```rust
    #[tokio::test]
    async fn db_set_zone_updates_and_returns_row() {
        let pool = fresh_pool().await;
        let m = db_add(&pool, NewCharacterModifier {
            source: SourceKind::Foundry,
            source_id: "actor-1".into(),
            name: "Slippery".into(),
            description: "".into(),
            effects: vec![],
            binding: ModifierBinding::Free,
            tags: vec![],
            origin_template_id: None,
            foundry_captured_labels: vec![],
            zone: ModifierZone::Character,
        }).await.unwrap();
        assert_eq!(m.zone, ModifierZone::Character);

        let updated = db_set_zone(&pool, m.id, ModifierZone::Situational).await.unwrap();
        assert_eq!(updated.zone, ModifierZone::Situational);
        assert_eq!(updated.id, m.id);
    }

    #[tokio::test]
    async fn db_set_zone_rejects_advantage_binding() {
        let pool = fresh_pool().await;
        let m = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "actor-1", "merit-xyz", "Beautiful", "",
        ).await.unwrap();
        assert_eq!(m.zone, ModifierZone::Character);

        let err = db_set_zone(&pool, m.id, ModifierZone::Situational).await.unwrap_err();
        assert!(err.contains("cannot reclassify advantage-bound"),
            "expected advantage rejection, got: {err}");
    }

    #[tokio::test]
    async fn db_materialize_advantage_locks_zone_to_character() {
        let pool = fresh_pool().await;
        let m = db_materialize_advantage(
            &pool, &SourceKind::Foundry, "actor-1", "merit-xyz", "Resilience", "",
        ).await.unwrap();
        assert_eq!(m.zone, ModifierZone::Character);
    }
```

- [ ] **Step 12: Verify all tests compile and pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml modifier`
Expected: all modifier-related tests pass, including the 5 new tests (2 in `shared/modifier.rs`, 3 in `db/modifier.rs`).

If any existing test fails: re-check that `zone` was added to the right place in `row_to_modifier`'s struct literal (between `foundry_captured_labels` and `created_at`).

- [ ] **Step 13: Aggregate verification**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 14: Commit**

```bash
git add src-tauri/src/shared/modifier.rs src-tauri/src/db/modifier.rs
git commit -m "feat(modifiers): zone enum, db read/write, set_zone with advantage reject

ModifierZone enum (Character | Situational) with Default = Character.
CharacterModifier and NewCharacterModifier gain a zone field with
serde(default). db_list/db_list_all/db_get/db_add updated to read+write
the column. db_materialize_advantage hard-codes zone='character'.
New db_set_zone rejects with a stable-prefix error when the target row
has an Advantage binding (defensive — UI matrix also prevents the call).
5 new Rust unit tests cover JSON round-trip, missing-field default,
set_zone success, set_zone advantage rejection, and the materialize
zone lock."
```

---

## Task 3: IPC command `set_modifier_zone` + lib.rs registration

**Files:**
- Modify: `src-tauri/src/db/modifier.rs` (add `#[tauri::command] set_modifier_zone`)
- Modify: `src-tauri/src/lib.rs` (register the new command)

**Anti-scope:** Do not touch any TypeScript (Task 4). Do not modify any other db/* function in this task.

**Depends on:** Task 2 (`db_set_zone` must exist).

**Invariants cited:** `ARCHITECTURE.md` §4 (Tauri IPC commands inventory; new command registered in `invoke_handler` in `lib.rs`); §7 (error prefix idiom).

**Tests:** none new — `db_set_zone` is already covered by Task 2; the `#[tauri::command]` wrapper is a one-line shim.

- [ ] **Step 1: Add the Tauri command in `db/modifier.rs`**

In `src-tauri/src/db/modifier.rs`, immediately after the existing `set_modifier_hidden` command (around line 294), add:

```rust
#[tauri::command]
pub async fn set_modifier_zone(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    zone: ModifierZone,
) -> Result<CharacterModifier, String> {
    db_set_zone(&pool.0, id, zone).await
}
```

- [ ] **Step 2: Register the command in `lib.rs`**

Open `src-tauri/src/lib.rs`. Find the modifier command block (around lines 111-118). Add a new line just after `db::modifier::set_modifier_hidden,`:

```rust
            db::modifier::list_character_modifiers,
            db::modifier::list_all_character_modifiers,
            db::modifier::add_character_modifier,
            db::modifier::update_character_modifier,
            db::modifier::delete_character_modifier,
            db::modifier::set_modifier_active,
            db::modifier::set_modifier_hidden,
            db::modifier::set_modifier_zone,
            db::modifier::materialize_advantage_modifier,
```

- [ ] **Step 3: Verify**

Run: `./scripts/verify.sh`
Expected: green. `cargo check` confirms the command compiles and is registered. `npm run check` is unaffected (TS mirror lands in Task 4).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/modifier.rs src-tauri/src/lib.rs
git commit -m "feat(modifiers): expose set_modifier_zone IPC command

Tauri-command shim over db_set_zone; registered in lib.rs invoke_handler
between set_modifier_hidden and materialize_advantage_modifier."
```

---

## Task 4: TypeScript mirror + typed wrapper + store action

**Files:**
- Modify: `src/types.ts` (add `ModifierZone` type alias; add `zone` field to `CharacterModifier` interface and `NewCharacterModifierInput` interface)
- Modify: `src/lib/modifiers/api.ts` (add `setModifierZone` wrapper)
- Modify: `src/store/modifiers.svelte.ts` (add `setZone` store action)

**Anti-scope:** Do not touch any `.svelte` component in this task — UI consumers come in Tasks 6-7. Do not touch `src/lib/dnd/` (lands in Tasks 8-10).

**Depends on:** Task 3 (`set_modifier_zone` must be registered).

**Invariants cited:** `ARCHITECTURE.md` §2 (TS mirror in `src/types.ts`); §4 (frontend never calls `invoke` directly — typed wrapper in `src/lib/**/api.ts`).

**Tests:** none new — type-checked by `tsc` via `npm run check`.

- [ ] **Step 1: Add `ModifierZone` type in `src/types.ts`**

Open `src/types.ts`. Just above `export type ModifierKind = 'pool' | …` (around line 227), add:

```ts
export type ModifierZone = 'character' | 'situational';
```

- [ ] **Step 2: Add `zone` field to `CharacterModifier` and `NewCharacterModifierInput`**

In the same file, in the `CharacterModifier` interface (line 245), add `zone` just before `createdAt`:

```ts
export interface CharacterModifier {
  id: number;
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  isActive: boolean;
  isHidden: boolean;
  originTemplateId: number | null;
  foundryCapturedLabels: string[];
  zone: ModifierZone;
  createdAt: string;
  updatedAt: string;
}
```

In the same file, in the `NewCharacterModifierInput` interface (line 269), add `zone` at the end (before the closing brace):

```ts
export interface NewCharacterModifierInput {
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  originTemplateId: number | null;
  foundryCapturedLabels: string[];
  zone: ModifierZone;
}
```

- [ ] **Step 3: Add `setModifierZone` to the typed API wrapper**

Open `src/lib/modifiers/api.ts`. Add `ModifierZone` to the type-only import list at the top:

```ts
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  PushReport,
  SourceKind,
  StatusTemplate,
  NewStatusTemplateInput,
  StatusTemplatePatchInput,
  ModifierZone,
} from '../../types';
```

Then, immediately after the existing `setModifierHidden` function (around line 43), add:

```ts
export function setModifierZone(id: number, zone: ModifierZone): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('set_modifier_zone', { id, zone });
}
```

- [ ] **Step 4: Add `setZone` to the modifiers store**

Open `src/store/modifiers.svelte.ts`. Add `setModifierZone` to the import block (around line 16):

```ts
import {
  listAllCharacterModifiers,
  addCharacterModifier,
  updateCharacterModifier,
  deleteCharacterModifier,
  setModifierActive,
  setModifierHidden,
  setModifierZone,
  materializeAdvantageModifier,
  pushToFoundry as apiPushToFoundry,
} from '$lib/modifiers/api';
```

Add `ModifierZone` to the type-only import block:

```ts
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  PushReport,
  SourceKind,
  ModifierZone,
} from '../types';
```

Then, in the `modifiers` exported object, after the existing `setHidden` method (around line 113), add:

```ts
  async setZone(id: number, zone: ModifierZone): Promise<void> {
    const row = await setModifierZone(id, zone);
    mergeRow(row);
  },
```

- [ ] **Step 5: Verify**

Run: `./scripts/verify.sh`
Expected: green. `npm run check` confirms TypeScript types align.

- [ ] **Step 6: Commit**

```bash
git add src/types.ts src/lib/modifiers/api.ts src/store/modifiers.svelte.ts
git commit -m "feat(modifiers): TS mirror, typed wrapper, store action for zone

ModifierZone type ('character' | 'situational'). CharacterModifier and
NewCharacterModifierInput gain a zone field. New setModifierZone typed
wrapper + modifiers.setZone store action mirror the Rust IPC contract."
```

---

## Task 5: CSS tokens for situational green family

**Files:**
- Modify: `src/routes/+layout.svelte` (add `:global(:root)` tokens)

**Anti-scope:** Do not modify any `.svelte` component using these tokens in this task — that lands in Task 6.

**Depends on:** none (independent of the type chain; can be done before or after Task 4).

**Invariants cited:** `ARCHITECTURE.md` §6 (CSS color tokens defined in `:global(:root)` in `src/routes/+layout.svelte`).

**Tests:** none — purely additive token definitions; visual smoke happens in Task 6.

- [ ] **Step 1: Add the new tokens**

Open `src/routes/+layout.svelte`. Find the `:global(:root)` block (around line 77). After the existing accent tokens (`--accent-amber` line, ~line 102), add:

```css
    /* Situational modifier zone — green family. Distinct from the red/amber
       accent palette so situational cards read as semantically different
       at a glance. New tokens introduced for the GM-screen modifier-zone
       split (spec 2026-05-14). */
    --accent-situational:        #4a8a4a;
    --accent-situational-bright: #6ab26a;
    --bg-situational-card:       #182218;
    --border-situational:        #3d6a3d;
```

- [ ] **Step 2: Verify**

Run: `./scripts/verify.sh`
Expected: green. The tokens are defined but not yet consumed; no rendering change.

- [ ] **Step 3: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "feat(theme): add situational green CSS tokens

--accent-situational, --accent-situational-bright, --bg-situational-card,
--border-situational. Consumed by ModifierCard's data-zone='situational'
treatment in the next commit."
```

---

## Task 6: `ModifierCard` — restructure (body/foot siblings), data-zone styles, "Situational" chip, trash button

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte`

**Anti-scope:** Do not modify `CharacterRow.svelte` in this task (carousel split lands in Task 7). Do not introduce DnD machinery (Tasks 8-10).

**Depends on:** Task 4 (TS `zone` field; the card reads `modifier.zone`), Task 5 (CSS tokens).

**Invariants cited:** spec §"Visual treatment", §"Delete UX", §"DnD state machine" (`.card-body` is sibling of `.foot` per the pickup-target structural restructure).

**Tests:** none new (no frontend test framework, per `ARCHITECTURE.md` §10). Manual smoke happens at end-of-task and again in Task 7.

- [ ] **Step 1: Add the `onDelete` prop**

Open `src/lib/components/gm-screen/ModifierCard.svelte`. In the `interface Props` block (around line 4), add a new optional prop just after `onHide`:

```ts
    onHide: () => void;
    /**
     * Hard-delete handler for free-bound cards. Distinct from onHide — when
     * present, the card renders a 🗑 trash button next to ×. Caller is
     * responsible for the confirm() dialog before invoking. Undefined for
     * advantage-bound cards (their lifecycle is owned by the live Foundry
     * data, not the GM).
     */
    onDelete?: () => void;
```

And add `onDelete` to the destructuring assignment (around line 59):

```ts
    onToggleActive, onOpenEditor, onHide, onDelete,
    originTemplateName = null,
```

- [ ] **Step 2: Restructure the card's outer markup into `.card-body` + `.foot` siblings**

The current `<div class="modifier-card" …>` directly contains `.head`, `.bonuses`, `.effects`, `.tags`, and `.foot` as siblings. The DnD primitive needs `.card-body` (everything pickup-sensitive) to be one element, and `.foot` (the buttons) to be a sibling, so that the future `DragSource` wraps `.card-body` only without ambiguity.

Replace the entire markup block (from `<div class="modifier-card" …>` down through the close of `</div>` for the card) with this exact structure:

```svelte
<div
  class="modifier-card"
  data-active={modifier.isActive ? 'true' : 'false'}
  data-hidden={modifier.isHidden ? 'true' : 'false'}
  data-zone={modifier.zone}
>
  <div class="card-body">
    {#if modifier.zone === 'situational'}
      <span class="zone-chip" aria-label="Situational modifier">Situational</span>
    {/if}
    <div class="head">
      <span class="name" title={modifier.name}>
        {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}{#if showOverride}<span class="override-mark" title="Saved local override — this card's data comes from your saved copy, which supersedes the live Foundry read-through">*</span>{/if}
      </span>
      <button
        bind:this={cogEl}
        class="cog"
        title="Edit effects"
        onclick={() => cogEl && onOpenEditor(cogEl)}
      >⚙</button>
    </div>
    {#if originTemplateName}
      <p class="origin">from "{originTemplateName}"</p>
    {/if}
    {#if bonuses.length > 0}
      <div class="bonuses">
        {#each bonuses as b}
          <p class="bonus" title={`${summarizeBonus(b)}${b.source ? ' — ' + b.source : ''}`}>
            <span class="bonus-value">{summarizeBonus(b)}</span>
            {#if b.source}<span class="bonus-source">{b.source}</span>{/if}
          </p>
        {/each}
      </div>
    {/if}
    {#if conditionalsSkipped.length > 0}
      <p
        class="conditionals-badge"
        title={conditionalsSkipped
          .map(b => `${b.source ?? '(unnamed)'} — ${b.activeWhen?.check ?? '?'}`)
          .join('\n')}
      >
        ({conditionalsSkipped.length} conditional{conditionalsSkipped.length === 1 ? '' : 's'})
      </p>
    {/if}
    <div class="effects">
      {#if modifier.effects.length === 0}
        <p class="no-effect">(no effect)</p>
      {:else}
        {#each modifier.effects as e}
          <p class="effect" title={summarize(e)}>{summarize(e)}</p>
        {/each}
      {/if}
    </div>
    {#if modifier.tags.length > 0}
      <div class="tags">
        {#each modifier.tags as t}<span class="tag">#{t}</span>{/each}
      </div>
    {/if}
  </div>
  <div class="foot">
    <button
      class="toggle"
      class:on={modifier.isActive}
      onclick={onToggleActive}
    >{modifier.isActive ? 'ON' : 'OFF'}</button>
    {#if onSaveAsOverride}
      <button
        class="save-override"
        title="Snapshot the live Foundry bonuses into a saved local override"
        aria-label="Save as local override"
        onclick={onSaveAsOverride}
      >💾</button>
    {/if}
    {#if canPush}
      <button
        class="push"
        title="Push these effects to the merit on Foundry"
        aria-label="Push effects to Foundry"
        onclick={onPush}
      >↑</button>
    {/if}
    {#if canReset}
      <button
        class="reset"
        title="Reset card — drops local effects/paths/tags. Foundry bonuses unaffected."
        aria-label="Reset card"
        onclick={onReset}
      >↺</button>
    {/if}
    {#if onDelete}
      <button
        class="delete"
        title="Delete card permanently"
        aria-label="Delete card permanently"
        onclick={onDelete}
      >🗑</button>
    {/if}
    <button
      class="hide"
      title={modifier.isHidden ? 'Show card again' : 'Hide card'}
      aria-label={modifier.isHidden ? 'Show card again' : 'Hide card'}
      onclick={onHide}
    >{modifier.isHidden ? '+' : '×'}</button>
  </div>
</div>
```

Key changes:
- New `<div class="card-body">` wraps `head`, `origin`, `bonuses`, `conditionals-badge`, `effects`, `tags`.
- `<div class="foot">` stays as a sibling of `.card-body` (was already a sibling of those elements, just grouped under the parent — now structurally explicit).
- New `data-zone={modifier.zone}` on the outer card.
- New `<span class="zone-chip">` rendered first inside `.card-body` when zone is situational.
- New `{#if onDelete}<button class="delete">🗑` rendered before `.hide` in `.foot`.

- [ ] **Step 3: Add the new CSS rules**

In the same file, in the `<style>` block, append these rules (near the end of the file, before the `@media (prefers-reduced-motion: reduce)` block):

```css
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    flex: 1;
    min-width: 0;
  }

  .modifier-card[data-zone="situational"] {
    background: var(--bg-situational-card);
    border-color: var(--border-situational);
  }
  .modifier-card[data-zone="situational"][data-active="true"] {
    border-color: var(--accent-situational-bright);
    background: var(--bg-situational-card);
  }

  .zone-chip {
    align-self: flex-start;
    font-size: 0.55rem;
    padding: 0.05rem 0.4rem;
    border-radius: 3px;
    background: var(--accent-situational);
    color: #d8f0d8;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    line-height: 1.3;
  }

  .delete {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease, color 120ms ease;
  }
  .modifier-card:hover .delete,
  .delete:focus { opacity: 1; }
  .delete:hover { color: var(--accent-amber); }
```

The `.card-body { flex: 1 }` keeps the foot pinned to the bottom of the 8rem fixed-height card (the existing `.modifier-card` is already `display: flex; flex-direction: column`).

- [ ] **Step 4: Verify (type-check and visual smoke)**

Run: `./scripts/verify.sh`
Expected: green.

Then start the dev server (`npm run tauri dev` — or `npm run dev` if you have the bridge mocked) and:

1. Open the GM Screen tool.
2. The existing cards render unchanged (zone defaults to `character`, no chip, no green theme).
3. Cog / toggle / push / reset / hide buttons still work — confirm none broke after the restructure.

The `onDelete` prop is wired but not yet supplied by `CharacterRow.svelte` — the trash button does not appear yet. That comes in Task 7.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "feat(gm-screen): card body/foot restructure, zone styles, delete prop

ModifierCard's body content (head/bonuses/effects/tags) now lives inside
a single .card-body div sibling of .foot — preparation for the DnD
primitive (DragSource will wrap .card-body only).

New data-zone attribute reads modifier.zone. Situational cards get the
green bg/border treatment from the just-added --accent-situational tokens
plus a 'Situational' pill chip in the body head.

New onDelete optional prop renders a 🗑 trash button (free-bound only —
CharacterRow wires it next commit). × hide stays alongside."
```

---

## Task 7: `CharacterRow` — split into two zone carousels + per-zone "+ Add" + `StatusPaletteDock` situational default + wire delete

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`
- Modify: `src/lib/components/gm-screen/StatusPaletteDock.svelte`

**Anti-scope:** Do not touch `GmScreen.svelte` in this task (DnD wiring lands in Task 10). Do not modify `ModifierCard.svelte` further. Do not introduce any DnD-store imports yet.

**Depends on:** Tasks 4 (TS `zone` field; store has `setZone`), 5 (CSS tokens), 6 (card has `onDelete` prop).

**Invariants cited:** spec §"Layout", §"Decisions on agent's recommendation" (template apply defaults `zone='situational'`); `ARCHITECTURE.md` §6 (`sibling-index()` works because each carousel is a separate DOM parent).

**Tests:** none new (no frontend test framework). Manual smoke at end-of-task is mandatory.

- [ ] **Step 1: Update the import block in `CharacterRow.svelte`**

Open `src/lib/components/gm-screen/CharacterRow.svelte`. Add `ModifierZone` to the type-only import block at the top:

```ts
  import type {
    BridgeCharacter, CharacterModifier, ModifierEffect, FoundryItem, FoundryItemBonus,
    ModifierZone,
  } from '../../../types';
```

- [ ] **Step 2: Update `addFreeModifier` to take a zone parameter**

Find `async function addFreeModifier()` (around line 343). Replace it with:

```ts
  async function addFreeModifier(zone: ModifierZone): Promise<void> {
    await modifiers.add({
      source: character.source,
      sourceId: character.source_id,
      name: 'New modifier',
      description: '',
      effects: [],
      binding: { kind: 'free' },
      tags: [],
      originTemplateId: null,
      foundryCapturedLabels: [],
      zone,
    });
  }
```

- [ ] **Step 3: Add a hard-delete handler**

Below `addFreeModifier`, add a new helper:

```ts
  async function handleHardDelete(mod: CharacterModifier): Promise<void> {
    const ok = confirm(`Delete "${mod.name}" permanently? This cannot be undone.`);
    if (!ok) return;
    await modifiers.delete(mod.id);
  }
```

- [ ] **Step 4: Split `visibleCards` into two zone-filtered derivations**

The current `visibleCards` derivation (around line 184) filters the full mixed list. Replace it with two zone-scoped derivations. Find the `let visibleCards = $derived(…)` block and replace with:

```ts
  // Shared filter/sort logic for both zones. visibleCards becomes two derivations.
  function filterAndSort(entries: CardEntry[]): CardEntry[] {
    return entries
      .filter(e => passesTagFilter(e) && passesHiddenFilter(e))
      .sort((a, b) => {
        const [ak, an] = sortKey(a);
        const [bk, bn] = sortKey(b);
        if (ak !== bk) return ak - bk;
        return an < bn ? -1 : an > bn ? 1 : 0;
      });
  }

  // Zone of a CardEntry: virtual cards are always character-zone (they derive
  // from a Foundry advantage item which is zone-locked to 'character').
  function entryZone(e: CardEntry): ModifierZone {
    return e.kind === 'virtual' ? 'character' : e.mod.zone;
  }

  let characterCards = $derived(filterAndSort(cardEntries.filter(e => entryZone(e) === 'character')));
  let situationalCards = $derived(filterAndSort(cardEntries.filter(e => entryZone(e) === 'situational')));
```

- [ ] **Step 5: Replace the single `.modifier-row` with two zone carousels**

Find the existing `<div class="modifier-row" style="--cards: …">…</div>` block inside `.row-body` (around lines 401-453). Replace **the entire `.modifier-row` div** with this two-carousel structure:

```svelte
  <div class="zone-stack">
    <div class="zone-column" data-zone="character">
      <div class="zone-label">Character</div>
      <div
        class="modifier-row"
        style="--cards: {characterCards.length};"
      >
        {#each characterCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
          {@render renderCard(entry)}
        {/each}
        <button class="add-modifier" onclick={() => addFreeModifier('character')}>+ Add modifier</button>
      </div>
    </div>
    <div class="zone-column" data-zone="situational">
      <div class="zone-label">Situational</div>
      <div
        class="modifier-row"
        style="--cards: {situationalCards.length};"
      >
        {#each situationalCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
          {@render renderCard(entry)}
        {/each}
        <button class="add-modifier" onclick={() => addFreeModifier('situational')}>+ Add modifier</button>
      </div>
    </div>
  </div>
```

- [ ] **Step 6: Extract `renderCard` as a Svelte snippet**

The two carousels share the same `<ModifierCard>` props mapping. To avoid duplication, declare a `{#snippet}` just above the markup (Svelte 5 runes-mode feature). Find the line just before `<section class="row" …>` and inside the existing `<script>` close + just before the markup, add this snippet block right after the closing `</script>` and BEFORE `<section …>`:

```svelte
{#snippet renderCard(entry: CardEntry)}
  <ModifierCard
    modifier={entry.kind === 'virtual'
      ? {
          id: 0,
          source: character.source,
          sourceId: character.source_id,
          name: entry.virt.name,
          description: entry.virt.description,
          effects: [],
          binding: { kind: 'advantage', item_id: entry.virt.item._id },
          tags: [],
          isActive: false,
          isHidden: false,
          originTemplateId: null,
          foundryCapturedLabels: [],
          zone: 'character',
          createdAt: '',
          updatedAt: '',
        }
      : entry.mod}
    isVirtual={entry.kind === 'virtual'}
    bonuses={entry.kind === 'virtual'
      ? bonusesFor(entry.virt.item._id)
      : entry.mod.binding.kind === 'advantage'
        ? bonusesFor(entry.mod.binding.item_id)
        : []}
    conditionalsSkipped={entry.kind === 'virtual'
      ? conditionalsFor(entry.virt.item._id)
      : entry.mod.binding.kind === 'advantage'
        ? conditionalsFor(entry.mod.binding.item_id)
        : []}
    onToggleActive={() => handleToggleActive(entry)}
    onHide={() => handleHide(entry)}
    onOpenEditor={(anchor) => openEditor(entry, anchor)}
    canPush={canPushFor(entry)}
    onPush={() => handlePush(entry)}
    canReset={canResetFor(entry)}
    onReset={() => handleReset(entry)}
    originTemplateName={entry.kind === 'materialized' && entry.mod.originTemplateId != null
      ? (statusTemplates.byId(entry.mod.originTemplateId)?.name ?? null)
      : null}
    showOverride={entry.kind === 'materialized' ? isSavedOverride(entry.mod) : false}
    onSaveAsOverride={entry.kind === 'virtual'
      ? () => saveAsOverride(entry.virt).catch(err => console.error('[gm-screen] save-as-override failed:', err))
      : undefined}
    onDelete={entry.kind === 'materialized' && entry.mod.binding.kind === 'free'
      ? () => handleHardDelete(entry.mod)
      : undefined}
  />
{/snippet}
```

The single new prop wired here is `onDelete` (line near the end). It is `undefined` for virtual cards and advantage-bound materialized cards; it is a function for free-bound materialized cards. This is what makes the 🗑 button only appear on free-bound cards.

- [ ] **Step 7: Add CSS for the zone-stack and zone-column**

In the same file's `<style>` block, find the existing `.modifier-row` rules (around line 524-539). Above them, add:

```css
  .zone-stack {
    display: flex;
    flex: 1;
    gap: 0.6rem;
    min-width: 0;
  }
  .zone-column {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .zone-label {
    font-size: 0.6rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding-left: 0.25rem;
  }
  .zone-column[data-zone="situational"] .zone-label {
    color: var(--accent-situational-bright);
  }
```

Adjust the `.row-body > .modifier-row { flex: 1; min-width: 0; }` rule (around line 522): it is no longer the direct child of `.row-body` — the new structure is `.row-body > .zone-stack > .zone-column > .modifier-row`. Replace that rule with:

```css
  .row-body > .zone-stack { flex: 1; min-width: 0; }
```

- [ ] **Step 8: Update `StatusPaletteDock.svelte` to apply with `zone: 'situational'`**

Open `src/lib/components/gm-screen/StatusPaletteDock.svelte`. Find the `applyTemplate` function (around line 27). In the `modifiers.add({ … })` call (around lines 38-48), add `zone: 'situational'` at the end of the object literal:

```ts
      await modifiers.add({
        source: focusedCharacter.source,
        sourceId: focusedCharacter.source_id,
        name: t.name,
        description: t.description,
        effects: $state.snapshot(t.effects),
        binding: { kind: 'free' },
        tags: [...t.tags],
        originTemplateId: t.id,
        foundryCapturedLabels: [],
        zone: 'situational',
      });
```

- [ ] **Step 9: Verify (type-check + visual smoke)**

Run: `./scripts/verify.sh`
Expected: green.

Then start the dev server. Manual smoke checklist:

1. Open the GM Screen tool. Each character row now shows three sections: Active effects, Character carousel (with its own "+ Add"), Situational carousel (with its own "+ Add").
2. Click "+ Add" in the Character box → a new card appears in the Character zone (gray theme, no chip).
3. Click "+ Add" in the Situational box → a new card appears in the Situational zone (green theme, "Situational" chip in the head).
4. Existing advantage-bound (Foundry merit) cards appear in the Character carousel.
5. Apply a status template via the dock click flow → the new card lands in the Situational carousel with the green theme.
6. The 🗑 trash button appears on hover only over free-bound cards; clicking prompts a confirm; on yes, the card is deleted; on no, the card stays.
7. The × hide button still works on all cards (no behavior regression).
8. Tag filter applied with both zones populated → both zones filter identically.
9. Toggle "Show hidden" in the header → hidden cards in both zones reappear.
10. Restart the app → all the above persist.

- [ ] **Step 10: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte src/lib/components/gm-screen/StatusPaletteDock.svelte
git commit -m "feat(gm-screen): split character row into two zone carousels

CharacterRow body now renders [ActiveEffectsSummary][Character carousel]
[Situational carousel]. Each carousel filters cardEntries by entryZone()
and gets its own '+ Add' button passing the zone into addFreeModifier().
A shared {#snippet renderCard} keeps the ModifierCard props mapping DRY.

Free-bound materialized cards get onDelete wired (confirm() then
modifiers.delete). Advantage-bound and virtual cards leave onDelete
undefined — only free-bound cards show 🗑.

StatusPaletteDock.applyTemplate now passes zone: 'situational' so click-
apply produces situational-zone modifiers by default."
```

---

## Task 8: DnD primitive — types, matrix function, state-machine store (no UI wiring)

**Files:**
- Create: `src/lib/dnd/types.ts`
- Create: `src/lib/dnd/actions.ts`
- Create: `src/lib/dnd/store.svelte.ts`

**Anti-scope:** Do not create the four DnD components yet (Task 9). Do not modify `CharacterRow.svelte` or `ModifierCard.svelte` (Task 10).

**Depends on:** Task 4 (TS types available).

**Invariants cited:** spec §"DnD primitive" (state machine, source/target/action contracts, getActionsFor matrix).

**Tests:** none new — no frontend test framework; the matrix function is exercised in the Task 11 manual smoke.

- [ ] **Step 1: Create `src/lib/dnd/types.ts`**

Create the file with this content:

```ts
/**
 * Discriminated unions for the DnD primitive's source, target, and action
 * contracts. Pinned in v1 so v2 (cross-row drag) and v3 (Status Template
 * palette as drag source) can extend without breaking — adding new variants
 * is additive, never structural.
 *
 * See spec docs/superpowers/specs/2026-05-14-gm-screen-modifier-zones-and-dnd-design.md
 * §"Source / Target contracts" and §"Permission matrix".
 */
import type {
  BridgeCharacter,
  CharacterModifier,
  ModifierZone,
  StatusTemplate,
} from '../../types';

/**
 * What is being dragged. Each variant carries the full source object so
 * action handlers don't have to refetch.
 *
 * v1 issues only `free-mod` from pointerdown handlers in DragSource.
 * Advantage-bound modifiers ARE allowed to enter the held state (the
 * matrix function rejects every drop target, so they snap back) — this
 * keeps the matrix as the single source of truth.
 */
export type DragSource =
  | { kind: 'free-mod'; mod: CharacterModifier }
  | { kind: 'advantage'; mod: CharacterModifier }   // v1: every target returns []
  | { kind: 'template'; template: StatusTemplate }; // v3: not used in v1

/**
 * Where a drop can land. The character is carried so cross-row v2 logic
 * (and the v1 same-row constraint) can compare source.character to target.character.
 */
export type DropTarget =
  | { kind: 'character-zone'; character: BridgeCharacter }
  | { kind: 'situational-zone'; character: BridgeCharacter };

/**
 * One action the user can execute by completing the drop. Returned from
 * `getActionsFor(source, target)`. Empty array = invalid drop = auto-cancel.
 * Single-element array = execute immediately. ≥2 elements = open DropMenu.
 *
 * v1 emits only `move-zone`. The other variants are pinned for v2/v3.
 */
export type Action =
  | { id: 'move-zone';      label: string; newZone: ModifierZone }   // v1
  | { id: 'move-character'; label: string; newSourceId: string }     // v2
  | { id: 'copy-character'; label: string; newSourceId: string }     // v2
  | { id: 'apply-template'; label: string; zone: ModifierZone };     // v3
```

- [ ] **Step 2: Create `src/lib/dnd/actions.ts` with the v1 matrix**

Create the file with this content:

```ts
/**
 * The permission matrix. Returns the set of actions available for a given
 * (source, target) pair. Empty = invalid drop (auto-cancel). Single =
 * execute immediately. ≥2 = open DropMenu with the action list.
 *
 * v1 returns one action for two specific cells (free-bound card moving
 * between Character and Situational zones on the same character row).
 * Everything else returns [].
 *
 * v2 will add four cells for cross-row Move/Copy. v3 will add two cells
 * for template-source → situational-zone Apply. Each new phase adds
 * branches here — no contract changes.
 *
 * See spec §"Permission matrix" for the full v1/v2/v3 matrix table.
 */
import type { Action, DragSource, DropTarget } from './types';

export function getActionsFor(source: DragSource, target: DropTarget): Action[] {
  // Advantage-bound source: always invalid (zone-locked to character; no cross-row in v1).
  if (source.kind === 'advantage') return [];

  // Template-source: v3 only. In v1 always invalid.
  if (source.kind === 'template') return [];

  // Free-bound source from here. v1 same-row constraint: target character
  // must match source character.
  const sameChar =
    source.mod.source === target.character.source &&
    source.mod.sourceId === target.character.source_id;
  if (!sameChar) return [];

  // Move from character-zone → situational-zone.
  if (source.mod.zone === 'character' && target.kind === 'situational-zone') {
    return [{ id: 'move-zone', label: 'Move to Situational', newZone: 'situational' }];
  }
  // Move from situational-zone → character-zone.
  if (source.mod.zone === 'situational' && target.kind === 'character-zone') {
    return [{ id: 'move-zone', label: 'Move to Character', newZone: 'character' }];
  }

  // Dropping on the same zone the card already lives in: no-op (invalid → snap back).
  return [];
}
```

- [ ] **Step 3: Create `src/lib/dnd/store.svelte.ts` with the state machine**

Create the file with this content:

```ts
/**
 * DnD state-machine store. Singleton runes store. Owns the pickup-and-place
 * lifecycle plus the cursor position. UI components subscribe to derive
 * highlight / overlay rendering.
 *
 * State machine: idle → held → dropped|cancelled → idle.
 *
 * Lifecycle methods (called by DragSource/DropZone/DropMenu and the
 * global cleanup listeners installed by GmScreen.svelte):
 *   - pickup(source, originRect) — left-click on a card body
 *   - setTarget(target | null) — pointermove over a DropZone or off it
 *   - moveCursor(x, y) — pointermove updates the overlay
 *   - drop() — left-click on a DropZone; resolves actions and routes
 *   - cancel() — right-click, Esc, blur, click-outside, pointercancel
 *   - executeAction(action) — DropMenu picks one
 *
 * The store knows about `modifiers.setZone` because the only v1 action
 * is move-zone — keeps the wiring trivial. v2/v3 will inject more action
 * handlers; the dispatch table can grow inside `executeAction`.
 *
 * See spec §"DnD primitive" for the full state machine and cleanup edges.
 */
import { modifiers } from '../../store/modifiers.svelte';
import { getActionsFor } from './actions';
import type { Action, DragSource, DropTarget } from './types';

type HeldState = {
  source: DragSource;
  originRect: DOMRect;
  cursorX: number;
  cursorY: number;
  target: DropTarget | null;
  /** Computed action list for current (source, target). Refreshes on setTarget. */
  actions: Action[];
  /** When the held → menu transition has fired and the user is choosing. */
  menuOpenAt: { x: number; y: number } | null;
};

let _held = $state<HeldState | null>(null);

function refreshActions(): void {
  if (!_held) return;
  _held.actions = _held.target ? getActionsFor(_held.source, _held.target) : [];
}

async function applyAction(action: Action, source: DragSource): Promise<void> {
  // v1: only move-zone is in the matrix. v2/v3 will branch here.
  if (action.id === 'move-zone' && source.kind === 'free-mod') {
    await modifiers.setZone(source.mod.id, action.newZone);
    return;
  }
  // Defensive: unknown action / unsupported source — log and no-op.
  console.warn('[dnd] unhandled action', action.id, 'for source', source.kind);
}

export const dndStore = {
  get held() { return _held; },

  pickup(source: DragSource, originRect: DOMRect, startX: number, startY: number): void {
    _held = {
      source,
      originRect,
      cursorX: startX,
      cursorY: startY,
      target: null,
      actions: [],
      menuOpenAt: null,
    };
  },

  setTarget(target: DropTarget | null): void {
    if (!_held) return;
    _held.target = target;
    refreshActions();
  },

  moveCursor(x: number, y: number): void {
    if (!_held) return;
    _held.cursorX = x;
    _held.cursorY = y;
  },

  async drop(): Promise<void> {
    if (!_held) return;
    const { actions, source } = _held;
    if (actions.length === 0) {
      this.cancel();
      return;
    }
    if (actions.length === 1) {
      const snapshotSource = source;
      _held = null;
      try {
        await applyAction(actions[0], snapshotSource);
      } catch (err) {
        console.error('[dnd] action failed:', err);
      }
      return;
    }
    // ≥2 actions: open the menu at the current cursor location.
    _held.menuOpenAt = { x: _held.cursorX, y: _held.cursorY };
  },

  async executeAction(action: Action): Promise<void> {
    if (!_held) return;
    const snapshotSource = _held.source;
    _held = null;
    try {
      await applyAction(action, snapshotSource);
    } catch (err) {
      console.error('[dnd] action failed:', err);
    }
  },

  cancel(): void {
    _held = null;
  },
};
```

- [ ] **Step 4: Verify**

Run: `./scripts/verify.sh`
Expected: green. New files are TypeScript-only and self-contained — no imports yet from consumer components.

- [ ] **Step 5: Commit**

```bash
git add src/lib/dnd/types.ts src/lib/dnd/actions.ts src/lib/dnd/store.svelte.ts
git commit -m "feat(dnd): primitive types, permission matrix, state-machine store

DragSource / DropTarget / Action discriminated unions pinned for v1 with
variants reserved for v2 (cross-row Move/Copy) and v3 (template palette
Apply). getActionsFor returns v1's two valid cells (free-bound move-zone
on same character row) and [] everywhere else.

dndStore is the singleton runes state machine — owns held state, cursor
position, computed action list per setTarget, and drop()/cancel()
lifecycle. drop() routes 0-action → cancel, 1-action → execute, 2+ →
menu-open. v1 only ever takes the 1-action branch."
```

---

## Task 9: DnD primitive — leaf components

**Files:**
- Create: `src/lib/components/dnd/DragSource.svelte`
- Create: `src/lib/components/dnd/DropZone.svelte`
- Create: `src/lib/components/dnd/DropMenu.svelte`
- Create: `src/lib/components/dnd/HeldCardOverlay.svelte`

**Anti-scope:** Do not wire these into `CharacterRow.svelte`, `ModifierCard.svelte`, or `GmScreen.svelte` in this task. That's Task 10. Do not create global event listeners (`window` blur, `keydown`, `contextmenu`) — those install in Task 10 alongside the wiring.

**Depends on:** Task 8 (`dndStore` and types).

**Invariants cited:** spec §"Component additions"; §"Visual feedback during held state".

**Tests:** none new (no framework). Manual smoke happens in Task 10.

- [ ] **Step 1: Create `DragSource.svelte`**

Create `src/lib/components/dnd/DragSource.svelte`:

```svelte
<script lang="ts">
  /**
   * Wraps a draggable element. On left-button pointerdown, snapshots the
   * element's bounding rect and asks dndStore to enter the held state with
   * the configured source. Calls stopPropagation so the parent row's
   * focus-handler does not also fire.
   *
   * The pickup target is the wrapped element itself (e.g. `.card-body`).
   * Button children of the wrapped element should be siblings, not nested
   * descendants, so they don't initiate pickup when clicked.
   */
  import { dndStore } from '../../dnd/store.svelte';
  import type { DragSource } from '../../dnd/types';
  import type { Snippet } from 'svelte';

  interface Props {
    source: DragSource;
    /** When true, the element is rendered but pointerdown is ignored. */
    disabled?: boolean;
    children: Snippet;
  }

  let { source, disabled = false, children }: Props = $props();

  let wrapEl: HTMLDivElement | undefined = $state();

  function handlePointerDown(e: PointerEvent): void {
    if (disabled) return;
    if (e.button !== 0) return;             // left button only
    if (!wrapEl) return;
    e.stopPropagation();
    e.preventDefault();
    const rect = wrapEl.getBoundingClientRect();
    dndStore.pickup(source, rect, e.clientX, e.clientY);
  }
</script>

<div bind:this={wrapEl} onpointerdown={handlePointerDown}>
  {@render children()}
</div>

<style>
  div {
    display: contents;   /* wrapper is layout-transparent */
  }
</style>
```

- [ ] **Step 2: Create `DropZone.svelte`**

Create `src/lib/components/dnd/DropZone.svelte`:

```svelte
<script lang="ts">
  /**
   * Wraps a drop target region. While dndStore.held is non-null and the
   * cursor is over this element, the store's target is set to our target.
   * On left-click pointerdown during held state, calls dndStore.drop().
   * Cursor leaving us calls setTarget(null).
   *
   * Renders a `data-drop-active` attribute when the current target equals
   * ours AND the resolved action list is non-empty, so the consumer can
   * style the "valid drop zone" outline.
   */
  import { dndStore } from '../../dnd/store.svelte';
  import type { DropTarget } from '../../dnd/types';
  import type { Snippet } from 'svelte';

  interface Props {
    target: DropTarget;
    children: Snippet;
  }

  let { target, children }: Props = $props();

  function isMine(): boolean {
    const h = dndStore.held;
    if (!h || !h.target) return false;
    if (h.target.kind !== target.kind) return false;
    return h.target.character.source === target.character.source
        && h.target.character.source_id === target.character.source_id;
  }

  let dropActive = $derived.by((): boolean => {
    const h = dndStore.held;
    if (!h) return false;
    return isMine() && h.actions.length > 0;
  });

  function handlePointerEnter(): void {
    if (!dndStore.held) return;
    dndStore.setTarget(target);
  }
  function handlePointerLeave(): void {
    if (!dndStore.held) return;
    if (isMine()) dndStore.setTarget(null);
  }
  async function handlePointerDown(e: PointerEvent): Promise<void> {
    if (!dndStore.held) return;
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    // Ensure target is set (pointerenter may not fire if cursor entered before pickup).
    dndStore.setTarget(target);
    await dndStore.drop();
  }
</script>

<div
  class="dnd-drop-zone"
  data-drop-active={dropActive ? 'true' : 'false'}
  onpointerenter={handlePointerEnter}
  onpointerleave={handlePointerLeave}
  onpointerdown={handlePointerDown}
>
  {@render children()}
</div>

<style>
  .dnd-drop-zone[data-drop-active="true"] {
    outline: 2px dashed var(--accent-situational-bright);
    outline-offset: 2px;
    border-radius: 0.625rem;
  }
</style>
```

- [ ] **Step 3: Create `DropMenu.svelte`**

Create `src/lib/components/dnd/DropMenu.svelte`:

```svelte
<script lang="ts">
  /**
   * Contextual action menu rendered at the cursor when a drop resolves to
   * ≥2 actions. v1 never opens this — single-action drops execute immediately.
   * v2/v3 will surface multi-action drops here.
   *
   * Closes via:
   *   - click on an action → executeAction()
   *   - click outside the menu element → cancel()
   *   - right-click / Esc / blur → handled by global listeners in
   *     GmScreen.svelte that call dndStore.cancel().
   */
  import { dndStore } from '../../dnd/store.svelte';
  import type { Action } from '../../dnd/types';

  let menuEl: HTMLDivElement | undefined = $state();

  async function pick(action: Action) {
    await dndStore.executeAction(action);
  }

  function handleOutsidePointerDown(e: PointerEvent) {
    if (!menuEl) return;
    if (!menuEl.contains(e.target as Node)) {
      dndStore.cancel();
    }
  }

  $effect(() => {
    if (dndStore.held?.menuOpenAt) {
      document.addEventListener('pointerdown', handleOutsidePointerDown, true);
      return () => document.removeEventListener('pointerdown', handleOutsidePointerDown, true);
    }
  });
</script>

{#if dndStore.held?.menuOpenAt}
  {@const pos = dndStore.held.menuOpenAt}
  {@const actions = dndStore.held.actions}
  <div
    bind:this={menuEl}
    class="drop-menu"
    style="left: {pos.x}px; top: {pos.y}px;"
    role="menu"
  >
    {#each actions as action}
      <button class="action" role="menuitem" onclick={() => pick(action)}>
        {action.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .drop-menu {
    position: fixed;
    z-index: 2000;
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.4rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    min-width: 13rem;
    padding: 0.25rem 0;
  }
  .action {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    color: var(--text-primary);
    border: none;
    padding: 0.4rem 0.75rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .action:hover {
    background: var(--bg-active);
  }
</style>
```

- [ ] **Step 4: Create `HeldCardOverlay.svelte`**

Create `src/lib/components/dnd/HeldCardOverlay.svelte`:

```svelte
<script lang="ts">
  /**
   * Top-level overlay rendered by GmScreen.svelte. While dndStore.held is
   * non-null, this fixed-position element follows the cursor with a small
   * miniature of the dragged card. Listens to global pointermove on the
   * document to update store cursor coords; the listener installs only
   * while held is non-null.
   */
  import { dndStore } from '../../dnd/store.svelte';

  function handleMove(e: PointerEvent) {
    if (!dndStore.held) return;
    dndStore.moveCursor(e.clientX, e.clientY);
  }

  $effect(() => {
    if (dndStore.held) {
      document.addEventListener('pointermove', handleMove);
      return () => document.removeEventListener('pointermove', handleMove);
    }
  });

  function labelOf(): string {
    const h = dndStore.held;
    if (!h) return '';
    if (h.source.kind === 'free-mod' || h.source.kind === 'advantage') return h.source.mod.name;
    if (h.source.kind === 'template') return h.source.template.name;
    return '';
  }
</script>

{#if dndStore.held && !dndStore.held.menuOpenAt}
  {@const h = dndStore.held}
  <div
    class="held-overlay"
    style="left: {h.cursorX + 8}px; top: {h.cursorY + 8}px;"
    aria-hidden="true"
  >
    {labelOf()}
  </div>
{/if}

<style>
  .held-overlay {
    position: fixed;
    z-index: 1500;
    pointer-events: none;
    background: var(--bg-card);
    border: 1px solid var(--accent-bright);
    border-radius: 0.4rem;
    padding: 0.3rem 0.6rem;
    font-size: 0.75rem;
    color: var(--text-primary);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.5);
    max-width: 14rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
```

- [ ] **Step 5: Verify**

Run: `./scripts/verify.sh`
Expected: green. Components are typed but not yet imported anywhere; build is unchanged behaviorally.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/dnd/
git commit -m "feat(dnd): DragSource, DropZone, DropMenu, HeldCardOverlay components

DragSource wraps a draggable element and emits dndStore.pickup() on left
pointerdown. DropZone wraps a target region, sets store target on enter,
clears on leave, drops on pointerdown — rendering data-drop-active when
the resolved action list is non-empty. DropMenu renders at the cursor
when dndStore opens the menu (≥2 actions). HeldCardOverlay tracks
pointermove on document and renders a fixed-position label following
the cursor. Components are isolated — wiring lands in the next commit."
```

---

## Task 10: Wire DnD into `ModifierCard` + `CharacterRow` + `GmScreen` + global cleanup listeners

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (wrap `.card-body` in `<DragSource>`; suppress hover transforms when ancestor has `.dnd-active`)
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (wrap each `.modifier-row` in `<DropZone>` with the right target)
- Modify: `src/tools/GmScreen.svelte` (mount `<HeldCardOverlay>` + `<DropMenu>` at root; toggle `.dnd-active` on the root; install global blur/Esc/contextmenu listeners; render-time effect that calls `dndStore.cancel()` on those events)

**Anti-scope:** Do not alter any DnD types, the matrix function, or the store's lifecycle methods — they were settled in Task 8.

**Depends on:** Tasks 8 (types + store), 9 (components).

**Invariants cited:** spec §"DnD state machine", §"Cleanup edges", §"Focus-handler interference", §"Visual feedback during held state".

**Tests:** none new (no framework). Manual smoke is the gate — run the full 14-step checklist from the spec.

- [ ] **Step 1: Wrap `.card-body` in `<DragSource>` inside `ModifierCard.svelte`**

Open `src/lib/components/gm-screen/ModifierCard.svelte`. Add the import at the top of the `<script>`:

```ts
  import DragSource from '../dnd/DragSource.svelte';
  import type { DragSource as DragSourceType } from '../../dnd/types';
```

Build the source object derived from props:

```ts
  let dragSource = $derived.by((): DragSourceType => {
    if (modifier.binding.kind === 'advantage') return { kind: 'advantage', mod: modifier };
    return { kind: 'free-mod', mod: modifier };
  });

  let dragDisabled = $derived(isVirtual);
```

`isVirtual` cards have no DB row yet (id=0). Picking them up would emit a DragSource with id=0 and confuse the matrix; better to disable pickup entirely on virtuals — the GM materializes by clicking toggle/cog first.

Now replace the `<div class="card-body">…</div>` block with a `<DragSource>` wrapping the same body content:

```svelte
  <DragSource source={dragSource} disabled={dragDisabled}>
    <div class="card-body">
      … (the existing contents of .card-body — zone-chip, head, origin, bonuses, conditionals-badge, effects, tags) …
    </div>
  </DragSource>
```

- [ ] **Step 2: Suppress hover transforms while a DnD is active**

In the same file's `<style>` block, find the existing `.modifier-card:hover` rule (around line 223). After that block, add a global-scope suppression rule:

```css
  :global(.dnd-active) .modifier-card:hover {
    transform: translateX(calc(var(--base-x) + var(--shift-x))) !important;
    box-shadow: none !important;
    z-index: calc(100 - var(--distance));
  }
  :global(.dnd-active) .modifier-card:hover + :global(.modifier-card),
  :global(.dnd-active) .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card),
  :global(.dnd-active) .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card),
  :global(.dnd-active) .modifier-card:has(+ :global(.modifier-card:hover)),
  :global(.dnd-active) .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card:hover)),
  :global(.dnd-active) .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card:hover)) {
    --shift-x: 0rem !important;
  }
```

The `!important` is acceptable here because we're explicitly overriding the per-card hover cascade only during the DnD-active state — there's no other rule that should win.

- [ ] **Step 3: Wrap each `.modifier-row` in `<DropZone>` inside `CharacterRow.svelte`**

Open `src/lib/components/gm-screen/CharacterRow.svelte`. Add the imports at the top of `<script>`:

```ts
  import DropZone from '$lib/components/dnd/DropZone.svelte';
  import type { DropTarget } from '$lib/dnd/types';
```

Add two derived target objects:

```ts
  let characterTarget = $derived<DropTarget>({ kind: 'character-zone', character });
  let situationalTarget = $derived<DropTarget>({ kind: 'situational-zone', character });
```

Then in the markup (the `.zone-stack` block added in Task 7), wrap each `.modifier-row` in a `<DropZone>`:

```svelte
  <div class="zone-stack">
    <div class="zone-column" data-zone="character">
      <div class="zone-label">Character</div>
      <DropZone target={characterTarget}>
        <div
          class="modifier-row"
          style="--cards: {characterCards.length};"
        >
          {#each characterCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
            {@render renderCard(entry)}
          {/each}
          <button class="add-modifier" onclick={() => addFreeModifier('character')}>+ Add modifier</button>
        </div>
      </DropZone>
    </div>
    <div class="zone-column" data-zone="situational">
      <div class="zone-label">Situational</div>
      <DropZone target={situationalTarget}>
        <div
          class="modifier-row"
          style="--cards: {situationalCards.length};"
        >
          {#each situationalCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
            {@render renderCard(entry)}
          {/each}
          <button class="add-modifier" onclick={() => addFreeModifier('situational')}>+ Add modifier</button>
        </div>
      </DropZone>
    </div>
  </div>
```

- [ ] **Step 4: Mount overlay + menu + cleanup listeners in `GmScreen.svelte`**

Open `src/tools/GmScreen.svelte`. Add the imports at the top of `<script>`:

```ts
  import HeldCardOverlay from '$lib/components/dnd/HeldCardOverlay.svelte';
  import DropMenu from '$lib/components/dnd/DropMenu.svelte';
  import { dndStore } from '$lib/dnd/store.svelte';
```

Add an effect for the global cleanup listeners — these install only while a pickup is held:

```ts
  $effect(() => {
    if (!dndStore.held) return;

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') dndStore.cancel();
    };
    const onContextMenu = (e: MouseEvent) => {
      e.preventDefault();
      dndStore.cancel();
    };
    const onBlur = () => dndStore.cancel();
    const onPointerCancel = () => dndStore.cancel();

    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('contextmenu', onContextMenu);
    window.addEventListener('blur', onBlur);
    window.addEventListener('pointercancel', onPointerCancel);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('contextmenu', onContextMenu);
      window.removeEventListener('blur', onBlur);
      window.removeEventListener('pointercancel', onPointerCancel);
    };
  });
```

Derive the `.dnd-active` class flag:

```ts
  let dndActive = $derived(dndStore.held !== null);
```

In the markup, update the root `<div class="gm-screen">` opening tag to include the class:

```svelte
<div class="gm-screen" class:dnd-active={dndActive}>
```

At the very end of the markup, immediately before the closing `</div>` of `.gm-screen`, mount the two top-level overlays:

```svelte
  <HeldCardOverlay />
  <DropMenu />
</div>
```

(The `<style>` of `.gm-screen` does NOT need a `.dnd-active` selector — the `.dnd-active` flag is only used by `:global(.dnd-active) .modifier-card:hover` rules set up in Step 2.)

Add a `cursor: grabbing` rule on `.dnd-active` so the cursor reflects held state:

```css
  .gm-screen.dnd-active {
    cursor: grabbing;
  }
  .gm-screen.dnd-active * {
    cursor: grabbing !important;
  }
```

- [ ] **Step 5: Click-outside cancel**

The DropZones cancel via their own pointerdown handler ONLY when the pointer is over them. If the user picks up a card and clicks on empty space (the GM screen background, the orphans section, a character header), nothing currently fires `cancel()`. Add a fallback global pointerdown listener — installed only during held state — that calls `cancel()` if the pointerdown's target is not inside a DropZone.

Extend the `$effect` from Step 4 with a final listener:

```ts
    const onGlobalDown = (e: PointerEvent) => {
      if (e.button !== 0) return;
      // If we're inside a .dnd-drop-zone, that zone's own handler runs and
      // calls drop(). Otherwise → cancel.
      const target = e.target as HTMLElement | null;
      if (target && target.closest('.dnd-drop-zone')) return;
      dndStore.cancel();
    };
    window.addEventListener('pointerdown', onGlobalDown);
    // …and in the cleanup:
    window.removeEventListener('pointerdown', onGlobalDown);
```

The combined effect:

```ts
  $effect(() => {
    if (!dndStore.held) return;

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') dndStore.cancel();
    };
    const onContextMenu = (e: MouseEvent) => {
      e.preventDefault();
      dndStore.cancel();
    };
    const onBlur = () => dndStore.cancel();
    const onPointerCancel = () => dndStore.cancel();
    const onGlobalDown = (e: PointerEvent) => {
      if (e.button !== 0) return;
      const target = e.target as HTMLElement | null;
      if (target && target.closest('.dnd-drop-zone')) return;
      dndStore.cancel();
    };

    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('contextmenu', onContextMenu);
    window.addEventListener('blur', onBlur);
    window.addEventListener('pointercancel', onPointerCancel);
    window.addEventListener('pointerdown', onGlobalDown);
    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('contextmenu', onContextMenu);
      window.removeEventListener('blur', onBlur);
      window.removeEventListener('pointercancel', onPointerCancel);
      window.removeEventListener('pointerdown', onGlobalDown);
    };
  });
```

Note: this fires BEFORE the DropZone's pointerdown handler in the bubble phase. Use a capture-phase listener if order matters — but actually the existing DropZone calls `stopPropagation()` and `preventDefault()`, so the window listener won't fire when the click hits a DropZone. The `.closest('.dnd-drop-zone')` check is belt-and-suspenders.

- [ ] **Step 6: Verify**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 7: Manual smoke — full DnD checklist**

Start the dev server (`npm run tauri dev`). Run through every step:

1. Open the GM Screen. Three-box layout shows per row.
2. Click "+ Add" in Character → card lands in Character (gray theme).
3. Click "+ Add" in Situational → card lands in Situational (green theme + "Situational" chip).
4. Left-click on a free-bound character-zone card body (not on a button). Card is "held": label appears at cursor, drop zones get green dashed outline.
5. Move cursor over the same row's Situational drop zone. Outline stays. Left-click → card moves zone, theme switches.
6. Same in reverse: pickup a free-bound situational-zone card; drop on Character → card moves, green theme drops.
7. Pickup a card and click on the same zone it lives in (no-op cell) → drop zone doesn't get active outline (action list is empty) → click anyway falls through → card snaps back (cancel).
8. Pickup a card → right-click → card snaps back (state → idle).
9. Pickup a card → press Esc → card snaps back.
10. Pickup a card → alt-tab away → card snaps back when window loses focus.
11. Try to pickup an advantage-bound (Foundry merit) card. Pickup succeeds (DragSource always emits) but every drop returns `[]` → no drop zones light up → clicking anywhere cancels.
12. Click 🗑 on a free-bound card → confirm dialog → on yes, the card is deleted; on no, the card stays.
13. Click × hide on a free-bound card → card hides (existing behavior, no regression).
14. Restart the app → zone moves from steps 5/6 persisted.
15. DropMenu render sanity: temporarily edit `getActionsFor` in `src/lib/dnd/actions.ts` to return TWO actions for the v1 same-row drop (e.g. `[{ id: 'move-zone', label: 'Move to Situational', newZone: 'situational' }, { id: 'move-zone', label: 'Test second action', newZone: 'situational' }]`). Perform a drop. The DropMenu should appear at cursor with both rows. Revert the edit before committing.
16. Tag filter with both carousels populated. Filter applies to both zones identically. Toggle "Show hidden" — hidden cards in both zones reappear.
17. Free-bound card foot overflow check: create a free-bound card. Verify the foot row (toggle, hide, trash) fits within the 9rem card width with no overflow in all combinations (on/off, hidden/visible).

If ANY step fails, do not commit. Fix the regression and re-run from step 1.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte src/lib/components/gm-screen/CharacterRow.svelte src/tools/GmScreen.svelte
git commit -m "feat(gm-screen): wire DnD primitive into card row and screen root

ModifierCard wraps .card-body in DragSource emitting {kind:'free-mod'|
'advantage', mod}. Virtual cards are pickup-disabled. .dnd-active
ancestor class suppresses the hover transforms and neighbor-shift
cascade so cursor sweep during pickup doesn't trigger card animations.

CharacterRow wraps each .modifier-row in a DropZone keyed to its zone +
character. GmScreen renders HeldCardOverlay and DropMenu at root, toggles
.dnd-active on the root container, and installs global cleanup listeners
(keydown Esc, contextmenu right-click, blur, pointercancel, click-outside-
drop-zone) that fire dndStore.cancel(). All listeners install only while
held is non-null and tear down on idle."
```

---

## Task 11: ARCHITECTURE.md update

**Files:**
- Modify: `ARCHITECTURE.md` §4 (IPC commands inventory)

**Anti-scope:** No other files in this task.

**Depends on:** Task 3 (`set_modifier_zone` registered).

**Invariants cited:** `ARCHITECTURE.md` is itself an invariant doc.

**Tests:** none — text-only documentation update.

- [ ] **Step 1: Locate the IPC commands inventory**

Open `ARCHITECTURE.md`. Find §4 "Tauri IPC commands" — specifically the `src-tauri/src/db/modifier.rs` entry (look for `db/modifier.rs` in the bulleted list).

If a `db/modifier.rs` entry already exists, update it to include `set_modifier_zone` in the command list and bump the count by 1. Example:

```markdown
- **`src-tauri/src/db/modifier.rs`** (N+1):
  `list_character_modifiers`, `list_all_character_modifiers`,
  `add_character_modifier`, `update_character_modifier`,
  `delete_character_modifier`, `set_modifier_active`,
  `set_modifier_hidden`, `set_modifier_zone`,
  `materialize_advantage_modifier`.
```

If there is no `db/modifier.rs` entry yet (current state — check via grep), add one. Insert it in the inventory after `db/dyscrasia.rs`:

```markdown
- **`src-tauri/src/db/modifier.rs`** (9):
  `list_character_modifiers`, `list_all_character_modifiers`,
  `add_character_modifier`, `update_character_modifier`,
  `delete_character_modifier`, `set_modifier_active`,
  `set_modifier_hidden`, `set_modifier_zone`,
  `materialize_advantage_modifier`.
```

Also update the inventory "Total: 32 commands" line at the bottom of §4 to reflect the new total. Use `grep -n "tauri::command" src-tauri/src` to get the precise count if uncertain.

- [ ] **Step 2: Verify**

Run: `./scripts/verify.sh`
Expected: green (docs change is no-op for build).

- [ ] **Step 3: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "docs(architecture): register set_modifier_zone in §4 IPC inventory

Adds db/modifier.rs entry (if missing) to the §4 Tauri IPC commands
inventory and lists set_modifier_zone alongside the existing modifier
commands. Updates the §4 running total."
```

---

## Task 12: Final branch-wide code review

**Files:** all files modified across Tasks 1-11.

**Anti-scope:** No new edits unless the review surfaces an issue.

**Depends on:** Tasks 1-11.

**Invariants cited:** project `CLAUDE.md` workflow override — "After ALL plan tasks are committed, run a SINGLE `code-review:code-review` against the full branch diff."

**Tests:** none new.

- [ ] **Step 1: Run the code-review skill against the branch**

Invoke the `code-review:code-review` skill against the full branch diff vs. `master`. The skill will surface any cross-task concerns (dead code, inconsistent naming, missed invariants, accidentally widened scope) that the per-task `verify.sh` gate wouldn't catch.

- [ ] **Step 2: Address any blocking findings**

If the review surfaces a blocker:
  - Fix it in a follow-up commit.
  - Re-run `./scripts/verify.sh`.
  - Re-run the smoke checklist for the specific feature touched.

If the review surfaces only non-blocking polish items:
  - Note them in the PR description or the merge commit body.
  - Don't gate the merge on cosmetic suggestions.

- [ ] **Step 3: Open PR (optional, per user preference)**

If the user requests, open the PR with a summary lifted from the spec:

```bash
gh pr create --title "GM screen modifier zones + drag-and-drop primitive" --body "$(cat <<'EOF'
## Summary
- Adds per-modifier zone (character | situational) with origin-template backfill
- Splits CharacterRow into three boxes: active effects · character carousel · situational carousel
- New green CSS tokens + "Situational" pill chip for situational-zone cards
- Hard-delete trash button on free-bound cards (× hide kept alongside)
- Pointer-events DnD primitive: pickup-and-place model with getActionsFor permission matrix; v1 ships same-row free-bound zone reclassify only; v2/v3 phases are designed-for via contracts pinned now

## Test plan
- [ ] cargo test passes (5 new modifier unit tests)
- [ ] ./scripts/verify.sh green
- [ ] All 17 manual smoke steps pass on dev server
- [ ] Migration backfill verified on a DB containing template-applied modifiers

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

If the user already merges directly to master via their own flow, skip this step.

---

## Plan self-review

Pass: every step contains the actual content an engineer needs.

- **Spec coverage** — walked through spec sections and confirmed each maps to a task:
  - Domain shape changes → Task 2 (enum, fields, db read/write, unit tests) + Task 4 (TS mirror)
  - DB migration with backfill → Task 1
  - IPC + typed wrapper + store → Tasks 3 + 4
  - Layout three-box → Task 7
  - Visual treatment (tokens, chip, data-zone) → Tasks 5 + 6
  - Delete UX → Tasks 6 (prop) + 7 (wiring)
  - DnD state machine → Tasks 8 + 9 + 10
  - Source/target/action contracts → Task 8
  - getActionsFor matrix → Task 8
  - Cleanup edges (blur, Esc, contextmenu, outside-click, pointercancel) → Task 10 step 5
  - Focus-handler interference → Task 10 (`stopPropagation()` in DragSource and DropZone)
  - Visual feedback during held (.dnd-active) → Task 10 steps 2 + 4
  - ARCHITECTURE.md update → Task 11
  - Testing — 5 Rust unit tests in Task 2; manual smoke checklist (17 steps) in Task 10 step 7
- **Placeholder scan** — no TBD / TODO / "implement later" / "fill in details" anywhere; every code block is complete.
- **Type consistency** — `ModifierZone`/`zone`/`setModifierZone`/`setZone`/`getActionsFor` spellings match across tasks. `dndStore` referenced consistently. Component prop names (`source`, `target`, `disabled`, `children`) match between Tasks 9 and 10.
- **Verify before every commit** — every commit step is preceded by a `./scripts/verify.sh` step per project `CLAUDE.md` hard rule.
