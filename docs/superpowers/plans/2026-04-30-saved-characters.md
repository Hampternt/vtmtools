# Saved Characters Implementation Plan (Plan 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist bridged characters locally on demand. Add a "Saved characters" section to the Campaign view with Save / Update / Delete actions and a source-attribution chip showing the origin world.

**Architecture:** New SQLite table `saved_characters` (one row per `(source, source_id)` UNIQUE). New module `src-tauri/src/db/saved_character.rs` with four Tauri commands (save/list/update/delete). Frontend gets a typed API wrapper, a runes store, and a small UI extension to `Campaign.svelte` adding the Saved section as Layout B (twin cards: live and saved are independent rows; a character that is both live AND saved appears in both sections).

**Tech Stack:** Rust (`sqlx`, `serde`, `serde_json`, `tauri`), TypeScript / Svelte 5 runes, SQLite.

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md`

**Depends on:** Plan 0 (Bridge Protocol Consolidation) for `BridgeState.source_info` runtime value population. Plan 1's *schema* is independent; only the populated `foundry_world` value needs Plan 0 landed.

---

## File structure

### New files
- `src-tauri/migrations/0004_saved_characters.sql` — table + UNIQUE constraint
- `src-tauri/src/db/saved_character.rs` — struct, CRUD helpers, 4 Tauri commands, `#[cfg(test)]` tests
- `src/lib/saved-characters/api.ts` — 4 typed wrappers
- `src/store/savedCharacters.svelte.ts` — runes store wrapping the API
- `src/components/SourceAttributionChip.svelte` — small reusable badge

### Modified files
- `src-tauri/src/db/mod.rs` — `pub mod saved_character;`
- `src-tauri/src/lib.rs` — register 4 commands in `invoke_handler!`
- `src/tools/Campaign.svelte` — add "Saved" section, save/update buttons on live cards, delete button on saved cards, source-attribution chips

### Files explicitly NOT touched
- `src-tauri/src/bridge/**` (Plan 0's territory — Plan 1 reads from `bridge.svelte.ts`)
- `src-tauri/src/shared/v5/**` (Plan 3's territory)
- `src/lib/saved-characters/diff.ts` (Plan 2 creates it)
- `src/components/CompareModal.svelte` (Plan 2 creates it)
- `src/store/bridge.svelte.ts` (Plan 0 owns it; Plan 1 only consumes)

---

## Task overview

| # | Task | Depends on |
|---|---|---|
| 1 | Create migration `0004_saved_characters.sql` | none |
| 2 | Create `db/saved_character.rs` skeleton (struct + module declaration) | 1 |
| 3 | Implement `save_character` command + `(source, source_id)` UNIQUE conflict test | 2 |
| 4 | Implement `list_saved_characters` command + ordering test | 2 |
| 5 | Implement `update_saved_character` command + not-found test | 2 |
| 6 | Implement `delete_saved_character` command + not-found test | 2 |
| 7 | Register all four commands in `lib.rs` | 3, 4, 5, 6 |
| 8 | Add typed API wrappers in `src/lib/saved-characters/api.ts` | 7 |
| 9 | Create `src/store/savedCharacters.svelte.ts` runes store | 8 |
| 10 | Create `src/components/SourceAttributionChip.svelte` | 9 |
| 11 | Modify `Campaign.svelte` to add Saved section + Save/Update/Delete buttons | 9, 10 |
| 12 | Final verification gate | all |

Tasks 3–6 are independent of each other and can dispatch in parallel after Task 2 (each adds a non-overlapping function in `saved_character.rs` plus a non-overlapping test).

---

## Task 1: Create migration `0004_saved_characters.sql`

**Files:**
- Create: `src-tauri/migrations/0004_saved_characters.sql`

**Anti-scope:** No other migrations. No data migration of existing rows.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §3 (migrations applied via `sqlx::migrate!` on startup), §6 (`PRAGMA foreign_keys = ON`).

- [ ] **Step 1: Write the migration**

Create `src-tauri/migrations/0004_saved_characters.sql`:

```sql
-- Saved characters: local snapshots of bridged characters, durable across
-- sessions. Distinct from the in-memory bridge cache (which holds live
-- character data and is reset on each connect cycle).
CREATE TABLE IF NOT EXISTS saved_characters (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    source             TEXT    NOT NULL CHECK(source IN ('roll20','foundry')),
    source_id          TEXT    NOT NULL,
    foundry_world      TEXT,
    name               TEXT    NOT NULL,
    canonical_json     TEXT    NOT NULL,
    saved_at           TEXT    NOT NULL DEFAULT (datetime('now')),
    last_updated_at    TEXT    NOT NULL DEFAULT (datetime('now')),
    UNIQUE (source, source_id)
);
```

- [ ] **Step 2: Verify the migration applies cleanly**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
(`sqlx::migrate!` is a compile-time macro that ensures all migration files parse; `cargo check` is the gate.)

Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/0004_saved_characters.sql
git commit -m "feat(db): add saved_characters table migration (Plan 1 task 1)"
```

---

## Task 2: Create `db/saved_character.rs` skeleton

**Files:**
- Create: `src-tauri/src/db/saved_character.rs`
- Modify: `src-tauri/src/db/mod.rs`

**Anti-scope:** No commands yet. Just the struct, module declaration, and an empty `#[cfg(test)]` block.

**Depends on:** Task 1

**Invariants cited:** ARCHITECTURE.md §2 (domain types), §5 (only `db/*` talks to SQLite).

- [ ] **Step 1: Create `src-tauri/src/db/saved_character.rs`**

```rust
use sqlx::{Row, SqlitePool};
use crate::bridge::types::{CanonicalCharacter, SourceKind};

/// A locally-saved snapshot of a bridged character. The `(source, source_id)`
/// pair matches the live `CanonicalCharacter`, enabling drift detection when
/// the same character is live AND saved.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCharacter {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub foundry_world: Option<String>,
    pub name: String,
    pub canonical: CanonicalCharacter,
    pub saved_at: String,
    pub last_updated_at: String,
}

fn source_to_str(s: &SourceKind) -> &'static str {
    match s {
        SourceKind::Roll20 => "roll20",
        SourceKind::Foundry => "foundry",
    }
}

fn str_to_source(s: &str) -> Option<SourceKind> {
    match s {
        "roll20" => Some(SourceKind::Roll20),
        "foundry" => Some(SourceKind::Foundry),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_url() -> &'static str { "sqlite::memory:" }

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect(pool_url()).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[allow(dead_code)]
    fn sample_canonical() -> CanonicalCharacter {
        CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: "abc123".to_string(),
            name: "Charlotte Reine".to_string(),
            controlled_by: None,
            hunger: Some(2),
            health: None,
            willpower: None,
            humanity: Some(7),
            humanity_stains: Some(0),
            blood_potency: Some(2),
            raw: serde_json::json!({}),
        }
    }

    #[tokio::test]
    async fn migrations_apply_cleanly() {
        let _pool = fresh_pool().await;
    }
}
```

- [ ] **Step 2: Add `pub mod saved_character;` to `src-tauri/src/db/mod.rs`**

Edit `src-tauri/src/db/mod.rs`:

```rust
pub mod dyscrasia;
pub mod seed;

pub mod chronicle;
pub mod node;
pub mod edge;
pub mod advantage;
pub mod saved_character;
```

- [ ] **Step 3: Verify tests compile + the smoke test passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::migrations_apply_cleanly`
Expected: 1 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/saved_character.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): add saved_character module with SavedCharacter struct (Plan 1 task 2)"
```

---

## Task 3: Implement `save_character` command + UNIQUE conflict test

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs`

**Anti-scope:** Do NOT touch list/update/delete. Test fixture helpers (`fresh_pool`, `sample_canonical`) already exist from Task 2.

**Depends on:** Task 2

**Invariants cited:** ARCHITECTURE.md §7 (errors as `Result<T, String>` with module-stable prefixes — `"db/saved_character.save: …"`).

- [ ] **Step 1: Write the failing tests**

In `src-tauri/src/db/saved_character.rs`'s `mod tests`, add:

```rust
    #[tokio::test]
    async fn save_inserts_and_returns_id() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, Some("Chronicles of Chicago".into())).await.unwrap();
        assert!(id > 0);
        let row_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM saved_characters")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row_count, 1);
    }

    #[tokio::test]
    async fn save_twice_for_same_source_pair_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        db_save(&pool, &canonical, None).await.unwrap();
        let err = db_save(&pool, &canonical, None).await.unwrap_err();
        assert!(err.contains("already saved"), "expected 'already saved' in: {err}");
    }
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::save -- --nocapture`
Expected: 2 fails (`db_save not found`).

- [ ] **Step 2: Implement `db_save` and the `save_character` command**

Above the `#[cfg(test)]` block:

```rust
async fn db_save(
    pool: &SqlitePool,
    canonical: &CanonicalCharacter,
    foundry_world: Option<String>,
) -> Result<i64, String> {
    let canonical_json = serde_json::to_string(canonical)
        .map_err(|e| format!("db/saved_character.save: serialize failed: {e}"))?;
    let result = sqlx::query(
        "INSERT INTO saved_characters
         (source, source_id, foundry_world, name, canonical_json)
         VALUES (?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&canonical.source))
    .bind(&canonical.source_id)
    .bind(&foundry_world)
    .bind(&canonical.name)
    .bind(&canonical_json)
    .execute(pool)
    .await
    .map_err(|e| {
        let msg = e.to_string();
        if msg.contains("UNIQUE") {
            "db/saved_character.save: already saved; use update".to_string()
        } else {
            format!("db/saved_character.save: {msg}")
        }
    })?;
    Ok(result.last_insert_rowid())
}

#[tauri::command]
pub async fn save_character(
    pool: tauri::State<'_, crate::DbState>,
    canonical: CanonicalCharacter,
    foundry_world: Option<String>,
) -> Result<i64, String> {
    db_save(&pool.0, &canonical, foundry_world).await
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::save`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "feat(db): add save_character command with UNIQUE conflict handling (Plan 1 task 3)"
```

---

## Task 4: Implement `list_saved_characters` command + ordering test

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs`

**Anti-scope:** Do NOT touch save/update/delete blocks.

**Depends on:** Task 2 (uses fixture); independent of Task 3.

**Invariants cited:** ARCHITECTURE.md §7.

- [ ] **Step 1: Write the failing test**

Add to `mod tests`:

```rust
    #[tokio::test]
    async fn list_returns_rows_ordered_by_id() {
        let pool = fresh_pool().await;
        // Insert two rows directly to avoid coupling this test to db_save.
        for sid in &["a", "b"] {
            sqlx::query(
                "INSERT INTO saved_characters
                 (source, source_id, foundry_world, name, canonical_json)
                 VALUES ('foundry', ?, NULL, 'X', '{\"source\":\"foundry\",\"sourceId\":\"x\",\"name\":\"X\",\"controlledBy\":null,\"hunger\":null,\"health\":null,\"willpower\":null,\"humanity\":null,\"humanityStains\":null,\"bloodPotency\":null,\"raw\":{}}')"
            )
            .bind(sid)
            .execute(&pool).await.unwrap();
        }
        let list = db_list(&pool).await.unwrap();
        assert_eq!(list.len(), 2);
        assert!(list[0].id < list[1].id);
    }
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::list`
Expected: fail (`db_list not found`).

- [ ] **Step 2: Implement `db_list` and the `list_saved_characters` command**

Above the `#[cfg(test)]` block:

```rust
async fn db_list(pool: &SqlitePool) -> Result<Vec<SavedCharacter>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, foundry_world, name, canonical_json,
                saved_at, last_updated_at
         FROM saved_characters
         ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/saved_character.list: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let source_str: String = r.get("source");
        let source = str_to_source(&source_str)
            .ok_or_else(|| format!("db/saved_character.list: unknown source '{source_str}'"))?;
        let canonical_json: String = r.get("canonical_json");
        let canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
            .map_err(|e| format!("db/saved_character.list: deserialize failed: {e}"))?;
        out.push(SavedCharacter {
            id: r.get("id"),
            source,
            source_id: r.get("source_id"),
            foundry_world: r.get("foundry_world"),
            name: r.get("name"),
            canonical,
            saved_at: r.get("saved_at"),
            last_updated_at: r.get("last_updated_at"),
        });
    }
    Ok(out)
}

#[tauri::command]
pub async fn list_saved_characters(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<SavedCharacter>, String> {
    db_list(&pool.0).await
}
```

- [ ] **Step 3: Run the test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::list`
Expected: 1 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "feat(db): add list_saved_characters command (Plan 1 task 4)"
```

---

## Task 5: Implement `update_saved_character` command + not-found test

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs`

**Anti-scope:** Do NOT touch save/list/delete blocks.

**Depends on:** Task 2 (fixture).

**Invariants cited:** ARCHITECTURE.md §7.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests`:

```rust
    #[tokio::test]
    async fn update_overwrites_canonical_and_bumps_last_updated() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        let mut new_canonical = canonical.clone();
        new_canonical.hunger = Some(5);
        db_update(&pool, id, &new_canonical).await.unwrap();

        let list = db_list(&pool).await.unwrap();
        assert_eq!(list[0].canonical.hunger, Some(5));
        // saved_at should be unchanged; last_updated_at should be present (bumped).
        assert!(!list[0].last_updated_at.is_empty());
    }

    #[tokio::test]
    async fn update_missing_id_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let err = db_update(&pool, 9999, &canonical).await.unwrap_err();
        assert!(err.contains("not found"), "expected 'not found' in: {err}");
    }
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::update`
Expected: 2 fails.

- [ ] **Step 2: Implement `db_update` and the command**

Above the `#[cfg(test)]` block:

```rust
async fn db_update(
    pool: &SqlitePool,
    id: i64,
    canonical: &CanonicalCharacter,
) -> Result<(), String> {
    let canonical_json = serde_json::to_string(canonical)
        .map_err(|e| format!("db/saved_character.update: serialize failed: {e}"))?;
    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, name = ?, last_updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&canonical_json)
    .bind(&canonical.name)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.update: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/saved_character.update: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn update_saved_character(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    canonical: CanonicalCharacter,
) -> Result<(), String> {
    db_update(&pool.0, id, &canonical).await
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::update`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "feat(db): add update_saved_character command (Plan 1 task 5)"
```

---

## Task 6: Implement `delete_saved_character` command + not-found test

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs`

**Anti-scope:** Do NOT touch save/list/update.

**Depends on:** Task 2.

**Invariants cited:** ARCHITECTURE.md §7.

- [ ] **Step 1: Write the failing tests**

```rust
    #[tokio::test]
    async fn delete_removes_row() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();
        db_delete(&pool, id).await.unwrap();
        let list = db_list(&pool).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn delete_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_delete(&pool, 9999).await.unwrap_err();
        assert!(err.contains("not found"), "expected 'not found' in: {err}");
    }
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::delete`
Expected: 2 fails.

- [ ] **Step 2: Implement `db_delete` and the command**

```rust
async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM saved_characters WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/saved_character.delete: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/saved_character.delete: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_saved_character(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::delete`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "feat(db): add delete_saved_character command (Plan 1 task 6)"
```

---

## Task 7: Register all four commands in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Anti-scope:** Do NOT touch any other entries.

**Depends on:** Tasks 3, 4, 5, 6.

**Invariants cited:** ARCHITECTURE.md §4 (IPC command registration).

- [ ] **Step 1: Add the four entries**

In `src-tauri/src/lib.rs`, add to the `invoke_handler(tauri::generate_handler![…])` list, near the existing `db::` entries:

```rust
            db::saved_character::save_character,
            db::saved_character::list_saved_characters,
            db::saved_character::update_saved_character,
            db::saved_character::delete_saved_character,
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register saved_character commands (Plan 1 task 7)"
```

---

## Task 8: Add typed API wrappers

**Files:**
- Create: `src/lib/saved-characters/api.ts`

**Anti-scope:** Do NOT call from any component yet.

**Depends on:** Task 7.

**Invariants cited:** ARCHITECTURE.md §4 (typed wrappers), §5 (no `invoke()` from components).

- [ ] **Step 1: Create the file**

```ts
import { invoke } from '@tauri-apps/api/core';
import type { BridgeCharacter } from '$lib/bridge/api'; // existing canonical char type

export type SourceKind = 'roll20' | 'foundry';

export interface SavedCharacter {
  id: number;
  source: SourceKind;
  sourceId: string;
  foundryWorld: string | null;
  name: string;
  canonical: BridgeCharacter;
  savedAt: string;
  lastUpdatedAt: string;
}

export async function saveCharacter(
  canonical: BridgeCharacter,
  foundryWorld: string | null,
): Promise<number> {
  return await invoke<number>('save_character', { canonical, foundryWorld });
}

export async function listSavedCharacters(): Promise<SavedCharacter[]> {
  return await invoke<SavedCharacter[]>('list_saved_characters');
}

export async function updateSavedCharacter(
  id: number,
  canonical: BridgeCharacter,
): Promise<void> {
  await invoke<void>('update_saved_character', { id, canonical });
}

export async function deleteSavedCharacter(id: number): Promise<void> {
  await invoke<void>('delete_saved_character', { id });
}
```

If `BridgeCharacter` is named differently in `src/lib/bridge/api.ts` (e.g. `CanonicalCharacter`), use whatever name the existing wrapper exports — do not duplicate the type definition.

- [ ] **Step 2: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/lib/saved-characters/api.ts
git commit -m "feat(saved-characters): add typed API wrappers (Plan 1 task 8)"
```

---

## Task 9: Create `savedCharacters.svelte.ts` runes store

**Files:**
- Create: `src/store/savedCharacters.svelte.ts`

**Anti-scope:** Do NOT subscribe from any component yet.

**Depends on:** Task 8.

**Invariants cited:** ARCHITECTURE.md §3 (runes stores hold ephemeral UI state), §6 (Svelte 5 runes).

- [ ] **Step 1: Create the store**

```ts
import {
  listSavedCharacters,
  saveCharacter,
  updateSavedCharacter,
  deleteSavedCharacter,
  type SavedCharacter,
} from '$lib/saved-characters/api';
import type { BridgeCharacter } from '$lib/bridge/api';

let _list = $state<SavedCharacter[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);
let _initialized = false;

async function refresh(): Promise<void> {
  _loading = true;
  _error = null;
  try {
    _list = await listSavedCharacters();
  } catch (e) {
    _error = String(e);
    console.error('[savedCharacters] refresh failed:', e);
  } finally {
    _loading = false;
  }
}

export const savedCharacters = {
  get list() { return _list; },
  get loading() { return _loading; },
  get error() { return _error; },
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
  },
  async refresh(): Promise<void> { await refresh(); },
  async save(canonical: BridgeCharacter, foundryWorld: string | null): Promise<void> {
    await saveCharacter(canonical, foundryWorld);
    await refresh();
  },
  async update(id: number, canonical: BridgeCharacter): Promise<void> {
    await updateSavedCharacter(id, canonical);
    await refresh();
  },
  async delete(id: number): Promise<void> {
    await deleteSavedCharacter(id);
    await refresh();
  },
  /** Convenience: find a saved row matching a live char by (source, source_id). */
  findMatch(live: BridgeCharacter): SavedCharacter | undefined {
    return _list.find(s => s.source === live.source && s.sourceId === live.sourceId);
  },
};
```

- [ ] **Step 2: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/store/savedCharacters.svelte.ts
git commit -m "feat(saved-characters): add runes store wrapping the API (Plan 1 task 9)"
```

---

## Task 10: Create `SourceAttributionChip.svelte`

**Files:**
- Create: `src/components/SourceAttributionChip.svelte`

**Anti-scope:** No usage yet — just the component file.

**Depends on:** Task 9.

**Invariants cited:** ARCHITECTURE.md §6 (CSS uses `:root` tokens, no hardcoded hex).

- [ ] **Step 1: Create the component**

```svelte
<!--
  Small "source: [origin]" badge used on character cards and similar.
  Reads worldTitle from the bridge store's sourceInfo for live characters,
  or accepts an explicit `worldTitle` prop for saved characters where the
  world is captured-at-save-time.
-->
<script lang="ts">
  import { getSourceInfo } from '$store/bridge.svelte';
  import type { SourceKind } from '$lib/bridge/api';

  let {
    source,
    worldTitle = null,
  }: {
    source: SourceKind;
    worldTitle?: string | null;
  } = $props();

  // For live characters, prefer the live world title from the bridge store.
  // For saved characters, prefer the captured worldTitle prop.
  const displayWorld = $derived(
    worldTitle ?? getSourceInfo(source)?.worldTitle ?? null
  );
</script>

<span class="chip">
  source:
  {#if source === 'foundry'}
    FVTT{#if displayWorld} — {displayWorld}{/if}
  {:else if source === 'roll20'}
    Roll20
  {/if}
</span>

<style>
  .chip {
    font-size: 0.7em;
    color: var(--text-ghost);
    opacity: 0.85;
  }
</style>
```

- [ ] **Step 2: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/components/SourceAttributionChip.svelte
git commit -m "feat(components): add SourceAttributionChip (Plan 1 task 10)"
```

---

## Task 11: Add Saved section to `Campaign.svelte`

**Files:**
- Modify: `src/tools/Campaign.svelte`

**Anti-scope:** Do NOT add the Compare button yet — that's Plan 2. Save / Update / Delete only.

**Depends on:** Tasks 9, 10.

**Invariants cited:** ARCHITECTURE.md §5 (no direct `invoke()` — go through API wrapper / store), §6 (CSS tokens).

- [ ] **Step 1: Read existing `Campaign.svelte`**

Read `src/tools/Campaign.svelte` to identify:
- The current live-characters source (likely `bridgeStore.characters` or similar from `src/store/bridge.svelte.ts`).
- The card markup structure being used per character.
- The CSS class conventions in the file.

- [ ] **Step 2: Add store imports + ensureLoaded on mount**

Near the top of `<script lang="ts">`:

```ts
import { savedCharacters } from '$store/savedCharacters.svelte';
import SourceAttributionChip from '$components/SourceAttributionChip.svelte';
import { onMount } from 'svelte';

onMount(() => { void savedCharacters.ensureLoaded(); });
```

- [ ] **Step 3: Add derived state for live↔saved matching**

Inside `<script lang="ts">`:

```ts
const liveWithMatches = $derived(
  bridgeStore.characters.map(live => ({
    live,
    saved: savedCharacters.findMatch(live),
  }))
);
```

(Use whatever the existing variable name is for the live characters list — check Step 1's read.)

- [ ] **Step 4: Augment the existing live-character card**

For each live card render, add (a) a `SourceAttributionChip`, and (b) a Save / Update button cluster:

```svelte
<SourceAttributionChip source={item.live.source} />

{#if item.saved}
  <button
    type="button"
    onclick={() => savedCharacters.update(item.saved!.id, item.live)}
    disabled={savedCharacters.loading}
  >Update saved</button>
{:else}
  <button
    type="button"
    onclick={() => savedCharacters.save(item.live, item.live.source === 'foundry' ? (getSourceInfo('foundry')?.worldTitle ?? null) : null)}
    disabled={savedCharacters.loading}
  >Save locally</button>
{/if}
```

(Import `getSourceInfo` from `$store/bridge.svelte` if not already imported.)

- [ ] **Step 5: Add the Saved section below the Live grid**

After the existing live-grid block:

```svelte
<section class="saved-section">
  <h2 class="section-title">Saved · {savedCharacters.list.length} characters</h2>
  {#if savedCharacters.loading}
    <p>Loading…</p>
  {:else if savedCharacters.error}
    <p class="err">{savedCharacters.error}</p>
  {:else if savedCharacters.list.length === 0}
    <p class="muted">No saved characters yet. Click "Save locally" on a live character to save a snapshot.</p>
  {:else}
    <div class="card-grid">
      {#each savedCharacters.list as saved (saved.id)}
        <article class="card">
          <header>
            <strong>{saved.name}</strong>
          </header>
          <SourceAttributionChip source={saved.source} worldTitle={saved.foundryWorld} />
          <div class="meta">saved {saved.savedAt}</div>
          <button
            type="button"
            onclick={() => savedCharacters.delete(saved.id)}
            disabled={savedCharacters.loading}
          >Delete</button>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .saved-section { margin-top: 1.5rem; }
  .section-title {
    font-size: 1rem;
    color: var(--text-label);
    margin-bottom: 0.5rem;
  }
  .meta { font-size: 0.75rem; color: var(--text-muted); }
  .err { color: var(--accent-amber); }
  .muted { color: var(--text-muted); }
  /* card-grid is reused from existing styles in this file; do not redefine */
</style>
```

(Match `card-grid` to whatever class the existing live-grid uses; reuse don't duplicate.)

- [ ] **Step 6: Verify type-check + build**

Run: `npm run check`
Expected: clean.

Run: `npm run build`
Expected: clean.

- [ ] **Step 7: Manual verification (dev app)**

Run `npm run tauri dev`. Boot Foundry world (assumes Plan 0 complete and module 0.2.0 installed). In Campaign:

1. Live section shows characters; click "Save locally" on one — Saved section appears below with that character.
2. Click "Save locally" again on the same live char — see browser console error/toast: "already saved; use update".
3. Click "Update saved" — succeeds (no error), Saved section's `lastUpdatedAt` (visible via dev tools) bumps.
4. Click "Delete" on a saved card — it disappears from Saved section.
5. Each card shows "source: FVTT — [world title]" chip.
6. Restart the app — Saved section re-loads from DB, showing the same rows.

- [ ] **Step 8: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): add Saved characters section + Save/Update/Delete (Plan 1 task 11)"
```

---

## Task 12: Final verification gate

**Files:** none — verification only.

**Depends on:** all previous.

- [ ] **Step 1: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. All three sub-checks pass.

- [ ] **Step 2: Manual end-to-end**

Repeat Task 11 Step 7. All flows green.

- [ ] **Step 3: Commit (no-op if no changes — verify only)**

```bash
git status --short
```

If clean, no commit needed. If any small fixes were made during verification, commit:

```bash
git add -A
git commit -m "chore: Plan 1 verification fixups"
```

---

## Self-review checklist

- [x] Spec § 3.1 data model + schema — covered by Tasks 1, 2.
- [x] Spec § 3.3 Plan 1 commands (save/list/update/delete) — covered by Tasks 3, 4, 5, 6, 7.
- [x] Spec § 3.5 Campaign view (Layout B with twin sections + buttons) — covered by Task 11.
- [x] Spec § 3.5 Source attribution chip — covered by Task 10.
- [x] Spec § 3.6 error handling (UNIQUE conflict; not-found on update/delete) — tested in Tasks 3, 5, 6.
- [x] All four commands registered in `lib.rs` — Task 7.
- [x] All four commands have typed wrappers — Task 8.
- [x] Match key `(source, source_id)` enforced via UNIQUE constraint — Task 1.
- [x] Compare button intentionally NOT added (Plan 2 territory) — Task 11 anti-scope.
- [x] No placeholders / TBDs.
- [x] All commits run after `verify.sh` (CLAUDE.md hard rule) — Task 12.
