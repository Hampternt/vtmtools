# GM Screen — Plan B: Status palette implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plan B tasks are committed, run a SINGLE `code-review:code-review` against the full Plan B branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** Ship the GM Screen's reusable status-effect palette dock — `🛡 GM Screen` gains a right-side panel of GM-authored templates (`Slippery`, `Blind`, etc.). Click a template while a character row is focused to drop an **independent copy** of the template's effects onto that character.

**Architecture:** New SQLite table `status_templates` (already migrated by Plan A's `0005_modifiers.sql` — Plan B simply wires CRUD against it). Four new Tauri commands in `src-tauri/src/db/status_template.rs`. Frontend reuses `src/lib/modifiers/api.ts` (extended with template wrappers), adds `src/store/statusTemplates.svelte.ts`, and adds two components: `StatusPaletteDock.svelte` (palette grid + apply UX) + `StatusTemplateEditor.svelte` (template authoring). The dock wires into the right side of `GmScreen.svelte`'s layout (spec §7.1). Apply path: read template → build `NewCharacterModifier` carrying the template's effects/tags/name + `originTemplateId` → call existing `add_character_modifier`. There is no separate `apply_template_to_character` command — provenance flows through the input shape (spec §5).

**Tech Stack:** Rust + sqlx (existing), Tauri 2 IPC, Svelte 5 runes mode, TypeScript. No new migration — `status_templates` table already exists.

**Spec:** `docs/superpowers/specs/2026-05-03-gm-screen-design.md` (stage 1 §10 Plan B).
**Architecture reference:** `ARCHITECTURE.md` §4 (IPC + typed wrappers), §5 (boundaries), §6 (CSS / token invariants), §7 (error-handling prefixes), §10 (testing).

**Spec defaults adopted:**
- Click-to-apply with focused-character convention (spec §13 default). Drag-and-drop is a polish pass — not in scope.
- Empty palette by default — no canned templates seeded (spec §13 default). GM populates via `+ New template`.

**Depends on:** Plan A merged. The `add_character_modifier` IPC and the `modifiers` store are required dependencies.

---

## File structure

### Files created

| Path | Responsibility |
|---|---|
| `src-tauri/src/db/status_template.rs` | 4 Tauri commands + `db_*` helpers + inline tests. Mirrors the modifier-db pattern from Plan A. |
| `src/store/statusTemplates.svelte.ts` | Cached template list, `ensureLoaded` / `refresh` / CRUD. Mirrors `src/store/modifiers.svelte.ts` shape. |
| `src/lib/components/gm-screen/StatusPaletteDock.svelte` | Right-side dock: template grid + `+ New template` + click-to-apply UX. Tracks the focused character. |
| `src/lib/components/gm-screen/StatusTemplateEditor.svelte` | Side-pane authoring form for a single template (name + description + effects + tags). Reuses the editor row pattern from `ModifierEffectEditor.svelte`. |

### Files modified

| Path | Change |
|---|---|
| `src-tauri/src/db/mod.rs` | Add `pub mod status_template;`. |
| `src-tauri/src/lib.rs` | Register 4 new commands in `invoke_handler(...)`. |
| `src-tauri/src/shared/modifier.rs` | Add `StatusTemplate` + `NewStatusTemplate` + `StatusTemplatePatch` type defs. |
| `src/types.ts` | Mirror `StatusTemplate`, `NewStatusTemplateInput`, `StatusTemplatePatchInput`. |
| `src/lib/modifiers/api.ts` | Extend with 4 template typed wrappers. |
| `src/lib/components/gm-screen/ModifierCard.svelte` | Add `originTemplateName` subtitle when set (provenance display per spec §8.4). |
| `src/tools/GmScreen.svelte` | Layout switches to two-column: rows on the left, `StatusPaletteDock` on the right. Track focused character row. |

### Files NOT touched in Plan B

- `src-tauri/src/db/modifier.rs` (frozen by Plan A)
- `src-tauri/migrations/*.sql` (no new migration — table already exists)
- `src/tools/Campaign.svelte`

---

## Task B1: status_template Rust module + 4 commands

**Goal:** Land the type defs in `shared/modifier.rs`, build the db module, and pass the round-trip / error tests. Same shape and style as Plan A's `db/modifier.rs`.

**Files:**
- Modify: `src-tauri/src/shared/modifier.rs` (append template types)
- Create: `src-tauri/src/db/status_template.rs`
- Modify: `src-tauri/src/db/mod.rs` (add `pub mod status_template;`)

**Anti-scope:** Do not touch `lib.rs`, `src/**/*`, `db/modifier.rs`, migrations.

**Depends on:** Plan A complete.

**Invariants cited:** ARCH §7 (error prefixes `db/status_template.<op>:`), spec §9 error table, ARCH §10 (Rust tests inline `#[cfg(test)] mod tests`).

**Tests required:** YES — JSON round-trip is real logic. TDD the round-trip + error cases.

- [ ] **Step 1: Append template types to `shared/modifier.rs`**

```rust
/// One row in the status_templates table — a GM-authored reusable bundle of
/// effects + tags. Templates have no character anchor; they're applied as
/// independent copies via add_character_modifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTemplate {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewStatusTemplate {
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTemplatePatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub effects: Option<Vec<ModifierEffect>>,
    pub tags: Option<Vec<String>>,
}
```

- [ ] **Step 2: Write the failing tests for `db_list` round-trip + `db_add` happy path + empty-name error**

Create `src-tauri/src/db/status_template.rs`:

```rust
use sqlx::{Row, SqlitePool};
use crate::shared::modifier::{
    ModifierEffect, NewStatusTemplate, StatusTemplate, StatusTemplatePatch,
};

fn row_to_template(r: &sqlx::sqlite::SqliteRow) -> Result<StatusTemplate, String> {
    let effects_json: String = r.get("effects_json");
    let effects: Vec<ModifierEffect> = serde_json::from_str(&effects_json)
        .map_err(|e| format!("db/status_template.list: effects deserialize: {e}"))?;
    let tags_json: String = r.get("tags_json");
    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| format!("db/status_template.list: tags deserialize: {e}"))?;
    Ok(StatusTemplate {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get("description"),
        effects,
        tags,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub(crate) async fn db_list(pool: &SqlitePool) -> Result<Vec<StatusTemplate>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, effects_json, tags_json, created_at, updated_at
         FROM status_templates ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/status_template.list: {e}"))?;
    rows.iter().map(row_to_template).collect()
}

pub(crate) async fn db_get(pool: &SqlitePool, id: i64) -> Result<StatusTemplate, String> {
    let row = sqlx::query(
        "SELECT id, name, description, effects_json, tags_json, created_at, updated_at
         FROM status_templates WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/status_template.get: {e}"))?
    .ok_or_else(|| "db/status_template.get: not found".to_string())?;
    row_to_template(&row)
}

#[tauri::command]
pub async fn list_status_templates(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<StatusTemplate>, String> {
    db_list(&pool.0).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::modifier::ModifierKind;
    use sqlx::SqlitePool;

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = fresh_pool().await;
        let result = db_list(&pool).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn round_trip_preserves_effects_and_tags() {
        let pool = fresh_pool().await;
        sqlx::query(
            "INSERT INTO status_templates (name, description, effects_json, tags_json)
             VALUES ('Slippery', 'Hard to grapple',
                     '[{\"kind\":\"difficulty\",\"scope\":\"grapple\",\"delta\":2,\"note\":null}]',
                     '[\"Combat\",\"Physical\"]')"
        )
        .execute(&pool).await.unwrap();
        let list = db_list(&pool).await.unwrap();
        assert_eq!(list.len(), 1);
        let t = &list[0];
        assert_eq!(t.name, "Slippery");
        assert_eq!(t.effects.len(), 1);
        assert_eq!(t.effects[0].kind, ModifierKind::Difficulty);
        assert_eq!(t.tags, vec!["Combat".to_string(), "Physical".to_string()]);
    }
}
```

- [ ] **Step 3: Run to verify they fail / pass mix**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- status_template::tests::list_
```

Expected: `list_empty_returns_empty_vec` and `round_trip_preserves_effects_and_tags` PASS (both use only `db_list`, which is implemented).

- [ ] **Step 4: Add the failing tests for `db_add` + `db_update` + `db_delete`**

Append to `tests` mod:

```rust
fn sample_new() -> NewStatusTemplate {
    NewStatusTemplate {
        name: "Slippery".to_string(),
        description: "Hard to grapple".to_string(),
        effects: vec![ModifierEffect {
            kind: ModifierKind::Difficulty,
            scope: Some("grapple".to_string()),
            delta: Some(2),
            note: None,
        }],
        tags: vec!["Combat".to_string()],
    }
}

#[tokio::test]
async fn add_inserts_and_returns_full_record() {
    let pool = fresh_pool().await;
    let t = db_add(&pool, sample_new()).await.unwrap();
    assert!(t.id > 0);
    assert_eq!(t.name, "Slippery");
    assert_eq!(t.effects.len(), 1);
    assert_eq!(t.tags, vec!["Combat".to_string()]);
}

#[tokio::test]
async fn add_rejects_empty_name() {
    let pool = fresh_pool().await;
    let mut new = sample_new();
    new.name = String::new();
    let err = db_add(&pool, new).await.unwrap_err();
    assert!(err.contains("empty name"), "got: {err}");
}

#[tokio::test]
async fn update_applies_partial_patch() {
    let pool = fresh_pool().await;
    let t = db_add(&pool, sample_new()).await.unwrap();
    let updated = db_update(&pool, t.id, StatusTemplatePatch {
        name: Some("Renamed".into()), description: None, effects: None, tags: None,
    }).await.unwrap();
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.effects.len(), 1); // untouched
}

#[tokio::test]
async fn update_missing_id_errors() {
    let pool = fresh_pool().await;
    let err = db_update(&pool, 9999, StatusTemplatePatch {
        name: Some("X".into()), description: None, effects: None, tags: None,
    }).await.unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}

#[tokio::test]
async fn delete_removes_row() {
    let pool = fresh_pool().await;
    let t = db_add(&pool, sample_new()).await.unwrap();
    db_delete(&pool, t.id).await.unwrap();
    assert!(db_list(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn delete_missing_id_errors() {
    let pool = fresh_pool().await;
    let err = db_delete(&pool, 9999).await.unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}
```

- [ ] **Step 5: Run to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- status_template
```

Expected: 6 new tests fail (`db_add`, `db_update`, `db_delete` not yet defined).

- [ ] **Step 6: Implement `db_add` / `db_update` / `db_delete` + their commands**

Insert into `src-tauri/src/db/status_template.rs` between the existing `list_status_templates` command and the `#[cfg(test)]` block:

```rust
pub(crate) async fn db_add(
    pool: &SqlitePool,
    input: NewStatusTemplate,
) -> Result<StatusTemplate, String> {
    if input.name.trim().is_empty() {
        return Err("db/status_template.add: empty name".to_string());
    }
    let effects_json = serde_json::to_string(&input.effects)
        .map_err(|e| format!("db/status_template.add: serialize effects: {e}"))?;
    let tags_json = serde_json::to_string(&input.tags)
        .map_err(|e| format!("db/status_template.add: serialize tags: {e}"))?;

    let result = sqlx::query(
        "INSERT INTO status_templates (name, description, effects_json, tags_json)
         VALUES (?, ?, ?, ?)"
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&tags_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/status_template.add: {e}"))?;
    db_get(pool, result.last_insert_rowid()).await
}

pub(crate) async fn db_update(
    pool: &SqlitePool,
    id: i64,
    patch: StatusTemplatePatch,
) -> Result<StatusTemplate, String> {
    let mut current = db_get(pool, id).await
        .map_err(|e| if e.contains("not found") { "db/status_template.update: not found".to_string() } else { format!("db/status_template.update: {e}") })?;

    if let Some(name) = patch.name {
        if name.trim().is_empty() {
            return Err("db/status_template.update: empty name".to_string());
        }
        current.name = name;
    }
    if let Some(desc) = patch.description { current.description = desc; }
    if let Some(effects) = patch.effects   { current.effects = effects; }
    if let Some(tags) = patch.tags         { current.tags = tags; }

    let effects_json = serde_json::to_string(&current.effects)
        .map_err(|e| format!("db/status_template.update: serialize effects: {e}"))?;
    let tags_json = serde_json::to_string(&current.tags)
        .map_err(|e| format!("db/status_template.update: serialize tags: {e}"))?;

    let result = sqlx::query(
        "UPDATE status_templates
         SET name = ?, description = ?, effects_json = ?, tags_json = ?,
             updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&current.name)
    .bind(&current.description)
    .bind(&effects_json)
    .bind(&tags_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/status_template.update: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/status_template.update: not found".to_string());
    }
    db_get(pool, id).await
}

pub(crate) async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM status_templates WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/status_template.delete: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/status_template.delete: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn add_status_template(
    pool: tauri::State<'_, crate::DbState>,
    input: NewStatusTemplate,
) -> Result<StatusTemplate, String> {
    db_add(&pool.0, input).await
}

#[tauri::command]
pub async fn update_status_template(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    patch: StatusTemplatePatch,
) -> Result<StatusTemplate, String> {
    db_update(&pool.0, id, patch).await
}

#[tauri::command]
pub async fn delete_status_template(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}
```

- [ ] **Step 7: Register the new db module**

Modify `src-tauri/src/db/mod.rs` — add `pub mod status_template;`.

- [ ] **Step 8: Run all status_template tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- status_template
```

Expected: PASS (8 tests total).

- [ ] **Step 9: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green. The 4 new commands will trigger Rust dead-code warnings until B2 wires them — expected.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/shared/modifier.rs src-tauri/src/db/status_template.rs src-tauri/src/db/mod.rs
git commit -m "feat(db/status_template): CRUD commands for GM Screen palette

Templates use the existing status_templates table (migrated by Plan A's
0005_modifiers.sql, inert until now). Same patch-with-load-write-back
shape as db/modifier.rs's update; same db/status_template.<op>: error
prefix convention (ARCH §7)."
```

---

## Task B2: Register commands + TS mirror + typed wrapper extension

**Goal:** Wire the 4 new commands and extend `src/lib/modifiers/api.ts` with template wrappers.

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts`
- Modify: `src/lib/modifiers/api.ts`

**Anti-scope:** Do not touch components or stores.

**Depends on:** B1.

**Invariants cited:** ARCH §4 (typed wrapper), §9 (add-a-command seam).

**Tests required:** NO — wiring; `verify.sh` is the gate.

- [ ] **Step 1: Register the 4 commands in `src-tauri/src/lib.rs`**

Add inside the `invoke_handler(tauri::generate_handler![...])` list, alongside the modifier commands:

```rust
            db::status_template::list_status_templates,
            db::status_template::add_status_template,
            db::status_template::update_status_template,
            db::status_template::delete_status_template,
```

- [ ] **Step 2: Mirror the types in `src/types.ts`**

Append to the `// GM Screen — character modifiers` block in `src/types.ts`:

```ts
export interface StatusTemplate {
  id: number;
  name: string;
  description: string;
  effects: ModifierEffect[];
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

export interface NewStatusTemplateInput {
  name: string;
  description: string;
  effects: ModifierEffect[];
  tags: string[];
}

export interface StatusTemplatePatchInput {
  name?: string;
  description?: string;
  effects?: ModifierEffect[];
  tags?: string[];
}
```

- [ ] **Step 3: Extend the typed wrapper**

Append to `src/lib/modifiers/api.ts`:

```ts
import type {
  StatusTemplate,
  NewStatusTemplateInput,
  StatusTemplatePatchInput,
} from '../../types';

export function listStatusTemplates(): Promise<StatusTemplate[]> {
  return invoke<StatusTemplate[]>('list_status_templates');
}

export function addStatusTemplate(input: NewStatusTemplateInput): Promise<StatusTemplate> {
  return invoke<StatusTemplate>('add_status_template', { input });
}

export function updateStatusTemplate(
  id: number,
  patch: StatusTemplatePatchInput,
): Promise<StatusTemplate> {
  return invoke<StatusTemplate>('update_status_template', { id, patch });
}

export function deleteStatusTemplate(id: number): Promise<void> {
  return invoke<void>('delete_status_template', { id });
}
```

> Note: Add the new types to the existing import block at the top of the file rather than duplicating the import. The example `import type { ... }` line above shows the **types being added** — fold them into the existing `import type { CharacterModifier, NewCharacterModifierInput, ModifierPatchInput, SourceKind } from '../../types';` line.

- [ ] **Step 4: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src/types.ts src/lib/modifiers/api.ts
git commit -m "feat(modifiers): register status template IPC + TS mirror + wrappers

4 new commands registered, StatusTemplate / NewStatusTemplateInput /
StatusTemplatePatchInput mirrored in src/types.ts, typed wrappers added
to src/lib/modifiers/api.ts (one shared module since templates and
modifiers are sibling concepts)."
```

---

## Task B3: statusTemplates store

**Goal:** Build the cache store mirroring `modifiers.svelte.ts` shape.

**Files:**
- Create: `src/store/statusTemplates.svelte.ts`

**Anti-scope:** Components / backend untouched.

**Depends on:** B2.

**Invariants cited:** none new — same mount-then-CRUD pattern as `modifiers.svelte.ts`.

**Tests required:** NO.

- [ ] **Step 1: Create the store**

`src/store/statusTemplates.svelte.ts`:

```ts
// GM Screen status templates runes store. Mirrors src/store/modifiers.svelte.ts
// shape: initialized flag, ensureLoaded / refresh, CRUD methods that merge
// the response row into the local list.

import {
  listStatusTemplates,
  addStatusTemplate,
  updateStatusTemplate,
  deleteStatusTemplate,
} from '$lib/modifiers/api';
import type {
  StatusTemplate,
  NewStatusTemplateInput,
  StatusTemplatePatchInput,
} from '../types';

let _list = $state<StatusTemplate[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);
let _initialized = false;

async function refresh(): Promise<void> {
  _loading = true;
  _error = null;
  try {
    _list = await listStatusTemplates();
  } catch (e) {
    _error = String(e);
    console.error('[statusTemplates] refresh failed:', e);
  } finally {
    _loading = false;
  }
}

function mergeRow(updated: StatusTemplate): void {
  const i = _list.findIndex(t => t.id === updated.id);
  if (i >= 0) _list[i] = updated; else _list.push(updated);
}

function dropRow(id: number): void {
  _list = _list.filter(t => t.id !== id);
}

export const statusTemplates = {
  get list() { return _list; },
  get loading() { return _loading; },
  get error() { return _error; },
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
  },
  async refresh(): Promise<void> { await refresh(); },
  async add(input: NewStatusTemplateInput): Promise<StatusTemplate> {
    const row = await addStatusTemplate(input);
    mergeRow(row);
    return row;
  },
  async update(id: number, patch: StatusTemplatePatchInput): Promise<StatusTemplate> {
    const row = await updateStatusTemplate(id, patch);
    mergeRow(row);
    return row;
  },
  async delete(id: number): Promise<void> {
    await deleteStatusTemplate(id);
    dropRow(id);
  },
  byId(id: number): StatusTemplate | undefined {
    return _list.find(t => t.id === id);
  },
};
```

- [ ] **Step 2: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/store/statusTemplates.svelte.ts
git commit -m "feat(store/statusTemplates): runes store mirroring modifiers store"
```

---

## Task B4: StatusTemplateEditor component

**Goal:** Side-pane authoring form for one template. Reuses the multi-effect-row + chip editor pattern from `ModifierEffectEditor.svelte`.

**Subagent dispatch hint:** Frontend-design candidate. Reference `src/lib/components/gm-screen/ModifierEffectEditor.svelte` (Plan A) — almost the same shape but with name + description fields prepended, and submit semantics that call `statusTemplates.add` / `.update` instead of returning effects to the parent.

**Files:**
- Create: `src/lib/components/gm-screen/StatusTemplateEditor.svelte`

**Anti-scope:** Do not touch other components, backend.

**Depends on:** B3.

**Invariants cited:** ARCH §6 (CSS tokens, rem, no hex literals).

**Tests required:** NO — UI.

- [ ] **Step 1: Create the component**

```svelte
<script lang="ts">
  import { statusTemplates } from '../../../store/statusTemplates.svelte';
  import type { ModifierEffect, ModifierKind, StatusTemplate } from '../../../types';

  interface Props {
    /** Existing template to edit, or null to author a new one. */
    existing: StatusTemplate | null;
    onClose: () => void;
  }

  let { existing, onClose }: Props = $props();

  let name = $state(existing?.name ?? '');
  let description = $state(existing?.description ?? '');
  let effects = $state<ModifierEffect[]>(
    (existing?.effects ?? []).map(e => ({ ...e }))
  );
  let tags = $state<string[]>([...(existing?.tags ?? [])]);
  let newTag = $state('');
  let saving = $state(false);
  let error = $state<string | null>(null);

  const KINDS: { value: ModifierKind; label: string }[] = [
    { value: 'pool',       label: 'Pool' },
    { value: 'difficulty', label: 'Difficulty' },
    { value: 'note',       label: 'Note' },
  ];

  function addEffect() {
    effects = [...effects, { kind: 'pool', scope: null, delta: 0, note: null }];
  }
  function removeEffect(i: number) { effects = effects.filter((_, idx) => idx !== i); }
  function bumpDelta(i: number, by: number) {
    const cur = effects[i].delta ?? 0;
    effects[i] = { ...effects[i], delta: Math.max(-10, Math.min(10, cur + by)) };
  }
  function setKind(i: number, kind: ModifierKind) {
    effects[i] = kind === 'note'
      ? { ...effects[i], kind, delta: null }
      : { ...effects[i], kind, note: null };
  }
  function commitTag() {
    const t = newTag.trim();
    if (!t || tags.includes(t)) { newTag = ''; return; }
    tags = [...tags, t];
    newTag = '';
  }
  function removeTag(t: string) { tags = tags.filter(x => x !== t); }

  async function save() {
    if (!name.trim()) { error = 'Name required'; return; }
    saving = true;
    error = null;
    try {
      if (existing) {
        await statusTemplates.update(existing.id, { name, description, effects, tags });
      } else {
        await statusTemplates.add({ name, description, effects, tags });
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function del() {
    if (!existing) return;
    if (!confirm(`Delete template "${existing.name}"?`)) return;
    saving = true;
    try {
      await statusTemplates.delete(existing.id);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<aside class="editor" role="dialog" aria-label="Edit status template">
  <header>
    <h3>{existing ? 'Edit template' : 'New template'}</h3>
    <button class="close" onclick={onClose} aria-label="Close">×</button>
  </header>

  <label>
    <span>Name</span>
    <input
      type="text"
      value={name}
      oninput={(e) => name = (e.currentTarget as HTMLInputElement).value}
    />
  </label>
  <label>
    <span>Description</span>
    <textarea
      rows="2"
      value={description}
      oninput={(e) => description = (e.currentTarget as HTMLTextAreaElement).value}
    ></textarea>
  </label>

  <fieldset>
    <legend>Effects</legend>
    {#each effects as effect, i (i)}
      <div class="effect-row">
        <select value={effect.kind} onchange={(e) => setKind(i, (e.currentTarget as HTMLSelectElement).value as ModifierKind)}>
          {#each KINDS as k}<option value={k.value}>{k.label}</option>{/each}
        </select>
        {#if effect.kind === 'note'}
          <input
            type="text"
            placeholder="Note text"
            value={effect.note ?? ''}
            oninput={(e) => effects[i] = { ...effects[i], note: (e.currentTarget as HTMLInputElement).value }}
          />
        {:else}
          <input
            type="text"
            placeholder="Scope (e.g. Social)"
            value={effect.scope ?? ''}
            oninput={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              effects[i] = { ...effects[i], scope: v === '' ? null : v };
            }}
          />
          <div class="stepper">
            <button onclick={() => bumpDelta(i, -1)} aria-label="Decrement">−</button>
            <span>{effect.delta ?? 0}</span>
            <button onclick={() => bumpDelta(i, 1)} aria-label="Increment">+</button>
          </div>
        {/if}
        <button class="remove" onclick={() => removeEffect(i)} aria-label="Remove effect">×</button>
      </div>
    {/each}
    <button class="add" onclick={addEffect}>+ Add effect</button>
  </fieldset>

  <fieldset>
    <legend>Tags</legend>
    <div class="tag-list">
      {#each tags as t}
        <span class="tag-chip">
          {t}
          <button onclick={() => removeTag(t)} aria-label="Remove tag {t}">×</button>
        </span>
      {/each}
      <input
        type="text"
        placeholder="+ tag"
        value={newTag}
        oninput={(e) => newTag = (e.currentTarget as HTMLInputElement).value}
        onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); commitTag(); } }}
        onblur={commitTag}
      />
    </div>
  </fieldset>

  {#if error}<p class="error">{error}</p>{/if}

  <footer>
    {#if existing}<button class="danger" onclick={del} disabled={saving}>Delete</button>{/if}
    <span class="spacer"></span>
    <button class="secondary" onclick={onClose}>Cancel</button>
    <button class="primary" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </footer>
</aside>

<style>
  .editor {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    width: 24rem;
    box-shadow: 0 0.75rem 2rem -0.25rem rgba(0,0,0,0.6);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-sizing: border-box;
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  header h3 { margin: 0; font-size: 0.95rem; color: var(--text-primary); }
  .close, .remove, .add, button.secondary, button.primary, button.danger, .stepper button {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  button.primary { background: var(--accent); color: var(--text-primary); border-color: var(--accent-bright); }
  button.danger { background: transparent; color: var(--accent-amber); border-color: var(--accent-amber); }
  label { display: flex; flex-direction: column; gap: 0.25rem; }
  label span { font-size: 0.75rem; color: var(--text-label); }
  label input, label textarea {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.3rem 0.5rem;
    font-size: 0.85rem;
    box-sizing: border-box;
    width: 100%;
  }
  fieldset { border: 1px solid var(--border-faint); border-radius: 0.4rem; padding: 0.5rem; margin: 0; }
  legend { font-size: 0.75rem; color: var(--text-label); padding: 0 0.3rem; }
  .effect-row {
    display: grid;
    grid-template-columns: 6rem 1fr auto auto;
    gap: 0.4rem;
    align-items: center;
    margin-bottom: 0.35rem;
  }
  .effect-row select, .effect-row input {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.4rem;
    font-size: 0.75rem;
    box-sizing: border-box;
    width: 100%;
  }
  .stepper { display: inline-flex; gap: 0.25rem; align-items: center; color: var(--text-primary); font-variant-numeric: tabular-nums; }
  .tag-list { display: flex; flex-wrap: wrap; gap: 0.3rem; align-items: center; }
  .tag-chip {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.7rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .tag-chip button { background: transparent; border: none; color: var(--text-muted); cursor: pointer; padding: 0; }
  .tag-list input { width: 7rem; }
  .error { color: var(--accent-amber); font-size: 0.75rem; margin: 0; }
  footer { display: flex; gap: 0.4rem; align-items: center; }
  .spacer { flex: 1; }
</style>
```

- [ ] **Step 2: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/StatusTemplateEditor.svelte
git commit -m "feat(gm-screen): StatusTemplateEditor side-pane

Authoring form for one template — name + description + multi-effect rows
+ tag chip editor + delete-when-existing footer button. Reuses the
multi-effect / chip editor shape from ModifierEffectEditor (Plan A) but
calls statusTemplates store CRUD directly rather than emitting changes
upward."
```

---

## Task B5: StatusPaletteDock + GmScreen wiring + provenance display

**Goal:** Land the right-side palette dock, wire it into `GmScreen.svelte`'s layout, plug the click-to-apply flow, and show `originTemplateId` provenance on instance cards.

**Files:**
- Create: `src/lib/components/gm-screen/StatusPaletteDock.svelte`
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (add `originTemplateName` subtitle when provenance is set)
- Modify: `src/tools/GmScreen.svelte` (two-column layout, focused-character tracking)

**Anti-scope:** Do not touch backend, db, store internals (only consume).

**Depends on:** B4.

**Invariants cited:** ARCH §6 (tokens, rem, no hex literals); spec §7.1 (page layout — dock on the right), §7.4 (palette dock interaction model — click-to-apply with focused-character convention), §8.4 (apply flow builds NewCharacterModifier with `origin_template_id`).

**Tests required:** NO — UI; manual smoke at Step 4 is the gate.

- [ ] **Step 1: Create `StatusPaletteDock.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { statusTemplates } from '../../../store/statusTemplates.svelte';
  import { modifiers } from '../../../store/modifiers.svelte';
  import StatusTemplateEditor from './StatusTemplateEditor.svelte';
  import type { StatusTemplate, BridgeCharacter } from '../../../types';

  interface Props {
    /**
     * Currently focused character (last-clicked row), or null if none.
     * Click-to-apply is gated on this — clicking a template with no focus
     * surfaces a hint instead of silently applying to the topmost character.
     */
    focusedCharacter: BridgeCharacter | null;
  }

  let { focusedCharacter }: Props = $props();

  let editorOpen = $state(false);
  let editorExisting = $state<StatusTemplate | null>(null);
  let applyHint = $state<string | null>(null);

  onMount(() => { void statusTemplates.ensureLoaded(); });

  async function applyTemplate(t: StatusTemplate): Promise<void> {
    if (!focusedCharacter) {
      applyHint = 'Click a character row first, then a template.';
      setTimeout(() => { applyHint = null; }, 2500);
      return;
    }
    // Spec §8.4: independent copy. structuredClone the effects array so the
    // new modifier doesn't share references with the template.
    await modifiers.add({
      source: focusedCharacter.source,
      sourceId: focusedCharacter.source_id,
      name: t.name,
      description: t.description,
      effects: structuredClone(t.effects),
      binding: { kind: 'free' },
      tags: [...t.tags],
      originTemplateId: t.id,
    });
    applyHint = `Applied "${t.name}" to ${focusedCharacter.name}.`;
    setTimeout(() => { applyHint = null; }, 2000);
  }

  function openNewEditor() {
    editorExisting = null;
    editorOpen = true;
  }

  function openEditEditor(t: StatusTemplate) {
    editorExisting = t;
    editorOpen = true;
  }

  function summarize(t: StatusTemplate): string {
    if (t.effects.length === 0) return '(no effects)';
    return t.effects.map(e => {
      if (e.kind === 'note') return e.note ?? 'note';
      const sign = (e.delta ?? 0) >= 0 ? '+' : '';
      const what = e.kind === 'pool' ? 'dice' : 'diff';
      return `${e.scope ? e.scope + ' ' : ''}${sign}${e.delta ?? 0} ${what}`;
    }).join(' · ');
  }
</script>

<aside class="palette">
  <header>
    <h2>Status palette</h2>
    <button class="new" onclick={openNewEditor}>+ New template</button>
  </header>

  {#if applyHint}<p class="hint">{applyHint}</p>{/if}
  {#if !focusedCharacter}<p class="hint muted">Click a character row to enable template apply.</p>{/if}

  {#if statusTemplates.list.length === 0}
    <p class="empty">No templates yet. Click <strong>+ New template</strong>.</p>
  {:else}
    <div class="grid">
      {#each statusTemplates.list as t (t.id)}
        <div class="template">
          <button class="apply" onclick={() => applyTemplate(t)} disabled={!focusedCharacter} title={focusedCharacter ? `Apply to ${focusedCharacter.name}` : 'Pick a character first'}>
            <span class="name">{t.name}</span>
            <span class="summary">{summarize(t)}</span>
            {#if t.tags.length > 0}
              <span class="tags">
                {#each t.tags as tag}<span class="tag">#{tag}</span>{/each}
              </span>
            {/if}
          </button>
          <button class="edit" title="Edit template" onclick={() => openEditEditor(t)}>✎</button>
        </div>
      {/each}
    </div>
  {/if}

  {#if editorOpen}
    <div class="editor-overlay">
      <StatusTemplateEditor existing={editorExisting} onClose={() => { editorOpen = false; editorExisting = null; }} />
    </div>
  {/if}
</aside>

<style>
  .palette {
    background: var(--bg-card);
    border-left: 1px solid var(--border-faint);
    padding: 0.85rem;
    width: 18rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    overflow-y: auto;
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  header h2 { margin: 0; font-size: 0.85rem; color: var(--text-label); }
  .new {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    cursor: pointer;
  }
  .hint { font-size: 0.7rem; color: var(--accent-amber); margin: 0; }
  .hint.muted { color: var(--text-muted); }
  .empty { color: var(--text-muted); font-size: 0.75rem; font-style: italic; }
  .grid { display: flex; flex-direction: column; gap: 0.4rem; }
  .template {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.3rem;
    align-items: stretch;
  }
  .apply {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 0.4rem;
    padding: 0.45rem 0.6rem;
    text-align: left;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .apply:hover:not(:disabled) { border-color: var(--accent-bright); background: var(--bg-active); }
  .apply:disabled { opacity: 0.5; cursor: not-allowed; }
  .apply .name { font-size: 0.8rem; font-weight: 500; }
  .apply .summary { font-size: 0.7rem; color: var(--text-secondary); }
  .apply .tags { display: flex; flex-wrap: wrap; gap: 0.2rem; margin-top: 0.15rem; }
  .apply .tag { font-size: 0.6rem; color: var(--text-muted); }
  .edit {
    background: transparent;
    border: 1px solid var(--border-faint);
    color: var(--text-muted);
    border-radius: 0.4rem;
    padding: 0 0.4rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .edit:hover { color: var(--text-primary); border-color: var(--border-surface); }

  .editor-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }
</style>
```

- [ ] **Step 2: Add provenance display to `ModifierCard.svelte`**

Modify `src/lib/components/gm-screen/ModifierCard.svelte`:

1. Extend the `Props` interface — add an optional `originTemplateName: string | null = null`. The parent (`CharacterRow`) looks this up via `statusTemplates.byId(modifier.originTemplateId)?.name` when `modifier.originTemplateId != null` and passes it down.

2. In the markup, between the `<div class="head">` and `<div class="effects">` blocks, add:

```svelte
{#if originTemplateName}
  <p class="origin">from "{originTemplateName}"</p>
{/if}
```

3. In `<style>`, add:

```css
.origin {
  margin: 0;
  font-size: 0.6rem;
  color: var(--text-muted);
  font-style: italic;
}
```

- [ ] **Step 3: Wire `originTemplateName` through `CharacterRow.svelte`**

Modify `src/lib/components/gm-screen/CharacterRow.svelte`:

1. Add import at the top of `<script>`: `import { statusTemplates } from '../../../store/statusTemplates.svelte';`

2. In the `{#each visibleCards as entry, i ...}` `<ModifierCard ... />` invocation, add the prop:

```svelte
originTemplateName={entry.kind === 'materialized' && entry.mod.originTemplateId != null
  ? (statusTemplates.byId(entry.mod.originTemplateId)?.name ?? null)
  : null}
```

- [ ] **Step 4: Wire dock + focused-character tracking into `GmScreen.svelte`**

Modify `src/tools/GmScreen.svelte`:

1. Add imports: `import { statusTemplates } from '../store/statusTemplates.svelte';` and `import StatusPaletteDock from '$lib/components/gm-screen/StatusPaletteDock.svelte';`

2. In `onMount`, add: `void statusTemplates.ensureLoaded();`

3. Add focused-character state:

```ts
let focusedCharacter = $state<BridgeCharacter | null>(null);
```

4. Restructure the layout into two columns (rows on the left, dock on the right). Replace the existing `<div class="rows">…</div>` block with:

```svelte
<div class="layout">
  <div class="rows">
    {#if displayCharacters.length === 0}
      <p class="empty">No characters available. Connect Foundry or Roll20, or load a saved character.</p>
    {:else}
      {#each displayCharacters as char (`${char.source}:${char.source_id}`)}
        <button
          class="row-focus-wrap"
          class:focused={focusedCharacter && focusedCharacter.source === char.source && focusedCharacter.source_id === char.source_id}
          onclick={() => focusedCharacter = char}
        >
          <CharacterRow
            character={char}
            activeFilterTags={modifiers.activeFilterTags}
            showHidden={modifiers.showHidden}
          />
        </button>
      {/each}
    {/if}

    {#if modifiers.showOrphans && orphans.length > 0}
      <section class="orphans">
        <h2>Orphans ({orphans.length})</h2>
        <p class="hint">Modifier rows whose character isn't currently live or saved.</p>
        {#each orphans as o}
          <div class="orphan-row">
            <span>{o.name}</span>
            <span class="meta">{o.source}:{o.sourceId}</span>
            <button onclick={() => modifiers.delete(o.id)}>Delete</button>
          </div>
        {/each}
      </section>
    {/if}
  </div>

  <StatusPaletteDock {focusedCharacter} />
</div>
```

5. Add styles:

```css
.layout { display: flex; flex: 1; min-height: 0; }
.rows { flex: 1; overflow-y: auto; padding: 0.75rem 1rem; }
.row-focus-wrap {
  display: block;
  width: 100%;
  background: transparent;
  border: 2px solid transparent;
  border-radius: 0.55rem;
  padding: 0;
  margin-bottom: 0.6rem;
  cursor: pointer;
  text-align: left;
}
.row-focus-wrap.focused { border-color: var(--accent-bright); }
```

The previous `.rows` block needs its `margin-bottom: 0.6rem;` rule on `.row` removed (now lives on `.row-focus-wrap`) — leave the rest of `.rows` styling alone.

- [ ] **Step 5: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 6: Manual smoke test (per spec §10 Plan B verification)**

```bash
npm run tauri dev
```

In the GM Screen tool:
1. Right-side dock shows `Status palette` heading, `+ New template` button, and (initially) the empty hint.
2. Click `+ New template` → editor overlay opens. Enter name `Slippery`, description `Hard to grapple`, add an effect (Difficulty, scope `grapple`, +2), add tag `Combat`, save → editor closes; `Slippery` appears in the dock with summary `grapple +2 diff` and tag chip.
3. Click a character row → row gains a focus border; the dock's hint vanishes; template buttons become enabled.
4. Click `Slippery` → confirmation hint appears (`Applied "Slippery" to <name>.`); the modifier row gains a new `Slippery` card showing `from "Slippery"` subtitle (provenance), the configured effect, and the tag chip. Card is independent — it's a free-binding card with `originTemplateId` set.
5. Open the `Slippery` template's `✎` edit button → editor reopens with current values; change name to `Slippery (v2)`; save. Verify the **already-applied** modifier on the character STILL shows `from "Slippery (v2)"` (because provenance is by id, looked up live) but its name does NOT change (independent copy — spec §8.4).
6. Delete the template via the editor's `Delete` button → confirm → template vanishes from the dock; the existing modifier on the character KEEPS its name + effects (provenance lookup returns null, the `from "..."` subtitle disappears).

If any step fails, fix before committing.

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/gm-screen/StatusPaletteDock.svelte \
        src/lib/components/gm-screen/ModifierCard.svelte \
        src/lib/components/gm-screen/CharacterRow.svelte \
        src/tools/GmScreen.svelte
git commit -m "feat(gm-screen): status palette dock + provenance display

Closes Plan B of GM Screen stage 1. Right-side dock with click-to-apply
(focused-character convention per spec §13). Apply path builds a
NewCharacterModifier with originTemplateId set — provenance flows
through the input shape, no separate apply-template command (spec §5).
Templates render an independent copy of effects/tags on apply
(structuredClone), so subsequent template edits do NOT propagate to
already-applied instances (spec §8.4). Provenance subtitle on the
ModifierCard looks up the template name live by id."
```

---

## Plan B self-review checklist

After completing all 5 tasks:

**1. Spec coverage** — every Plan B requirement implemented?

| Spec section | Implemented in |
|---|---|
| §3 status template instance binding | B5 (apply path builds free-binding modifier with originTemplateId) |
| §4 StatusTemplate type + schema | B1 (types), Plan A migration (schema) |
| §5 4 status_template commands + no apply_template_to_character (provenance via input shape) | B1 (commands), B5 (apply path uses add_character_modifier) |
| §6 frontend file inventory (templates) | B2 (api.ts ext, types.ts), B3 (store), B4 + B5 (components) |
| §7.4 palette dock UX (click-to-apply, focused character) | B5 |
| §7.5 (no change — palette templates are independent of the filter) | n/a |
| §8.4 status template apply flow + independent copy | B5 (`structuredClone` of effects on apply) |
| §9 error contracts | B1 (all command errors prefixed `db/status_template.<op>:`) |
| §11 Rust tests | B1 (~8 tests inline) |

**2. Placeholder scan** — search for `TBD`, `TODO`, `placeholder`, `implement later`, `add appropriate`. Should return zero hits.

**3. Type consistency** — names referenced across Plan B tasks:
- `db_add` / `db_update` / `db_delete` / `db_get` / `db_list` — all in `src-tauri/src/db/status_template.rs` (B1)
- `addStatusTemplate` / `updateStatusTemplate` / `deleteStatusTemplate` / `listStatusTemplates` — all in `src/lib/modifiers/api.ts` (B2)
- `statusTemplates.add` / `.update` / `.delete` / `.byId` / `.list` / `.ensureLoaded` — all on the store in B3; consumed by B4 + B5
- `originTemplateId` (Rust + TS field) → looked up via `statusTemplates.byId(...)?.name` in B5 → passed as `originTemplateName` prop to ModifierCard in B5

**4. After all tasks committed:** dispatch ONE `code-review:code-review` against the full Plan B branch diff (per project lean-execution override).
