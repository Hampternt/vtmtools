# Domains Manager Design

**Date:** 2026-04-18
**Status:** Draft
**Scope:** Backend-only — SQLite schema, Rust types, Tauri commands, documentation. UI is deferred to a later phase.

---

## Overview

Add a new tool to vtmtools — **Domains Manager** — that lets the Storyteller model a VTM chronicle as a graph of typed nodes connected by typed edges. A "node" is any discrete thing in the chronicle (a geographic area, a Kindred, an institution, a division, a merit, an influence holding, a business venture), and an "edge" is any typed relationship between two nodes (`contains`, `controls`, `has-influence-in`, `allied-with`, etc.). Hierarchy is derived from edges at read time, not hardcoded into the schema.

The design is heavily inspired by [Kumu.io's architecture](https://docs.kumu.io/overview/kumus-architecture) — elements + typed connections + user-definable fields — adapted for a single-user local desktop tool backed by SQLite. Key Kumu lessons adopted: the master-list-vs-views separation, universal profiles (no per-type schema), typed fields created ad-hoc, and hierarchy expressed via connections rather than native containment.

Phase 1 (this spec) ships backend only. No UI, no sidebar entry, no Svelte component. A future phase will build the GM-facing UI (drilldown navigator, node editor, graph view) against the locked backend contract defined here.

---

## Scope

### In Scope (Phase 1)

- A new SQLite migration (`0002_chronicle_graph.sql`) creating three tables: `chronicles`, `nodes`, `edges`.
- Rust types: `Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `EdgeDirection`.
- Twenty Tauri commands covering chronicle/node/edge CRUD plus five derived tree queries (parent, children, siblings, path-to-root, subtree).
- TypeScript type definitions in `src/types.ts` mirroring the Rust structs.
- Updates to `docs/design/data-sources.md`, `docs/design/data-sources-kumu.json`, and `CLAUDE.md`.
- Manual smoke-test procedure executed via Tauri devtools.

### Out of Scope (deferred to later phases)

- Any UI. The tool is not added to `tools.ts` yet; no Svelte component is created.
- Graph visualization (Kumu-style mindmap view).
- Drilldown UI (breadcrumb + siblings strip + children list).
- Cluster-by-field (deriving virtual parent nodes from shared property values).
- Tag normalization (promoting `tags_json` to a `node_tags` table).
- Full-text or faceted search.
- Node-type registry (UI hints for suggested fields per type).
- Markdown sanitizer integration (frontend concern).
- Roll20 export ("print game status to Roll20").
- Undo/redo, edit history, soft deletes.

### Out of Scope Forever

- Multi-user / collaborative editing. This is a single GM's local tool.
- Cloud sync. All data local.
- Historical versioning. Deletes are permanent.
- "Loops" as a separate primitive (Kumu's third concept). Cyclical connection patterns are derivable from edges at query time if ever needed; no new table or type required.

---

## Semantic Model

Four primitives.

### Chronicle

A running game. Contains all nodes, edges, and metadata for that campaign. Chronicles are fully isolated from each other — the app can hold many chronicles; the user picks one to work within. Analogous to Kumu's "project."

### Node

Any discrete thing in a chronicle. Every node has:

- A `type` — freeform string (`"area"`, `"character"`, `"institution"`, `"business"`, `"domain"`, `"merit"`, or anything the user invents).
- A `label` — display name.
- A `description` — markdown body.
- Free-form string `tags` for cross-cutting categorization.
- A typed `properties` bag — array of named fields with declared types.

`type` is *descriptive*, not structural. A "character" and an "area" are structurally identical rows; `type` influences UI rendering and filtering but imposes no schema constraint on fields.

### Edge

A typed, directional relationship between two nodes. Every edge has:

- An `edge_type` — freeform string (`"contains"`, `"controls"`, `"adjacent-to"`, `"has-influence-in"`, `"allied-with"`, etc.).
- A `from_node_id` and `to_node_id`.
- Optional `description` and a typed `properties` bag.

One edge type is **privileged by convention**: `"contains"`. The UI treats it as the navigation/containment edge — breadcrumbs walk it, drilldown follows it, siblings derive from it. The database does not enforce this; `"contains"` is just another value in the `edge_type` column. All other edge types are equal at the schema level.

### Tag

A free-form string label on a node. Used for cross-cutting categorization that does not fit the hierarchy (e.g., `"waterfront"`, `"Anarch-held"`, `"contested"`). Stored as a JSON array on each node; promotable to a normalized table in a future phase if filter performance demands it.

### Why this model

- **Mixed trees work naturally.** Geographic and organizational hierarchies interleave freely because `contains` does not enforce an axis. NYPD (institution) can contain Precinct 17 (institution) which contains Holding Cells (area) — one `contains` edge type spans all three transitions.
- **User-invented mechanics require no schema change.** Node types and edge types are strings; a new concept is just a new value.
- **One query language for the whole chronicle.** "What does Elias control?" = outgoing `controls` edges from Elias. "What's inside Manhattan?" = outgoing `contains` edges from Manhattan. "Who has influence in the Docks?" = incoming `has-influence-in` edges to the Docks. Same traversal shape; different filter.
- **Extension is strictly additive.** Phase 2+ UI features (graph view, clustering, filtering) are new queries over the same schema, not migrations.

### What the model explicitly does *not* do

- No enforced per-type schemas. An `"area"` node is not required to have certain fields. Kumu avoids this; so do we.
- No required root. Any node with no incoming `contains` edge is effectively a root; adding a layer above just creates a new node and repoints.
- No label uniqueness. Two nodes may share a label; their integer `id` disambiguates. (Kumu's "find and reuse by matching label+type" behavior is avoided — it can silently merge things that shouldn't be merged.)
- No self-loops (enforced via `CHECK`).
- No duplicate edges of the same type between the same two nodes (enforced via `UNIQUE`).
- No multiple `contains` parents. A node has at most one parent under `contains`, making the `contains` sub-graph a strict tree (forest, really). Other edge types are unconstrained — a node can legitimately have many `controls`, `has-influence-in`, or `allied-with` relations.

---

## Repository Layout

New and modified files:

```
vtmtools/
├── src/
│   └── types.ts                            # modified: append Chronicle, ChronicleNode, ChronicleEdge, Field, FieldValue, EdgeDirection
├── src-tauri/
│   ├── migrations/
│   │   └── 0002_chronicle_graph.sql        # new: schema for chronicles, nodes, edges
│   └── src/
│       ├── lib.rs                          # modified: register new Tauri commands; verify PRAGMA foreign_keys = ON
│       ├── shared/
│       │   └── types.rs                    # modified: append Chronicle, Node, Edge, Field, FieldValue, EdgeDirection
│       └── db/
│           ├── mod.rs                      # modified: add pub mod chronicle; pub mod node; pub mod edge;
│           ├── chronicle.rs                # new: chronicles CRUD + 5 Tauri commands
│           ├── node.rs                     # new: nodes CRUD + derived tree queries + 10 Tauri commands
│           └── edge.rs                     # new: edges CRUD + 5 Tauri commands
└── docs/
    └── design/
        ├── data-sources.md                 # modified: add Chronicle Store section
        └── data-sources-kumu.json          # modified: add Chronicle Store element, Domains Manager element, and 2 connections
```

No new dependencies — all required crates (`sqlx`, `serde`, `serde_json`, `tokio`) are already in `Cargo.toml` from the Dyscrasia Store implementation.

---

## Database Schema

**Migration file:** `src-tauri/migrations/0002_chronicle_graph.sql`

```sql
-- Chronicles: one per running game. Deleting a chronicle cascades to its nodes and edges.
CREATE TABLE IF NOT EXISTS chronicles (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT    NOT NULL,
    description  TEXT    NOT NULL DEFAULT '',
    created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
);

-- Nodes: any discrete thing in a chronicle. `type` is a freeform user-chosen string.
-- `tags_json` is a JSON array of strings. `properties_json` is a JSON array of typed
-- Field records (see Rust Types section).
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

-- Edges: typed directional relationships between nodes. `edge_type` is a freeform
-- user-chosen string; `"contains"` is the UI's drilldown convention but the DB imposes
-- no special meaning on it.
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
-- Other edge types (controls, adjacent-to, etc.) have no such restriction.
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

### Schema design notes

- **`ON DELETE CASCADE` everywhere.** Deleting a chronicle cascades to all its nodes and edges; deleting a node cascades to all edges it's part of. This requires `PRAGMA foreign_keys = ON` on the pool — a startup-time check must verify this is set in `lib.rs` (add if missing).
- **`UNIQUE(from_node_id, to_node_id, edge_type)`** prevents duplicate edges of the same type between the same pair. Different types between the same pair are allowed (e.g., `contains` and `adjacent-to` can coexist).
- **Partial unique index `idx_edges_contains_single_parent`** enforces that any given node has at most one incoming `contains` edge — i.e. at most one parent. This makes the `contains` sub-graph a strict tree (actually a forest, since any node with no incoming `contains` edge is a root). Other edge types are unrestricted and may freely form multi-source, multi-target, and cyclic patterns. This is what keeps `get_parent_of -> Option<Node>` and breadcrumb navigation unambiguous.
- **`CHECK (from_node_id != to_node_id)`** forbids self-loops. Cycles across multiple nodes cannot be cheaply enforced in SQL and are prevented at the application layer instead (see Validation).
- **JSON stored as `TEXT`.** SQLite has no native JSON type; text + `json_extract()` is the standard idiom. Validation happens in the Rust serde layer, not the DB.
- **No seed data.** All chronicles and nodes are user-authored. The Dyscrasia Store's `seed.rs` pattern does not apply here.

---

## Rust Types

Appended to `src-tauri/src/shared/types.rs`.

```rust
use serde::{Deserialize, Serialize};

/// A running game. Contains nodes and edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chronicle {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,  // ISO-8601 from SQLite's datetime('now')
    pub updated_at: String,
}

/// A typed field value. The serde `tag = "type"` attribute means the JSON
/// discriminator field chooses which variant is parsed, and a value of the
/// wrong type fails to deserialize automatically — no manual validation
/// code needed.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FieldValue {
    String    { value: StringFieldValue },
    Text      { value: String           },
    Number    { value: NumberFieldValue },
    Date      { value: String           },  // ISO-8601 date
    Url       { value: String           },
    Email     { value: String           },
    Bool      { value: bool             },
    Reference { value: i64              },  // node id in same chronicle
}

/// Single-or-multi string. Serialized untagged: a raw string for single,
/// an array for multi.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringFieldValue {
    Single(String),
    Multi(Vec<String>),
}

/// Single-or-multi number.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberFieldValue {
    Single(f64),
    Multi(Vec<f64>),
}

/// A named, typed field. JSON shape (example):
///   {"name": "influence_rating", "type": "number", "value": 3}
#[derive(Clone, Debug, Serialize, Deserialize)]
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
    pub tags: Vec<String>,           // serialized to/from tags_json column
    pub properties: Vec<Field>,      // serialized to/from properties_json column
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

/// Direction parameter for list_edges_for_node.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeDirection { In, Out, Both }
```

### Wire format example

A node's `properties` round-trips through JSON as:

```json
[
  {"name": "influence_rating", "type": "number",    "value": 3},
  {"name": "faction_notes",    "type": "text",      "value": "has beef with the Prince"},
  {"name": "known_aliases",    "type": "string",    "value": ["Nightjar", "V"]},
  {"name": "controlled_by",    "type": "reference", "value": 47}
]
```

A payload with `"type": "number"` and `"value": "not-a-number"` fails to deserialize — serde's discriminator-driven variant selection catches the mismatch as a `DeserializeError`, which the Tauri command returns as an `Err(String)`.

---

## Tauri Commands

All commands live in `src-tauri/src/db/chronicle.rs`, `node.rs`, and `edge.rs`, following the single-file-per-table pattern established by `dyscrasia.rs`. All are registered in `lib.rs` via `.invoke_handler(tauri::generate_handler![...])`.

Parameter types below are illustrative; actual signatures use `tauri::State<'_, SqlitePool>` for the pool parameter in the command wrappers, mirroring `dyscrasia.rs`.

### Chronicles (`db/chronicle.rs`)

```rust
#[tauri::command] list_chronicles() -> Result<Vec<Chronicle>, String>
#[tauri::command] get_chronicle(id: i64) -> Result<Chronicle, String>
#[tauri::command] create_chronicle(name: String, description: String) -> Result<Chronicle, String>
#[tauri::command] update_chronicle(id: i64, name: String, description: String) -> Result<Chronicle, String>
#[tauri::command] delete_chronicle(id: i64) -> Result<(), String>
```

### Nodes (`db/node.rs`)

```rust
// Basic CRUD
#[tauri::command] list_nodes(chronicle_id: i64, type_filter: Option<String>) -> Result<Vec<Node>, String>
#[tauri::command] get_node(id: i64) -> Result<Node, String>
#[tauri::command] create_node(
    chronicle_id: i64,
    node_type:    String,
    label:        String,
    description:  String,
    tags:         Vec<String>,
    properties:   Vec<Field>,
) -> Result<Node, String>
#[tauri::command] update_node(
    id:           i64,
    node_type:    String,
    label:        String,
    description:  String,
    tags:         Vec<String>,
    properties:   Vec<Field>,
) -> Result<Node, String>
#[tauri::command] delete_node(id: i64) -> Result<(), String>

// Derived tree queries (follow edge_type = 'contains')
#[tauri::command] get_parent_of(node_id: i64) -> Result<Option<Node>, String>
#[tauri::command] get_children_of(node_id: i64) -> Result<Vec<Node>, String>
#[tauri::command] get_siblings_of(node_id: i64) -> Result<Vec<Node>, String>
#[tauri::command] get_path_to_root(node_id: i64) -> Result<Vec<Node>, String>          // ancestors, bottom-up
#[tauri::command] get_subtree(node_id: i64, max_depth: Option<i32>) -> Result<Vec<Node>, String>  // descendants
```

### Edges (`db/edge.rs`)

```rust
#[tauri::command] list_edges(chronicle_id: i64, edge_type_filter: Option<String>) -> Result<Vec<Edge>, String>
#[tauri::command] list_edges_for_node(
    node_id:          i64,
    direction:        EdgeDirection,
    edge_type_filter: Option<String>,
) -> Result<Vec<Edge>, String>
#[tauri::command] create_edge(
    chronicle_id: i64,
    from_node_id: i64,
    to_node_id:   i64,
    edge_type:    String,
    description:  String,
    properties:   Vec<Field>,
) -> Result<Edge, String>
#[tauri::command] update_edge(
    id:           i64,
    edge_type:    String,
    description:  String,
    properties:   Vec<Field>,
) -> Result<Edge, String>
#[tauri::command] delete_edge(id: i64) -> Result<(), String>
```

### Example: derived query `get_path_to_root`

The five derived queries all use recursive CTEs in the same shape; `get_path_to_root` is the canonical example:

```sql
WITH RECURSIVE ancestors(id, chronicle_id, type, label, description, tags_json, properties_json, created_at, updated_at, depth) AS (
    -- seed row: the starting node
    SELECT n.*, 0 FROM nodes n WHERE n.id = ?

    UNION ALL

    -- step: walk up via `contains` edges; the current ancestor's id matches the
    -- to_node_id of a contains-edge, and the from_node_id is the next ancestor
    SELECT n.*, a.depth + 1
    FROM nodes n
    JOIN edges e ON e.from_node_id = n.id
    JOIN ancestors a ON a.id = e.to_node_id
    WHERE e.edge_type = 'contains'
      AND a.depth < 32    -- cycle-safety cap
)
SELECT * FROM ancestors WHERE depth > 0 ORDER BY depth ASC;
```

`get_subtree` is the mirror image (swap `from_node_id` and `to_node_id` in the step clause). `get_children_of` and `get_siblings_of` are single-level non-recursive queries.

---

## Validation & Safety

### What's enforced where

| Concern | Layer | Mechanism |
|---|---|---|
| SQL injection | Rust | `sqlx` parameterized queries (`.bind()`) throughout. User text never concatenated into SQL. |
| Field-value type mismatch | Rust | serde deserialization of `FieldValue` with `tag = "type"`. Mismatched payloads fail at deserialize time. |
| JSON structural corruption | Rust | `serde_json::from_str` validates on read; `to_string` escapes on write. |
| Self-loop (node referencing itself) | DB | `CHECK (from_node_id != to_node_id)` on `edges`. |
| Duplicate edges (same pair, same type) | DB | `UNIQUE(from_node_id, to_node_id, edge_type)` on `edges`. |
| Multiple `contains` parents for one node | DB | Partial unique index on `edges(to_node_id) WHERE edge_type = 'contains'`. Enforces strict-tree topology under `contains`. |
| Cascade on deletion | DB | `ON DELETE CASCADE` foreign keys. Requires `PRAGMA foreign_keys = ON`. |
| Cycle in `contains` edges | Rust + SQL | Application-layer check before creating a `contains` edge; SQL depth cap (32) on recursive CTEs as runtime safety net. |
| Markdown XSS in descriptions | Frontend (deferred) | Must sanitize when rendering. Backend stores raw text untouched. |

### Cycle prevention for `contains` edges

Before creating a `contains` edge from `A` to `B`, the `create_edge` command must verify that `A` is not already a descendant of `B`. This is a single recursive-CTE check (reusable from `get_subtree(B)` logic) confirming `A` is not in `B`'s subtree. If the check fails, the command returns an `Err("cycle detected")`. This prevents cycles at author time.

The 32-depth cap in recursive CTEs is the safety net against any cycle that somehow bypasses the author-time check, or against deep legitimate trees.

The cycle check applies only to `edge_type = 'contains'`. Other edge types (`allied-with`, `at-war-with`, etc.) may legitimately form cycles and no check is performed for them.

### Foreign-key enforcement

SQLite does not enforce foreign keys by default. The pool must issue `PRAGMA foreign_keys = ON` immediately after opening. The existing `lib.rs` may already do this for the Dyscrasia Store; if not, it must be added before running migration `0002`. This is verified as part of the implementation's startup path.

---

## TypeScript Types

Appended to `src/types.ts`:

```ts
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

`ChronicleNode` and `ChronicleEdge` are prefixed to avoid collision with the DOM's global `Node` and `Edge` types in Svelte templates.

---

## Documentation Updates

### `docs/design/data-sources.md`

Insert a new "Chronicle Store" entry between the Dyscrasia Store and Roll20 Live Feed sections:

> ## Chronicle Store
>
> **Direction:** Both (app reads and writes)
> **What it is:** The SQLite tables holding all user-authored chronicle data — geographic/organizational areas, characters, businesses, merits, influence holdings, and typed relationships between them. Split across three tables: `chronicles`, `nodes`, `edges`.
>
> **Data carried per chronicle:** name, description, timestamps.
>
> **Data carried per node:** type (freeform user-chosen string), label, markdown description, cross-cutting tags, and a typed property bag (array of named fields with declared types: string, text, number, date, url, email, bool, reference).
>
> **Data carried per edge:** type (freeform user-chosen string), from-node, to-node, markdown description, typed property bag.
>
> **Behavior:**
> - Nothing is seeded; all data is user-created.
> - The `"contains"` edge type is treated by the UI as the navigation relationship (breadcrumbs and drilldown walk it), but the DB does not privilege it.
> - Deleting a chronicle cascades to all its nodes and edges. Deleting a node cascades to all its edges.
>
> **Currently used by:** Domains Manager (full CRUD + derived tree queries).

### `docs/design/data-sources-kumu.json`

Add two elements to the `elements` array:

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

Add two connections to the `connections` array:

```json
{ "from": "Storyteller",     "to": "Domains Manager",  "type": "manages",          "direction": "directed" },
{ "from": "Domains Manager", "to": "Chronicle Store",  "type": "reads and writes", "direction": "mutual"   }
```

### `CLAUDE.md`

Append to the `### Database` section, after the existing dyscrasias paragraph:

> Migration `0002_chronicle_graph.sql` adds three tables for the Domains Manager tool: `chronicles` (one row per running game), `nodes` (any discrete thing — area, character, institution, business, merit), and `edges` (typed directional relationships between nodes). `nodes.type` and `edges.edge_type` are freeform user-authored strings; no enum enforcement. Custom fields live in `nodes.properties_json` / `edges.properties_json` as a JSON array of typed Field records (each `{name, type, value}`). Deleting a chronicle cascades to its nodes and edges; deleting a node cascades to its edges. The `"contains"` edge type is the UI's convention for hierarchy/drilldown but the DB does not enforce it.

---

## Manual Validation Procedure

Matching project convention (no test suites; `npm run check` + `cargo check` are the correctness gates):

1. `cargo check --manifest-path src-tauri/Cargo.toml` — Rust compiles including new types and commands.
2. `npm run check` — TypeScript types in `src/types.ts` are valid.
3. `npm run tauri dev` — app launches; migration `0002` runs cleanly against a fresh DB.
4. From the Tauri devtools console, exercise the happy path:

   ```js
   const chronicle = await window.__TAURI__.core.invoke('create_chronicle', {
     name: 'Smoke Test', description: ''
   });
   const manhattan = await window.__TAURI__.core.invoke('create_node', {
     chronicleId: chronicle.id, nodeType: 'area', label: 'Manhattan',
     description: '', tags: ['geographic'], properties: []
   });
   const docks = await window.__TAURI__.core.invoke('create_node', {
     chronicleId: chronicle.id, nodeType: 'area', label: 'The Docks',
     description: '', tags: ['waterfront'], properties: []
   });
   await window.__TAURI__.core.invoke('create_edge', {
     chronicleId: chronicle.id, fromNodeId: manhattan.id, toNodeId: docks.id,
     edgeType: 'contains', description: '', properties: []
   });
   const children = await window.__TAURI__.core.invoke('get_children_of', {
     nodeId: manhattan.id
   });
   console.log(children);  // expect: [{ ...docks... }]
   ```

5. Negative path:
   - Attempt to create a self-referential `contains` edge (`fromNodeId === toNodeId`) → expect `CHECK` constraint error.
   - Create A contains B, then attempt B contains A → expect `cycle detected` error from the application-layer check.
   - Attempt to create a duplicate edge of the same type between the same pair → expect `UNIQUE` constraint error.
   - Create two separate nodes that both attempt to `contains` the same child node → expect `UNIQUE` constraint error from the partial index (at most one contains-parent per node).
   - Send a `create_node` payload with a malformed field (e.g., `{"type": "number", "value": "not-a-number"}`) → expect deserialize error returned as `Err(String)`.

---

## Acceptance Criteria

Phase 1 is complete when:

- [ ] Migration `0002_chronicle_graph.sql` creates the three tables, indexes, and triggers as specified.
- [ ] `PRAGMA foreign_keys = ON` is confirmed set on pool startup.
- [ ] All Rust types (`Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`) compile and round-trip through JSON correctly.
- [ ] All 20 Tauri commands are registered and callable.
- [ ] All five derived tree queries return correct results, including with the 32-depth cycle-safety cap.
- [ ] Cycle prevention on `contains` edge creation works (a cycle attempt is rejected with a clear error).
- [ ] Single-parent enforcement under `contains` works (a second `contains` edge into the same child is rejected by the partial unique index).
- [ ] `src/types.ts` types match the Rust structs and `npm run check` passes.
- [ ] `docs/design/data-sources.md`, `docs/design/data-sources-kumu.json`, and `CLAUDE.md` are updated.
- [ ] The manual smoke-test procedure above passes end-to-end, including all negative-path cases.

---

## References

- [Kumu's Architecture](https://docs.kumu.io/overview/kumus-architecture) — the conceptual model this design is inspired by.
- [Kumu Fields](https://docs.kumu.io/guides/fields) — source of the typed-field vocabulary.
- [Kumu Clustering](https://docs.kumu.io/guides/clustering) — the future-phase pattern for deriving connections from field values.
- `docs/design/data-sources.md` — project convention for naming and describing data sources.
- `src-tauri/src/db/dyscrasia.rs` — reference implementation of the single-file-per-table CRUD-plus-commands pattern this design follows.
