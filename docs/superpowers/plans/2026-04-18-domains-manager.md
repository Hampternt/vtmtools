# Domains Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the backend for the Domains Manager tool — a hierarchical chronicle tracker modeling a VTM game as a graph of typed nodes connected by typed edges — as SQLite tables, Rust types, Tauri commands, and in-module unit tests. No UI in this plan; UI is a future phase.

**Architecture:** Three new SQLite tables (`chronicles`, `nodes`, `edges`) added in migration `0002_chronicle_graph.sql`. Rust types in `shared/types.rs` with serde `tag = "type"` on a `FieldValue` enum for automatic type validation of user-defined custom fields. CRUD and derived-query logic in `db/chronicle.rs`, `db/node.rs`, `db/edge.rs`, each mirroring the single-file pattern used by `db/dyscrasia.rs`. Twenty Tauri commands registered in `lib.rs`. `PRAGMA foreign_keys = ON` enabled via `SqliteConnectOptions` to activate `ON DELETE CASCADE`.

**Tech Stack:** Rust (`sqlx 0.7` with SQLite + migrate features, `serde`, `serde_json`, `tokio`), Tauri 2, TypeScript (for `src/types.ts` mirrors). In-memory SQLite for unit tests. No new dependencies — all required crates are already in `Cargo.toml`.

---

## File Map

**Create:**
- `src-tauri/migrations/0002_chronicle_graph.sql` — schema for `chronicles`, `nodes`, `edges` plus indexes, triggers, and partial unique index for single-parent-under-contains
- `src-tauri/src/db/chronicle.rs` — chronicles CRUD (`db_*` helpers + 5 Tauri commands + unit tests)
- `src-tauri/src/db/node.rs` — nodes CRUD + 5 derived tree queries + cycle-check helper (10 Tauri commands + unit tests)
- `src-tauri/src/db/edge.rs` — edges CRUD + cycle-check integration (5 Tauri commands + unit tests)

**Modify:**
- `src-tauri/src/lib.rs` — switch pool to `SqliteConnectOptions` with `foreign_keys(true)`; register 20 new commands
- `src-tauri/src/db/mod.rs` — add `pub mod chronicle; pub mod node; pub mod edge;`
- `src-tauri/src/shared/types.rs` — append `Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`
- `src/types.ts` — append `Chronicle`, `ChronicleNode`, `ChronicleEdge`, `Field`, `FieldValue`, `EdgeDirection`
- `docs/design/data-sources.md` — add "Chronicle Store" section
- `docs/design/data-sources-kumu.json` — add two elements + two connections
- `CLAUDE.md` — append schema paragraph to `### Database`

---

## Task 1: Write the migration file

**Files:**
- Create: `src-tauri/migrations/0002_chronicle_graph.sql`

- [ ] **Step 1: Create the migration file**

Create `src-tauri/migrations/0002_chronicle_graph.sql` with this exact content:

```sql
-- Chronicles: one per running game. Deleting a chronicle cascades to its nodes and edges.
CREATE TABLE IF NOT EXISTS chronicles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    description  TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Nodes: any discrete thing in a chronicle (area, character, institution, business, merit).
-- `type` is a freeform user-chosen string. `tags_json` is a JSON array of strings.
-- `properties_json` is a JSON array of typed Field records.
CREATE TABLE IF NOT EXISTS nodes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chronicle_id    INTEGER NOT NULL REFERENCES chronicles(id) ON DELETE CASCADE,
    type            TEXT    NOT NULL,
    label           TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Edges: typed directional relationships between nodes.
-- `"contains"` is the UI's drilldown convention but the DB imposes no special meaning on it.
CREATE TABLE IF NOT EXISTS edges (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    chronicle_id    INTEGER NOT NULL REFERENCES chronicles(id) ON DELETE CASCADE,
    from_node_id    INTEGER NOT NULL REFERENCES nodes(id)      ON DELETE CASCADE,
    to_node_id      INTEGER NOT NULL REFERENCES nodes(id)      ON DELETE CASCADE,
    edge_type       TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now')),

    CHECK (from_node_id != to_node_id),
    UNIQUE (from_node_id, to_node_id, edge_type)
);

-- Indexes on common query paths.
CREATE INDEX IF NOT EXISTS idx_nodes_chronicle  ON nodes(chronicle_id);
CREATE INDEX IF NOT EXISTS idx_nodes_type       ON nodes(chronicle_id, type);
CREATE INDEX IF NOT EXISTS idx_edges_chronicle  ON edges(chronicle_id);
CREATE INDEX IF NOT EXISTS idx_edges_from       ON edges(from_node_id, edge_type);
CREATE INDEX IF NOT EXISTS idx_edges_to         ON edges(to_node_id,   edge_type);

-- Enforce "strict tree under contains": a node may have at most one contains-parent.
-- Other edge types have no such restriction.
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_contains_single_parent
    ON edges(to_node_id) WHERE edge_type = 'contains';

-- SQLite has no ON UPDATE CURRENT_TIMESTAMP, so maintain updated_at via triggers.
CREATE TRIGGER IF NOT EXISTS trg_chronicles_updated
    AFTER UPDATE ON chronicles FOR EACH ROW
    BEGIN UPDATE chronicles SET updated_at = datetime('now') WHERE id = NEW.id; END;

CREATE TRIGGER IF NOT EXISTS trg_nodes_updated
    AFTER UPDATE ON nodes FOR EACH ROW
    BEGIN UPDATE nodes SET updated_at = datetime('now') WHERE id = NEW.id; END;

CREATE TRIGGER IF NOT EXISTS trg_edges_updated
    AFTER UPDATE ON edges FOR EACH ROW
    BEGIN UPDATE edges SET updated_at = datetime('now') WHERE id = NEW.id; END;
```

- [ ] **Step 2: Verify SQL syntax against a scratch in-memory database**

Run:

```bash
sqlite3 :memory: < src-tauri/migrations/0002_chronicle_graph.sql && echo OK
```

Expected: prints `OK`. Any error output means a syntax problem — fix and re-run.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/0002_chronicle_graph.sql
git commit -m "feat(db): add chronicle_graph migration (chronicles, nodes, edges)"
```

---

## Task 2: Enable PRAGMA foreign_keys on the SqlitePool

**Files:**
- Modify: `src-tauri/src/lib.rs`

Without this change, SQLite ignores foreign-key constraints, which means `ON DELETE CASCADE` won't fire and deleting a chronicle would leave orphaned nodes and edges.

- [ ] **Step 1: Import SqliteConnectOptions and FromStr**

Open `src-tauri/src/lib.rs`. At the top, replace the line:

```rust
use sqlx::SqlitePool;
```

with:

```rust
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::str::FromStr;
```

- [ ] **Step 2: Replace pool construction**

Inside the `tauri::async_runtime::block_on(async move { ... })` block, find the line:

```rust
let pool = SqlitePool::connect(&db_url).await
    .expect("Failed to connect to database");
```

Replace it with:

```rust
let opts = SqliteConnectOptions::from_str(&db_url)
    .expect("Invalid db_url")
    .foreign_keys(true);
let pool = SqlitePool::connect_with(opts).await
    .expect("Failed to connect to database");
```

- [ ] **Step 3: Compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: clean compile (zero warnings, zero errors, or only pre-existing warnings unrelated to this change).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(db): enable foreign_keys PRAGMA on SqlitePool for cascade support"
```

---

## Task 3: Add Rust shared types

**Files:**
- Modify: `src-tauri/src/shared/types.rs`

- [ ] **Step 1: Append the new types to shared/types.rs**

Open `src-tauri/src/shared/types.rs`. Append the following at the end of the file (leave existing content untouched):

```rust
// ---------------------------------------------------------------------------
// Domains Manager / Chronicle graph types
// ---------------------------------------------------------------------------

/// A running game. Contains nodes and edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chronicle {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Single-or-multi string value. Serialized untagged: a raw string for single,
/// an array for multi.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringFieldValue {
    Single(String),
    Multi(Vec<String>),
}

/// Single-or-multi number value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum NumberFieldValue {
    Single(f64),
    Multi(Vec<f64>),
}

/// A typed field value. `#[serde(tag = "type")]` means the JSON discriminator field
/// `"type"` chooses which variant is parsed; a value of the wrong type fails to
/// deserialize — no manual validation code needed.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FieldValue {
    String    { value: StringFieldValue },
    Text      { value: String           },
    Number    { value: NumberFieldValue },
    Date      { value: String           },
    Url       { value: String           },
    Email     { value: String           },
    Bool      { value: bool             },
    Reference { value: i64              },
}

/// A named, typed field. JSON shape example:
///   {"name": "influence_rating", "type": "number", "value": 3}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    #[serde(flatten)]
    pub value: FieldValue,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub chronicle_id: i64,
    #[serde(rename = "type")]
    pub node_type: String,
    pub label: String,
    pub description: String,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub id: i64,
    pub chronicle_id: i64,
    pub from_node_id: i64,
    pub to_node_id: i64,
    pub edge_type: String,
    pub description: String,
    pub properties: Vec<Field>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeDirection {
    In,
    Out,
    Both,
}
```

- [ ] **Step 2: If `serde::{Deserialize, Serialize}` is not already imported at the top of the file, add it**

Check the top of `shared/types.rs`. If it already has `use serde::{Deserialize, Serialize};`, skip this step. Otherwise, add it to the imports.

- [ ] **Step 3: Compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: clean compile. The new types are not used anywhere yet; the compiler will emit `dead_code` warnings — ignore them (they'll resolve in the next tasks).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/types.rs
git commit -m "feat(types): add Chronicle, Node, Edge, Field types"
```

---

## Task 4: Create `db/chronicle.rs` with CRUD + commands + tests

**Files:**
- Create: `src-tauri/src/db/chronicle.rs`
- Modify: `src-tauri/src/db/mod.rs`

- [ ] **Step 1: Register the new module in db/mod.rs**

Open `src-tauri/src/db/mod.rs`. It currently contains:

```rust
pub mod dyscrasia;
pub mod seed;
```

Append:

```rust
pub mod chronicle;
```

- [ ] **Step 2: Create chronicle.rs with CRUD helpers, Tauri commands, and unit tests**

Create `src-tauri/src/db/chronicle.rs` with this exact content:

```rust
use sqlx::{Row, SqlitePool};
use crate::shared::types::Chronicle;

// -------- Pure CRUD helpers (testable without Tauri state) -------------

async fn db_list(pool: &SqlitePool) -> Result<Vec<Chronicle>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, name, description, created_at, updated_at
         FROM chronicles
         ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| Chronicle {
        id:          r.get("id"),
        name:        r.get("name"),
        description: r.get("description"),
        created_at:  r.get("created_at"),
        updated_at:  r.get("updated_at"),
    }).collect())
}

async fn db_get(pool: &SqlitePool, id: i64) -> Result<Chronicle, sqlx::Error> {
    let r = sqlx::query(
        "SELECT id, name, description, created_at, updated_at
         FROM chronicles WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    Ok(Chronicle {
        id:          r.get("id"),
        name:        r.get("name"),
        description: r.get("description"),
        created_at:  r.get("created_at"),
        updated_at:  r.get("updated_at"),
    })
}

async fn db_create(pool: &SqlitePool, name: &str, description: &str) -> Result<Chronicle, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO chronicles (name, description) VALUES (?, ?)"
    )
    .bind(name)
    .bind(description)
    .execute(pool)
    .await?;
    db_get(pool, result.last_insert_rowid()).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    description: &str,
) -> Result<Chronicle, sqlx::Error> {
    sqlx::query(
        "UPDATE chronicles SET name = ?, description = ? WHERE id = ?"
    )
    .bind(name)
    .bind(description)
    .bind(id)
    .execute(pool)
    .await?;
    db_get(pool, id).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM chronicles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

// -------- Tauri commands (thin wrappers; map errors to Strings) --------

#[tauri::command]
pub async fn list_chronicles(
    pool: tauri::State<'_, crate::DbState>,
) -> Result<Vec<Chronicle>, String> {
    db_list(&pool.0).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<Chronicle, String> {
    db_get(&pool.0, id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    name: String,
    description: String,
) -> Result<Chronicle, String> {
    db_create(&pool.0, &name, &description).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
) -> Result<Chronicle, String> {
    db_update(&pool.0, id, &name, &description).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_chronicle(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await.map_err(|e| e.to_string())
}

// -------- Test helper (top-level so other db modules' tests can reach it) ------

/// Build an in-memory pool with the migration applied and foreign keys enabled.
/// Declared at the file top level (rather than inside `mod tests`) so that
/// sibling test modules (e.g. `db::node::tests`, `db::edge::tests`) can
/// import it as `crate::db::chronicle::test_pool`.
#[cfg(test)]
pub(crate) async fn test_pool() -> SqlitePool {
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;

    let opts = SqliteConnectOptions::from_str("sqlite::memory:")
        .unwrap()
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(opts).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

// -------- Unit tests --------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = test_pool().await;
        assert!(db_list(&pool).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn create_then_list_round_trips() {
        let pool = test_pool().await;
        let c = db_create(&pool, "Anarch Nights", "test chronicle").await.unwrap();
        assert_eq!(c.name, "Anarch Nights");
        assert_eq!(c.description, "test chronicle");

        let all = db_list(&pool).await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].id, c.id);
    }

    #[tokio::test]
    async fn update_changes_name() {
        let pool = test_pool().await;
        let c = db_create(&pool, "Old Name", "").await.unwrap();
        let updated = db_update(&pool, c.id, "New Name", "desc").await.unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.description, "desc");
    }

    #[tokio::test]
    async fn delete_removes_chronicle() {
        let pool = test_pool().await;
        let c = db_create(&pool, "X", "").await.unwrap();
        db_delete(&pool, c.id).await.unwrap();
        assert!(db_list(&pool).await.unwrap().is_empty());
    }
}
```

- [ ] **Step 3: Run the tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::chronicle::tests
```

Expected: `test result: ok. 4 passed; 0 failed`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/chronicle.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): chronicles CRUD with Tauri commands and unit tests"
```

---

## Task 5: Create `db/node.rs` basic CRUD (5 commands) + tests

**Files:**
- Create: `src-tauri/src/db/node.rs`
- Modify: `src-tauri/src/db/mod.rs`

- [ ] **Step 1: Register the module in db/mod.rs**

Open `src-tauri/src/db/mod.rs` and append:

```rust
pub mod node;
```

- [ ] **Step 2: Create node.rs with CRUD helpers, Tauri commands, and tests**

Create `src-tauri/src/db/node.rs` with this exact content:

```rust
use sqlx::{Row, SqlitePool};
use crate::shared::types::{Node, Field};

// -------- Serialization helpers --------

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

fn row_to_node(r: &sqlx::sqlite::SqliteRow) -> Result<Node, String> {
    let tags_json: String = r.get("tags_json");
    let properties_json: String = r.get("properties_json");
    Ok(Node {
        id:           r.get("id"),
        chronicle_id: r.get("chronicle_id"),
        node_type:    r.get("type"),
        label:        r.get("label"),
        description:  r.get("description"),
        tags:         deserialize_tags(&tags_json)?,
        properties:   deserialize_properties(&properties_json)?,
        created_at:   r.get("created_at"),
        updated_at:   r.get("updated_at"),
    })
}

// -------- Pure CRUD helpers --------

async fn db_list(
    pool: &SqlitePool,
    chronicle_id: i64,
    type_filter: Option<&str>,
) -> Result<Vec<Node>, String> {
    let rows = match type_filter {
        Some(t) => sqlx::query(
            "SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
             FROM nodes WHERE chronicle_id = ? AND type = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).bind(t).fetch_all(pool).await,
        None => sqlx::query(
            "SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
             FROM nodes WHERE chronicle_id = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).fetch_all(pool).await,
    }.map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get(pool: &SqlitePool, id: i64) -> Result<Node, String> {
    let r = sqlx::query(
        "SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
         FROM nodes WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    row_to_node(&r)
}

async fn db_create(
    pool: &SqlitePool,
    chronicle_id: i64,
    node_type: &str,
    label: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<Node, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;
    let result = sqlx::query(
        "INSERT INTO nodes (chronicle_id, type, label, description, tags_json, properties_json)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(chronicle_id)
    .bind(node_type)
    .bind(label)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, result.last_insert_rowid()).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    node_type: &str,
    label: &str,
    description: &str,
    tags: &[String],
    properties: &[Field],
) -> Result<Node, String> {
    let tags_json = serialize_tags(tags)?;
    let properties_json = serialize_properties(properties)?;
    sqlx::query(
        "UPDATE nodes SET type = ?, label = ?, description = ?, tags_json = ?, properties_json = ?
         WHERE id = ?"
    )
    .bind(node_type)
    .bind(label)
    .bind(description)
    .bind(&tags_json)
    .bind(&properties_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, id).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM nodes WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// -------- Tauri commands --------

#[tauri::command]
pub async fn list_nodes(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    type_filter: Option<String>,
) -> Result<Vec<Node>, String> {
    db_list(&pool.0, chronicle_id, type_filter.as_deref()).await
}

#[tauri::command]
pub async fn get_node(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<Node, String> {
    db_get(&pool.0, id).await
}

#[tauri::command]
pub async fn create_node(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    node_type: String,
    label: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Node, String> {
    db_create(&pool.0, chronicle_id, &node_type, &label, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn update_node(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    node_type: String,
    label: String,
    description: String,
    tags: Vec<String>,
    properties: Vec<Field>,
) -> Result<Node, String> {
    db_update(&pool.0, id, &node_type, &label, &description, &tags, &properties).await
}

#[tauri::command]
pub async fn delete_node(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

// -------- Unit tests --------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::chronicle::test_pool;
    use crate::shared::types::{FieldValue, StringFieldValue, NumberFieldValue};

    async fn make_chronicle(pool: &SqlitePool) -> i64 {
        let r = sqlx::query("INSERT INTO chronicles (name) VALUES ('Test')")
            .execute(pool).await.unwrap();
        r.last_insert_rowid()
    }

    #[tokio::test]
    async fn create_and_get_round_trips() {
        let pool = test_pool().await;
        let chronicle_id = make_chronicle(&pool).await;

        let props = vec![
            Field {
                name: "influence_rating".into(),
                value: FieldValue::Number { value: NumberFieldValue::Single(3.0) },
            },
            Field {
                name: "aliases".into(),
                value: FieldValue::String {
                    value: StringFieldValue::Multi(vec!["Nightjar".into(), "V".into()]),
                },
            },
        ];

        let created = db_create(
            &pool,
            chronicle_id,
            "area",
            "Manhattan",
            "The big borough",
            &["geographic".into(), "urban".into()],
            &props,
        ).await.unwrap();

        assert_eq!(created.label, "Manhattan");
        assert_eq!(created.node_type, "area");
        assert_eq!(created.tags, vec!["geographic".to_string(), "urban".into()]);
        assert_eq!(created.properties.len(), 2);

        let fetched = db_get(&pool, created.id).await.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.properties, props);
    }

    #[tokio::test]
    async fn list_filters_by_chronicle_and_type() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;

        db_create(&pool, cid, "area",      "A", "", &[], &[]).await.unwrap();
        db_create(&pool, cid, "area",      "B", "", &[], &[]).await.unwrap();
        db_create(&pool, cid, "character", "C", "", &[], &[]).await.unwrap();

        let areas = db_list(&pool, cid, Some("area")).await.unwrap();
        assert_eq!(areas.len(), 2);
        let all = db_list(&pool, cid, None).await.unwrap();
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn update_persists_changes() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let n = db_create(&pool, cid, "area", "Old", "", &[], &[]).await.unwrap();

        let updated = db_update(&pool, n.id, "area", "New", "desc", &["tag1".into()], &[]).await.unwrap();
        assert_eq!(updated.label, "New");
        assert_eq!(updated.description, "desc");
        assert_eq!(updated.tags, vec!["tag1".to_string()]);
    }

    #[tokio::test]
    async fn delete_removes_node() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let n = db_create(&pool, cid, "area", "X", "", &[], &[]).await.unwrap();
        db_delete(&pool, n.id).await.unwrap();
        assert!(db_list(&pool, cid, None).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn chronicle_delete_cascades_to_nodes() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        db_create(&pool, cid, "area", "X", "", &[], &[]).await.unwrap();

        sqlx::query("DELETE FROM chronicles WHERE id = ?")
            .bind(cid).execute(&pool).await.unwrap();

        assert!(db_list(&pool, cid, None).await.unwrap().is_empty());
    }
}
```

- [ ] **Step 3: Confirm `test_pool` is reachable from this module**

The tests in node.rs reference `crate::db::chronicle::test_pool`. In Task 4 Step 2 we placed `test_pool` at the file's top level (outside `mod tests`) with `#[cfg(test)] pub(crate)` visibility, which makes it reachable by siblings. Verify that in `src-tauri/src/db/chronicle.rs` there is a top-level function declaration matching:

```rust
#[cfg(test)]
pub(crate) async fn test_pool() -> SqlitePool {
    // ...
}
```

If it's nested inside `mod tests`, move it out to the file level. Sibling modules cannot see items inside another module's private `tests` submodule.

- [ ] **Step 4: Run the tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::node::tests
```

Expected: `test result: ok. 5 passed; 0 failed`.

- [ ] **Step 5: Run the full test suite to make sure chronicle tests still pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass (chronicle's 4 + node's 5 + any pre-existing dyscrasia tests).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db/node.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): nodes CRUD with typed properties and unit tests"
```

---

## Task 6: Add derived tree queries and cycle-check helper to `db/node.rs`

**Files:**
- Modify: `src-tauri/src/db/node.rs`

This task adds five recursive-CTE queries (parent, children, siblings, path-to-root, subtree) plus a `would_create_cycle` helper that Task 7 will call from edge creation.

- [ ] **Step 1: Append the derived-query helpers and commands**

Append the following to `src-tauri/src/db/node.rs` (after the existing `db_delete` function, before the `#[cfg(test)]` block):

```rust
// -------- Derived tree queries (all follow edge_type = 'contains') --------

async fn db_get_parent(pool: &SqlitePool, node_id: i64) -> Result<Option<Node>, String> {
    let r = sqlx::query(
        "SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at
         FROM nodes n
         JOIN edges e ON e.from_node_id = n.id
         WHERE e.to_node_id = ? AND e.edge_type = 'contains'
         LIMIT 1"
    )
    .bind(node_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())?;

    match r {
        Some(row) => Ok(Some(row_to_node(&row)?)),
        None => Ok(None),
    }
}

async fn db_get_children(pool: &SqlitePool, node_id: i64) -> Result<Vec<Node>, String> {
    let rows = sqlx::query(
        "SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at
         FROM nodes n
         JOIN edges e ON e.to_node_id = n.id
         WHERE e.from_node_id = ? AND e.edge_type = 'contains'
         ORDER BY n.id ASC"
    )
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get_siblings(pool: &SqlitePool, node_id: i64) -> Result<Vec<Node>, String> {
    let rows = sqlx::query(
        "SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at
         FROM nodes n
         JOIN edges e ON e.to_node_id = n.id
         WHERE e.edge_type = 'contains'
           AND e.from_node_id = (
               SELECT from_node_id FROM edges
               WHERE to_node_id = ? AND edge_type = 'contains'
               LIMIT 1
           )
           AND n.id != ?
         ORDER BY n.id ASC"
    )
    .bind(node_id)
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get_path_to_root(pool: &SqlitePool, node_id: i64) -> Result<Vec<Node>, String> {
    let rows = sqlx::query(
        "WITH RECURSIVE ancestors(id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at, depth) AS (
            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, 0
            FROM nodes n WHERE n.id = ?

            UNION ALL

            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, a.depth + 1
            FROM nodes n
            JOIN edges e ON e.from_node_id = n.id
            JOIN ancestors a ON a.id = e.to_node_id
            WHERE e.edge_type = 'contains' AND a.depth < 32
        )
        SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
        FROM ancestors WHERE depth > 0 ORDER BY depth ASC"
    )
    .bind(node_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

async fn db_get_subtree(
    pool: &SqlitePool,
    node_id: i64,
    max_depth: Option<i32>,
) -> Result<Vec<Node>, String> {
    let cap = max_depth.unwrap_or(32);
    let rows = sqlx::query(
        "WITH RECURSIVE descendants(id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at, depth) AS (
            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, 0
            FROM nodes n WHERE n.id = ?

            UNION ALL

            SELECT n.id, n.chronicle_id, n.type, n.label, n.description, n.tags_json, n.properties_json, n.created_at, n.updated_at, d.depth + 1
            FROM nodes n
            JOIN edges e ON e.to_node_id = n.id
            JOIN descendants d ON d.id = e.from_node_id
            WHERE e.edge_type = 'contains' AND d.depth < ?
        )
        SELECT id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at
        FROM descendants WHERE depth > 0 ORDER BY depth ASC, id ASC"
    )
    .bind(node_id)
    .bind(cap)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    rows.iter().map(row_to_node).collect()
}

/// Returns true if creating a contains edge `from -> to` would produce a cycle.
/// Cycle exists if `from` is already in `to`'s descendant set (via contains).
pub(crate) async fn would_create_cycle(
    pool: &SqlitePool,
    from_node_id: i64,
    to_node_id: i64,
) -> Result<bool, String> {
    let row = sqlx::query(
        "WITH RECURSIVE descendants(id, depth) AS (
            SELECT ?, 0
            UNION ALL
            SELECT e.to_node_id, d.depth + 1
            FROM edges e
            JOIN descendants d ON e.from_node_id = d.id
            WHERE e.edge_type = 'contains' AND d.depth < 32
        )
        SELECT COUNT(*) AS cnt FROM descendants WHERE id = ?"
    )
    .bind(to_node_id)
    .bind(from_node_id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    let cnt: i64 = row.get("cnt");
    Ok(cnt > 0)
}

// -------- Derived-query Tauri commands --------

#[tauri::command]
pub async fn get_parent_of(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Option<Node>, String> {
    db_get_parent(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_children_of(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Vec<Node>, String> {
    db_get_children(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_siblings_of(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Vec<Node>, String> {
    db_get_siblings(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_path_to_root(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
) -> Result<Vec<Node>, String> {
    db_get_path_to_root(&pool.0, node_id).await
}

#[tauri::command]
pub async fn get_subtree(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
    max_depth: Option<i32>,
) -> Result<Vec<Node>, String> {
    db_get_subtree(&pool.0, node_id, max_depth).await
}
```

- [ ] **Step 2: Append tests for the derived queries to the existing `mod tests` block**

In `src-tauri/src/db/node.rs`, locate the existing `#[cfg(test)] mod tests { ... }` block from Task 5. Append the following test functions inside it (before the closing `}` of `mod tests`):

```rust
    /// Helper: insert a raw contains edge for test scaffolding, bypassing edge.rs logic.
    async fn insert_contains(pool: &SqlitePool, chronicle_id: i64, from: i64, to: i64) {
        sqlx::query(
            "INSERT INTO edges (chronicle_id, from_node_id, to_node_id, edge_type)
             VALUES (?, ?, ?, 'contains')"
        )
        .bind(chronicle_id).bind(from).bind(to)
        .execute(pool).await.unwrap();
    }

    #[tokio::test]
    async fn get_parent_returns_some_when_parent_exists() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let parent = db_create(&pool, cid, "area", "Parent", "", &[], &[]).await.unwrap();
        let child  = db_create(&pool, cid, "area", "Child",  "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, parent.id, child.id).await;

        let p = db_get_parent(&pool, child.id).await.unwrap();
        assert!(p.is_some());
        assert_eq!(p.unwrap().id, parent.id);
    }

    #[tokio::test]
    async fn get_parent_returns_none_for_root() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let root = db_create(&pool, cid, "area", "Root", "", &[], &[]).await.unwrap();
        assert!(db_get_parent(&pool, root.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn get_children_returns_all_direct_children() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let parent = db_create(&pool, cid, "area", "Parent", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "C", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, parent.id, a.id).await;
        insert_contains(&pool, cid, parent.id, b.id).await;
        insert_contains(&pool, cid, a.id,      c.id).await;  // grandchild, should NOT appear

        let kids = db_get_children(&pool, parent.id).await.unwrap();
        let ids: Vec<i64> = kids.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![a.id, b.id]);
    }

    #[tokio::test]
    async fn get_siblings_returns_peers() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let parent = db_create(&pool, cid, "area", "P", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "C", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, parent.id, a.id).await;
        insert_contains(&pool, cid, parent.id, b.id).await;
        insert_contains(&pool, cid, parent.id, c.id).await;

        let sibs = db_get_siblings(&pool, a.id).await.unwrap();
        let ids: Vec<i64> = sibs.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![b.id, c.id]);
    }

    #[tokio::test]
    async fn get_siblings_empty_for_root() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let n = db_create(&pool, cid, "area", "Solo", "", &[], &[]).await.unwrap();
        assert!(db_get_siblings(&pool, n.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn get_path_to_root_returns_ancestors_bottom_up() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let g = db_create(&pool, cid, "area", "State",     "", &[], &[]).await.unwrap();
        let p = db_create(&pool, cid, "area", "City",      "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "Borough",   "", &[], &[]).await.unwrap();
        let l = db_create(&pool, cid, "area", "Neighbor",  "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, g.id, p.id).await;
        insert_contains(&pool, cid, p.id, c.id).await;
        insert_contains(&pool, cid, c.id, l.id).await;

        let path = db_get_path_to_root(&pool, l.id).await.unwrap();
        let ids: Vec<i64> = path.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![c.id, p.id, g.id]);
    }

    #[tokio::test]
    async fn get_subtree_returns_descendants() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let r = db_create(&pool, cid, "area", "Root", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A",    "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B",    "", &[], &[]).await.unwrap();
        let c = db_create(&pool, cid, "area", "C",    "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, r.id, a.id).await;
        insert_contains(&pool, cid, r.id, b.id).await;
        insert_contains(&pool, cid, a.id, c.id).await;

        let sub = db_get_subtree(&pool, r.id, None).await.unwrap();
        let ids: Vec<i64> = sub.iter().map(|n| n.id).collect();
        assert_eq!(ids.len(), 3);
        assert!(ids.contains(&a.id));
        assert!(ids.contains(&b.id));
        assert!(ids.contains(&c.id));
    }

    #[tokio::test]
    async fn get_subtree_respects_max_depth() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let r = db_create(&pool, cid, "area", "R", "", &[], &[]).await.unwrap();
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, r.id, a.id).await;
        insert_contains(&pool, cid, a.id, b.id).await;

        let sub = db_get_subtree(&pool, r.id, Some(1)).await.unwrap();
        let ids: Vec<i64> = sub.iter().map(|n| n.id).collect();
        assert_eq!(ids, vec![a.id]);  // 'b' excluded at depth 2
    }

    #[tokio::test]
    async fn would_create_cycle_true_for_back_edge() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();
        insert_contains(&pool, cid, a.id, b.id).await;

        // Creating b.contains(a) would cycle.
        assert!(would_create_cycle(&pool, b.id, a.id).await.unwrap());
    }

    #[tokio::test]
    async fn would_create_cycle_false_for_safe_edge() {
        let pool = test_pool().await;
        let cid = make_chronicle(&pool).await;
        let a = db_create(&pool, cid, "area", "A", "", &[], &[]).await.unwrap();
        let b = db_create(&pool, cid, "area", "B", "", &[], &[]).await.unwrap();

        // No existing edges; creating a.contains(b) is safe.
        assert!(!would_create_cycle(&pool, a.id, b.id).await.unwrap());
    }
}
```

- [ ] **Step 3: Run the tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::node::tests
```

Expected: `test result: ok. 15 passed; 0 failed` (5 from Task 5 + 10 new).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/db/node.rs
git commit -m "feat(db): derived tree queries and cycle-check helper on nodes"
```

---

## Task 7: Create `db/edge.rs` with CRUD, cycle check, and tests

**Files:**
- Create: `src-tauri/src/db/edge.rs`
- Modify: `src-tauri/src/db/mod.rs`

- [ ] **Step 1: Register the module in db/mod.rs**

Open `src-tauri/src/db/mod.rs` and append:

```rust
pub mod edge;
```

- [ ] **Step 2: Create edge.rs with CRUD, cycle-integration, Tauri commands, and tests**

Create `src-tauri/src/db/edge.rs` with this exact content:

```rust
use sqlx::{Row, SqlitePool};
use crate::shared::types::{Edge, Field, EdgeDirection};
use crate::db::node::would_create_cycle;

// -------- Serialization helper (mirrors node.rs for properties) --------

fn serialize_properties(props: &[Field]) -> Result<String, String> {
    serde_json::to_string(props).map_err(|e| e.to_string())
}

fn deserialize_properties(s: &str) -> Result<Vec<Field>, String> {
    serde_json::from_str(s).map_err(|e| e.to_string())
}

fn row_to_edge(r: &sqlx::sqlite::SqliteRow) -> Result<Edge, String> {
    let properties_json: String = r.get("properties_json");
    Ok(Edge {
        id:              r.get("id"),
        chronicle_id:    r.get("chronicle_id"),
        from_node_id:    r.get("from_node_id"),
        to_node_id:      r.get("to_node_id"),
        edge_type:       r.get("edge_type"),
        description:     r.get("description"),
        properties:      deserialize_properties(&properties_json)?,
        created_at:      r.get("created_at"),
        updated_at:      r.get("updated_at"),
    })
}

// -------- Pure CRUD helpers --------

async fn db_list(
    pool: &SqlitePool,
    chronicle_id: i64,
    edge_type_filter: Option<&str>,
) -> Result<Vec<Edge>, String> {
    let rows = match edge_type_filter {
        Some(t) => sqlx::query(
            "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at
             FROM edges WHERE chronicle_id = ? AND edge_type = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).bind(t).fetch_all(pool).await,
        None => sqlx::query(
            "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at
             FROM edges WHERE chronicle_id = ?
             ORDER BY id ASC"
        ).bind(chronicle_id).fetch_all(pool).await,
    }.map_err(|e| e.to_string())?;

    rows.iter().map(row_to_edge).collect()
}

async fn db_list_for_node(
    pool: &SqlitePool,
    node_id: i64,
    direction: &EdgeDirection,
    edge_type_filter: Option<&str>,
) -> Result<Vec<Edge>, String> {
    const COLS: &str = "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at FROM edges";

    let rows = match (direction, edge_type_filter) {
        (EdgeDirection::Out,  Some(t)) => sqlx::query(&format!("{COLS} WHERE from_node_id = ? AND edge_type = ? ORDER BY id ASC"))
            .bind(node_id).bind(t).fetch_all(pool).await,
        (EdgeDirection::Out,  None)    => sqlx::query(&format!("{COLS} WHERE from_node_id = ? ORDER BY id ASC"))
            .bind(node_id).fetch_all(pool).await,
        (EdgeDirection::In,   Some(t)) => sqlx::query(&format!("{COLS} WHERE to_node_id = ? AND edge_type = ? ORDER BY id ASC"))
            .bind(node_id).bind(t).fetch_all(pool).await,
        (EdgeDirection::In,   None)    => sqlx::query(&format!("{COLS} WHERE to_node_id = ? ORDER BY id ASC"))
            .bind(node_id).fetch_all(pool).await,
        (EdgeDirection::Both, Some(t)) => sqlx::query(&format!("{COLS} WHERE (from_node_id = ? OR to_node_id = ?) AND edge_type = ? ORDER BY id ASC"))
            .bind(node_id).bind(node_id).bind(t).fetch_all(pool).await,
        (EdgeDirection::Both, None)    => sqlx::query(&format!("{COLS} WHERE from_node_id = ? OR to_node_id = ? ORDER BY id ASC"))
            .bind(node_id).bind(node_id).fetch_all(pool).await,
    }.map_err(|e| e.to_string())?;

    rows.iter().map(row_to_edge).collect()
}

async fn db_get(pool: &SqlitePool, id: i64) -> Result<Edge, String> {
    let r = sqlx::query(
        "SELECT id, chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json, created_at, updated_at
         FROM edges WHERE id = ?"
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;
    row_to_edge(&r)
}

async fn db_create(
    pool: &SqlitePool,
    chronicle_id: i64,
    from_node_id: i64,
    to_node_id: i64,
    edge_type: &str,
    description: &str,
    properties: &[Field],
) -> Result<Edge, String> {
    // Cycle prevention for contains edges only.
    if edge_type == "contains"
        && would_create_cycle(pool, from_node_id, to_node_id).await?
    {
        return Err("cycle detected: creating this edge would form a cycle in the contains graph".to_string());
    }

    let properties_json = serialize_properties(properties)?;
    let result = sqlx::query(
        "INSERT INTO edges (chronicle_id, from_node_id, to_node_id, edge_type, description, properties_json)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(chronicle_id)
    .bind(from_node_id)
    .bind(to_node_id)
    .bind(edge_type)
    .bind(description)
    .bind(&properties_json)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, result.last_insert_rowid()).await
}

async fn db_update(
    pool: &SqlitePool,
    id: i64,
    edge_type: &str,
    description: &str,
    properties: &[Field],
) -> Result<Edge, String> {
    let properties_json = serialize_properties(properties)?;
    sqlx::query(
        "UPDATE edges SET edge_type = ?, description = ?, properties_json = ?
         WHERE id = ?"
    )
    .bind(edge_type)
    .bind(description)
    .bind(&properties_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    db_get(pool, id).await
}

async fn db_delete(pool: &SqlitePool, id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM edges WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

// -------- Tauri commands --------

#[tauri::command]
pub async fn list_edges(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    edge_type_filter: Option<String>,
) -> Result<Vec<Edge>, String> {
    db_list(&pool.0, chronicle_id, edge_type_filter.as_deref()).await
}

#[tauri::command]
pub async fn list_edges_for_node(
    pool: tauri::State<'_, crate::DbState>,
    node_id: i64,
    direction: EdgeDirection,
    edge_type_filter: Option<String>,
) -> Result<Vec<Edge>, String> {
    db_list_for_node(&pool.0, node_id, &direction, edge_type_filter.as_deref()).await
}

#[tauri::command]
pub async fn create_edge(
    pool: tauri::State<'_, crate::DbState>,
    chronicle_id: i64,
    from_node_id: i64,
    to_node_id: i64,
    edge_type: String,
    description: String,
    properties: Vec<Field>,
) -> Result<Edge, String> {
    db_create(&pool.0, chronicle_id, from_node_id, to_node_id, &edge_type, &description, &properties).await
}

#[tauri::command]
pub async fn update_edge(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    edge_type: String,
    description: String,
    properties: Vec<Field>,
) -> Result<Edge, String> {
    db_update(&pool.0, id, &edge_type, &description, &properties).await
}

#[tauri::command]
pub async fn delete_edge(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    db_delete(&pool.0, id).await
}

// -------- Unit tests --------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::chronicle::test_pool;

    async fn mk_node(pool: &SqlitePool, cid: i64, label: &str) -> i64 {
        let r = sqlx::query("INSERT INTO nodes (chronicle_id, type, label) VALUES (?, 'area', ?)")
            .bind(cid).bind(label).execute(pool).await.unwrap();
        r.last_insert_rowid()
    }

    async fn setup(pool: &SqlitePool) -> (i64, i64, i64, i64) {
        let r = sqlx::query("INSERT INTO chronicles (name) VALUES ('T')")
            .execute(pool).await.unwrap();
        let cid = r.last_insert_rowid();
        let a = mk_node(pool, cid, "A").await;
        let b = mk_node(pool, cid, "B").await;
        let c = mk_node(pool, cid, "C").await;
        (cid, a, b, c)
    }

    #[tokio::test]
    async fn create_edge_happy_path() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        let e = db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        assert_eq!(e.from_node_id, a);
        assert_eq!(e.to_node_id, b);
        assert_eq!(e.edge_type, "contains");
    }

    #[tokio::test]
    async fn self_loop_rejected() {
        let pool = test_pool().await;
        let (cid, a, _, _) = setup(&pool).await;
        let result = db_create(&pool, cid, a, a, "contains", "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn duplicate_edge_same_type_rejected() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        let result = db_create(&pool, cid, a, b, "contains", "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn duplicate_edge_different_type_allowed() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains",    "", &[]).await.unwrap();
        db_create(&pool, cid, a, b, "adjacent-to", "", &[]).await.unwrap();  // should succeed
    }

    #[tokio::test]
    async fn second_contains_parent_rejected() {
        let pool = test_pool().await;
        let (cid, a, b, c) = setup(&pool).await;
        db_create(&pool, cid, a, c, "contains", "", &[]).await.unwrap();
        // b also tries to contain c — partial unique index should reject.
        let result = db_create(&pool, cid, b, c, "contains", "", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn cycle_rejected() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        // Attempting b.contains(a) would close the cycle.
        let result = db_create(&pool, cid, b, a, "contains", "", &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycle"));
    }

    #[tokio::test]
    async fn cycle_check_only_applies_to_contains() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains",     "", &[]).await.unwrap();
        // allied-with can freely cycle; not rejected.
        db_create(&pool, cid, b, a, "allied-with", "", &[]).await.unwrap();
    }

    #[tokio::test]
    async fn list_for_node_filters_direction() {
        let pool = test_pool().await;
        let (cid, a, b, c) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        db_create(&pool, cid, c, a, "contains", "", &[]).await.unwrap();

        let out  = db_list_for_node(&pool, a, &EdgeDirection::Out,  None).await.unwrap();
        let inc  = db_list_for_node(&pool, a, &EdgeDirection::In,   None).await.unwrap();
        let both = db_list_for_node(&pool, a, &EdgeDirection::Both, None).await.unwrap();
        assert_eq!(out.len(),  1);
        assert_eq!(inc.len(),  1);
        assert_eq!(both.len(), 2);
    }

    #[tokio::test]
    async fn node_delete_cascades_to_edges() {
        let pool = test_pool().await;
        let (cid, a, b, _) = setup(&pool).await;
        db_create(&pool, cid, a, b, "contains", "", &[]).await.unwrap();
        sqlx::query("DELETE FROM nodes WHERE id = ?").bind(a).execute(&pool).await.unwrap();

        let left = db_list(&pool, cid, None).await.unwrap();
        assert!(left.is_empty(), "edge should have cascaded");
    }
}
```

- [ ] **Step 3: Run the tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::edge::tests
```

Expected: `test result: ok. 9 passed; 0 failed`.

- [ ] **Step 4: Run the full test suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all tests pass. Check the count: roughly 28 tests (dyscrasia's existing + 4 chronicle + 15 node + 9 edge).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/edge.rs src-tauri/src/db/mod.rs
git commit -m "feat(db): edges CRUD with cycle check and unit tests"
```

---

## Task 8: Register all 20 new commands in lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Append the 20 command references to `invoke_handler`**

Open `src-tauri/src/lib.rs`. Locate the `.invoke_handler(tauri::generate_handler![...])` block. Add the following lines inside the `generate_handler![...]` macro, after the existing entries (before the closing `]`):

```rust
            db::chronicle::list_chronicles,
            db::chronicle::get_chronicle,
            db::chronicle::create_chronicle,
            db::chronicle::update_chronicle,
            db::chronicle::delete_chronicle,
            db::node::list_nodes,
            db::node::get_node,
            db::node::create_node,
            db::node::update_node,
            db::node::delete_node,
            db::node::get_parent_of,
            db::node::get_children_of,
            db::node::get_siblings_of,
            db::node::get_path_to_root,
            db::node::get_subtree,
            db::edge::list_edges,
            db::edge::list_edges_for_node,
            db::edge::create_edge,
            db::edge::update_edge,
            db::edge::delete_edge,
```

- [ ] **Step 2: Full compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: clean compile. Any `unused` warnings on the new commands should be gone because they're now referenced by the handler.

- [ ] **Step 3: Full build verification**

```bash
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: successful build. This is slower than `check` but verifies the whole binary links correctly with the new Tauri command bindings.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(tauri): register 20 Domains Manager commands in invoke_handler"
```

---

## Task 9: Add TypeScript types to `src/types.ts`

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Append the new types**

Open `src/types.ts`. Append the following at the end of the file:

```ts

// ---------------------------------------------------------------------------
// Domains Manager / Chronicle graph types
// ---------------------------------------------------------------------------

export interface Chronicle {
  id: number;
  name: string;
  description: string;
  created_at: string;
  updated_at: string;
}

export type FieldValue =
  | { type: 'string';    value: string | string[] }
  | { type: 'text';      value: string }
  | { type: 'number';    value: number | number[] }
  | { type: 'date';      value: string }
  | { type: 'url';       value: string }
  | { type: 'email';     value: string }
  | { type: 'bool';      value: boolean }
  | { type: 'reference'; value: number };

export type Field = { name: string } & FieldValue;

export interface ChronicleNode {
  id: number;
  chronicle_id: number;
  type: string;
  label: string;
  description: string;
  tags: string[];
  properties: Field[];
  created_at: string;
  updated_at: string;
}

export interface ChronicleEdge {
  id: number;
  chronicle_id: number;
  from_node_id: number;
  to_node_id: number;
  edge_type: string;
  description: string;
  properties: Field[];
  created_at: string;
  updated_at: string;
}

export type EdgeDirection = 'in' | 'out' | 'both';
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```

Expected: clean — no new errors. Pre-existing non-blocking warnings are fine; new errors caused by the appended types are not.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat(types): add Chronicle, ChronicleNode, ChronicleEdge TS types"
```

---

## Task 10: Update `docs/design/data-sources.md`

**Files:**
- Modify: `docs/design/data-sources.md`

- [ ] **Step 1: Insert the Chronicle Store section**

Open `docs/design/data-sources.md`. Find the "## Dyscrasia Store" section. Directly **after** that section (and before "## Roll20 Live Feed"), insert the following with a `---` separator above and below:

```markdown
---

## Chronicle Store

**Direction:** Both (app reads and writes)
**What it is:** The SQLite tables holding all user-authored chronicle data — geographic/organizational areas, characters, businesses, merits, influence holdings, and typed relationships between them. Split across three tables: `chronicles`, `nodes`, `edges`.

**Data carried per chronicle:** name, description, timestamps.

**Data carried per node:** type (freeform user-chosen string), label, markdown description, cross-cutting tags, and a typed property bag (array of named fields with declared types: string, text, number, date, url, email, bool, reference).

**Data carried per edge:** type (freeform user-chosen string), from-node, to-node, markdown description, typed property bag.

**Behavior:**
- Nothing is seeded; all data is user-created.
- The `"contains"` edge type is treated by the UI as the navigation relationship (breadcrumbs and drilldown walk it), but the DB does not privilege it.
- Deleting a chronicle cascades to all its nodes and edges. Deleting a node cascades to all its edges.
- A node has at most one incoming `contains` edge (enforced by a partial unique index) — the `contains` sub-graph is a strict tree.

**Currently used by:** Domains Manager (full CRUD + derived tree queries).
```

- [ ] **Step 2: Also update the Quick Reference Table at the bottom**

Still in `docs/design/data-sources.md`, find the "### Quick Reference Table" at the end. Add this row after the `Dyscrasia Store` row:

```markdown
| Chronicle Store         | Both            | SQLite     | Backend          |
```

- [ ] **Step 3: Commit**

```bash
git add docs/design/data-sources.md
git commit -m "docs: add Chronicle Store data source entry"
```

---

## Task 11: Update `docs/design/data-sources-kumu.json`

**Files:**
- Modify: `docs/design/data-sources-kumu.json`

- [ ] **Step 1: Add two new elements to the `elements` array**

Open `docs/design/data-sources-kumu.json`. Find the `elements` array. Add these two entries at the end of the array (before the closing `]` of `elements`). Make sure to add a comma after the preceding element:

```json
    {
      "label": "Chronicle Store",
      "type": "Data Source",
      "description": "SQLite tables (chronicles, nodes, edges) holding all user-authored game state — areas, characters, businesses, merits, influence, and typed relationships.",
      "tags": ["input", "output", "persistent"],
      "Direction": "Both",
      "Storage": "SQLite",
      "Layer": "Backend"
    },
    {
      "label": "Domains Manager",
      "type": "Tool",
      "description": "Hierarchical chronicle tracker. Lets the Storyteller model a game's territories, institutions, characters, and relationships as a graph of typed nodes connected by typed edges.",
      "tags": ["tool", "frontend"]
    }
```

- [ ] **Step 2: Add two new connections to the `connections` array**

Still in `docs/design/data-sources-kumu.json`. Find the `connections` array. Add these two entries at the end (before the closing `]`). Add a comma after the preceding connection:

```json
    { "from": "Storyteller",     "to": "Domains Manager",  "type": "manages",          "direction": "directed" },
    { "from": "Domains Manager", "to": "Chronicle Store",  "type": "reads and writes", "direction": "mutual"   }
```

- [ ] **Step 3: Verify the JSON is valid**

```bash
python3 -m json.tool docs/design/data-sources-kumu.json > /dev/null && echo OK
```

Expected: prints `OK`. Any error means a syntax slip (missing comma, trailing comma, unquoted key) — fix and re-run.

- [ ] **Step 4: Commit**

```bash
git add docs/design/data-sources-kumu.json
git commit -m "docs: register Chronicle Store + Domains Manager in Kumu map"
```

---

## Task 12: Update `CLAUDE.md`

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Append the schema paragraph to the Database section**

Open `CLAUDE.md`. Find the `### Database` section. Locate the existing paragraph ending with "... DB enforces the former." Directly after that paragraph, append:

```markdown

Migration `0002_chronicle_graph.sql` adds three tables for the Domains Manager tool: `chronicles` (one row per running game), `nodes` (any discrete thing — area, character, institution, business, merit), and `edges` (typed directional relationships between nodes). `nodes.type` and `edges.edge_type` are freeform user-authored strings; no enum enforcement. Custom fields live in `nodes.properties_json` / `edges.properties_json` as a JSON array of typed Field records (each `{name, type, value}`). Deleting a chronicle cascades to its nodes and edges; deleting a node cascades to its edges. The `"contains"` edge type is the UI's convention for hierarchy/drilldown (and a partial unique index enforces at most one `contains` parent per node) but other edge types are unrestricted.
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude): document Domains Manager schema in Database section"
```

---

## Task 13: Manual smoke test

This is the end-to-end verification. No commit at the end — this is a validation step, not a code change.

- [ ] **Step 1: Launch the app in dev mode**

```bash
npm run tauri dev
```

Expected: app opens. Migration `0002` runs automatically. No errors in the terminal.

- [ ] **Step 2: Open devtools**

In the running Tauri window: right-click → Inspect (or `Ctrl+Shift+I`). Go to the Console tab.

- [ ] **Step 3: Run happy-path smoke test**

Paste this block into the console and press Enter. It should log a chronicle id, two node ids, an edge id, and an array with one child node (the Docks).

```js
const invoke = window.__TAURI__.core.invoke;
(async () => {
  const chronicle = await invoke('create_chronicle', {
    name: 'Smoke Test', description: ''
  });
  console.log('chronicle:', chronicle);

  const manhattan = await invoke('create_node', {
    chronicleId: chronicle.id, nodeType: 'area', label: 'Manhattan',
    description: '', tags: ['geographic'], properties: []
  });
  console.log('manhattan:', manhattan);

  const docks = await invoke('create_node', {
    chronicleId: chronicle.id, nodeType: 'area', label: 'The Docks',
    description: '', tags: ['waterfront'], properties: []
  });
  console.log('docks:', docks);

  const edge = await invoke('create_edge', {
    chronicleId: chronicle.id,
    fromNodeId: manhattan.id, toNodeId: docks.id,
    edgeType: 'contains', description: '', properties: []
  });
  console.log('edge:', edge);

  const children = await invoke('get_children_of', { nodeId: manhattan.id });
  console.log('children of Manhattan:', children);
})();
```

Expected output includes a `children of Manhattan:` line with one element whose `label` is `"The Docks"`.

- [ ] **Step 4: Self-loop should be rejected**

```js
(async () => {
  try {
    const [c] = await invoke('list_chronicles', {});
    const [n] = await invoke('list_nodes', { chronicleId: c.id, typeFilter: null });
    await invoke('create_edge', {
      chronicleId: c.id, fromNodeId: n.id, toNodeId: n.id,
      edgeType: 'contains', description: '', properties: []
    });
    console.error('FAIL: self-loop was not rejected');
  } catch (e) {
    console.log('OK: self-loop rejected →', e);
  }
})();
```

Expected: logs `OK: self-loop rejected →` followed by a CHECK constraint error.

- [ ] **Step 5: Cycle should be rejected**

```js
(async () => {
  const [c] = await invoke('list_chronicles', {});
  const nodes = await invoke('list_nodes', { chronicleId: c.id, typeFilter: null });
  const manhattan = nodes.find(n => n.label === 'Manhattan');
  const docks     = nodes.find(n => n.label === 'The Docks');
  try {
    await invoke('create_edge', {
      chronicleId: c.id, fromNodeId: docks.id, toNodeId: manhattan.id,
      edgeType: 'contains', description: '', properties: []
    });
    console.error('FAIL: cycle was not rejected');
  } catch (e) {
    console.log('OK: cycle rejected →', e);
  }
})();
```

Expected: logs `OK: cycle rejected → ... cycle detected ...`.

- [ ] **Step 6: Duplicate edge (same type) should be rejected**

```js
(async () => {
  const [c] = await invoke('list_chronicles', {});
  const nodes = await invoke('list_nodes', { chronicleId: c.id, typeFilter: null });
  const manhattan = nodes.find(n => n.label === 'Manhattan');
  const docks     = nodes.find(n => n.label === 'The Docks');
  try {
    await invoke('create_edge', {
      chronicleId: c.id, fromNodeId: manhattan.id, toNodeId: docks.id,
      edgeType: 'contains', description: '', properties: []
    });
    console.error('FAIL: duplicate edge was not rejected');
  } catch (e) {
    console.log('OK: duplicate rejected →', e);
  }
})();
```

Expected: logs `OK: duplicate rejected →` followed by a UNIQUE constraint error.

- [ ] **Step 7: Second contains-parent should be rejected**

```js
(async () => {
  const [c] = await invoke('list_chronicles', {});
  const nodes = await invoke('list_nodes', { chronicleId: c.id, typeFilter: null });
  const docks = nodes.find(n => n.label === 'The Docks');

  // Create a second would-be parent.
  const brooklyn = await invoke('create_node', {
    chronicleId: c.id, nodeType: 'area', label: 'Brooklyn',
    description: '', tags: [], properties: []
  });

  try {
    await invoke('create_edge', {
      chronicleId: c.id, fromNodeId: brooklyn.id, toNodeId: docks.id,
      edgeType: 'contains', description: '', properties: []
    });
    console.error('FAIL: second contains-parent was not rejected');
  } catch (e) {
    console.log('OK: second parent rejected →', e);
  }
})();
```

Expected: logs `OK: second parent rejected →` followed by a UNIQUE INDEX error (`idx_edges_contains_single_parent`).

- [ ] **Step 8: Malformed field value should be rejected**

```js
(async () => {
  const [c] = await invoke('list_chronicles', {});
  try {
    await invoke('create_node', {
      chronicleId: c.id, nodeType: 'character', label: 'Bad',
      description: '', tags: [],
      properties: [{ name: 'influence', type: 'number', value: 'not-a-number' }]
    });
    console.error('FAIL: bad field not rejected');
  } catch (e) {
    console.log('OK: bad field rejected →', e);
  }
})();
```

Expected: logs `OK: bad field rejected →` with a deserialization error.

- [ ] **Step 9: Cascade delete check**

```js
(async () => {
  const [c] = await invoke('list_chronicles', {});
  await invoke('delete_chronicle', { id: c.id });
  const remaining = await invoke('list_chronicles', {});
  const nodesNow = await invoke('list_nodes', { chronicleId: c.id, typeFilter: null });
  console.log('chronicles remaining:', remaining.length,
              '| nodes in deleted chronicle:', nodesNow.length);
})();
```

Expected: `chronicles remaining: 0 | nodes in deleted chronicle: 0`. This confirms `ON DELETE CASCADE` is actually firing (proves `PRAGMA foreign_keys = ON` from Task 2 is in effect).

- [ ] **Step 10: Close the app**

Close the Tauri window. No commit — smoke test is complete. If any step failed, reopen the relevant task, fix, recommit, re-run.

---

## Acceptance Criteria

Phase 1 is complete when all of the following are true. This mirrors the spec's acceptance list.

- [ ] Migration `0002_chronicle_graph.sql` creates the three tables, five indexes, one partial unique index, and three triggers as specified.
- [ ] `PRAGMA foreign_keys = ON` is active on the pool (verified by the cascade test in Task 13 Step 9).
- [ ] All Rust types (`Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`) compile and round-trip through JSON correctly (verified by Task 5 Step 2's property-round-trip test).
- [ ] All 20 Tauri commands are registered in `lib.rs` and callable from the frontend (verified by the smoke-test happy path in Task 13 Step 3).
- [ ] All five derived tree queries return correct results (verified by Task 6 Step 3's 8 tree-query tests).
- [ ] Cycle-safety 32-depth cap protects against runaway recursion (implicit in the recursive-CTE guards; not separately tested).
- [ ] Cycle prevention on `contains` edge creation rejects back-edges with a clear error (Task 7 Step 3 `cycle_rejected` test + Task 13 Step 5 smoke test).
- [ ] Single-parent enforcement under `contains` rejects a second contains-parent (Task 7 Step 3 `second_contains_parent_rejected` test + Task 13 Step 7 smoke test).
- [ ] `src/types.ts` types match the Rust structs and `npm run check` passes (Task 9 Step 2).
- [ ] `docs/design/data-sources.md`, `docs/design/data-sources-kumu.json`, and `CLAUDE.md` are updated (Tasks 10–12).
- [ ] Manual smoke-test procedure passes end-to-end, including all negative-path cases (Task 13).
