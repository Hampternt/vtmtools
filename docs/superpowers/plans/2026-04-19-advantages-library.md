# Advantages Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a local curated library of VTM 5e Merits, Backgrounds, and Flaws — `Advantages` — with CRUD, tag filtering, text search, flexible per-row typed fields, and destructive reseed of the built-in set.

**Architecture:** Clone the Dyscrasia pattern: single SQLite table (`advantages`), five Tauri commands in `src-tauri/src/db/advantage.rs`, typed TS wrapper at `src/lib/advantages/api.ts`, Svelte tool + Card + Form components. The schema stores only `id / name / description / tags_json / properties_json / is_custom` — per-row attributes like `level`, `min_level`, `max_level` live inside `properties_json` as `Vec<Field>` blobs, reusing the existing `Field` / `FieldValue` infrastructure. The generic `PropertyEditor` and its widget registry currently under `components/domains/` are moved to `components/properties/` so both Domains and Advantages consume from the same shared location.

**Tech Stack:** Rust (Tauri 2, sqlx, serde, rand), TypeScript, Svelte 5 runes mode, SQLite, `serde_json` for tags/properties blobs.

**Spec reference:** `docs/superpowers/specs/2026-04-19-advantages-library-design.md`

**Conventions cited:**
- `ARCHITECTURE.md` §2 (Field / FieldValue shapes), §3 (seed policy), §4 (Tauri IPC contracts), §5 (module boundaries), §6 (invariants — CSS Grid + `align-items: start`, rem not px, tokens not hex), §7 (error prefixes), §9 (extensibility seams), §10 (testing + verify.sh).
- `ADR 0002` (destructive reseed of `is_custom = 0` rows).
- `ADR 0003` (freeform strings — tags are freeform, chips derived from data).

---

## File map

**Create:**
- `src-tauri/migrations/0003_advantages.sql` — DDL for `advantages` table.
- `src-tauri/src/db/advantage.rs` — helpers + 5 `#[tauri::command]` handlers + inline tests.
- `src/lib/advantages/api.ts` — typed IPC wrapper.
- `src/lib/advantages/fieldPresets.ts` — quick-add preset constant.
- `src/lib/components/AdvantageCard.svelte` — library card.
- `src/lib/components/AdvantageForm.svelte` — add/edit form.
- `src/lib/components/properties/PropertyEditor.svelte` — **moved** from `components/domains/`.
- `src/lib/components/properties/property-widgets/{index.ts, BoolWidget.svelte, NumberWidget.svelte, StringWidget.svelte, TextWidget.svelte}` — **moved** from `components/domains/property-widgets/`.
- `src/tools/AdvantagesManager.svelte` — top-level tool.

**Modify:**
- `src-tauri/src/shared/types.rs` — add `Advantage` struct.
- `src-tauri/src/db/mod.rs` — `pub mod advantage;`.
- `src-tauri/src/db/seed.rs` — add `seed_advantages` function.
- `src-tauri/src/lib.rs` — call `seed_advantages`; register 5 commands in `invoke_handler!`.
- `src/types.ts` — mirror `Advantage` type.
- `src/lib/components/domains/NodeForm.svelte` — update 2 import paths after PropertyEditor move.
- `src/tools.ts` — add `advantages` tool entry.

**Delete:**
- `src/lib/components/domains/PropertyEditor.svelte` (moved).
- `src/lib/components/domains/property-widgets/` (moved — 5 files + directory).

---

## Task 1: Domain contract (migration + Rust struct + TS mirror)

One commit — schema and mirror land together per ARCHITECTURE.md §9.

**Files:**
- Create: `src-tauri/migrations/0003_advantages.sql`
- Modify: `src-tauri/src/shared/types.rs` (after the `DyscrasiaEntry` struct at line 114, before the `ResonanceRollResult` struct around line 124)
- Modify: `src/types.ts` (after the `DyscrasiaEntry` interface around line 27)

- [ ] **Step 1: Create the migration file**

Create `src-tauri/migrations/0003_advantages.sql`:

```sql
-- Local library of VTM 5e Merits, Backgrounds, and Flaws (collectively Advantages).
-- Per-row attributes (level, min_level, max_level, source, prereq, …) live inside
-- properties_json, mirroring nodes.properties_json so the future character builder
-- can consume advantages natively.
CREATE TABLE IF NOT EXISTS advantages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    is_custom       INTEGER NOT NULL DEFAULT 0
);
```

- [ ] **Step 2: Add the `Advantage` struct to `src-tauri/src/shared/types.rs`**

Insert after the `DyscrasiaEntry` struct (around line 121):

```rust
/// A library entry for a VTM 5e Merit, Background, or Flaw.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Advantage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,
    pub is_custom: bool,
}
```

Note: `Field` is already defined later in the same file (around line 193) but Rust allows forward references within a module, so ordering is fine. If the compiler complains, move the struct below `Field`.

- [ ] **Step 3: Add the `Advantage` TS mirror to `src/types.ts`**

Insert after the `DyscrasiaEntry` interface (around line 28), before the `ResonanceRollResult` interface:

```ts
export interface Advantage {
  id: number;
  name: string;
  description: string;
  tags: string[];
  properties: Field[];
  isCustom: boolean;
}
```

`Field` is defined later in the same file (around line 81) — TypeScript hoists type references so ordering is fine.

- [ ] **Step 4: Run cargo check to verify the Rust side compiles**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: completes with 0 errors. Existing "never constructed" warnings on `FieldValue::{Date, Url, Email, Reference}` may persist — they are expected per ARCHITECTURE.md §10.

- [ ] **Step 5: Run npm run check to verify TypeScript**

Run:
```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations/0003_advantages.sql src-tauri/src/shared/types.rs src/types.ts
git commit -m "$(cat <<'EOF'
feat(advantages): add schema + Advantage domain type

Migration 0003 adds the advantages table (id, name, description,
tags_json, properties_json, is_custom) — schema kept minimal; per-row
attributes live inside properties_json as Vec<Field> blobs, mirroring
nodes.properties_json. Rust struct + TS mirror land in the same commit
per ARCHITECTURE.md §9.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: DB helpers + commands + inline tests

TDD: define internal helpers first with tests, then wrap them in `#[tauri::command]` handlers. Helpers are what the tests target (`tauri::State` is awkward to construct in unit tests) — same pattern as `db/dyscrasia.rs`.

**Files:**
- Create: `src-tauri/src/db/advantage.rs`

- [ ] **Step 1: Create the module skeleton with failing tests (list + insert round-trip)**

Create `src-tauri/src/db/advantage.rs`:

```rust
use rand::seq::SliceRandom;
use sqlx::{Row, SqlitePool};
use crate::shared::types::{Advantage, Field};

// --------------------------------------------------------------------------
// JSON serde helpers for tags_json and properties_json columns
// --------------------------------------------------------------------------

fn serialize_tags(tags: &[String]) -> Result<String, String> {
    serde_json::to_string(tags).map_err(|e| e.to_string())
}

fn deserialize_tags(s: &str) -> Result<Vec<String>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

fn serialize_properties(props: &[Field]) -> Result<String, String> {
    serde_json::to_string(props).map_err(|e| e.to_string())
}

fn deserialize_properties(s: &str) -> Result<Vec<Field>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

// --------------------------------------------------------------------------
// Internal helpers (testable — take &SqlitePool directly)
// --------------------------------------------------------------------------

async fn db_list(pool: &SqlitePool) -> Result<Vec<Advantage>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, tags_json, properties_json, is_custom
         FROM advantages ORDER BY is_custom ASC, id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows.iter() {
        let tags_json: String = r.get("tags_json");
        let properties_json: String = r.get("properties_json");
        out.push(Advantage {
            id: r.get("id"),
            name: r.get("name"),
            description: r.get("description"),
            tags: deserialize_tags(&tags_json)?,
            properties: deserialize_properties(&properties_json)?,
            is_custom: r.get::<bool, _>("is_custom"),
        });
    }
    Ok(out)
}

async fn db_insert(
    pool: &SqlitePool,
    name: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<Advantage, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;

    let result = sqlx::query(
        "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
         VALUES (?, ?, ?, ?, 1)"
    )
    .bind(name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.insert: {}", e))?;

    Ok(Advantage {
        id: result.last_insert_rowid(),
        name: name.to_string(),
        description: description.to_string(),
        tags: tags.to_vec(),
        properties: properties.to_vec(),
        is_custom: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{Field, FieldValue, NumberFieldValue};
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE advantages (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL,
                description     TEXT NOT NULL DEFAULT '',
                tags_json       TEXT NOT NULL DEFAULT '[]',
                properties_json TEXT NOT NULL DEFAULT '[]',
                is_custom       INTEGER NOT NULL DEFAULT 0
            )"
        ).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = test_pool().await;
        let result = db_list(&pool).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn insert_and_list_round_trips_tags_and_properties() {
        let pool = test_pool().await;
        let tags = vec!["VTM 5e".to_string(), "Merit".to_string()];
        let props = vec![Field {
            name: "level".to_string(),
            value: FieldValue::Number { value: NumberFieldValue::Single(3.0) },
        }];

        let inserted = db_insert(&pool, "Iron Gullet", "Can drink rancid blood", &tags, &props)
            .await
            .unwrap();
        assert_eq!(inserted.name, "Iron Gullet");
        assert!(inserted.is_custom);

        let entries = db_list(&pool).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tags, tags);
        assert_eq!(entries[0].properties, props);
    }
}
```

- [ ] **Step 2: Register the module**

Modify `src-tauri/src/db/mod.rs`:

```rust
pub mod dyscrasia;
pub mod seed;

pub mod chronicle;
pub mod node;
pub mod edge;

pub mod advantage;
```

- [ ] **Step 3: Run tests — confirm they pass**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml advantage
```

Expected: 2 tests pass (`list_empty_returns_empty_vec`, `insert_and_list_round_trips_tags_and_properties`).

- [ ] **Step 4: Add update + delete helpers and their tests**

Append to `src-tauri/src/db/advantage.rs` (above the `#[cfg(test)]` block):

```rust
async fn db_update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<(), String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;

    let result = sqlx::query(
        "UPDATE advantages
         SET name = ?, description = ?, tags_json = ?, properties_json = ?
         WHERE id = ? AND is_custom = 1"
    )
    .bind(name)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.update: {}", e))?;

    if result.rows_affected() == 0 {
        return Err("db/advantage.update: row not found or not editable".to_string());
    }
    Ok(())
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    let result = sqlx::query("DELETE FROM advantages WHERE id = ? AND is_custom = 1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/advantage.delete: {}", e))?;

    if result.rows_affected() == 0 {
        return Err("db/advantage.delete: row not found or not deletable".to_string());
    }
    Ok(())
}
```

Append inside the `mod tests` block:

```rust
    #[tokio::test]
    async fn update_rejects_builtin_row() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
             VALUES ('Allies', '', '[]', '[]', 0)"
        ).execute(&pool).await.unwrap();

        let err = db_update(&pool, 1, "X", "", &[], &[]).await.unwrap_err();
        assert!(err.contains("not editable"));
    }

    #[tokio::test]
    async fn delete_rejects_builtin_row() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
             VALUES ('Allies', '', '[]', '[]', 0)"
        ).execute(&pool).await.unwrap();

        let err = db_delete(&pool, 1).await.unwrap_err();
        assert!(err.contains("not deletable"));
    }

    #[tokio::test]
    async fn update_succeeds_on_custom_row() {
        let pool = test_pool().await;
        let inserted = db_insert(&pool, "Old Name", "", &[], &[]).await.unwrap();
        db_update(&pool, inserted.id, "New Name", "desc", &[], &[]).await.unwrap();
        let rows = db_list(&pool).await.unwrap();
        assert_eq!(rows[0].name, "New Name");
    }

    #[tokio::test]
    async fn delete_succeeds_on_custom_row() {
        let pool = test_pool().await;
        let inserted = db_insert(&pool, "To Delete", "", &[], &[]).await.unwrap();
        db_delete(&pool, inserted.id).await.unwrap();
        assert!(db_list(&pool).await.unwrap().is_empty());
    }
```

- [ ] **Step 5: Run tests — confirm all 6 pass**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml advantage
```

Expected: 6 tests pass.

- [ ] **Step 6: Add `db_roll_random` helper and its tests**

Append to `src-tauri/src/db/advantage.rs` (above the `#[cfg(test)]` block):

```rust
async fn db_roll_random(
    pool: &SqlitePool,
    tags: &[String],
) -> Result<Option<Advantage>, String> {
    let all = db_list(pool).await?;

    let pool_of_matches: Vec<Advantage> = if tags.is_empty() {
        all
    } else {
        all.into_iter()
            .filter(|row| row.tags.iter().any(|t| tags.contains(t)))
            .collect()
    };

    if pool_of_matches.is_empty() {
        return Ok(None);
    }
    Ok(pool_of_matches.choose(&mut rand::thread_rng()).cloned())
}
```

Append inside the `mod tests` block:

```rust
    async fn seed_three(pool: &SqlitePool) {
        db_insert(pool, "M1", "", &vec!["Merit".to_string()],     &[]).await.unwrap();
        db_insert(pool, "B1", "", &vec!["Background".to_string()], &[]).await.unwrap();
        db_insert(pool, "F1", "", &vec!["Flaw".to_string()],       &[]).await.unwrap();
    }

    #[tokio::test]
    async fn roll_random_empty_tags_returns_any_row() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        let picked = db_roll_random(&pool, &[]).await.unwrap();
        assert!(picked.is_some(), "expected Some(row), got None");
    }

    #[tokio::test]
    async fn roll_random_single_tag_returns_only_matching() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        for _ in 0..20 {
            let picked = db_roll_random(&pool, &["Merit".to_string()]).await.unwrap().unwrap();
            assert!(picked.tags.contains(&"Merit".to_string()));
        }
    }

    #[tokio::test]
    async fn roll_random_multi_tag_is_or_match() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        let filter = vec!["Merit".to_string(), "Background".to_string()];
        for _ in 0..20 {
            let picked = db_roll_random(&pool, &filter).await.unwrap().unwrap();
            assert!(picked.tags.iter().any(|t| filter.contains(t)));
            assert!(!picked.tags.contains(&"Flaw".to_string()));
        }
    }

    #[tokio::test]
    async fn roll_random_no_match_returns_none() {
        let pool = test_pool().await;
        seed_three(&pool).await;

        let picked = db_roll_random(&pool, &["NonexistentTag".to_string()]).await.unwrap();
        assert!(picked.is_none());
    }

    #[tokio::test]
    async fn roll_random_empty_table_returns_none() {
        let pool = test_pool().await;
        let picked = db_roll_random(&pool, &[]).await.unwrap();
        assert!(picked.is_none());
    }
```

- [ ] **Step 7: Run tests — confirm all 11 pass**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml advantage
```

Expected: 11 tests pass.

- [ ] **Step 8: Add the 5 `#[tauri::command]` handlers**

Append to `src-tauri/src/db/advantage.rs` (above the `#[cfg(test)]` block):

```rust
// --------------------------------------------------------------------------
// Tauri command handlers (thin wrappers around the helpers above)
// --------------------------------------------------------------------------

#[tauri::command]
pub async fn list_advantages(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<Advantage>, String> {
    db_list(&pool.0).await
}

#[tauri::command]
pub async fn add_advantage(
    pool: tauri::State<'_, crate::DbState>,
    name: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Advantage, String> {
    db_insert(&pool.0, &name, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn update_advantage(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<(), String> {
    db_update(&pool.0, id, &name, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn delete_advantage(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

#[tauri::command]
pub async fn roll_random_advantage(
    pool: tauri::State<'_, crate::DbState>,
    tags: Vec<String>,
) -> Result<Option<Advantage>, String> {
    db_roll_random(&pool.0, &tags).await
}
```

- [ ] **Step 9: Run cargo check and tests**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml && \
  cargo test --manifest-path src-tauri/Cargo.toml advantage
```

Expected: 0 errors, 11 tests pass.

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/db/advantage.rs src-tauri/src/db/mod.rs
git commit -m "$(cat <<'EOF'
feat(advantages): DB helpers + 5 Tauri commands + inline tests

Adds list/add/update/delete/roll_random helpers targeting &SqlitePool
directly (so they're testable without constructing tauri::State), plus
thin #[tauri::command] wrappers. Error strings use the db/advantage.<op>
prefix convention per ARCHITECTURE.md §7. 11 inline unit tests cover
list, insert round-trip, update/delete rejection of is_custom=0 rows,
and the full OR-semantics of roll_random_advantage.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Register commands in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs` (the `invoke_handler` list around lines 48-81)

- [ ] **Step 1: Register all 5 commands**

Modify `src-tauri/src/lib.rs` — locate the `invoke_handler(tauri::generate_handler![…])` block (around line 48) and append these 5 entries at the end of the list (after the last `db::edge::delete_edge,`):

```rust
            db::edge::delete_edge,
            db::advantage::list_advantages,
            db::advantage::add_advantage,
            db::advantage::update_advantage,
            db::advantage::delete_advantage,
            db::advantage::roll_random_advantage,
```

- [ ] **Step 2: Run cargo check**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(advantages): wire 5 commands into invoke_handler

Registers list/add/update/delete/roll_random_advantage so the frontend
can call them via invoke(). No ACL change needed — the existing
core:default + opener:default grant covers new #[tauri::command]
handlers (ARCHITECTURE.md §8).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Seed built-in advantages on startup

Destructive reseed per ADR 0002: every app start, delete all `is_custom = 0` rows from `advantages` and reinsert the canonical V5 set. User-authored rows (`is_custom = 1`) are preserved.

**Files:**
- Modify: `src-tauri/src/db/seed.rs`
- Modify: `src-tauri/src/lib.rs` (the `setup` block around line 33)

- [ ] **Step 1: Add `seed_advantages` to `seed.rs`**

Append to `src-tauri/src/db/seed.rs` (after the `seed_dyscrasias` function):

```rust
use crate::shared::types::{Field, FieldValue, NumberFieldValue};

/// One canonical row shape used during seeding. `level` and `level_max` are
/// optional; when present they become number-typed Fields inside properties_json.
struct SeedRow {
    name: &'static str,
    description: &'static str,
    tags: &'static [&'static str],
    level: Option<i64>,
    level_max: Option<i64>,
    source: &'static str,
}

fn seed_rows() -> &'static [SeedRow] {
    &[
        // -------- Merits (V5 Corebook) --------
        SeedRow {
            name: "Iron Gullet",
            description: "Your character can digest rancid, defiled, or otherwise corrupted blood without issue.",
            tags: &["VTM 5e", "Merit", "Feeding"],
            level: Some(3), level_max: None,
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Eat Food",
            description: "Your character can still consume and enjoy food like a mortal would, though it does not nourish them.",
            tags: &["VTM 5e", "Merit", "Feeding"],
            level: Some(2), level_max: None,
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Bloodhound",
            description: "Your character can sniff out distinct Resonances in a blood vessel simply by being near them.",
            tags: &["VTM 5e", "Merit", "Supernatural"],
            level: Some(1), level_max: None,
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Beautiful",
            description: "Add one die to related Social pools.",
            tags: &["VTM 5e", "Merit", "Social"],
            level: Some(2), level_max: None,
            source: "V5 Corebook",
        },
        // -------- Backgrounds --------
        SeedRow {
            name: "Allies",
            description: "Mortal friends or family who stand with your character, specified at purchase. Rated by Effectiveness (dots) and Reliability (further dots).",
            tags: &["VTM 5e", "Background", "Social"],
            level: Some(1), level_max: Some(5),
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Contacts",
            description: "Mortal sources of information or goods. Rated by usefulness and influence.",
            tags: &["VTM 5e", "Background", "Social"],
            level: Some(1), level_max: Some(5),
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Haven",
            description: "A refuge your character can use as a base. Rated from a squat (1) to a fortress (5).",
            tags: &["VTM 5e", "Background", "Territorial"],
            level: Some(1), level_max: Some(5),
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Resources",
            description: "Financial stability ranging from beggary (1) to millionaire (5). Does not represent liquid cash.",
            tags: &["VTM 5e", "Background", "Material"],
            level: Some(1), level_max: Some(5),
            source: "V5 Corebook",
        },
        // -------- Flaws --------
        SeedRow {
            name: "Prey Exclusion",
            description: "Your character cannot feed from a specific class of mortals (children, the elderly, etc.). Suffer one-point stains if they do.",
            tags: &["VTM 5e", "Flaw", "Feeding"],
            level: Some(1), level_max: None,
            source: "V5 Corebook",
        },
        SeedRow {
            name: "Enemy",
            description: "A mortal or ghoul who actively works against your character. The player and Storyteller define the threat.",
            tags: &["VTM 5e", "Flaw", "Social"],
            level: Some(1), level_max: Some(2),
            source: "V5 Corebook",
        },
    ]
}

fn row_to_properties(row: &SeedRow) -> Vec<Field> {
    let mut props: Vec<Field> = Vec::new();
    if let Some(l) = row.level {
        props.push(Field {
            name: "level".to_string(),
            value: FieldValue::Number { value: NumberFieldValue::Single(l as f64) },
        });
    }
    if let Some(max) = row.level_max {
        let min = row.level.unwrap_or(1) as f64;
        props.push(Field {
            name: "min_level".to_string(),
            value: FieldValue::Number { value: NumberFieldValue::Single(min) },
        });
        props.push(Field {
            name: "max_level".to_string(),
            value: FieldValue::Number { value: NumberFieldValue::Single(max as f64) },
        });
    }
    props.push(Field {
        name: "source".to_string(),
        value: FieldValue::String {
            value: crate::shared::types::StringFieldValue::Single(row.source.to_string()),
        },
    });
    props
}

/// Replaces all built-in Advantage entries with the canonical VTM 5e corebook set.
/// Custom entries (is_custom = 1) are never touched.
pub async fn seed_advantages(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM advantages WHERE is_custom = 0")
        .execute(pool)
        .await?;

    for row in seed_rows() {
        let tags_vec: Vec<String> = row.tags.iter().map(|s| s.to_string()).collect();
        let tags_json = serde_json::to_string(&tags_vec)
            .expect("seed tags must serialize");
        let props = row_to_properties(row);
        let props_json = serde_json::to_string(&props)
            .expect("seed properties must serialize");

        sqlx::query(
            "INSERT INTO advantages (name, description, tags_json, properties_json, is_custom)
             VALUES (?, ?, ?, ?, 0)"
        )
        .bind(row.name)
        .bind(row.description)
        .bind(&tags_json)
        .bind(&props_json)
        .execute(pool)
        .await?;
    }
    Ok(())
}
```

Note: this seed ships 10 representative corebook rows covering Merit, Background, and Flaw categories with both fixed-level and ranged-level examples. The full corebook list is an additive data-entry task for a follow-up commit — add rows by appending to `seed_rows()` with the same shape. User-added entries (`is_custom = 1`) are never affected by this function.

- [ ] **Step 2: Call `seed_advantages` from `lib.rs` setup**

Modify `src-tauri/src/lib.rs` — in the async setup block, add the call immediately after `seed_dyscrasias` (around line 33):

```rust
                db::seed::seed_dyscrasias(&pool).await
                    .expect("Failed to seed dyscrasias");
                db::seed::seed_advantages(&pool).await
                    .expect("Failed to seed advantages");
                handle.manage(DbState(Arc::new(pool)));
```

- [ ] **Step 3: Run cargo check**

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: 0 errors.

- [ ] **Step 4: Run the full verify gate**

Run:
```bash
./scripts/verify.sh
```

Expected: pass (both `cargo test` and `npm run build` complete cleanly; the previously documented "never constructed" warnings on `FieldValue::{Date, Url, Email, Reference}` may still appear per §10).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/seed.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(advantages): destructive reseed with 10 V5 corebook rows

seed_advantages follows ADR 0002: deletes all is_custom=0 rows from
advantages then reinserts the canonical set. Custom rows (is_custom=1)
are preserved across restarts. Initial seed covers Merit, Background,
and Flaw categories with both fixed-level (Iron Gullet) and ranged
(Allies 1–5) examples; remaining corebook entries are additive
follow-up data entry.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Extract `PropertyEditor` + widgets to a shared location

The existing components are already generic (take `Field` / `FieldValue`, don't import `domains/api.ts`) — this is a file move, not a refactor. All internal relative imports (`'../../../types'`, `'../../../../types'`, `'./property-widgets'`) preserve because old and new paths are the same depth. Only the two imports in `NodeForm.svelte` need updating.

**Files:**
- Create (by git-mv): `src/lib/components/properties/PropertyEditor.svelte`
- Create (by git-mv): `src/lib/components/properties/property-widgets/{index.ts, BoolWidget.svelte, NumberWidget.svelte, StringWidget.svelte, TextWidget.svelte}`
- Modify: `src/lib/components/domains/NodeForm.svelte` (lines 6-7)

- [ ] **Step 1: Create the target directory and move the files**

Run:
```bash
mkdir -p src/lib/components/properties
git mv src/lib/components/domains/PropertyEditor.svelte src/lib/components/properties/PropertyEditor.svelte
git mv src/lib/components/domains/property-widgets src/lib/components/properties/property-widgets
```

- [ ] **Step 2: Update `NodeForm.svelte` imports**

Modify `src/lib/components/domains/NodeForm.svelte` — lines 6-7:

Before:
```ts
  import PropertyEditor from './PropertyEditor.svelte';
  import { SUPPORTED_TYPES } from './property-widgets';
```

After:
```ts
  import PropertyEditor from '../properties/PropertyEditor.svelte';
  import { SUPPORTED_TYPES } from '../properties/property-widgets';
```

- [ ] **Step 3: Run type-check to verify nothing else referenced the old paths**

Run:
```bash
npm run check
```

Expected: 0 errors.

If `npm run check` surfaces an import from elsewhere in the codebase pointing at the old path, update it to use the new path. (As of this plan, `NodeForm.svelte` is the only consumer.)

- [ ] **Step 4: Run the full verify gate**

Run:
```bash
./scripts/verify.sh
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/properties src/lib/components/domains/NodeForm.svelte
git commit -m "$(cat <<'EOF'
refactor(properties): move PropertyEditor + widgets out of domains/

PropertyEditor and its widget registry are already tool-agnostic (take
Field/FieldValue, don't import domains/api.ts). Filesystem location
implied domains-only ownership; move to components/properties/ so the
upcoming AdvantagesManager can share the same editor without
cross-tool coupling that violates ARCHITECTURE.md §5.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Typed API wrapper + field presets

**Files:**
- Create: `src/lib/advantages/api.ts`
- Create: `src/lib/advantages/fieldPresets.ts`

- [ ] **Step 1: Create the typed API wrapper**

Create `src/lib/advantages/api.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import type { Advantage, Field } from '../../types';

export type AdvantageInput = {
  name: string;
  description: string;
  tags: string[];
  properties: Field[];
};

export function listAdvantages(): Promise<Advantage[]> {
  return invoke<Advantage[]>('list_advantages');
}

export function addAdvantage(input: AdvantageInput): Promise<Advantage> {
  return invoke<Advantage>('add_advantage', input);
}

export function updateAdvantage(id: number, input: AdvantageInput): Promise<void> {
  return invoke<void>('update_advantage', { id, ...input });
}

export function deleteAdvantage(id: number): Promise<void> {
  return invoke<void>('delete_advantage', { id });
}

export function rollRandomAdvantage(tags: string[]): Promise<Advantage | null> {
  return invoke<Advantage | null>('roll_random_advantage', { tags });
}
```

- [ ] **Step 2: Create the field presets constant**

Create `src/lib/advantages/fieldPresets.ts`:

```ts
import type { FieldValue } from '../../types';

export type FieldPreset = {
  name: string;
  type: FieldValue['type'];
  defaultValue: string | number | boolean;
  hint: string;
};

/**
 * Quick-add chips surfaced in AdvantageForm's properties section.
 * Clicking one appends a new Field with this name/type/defaultValue.
 * A preset chip is disabled when a field with the same name already
 * exists on the row (name uniqueness is enforced in the form).
 */
export const FIELD_PRESETS: FieldPreset[] = [
  { name: 'level',     type: 'number', defaultValue: 1,  hint: 'Fixed dot cost' },
  { name: 'min_level', type: 'number', defaultValue: 1,  hint: 'Minimum dots (for ranged merits)' },
  { name: 'max_level', type: 'number', defaultValue: 5,  hint: 'Maximum dots (for ranged merits)' },
  { name: 'source',    type: 'string', defaultValue: '', hint: 'Sourcebook reference' },
  { name: 'prereq',    type: 'text',   defaultValue: '', hint: 'Prerequisite text' },
];
```

- [ ] **Step 3: Run type-check**

Run:
```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/advantages/api.ts src/lib/advantages/fieldPresets.ts
git commit -m "$(cat <<'EOF'
feat(advantages): typed IPC wrapper + field-preset constant

api.ts exports one function per Tauri command per ARCHITECTURE.md §4 —
components never call invoke() directly. fieldPresets.ts is the hardcoded
TS constant driving the form's quick-add chips (level / min_level /
max_level / source / prereq). DB-backed preset management is a future
seam; v1 ships presets in code.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: `AdvantageCard.svelte` component

**Files:**
- Create: `src/lib/components/AdvantageCard.svelte`

- [ ] **Step 1: Create the card component**

Create `src/lib/components/AdvantageCard.svelte`:

```svelte
<script lang="ts">
  import type { Advantage, Field } from '../../types';
  import { fade } from 'svelte/transition';

  const { entry, onedit, ondelete }: {
    entry: Advantage;
    onedit?: () => void;
    ondelete?: () => void;
  } = $props();

  function findField(name: string): Field | undefined {
    return entry.properties.find(p => p.name === name);
  }

  function numberValue(f: Field | undefined): number | null {
    if (!f) return null;
    if (f.type !== 'number') return null;
    return Array.isArray(f.value) ? f.value[0] ?? null : f.value;
  }

  const level      = $derived(numberValue(findField('level')));
  const minLevel   = $derived(numberValue(findField('min_level')));
  const maxLevel   = $derived(numberValue(findField('max_level')));
  const dotCeiling = 5;

  // Other properties — everything except the level/min/max triple, rendered as key: value.
  const otherProps = $derived(
    entry.properties.filter(p => !['level', 'min_level', 'max_level'].includes(p.name))
  );

  function displayValue(f: Field): string {
    switch (f.type) {
      case 'string':  return Array.isArray(f.value) ? f.value.join(', ') : String(f.value);
      case 'text':    return f.value;
      case 'number':  return Array.isArray(f.value) ? f.value.join(', ') : String(f.value);
      case 'bool':    return f.value ? 'yes' : 'no';
      case 'date':
      case 'url':
      case 'email':   return f.value;
      case 'reference': return `#${f.value}`;
      default:        return '';
    }
  }
</script>

<article class="card" transition:fade={{ duration: 120 }}>
  <header class="head">
    <h3 class="name">{entry.name}</h3>
    {#if entry.tags.length > 0}
      <div class="tags">
        {#each entry.tags as t}
          <span class="tag">{t}</span>
        {/each}
      </div>
    {/if}
  </header>

  {#if entry.description}
    <p class="desc">{entry.description}</p>
  {/if}

  {#if level !== null || (minLevel !== null && maxLevel !== null)}
    <div class="dots" aria-label="dot cost">
      {#if level !== null}
        {#each Array(dotCeiling) as _, i}
          <span class:filled={i < level}>●</span>
        {/each}
      {:else if minLevel !== null && maxLevel !== null}
        <span class="range">{minLevel}–{maxLevel} dots</span>
      {/if}
    </div>
  {/if}

  {#if otherProps.length > 0}
    <ul class="props">
      {#each otherProps as p}
        <li><span class="k">{p.name}:</span> {displayValue(p)}</li>
      {/each}
    </ul>
  {/if}

  <footer class="foot">
    {#if entry.isCustom}
      <button class="btn edit"   onclick={onedit}   aria-label="Edit">✎</button>
      <button class="btn delete" onclick={ondelete} aria-label="Delete">✕</button>
    {:else}
      <span class="builtin">built-in</span>
    {/if}
  </footer>
</article>

<style>
  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 6px;
    padding: 0.65rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    box-sizing: border-box;
  }
  .head { display: flex; flex-direction: column; gap: 0.3rem; }
  .name { font-size: 0.92rem; color: var(--text-primary); margin: 0; }
  .tags { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .tag {
    background: var(--bg-sunken);
    color: var(--text-muted);
    border-radius: 10px;
    padding: 0.06rem 0.45rem;
    font-size: 0.62rem;
  }
  .desc { color: var(--text-secondary); font-size: 0.74rem; margin: 0; line-height: 1.4; }
  .dots { color: var(--text-ghost); letter-spacing: 0.08em; font-size: 0.85rem; }
  .dots .filled { color: var(--accent); }
  .dots .range  { font-size: 0.72rem; font-style: italic; }
  .props { list-style: none; margin: 0; padding: 0; font-size: 0.7rem; color: var(--text-muted); }
  .props li { padding: 0.08rem 0; }
  .props .k { color: var(--text-label); }
  .foot { display: flex; justify-content: flex-end; gap: 0.3rem; }
  .btn {
    background: none;
    border: 1px solid var(--border-faint);
    color: var(--text-ghost);
    border-radius: 3px;
    padding: 0.1rem 0.45rem;
    font-size: 0.68rem;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .btn:hover { color: var(--accent); border-color: var(--accent); }
  .builtin { color: var(--text-ghost); font-size: 0.62rem; font-style: italic; }
</style>
```

- [ ] **Step 2: Run type-check**

Run:
```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
feat(advantages): AdvantageCard component

Renders one library entry: name, tag chips, description, dot strip
(level) or range label (min_level–max_level), and a key:value list for
remaining properties. Built-in rows show 'built-in' badge; custom rows
expose Edit/Delete buttons. Uses layout tokens from :root per §6.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: `AdvantageForm.svelte` component

**Files:**
- Create: `src/lib/components/AdvantageForm.svelte`

- [ ] **Step 1: Create the form component**

Create `src/lib/components/AdvantageForm.svelte`:

```svelte
<script lang="ts">
  import type { Advantage, Field, FieldValue } from '../../types';
  import { addAdvantage, updateAdvantage, type AdvantageInput } from '$lib/advantages/api';
  import { FIELD_PRESETS, type FieldPreset } from '$lib/advantages/fieldPresets';
  import PropertyEditor from '$lib/components/properties/PropertyEditor.svelte';
  import { SUPPORTED_TYPES } from '$lib/components/properties/property-widgets';

  const { entry, oncancel, onsave }: {
    entry?: Advantage;
    oncancel?: () => void;
    onsave?: () => void;
  } = $props();

  let name        = $state(entry?.name ?? '');
  let description = $state(entry?.description ?? '');
  let tags: string[]          = $state(entry ? [...entry.tags]       : []);
  let properties: Field[]     = $state(entry ? [...entry.properties] : []);
  let tagDraft    = $state('');
  let saveError   = $state('');
  let saving      = $state(false);

  const trimmedTags         = $derived(tags.map(t => t.trim()).filter(t => t.length > 0));
  const tagsUnique          = $derived(new Set(trimmedTags).size === trimmedTags.length);
  const propertyNames       = $derived(properties.map(p => p.name.trim()));
  const propertiesUnique    = $derived(new Set(propertyNames).size === propertyNames.length);
  const propertiesNonEmpty  = $derived(propertyNames.every(n => n.length > 0));

  const valid = $derived(
    name.trim().length > 0 && tagsUnique && propertiesUnique && propertiesNonEmpty
  );

  function addTag() {
    const v = tagDraft.trim();
    if (!v) return;
    if (tags.includes(v)) { tagDraft = ''; return; }
    tags = [...tags, v];
    tagDraft = '';
  }

  function removeTag(i: number) {
    tags = tags.filter((_, idx) => idx !== i);
  }

  function onTagKey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); addTag(); }
  }

  function applyPreset(preset: FieldPreset) {
    if (properties.some(p => p.name === preset.name)) return;
    const newField = buildPresetField(preset);
    properties = [...properties, newField];
  }

  function buildPresetField(preset: FieldPreset): Field {
    switch (preset.type) {
      case 'number':
        return { name: preset.name, type: 'number', value: Number(preset.defaultValue) };
      case 'bool':
        return { name: preset.name, type: 'bool',   value: Boolean(preset.defaultValue) };
      case 'text':
        return { name: preset.name, type: 'text',   value: String(preset.defaultValue) };
      case 'string':
      default:
        return { name: preset.name, type: 'string', value: String(preset.defaultValue) };
    }
  }

  function addCustomProperty() {
    properties = [...properties, { name: '', type: 'string', value: '' } as Field];
  }

  function renameProperty(i: number, newName: string) {
    properties = properties.map((p, idx) => idx === i ? { ...p, name: newName } : p);
  }

  function retypeProperty(i: number, newType: FieldValue['type']) {
    const prev = properties[i];
    const blank = buildPresetField({ name: prev.name, type: newType, defaultValue: '', hint: '' });
    properties = properties.map((p, idx) => idx === i ? blank : p);
  }

  function updateProperty(i: number, updated: Field) {
    properties = properties.map((p, idx) => idx === i ? updated : p);
  }

  function removeProperty(i: number) {
    properties = properties.filter((_, idx) => idx !== i);
  }

  async function handleSave() {
    if (!valid) return;
    saving = true;
    saveError = '';
    const input: AdvantageInput = {
      name: name.trim(),
      description,
      tags: trimmedTags,
      properties: properties.map(p => ({ ...p, name: p.name.trim() })),
    };
    try {
      if (entry) {
        await updateAdvantage(entry.id, input);
      } else {
        await addAdvantage(input);
      }
      onsave?.();
    } catch (e) {
      saveError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<form class="form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
  <section class="section">
    <label class="label" for="adv-name">Name</label>
    <input id="adv-name" class="input" bind:value={name} />

    <label class="label" for="adv-desc">Description</label>
    <textarea id="adv-desc" class="textarea" bind:value={description}></textarea>
  </section>

  <section class="section">
    <div class="label">Tags</div>
    <div class="tag-row">
      {#each tags as t, i}
        <span class="tag">
          {t}
          <button type="button" class="tag-x" onclick={() => removeTag(i)} aria-label="Remove tag">×</button>
        </span>
      {/each}
      <input
        class="tag-input"
        bind:value={tagDraft}
        onkeydown={onTagKey}
        placeholder="Add tag (Enter)"
      />
    </div>
    {#if !tagsUnique}
      <p class="validation">Tags must be unique.</p>
    {/if}
  </section>

  <section class="section">
    <div class="label">Properties</div>
    <div class="preset-row">
      {#each FIELD_PRESETS as preset}
        {@const disabled = properties.some(p => p.name === preset.name)}
        <button
          type="button"
          class="preset"
          {disabled}
          title={preset.hint}
          onclick={() => applyPreset(preset)}
        >+ {preset.name}</button>
      {/each}
      <button type="button" class="preset" onclick={addCustomProperty}>+ Custom…</button>
    </div>

    <ul class="prop-list">
      {#each properties as prop, i (i)}
        <li class="prop-row">
          <input
            class="prop-name"
            value={prop.name}
            placeholder="field name"
            oninput={(e) => renameProperty(i, (e.target as HTMLInputElement).value)}
          />
          <select
            class="prop-type"
            value={prop.type}
            onchange={(e) => retypeProperty(i, (e.target as HTMLSelectElement).value as FieldValue['type'])}
          >
            {#each SUPPORTED_TYPES as t}
              <option value={t}>{t}</option>
            {/each}
          </select>
          <div class="prop-widget">
            <PropertyEditor
              field={prop}
              readonly={false}
              onchange={(u) => updateProperty(i, u)}
              onremove={() => removeProperty(i)}
            />
          </div>
        </li>
      {/each}
    </ul>

    {#if !propertiesUnique}
      <p class="validation">Property names must be unique.</p>
    {/if}
    {#if !propertiesNonEmpty}
      <p class="validation">Every property needs a name.</p>
    {/if}
  </section>

  {#if saveError}
    <p class="error">{saveError}</p>
  {/if}

  <div class="footer">
    <button type="button" class="btn" onclick={oncancel}>Cancel</button>
    <button type="submit"  class="btn primary" disabled={!valid || saving}>
      {saving ? 'Saving…' : (entry ? 'Save' : 'Add')}
    </button>
  </div>
</form>

<style>
  .form {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 6px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-sizing: border-box;
  }
  .section  { display: flex; flex-direction: column; gap: 0.35rem; }
  .label    { color: var(--text-label); font-size: 0.7rem; }
  .input, .textarea, .tag-input, .prop-name, .prop-type {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.32rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
    box-sizing: border-box;
  }
  .textarea { min-height: 4rem; resize: vertical; }
  .tag-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem;
    box-sizing: border-box;
  }
  .tag-row .tag-input {
    flex: 1;
    min-width: 6rem;
    border: none;
    padding: 0.2rem;
    background: transparent;
  }
  .tag {
    background: var(--bg-sunken);
    color: var(--text-secondary);
    border-radius: 10px;
    padding: 0.1rem 0.4rem 0.1rem 0.5rem;
    font-size: 0.66rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .tag-x {
    background: none;
    border: none;
    color: var(--text-ghost);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0;
  }
  .tag-x:hover { color: var(--accent); }
  .preset-row { display: flex; flex-wrap: wrap; gap: 0.3rem; }
  .preset {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.2rem 0.55rem;
    font-size: 0.68rem;
    cursor: pointer;
  }
  .preset:hover:not([disabled]) { color: var(--accent); border-color: var(--accent); }
  .preset[disabled] { opacity: 0.45; cursor: not-allowed; }
  .prop-list { list-style: none; padding: 0; margin: 0.3rem 0 0 0; display: flex; flex-direction: column; gap: 0.35rem; }
  .prop-row {
    display: grid;
    grid-template-columns: 7rem 5.5rem 1fr;
    gap: 0.35rem;
    align-items: start;
  }
  .prop-widget { min-width: 0; }
  .validation { color: var(--accent); font-size: 0.68rem; margin: 0.2rem 0 0 0; }
  .error      { color: var(--accent); font-size: 0.72rem; margin: 0; }
  .footer     { display: flex; justify-content: flex-end; gap: 0.4rem; }
  .btn {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.7rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .btn.primary { background: var(--bg-active); border-color: var(--border-active); color: var(--accent); }
  .btn[disabled] { opacity: 0.45; cursor: not-allowed; }
</style>
```

- [ ] **Step 2: Run type-check**

Run:
```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/AdvantageForm.svelte
git commit -m "$(cat <<'EOF'
feat(advantages): AdvantageForm component

Three-section editor: Basics (name, description), Tags (chip editor),
Properties (field-preset row + per-field name/type/widget via the
shared PropertyEditor). Validates name non-empty, tag uniqueness,
property-name uniqueness + non-empty. Uses the typed api.ts wrapper
per ARCHITECTURE.md §4 — no invoke() calls from components.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: `AdvantagesManager.svelte` tool

**Files:**
- Create: `src/tools/AdvantagesManager.svelte`

- [ ] **Step 1: Create the tool component**

Create `src/tools/AdvantagesManager.svelte`:

```svelte
<script lang="ts">
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import AdvantageCard from '$lib/components/AdvantageCard.svelte';
  import AdvantageForm from '$lib/components/AdvantageForm.svelte';
  import { listAdvantages, deleteAdvantage } from '$lib/advantages/api';
  import type { Advantage, Field } from '../types';

  type SortKey = 'name-asc' | 'name-desc' | 'level-asc' | 'level-desc' | 'recent';

  let allEntries: Advantage[] = $state([]);
  let loading       = $state(true);
  let loadError     = $state('');
  let activeTags: Set<string> = $state(new Set(['__all__']));
  let rawSearch     = $state('');
  let searchQuery   = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let sortKey: SortKey = $state('name-asc');
  let showAddForm   = $state(false);
  let editingId: number | null = $state(null);

  // ---- helpers --------------------------------------------------------------

  function findField(adv: Advantage, name: string): Field | undefined {
    return adv.properties.find(p => p.name === name);
  }

  function levelNumber(adv: Advantage): number | null {
    const levelField = findField(adv, 'level');
    if (levelField && levelField.type === 'number') {
      return Array.isArray(levelField.value) ? levelField.value[0] ?? null : levelField.value;
    }
    const minField = findField(adv, 'min_level');
    if (minField && minField.type === 'number') {
      return Array.isArray(minField.value) ? minField.value[0] ?? null : minField.value;
    }
    return null;
  }

  function matchesTags(adv: Advantage): boolean {
    if (activeTags.has('__all__')) return true;
    return adv.tags.some(t => activeTags.has(t));
  }

  function matchesQuery(adv: Advantage): boolean {
    const q = searchQuery.toLowerCase();
    if (!q) return true;
    if (adv.name.toLowerCase().includes(q)) return true;
    if (adv.description.toLowerCase().includes(q)) return true;
    if (adv.tags.some(t => t.toLowerCase().includes(q))) return true;
    return false;
  }

  function sortRows(rows: Advantage[]): Advantage[] {
    const copy = [...rows];
    switch (sortKey) {
      case 'name-asc':  return copy.sort((a, b) => a.name.localeCompare(b.name));
      case 'name-desc': return copy.sort((a, b) => b.name.localeCompare(a.name));
      case 'recent':    return copy.sort((a, b) => b.id - a.id);
      case 'level-asc':
      case 'level-desc': {
        const dir = sortKey === 'level-asc' ? 1 : -1;
        return copy.sort((a, b) => {
          const la = levelNumber(a);
          const lb = levelNumber(b);
          if (la === null && lb === null) return 0;
          if (la === null) return 1;   // missing-level rows always last
          if (lb === null) return -1;
          return dir * (la - lb);
        });
      }
    }
  }

  // ---- derived state --------------------------------------------------------

  const distinctTags = $derived(
    [...new Set(allEntries.flatMap(e => e.tags))].sort((a, b) => a.localeCompare(b))
  );

  const visible = $derived(
    sortRows(allEntries.filter(e => matchesTags(e) && matchesQuery(e)))
  );

  // ---- actions --------------------------------------------------------------

  async function loadAll() {
    loading = true;
    loadError = '';
    try {
      allEntries = await listAdvantages();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  }

  function toggleTag(tag: string) {
    if (tag === '__all__') {
      activeTags = new Set(['__all__']);
      return;
    }
    const next = new Set(activeTags);
    next.delete('__all__');
    if (next.has(tag)) next.delete(tag);
    else next.add(tag);
    if (next.size === 0) next.add('__all__');
    activeTags = next;
  }

  function onSearchInput(e: Event) {
    rawSearch = (e.target as HTMLInputElement).value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = rawSearch; }, 110);
  }

  function handleSave() {
    showAddForm = false;
    editingId = null;
    loadAll();
  }

  async function handleDelete(id: number) {
    try {
      await deleteAdvantage(id);
      loadAll();
    } catch (e) {
      loadError = String(e);
    }
  }

  $effect(() => { untrack(() => loadAll()); });
  $effect(() => { return () => { if (searchTimer) clearTimeout(searchTimer); }; });
</script>

<div class="page">
  <h1 class="title">Advantages</h1>

  <div class="controls">
    <input
      class="search"
      type="text"
      value={rawSearch}
      oninput={onSearchInput}
      placeholder="Search by name, description, or tag…"
    />
    <select class="sort" bind:value={sortKey}>
      <option value="name-asc">Name A–Z</option>
      <option value="name-desc">Name Z–A</option>
      <option value="level-asc">Level ↑</option>
      <option value="level-desc">Level ↓</option>
      <option value="recent">Recently added</option>
    </select>
    <button class="add-btn" onclick={() => { showAddForm = !showAddForm; editingId = null; }}>
      {showAddForm ? '✕ Cancel' : '+ Add Custom'}
    </button>
  </div>

  <div class="chips">
    <button
      class="chip"
      class:active={activeTags.has('__all__')}
      onclick={() => toggleTag('__all__')}
    >All</button>
    {#each distinctTags as tag}
      <button
        class="chip"
        class:active={activeTags.has(tag)}
        onclick={() => toggleTag(tag)}
      >{tag}</button>
    {/each}
  </div>

  <p class="results-label">Showing {visible.length} advantages</p>

  {#if loading}
    <p class="loading-text">Loading…</p>
  {:else if loadError}
    <p class="error-text">{loadError}</p>
  {:else}
    <div class="grid">
      {#if showAddForm}
        <div
          in:scale={{ start: 0.9, duration: 200, easing: cubicOut }}
          out:fade={{ duration: 150 }}
        >
          <AdvantageForm
            oncancel={() => { showAddForm = false; }}
            onsave={handleSave}
          />
        </div>
      {/if}

      {#each visible as entry (entry.id)}
        <div animate:flip={{ duration: 300, easing: cubicOut }}>
          {#if editingId === entry.id}
            <div
              in:scale={{ start: 0.9, duration: 200, easing: cubicOut }}
              out:fade={{ duration: 150 }}
            >
              <AdvantageForm
                {entry}
                oncancel={() => { editingId = null; }}
                onsave={handleSave}
              />
            </div>
          {:else}
            <AdvantageCard
              {entry}
              onedit={() => { editingId = entry.id; showAddForm = false; }}
              ondelete={() => handleDelete(entry.id)}
            />
          {/if}
        </div>
      {/each}

      {#if visible.length === 0 && !showAddForm}
        <p class="empty" transition:fade>No advantages match your filters.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page   { padding: 1rem 1.25rem; }
  .title  { color: var(--accent); font-size: 1.4rem; margin-bottom: 1rem; }

  .controls { display: flex; gap: 0.6rem; margin-bottom: 0.75rem; align-items: center; }
  .search {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 5px;
    padding: 0.5rem 0.75rem;
    color: var(--text-primary);
    font-size: 0.82rem;
    outline: none;
    box-sizing: border-box;
  }
  .search:focus { border-color: var(--accent); }
  .sort {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 5px;
    padding: 0.45rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
  }
  .add-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 5px;
    padding: 0.5rem 0.9rem;
    font-size: 0.78rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .chips { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-bottom: 0.75rem; }
  .chip {
    padding: 0.28rem 0.7rem;
    border-radius: 20px;
    font-size: 0.72rem;
    border: 1px solid var(--border-card);
    color: var(--text-label);
    background: var(--bg-card);
    cursor: pointer;
  }
  .chip:hover  { border-color: var(--border-surface); color: var(--text-primary); }
  .chip.active { border-color: var(--text-label); color: var(--text-primary); background: var(--bg-raised); }

  .results-label { font-size: 0.68rem; color: var(--text-ghost); margin-bottom: 0.75rem; }
  .loading-text  { color: var(--text-ghost); font-size: 0.8rem; }
  .error-text    { color: var(--accent); font-size: 0.8rem; padding: 1rem 0; }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(12.5rem, 1fr));
    gap: 0.75rem;
    align-items: start;
  }
  .empty { color: var(--text-ghost); font-size: 0.8rem; text-align: center; padding: 2rem; grid-column: 1 / -1; }
</style>
```

- [ ] **Step 2: Run type-check**

Run:
```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/tools/AdvantagesManager.svelte
git commit -m "$(cat <<'EOF'
feat(advantages): AdvantagesManager top-level tool

Search (debounced 110ms), dynamic chip filter derived from DISTINCT
tags in the data (multi-select OR semantics), sort dropdown (Name A–Z
/ Z–A, Level ↑/↓ with min_level fallback for ranged merits, Recently
added), CSS Grid with align-items: start per §6, and add/edit toggling
identical to DyscrasiaManager. All IPC flows through the typed
advantages/api.ts wrapper.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 10: Tool registry entry

**Files:**
- Modify: `src/tools.ts`

- [ ] **Step 1: Register the tool in the sidebar**

Modify `src/tools.ts` — append an entry to the `tools` array (after the `domains` entry):

```ts
  {
    id: 'domains',
    label: 'Domains',
    icon: '🏰',
    component: () => import('./tools/DomainsManager.svelte'),
  },
  {
    id: 'advantages',
    label: 'Advantages',
    icon: '⚜',
    component: () => import('./tools/AdvantagesManager.svelte'),
  },
];
```

- [ ] **Step 2: Run type-check**

Run:
```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 3: Run the full verify gate**

Run:
```bash
./scripts/verify.sh
```

Expected: pass.

- [ ] **Step 4: Commit**

```bash
git add src/tools.ts
git commit -m "$(cat <<'EOF'
feat(advantages): wire AdvantagesManager into the sidebar

Adds one entry to src/tools.ts per ARCHITECTURE.md §9 "Add a tool" —
lazy-loaded component + sidebar wiring is automatic.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 11: End-to-end verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run the full verify gate from a clean state**

Run:
```bash
./scripts/verify.sh
```

Expected: pass. Expected non-regression warnings per ARCHITECTURE.md §10 may appear:
- `npm run build`: unused `listen` import in `Campaign.svelte` and `Resonance.svelte` — pre-existing, do not touch.
- `shared/types.rs`: `FieldValue::{Date, Url, Email, Reference}` "never constructed" — pre-existing, do not touch.

No new warnings should appear. If something new surfaces, investigate — do not suppress.

- [ ] **Step 2: Manual smoke test in the Tauri dev window**

Run:
```bash
npm run tauri dev
```

Expected interactive checks (see ARCHITECTURE.md §10 — no frontend test framework, so manual verification is the norm for UI):
- Sidebar lists "Advantages" with the `⚜` icon.
- Opening the tool shows the 10 seeded rows (Iron Gullet, Eat Food, Bloodhound, Beautiful, Allies, Contacts, Haven, Resources, Prey Exclusion, Enemy).
- Tag chips include at least `VTM 5e`, `Merit`, `Background`, `Flaw`, plus subcategories. Clicking a chip narrows the grid.
- Sort dropdown "Level ↑" places 1-dot merits first and ranged rows by their min_level; rows without any level property sort to the end.
- "+ Add Custom" opens the form. Adding a row with name "Test Merit", tag "Custom", and a `level: 2` preset field persists to the DB — close and re-open the tool, it remains.
- Deleting the custom row removes it; deleting a built-in row is not exposed (buttons absent for `isCustom: false`).
- Restarting the app preserves the custom row and still shows the 10 built-ins.

If any of these fail, roll back to the last passing task and debug from there.

---

## Self-review

**Spec coverage:**
- §Context / Goals — delivered by Tasks 1–10.
- §Non-goals — plan does not add Roll20 integration (§Task 3 commit message explicitly notes no ACL change); no character-attachment wiring; no DB-backed preset management (Task 6 ships `fieldPresets.ts` as a TS constant).
- §Domain model — Task 1 step 2 + step 3.
- §Schema — Task 1 step 1.
- §Tauri commands — Task 2 step 8 (all 5) + Task 3 (registration).
- §Frontend API wrapper — Task 6 step 1.
- §Seed policy — Task 4 steps 1–2.
- §Field presets — Task 6 step 2.
- §Frontend surfaces — Task 7 (Card), Task 8 (Form), Task 9 (Manager), Task 10 (registry).
- §Error handling — Task 2 (`db/advantage.<op>:` prefix in `db_insert` / `db_update` / `db_delete`).
- §Testing — Task 2 (11 inline tests); Task 11 (verify.sh + smoke test).
- §Forward-compat — no direct task; the schema/API choices in Tasks 1 + 6 already satisfy the requirement (Advantage.properties is `Field[]`, same shape as `nodes.properties_json`).
- §Capability / ACL impact — Task 3 commit message notes no change required.
- §Freeform tags / ADR 0003 — Task 2 (no enum column for category) + Task 9 (dynamic chip derivation).

**Placeholder scan:** no TBDs / "add appropriate X" / "similar to Task N" / references to undefined functions. The seed's partial-corebook coverage (10 of ~40 entries) is explicitly called out in Task 4 as additive follow-up data entry, not a placeholder.

**Type consistency:**
- `Advantage` struct fields (Rust) and TS interface fields match: `id / name / description / tags / properties / isCustom`.
- API wrapper input type `AdvantageInput` omits `id / isCustom`; `updateAdvantage(id, input)` shape matches the Rust command signature (`id` + `name/description/tags/properties`).
- `FIELD_PRESETS` `type` field uses `FieldValue['type']` — aligns with `SUPPORTED_TYPES` in the widget registry.
- `buildPresetField` handles all four `SUPPORTED_TYPES` variants (`string`, `text`, `number`, `bool`); the form's type dropdown is populated from the same source, so users can't pick an unsupported variant.
- All command names in `invoke_handler!` (Task 3) match those the TS wrapper uses (Task 6): `list_advantages`, `add_advantage`, `update_advantage`, `delete_advantage`, `roll_random_advantage`.

Plan is self-consistent and plan-ready.
