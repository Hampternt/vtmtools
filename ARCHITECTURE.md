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
operation, and no network surface beyond two localhost VTT-bridge
listeners (`ws://127.0.0.1:7423` for Roll20, `wss://127.0.0.1:7424`
for Foundry — see [ADR 0006](docs/adr/0006-bridge-source-generalization.md)).
See [ADR 0001](docs/adr/0001-tauri-2-stack.md) for the stack
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

### Bridge domain

The bridge layer mirrors live character data from one or more VTT
sources (Roll20, Foundry) into a source-agnostic shape the frontend
consumes. Source-specific wire types live under
`src-tauri/src/bridge/<source>/types.rs`; the canonical shape is
defined once in `src-tauri/src/bridge/types.rs`. See
[ADR 0006](docs/adr/0006-bridge-source-generalization.md).

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind { Roll20, Foundry }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthTrack {
    pub max: u8,
    pub superficial: u8,
    pub aggravated: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalCharacter {
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub controlled_by: Option<String>,
    pub hunger: Option<u8>,
    pub health: Option<HealthTrack>,
    pub willpower: Option<HealthTrack>,
    pub humanity: Option<u8>,
    pub humanity_stains: Option<u8>,
    pub blood_potency: Option<u8>,
    /// Source-specific extras the canonical fields don't capture.
    /// Roll20: serialized `Character` with the raw attribute list.
    /// Foundry: serialized `FoundryActor` with the full system blob.
    pub raw: serde_json::Value,
}
```

The cached-character payload. The `bridge://characters-updated`
event carries `Vec<CanonicalCharacter>` — the merged cache across
all connected sources, not a diff. The frontend re-renders from
the full list.

```rust
/// Stateless protocol adapter — one impl per VTT in
/// `src-tauri/src/bridge/<source>/mod.rs`.
#[async_trait]
pub trait BridgeSource: Send + Sync {
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String>;
    fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Result<Value, String>;
    fn build_refresh(&self) -> Value;
}

/// One event emitted from a single inbound frame. A frame may yield zero,
/// one, or many events.
pub enum InboundEvent {
    /// Bulk truth from one source. The cache replaces this source's
    /// slice (drops every prior entry whose `source` matches, then
    /// inserts the new set). Empty `characters` is legal — "this source
    /// now has zero characters". Fires from Roll20 `characters` /
    /// Foundry `actors`.
    CharactersSnapshot { source: SourceKind, characters: Vec<CanonicalCharacter> },
    /// One character added or updated. Cache inserts/overwrites one entry.
    CharacterUpdated(CanonicalCharacter),
    /// One character removed from its source. Cache evicts one entry.
    /// Foundry emits this from `deleteActor`; Roll20's wire protocol has
    /// no per-event deletion yet (relies on snapshot reconciliation).
    CharacterRemoved { source: SourceKind, source_id: String },
    RollReceived(CanonicalRoll),
    ItemDeleted { source: SourceKind, source_id: String, item_id: String },
}
```

Sources are stateless transformers. Shared connection state
(per-source connected flag, outbound `mpsc::Sender`, merged
characters map, roll-history ring) lives in `BridgeState`
(`src-tauri/src/bridge/mod.rs`).

**Roll20 source** wire protocol (extension protocol — preserved verbatim
from the pre-bridge era so the existing browser extension keeps working):

```rust
// bridge/roll20/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute { pub name: String, pub current: String, pub max: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub controlled_by: String,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMsg {
    Characters { characters: Vec<Character> },
    CharacterUpdate { character: Character },
}
// Outbound messages built inline via `serde_json::json!` in the
// Roll20Source::build_set_attribute / build_refresh impls.
```

**Foundry source** wire protocol (Foundry module → Tauri):

```rust
// bridge/foundry/types.rs
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    Actors { actors: Vec<FoundryActor> },
    ActorUpdate { actor: FoundryActor },
    Hello {
        #[serde(default)] protocol_version: Option<u32>,
        #[serde(default)] world_id: Option<String>,
        #[serde(default)] world_title: Option<String>,
        #[serde(default)] system_id: Option<String>,
        #[serde(default)] system_version: Option<String>,
        #[serde(default)] capabilities: Option<Vec<String>>,
    },
    /// Module-side handler threw; surfaced to the GM via toast.
    Error {
        refers_to: String,
        #[serde(default)] request_id: Option<String>,
        code: String,
        message: String,
    },
    /// Inbound roll result captured by the Foundry-side `createChatMessage`
    /// hook; decoded into `CanonicalRoll` by `bridge/foundry/translate_roll.rs`.
    RollResult { message: FoundryRollMessage },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundryActor {
    pub id: String,
    pub name: String,
    pub owner: Option<String>,
    /// Raw `actor.system` blob — translate.rs picks paths from
    /// docs/reference/foundry-vtm5e-paths.md.
    pub system: serde_json::Value,
}
```

Outbound (Tauri → Foundry module): dot-namespaced wire types
(`actor.update_field`, `actor.create_item_simple`,
`actor.apply_dyscrasia`, `refresh`). Each helper has a typed
Rust builder in `bridge/foundry/actions/<umbrella>.rs` and a
JS executor in `vtmtools-bridge/scripts/foundry-actions/<umbrella>.js`
registered into `bridge.js::handleInbound`'s handler-map dispatch.
See `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`
for the umbrella conventions.

### Mirror layer

The frontend `src/types.ts` mirrors these shapes in TypeScript.
Drift is not tolerated — changing a Rust struct requires updating
the TS mirror in the same commit. The TS mirror is `BridgeCharacter`
(canonical) plus `Roll20Raw` / `Roll20RawAttribute` for
source-specific helpers reading off `char.raw` when source is
Roll20.

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
  - `Arc<BridgeState>` on the Rust side holds the merged
    canonical-character cache (`HashMap<String, CanonicalCharacter>`
    keyed by `<source>:<source_id>`), the per-source `ConnectionInfo`
    map (connected flag + outbound mpsc sender), and the registered
    `BridgeSource` impls. Shared between every accept loop and the
    Tauri command handlers. See
    [ADR 0006](docs/adr/0006-bridge-source-generalization.md).
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
- **`src-tauri/src/db/modifier.rs`** (9):
  `list_character_modifiers`, `list_all_character_modifiers`,
  `add_character_modifier`, `update_character_modifier`,
  `delete_character_modifier`, `set_modifier_active`,
  `set_modifier_hidden`, `set_modifier_zone`,
  `materialize_advantage_modifier`.
- **`src-tauri/src/db/advantage.rs`** (5):
  `list_advantages`, `add_advantage`, `update_advantage`,
  `delete_advantage`, `roll_random_advantage`.
- **`src-tauri/src/db/status_template.rs`** (4):
  `list_status_templates`, `add_status_template`,
  `update_status_template`, `delete_status_template`.
- **`src-tauri/src/db/saved_character.rs`** (5):
  `save_character`, `list_saved_characters`,
  `update_saved_character`, `delete_saved_character`,
  `patch_saved_field`.
- **`src-tauri/src/tools/resonance.rs`** (1): `roll_resonance`.
- **`src-tauri/src/tools/skill_check.rs`** (1): `roll_skill_check`.
- **`src-tauri/src/tools/export.rs`** (1): `export_result_to_md`.
- **`src-tauri/src/tools/character.rs`** (3):
  `character_set_field`, `character_add_advantage`,
  `character_remove_advantage`.
- **`src-tauri/src/tools/foundry_chat.rs`** (2):
  `trigger_foundry_roll`, `post_foundry_chat`.
- **`src-tauri/src/tools/gm_screen.rs`** (1):
  `gm_screen_push_to_foundry`.
- **`src-tauri/src/tools/library_push.rs`** (1):
  `push_advantage_to_world`.
- **`src-tauri/src/bridge/commands.rs`** (7):
  `bridge_get_characters`, `bridge_get_rolls`,
  `bridge_get_world_items`, `bridge_get_status`, `bridge_refresh`,
  `bridge_set_attribute`, `bridge_get_source_info`.
  Generic across Roll20 and Foundry — `set_attribute`'s `name` is
  opaque to the frontend and translated per-source by the source's
  `BridgeSource` impl. `bridge_get_rolls` snapshots the in-memory
  roll-history ring (capacity 200, dedup by `source_id`); see the
  events table for the paired live event. `bridge_get_world_items`
  snapshots the per-source world-level item caches (Foundry only in
  v1); paired live event is `bridge://foundry/items-updated`.

Total: 65 commands. New commands are registered in
`src-tauri/src/lib.rs` (`invoke_handler(tauri::generate_handler![...])`).
See §8 for the Tauri capability / ACL surface.

### Typed frontend API wrapper modules

Frontend components never call `invoke(...)` directly. IPC goes
through typed wrapper modules in `src/lib/**/api.ts` (see
`src/lib/domains/api.ts` for the reference implementation: one
exported function per Tauri command, return type matching the Rust
response). New tools adopt the same pattern.

### Bridge WebSocket protocol

Two listeners, both localhost, each pinned to a single source by port
(see [ADR 0006](docs/adr/0006-bridge-source-generalization.md)):

- **`ws://127.0.0.1:7423` → Roll20.** Plain WebSocket. Preserves the
  Roll20 extension's pre-bridge wire protocol byte-for-byte. Connection
  owner is `src-tauri/src/bridge/mod.rs::accept_loop`. At most one
  active extension session.
- **`wss://127.0.0.1:7424` → Foundry.** TLS WebSocket using a self-signed
  cert generated on first launch by `rcgen` and persisted in the Tauri
  app data dir (`bridge-cert.pem`, `bridge-key.pem`). The cert SAN is
  `localhost` only. The GM accepts the cert warning once per browser by
  visiting `https://localhost:7424` directly. wss is non-optional
  because Foundry is commonly served over HTTPS (Forge, Molten,
  reverse-proxied self-host) and browsers block `ws://localhost` from
  HTTPS pages as mixed content.

Message framing is JSON text frames per source — see §2 Bridge domain
for the per-source inbound/outbound variants. The wire shape on each
port is defined entirely by that source's `BridgeSource` impl; there
is no in-message source tag.

If TLS init fails at startup, the Foundry accept loop is NOT spawned
(falling back to plain ws on `:7424` would produce mystery cert errors
in the Foundry browser). The failure is logged once and Foundry stays
disabled for the session; Roll20 is unaffected.

### Tauri events (backend → frontend)

| Event | Payload | Emitted when |
|---|---|---|
| `bridge://roll20/connected` | none | Roll20 extension opens WS connection |
| `bridge://roll20/disconnected` | none | Roll20 extension closes WS connection |
| `bridge://foundry/connected` | none | Foundry module opens wss connection |
| `bridge://foundry/disconnected` | none | Foundry module closes wss connection |
| `bridge://characters-updated` | `Vec<CanonicalCharacter>` | Any source pushed updated characters; carries the merged cache across all sources |
| `bridge://roll-received` | `CanonicalRoll` | Foundry source decoded a `roll_result` chat message into a canonical roll; pushed into bridge state ring (capacity 200, dedup by `source_id`) and emitted in one accept-loop arm |
| `bridge://foundry/items-updated` | `Vec<CanonicalWorldItem>` | Foundry world-level item cache changed (snapshot, upsert, or delete); carries the merged cache flattened across all sources |
| `modifiers://rows-reaped` | `{ ids: number[] }` | Backend reaped advantage-bound `character_modifiers` rows after a Foundry `deleteItem` hook (via `item_deleted` wire); frontend modifier store drops the listed ids |

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
- Only `src-tauri/src/bridge/*` talks to the WebSocket / WSS servers.
  No other backend module binds a socket. Per-source impls live under
  `bridge/<source>/` and implement `BridgeSource`; they do not bind
  ports themselves — `bridge/mod.rs` owns the accept loops.
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
- **`advantages.is_custom` tri-state read.** Pre-Phase-4 the column
  was binary (0 = corebook reseed-managed, 1 = GM hand-authored).
  Post-Phase-4 imported FVTT rows are also `is_custom = 1` (so
  destructive reseed leaves them alone — same survival semantics as
  hand-authored) BUT they're visually distinguished by a non-null
  `source_attribution` JSON column. The tri-state is therefore:
  - `is_custom = 0` → corebook seed; replaced on every startup
    ([ADR 0002](docs/adr/0002-destructive-reseed.md)).
  - `is_custom = 1 AND source_attribution IS NULL` → GM hand-authored
    locally; survives reseed; editable in AdvantagesManager.
  - `is_custom = 1 AND source_attribution IS NOT NULL` →
    FVTT-imported; survives reseed; UI shows source chip with world
    title.

  UI filters and reaper helpers MUST treat the latter two as
  semantically distinct "local" vs. "imported" states despite
  identical persistence flags.
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
- **Card pattern.** Card-shaped UI surfaces (modifier cards, status
  palette templates, character cards, dyscrasia cards) follow a shared
  anatomy: *drag handle (top, persistent grab affordance) → name →
  body content → optional overflow pill (bottom-right)*. Overflow
  content opens a native `<dialog>` overlay. Context menus and
  overlays must render outside the card's stacking context (portal
  pattern in `DropMenu.svelte`, or native `<dialog>`/Popover API) to
  escape `overflow: hidden` and `transform` parents. CSS container
  queries (`container-type: inline-size`) drive fluid card sizing
  via `clamp(min, Ncqi, max)` on the row. See
  [`docs/superpowers/specs/2026-05-14-gm-screen-card-redesign-design.md`](docs/superpowers/specs/2026-05-14-gm-screen-card-redesign-design.md).
- In Svelte 5 runes mode, `in:` / `out:` transitions are placed on
  elements whose lifecycle is controlled by the enclosing `{#each}`
  or `{#if}`, not on runes-mode component roots. Use a plain wrapper
  `<div in:scale out:fade>` in the parent's `{#each}` block.
- At most one connection per source at a time (one Roll20 extension,
  one Foundry GM browser session). State is held in
  `Arc<BridgeState>` shared between every accept loop and the Tauri
  command handlers; per-source `ConnectionInfo` lives inside it.
- The merged characters cache is source-slice-authoritative on
  `CharactersSnapshot`: when one source sends a fresh snapshot, every
  prior cache entry from that source is dropped before the new set is
  inserted. The cache never carries entries from a source whose latest
  snapshot omitted them. Replaces the earlier merge-only semantic that
  produced ghost-character bugs.
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
  appropriate `bridge://<source>/disconnected` event fires, the UI
  shifts that source's pip to "not connected", and the next reconnect
  restores service. Other sources are unaffected.
- Database errors from `sqlx` propagate as `Err(String)` with
  module-stable prefixes. Migration failures on startup are fatal
  (the app exits with a user-visible error).

## §8 Security model

Trust boundaries and assumptions for a single-user local desktop tool.

- **Trust posture.** Single user, single machine. No authentication,
  no authorization, no user-level access control. These are non-
  goals (§12).
- **Network surface.** Exactly two listeners, both bound to loopback:
  - `ws://127.0.0.1:7423` for the Roll20 extension.
  - `wss://127.0.0.1:7424` for the Foundry module (TLS via self-signed
    `localhost` cert in the app data dir).

  Neither must ever bind to `0.0.0.0` or any routable interface. No
  other external network call is made by the app.
- **Localhost WS trust model.** Any process running as the user can
  connect to either listener. This is equivalent to trusting the
  user and is the intended posture. Do not add authentication to
  the listeners without a specific threat that justifies it.
- **TLS cert.** Self-signed for `localhost` only, generated by
  `rcgen` on first launch, persisted as `bridge-cert.pem` /
  `bridge-key.pem` in the Tauri app data dir. Not installed into the
  OS trust store; the GM accepts the cert warning once per browser
  by visiting `https://localhost:7424`. Cert key material never
  leaves the user's machine.
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
- **Browser extension DOM-read surface (Roll20).** The extension
  reads Roll20 DOM nodes that feed the `Character`/`Attribute`
  shape (§2 Bridge domain → Roll20 source). It never sends data
  outside the localhost WebSocket.
- **Foundry module surface.** `vtmtools-bridge/` (a Foundry module
  installed in the world) runs in the GM's browser only —
  initialization is gated on `game.user.isGM`. It reads
  `game.actors`, hooks `updateActor` / `createActor` / `deleteActor`,
  and dials `wss://localhost:7424`. Players' browsers never connect.
- **Secrets.** None. No API keys, tokens, or credentials in the
  app. If a future feature introduces one, this section is updated
  and an ADR is filed.

## §9 Extensibility seams

Named places to add things. Feature specs cite a seam instead of
inventing a new hook.

- **Add a tool.** Add one entry to `src/tools.ts`. Sidebar +
  lazy-loaded component wiring is automatic. Existing examples:
  `Resonance.svelte`, `DyscrasiaManager.svelte`, `Campaign.svelte`,
  `DomainsManager.svelte`, `GmScreen.svelte`, `RollFeed.svelte` —
  the pattern is stable.
- **Add a schema change.** Add a new
  `src-tauri/migrations/NNNN_*.sql` file; migrations run on app
  start. Mirror the shape change in `shared/types.rs` and
  `src/types.ts` in the same commit.
- **Add a node or edge type.** No code change. The chronicle graph
  uses freeform strings ([ADR 0003](docs/adr/0003-freeform-node-edge-types.md));
  the UI derives autocomplete from existing distinct values.
- **Add a Tauri command.** Declare in the relevant
  `src-tauri/src/**/commands.rs` (or in a per-feature module file
  under the feature directory, e.g. `src-tauri/src/tools/resonance.rs`),
  register in `src-tauri/src/lib.rs` inside the
  `invoke_handler(tauri::generate_handler![...])` list, revisit the
  capability JSON if a narrower ACL is in force (§8), then add a
  typed wrapper in `src/lib/**/api.ts`. Components call the wrapper,
  never `invoke(...)` directly.
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
- **Add a VTT bridge source.** Add a `SourceKind` variant in
  `src-tauri/src/bridge/types.rs`, create
  `src-tauri/src/bridge/<vtt>/{mod.rs,types.rs,translate.rs}` with a
  `BridgeSource` impl, and register it in `lib.rs` (insert into the
  `sources` map before `start_servers`). Pick a port: plain ws is
  fine if the VTT serves over plain http and the data path is a
  browser extension; wss is required if the data path is a page-
  context module from a TLS-served VTT (browsers block mixed
  content). No protocol, command, or frontend changes are required
  beyond the new `SourceKind` variant — the bridge store and tools
  already iterate per-source. See
  [ADR 0006](docs/adr/0006-bridge-source-generalization.md).
- **Add a library kind.** Phase 4 partitioning rule: **same row
  shape → polymorphic table with a `kind` discriminator column;
  different row shape → its own table.** The four featuretype
  variants (`merit`, `flaw`, `background`, `boon`) share an
  identical row shape AND an identical Foundry push contract
  (`actor.create_feature` accepts featuretype as a payload field —
  foundry helper roadmap §5), so they share the polymorphic
  `advantages` table. Dyscrasias have a distinct shape
  (`resonance_type`, `bonus`) and their own table. Disciplines
  (when they land) will have yet another distinct shape (power
  tree, Amalgam refs, level-gated powers) and their own table. To
  add a new variant that shares the advantage row shape: extend
  `AdvantageKind` enum in `shared/types.rs`, update the SQL CHECK
  constraint via a new migration, annotate any new seed rows, and
  wire the chip in `AdvantagesManager.svelte`. To add a new
  variant that does NOT share the row shape: new table + new
  `db/<kind>.rs` module + new manager tool, following the
  dyscrasia pattern.
- **Add a card-shaped surface.** Follow the card pattern in §6:
  handle + name + body + optional overflow pill, with menus and
  overlays portal-rendered. Reuse `CardContextMenu` for right-click
  actions and `CardOverlay` for the "open full" view; both are
  zero-dep wrappers (Svelte 5 runes, `<dialog>` native, position-
  fixed portal). Per-domain content goes inside the overlay body
  snippet.

## §10 Testing & verification

- Rust unit tests live as `#[cfg(test)] mod tests` inside each
  source file. Current test modules: `shared/dice.rs`,
  `shared/resonance.rs`, `db/dyscrasia.rs`, `db/chronicle.rs`,
  `db/node.rs`, `db/edge.rs`, `db/saved_character.rs`,
  `tools/export.rs`, `bridge/foundry/mod.rs`,
  `bridge/foundry/types.rs`. (Run
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
  - `shared/types.rs`: `FieldValue` variants `Date`, `Url`,
    `Email`, and `Reference` may surface "never constructed" —
    the v1 Domains UI uses only `String`, `Text`, `Number`, and
    `Bool` widgets. The unused variants ship as extensibility
    seams for future property widgets (see §9). Do not remove.
  - The pre-bridge "unused `listen` import in Campaign.svelte and
    Resonance.svelte" entry is no longer applicable — the cutover
    to the bridge store ([ADR 0006](docs/adr/0006-bridge-source-generalization.md))
    moved all `listen()` calls into `src/store/bridge.svelte.ts`.

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
- No network surface beyond the two localhost bridge listeners
  (`127.0.0.1:7423` plain ws for Roll20, `127.0.0.1:7424` wss for
  Foundry).
- No ingestion path for Roll20 data other than the browser extension
  bridge ([ADR 0005](docs/adr/0005-roll20-ws-extension-bridge.md),
  superseded structurally by [ADR 0006](docs/adr/0006-bridge-source-generalization.md)).
- No ingestion path for Foundry data other than the
  `vtmtools-bridge/` Foundry module
  ([ADR 0006](docs/adr/0006-bridge-source-generalization.md)).
- No multi-session per source. At most one Roll20 extension and one
  Foundry GM browser at a time.
- No additional VTT bridges in v1. New sources require a new
  `bridge/<vtt>/` impl plus a `SourceKind` variant — no protocol or
  command-surface change. See §9 Extensibility seams.
- No frontend testing framework.

## §13 ADR index

| # | Title | Status |
|---|---|---|
| 0001 | [Tauri 2 + SvelteKit + SQLite stack](docs/adr/0001-tauri-2-stack.md) | accepted |
| 0002 | [Destructive reseed of non-custom dyscrasias on startup](docs/adr/0002-destructive-reseed.md) | accepted |
| 0003 | [Freeform strings for nodes.type and edges.edge_type](docs/adr/0003-freeform-node-edge-types.md) | accepted |
| 0004 | [Dark-only theming](docs/adr/0004-dark-only-theming.md) | accepted |
| 0005 | [Roll20 integration via localhost WebSocket + browser extension](docs/adr/0005-roll20-ws-extension-bridge.md) | superseded by 0006 |
| 0006 | [Generalize Roll20 bridge into a multi-source `bridge/` layer](docs/adr/0006-bridge-source-generalization.md) | accepted |

Add new rows here as ADRs are written. When an ADR is superseded,
update its Status column to `superseded by NNNN`.
