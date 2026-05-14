# GM Screen — Plan A: Modifier core implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. Do NOT spawn per-task spec-compliance or code-quality reviewer subagents. After ALL Plan A tasks are committed, run a SINGLE `code-review:code-review` against the full branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required. Default for wiring / IPC-router / typed-wrapper tasks is no new tests — `verify.sh` is the gate. TDD-required tasks have explicit "write failing test" steps.

**Goal:** Ship the `🛡 GM Screen` tool's stage-1 modifier core: per-character vertical list with horizontal modifier-card carousels, per-card on/off toggle, hide/show, cog-edit popover, freeform tag filter bar. Status palette dock ships separately as Plan B.

**Architecture:** New SQLite table `character_modifiers` keyed by `(source, source_id)` (no FK to `saved_characters` — modifiers anchor to the same composite key the live cache uses, so a modifier joins the live cache OR a saved row OR neither). Eight new Tauri commands in `src-tauri/src/db/modifier.rs`. Three frontend layers: typed API wrapper (`src/lib/modifiers/api.ts`), Svelte 5 runes store (`src/store/modifiers.svelte.ts`), and tool components under `src/tools/GmScreen.svelte` + `src/lib/components/gm-screen/*.svelte`. Modifier rows come in three flavors: advantage-derived (auto-rendered from `canonical.raw.items`, materialized on first GM engagement via idempotent upsert), free-floating (GM-added, materialized immediately), and status-template instances (handled by Plan B).

**Tech Stack:** Rust + sqlx (existing), Tauri 2 IPC, Svelte 5 runes mode, TypeScript. SQLite migration `0005_modifiers.sql` ships both `character_modifiers` and `status_templates` tables (templates table is inert until Plan B wires its commands — single migration is cheaper than two per spec §10).

**Spec:** `docs/superpowers/specs/2026-05-03-gm-screen-design.md`
**Architecture reference:** `ARCHITECTURE.md` §2 (domain model — extending), §3 (storage), §4 (IPC + typed wrappers), §5 (boundaries), §6 (CSS / token invariants), §7 (error-handling prefixes), §9 (extensibility seams — adding a tool, schema, command), §10 (testing).

**Spec defaults adopted (§13 open questions, no override needed under auto-mode):**
- Stage 1 ships free-floating + palette modifiers for Roll20 characters; **no auto-spawned advantage cards from Roll20.** Roll20 advantage auto-spawn deferred to Phase 2.5. The advantage-derived render path in §8.1 reads only Foundry `canonical.raw.items`.
- Effect editor `delta` widget = `[− 0 +]` numeric stepper bounded -10..+10, freeform `scope` text input.
- Status template apply uses click-to-apply with focused-character convention (Plan B detail).
- Empty palette by default — no canned-templates seed (Plan B detail).

**Path correction (spec §6 inventory):** spec lists components under `src/components/gm-screen/`; the project actually uses **`src/lib/components/gm-screen/`** (see `src/lib/components/AdvantageForm.svelte`, `src/lib/components/CompareModal.svelte` etc.). All component paths in this plan use the correct `src/lib/components/...` location.

---

## File structure

### Files created

| Path | Responsibility |
|---|---|
| `src-tauri/migrations/0005_modifiers.sql` | DDL for `character_modifiers` + `status_templates` (Plan B inert until B wires it). |
| `src-tauri/src/shared/modifier.rs` | Modifier type defs: `CharacterModifier`, `ModifierBinding`, `ModifierEffect`, `ModifierKind`, `NewCharacterModifier`, `ModifierPatch`. Pure types + serde derives. |
| `src-tauri/src/db/modifier.rs` | 8 Tauri commands + private `db_*` helpers + inline `#[cfg(test)]` tests. |
| `src/lib/modifiers/api.ts` | Typed Tauri-`invoke` wrappers (one function per command). Components never call `invoke(...)` directly (ARCH §4). |
| `src/store/modifiers.svelte.ts` | Svelte 5 runes store: cached modifier list, `ensureLoaded` / `refresh` / CRUD methods, UI prefs (`activeFilterTags`, `showHidden`, `showOrphans`). |
| `src/tools/GmScreen.svelte` | Top-level tool: layout, mounts the store, renders filter bar + character rows + orphans section. |
| `src/lib/components/gm-screen/CharacterRow.svelte` | One vertical row per character: header line + horizontal card carousel + `+ Add modifier`. Walks `canonical.raw.items` for derived virtual cards. |
| `src/lib/components/gm-screen/ModifierCard.svelte` | Single modifier card: stacked-overlapping carousel CSS, state axes (active / hidden / hover), toggle pill, cog. **Frontend-design candidate.** |
| `src/lib/components/gm-screen/ModifierEffectEditor.svelte` | Cog-anchored inline popover: list of `ModifierEffect` rows + add/remove + tag chip editor. **Frontend-design candidate.** |
| `src/lib/components/gm-screen/TagFilterBar.svelte` | Chip filter strip; OR semantics; empty active set = no filter. Mirrors `AdvantagesManager.svelte` chip pattern. |

### Files modified

| Path | Change |
|---|---|
| `src-tauri/src/shared/mod.rs` | Add `pub mod modifier;`. |
| `src-tauri/src/db/mod.rs` | Add `pub mod modifier;`. |
| `src-tauri/src/lib.rs` | Register 8 new commands in `invoke_handler(tauri::generate_handler![...])`. |
| `src/types.ts` | Mirror `CharacterModifier`, `ModifierBinding`, `ModifierEffect`, `ModifierKind`, `NewCharacterModifier`, `ModifierPatch`. |
| `src/tools.ts` | One new entry: `{ id: 'gm-screen', label: 'GM Screen', icon: '🛡', component: () => import('./tools/GmScreen.svelte') }`. |

### Files NOT touched in Plan A (reserved for Plan B)

- `src-tauri/src/db/status_template.rs` (created in B)
- `src/lib/components/gm-screen/StatusPaletteDock.svelte` (B)
- `src/lib/components/gm-screen/StatusTemplateEditor.svelte` (B)
- `src/store/statusTemplates.svelte.ts` (B)
- `src/tools/Campaign.svelte` (no edits — GM Screen does not modify Campaign)

---

## Task A1: Schema migration + Rust modifier types + list commands

**Goal:** Land the migration (both tables), define all Rust types, and implement the two read commands with JSON round-trip tests. After this task the backend can persist + retrieve modifier rows; nothing else uses them yet.

**Files:**
- Create: `src-tauri/migrations/0005_modifiers.sql`
- Create: `src-tauri/src/shared/modifier.rs`
- Modify: `src-tauri/src/shared/mod.rs` (add `pub mod modifier;`)
- Create: `src-tauri/src/db/modifier.rs`
- Modify: `src-tauri/src/db/mod.rs` (add `pub mod modifier;`)

**Anti-scope (do NOT touch):** `src-tauri/src/lib.rs` (commands not registered until A4), `src/**/*` (frontend not wired until A4), `db/status_template.rs` (Plan B).

**Depends on:** none — first task.

**Invariants cited:** ARCH §3 (storage strategy — SQLite + migrations), §5 (only `db/*` talks to SQLite), §6 (`PRAGMA foreign_keys = ON` already enforced via `SqliteConnectOptions`).

**Tests required:** YES — JSON round-trip is real logic, TDD. (Per project TDD-on-demand override.)

- [ ] **Step 1: Write the migration**

Create `src-tauri/migrations/0005_modifiers.sql`:

```sql
-- Per-character modifier records. Anchored to (source, source_id) — the same
-- composite key the live bridge cache uses — with no FK to saved_characters
-- so modifiers can attach to live-only OR saved-only OR both characters.
CREATE TABLE IF NOT EXISTS character_modifiers (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    source               TEXT    NOT NULL CHECK(source IN ('roll20','foundry')),
    source_id            TEXT    NOT NULL,
    name                 TEXT    NOT NULL,
    description          TEXT    NOT NULL DEFAULT '',
    effects_json         TEXT    NOT NULL DEFAULT '[]',
    binding_json         TEXT    NOT NULL DEFAULT '{"kind":"free"}',
    tags_json            TEXT    NOT NULL DEFAULT '[]',
    is_active            INTEGER NOT NULL DEFAULT 0,
    is_hidden            INTEGER NOT NULL DEFAULT 0,
    origin_template_id   INTEGER,
    created_at           TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_modifiers_char
    ON character_modifiers(source, source_id);

-- Status templates: GM-authored reusable effect bundles (Slippery, Blind, etc.).
-- Inert in Plan A — Plan B wires the CRUD commands and palette UI.
CREATE TABLE IF NOT EXISTS status_templates (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    effects_json    TEXT    NOT NULL DEFAULT '[]',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);
```

- [ ] **Step 2: Write the modifier types module**

Create `src-tauri/src/shared/modifier.rs`:

```rust
use serde::{Deserialize, Serialize};
use crate::bridge::types::SourceKind;

/// One row in the `character_modifiers` table, hydrated from JSON columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterModifier {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub is_active: bool,
    pub is_hidden: bool,
    pub origin_template_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Tagged enum describing what the modifier is bound to. New variants
/// (Room, FoundryEffect) can be added without a migration — just deserialize
/// a new shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ModifierBinding {
    Free,
    Advantage { item_id: String },
    // Future variants intentionally unstubbed (per ARCH §11 plan packaging
    // — strict additivity preserved without dead-code stubs):
    //   Room { room_id: i64 }              — rooms/bundles future
    //   FoundryEffect { effect_id: String } — Phase 4 mirror
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModifierEffect {
    pub kind: ModifierKind,
    pub scope: Option<String>,
    pub delta: Option<i32>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind { Pool, Difficulty, Note }

/// Argument shape for `add_character_modifier`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCharacterModifier {
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub origin_template_id: Option<i64>,
}

/// Patch shape for `update_character_modifier`. Active/hidden have dedicated
/// setters; binding cannot be changed once set.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifierPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub effects: Option<Vec<ModifierEffect>>,
    pub tags: Option<Vec<String>>,
}
```

- [ ] **Step 3: Register the new shared module**

Modify `src-tauri/src/shared/mod.rs` — add `pub mod modifier;` at the end:

```rust
pub mod types;
pub mod dice;
pub mod resonance;
pub mod v5;
pub mod canonical_fields;
pub mod modifier;
```

- [ ] **Step 4: Write the failing JSON round-trip test**

Create `src-tauri/src/db/modifier.rs`:

```rust
use sqlx::{Row, SqlitePool};
use crate::bridge::types::SourceKind;
use crate::shared::modifier::{
    CharacterModifier, ModifierBinding, ModifierEffect, ModifierKind,
};

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

fn row_to_modifier(r: &sqlx::sqlite::SqliteRow) -> Result<CharacterModifier, String> {
    let source_str: String = r.get("source");
    let source = str_to_source(&source_str)
        .ok_or_else(|| format!("db/modifier.list: unknown source '{source_str}'"))?;
    let effects_json: String = r.get("effects_json");
    let effects: Vec<ModifierEffect> = serde_json::from_str(&effects_json)
        .map_err(|e| format!("db/modifier.list: effects deserialize: {e}"))?;
    let binding_json: String = r.get("binding_json");
    let binding: ModifierBinding = serde_json::from_str(&binding_json)
        .map_err(|e| format!("db/modifier.list: binding deserialize: {e}"))?;
    let tags_json: String = r.get("tags_json");
    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| format!("db/modifier.list: tags deserialize: {e}"))?;
    Ok(CharacterModifier {
        id: r.get("id"),
        source,
        source_id: r.get("source_id"),
        name: r.get("name"),
        description: r.get("description"),
        effects,
        binding,
        tags,
        is_active: r.get::<bool, _>("is_active"),
        is_hidden: r.get::<bool, _>("is_hidden"),
        origin_template_id: r.get("origin_template_id"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub(crate) async fn db_list(
    pool: &SqlitePool,
    source: &SourceKind,
    source_id: &str,
) -> Result<Vec<CharacterModifier>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, created_at, updated_at
         FROM character_modifiers
         WHERE source = ? AND source_id = ?
         ORDER BY id ASC"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/modifier.list: {e}"))?;
    rows.iter().map(row_to_modifier).collect()
}

pub(crate) async fn db_list_all(pool: &SqlitePool) -> Result<Vec<CharacterModifier>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, created_at, updated_at
         FROM character_modifiers
         ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/modifier.list_all: {e}"))?;
    rows.iter().map(row_to_modifier).collect()
}

#[tauri::command]
pub async fn list_character_modifiers(
    pool: tauri::State<'_, crate::DbState>,
    source: SourceKind,
    source_id: String,
) -> Result<Vec<CharacterModifier>, String> {
    db_list(&pool.0, &source, &source_id).await
}

#[tauri::command]
pub async fn list_all_character_modifiers(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<CharacterModifier>, String> {
    db_list_all(&pool.0).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn fresh_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = fresh_pool().await;
        let result = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn round_trip_preserves_effects_binding_tags() {
        let pool = fresh_pool().await;
        // Insert directly (db_add doesn't exist yet — round-trip-only test).
        sqlx::query(
            "INSERT INTO character_modifiers
             (source, source_id, name, description, effects_json, binding_json, tags_json, is_active, is_hidden)
             VALUES ('foundry', 'abc', 'Beautiful', 'desc',
                     '[{\"kind\":\"pool\",\"scope\":\"Social\",\"delta\":1,\"note\":null}]',
                     '{\"kind\":\"advantage\",\"item_id\":\"item-xyz\"}',
                     '[\"Social\",\"Looks\"]',
                     1, 0)"
        )
        .execute(&pool).await.unwrap();

        let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert_eq!(list.len(), 1);
        let m = &list[0];
        assert_eq!(m.name, "Beautiful");
        assert_eq!(m.effects.len(), 1);
        assert_eq!(m.effects[0].kind, ModifierKind::Pool);
        assert_eq!(m.effects[0].scope.as_deref(), Some("Social"));
        assert_eq!(m.effects[0].delta, Some(1));
        match &m.binding {
            ModifierBinding::Advantage { item_id } => assert_eq!(item_id, "item-xyz"),
            other => panic!("expected Advantage binding, got {other:?}"),
        }
        assert_eq!(m.tags, vec!["Social".to_string(), "Looks".to_string()]);
        assert!(m.is_active);
        assert!(!m.is_hidden);
    }

    #[tokio::test]
    async fn list_all_returns_rows_across_characters() {
        let pool = fresh_pool().await;
        for sid in &["a", "b", "c"] {
            sqlx::query(
                "INSERT INTO character_modifiers (source, source_id, name) VALUES ('foundry', ?, 'X')"
            )
            .bind(sid)
            .execute(&pool).await.unwrap();
        }
        let list = db_list_all(&pool).await.unwrap();
        assert_eq!(list.len(), 3);
    }

    #[tokio::test]
    async fn list_filters_by_source_and_source_id() {
        let pool = fresh_pool().await;
        sqlx::query("INSERT INTO character_modifiers (source, source_id, name) VALUES ('foundry', 'abc', 'X')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO character_modifiers (source, source_id, name) VALUES ('foundry', 'def', 'Y')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO character_modifiers (source, source_id, name) VALUES ('roll20', 'abc', 'Z')")
            .execute(&pool).await.unwrap();
        let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "X");
    }
}
```

- [ ] **Step 5: Register the new db module**

Modify `src-tauri/src/db/mod.rs` — add `pub mod modifier;` (location: alongside the other `pub mod ...;` declarations).

- [ ] **Step 6: Run tests to verify they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier
```

Expected: all 4 tests pass (`list_empty_returns_empty_vec`, `round_trip_preserves_effects_binding_tags`, `list_all_returns_rows_across_characters`, `list_filters_by_source_and_source_id`).

- [ ] **Step 7: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green. Two new commands (`list_character_modifiers`, `list_all_character_modifiers`) will trigger Rust dead-code warnings since they aren't yet in the `invoke_handler` list — those are expected and resolved in Task A4.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/migrations/0005_modifiers.sql \
        src-tauri/src/shared/modifier.rs \
        src-tauri/src/shared/mod.rs \
        src-tauri/src/db/modifier.rs \
        src-tauri/src/db/mod.rs
git commit -m "feat(db): add character_modifiers table + list commands

Migration ships character_modifiers + status_templates tables (templates
inert until Plan B). New shared/modifier.rs defines CharacterModifier
and tagged ModifierBinding enum. db/modifier.rs implements list /
list_all commands with JSON round-trip for effects/binding/tags."
```

---

## Task A2: Add / update / delete commands

**Goal:** Implement the three core write commands with validation and error contracts per spec §9.

**Files:**
- Modify: `src-tauri/src/db/modifier.rs` (extend existing module)

**Anti-scope:** Do not touch `lib.rs`, `src/**/*`, status_template files.

**Depends on:** A1.

**Invariants cited:** ARCH §7 (error prefixes `db/modifier.<op>:`), spec §9 error table.

**Tests required:** YES — partial-patch logic and validation are real logic.

- [ ] **Step 1: Write the failing test for `db_add` happy path**

Add to `#[cfg(test)] mod tests` in `src-tauri/src/db/modifier.rs`:

```rust
fn sample_new(source_id: &str) -> crate::shared::modifier::NewCharacterModifier {
    use crate::shared::modifier::*;
    NewCharacterModifier {
        source: SourceKind::Foundry,
        source_id: source_id.to_string(),
        name: "Beautiful".to_string(),
        description: "Looks bonus".to_string(),
        effects: vec![ModifierEffect {
            kind: ModifierKind::Pool,
            scope: Some("Social".to_string()),
            delta: Some(1),
            note: None,
        }],
        binding: ModifierBinding::Free,
        tags: vec!["Social".to_string()],
        origin_template_id: None,
    }
}

#[tokio::test]
async fn add_inserts_and_returns_full_record() {
    let pool = fresh_pool().await;
    let m = db_add(&pool, sample_new("abc")).await.unwrap();
    assert!(m.id > 0);
    assert_eq!(m.name, "Beautiful");
    assert_eq!(m.effects.len(), 1);
    assert!(matches!(m.binding, ModifierBinding::Free));
    assert!(!m.is_active);
    assert!(!m.is_hidden);
    let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn add_rejects_empty_name() {
    let pool = fresh_pool().await;
    let mut new = sample_new("abc");
    new.name = String::new();
    let err = db_add(&pool, new).await.unwrap_err();
    assert!(err.contains("empty name"), "got: {err}");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::add
```

Expected: FAIL with `cannot find function 'db_add'`.

- [ ] **Step 3: Implement `db_add` + `add_character_modifier`**

Append to `src-tauri/src/db/modifier.rs` (above `#[cfg(test)] mod tests`):

```rust
use crate::shared::modifier::{NewCharacterModifier, ModifierPatch};

pub(crate) async fn db_add(
    pool: &SqlitePool,
    input: NewCharacterModifier,
) -> Result<CharacterModifier, String> {
    if input.name.trim().is_empty() {
        return Err("db/modifier.add: empty name".to_string());
    }
    let effects_json = serde_json::to_string(&input.effects)
        .map_err(|e| format!("db/modifier.add: serialize effects: {e}"))?;
    let binding_json = serde_json::to_string(&input.binding)
        .map_err(|e| format!("db/modifier.add: serialize binding: {e}"))?;
    let tags_json = serde_json::to_string(&input.tags)
        .map_err(|e| format!("db/modifier.add: serialize tags: {e}"))?;

    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          origin_template_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&input.source))
    .bind(&input.source_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&binding_json)
    .bind(&tags_json)
    .bind(input.origin_template_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.add: {e}"))?;
    let id = result.last_insert_rowid();
    db_get(pool, id).await
}

pub(crate) async fn db_get(pool: &SqlitePool, id: i64) -> Result<CharacterModifier, String> {
    let row = sqlx::query(
        "SELECT id, source, source_id, name, description, effects_json,
                binding_json, tags_json, is_active, is_hidden,
                origin_template_id, created_at, updated_at
         FROM character_modifiers WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/modifier.get: {e}"))?
    .ok_or_else(|| "db/modifier.get: not found".to_string())?;
    row_to_modifier(&row)
}

#[tauri::command]
pub async fn add_character_modifier(
    pool: tauri::State<'_, crate::DbState>,
    input: NewCharacterModifier,
) -> Result<CharacterModifier, String> {
    db_add(&pool.0, input).await
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::add
```

Expected: PASS.

- [ ] **Step 5: Write the failing test for `db_update` partial patch**

Add to `tests` mod:

```rust
#[tokio::test]
async fn update_applies_partial_patch_and_preserves_untouched_fields() {
    let pool = fresh_pool().await;
    let m = db_add(&pool, sample_new("abc")).await.unwrap();
    let original_desc = m.description.clone();

    let patch = ModifierPatch {
        name: Some("Renamed".to_string()),
        description: None,
        effects: None,
        tags: None,
    };
    let updated = db_update(&pool, m.id, patch).await.unwrap();
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.description, original_desc);
    assert_eq!(updated.effects.len(), 1); // untouched
    assert_eq!(updated.tags, vec!["Social".to_string()]); // untouched
}

#[tokio::test]
async fn update_missing_id_returns_not_found() {
    let pool = fresh_pool().await;
    let patch = ModifierPatch { name: Some("X".into()), description: None, effects: None, tags: None };
    let err = db_update(&pool, 9999, patch).await.unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}
```

- [ ] **Step 6: Run to verify it fails**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::update
```

Expected: FAIL with `cannot find function 'db_update'`.

- [ ] **Step 7: Implement `db_update` + `update_character_modifier`**

Append to `src-tauri/src/db/modifier.rs` (still above the `#[cfg(test)]` block):

```rust
pub(crate) async fn db_update(
    pool: &SqlitePool,
    id: i64,
    patch: ModifierPatch,
) -> Result<CharacterModifier, String> {
    // Load existing, apply patch in memory, write back. Simpler than dynamic SQL
    // and avoids COALESCE-with-JSON gymnastics.
    let mut current = db_get(pool, id).await
        .map_err(|e| if e.contains("not found") { "db/modifier.update: not found".to_string() } else { format!("db/modifier.update: {e}") })?;

    if let Some(name) = patch.name {
        if name.trim().is_empty() {
            return Err("db/modifier.update: empty name".to_string());
        }
        current.name = name;
    }
    if let Some(desc) = patch.description { current.description = desc; }
    if let Some(effects) = patch.effects   { current.effects = effects; }
    if let Some(tags) = patch.tags         { current.tags = tags; }

    let effects_json = serde_json::to_string(&current.effects)
        .map_err(|e| format!("db/modifier.update: serialize effects: {e}"))?;
    let tags_json = serde_json::to_string(&current.tags)
        .map_err(|e| format!("db/modifier.update: serialize tags: {e}"))?;

    let result = sqlx::query(
        "UPDATE character_modifiers
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
    .map_err(|e| format!("db/modifier.update: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/modifier.update: not found".to_string());
    }
    db_get(pool, id).await
}

#[tauri::command]
pub async fn update_character_modifier(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    patch: ModifierPatch,
) -> Result<CharacterModifier, String> {
    db_update(&pool.0, id, patch).await
}
```

- [ ] **Step 8: Run update tests to verify they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::update
```

Expected: PASS.

- [ ] **Step 9: Implement `db_delete` + `delete_character_modifier`** (no separate failing test — pattern is identical to `delete_saved_character` and similarly mechanical)

Append:

```rust
pub(crate) async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM character_modifiers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/modifier.delete: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/modifier.delete: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_character_modifier(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}
```

Add to tests mod:

```rust
#[tokio::test]
async fn delete_removes_row() {
    let pool = fresh_pool().await;
    let m = db_add(&pool, sample_new("abc")).await.unwrap();
    db_delete(&pool, m.id).await.unwrap();
    let list = db_list(&pool, &SourceKind::Foundry, "abc").await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn delete_missing_id_returns_not_found() {
    let pool = fresh_pool().await;
    let err = db_delete(&pool, 9999).await.unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}
```

- [ ] **Step 10: Run all modifier tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier
```

Expected: all tests pass (4 from A1 + 6 from A2 = 10 tests total).

- [ ] **Step 11: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 12: Commit**

```bash
git add src-tauri/src/db/modifier.rs
git commit -m "feat(db/modifier): add / update / delete commands

ModifierPatch supports partial updates of name/description/effects/tags;
binding is immutable post-create. Empty-name validation per spec §9.
Error prefixes follow db/modifier.<op>: convention (ARCH §7)."
```

---

## Task A3: Active / hidden setters + `materialize_advantage_modifier`

**Goal:** Land the two boolean setters and the idempotent advantage materialization upsert. Materialization is the only real logic here — TDD it carefully because the spec's data flow (§8.2) hinges on idempotency.

**Files:**
- Modify: `src-tauri/src/db/modifier.rs`

**Anti-scope:** Do not touch `lib.rs`, `src/**/*`, status_template files.

**Depends on:** A2.

**Invariants cited:** spec §5 (no unique constraint on `(source, source_id, binding.item_id)` — multiple modifiers per advantage allowed), §8.2 (materialize idempotency), ARCH §7 (error prefixes).

**Tests required:** YES — materialization idempotency is real logic.

- [ ] **Step 1: Write failing tests for active/hidden setters**

Add to `tests` mod in `src-tauri/src/db/modifier.rs`:

```rust
#[tokio::test]
async fn set_active_flips_flag() {
    let pool = fresh_pool().await;
    let m = db_add(&pool, sample_new("abc")).await.unwrap();
    assert!(!m.is_active);
    db_set_active(&pool, m.id, true).await.unwrap();
    let after = db_get(&pool, m.id).await.unwrap();
    assert!(after.is_active);
    db_set_active(&pool, m.id, false).await.unwrap();
    let after = db_get(&pool, m.id).await.unwrap();
    assert!(!after.is_active);
}

#[tokio::test]
async fn set_active_missing_id_errors() {
    let pool = fresh_pool().await;
    let err = db_set_active(&pool, 9999, true).await.unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}

#[tokio::test]
async fn set_hidden_flips_flag() {
    let pool = fresh_pool().await;
    let m = db_add(&pool, sample_new("abc")).await.unwrap();
    assert!(!m.is_hidden);
    db_set_hidden(&pool, m.id, true).await.unwrap();
    let after = db_get(&pool, m.id).await.unwrap();
    assert!(after.is_hidden);
}

#[tokio::test]
async fn set_hidden_missing_id_errors() {
    let pool = fresh_pool().await;
    let err = db_set_hidden(&pool, 9999, true).await.unwrap_err();
    assert!(err.contains("not found"), "got: {err}");
}
```

- [ ] **Step 2: Run to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::set_
```

Expected: FAIL with `cannot find function 'db_set_active'` / `db_set_hidden`.

- [ ] **Step 3: Implement the setters**

Append to `src-tauri/src/db/modifier.rs`:

```rust
pub(crate) async fn db_set_active(pool: &SqlitePool, id: i64, value: bool) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE character_modifiers SET is_active = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(value as i64)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.set_active: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/modifier.set_active: not found".to_string());
    }
    Ok(())
}

pub(crate) async fn db_set_hidden(pool: &SqlitePool, id: i64, value: bool) -> Result<(), String> {
    let result = sqlx::query(
        "UPDATE character_modifiers SET is_hidden = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(value as i64)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.set_hidden: {e}"))?;
    if result.rows_affected() == 0 {
        return Err("db/modifier.set_hidden: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn set_modifier_active(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    is_active: bool,
) -> Result<(), String> {
    db_set_active(&pool.0, id, is_active).await
}

#[tauri::command]
pub async fn set_modifier_hidden(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    is_hidden: bool,
) -> Result<(), String> {
    db_set_hidden(&pool.0, id, is_hidden).await
}
```

- [ ] **Step 4: Run setters tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::set_
```

Expected: PASS (4 tests).

- [ ] **Step 5: Write the failing tests for `db_materialize_advantage`**

Add to `tests` mod:

```rust
#[tokio::test]
async fn materialize_inserts_when_absent() {
    let pool = fresh_pool().await;
    let m = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-merit-1",
        "Beautiful", "Looks bonus",
    ).await.unwrap();
    assert!(m.id > 0);
    assert_eq!(m.name, "Beautiful");
    assert_eq!(m.description, "Looks bonus");
    assert!(m.effects.is_empty());
    assert!(!m.is_active);
    assert!(!m.is_hidden);
    match &m.binding {
        ModifierBinding::Advantage { item_id } => assert_eq!(item_id, "item-merit-1"),
        other => panic!("expected Advantage binding, got {other:?}"),
    }
}

#[tokio::test]
async fn materialize_returns_existing_unchanged_when_present() {
    let pool = fresh_pool().await;
    let first = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-merit-1",
        "Beautiful", "Looks bonus",
    ).await.unwrap();

    // Mutate the row to verify the second call does NOT overwrite it.
    db_set_active(&pool, first.id, true).await.unwrap();

    // Calling materialize again with different name/description must NOT change the row.
    let second = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-merit-1",
        "Different name", "Different description",
    ).await.unwrap();

    assert_eq!(second.id, first.id);
    assert_eq!(second.name, "Beautiful");           // original preserved
    assert_eq!(second.description, "Looks bonus");  // original preserved
    assert!(second.is_active);                      // mutation preserved
}

#[tokio::test]
async fn materialize_distinguishes_by_character_and_by_item_id() {
    let pool = fresh_pool().await;
    let a = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-x", "X", "x",
    ).await.unwrap();
    // Same item_id, different character → distinct row.
    let b = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-2", "item-x", "X", "x",
    ).await.unwrap();
    // Same character, different item_id → distinct row.
    let c = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-y", "Y", "y",
    ).await.unwrap();
    assert_ne!(a.id, b.id);
    assert_ne!(a.id, c.id);
    assert_ne!(b.id, c.id);
}

#[tokio::test]
async fn materialize_allows_existing_free_modifiers_for_same_character() {
    // No unique constraint per spec §5 rationale — a free-floating modifier
    // and an advantage-bound modifier coexist freely.
    let pool = fresh_pool().await;
    let _free = db_add(&pool, sample_new("char-1")).await.unwrap();
    let bound = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-1", "Bound", "b",
    ).await.unwrap();
    let list = db_list(&pool, &SourceKind::Foundry, "char-1").await.unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|m| m.id == bound.id));
}

#[tokio::test]
async fn materialize_rejects_empty_name() {
    let pool = fresh_pool().await;
    let err = db_materialize_advantage(
        &pool, &SourceKind::Foundry, "char-1", "item-1", "", "desc",
    ).await.unwrap_err();
    assert!(err.contains("empty name"), "got: {err}");
}
```

- [ ] **Step 6: Run to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::materialize
```

Expected: FAIL with `cannot find function 'db_materialize_advantage'`.

- [ ] **Step 7: Implement `db_materialize_advantage` + `materialize_advantage_modifier`**

The query uses `json_extract` to find an existing row whose `binding_json` is an `Advantage` variant with the given `item_id`.

Append to `src-tauri/src/db/modifier.rs`:

```rust
/// Idempotent upsert. If a row exists for (source, source_id, binding=Advantage{item_id}),
/// returns it unchanged. Otherwise inserts with empty effects, is_active=false,
/// is_hidden=false. Spec §8.2.
pub(crate) async fn db_materialize_advantage(
    pool: &SqlitePool,
    source: &SourceKind,
    source_id: &str,
    item_id: &str,
    name: &str,
    description: &str,
) -> Result<CharacterModifier, String> {
    if name.trim().is_empty() {
        return Err("db/modifier.materialize: empty name".to_string());
    }

    let existing = sqlx::query(
        "SELECT id FROM character_modifiers
         WHERE source = ? AND source_id = ?
           AND json_extract(binding_json, '$.kind') = 'advantage'
           AND json_extract(binding_json, '$.item_id') = ?
         LIMIT 1"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/modifier.materialize: {e}"))?;

    if let Some(row) = existing {
        let id: i64 = row.get("id");
        return db_get(pool, id).await;
    }

    // Build the binding JSON inline (the enum's tag/snake_case variant rename is
    // the source of truth for the literal we write here).
    let binding_json = format!("{{\"kind\":\"advantage\",\"item_id\":{}}}",
        serde_json::to_string(item_id).map_err(|e| format!("db/modifier.materialize: encode item_id: {e}"))?);

    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json)
         VALUES (?, ?, ?, ?, '[]', ?, '[]')"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .bind(name)
    .bind(description)
    .bind(&binding_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.materialize: {e}"))?;
    db_get(pool, result.last_insert_rowid()).await
}

#[tauri::command]
pub async fn materialize_advantage_modifier(
    pool: tauri::State<'_, crate::DbState>,
    source: SourceKind,
    source_id: String,
    item_id: String,
    name: String,
    description: String,
) -> Result<CharacterModifier, String> {
    db_materialize_advantage(&pool.0, &source, &source_id, &item_id, &name, &description).await
}
```

- [ ] **Step 8: Run materialize tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::materialize
```

Expected: PASS (5 tests).

- [ ] **Step 9: Run all modifier tests + verify.sh**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- modifier && ./scripts/verify.sh
```

Expected: green. Full modifier test suite is now ~19 tests (4 from A1 + 6 from A2 + 4 setters + 5 materialize).

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/db/modifier.rs
git commit -m "feat(db/modifier): set_active / set_hidden + materialize_advantage upsert

materialize_advantage_modifier is idempotent: if a row exists for
(source, source_id, binding=Advantage{item_id}) it is returned unchanged;
otherwise inserted with empty effects and both flags false. Multiple
modifiers per (character, item_id) intentionally allowed (spec §5
rationale — compound merits)."
```

---

## Task A4: Register commands + TS mirror + typed wrapper

**Goal:** Wire all 8 commands into the Tauri router, mirror the Rust types in `src/types.ts`, and create the typed frontend wrapper. After this task the frontend can call any modifier command with full type safety.

**Files:**
- Modify: `src-tauri/src/lib.rs` (extend `invoke_handler` list)
- Modify: `src/types.ts` (mirror new types)
- Create: `src/lib/modifiers/api.ts`

**Anti-scope:** Do not touch any component, store, or status_template file.

**Depends on:** A3.

**Invariants cited:** ARCH §4 (typed wrappers, components never call `invoke()`), ARCH §9 (add-a-command seam: declare → register in lib.rs → typed wrapper).

**Tests required:** NO — pure wiring, `verify.sh` is the gate (per project TDD-on-demand override).

- [ ] **Step 1: Register commands in `src-tauri/src/lib.rs`**

In the `invoke_handler(tauri::generate_handler![...])` list, add 8 lines (place them alphabetically near the other `db::` entries — after the `db::edge::*` block is fine):

```rust
            db::modifier::list_character_modifiers,
            db::modifier::list_all_character_modifiers,
            db::modifier::add_character_modifier,
            db::modifier::update_character_modifier,
            db::modifier::delete_character_modifier,
            db::modifier::set_modifier_active,
            db::modifier::set_modifier_hidden,
            db::modifier::materialize_advantage_modifier,
```

- [ ] **Step 2: Mirror types in `src/types.ts`**

Append to `src/types.ts` (after the `Roll20Raw` block; before the Domains section is the cleanest neighbor since both blocks describe bridge-adjacent shapes):

```ts
// ---------------------------------------------------------------------------
// GM Screen — character modifiers (mirrors src-tauri/src/shared/modifier.rs).
// CharacterModifier serializes camelCase via Rust serde rename; the binding
// discriminator is `kind` and uses snake_case variants ('free' / 'advantage').
// ---------------------------------------------------------------------------

export type ModifierKind = 'pool' | 'difficulty' | 'note';

export interface ModifierEffect {
  kind: ModifierKind;
  scope: string | null;
  delta: number | null;
  note: string | null;
}

export type ModifierBinding =
  | { kind: 'free' }
  | { kind: 'advantage'; item_id: string };

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
  createdAt: string;
  updatedAt: string;
}

export interface NewCharacterModifierInput {
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  originTemplateId: number | null;
}

export interface ModifierPatchInput {
  name?: string;
  description?: string;
  effects?: ModifierEffect[];
  tags?: string[];
}
```

> **Type-naming note** (memory: BridgeCharacter snake_case vs SavedCharacter camelCase): The new `CharacterModifier` follows the SavedCharacter pattern (camelCase TS — `sourceId`, `isActive`) because the Rust struct carries `#[serde(rename_all = "camelCase")]`. The `ModifierBinding.advantage` variant intentionally keeps snake_case `item_id` because the Rust enum uses `#[serde(rename_all = "snake_case", tag = "kind")]` at the enum level; `item_id` is a struct field on the variant and is NOT renamed by enum-level rename rules — it serializes verbatim from the Rust field name.

- [ ] **Step 3: Create the typed wrapper**

Create `src/lib/modifiers/api.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  SourceKind,
} from '../../types';

export function listCharacterModifiers(
  source: SourceKind,
  sourceId: string,
): Promise<CharacterModifier[]> {
  return invoke<CharacterModifier[]>('list_character_modifiers', { source, sourceId });
}

export function listAllCharacterModifiers(): Promise<CharacterModifier[]> {
  return invoke<CharacterModifier[]>('list_all_character_modifiers');
}

export function addCharacterModifier(input: NewCharacterModifierInput): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('add_character_modifier', { input });
}

export function updateCharacterModifier(
  id: number,
  patch: ModifierPatchInput,
): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('update_character_modifier', { id, patch });
}

export function deleteCharacterModifier(id: number): Promise<void> {
  return invoke<void>('delete_character_modifier', { id });
}

export function setModifierActive(id: number, isActive: boolean): Promise<void> {
  return invoke<void>('set_modifier_active', { id, isActive });
}

export function setModifierHidden(id: number, isHidden: boolean): Promise<void> {
  return invoke<void>('set_modifier_hidden', { id, isHidden });
}

export function materializeAdvantageModifier(args: {
  source: SourceKind;
  sourceId: string;
  itemId: string;
  name: string;
  description: string;
}): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('materialize_advantage_modifier', args);
}
```

> **IPC argument-naming note:** Tauri serializes JS object keys using camelCase → snake_case conversion when the Rust command parameter is a primitive (e.g. `is_active: bool` ← JS `isActive`), but **passes objects through as-is** when the parameter is a struct (`input: NewCharacterModifier`, `patch: ModifierPatch`). The struct then deserializes via its own serde rules (`rename_all = "camelCase"`). Reference: `src/lib/saved-characters/api.ts` (existing precedent — `saveCharacter` passes `canonical` as a struct, `patchSavedField` passes `id` + `name` + `value` as primitives).

- [ ] **Step 4: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green. `npm run check` should now pass without dead-import warnings — every wrapper function is exported and matches its Rust counterpart.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src/types.ts src/lib/modifiers/api.ts
git commit -m "feat(modifiers): register 8 IPC commands + TS mirror + typed wrappers

src/lib/modifiers/api.ts is the only path components may use to talk to
the modifier backend (ARCH §4). CharacterModifier mirrors the Rust struct
camelCase rename; ModifierBinding.advantage keeps snake_case item_id
(enum-level rename does not propagate into variant struct fields)."
```

---

## Task A5: Modifier Svelte runes store

**Goal:** Build the cache + UI-prefs store consumed by GM Screen components. Uses the `savedCharacters.svelte.ts` shape as its template (initialized flag, `_loading` / `_error`, CRUD methods that refresh state).

**Files:**
- Create: `src/store/modifiers.svelte.ts`

**Anti-scope:** Do not touch any component or backend file.

**Depends on:** A4.

**Invariants cited:** spec §8.6 (no per-event refetch — only on mount + on successful CRUD response).

**Tests required:** NO — wiring; `npm run check` is the gate.

- [ ] **Step 1: Create the store**

Create `src/store/modifiers.svelte.ts`:

```ts
// GM Screen modifiers runes store. Wraps the modifier IPC commands so
// components can call .add / .update / .delete / .setActive / .setHidden /
// .materializeAdvantage without re-fetching the full list themselves.
// Per spec §8.6: no auto-refetch on bridge://characters-updated — modifier
// records are independent of bridge state. Refetch only on mount + on
// successful CRUD response (which carries the updated row).

import {
  listAllCharacterModifiers,
  addCharacterModifier,
  updateCharacterModifier,
  deleteCharacterModifier,
  setModifierActive,
  setModifierHidden,
  materializeAdvantageModifier,
} from '$lib/modifiers/api';
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  SourceKind,
} from '../types';

let _list = $state<CharacterModifier[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);
let _initialized = false;

// UI preferences — per spec §8.3 / §7.5; held in-memory only, not persisted.
let _activeFilterTags = $state<Set<string>>(new Set());
let _showHidden = $state(false);
let _showOrphans = $state(false);

async function refresh(): Promise<void> {
  _loading = true;
  _error = null;
  try {
    _list = await listAllCharacterModifiers();
  } catch (e) {
    _error = String(e);
    console.error('[modifiers] refresh failed:', e);
  } finally {
    _loading = false;
  }
}

function mergeRow(updated: CharacterModifier): void {
  const i = _list.findIndex(m => m.id === updated.id);
  if (i >= 0) _list[i] = updated; else _list.push(updated);
}

function dropRow(id: number): void {
  _list = _list.filter(m => m.id !== id);
}

export const modifiers = {
  get list() { return _list; },
  get loading() { return _loading; },
  get error() { return _error; },

  // UI prefs (reactive getters/setters via runes)
  get activeFilterTags() { return _activeFilterTags; },
  setActiveFilterTags(next: Set<string>) { _activeFilterTags = new Set(next); },
  get showHidden() { return _showHidden; },
  set showHidden(v: boolean) { _showHidden = v; },
  get showOrphans() { return _showOrphans; },
  set showOrphans(v: boolean) { _showOrphans = v; },

  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
  },
  async refresh(): Promise<void> { await refresh(); },

  /** Lookup helpers — caller filters in-memory for free. */
  forCharacter(source: SourceKind, sourceId: string): CharacterModifier[] {
    return _list.filter(m => m.source === source && m.sourceId === sourceId);
  },

  /** CRUD — each refreshes the row in the local list from the response. */
  async add(input: NewCharacterModifierInput): Promise<CharacterModifier> {
    const row = await addCharacterModifier(input);
    mergeRow(row);
    return row;
  },
  async update(id: number, patch: ModifierPatchInput): Promise<CharacterModifier> {
    const row = await updateCharacterModifier(id, patch);
    mergeRow(row);
    return row;
  },
  async delete(id: number): Promise<void> {
    await deleteCharacterModifier(id);
    dropRow(id);
  },
  async setActive(id: number, isActive: boolean): Promise<void> {
    await setModifierActive(id, isActive);
    const i = _list.findIndex(m => m.id === id);
    if (i >= 0) _list[i] = { ..._list[i], isActive };
  },
  async setHidden(id: number, isHidden: boolean): Promise<void> {
    await setModifierHidden(id, isHidden);
    const i = _list.findIndex(m => m.id === id);
    if (i >= 0) _list[i] = { ..._list[i], isHidden };
  },
  async materializeAdvantage(args: {
    source: SourceKind;
    sourceId: string;
    itemId: string;
    name: string;
    description: string;
  }): Promise<CharacterModifier> {
    const row = await materializeAdvantageModifier(args);
    mergeRow(row);
    return row;
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
git add src/store/modifiers.svelte.ts
git commit -m "feat(store/modifiers): runes store with CRUD + UI prefs

Store mirrors savedCharacters.svelte.ts shape: initialized flag,
ensureLoaded / refresh, CRUD methods that merge the response row into
the local list (no full refetch). UI prefs (activeFilterTags, showHidden,
showOrphans) held in-memory only per spec §8.3."
```

---

## Task A6: TagFilterBar component

**Goal:** Build the chip filter strip used at the top of GM Screen. Mirrors `AdvantagesManager.svelte`'s chip pattern but with empty-set = no filter (spec §7.5) instead of an `__all__` sentinel.

**Files:**
- Create: `src/lib/components/gm-screen/TagFilterBar.svelte`

**Anti-scope:** Do not touch any other GM Screen component, the store, or backend.

**Depends on:** A5.

**Invariants cited:** ARCH §6 (CSS tokens from `:root`, no hex literals; layout in `rem`).

**Tests required:** NO — UI component (ARCH §10 has no frontend test framework).

- [ ] **Step 1: Create the component**

Create `src/lib/components/gm-screen/TagFilterBar.svelte`:

```svelte
<script lang="ts">
  // GM Screen tag filter chip bar. Empty active set = no filter (spec §7.5).
  // OR semantics: a card matches when it has any of the active tags.
  //
  // Mirrors the chip pattern from src/tools/AdvantagesManager.svelte but
  // without the __all__ sentinel — empty Set is the unfiltered state here.

  interface Props {
    allTags: string[];
    activeTags: Set<string>;
    onToggleTag: (tag: string) => void;
    onClearAll: () => void;
  }

  let { allTags, activeTags, onToggleTag, onClearAll }: Props = $props();

  let sortedTags = $derived([...new Set(allTags)].sort());
</script>

<div class="filter-bar">
  <span class="label">Filter:</span>
  <button
    class="chip"
    class:active={activeTags.size === 0}
    onclick={onClearAll}
  >All</button>
  {#each sortedTags as tag}
    <button
      class="chip"
      class:active={activeTags.has(tag)}
      onclick={() => onToggleTag(tag)}
    >{tag}</button>
  {/each}
</div>

<style>
  .filter-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border-faint);
  }
  .label {
    font-size: 0.75rem;
    color: var(--text-label);
    margin-right: 0.5rem;
  }
  .chip {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 999px;
    padding: 0.25rem 0.65rem;
    font-size: 0.75rem;
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }
  .chip:hover  { border-color: var(--border-surface); color: var(--text-primary); }
  .chip.active { border-color: var(--text-label); color: var(--text-primary); background: var(--bg-raised); }
</style>
```

- [ ] **Step 2: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green. (`npm run check` will validate the Svelte 5 runes syntax; `npm run build` covers the production bundle.)

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/TagFilterBar.svelte
git commit -m "feat(gm-screen): TagFilterBar chip strip

Empty active set = no filter (spec §7.5); OR semantics. Pattern mirrors
AdvantagesManager chip bar but without the __all__ sentinel since the
spec wants empty-set-as-unfiltered. Uses :root CSS tokens (ARCH §6)."
```

---

## Task A7: ModifierCard component (carousel + state axes)

**Goal:** Build the visually distinctive single-modifier card with the §7.2 stacked-overlapping carousel CSS (driven by `sibling-index()` / `sibling-count()` + neighbor-shift `:has()` cascade) and the §7.3 content layout (name + cog, effect summary lines, tag chips, toggle pill). The carousel layout math lives on the parent `.modifier-row` selector but the card itself owns its own positioning vars.

**Subagent dispatch hint (per spec §10):** This is a frontend-design candidate. When dispatching the implementer, hand them the spec §7.2 CSS verbatim (the `linear()` easing curve is load-bearing — copy it exactly) plus a pointer to `src/lib/components/AdvantageCard.svelte` and `src/routes/+layout.svelte` for the existing CSS-token vocabulary. Acceptable to dispatch via `frontend-design:frontend-design`.

**Files:**
- Create: `src/lib/components/gm-screen/ModifierCard.svelte`

**Anti-scope:** Do not touch other components, store, or backend.

**Depends on:** A5 (uses `CharacterModifier` from `src/types.ts`).

**Invariants cited:** ARCH §6 (CSS tokens, `box-sizing: border-box` for any width:100% + padding combo, rem units, no theme toggle, dark only); spec §7.2 (carousel CSS), §7.3 (card content layout), §7.6 (color tokens).

**Tests required:** NO — UI component.

- [ ] **Step 1: Create the component**

Create `src/lib/components/gm-screen/ModifierCard.svelte`:

```svelte
<script lang="ts">
  import type { CharacterModifier, ModifierEffect } from '../../../types';

  interface Props {
    /**
     * Card data. For an advantage-derived virtual card (no DB row yet) the
     * caller passes a synthesized object with id=0 and the displayable
     * name/description from the Foundry feature item.
     */
    modifier: CharacterModifier;
    /** Marks an advantage-derived card not yet materialized — UI shows asterisk */
    isVirtual?: boolean;
    /** Marks a stale card whose source merit was deleted — UI shows badge */
    isStale?: boolean;
    onToggleActive: () => void;
    onOpenEditor: (anchor: HTMLElement) => void;
    onHide: () => void;
  }

  let { modifier, isVirtual = false, isStale = false, onToggleActive, onOpenEditor, onHide }: Props = $props();

  let cogEl: HTMLButtonElement | undefined = $state();

  function summarize(e: ModifierEffect): string {
    if (e.kind === 'note') return e.note ?? 'note';
    const sign = (e.delta ?? 0) >= 0 ? '+' : '';
    const scope = e.scope ? `${e.scope} ` : '';
    const label = e.kind === 'pool' ? 'dice' : 'difficulty';
    return `${scope}${sign}${e.delta ?? 0} ${label}`;
  }
</script>

<div
  class="modifier-card"
  data-active={modifier.isActive ? 'true' : 'false'}
  data-hidden={modifier.isHidden ? 'true' : 'false'}
>
  <div class="head">
    <span class="name">
      {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}
      {#if isStale}<span class="stale" title="Source merit removed">stale</span>{/if}
    </span>
    <button
      bind:this={cogEl}
      class="cog"
      title="Edit effects"
      onclick={() => cogEl && onOpenEditor(cogEl)}
    >⚙</button>
  </div>
  <div class="effects">
    {#if modifier.effects.length === 0}
      <p class="no-effect">(no effect)</p>
    {:else}
      {#each modifier.effects as e}
        <p class="effect">{summarize(e)}</p>
      {/each}
    {/if}
  </div>
  {#if modifier.tags.length > 0}
    <div class="tags">
      {#each modifier.tags as t}<span class="tag">#{t}</span>{/each}
    </div>
  {/if}
  <div class="foot">
    <button
      class="toggle"
      class:on={modifier.isActive}
      onclick={onToggleActive}
    >{modifier.isActive ? 'ON' : 'OFF'}</button>
    {#if !modifier.isHidden}
      <button class="hide" title="Hide card" onclick={onHide}>×</button>
    {/if}
  </div>
</div>

<style>
  /* Per-card positioning variables — the parent .modifier-row provides
     --card-width / --card-overlap / --card-shift-delta / --cards (spec §7.2). */
  .modifier-card {
    --card-i: sibling-index();
    --base-x: calc((var(--card-i) - 1) * var(--card-width) * (1 - var(--card-overlap)));
    --shift-x: 0rem;
    --centre: calc((var(--cards) + 1) / 2);
    --distance: max(calc(var(--card-i) - var(--centre)), calc(var(--centre) - var(--card-i)));

    position: absolute;
    left: 0;
    top: 0;
    width: var(--card-width);
    height: 100%;
    padding: 0.6rem 0.75rem;
    box-sizing: border-box;            /* ARCH §6: no global reset */
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.625rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    z-index: calc(100 - var(--distance));

    transform: translateX(calc(var(--base-x) + var(--shift-x)));
    transition: transform var(--card-trans-duration) var(--card-trans-easing),
                box-shadow var(--card-trans-duration) var(--card-trans-easing),
                border-color 200ms ease;
  }

  .modifier-card:hover {
    z-index: 100;
    transform: translateX(calc(var(--base-x) + var(--shift-x))) translateY(-0.75rem) translateZ(20px);
    box-shadow: 0 1.25rem 2rem -0.5rem var(--accent);
  }

  /* neighbour-shift cascade — cards AFTER hovered slide right */
  .modifier-card:hover + :global(.modifier-card)                                              { --shift-x: calc(var(--card-shift-delta) * 3); }
  .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card)                    { --shift-x: calc(var(--card-shift-delta) * 2); }
  .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card) {
    --shift-x: calc(var(--card-shift-delta) * 1);
  }
  /* neighbour-shift cascade — cards BEFORE hovered slide left, via :has() */
  .modifier-card:has(+ :global(.modifier-card:hover))                                                                       { --shift-x: calc(var(--card-shift-delta) * -3); }
  .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card:hover))                                             { --shift-x: calc(var(--card-shift-delta) * -2); }
  .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card:hover))                   { --shift-x: calc(var(--card-shift-delta) * -1); }

  .modifier-card[data-active="true"] {
    border-color: var(--accent-bright);
    background: var(--bg-active);
  }
  .modifier-card[data-hidden="true"] {
    opacity: 0.45;
    filter: saturate(0.6);
  }

  .head { display: flex; align-items: center; justify-content: space-between; gap: 0.4rem; }
  .name { font-size: 0.85rem; color: var(--text-primary); font-weight: 500; }
  .virtual-mark { color: var(--accent-amber); margin-left: 0.15rem; }
  .stale { font-size: 0.65rem; color: var(--accent-amber); margin-left: 0.4rem; }
  .cog {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .modifier-card:hover .cog,
  .cog:focus { opacity: 1; }

  .effects { display: flex; flex-direction: column; gap: 0.15rem; }
  .effect, .no-effect { font-size: 0.7rem; margin: 0; color: var(--text-secondary); }
  .no-effect { color: var(--text-muted); font-style: italic; }

  .tags { display: flex; flex-wrap: wrap; gap: 0.2rem; }
  .tag { font-size: 0.65rem; color: var(--text-muted); }

  .foot { display: flex; justify-content: space-between; align-items: center; margin-top: auto; }
  .toggle {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 999px;
    padding: 0.15rem 0.55rem;
    font-size: 0.65rem;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .toggle.on {
    background: var(--accent);
    color: var(--text-primary);
    border-color: var(--accent-bright);
  }
  .hide {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .modifier-card:hover .hide,
  .hide:focus { opacity: 1; }

  @media (prefers-reduced-motion: reduce) {
    .modifier-card {
      transition: none;
    }
    .modifier-card:hover {
      transform: translateX(calc(var(--base-x) + var(--shift-x)));
    }
  }
</style>
```

- [ ] **Step 2: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "feat(gm-screen): ModifierCard with stacked-overlapping carousel CSS

Adapts the radial-fan technique to a horizontal straight layout per spec
§7.2: sibling-index() positioning, neighbour-shift cascade via :has(),
load-bearing linear() easing curve preserved. Three independent visual
state axes (data-active, data-hidden, hover). Reduced-motion fallback."
```

---

## Task A8: ModifierEffectEditor component (cog popover)

**Goal:** Build the inline popover anchored to a `ModifierCard`'s cog button. Lists `ModifierEffect` rows with `kind` / `scope` / `delta` / `note` widgets, plus an effect-add button and a tag chip editor. Save fires a callback the parent uses to commit a `ModifierPatch` (or `materializeAdvantage` then patch, when the source card is virtual).

**Subagent dispatch hint:** Frontend-design candidate. Reference `src/lib/components/AdvantageForm.svelte` for the existing chip-editor + multi-row field pattern.

**Files:**
- Create: `src/lib/components/gm-screen/ModifierEffectEditor.svelte`

**Anti-scope:** Do not touch other components, store, or backend.

**Depends on:** A5.

**Invariants cited:** ARCH §6 (tokens, rem, no hex literals); spec §7.3 (popover anchors to cog, not modal); spec §13 default — `delta` widget is `[− 0 +]` numeric stepper bounded -10..+10.

**Tests required:** NO — UI component.

- [ ] **Step 1: Create the component**

Create `src/lib/components/gm-screen/ModifierEffectEditor.svelte`:

```svelte
<script lang="ts">
  import type { ModifierEffect, ModifierKind } from '../../../types';

  interface Props {
    initialEffects: ModifierEffect[];
    initialTags: string[];
    onSave: (effects: ModifierEffect[], tags: string[]) => Promise<void>;
    onCancel: () => void;
  }

  let { initialEffects, initialTags, onSave, onCancel }: Props = $props();

  let effects = $state<ModifierEffect[]>(initialEffects.map(e => ({ ...e })));
  let tags = $state<string[]>([...initialTags]);
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

  function removeEffect(i: number) {
    effects = effects.filter((_, idx) => idx !== i);
  }

  function bumpDelta(i: number, by: number) {
    const cur = effects[i].delta ?? 0;
    const next = Math.max(-10, Math.min(10, cur + by));
    effects[i] = { ...effects[i], delta: next };
  }

  function setKind(i: number, kind: ModifierKind) {
    // When switching to/from 'note', clear the now-irrelevant fields per spec
    // §4 (delta=None for note kind, note=None for pool/difficulty kinds).
    if (kind === 'note') {
      effects[i] = { ...effects[i], kind, delta: null };
    } else {
      effects[i] = { ...effects[i], kind, note: null };
    }
  }

  function commitTag() {
    const t = newTag.trim();
    if (!t || tags.includes(t)) { newTag = ''; return; }
    tags = [...tags, t];
    newTag = '';
  }

  function removeTag(t: string) {
    tags = tags.filter(x => x !== t);
  }

  async function handleSave() {
    saving = true;
    error = null;
    try {
      await onSave(effects, tags);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="popover" role="dialog" aria-label="Edit modifier effects">
  <header>
    <h3>Effects</h3>
    <button class="close" onclick={onCancel} aria-label="Cancel">×</button>
  </header>

  <div class="effects-list">
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
            class="scope"
            value={effect.scope ?? ''}
            oninput={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              effects[i] = { ...effects[i], scope: v === '' ? null : v };
            }}
          />
          <div class="stepper">
            <button onclick={() => bumpDelta(i, -1)} aria-label="Decrement">−</button>
            <span class="delta">{effect.delta ?? 0}</span>
            <button onclick={() => bumpDelta(i, 1)} aria-label="Increment">+</button>
          </div>
        {/if}

        <button class="remove" onclick={() => removeEffect(i)} aria-label="Remove effect">×</button>
      </div>
    {/each}
    <button class="add" onclick={addEffect}>+ Add effect</button>
  </div>

  <div class="tags-section">
    <h4>Tags</h4>
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
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  <footer>
    <button class="secondary" onclick={onCancel}>Cancel</button>
    <button class="primary" onclick={handleSave} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </footer>
</div>

<style>
  .popover {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.5rem;
    padding: 0.85rem;
    width: 22rem;
    box-shadow: 0 0.75rem 2rem -0.25rem rgba(0,0,0,0.6);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-sizing: border-box;            /* ARCH §6 */
  }
  header { display: flex; align-items: center; justify-content: space-between; }
  header h3 { margin: 0; font-size: 0.9rem; color: var(--text-primary); }
  .close, .remove, .add, button.secondary, button.primary, .stepper button {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .close { padding: 0.1rem 0.4rem; }
  .effects-list { display: flex; flex-direction: column; gap: 0.4rem; }
  .effect-row {
    display: grid;
    grid-template-columns: 6rem 1fr auto auto;
    gap: 0.4rem;
    align-items: center;
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
  .stepper { display: inline-flex; gap: 0.25rem; align-items: center; }
  .stepper .delta {
    min-width: 1.6rem;
    text-align: center;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .tags-section h4 { margin: 0 0 0.4rem 0; font-size: 0.75rem; color: var(--text-label); font-weight: 500; }
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
  .tag-chip button {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.7rem;
    padding: 0;
  }
  .tag-list input { width: 6rem; }
  .error { color: var(--accent-amber); font-size: 0.75rem; margin: 0; }
  footer { display: flex; justify-content: flex-end; gap: 0.4rem; }
  button.primary { background: var(--accent); color: var(--text-primary); border-color: var(--accent-bright); }
</style>
```

- [ ] **Step 2: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/ModifierEffectEditor.svelte
git commit -m "feat(gm-screen): ModifierEffectEditor cog popover

Multi-effect editor with kind/scope/delta/note widgets per spec §7.3.
Delta widget is a [− 0 +] stepper bounded -10..+10 (spec §13 default).
Switching kind to/from 'note' clears the now-irrelevant field per
spec §4 (delta=None for note, note=None for pool/difficulty)."
```

---

## Task A9: CharacterRow component

**Goal:** Render one character's full row: header (hunger / WP / damage), horizontal carousel of modifier cards (advantage-derived virtual cards merged with materialized rows + free-floating), `+ Add modifier` button, materialize-on-engagement glue. Implements the spec §8.1 read flow + §8.2 materialize-on-engagement + sort logic (active DESC, created_at ASC).

**Files:**
- Create: `src/lib/components/gm-screen/CharacterRow.svelte`

**Anti-scope:** Do not touch other components, store, backend.

**Depends on:** A6, A7, A8.

**Invariants cited:** spec §8.1 (read flow + sort), §8.2 (materialize-on-engagement), §8.3 (hidden filter), spec §3 ("Stage 1 reads only Foundry advantages").

**Tests required:** NO — UI; the render-flow logic is small enough to verify by manual smoke (verify steps in Task A10).

- [ ] **Step 1: Create the component**

Create `src/lib/components/gm-screen/CharacterRow.svelte`:

```svelte
<script lang="ts">
  import { modifiers } from '../../../store/modifiers.svelte';
  import type {
    BridgeCharacter, CharacterModifier, ModifierEffect, FoundryItem,
  } from '../../../types';
  import ModifierCard from './ModifierCard.svelte';
  import ModifierEffectEditor from './ModifierEffectEditor.svelte';

  interface Props {
    character: BridgeCharacter;
    activeFilterTags: Set<string>;
    showHidden: boolean;
  }
  let { character, activeFilterTags, showHidden }: Props = $props();

  // Editor popover state — anchored to the cog via getBoundingClientRect()
  // (spec §7.3 "anchored to the cog itself, not a modal"). popoverPos uses
  // viewport coords so the wrap is position:fixed.
  type EditorTarget = { kind: 'materialized', mod: CharacterModifier }
                    | { kind: 'virtual', virt: VirtualCard };
  let editorOpen = $state(false);
  let editorTarget = $state<EditorTarget | null>(null);
  let popoverPos = $state<{ left: number; top: number } | null>(null);

  /**
   * Virtual cards are advantage-derived rows the GM hasn't engaged yet — no
   * DB id. Materialize on first engagement (toggle / hide / save edits).
   */
  interface VirtualCard {
    item: FoundryItem;
    name: string;
    description: string;
  }

  // Stage 1: only Foundry advantages auto-render (spec §3 / Roll20 deferred to Phase 2.5).
  let foundryItems = $derived(
    character.source === 'foundry' && character.raw && typeof character.raw === 'object' && 'items' in (character.raw as Record<string, unknown>)
      ? ((character.raw as { items: FoundryItem[] }).items ?? [])
      : []
  );

  let advantageItems = $derived(
    foundryItems.filter(it => {
      if (it.type !== 'feature') return false;
      const ft = (it.system as Record<string, unknown>)?.featuretype as string | undefined;
      return ft === 'merit' || ft === 'flaw' || ft === 'background' || ft === 'boon';
    })
  );

  let charMods = $derived(modifiers.forCharacter(character.source, character.source_id));

  // Build the card list per spec §8.1.
  type CardEntry =
    | { kind: 'materialized'; mod: CharacterModifier; isStale: boolean }
    | { kind: 'virtual'; virt: VirtualCard };

  let cardEntries = $derived.by((): CardEntry[] => {
    const entries: CardEntry[] = [];

    // (2) Walk advantage items, merging with materialized rows.
    for (const item of advantageItems) {
      const matched = charMods.find(m => m.binding.kind === 'advantage' && m.binding.item_id === item._id);
      if (matched) {
        entries.push({ kind: 'materialized', mod: matched, isStale: false });
      } else {
        entries.push({ kind: 'virtual', virt: {
          item,
          name: item.name,
          description: ((item.system as Record<string, unknown>)?.description as string | undefined) ?? '',
        }});
      }
    }

    // (3) Append free-floating modifiers (and any 'advantage' mods whose item was deleted — these become stale).
    const knownAdvantageItemIds = new Set(advantageItems.map(it => it._id));
    for (const m of charMods) {
      if (m.binding.kind === 'free') {
        entries.push({ kind: 'materialized', mod: m, isStale: false });
      } else if (m.binding.kind === 'advantage' && !knownAdvantageItemIds.has(m.binding.item_id)) {
        entries.push({ kind: 'materialized', mod: m, isStale: true });
      }
    }
    return entries;
  });

  // (4) Apply filter — active cards always pinned past the filter (spec §7.5).
  function passesTagFilter(e: CardEntry): boolean {
    if (activeFilterTags.size === 0) return true;
    if (e.kind === 'virtual') return false; // virtual has no tags yet
    if (e.mod.isActive) return true;        // active pin rule
    return e.mod.tags.some(t => activeFilterTags.has(t));
  }

  function passesHiddenFilter(e: CardEntry): boolean {
    if (e.kind === 'virtual') return true;
    if (e.mod.isHidden) return showHidden;
    return true;
  }

  // (5) Sort: active DESC, then created_at ASC for materialized; virtuals sort by item name.
  function sortKey(e: CardEntry): [number, string] {
    if (e.kind === 'virtual') return [1, e.virt.name];
    return [e.mod.isActive ? 0 : 1, e.mod.createdAt];
  }

  let visibleCards = $derived(
    cardEntries
      .filter(e => passesTagFilter(e) && passesHiddenFilter(e))
      .sort((a, b) => {
        const [ak, an] = sortKey(a);
        const [bk, bn] = sortKey(b);
        if (ak !== bk) return ak - bk;
        return an < bn ? -1 : an > bn ? 1 : 0;
      })
  );

  /** Materialize a virtual card before applying any change. */
  async function materialize(virt: VirtualCard): Promise<CharacterModifier> {
    return await modifiers.materializeAdvantage({
      source: character.source,
      sourceId: character.source_id,
      itemId: virt.item._id,
      name: virt.name,
      description: virt.description,
    });
  }

  async function handleToggleActive(e: CardEntry): Promise<void> {
    if (e.kind === 'virtual') {
      const m = await materialize(e.virt);
      await modifiers.setActive(m.id, true);
    } else {
      await modifiers.setActive(e.mod.id, !e.mod.isActive);
    }
  }

  async function handleHide(e: CardEntry): Promise<void> {
    if (e.kind === 'virtual') {
      const m = await materialize(e.virt);
      await modifiers.setHidden(m.id, true);
    } else {
      await modifiers.setHidden(e.mod.id, true);
    }
  }

  function openEditor(e: CardEntry, anchor: HTMLElement): void {
    editorTarget = e.kind === 'materialized'
      ? { kind: 'materialized', mod: e.mod }
      : { kind: 'virtual', virt: e.virt };
    // Anchor the popover just to the right of the cog and slightly below.
    // Viewport coords pair with position:fixed below.
    const rect = anchor.getBoundingClientRect();
    popoverPos = { left: rect.right + 8, top: rect.bottom + 4 };
    editorOpen = true;
  }

  function closeEditor(): void {
    editorOpen = false;
    editorTarget = null;
    popoverPos = null;
  }

  async function saveEditor(effects: ModifierEffect[], tags: string[]): Promise<void> {
    if (!editorTarget) return;
    let id: number;
    if (editorTarget.kind === 'virtual') {
      const m = await materialize(editorTarget.virt);
      id = m.id;
    } else {
      id = editorTarget.mod.id;
    }
    await modifiers.update(id, { effects, tags });
    closeEditor();
  }

  async function addFreeModifier(): Promise<void> {
    await modifiers.add({
      source: character.source,
      sourceId: character.source_id,
      name: 'New modifier',
      description: '',
      effects: [],
      binding: { kind: 'free' },
      tags: [],
      originTemplateId: null,
    });
  }

  function damageSummary(): string {
    if (!character.health) return '—';
    const { superficial, aggravated } = character.health;
    if (superficial === 0 && aggravated === 0) return 'Dmg —';
    return `Dmg ${superficial}s/${aggravated}a`;
  }
</script>

<section class="row" data-source={character.source}>
  <header>
    <h2>{character.name}</h2>
    <span class="source">{character.source}</span>
    {#if character.hunger != null}<span class="stat">Hunger {character.hunger}</span>{/if}
    {#if character.willpower}
      <span class="stat">WP {character.willpower.max - character.willpower.superficial - character.willpower.aggravated}/{character.willpower.max}</span>
    {/if}
    <span class="stat">{damageSummary()}</span>
  </header>

  <div
    class="modifier-row"
    style="--cards: {visibleCards.length};"
  >
    {#each visibleCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
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
              createdAt: '',
              updatedAt: '',
            }
          : entry.mod}
        isVirtual={entry.kind === 'virtual'}
        isStale={entry.kind === 'materialized' && entry.isStale}
        onToggleActive={() => handleToggleActive(entry)}
        onHide={() => handleHide(entry)}
        onOpenEditor={(anchor) => openEditor(entry, anchor)}
      />
    {/each}
    <button class="add-modifier" onclick={addFreeModifier}>+ Add modifier</button>
  </div>

  {#if editorOpen && editorTarget && popoverPos}
    <div class="popover-wrap" style="left: {popoverPos.left}px; top: {popoverPos.top}px;">
      <ModifierEffectEditor
        initialEffects={editorTarget.kind === 'materialized' ? editorTarget.mod.effects : []}
        initialTags={editorTarget.kind === 'materialized' ? editorTarget.mod.tags : []}
        onSave={saveEditor}
        onCancel={closeEditor}
      />
    </div>
  {/if}
</section>

<style>
  .row {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.5rem;
    padding: 0.75rem;
    margin-bottom: 0.6rem;
    box-sizing: border-box;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 0.65rem;
    margin-bottom: 0.6rem;
  }
  header h2 { margin: 0; font-size: 0.95rem; color: var(--text-primary); }
  .source {
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .stat { font-size: 0.75rem; color: var(--text-secondary); }

  .modifier-row {
    --card-trans-duration: 600ms;
    --card-trans-easing: linear(
      0, 0.01 0.8%, 0.038 1.6%, 0.154 3.4%, 0.781 9.7%, 1.01 12.5%,
      1.089 13.8%, 1.153 15.2%, 1.195 16.6%, 1.219 18%, 1.224 19.7%,
      1.208 21.6%, 1.172 23.6%, 1.057 28.6%, 1.007 31.2%, 0.969 34.1%,
      0.951 37.1%, 0.953 40.9%, 0.998 50.4%, 1.011 56%, 0.998 74.7%, 1
    );
    --card-width: 9rem;
    --card-overlap: 0.55;
    --card-shift-delta: 0.5rem;
    /* --cards is set inline via the style prop above to drive the z-stack centering math. */
    position: relative;
    height: 8rem;
    perspective: 800px;
  }

  .add-modifier {
    position: absolute;
    /* placed past the last card; uses the same overlap math */
    left: calc(var(--cards) * var(--card-width) * (1 - var(--card-overlap)));
    top: 0;
    height: 100%;
    width: 9rem;
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px dashed var(--border-faint);
    border-radius: 0.625rem;
    cursor: pointer;
    box-sizing: border-box;
  }
  .add-modifier:hover { color: var(--text-primary); border-color: var(--border-surface); }

  .popover-wrap {
    /* Anchored to the cog via getBoundingClientRect() — viewport coords. */
    position: fixed;
    z-index: 1000;
  }

  @media (prefers-reduced-motion: reduce) {
    .modifier-row {
      --card-overlap: 0;
      --card-shift-delta: 0;
    }
  }
</style>
```

- [ ] **Step 2: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "feat(gm-screen): CharacterRow with virtual + materialized merge

Implements spec §8.1 read flow: walks Foundry canonical.raw.items for
advantage-derived virtual cards, merges with materialized rows from the
modifiers store, appends free-floating, applies filter (active cards
pinned past filter — spec §7.5), sorts active DESC + created_at ASC.
Materialize-on-engagement glue (§8.2) routes any toggle/hide/save on a
virtual card through materializeAdvantage first."
```

---

## Task A10: GmScreen tool + tool registry entry

**Goal:** Land the top-level tool component, register it in the sidebar, and run a manual smoke test of the full Plan A surface.

**Files:**
- Create: `src/tools/GmScreen.svelte`
- Modify: `src/tools.ts` (add one entry)

**Anti-scope:** Do not touch other components, store, backend.

**Depends on:** A6, A7, A8, A9.

**Invariants cited:** ARCH §9 ("add a tool" seam — one entry in `src/tools.ts`).

**Tests required:** NO — but the manual smoke at Step 4 is the gate that proves Plan A works end-to-end.

- [ ] **Step 1: Create `src/tools/GmScreen.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge, initBridge } from '../store/bridge.svelte';
  import { savedCharacters } from '../store/savedCharacters.svelte';
  import { modifiers } from '../store/modifiers.svelte';
  import TagFilterBar from '$lib/components/gm-screen/TagFilterBar.svelte';
  import CharacterRow from '$lib/components/gm-screen/CharacterRow.svelte';
  import type { BridgeCharacter, SourceKind } from '../types';

  onMount(() => {
    void initBridge();
    void savedCharacters.ensureLoaded();
    void modifiers.ensureLoaded();
  });

  // All tags currently in use across all materialized modifiers — drives the chip bar.
  let allTags = $derived(
    [...new Set(modifiers.list.flatMap(m => m.tags))].sort()
  );

  function toggleTag(t: string): void {
    const next = new Set(modifiers.activeFilterTags);
    if (next.has(t)) next.delete(t); else next.add(t);
    modifiers.setActiveFilterTags(next);
  }

  function clearFilter(): void {
    modifiers.setActiveFilterTags(new Set());
  }

  // Orphans = modifier rows whose (source, source_id) matches no live or saved char.
  let orphans = $derived(
    modifiers.list.filter(m => {
      const liveMatch = bridge.characters.some(
        c => c.source === m.source && c.source_id === m.sourceId
      );
      const savedMatch = savedCharacters.list.some(
        s => s.source === m.source && s.sourceId === m.sourceId
      );
      return !liveMatch && !savedMatch;
    })
  );

  // Synthesize a BridgeCharacter shell for a saved-only character so CharacterRow renders it.
  function savedAsBridge(s: { source: SourceKind; sourceId: string; canonical: BridgeCharacter }): BridgeCharacter {
    return s.canonical;
  }

  // Combined character list: live first, then saved-only (no live match).
  let displayCharacters = $derived.by((): BridgeCharacter[] => {
    const live = bridge.characters;
    const liveKeys = new Set(live.map(c => `${c.source}:${c.source_id}`));
    const savedOnly = savedCharacters.list
      .filter(s => !liveKeys.has(`${s.source}:${s.sourceId}`))
      .map(savedAsBridge);
    return [...live, ...savedOnly];
  });
</script>

<div class="gm-screen">
  <header class="title-bar">
    <h1>🛡 GM Screen</h1>
    <div class="toggles">
      <label>
        <input
          type="checkbox"
          checked={modifiers.showHidden}
          onchange={(e) => modifiers.showHidden = (e.currentTarget as HTMLInputElement).checked}
        /> Show hidden
      </label>
      <label>
        <input
          type="checkbox"
          checked={modifiers.showOrphans}
          onchange={(e) => modifiers.showOrphans = (e.currentTarget as HTMLInputElement).checked}
        /> Show orphans
      </label>
    </div>
  </header>

  <TagFilterBar
    {allTags}
    activeTags={modifiers.activeFilterTags}
    onToggleTag={toggleTag}
    onClearAll={clearFilter}
  />

  <div class="rows">
    {#if displayCharacters.length === 0}
      <p class="empty">No characters available. Connect Foundry or Roll20, or load a saved character.</p>
    {:else}
      {#each displayCharacters as char (`${char.source}:${char.source_id}`)}
        <CharacterRow
          character={char}
          activeFilterTags={modifiers.activeFilterTags}
          showHidden={modifiers.showHidden}
        />
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
</div>

<style>
  .gm-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-base);
    color: var(--text-primary);
    box-sizing: border-box;
  }
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .title-bar h1 { margin: 0; font-size: 1.05rem; }
  .toggles { display: flex; gap: 1rem; font-size: 0.8rem; color: var(--text-secondary); }
  .toggles label { display: inline-flex; gap: 0.3rem; align-items: center; cursor: pointer; }
  .rows { flex: 1; overflow-y: auto; padding: 0.75rem 1rem; }
  .empty { color: var(--text-muted); font-style: italic; }
  .orphans { margin-top: 1rem; padding-top: 0.75rem; border-top: 1px solid var(--border-faint); }
  .orphans h2 { font-size: 0.85rem; margin: 0 0 0.4rem 0; color: var(--text-label); }
  .orphans .hint { font-size: 0.7rem; color: var(--text-muted); margin: 0 0 0.5rem 0; }
  .orphan-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0;
    font-size: 0.8rem;
  }
  .meta { color: var(--text-muted); font-size: 0.7rem; font-family: monospace; }
</style>
```

- [ ] **Step 2: Add the tool registry entry**

Modify `src/tools.ts` — append one entry to the `tools` array (after the `foundry-test` entry):

```ts
  {
    id: 'gm-screen',
    label: 'GM Screen',
    icon: '🛡',
    component: () => import('./tools/GmScreen.svelte'),
  },
```

- [ ] **Step 3: Run verify.sh**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 4: Manual smoke test (per spec §10 Plan A verification)**

Start the dev app:

```bash
npm run tauri dev
```

In the app:
1. Open the new `🛡 GM Screen` tool from the sidebar.
2. Connect Foundry (or load a saved character to exercise the saved-only path).
3. Verify a character row renders with derived advantage cards (asterisk-marked) for any merits/flaws/backgrounds/boons on the actor.
4. Click `+ Add modifier` — a free-floating card appears with name "New modifier".
5. Click the new card's cog → editor popover opens **anchored just to the right of and below the cog button** (spec §7.3 — verify the popover is visually attached to the clicked cog, not floating in a fixed page corner; scroll the page after opening and confirm the popover stays at viewport coords as designed). Add an effect (Pool, scope "Social", delta +1), add a tag "Social", Save → card shows the effect summary and the tag chip.
6. Click the new card's `OFF` toggle → flips to `ON`, card border + background switches to active state. Click again → flips back.
7. Click the `×` (hide) on the card → card disappears. Toggle `Show hidden` in the title bar → muted card reappears.
8. Click an asterisk-marked (virtual) advantage card's cog → editor opens. Add an effect → on save, the card materializes (asterisk disappears) and shows the effect.
9. Add a second tag chip via the filter bar — verify only matching cards remain visible BUT any active card stays pinned (spec §7.5).
10. Toggle `Show orphans` — verify the section is empty (no orphans yet).

If any step fails, file a bug + fix before committing.

- [ ] **Step 5: Commit**

```bash
git add src/tools/GmScreen.svelte src/tools.ts
git commit -m "feat(tool): GM Screen — modifier dashboard (Plan A)

Closes Plan A of GM Screen stage 1. Top-level tool composes TagFilterBar
+ per-character CharacterRow + orphan section. Reads from bridge store +
saved-characters store + modifiers store. Status palette (Plan B) wires
into the right side of this layout in a follow-up.

Spec: docs/superpowers/specs/2026-05-03-gm-screen-design.md (stage 1 §10
Plan A)."
```

---

## Plan A self-review checklist

After completing all 10 tasks, before declaring Plan A done:

**1. Spec coverage** — every §3-§9 requirement implemented?

| Spec section | Implemented in |
|---|---|
| §3 advantage-derived render | A9 (CharacterRow walks `canonical.raw.items` Foundry path) |
| §3 free-floating add | A9 (`addFreeModifier`) |
| §3 status template instance | Plan B |
| §4 schema + types | A1 |
| §5 8 commands | A1 (list ×2), A2 (add/update/delete), A3 (set_active/set_hidden/materialize) |
| §6 frontend file inventory | A1 (types.ts), A4 (api.ts), A5 (store), A6-A10 (components) |
| §7 layout + carousel + content | A6 (filter), A7 (card), A8 (editor), A9 (row), A10 (tool) |
| §8.1 read flow | A9 |
| §8.2 materialize-on-engagement | A9 (`materialize` helper) |
| §8.3 hide/show | A7 (data-hidden CSS), A9 (`passesHiddenFilter`), A10 (toggle) |
| §8.5 orphans | A10 |
| §8.6 no auto-refetch | A5 (store comment + design) |
| §9 error contracts | A1-A3 (all command errors prefixed `db/modifier.<op>:`) |
| §11 Rust tests | A1-A3 (~19 tests inline) |

**2. Placeholder scan** — search the plan files for: `TBD`, `TODO`, `placeholder`, `implement later`, `add appropriate`. Should return zero hits.

**3. Type consistency** — names referenced across tasks:
- `db_add` / `db_update` / `db_delete` / `db_get` / `db_set_active` / `db_set_hidden` / `db_materialize_advantage` / `db_list` / `db_list_all` — all defined in `src-tauri/src/db/modifier.rs` (A1-A3)
- `addCharacterModifier` / `updateCharacterModifier` / `deleteCharacterModifier` / `setModifierActive` / `setModifierHidden` / `materializeAdvantageModifier` / `listCharacterModifiers` / `listAllCharacterModifiers` — all in `src/lib/modifiers/api.ts` (A4)
- `modifiers.add` / `.update` / `.delete` / `.setActive` / `.setHidden` / `.materializeAdvantage` / `.forCharacter` / `.ensureLoaded` / `.refresh` / `.activeFilterTags` / `.setActiveFilterTags` / `.showHidden` / `.showOrphans` / `.list` — all on the store in A5; consumed by A9 + A10

**4. After all tasks committed:** dispatch ONE `code-review:code-review` against the full Plan A branch diff (per project lean-execution override).
