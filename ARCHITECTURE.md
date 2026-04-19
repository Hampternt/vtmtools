# ARCHITECTURE.md

> Canonical cross-feature reference for vtmtools. Load this file for any
> code change. Feature-specific details live in `docs/superpowers/specs/`
> and `docs/superpowers/plans/`; historical decisions live in
> `docs/adr/`; stable reference knowledge lives in `docs/reference/`.

---

## §1 Overview & stack

vtmtools is a desktop-first, single-GM tool for running Vampire: The
Masquerade 5th Edition. The stack is Tauri 2 + SvelteKit (static SPA,
no SSR) + a Rust backend backed by SQLite, running fully offline on
one machine. The UI is dark-only, with no cloud sync, no multi-user
operation, and no network surface beyond a single localhost Roll20
bridge. See [ADR 0001](docs/adr/0001-tauri-2-stack.md) for the stack
decision and [ADR 0004](docs/adr/0004-dark-only-theming.md) for the
theming posture.

## §2 Domain model

The canonical data shapes. Features that produce or consume any of
these must honor the shape defined here.

### Dyscrasia domain

```rust
/// A Dyscrasia entry from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DyscrasiaEntry {
    pub id: i64,
    pub resonance_type: ResonanceType,
    pub name: String,
    pub description: String,
    pub bonus: String,
    pub is_custom: bool,
}
```

A single dyscrasia row — a typed name + effect + short mechanical
bonus tag, tied to one resonance type. Built-in rows ship with
`is_custom = 0`; user-authored rows ship with `is_custom = 1` and
are preserved across reseeds.

```sql
CREATE TABLE IF NOT EXISTS dyscrasias (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resonance_type  TEXT    NOT NULL CHECK(resonance_type IN ('Phlegmatic','Melancholy','Choleric','Sanguine')),
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL,
    bonus           TEXT    NOT NULL,
    is_custom       INTEGER NOT NULL DEFAULT 0
);
```

The on-disk schema. The `resonance_type` CHECK constraint enforces
the spelling `'Melancholy'` (not `'Melancholic'`) — see §6
Invariants.

### Dice / resonance domain

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Temperament {
    Negligible,
    Fleeting,
    Intense,
}
```

The three buckets a temperament die falls into after thresholding.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResonanceType {
    Phlegmatic,
    Melancholy,
    Choleric,
    Sanguine,
}
```

The four VTM 5e resonance types. Shared by the dice roller, the
dyscrasia table, and the frontend display.

```rust
/// 7-step slider level for resonance type weighting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SliderLevel {
    Impossible,
    ExtremelyUnlikely,
    Unlikely,
    Neutral,
    Likely,
    ExtremelyLikely,
    Guaranteed,
}
```

GM-facing slider positions. Each maps to a multiplier applied against
the base probability of a resonance type (`Impossible` → 0,
`Guaranteed` → ∞, `Neutral` → 1.0).

```rust
/// GM-configurable temperament roll options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemperamentConfig {
    /// How many d10s to roll (1–5). Result is best or worst of the pool.
    pub dice_count: u8,
    /// true = take highest die (biases toward Intense), false = take lowest
    pub take_highest: bool,
    /// Upper bound (inclusive) for Negligible result. Default 5.
    pub negligible_max: u8,
    /// Upper bound (inclusive) for Fleeting result. Default 8. Intense = above this.
    pub fleeting_max: u8,
}
```

GM-tunable knobs for the temperament roll. Shared between the Rust
roller and the frontend GM Roll Config UI.

```rust
/// GM-configurable weighting for resonance type selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResonanceWeights {
    pub phlegmatic: SliderLevel,
    pub melancholy: SliderLevel,
    pub choleric: SliderLevel,
    pub sanguine: SliderLevel,
}
```

Per-type slider positions. Fed into the weighted resonance picker.

```rust
/// Full GM config passed to a roll
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollConfig {
    pub temperament: TemperamentConfig,
    pub weights: ResonanceWeights,
}
```

The bundle the frontend sends into `roll_resonance`.

```rust
/// Full result of one resonance roll sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResonanceRollResult {
    /// All dice rolled for temperament (for display)
    pub temperament_dice: Vec<u8>,
    /// The die value that determined temperament
    pub temperament_die: u8,
    pub temperament: Temperament,
    /// Set if temperament is Fleeting or Intense
    pub resonance_type: Option<ResonanceType>,
    /// d10 rolled for resonance (display only — weighted pick determines actual result)
    pub resonance_die: Option<u8>,
    /// Set if temperament is Intense
    pub acute_die: Option<u8>,
    pub is_acute: bool,
    /// Populated after GM rolls/picks Dyscrasia (not auto-populated here)
    pub dyscrasia: Option<DyscrasiaEntry>,
}
```

The complete payload returned from one roll sequence. Each die is
included for display; consumers never re-roll to reconstruct.

### Chronicle graph domain

```rust
/// A running game. Contains nodes and edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Chronicle {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}
```

A single running game. Timestamps are ISO-8601 strings produced by
SQLite's `datetime('now')`.

```rust
/// Single-or-multi string value. Serialized untagged: a raw string for single,
/// an array for multi.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum StringFieldValue {
    Single(String),
    Multi(Vec<String>),
}
```

```rust
/// Single-or-multi number value.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum NumberFieldValue {
    Single(f64),
    Multi(Vec<f64>),
}
```

```rust
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
```

```rust
/// A named, typed field. JSON shape example:
///   {"name": "influence_rating", "type": "number", "value": 3}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    #[serde(flatten)]
    pub value: FieldValue,
}
```

The typed property model for nodes and edges. Serde's tag + flatten
combination means the JSON form is `{"name": "...", "type": "...",
"value": ...}`; wrong-type values fail to deserialize at the IPC
boundary.

```rust
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
```

```rust
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
```

```rust
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeDirection {
    In,
    Out,
    Both,
}
```

Nodes and edges form the chronicle graph. `Node.node_type` and
`Edge.edge_type` are freeform user-authored strings — the backend
imposes no enum — see [ADR 0003](docs/adr/0003-freeform-node-edge-types.md).
`EdgeDirection` is the filter used by queries that ask for edges
incident to a node.

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
```

The graph tables. `ON DELETE CASCADE` rules mean deleting a
chronicle deletes its nodes and edges; deleting a node deletes its
incident edges. The `(from_node_id, to_node_id, edge_type)` UNIQUE
constraint forbids duplicate same-type edges between the same two
nodes; a separate partial unique index enforces at most one
`contains` parent per node:

```sql
-- Enforce "strict tree under contains": a node may have at most one contains-parent.
-- Other edge types have no such restriction.
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_contains_single_parent
    ON edges(to_node_id) WHERE edge_type = 'contains';
```

### Roll20 domain

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub current: String,
    pub max: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub controlled_by: String,
    pub attributes: Vec<Attribute>,
}
```

The cached-character payload. The `roll20://characters-updated`
event carries `Vec<Character>` — the full cache, not a diff. The
frontend re-renders from the full list.

```rust
/// Inbound messages from the browser extension.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMsg {
    Characters { characters: Vec<Character> },
    CharacterUpdate { character: Character },
}

/// Outbound messages sent to the browser extension.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMsg {
    Refresh,
    SendChat { message: String },
    SetAttribute {
        character_id: String,
        name: String,
        value: String,
    },
}
```

Wire protocol with the browser extension. Tagged JSON with a `type`
discriminator; `Characters` carries the whole cache, `CharacterUpdate`
patches one entry. Outbound messages are driven by the matching
Tauri commands in `roll20/commands.rs`.

### Mirror layer

The frontend `src/types.ts` mirrors these shapes in TypeScript.
Drift is not tolerated — changing a Rust struct requires updating
the TS mirror in the same commit.

## §3 Storage strategy

- **SQLite** at `app.path().app_data_dir()` (file `vtmtools.db`).
  All durable application state lives here: `dyscrasias`,
  `chronicles`, `nodes`, `edges`.
- **Migrations** in `src-tauri/migrations/NNNN_*.sql`, applied on
  every startup via `sqlx::migrate!`. `PRAGMA foreign_keys = ON` is
  enabled on the pool via `SqliteConnectOptions` so `ON DELETE
  CASCADE` actually fires.
- **Seed policy:** `src-tauri/src/db/seed.rs` deletes all
  `is_custom = 0` rows in `dyscrasias` and reinserts the canonical
  set on every start. Intentional; see
  [ADR 0002](docs/adr/0002-destructive-reseed.md).
- **Ephemeral state:**
  - Svelte runes stores in `src/store/*` (e.g.
    `domains.svelte.ts` for chronicle UI state, `toolEvents.ts` for
    cross-tool pub/sub).
  - `Arc<Roll20State>` on the Rust side holds the Roll20 character
    cache (`HashMap<String, Character>`), connection flag, and the
    outbound-message mpsc sender. Shared between the WS loop and
    the Tauri command handlers.
- **Export artifacts:** Markdown written to `~/Documents/vtmtools/`
  via `tokio::fs` in `src-tauri/src/tools/export.rs`.
- **No cloud, no remote sync, no network storage** (see §12
  Non-goals).

## §4 I/O contracts

Named surfaces at module boundaries. Feature tasks that cross any of
these must honor the contract shapes declared here and must not bypass
the wrapper layers described below.

### Tauri IPC commands

Inventoried by module. A command's request/response shape (from its
Rust signature + the types it references in §2) is the stable contract.

- **`src-tauri/src/db/chronicle.rs`** (5):
  `list_chronicles`, `get_chronicle`, `create_chronicle`,
  `update_chronicle`, `delete_chronicle`.
- **`src-tauri/src/db/node.rs`** (10):
  `list_nodes`, `get_node`, `create_node`, `update_node`,
  `delete_node`, `get_parent_of`, `get_children_of`,
  `get_siblings_of`, `get_path_to_root`, `get_subtree`.
- **`src-tauri/src/db/edge.rs`** (5):
  `list_edges`, `list_edges_for_node`, `create_edge`,
  `update_edge`, `delete_edge`.
- **`src-tauri/src/db/dyscrasia.rs`** (5):
  `list_dyscrasias`, `add_dyscrasia`, `update_dyscrasia`,
  `delete_dyscrasia`, `roll_random_dyscrasia`.
- **`src-tauri/src/tools/resonance.rs`** (1): `roll_resonance`.
- **`src-tauri/src/tools/export.rs`** (1): `export_result_to_md`.
- **`src-tauri/src/roll20/commands.rs`** (5):
  `get_roll20_characters`, `get_roll20_status`,
  `refresh_roll20_data`, `send_roll20_chat`,
  `set_roll20_attribute`.

Total: 32 commands. New commands are registered in
`src-tauri/src/lib.rs` (`invoke_handler(tauri::generate_handler![...])`).
See §8 for the Tauri capability / ACL surface.

### Typed frontend API wrapper modules

Frontend components never call `invoke(...)` directly. IPC goes
through typed wrapper modules in `src/lib/**/api.ts` (see
`src/lib/domains/api.ts` for the reference implementation: one
exported function per Tauri command, return type matching the Rust
response). New tools adopt the same pattern.

### Roll20 WebSocket protocol

- Binding: `127.0.0.1:7423`, localhost-only (see §8 Security model,
  [ADR 0005](docs/adr/0005-roll20-ws-extension-bridge.md)).
- At most one active extension session. Connection owner is
  `src-tauri/src/roll20/mod.rs`.
- Message framing is JSON text frames. Inbound (extension → app)
  variants: `Characters { characters: Vec<Character> }` replaces
  the cache, `CharacterUpdate { character: Character }` upserts one
  entry by `id`. Outbound (app → extension) variants: `Refresh`,
  `SendChat { message }`, `SetAttribute { character_id, name, value }`.
  The exact Rust types are inlined in §2 under "Roll20 domain".

### Tauri events (backend → frontend)

| Event | Payload | Emitted when |
|---|---|---|
| `roll20://connected` | none | Extension opens WS connection |
| `roll20://disconnected` | none | Extension closes WS connection |
| `roll20://characters-updated` | `Vec<Character>` | Character cache refreshed |

### Svelte cross-tool pub/sub

`src/store/toolEvents.ts` exposes `publishEvent(event)` and a
`toolEvents` writable store. Subscribers are loose — document event
names + payload shapes near the publisher.

### Tools registry

`src/tools.ts` is THE add-a-tool seam. Adding an entry auto-wires the
sidebar and lazy-loaded component. The entry shape:

```ts
export interface Tool {
  id: string;
  label: string;
  icon: string; // emoji or SVG string
  component: () => Promise<{ default: Component<any> }>;
}
```

See §9 Extensibility seams.

## §5 Module boundaries

Forbidding rules. Violations are merge blockers.

- Only `src-tauri/src/db/*` talks to SQLite. No component or other
  backend module invokes `sqlx` or opens a connection.
- Only `src-tauri/src/roll20/*` talks to the WebSocket server. No
  other backend module binds a socket.
- Frontend components never import SQL drivers or call the database
  directly. Database access is exclusively via Tauri `invoke(...)`
  calls, and those go through the typed API wrappers in
  `src/lib/**/api.ts`.
- All Tauri commands that do I/O are `async` and use `tokio::fs`,
  never `std::fs`.
- Frontend tools (`src/tools/*.svelte`) never import another tool
  directly. Cross-tool coordination goes through `toolEvents` or
  shared stores (e.g. `src/store/domains.svelte.ts` for chronicle-
  aware tools).

## §6 Invariants

Properties that must hold across all features.

- `PRAGMA foreign_keys = ON` is enabled on the pool via
  `SqliteConnectOptions`. `ON DELETE CASCADE` depends on this.
- `resonance_type` uses the spelling `'Melancholy'` (not
  `'Melancholic'`). The DB CHECK constraint enforces this.
- Seed reconciliation on app start is destructive for
  `is_custom = 0` rows. Do not revert to a count-check guard
  ([ADR 0002](docs/adr/0002-destructive-reseed.md)).
- CSS colors use tokens from `:global(:root)` in
  `src/routes/+layout.svelte`. Hex is allowed only for transient
  states with no semantic token (hover intermediates, glow shadows).
  Token groups (as of this writing):
  - Text: `--text-primary`, `--text-label`, `--text-secondary`,
    `--text-muted`, `--text-ghost`.
  - Surfaces: `--bg-base`, `--bg-card`, `--bg-raised`, `--bg-input`,
    `--bg-sunken`, `--bg-active`.
  - Borders: `--border-faint`, `--border-card`, `--border-surface`,
    `--border-active`.
  - Accents: `--accent`, `--accent-bright`, `--accent-amber`.
  - Temperament: `--temp-negligible`, `--temp-negligible-dim`,
    `--temp-fleeting-dim`, `--temp-intense-dim`.
- Layout and typography sizes are in `rem`. Root font-size is
  `clamp(16px, 1.0vw, 32px)`; `rem` scales automatically. Never use
  `px` for font sizes or layout widths.
- Card grids use CSS Grid with `align-items: start`. Never use CSS
  multi-column — incompatible with `animate:flip`.
- Any element combining `width: 100%` with `padding` must set
  `box-sizing: border-box` (there is no global reset).
- In Svelte 5 runes mode, `in:` / `out:` transitions are placed on
  elements whose lifecycle is controlled by the enclosing `{#each}`
  or `{#if}`, not on runes-mode component roots. Use a plain wrapper
  `<div in:scale out:fade>` in the parent's `{#each}` block.
- Only one Roll20 extension session is active at a time. State is
  held in `Arc<Roll20State>` shared between the WS loop and the
  Tauri command handlers.
- Dark-only. No theme toggle exists or will be added
  ([ADR 0004](docs/adr/0004-dark-only-theming.md)).

## §7 Error handling

How failures propagate across the Rust ↔ Tauri IPC ↔ Svelte boundary.

- Rust commands return `Result<T, E>`. At the Tauri IPC boundary,
  the error type is serialized as `String` (Tauri rejects the
  frontend promise with that string). Prefix stable per-command
  identifiers where useful (e.g. `"db/dyscrasia.create: …"`) so the
  frontend can categorize without parsing free-form prose.
- Frontend catches rejected `invoke` promises in the typed API
  wrapper (`src/lib/**/api.ts`) or at the call site, and surfaces
  user-visible errors via toast / inline error state. Raw errors
  are logged to the console.
- Panics in command paths are bugs, not error flow. No `unwrap()`
  in production code; use `?` or explicit error mapping.
- WebSocket disconnect is expected flow, not an error. The
  `roll20://disconnected` event fires, the UI shifts to "not
  connected" state, and the next extension reconnect restores
  service.
- Database errors from `sqlx` propagate as `Err(String)` with
  module-stable prefixes. Migration failures on startup are fatal
  (the app exits with a user-visible error).

## §8 Security model

Trust boundaries and assumptions for a single-user local desktop tool.

- **Trust posture.** Single user, single machine. No authentication,
  no authorization, no user-level access control. These are non-
  goals (§12).
- **Network surface.** Exactly one listener: the Roll20 WebSocket
  on `127.0.0.1:7423`. It must never bind to `0.0.0.0` or any
  routable interface. No other external network call is made by
  the app.
- **Localhost WS trust model.** Any process running as the user can
  connect to `127.0.0.1:7423`. This is equivalent to trusting the
  user and is the intended posture. Do not add authentication to
  the WS without a specific threat that justifies it.
- **Tauri capabilities / ACL.** `src-tauri/capabilities/default.json`
  currently grants `core:default` and `opener:default`. Custom
  `#[tauri::command]` handlers registered via
  `invoke_handler(tauri::generate_handler![...])` in
  `src-tauri/src/lib.rs` are callable from the main window under
  this configuration. If a future change scopes capabilities more
  tightly (e.g. per-command allow/deny lists), this section must be
  updated in the same commit and the change noted in an ADR.
- **Filesystem write scope.** Writes are limited to
  `app.path().app_data_dir()` (for SQLite) and
  `~/Documents/vtmtools/` (for exports). Any new write path must
  be added to this list and justified.
- **Browser extension DOM-read surface.** The extension reads
  Roll20 DOM nodes that feed the `Character`/`Attribute` shape
  (§2). It never sends data outside the localhost WebSocket.
- **Secrets.** None. No API keys, tokens, or credentials in the
  app. If a future feature introduces one, this section is updated
  and an ADR is filed.

## §9 Extensibility seams

Named places to add things. Feature specs cite a seam instead of
inventing a new hook.

- **Add a tool.** Add one entry to `src/tools.ts`. Sidebar +
  lazy-loaded component wiring is automatic. Existing examples:
  `Resonance.svelte`, `DyscrasiaManager.svelte`, `Campaign.svelte`,
  `DomainsManager.svelte` — the pattern is stable.
- **Add a schema change.** Add a new
  `src-tauri/migrations/NNNN_*.sql` file; migrations run on app
  start. Mirror the shape change in `shared/types.rs` and
  `src/types.ts` in the same commit.
- **Add a node or edge type.** No code change. The chronicle graph
  uses freeform strings ([ADR 0003](docs/adr/0003-freeform-node-edge-types.md));
  the UI derives autocomplete from existing distinct values.
- **Add a Tauri command.** Declare in the relevant
  `src-tauri/src/**/commands.rs` (or the module's `mod.rs` for
  single-file modules), register in `src-tauri/src/lib.rs` inside
  the `invoke_handler(tauri::generate_handler![...])` list, revisit
  the capability JSON if a narrower ACL is in force (§8), then add
  a typed wrapper in `src/lib/**/api.ts`. Components call the
  wrapper, never `invoke(...)` directly.
- **Add a cross-tool event.** Publish via
  `src/store/toolEvents.ts`. Document the event name + payload
  shape near the publisher.
- **Add a property field type.** Extend the `FieldValue` enum in
  `src-tauri/src/shared/types.rs`, mirror it in `src/types.ts`,
  and register a widget in the Domains Manager property-editor
  registry. Existing variants: `string`, `text`, `number`,
  `date`, `url`, `email`, `bool`, `reference`. v1 UI widgets
  ship for `string`, `text`, `number`, `bool`; the other variants
  are extensibility seams whose widgets will follow.

## §10 Testing & verification

- Rust unit tests live as `#[cfg(test)] mod tests` inside each
  source file. Current test modules: `shared/dice.rs`,
  `shared/resonance.rs`, `db/dyscrasia.rs`, `db/chronicle.rs`,
  `db/node.rs`, `db/edge.rs`, `tools/export.rs`. (Run
  `grep -rn "#\[cfg(test)\]" src-tauri/src` to confirm current
  state before editing; `db/chronicle.rs` currently carries two
  `#[cfg(test)]` annotations.)
- No frontend test framework is installed. This is a deliberate
  current choice, not an oversight. Introducing one is a scope
  change to be raised explicitly.
- `./scripts/verify.sh` is the aggregate gate: runs `npm run
  check`, `cargo test`, and `npm run build`. All claims of "done"
  must be backed by a green run.
- Expected (non-regression) warnings — do NOT "fix" these:
  - `npm run build`: unused `listen` import in
    `src/tools/Campaign.svelte` and `src/tools/Resonance.svelte`.
    In-progress surface, not dead code.
  - `shared/types.rs`: `FieldValue` variants `Date`, `Url`,
    `Email`, and `Reference` may surface "never constructed" —
    the v1 Domains UI uses only `String`, `Text`, `Number`, and
    `Bool` widgets. The unused variants ship as extensibility
    seams for future property widgets (see §9). Do not remove.

## §11 Plan & execution conventions

Plans produced against this architecture are structured for parallel
sub-agent-driven execution (`superpowers:subagent-driven-development`,
`superpowers:dispatching-parallel-agents`).

Each plan task declares:

- **Files (create / modify / delete):** tight, explicit scope.
- **Anti-scope:** files the task MUST NOT touch. Prevents silent
  collisions when two sub-agents run concurrently.
- **Depends on:** predecessor task IDs, or `none` if independent.
- **Invariants cited:** pointers to specific ARCHITECTURE.md
  sections the task must honor.

Seams between parallel tasks are drawn along §4 I/O contracts. If
two tasks share a contract, the contract shape is settled in a
preliminary task before either implementation task starts; both
parallel tasks then work behind the frozen contract.

Verification gate: every sub-agent runs `./scripts/verify.sh`
before reporting success. Green output is required; self-reports
without verification are not accepted.

Isolation: prefer `superpowers:using-git-worktrees` for multi-agent
dispatch so concurrent edits don't collide in a shared working tree.

## §12 Non-goals

Explicitly out of scope. A feature spec that proposes any of these
must first raise a scope change; do not assume it's allowed.

- No multi-user or multi-tenant operation.
- No cloud sync, remote storage, or server-backed state.
- No light-mode, theme toggle, or configurable color scheme
  ([ADR 0004](docs/adr/0004-dark-only-theming.md)).
- No authentication or authorization of any kind.
- No network surface beyond the single `127.0.0.1:7423` WebSocket
  listener.
- No ingestion path for Roll20 data other than the browser extension
  bridge ([ADR 0005](docs/adr/0005-roll20-ws-extension-bridge.md)).
- No multi-session Roll20 support. One extension session at a time.
- No frontend testing framework.

## §13 ADR index

| # | Title | Status |
|---|---|---|
| 0001 | [Tauri 2 + SvelteKit + SQLite stack](docs/adr/0001-tauri-2-stack.md) | accepted |
| 0002 | [Destructive reseed of non-custom dyscrasias on startup](docs/adr/0002-destructive-reseed.md) | accepted |
| 0003 | [Freeform strings for nodes.type and edges.edge_type](docs/adr/0003-freeform-node-edge-types.md) | accepted |
| 0004 | [Dark-only theming](docs/adr/0004-dark-only-theming.md) | accepted |
| 0005 | [Roll20 integration via localhost WebSocket + browser extension](docs/adr/0005-roll20-ws-extension-bridge.md) | accepted |

Add new rows here as ADRs are written. When an ADR is superseded,
update its Status column to `superseded by NNNN`.
