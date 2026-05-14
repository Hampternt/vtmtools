# Advantages Library — Design Spec
_2026-04-19_

## Context

VTM 5e characters build with **Advantages** — an umbrella term covering Merits, Backgrounds, and Flaws (Flaws are negative Advantages). The game currently ships dozens of these spread across the corebook and supplements, each with a short description, a dot cost (fixed or ranged), and sometimes prerequisites or source attribution.

vtmtools already has a reference library for one kind of curated game content: the Dyscrasia Manager (`src/tools/DyscrasiaManager.svelte`, `src-tauri/src/db/dyscrasia.rs`). This spec adds a second reference library — the **Advantages Library** — following the same "curated catalog + user customs" pattern, but with a more flexible schema that supports a future character builder.

### Goals

1. Catalog the canonical V5 Merits, Backgrounds, and Flaws as browsable library entries.
2. Allow the user to add, edit, and delete custom entries that survive reseeds.
3. Filter and search by tag (freeform, multi-select) and by text query.
4. Support arbitrary per-entry typed fields (level, min_level, max_level, source, prereq, and anything else the user adds later) without requiring a schema migration.
5. Stay forward-compatible with a future character builder that will consume entries as drag-and-drop sources.

### Non-goals (v1)

- **No Roll20 integration.** The Jumpgate VTM 5e sheet stores merits in repeating sections (`repeating_meritsflaws_<rowID>_…`). The current outbound WS protocol (`OutboundMsg::SetAttribute { name, value }` in `src-tauri/src/roll20/mod.rs`) cannot mint new repeating-row IDs, and removing a row requires deleting every field in the group. Adding either flow would require extending both the protocol and the browser extension's content script — out of scope for this feature.
- **No character-attachment wiring.** The character builder is a separate future feature. This library only delivers the catalog + CRUD.
- **No DB-backed preset management.** Field presets (level, min_level, …) ship as a hardcoded TS constant. A `field_presets` table is a future seam if the user wants to curate presets inside the app.

---

## Architecture

Single SQLite table, five Tauri commands, one typed TS API wrapper, one Svelte tool, two presentational components. Mirrors `ARCHITECTURE.md` §9 seam for adding a tool and seam for adding a schema change.

### Domain model

```rust
// src-tauri/src/shared/types.rs (additions)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Advantage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,   // reuses existing Field / FieldValue (ARCHITECTURE.md §2)
    pub is_custom: bool,
}
```

`Field` and `FieldValue` already exist in `shared/types.rs` and are already mirrored in `src/types.ts`. The new `Advantage` struct reuses them directly — no new property infrastructure is introduced.

### Schema

```sql
-- src-tauri/migrations/0003_advantages.sql
CREATE TABLE IF NOT EXISTS advantages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    properties_json TEXT    NOT NULL DEFAULT '[]',
    is_custom       INTEGER NOT NULL DEFAULT 0
);
```

Intentionally minimal. Level, min_level, max_level, source, prereq, and any future per-entry attributes live inside `properties_json` as `Vec<Field>` blobs — same storage posture as `nodes.properties_json` (ARCHITECTURE.md §2 Chronicle graph domain).

Tags are freeform strings, not an enum. This adopts the same posture as [ADR 0003 (freeform strings for `nodes.type` / `edges.edge_type`)](docs/adr/0003-freeform-node-edge-types.md) — filter chips are derived from `DISTINCT` values in the data, not declared up-front. No new ADR is required; this is a direct application of an existing accepted decision.

Tag & property-name uniqueness (no duplicate tag strings within a row's `tags`, no duplicate `name` values within a row's `properties`) is enforced in the editor form, not by the database — matching the Domains Manager's convention.

### Tauri commands

Five commands live in `src-tauri/src/db/advantage.rs`, registered in `src-tauri/src/lib.rs`'s `invoke_handler!` list (ARCHITECTURE.md §9):

| Command | Signature | Notes |
|---|---|---|
| `list_advantages` | `() -> Vec<Advantage>` | Returns all rows ordered `is_custom ASC, id ASC`. Sort/filter is frontend work. |
| `add_advantage` | `(name, description, tags, properties) -> Advantage` | Always inserts with `is_custom = 1`. |
| `update_advantage` | `(id, name, description, tags, properties) -> ()` | Rejects rows with `is_custom = 0`. Returns `Err(String)` with prefix `"db/advantage.update: …"`. |
| `delete_advantage` | `(id) -> ()` | Rejects rows with `is_custom = 0`. |
| `roll_random_advantage` | `(tags: Vec<String>) -> Option<Advantage>` | `tags` is OR-semantics, matching the chip-filter UI: empty vec = no filter, non-empty = pick randomly among rows whose `tags` intersects `tags`. Returns `None` when the filter set is empty. |

Error strings use the `"db/advantage.<op>: …"` prefix convention (ARCHITECTURE.md §7).

**Capability / ACL impact.** None. These five commands register under the existing `core:default + opener:default` grant in `src-tauri/capabilities/default.json` (ARCHITECTURE.md §8). No narrower per-command ACL is in force, so no capability JSON change is required. If that posture ever tightens, this spec and its plan must be revisited per §8's directive.

### Frontend API wrapper

```ts
// src/lib/advantages/api.ts
export async function listAdvantages(): Promise<Advantage[]> { … }
export async function addAdvantage(input: AdvantageInput): Promise<Advantage> { … }
export async function updateAdvantage(id: number, input: AdvantageInput): Promise<void> { … }
export async function deleteAdvantage(id: number): Promise<void> { … }
export async function rollRandomAdvantage(tags: string[]): Promise<Advantage | null> { … }
```

Components import from this module and never call `invoke(...)` directly (ARCHITECTURE.md §4, §5). This corrects the pre-convention pattern in `DyscrasiaManager.svelte` rather than copying it.

---

## Seed policy

`src-tauri/src/db/seed.rs` grows a new pass that deletes all `is_custom = 0` rows from `advantages` and reinserts the canonical V5 set on every app start. Destructive-reseed semantics per ADR 0002. User-authored rows (`is_custom = 1`) are preserved.

### Seeded-row tagging convention

Each built-in row is tagged with at least:
- `"VTM 5e"` — the game-system tag.
- One of `"Merit"`, `"Background"`, `"Flaw"` — the primary category.
- Zero or more subcategory tags from the corebook's grouping (e.g. `"Physical"`, `"Social"`, `"Camarilla"`, `"Thin-Blood"`).

Example seeded row:

```json
{
  "name": "Allies",
  "description": "Mortal assistants who will do you favors…",
  "tags": ["VTM 5e", "Background", "Social"],
  "properties": [
    { "name": "min_level", "type": "number", "value": 1 },
    { "name": "max_level", "type": "number", "value": 5 },
    { "name": "source",    "type": "string", "value": "V5 Corebook p.194" }
  ],
  "is_custom": false
}
```

Seed coverage for v1: all corebook Merits, Backgrounds, and Flaws. Supplements are out of scope for the initial seed — the user can add them as customs.

---

## Field presets (UI convenience)

Hardcoded in TypeScript, rendered as quick-add chips in the `AdvantageForm` "Add field" row:

```ts
// src/lib/advantages/fieldPresets.ts
export type FieldPreset = {
  name: string;
  type: 'string' | 'text' | 'number' | 'bool';
  defaultValue: string | number | boolean;
  hint: string;
};

export const FIELD_PRESETS: FieldPreset[] = [
  { name: 'level',     type: 'number', defaultValue: 1,   hint: 'Fixed dot cost' },
  { name: 'min_level', type: 'number', defaultValue: 1,   hint: 'Minimum dots (for ranged merits)' },
  { name: 'max_level', type: 'number', defaultValue: 5,   hint: 'Maximum dots (for ranged merits)' },
  { name: 'source',    type: 'string', defaultValue: '',  hint: 'Sourcebook reference' },
  { name: 'prereq',    type: 'text',   defaultValue: '',  hint: 'Prerequisite text' },
];
```

**Behaviour.** In the form's properties section, a row of preset chips sits above the existing property list. Clicking a preset chip appends a new `Field` with the preset's name/type/defaultValue to the row's `properties`. A preset chip is disabled when a field with that name already exists on the row (to preserve name uniqueness). A `Custom…` chip opens the generic name/type picker already used by the Domains Manager property editor.

No DB storage in v1. Future: if the user wants to curate their own presets, migrate to a `field_presets` table + CRUD commands.

---

## Frontend surfaces

### Registration

Add one entry to `src/tools.ts` (ARCHITECTURE.md §9 "Add a tool"):

```ts
{ id: 'advantages', label: 'Advantages', icon: '⚜', component: () => import('./tools/AdvantagesManager.svelte') }
```

### `AdvantagesManager.svelte` layout

Same structure as `DyscrasiaManager.svelte`:

- **Search bar** — placeholder `"Search by name, description, or tag…"`. Debounced ~110ms like Dyscrasia's.
- **Filter chips** — dynamically derived from `[...new Set(rows.flatMap(r => r.tags))].sort()`. An `All` chip is always present and mutually exclusive with the others (same semantics as Dyscrasia). Multi-select chips: `OR` semantics — a row matches if it has *any* of the active tags.
- **Sort dropdown** — to the right of the search bar. Options: `Name A-Z`, `Name Z-A`, `Level ↑`, `Level ↓`, `Recently added`. Level-based sorts read `level ?? min_level` — i.e., a ranged merit (no `level`, only `min_level`/`max_level`) sorts by its minimum dot cost rather than sinking to the bottom. Rows with neither `level` nor `min_level` sort to the end regardless of direction.
- **Results count** — `"Showing N advantages"`.
- **Card grid** — CSS grid, `repeat(auto-fill, minmax(12.5rem, 1fr))`, `align-items: start` (ARCHITECTURE.md §6).
- **Add / Edit** — identical toggling pattern to Dyscrasia's (`showAddForm`, `editingId`).

### `AdvantageCard.svelte`

Built-in (`is_custom = false`):

```
[Name — bold]
[Tags row — small chips]
[Description — clipped with "show more" like Dyscrasia]
──────────────────────────
[Property summary]            [built-in]
```

Custom (`is_custom = true`): same layout, footer shows `[✎ Edit] [✕]` in place of `[built-in]`.

**Property summary.** If a `level` field is present, render a dot strip (`●●●○○`). Else if `min_level` and `max_level` both exist, render `min_level–max_level dots`. Else render nothing. Any other properties render as a small `key: value` list below the dots.

### `AdvantageForm.svelte`

Three sections:

1. **Basics.** Name (required), description (textarea).
2. **Tags.** A chip-editor: existing tags as chips with `×`, plus a text input that adds a tag on `Enter`. Suggests existing tags via datalist (dynamically pulled from the full library).
3. **Properties.** The field preset row (from §Field Presets), then one `PropertyEditor` row per field in `properties`. The existing `src/lib/components/domains/PropertyEditor.svelte` and its `property-widgets/` registry are already generic (take a `Field`, not a `Node`, and do not import `domains/api.ts`) — but their filesystem location implies domains-only ownership. This spec includes a **one-time move** to `src/lib/components/properties/` (updating the single import in `src/lib/components/domains/NodeForm.svelte`) so both tools consume from a shared, tool-agnostic location. No behaviour change.

Save validates: name is non-empty, tag strings are unique and non-empty, property names are unique and non-empty. Invalid state disables Save and surfaces an inline error string.

---

## Error handling

Follows ARCHITECTURE.md §7:
- Rust commands return `Result<T, String>`. Errors prefix with `"db/advantage.<op>: …"`.
- Frontend API wrapper catches rejected promises at the call site. `AdvantagesManager` sets a `loadError` banner string.
- `roll_random_advantage` returns `Ok(None)` (not an error) when the filtered set is empty.

---

## Testing

Rust tests live as `#[cfg(test)] mod tests` inline in `src-tauri/src/db/advantage.rs` — mirroring `db/dyscrasia.rs`'s coverage:

- Empty list returns empty vec.
- Insert-then-list round-trip preserves tags and properties.
- `update_advantage` rejects `is_custom = 0` rows.
- `delete_advantage` rejects `is_custom = 0` rows.
- `roll_random_advantage` with empty `tags` returns a random row from the full library.
- `roll_random_advantage` with a multi-tag `tags` vec returns only rows whose `tags` intersect (OR-match).
- `roll_random_advantage` with tag filter matching no rows returns `Ok(None)`.

No frontend tests (ARCHITECTURE.md §10 — the no-frontend-test-framework posture stands).

Verification gate: `./scripts/verify.sh` (ARCHITECTURE.md §10) must pass before merge.

---

## Forward-compat: character builder

This spec is sized to be self-contained, but the schema choices are deliberately pointed at a future character builder:

- `properties: Vec<Field>` is the same shape `nodes.properties_json` uses. "Drag merit onto character node" becomes: copy the advantage's fields into the character node's properties, optionally with a per-merit `level` override.
- Creating an `edge` of type `"has_advantage"` from a character node to an advantage-source node is not possible today because advantages are not nodes and live cross-chronicle. When the character builder lands, the options are (a) promote advantages to nodes in a synthetic library chronicle, or (b) store a reference-by-id on the character node (new `FieldValue::Reference` use). Decision deferred to that feature's spec.

No API change to the library commands is expected for builder integration. Any additions should be additive.

---

## Work breakdown (high-level)

The detailed plan is the job of the writing-plans skill after this spec is approved. At a glance, the shape is:

1. Migration + domain type + TS mirror.
2. Tauri commands + inline unit tests.
3. Seed-data population (largest single chunk — the V5 corebook data).
4. Typed API wrapper.
5. Preset constant.
6. Extract `PropertyEditor.svelte` + `property-widgets/` from `components/domains/` to `components/properties/`; update the one import in `NodeForm.svelte`.
7. Card, Form, Manager components.
8. Tool registry entry.
9. `./scripts/verify.sh` green.

---

## Open questions for implementation

None blocking. Confirmed with user:
- One unified library, category-by-tag.
- Level/min_level/max_level live in properties, not columns.
- `roll_random_advantage` is kept (useful for GM random-NPC-background helper).
- No Roll20 integration in v1.

Resolved during review:
- `PropertyEditor.svelte` / `property-widgets/` are already tool-agnostic — a file move, not a refactor, suffices for shared reuse.
- `roll_random_advantage` signature is `(tags: Vec<String>)` to match the multi-select chip filter's OR semantics, not `Option<String>`.
- Level-based sorts fall back from `level` to `min_level` so ranged merits don't sink to the bottom of "Level ↑".
