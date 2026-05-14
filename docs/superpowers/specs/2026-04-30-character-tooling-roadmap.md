# Character Tooling Roadmap

> **Status:** directional roadmap with implementable Phase 1. Phase 1 spawns four plans that ship together; Phases 2–5 are sketched and will produce their own spec + plan cycles when they mature.
>
> **Audience:** anyone designing or implementing vtmtools features that touch character data, dice mechanics, or the Foundry bridge protocol.

---

## §1 What this is

A roadmap for evolving vtmtools from "GM dashboard that views bridged characters" to a first-class GM workbench that owns local character data, runs V5 dice mechanics natively, and bidirectionally interacts with a live Foundry world.

The roadmap covers four phases. Phase 1 is fully designed in this document and produces four plans. Phases 2–5 are sketched at architecture level — they will get their own brainstorm + spec + plan cycles when their requirements mature, on the same pattern as `2026-04-26-foundry-helper-library-roadmap.md`.

## §2 Phase index

| Phase | Goal | Status |
|---|---|---|
| **1 — Foundations** | Local character persistence; V5 dice library; bridge protocol consolidation; source attribution | **fully designed in this doc → 4 plans** |
| 2 — Character editing | Generic `character::set_field` router; add/remove advantage; stat editor UI | sketched |
| 3 — Roll mirroring | Roll-source toggle (here vs. Foundry); ChatMessage readback; hunger/remorse/contested rolls; roll log | sketched |
| 4 — Library sync | Push/pull merits & dyscrasias between tool DB and Foundry world; source-attribution badges; dedup | sketched |
| 5+ — Foundry data surface | Reserved umbrellas (`journal.*`, `scene.*`, `item.*`, `chat.*`, `combat.*`); each activated when its consumer feature lands | reserved by name only |

---

## §3 Phase 1 design

### §3.1 Data model & storage (Plan 1 foundation)

#### Rust types

```rust
// src-tauri/src/db/saved_character.rs (new module)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCharacter {
    pub id: i64,
    pub source: SourceKind,            // reuses existing bridge enum (Roll20 | Foundry)
    pub source_id: String,             // matches CanonicalCharacter.source_id
    pub foundry_world: Option<String>, // captured at save time; powers source attribution
    pub name: String,                  // duplicated from canonical for fast list queries
    pub canonical: CanonicalCharacter, // full snapshot, persisted as JSON column
    pub saved_at: String,              // ISO-8601, set on insert, never changed
    pub last_updated_at: String,       // ISO-8601, bumped on each Update
}
```

#### SQLite schema (new migration)

```sql
-- src-tauri/migrations/NNNN_saved_characters.sql
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

#### Match key

Frontend computes `(source, source_id)` for every live `CanonicalCharacter` and joins against the `saved_characters` rows:

- **Live ∩ Saved match** → live card in Live section + saved card in Saved section (Layout B). Saved card shows "drift" badge if `canonical_json` differs from current live.
- **No match** → Live-only or Saved-only depending on which side has the row.

#### Source attribution

Phase 1 attribution = `(source, foundry_world)` derived in TypeScript for display ("source: FVTT — Chronicles of Chicago" or "source: Roll20"). Phase 4 imported items will gain a separate `source_attribution` JSON column on those records — Phase 1 does **not** introduce a polymorphic attribution table.

#### Why this shape

- **One row per `(source, source_id)`** — versioned saves are YAGNI for v1; the UNIQUE constraint makes save idempotent and live↔saved matching a single index lookup.
- **Full canonical blob in one TEXT column** — diffing is JSON-walk on the deserialized struct; no normalized stats schema needed.
- **`foundry_world` as a plain optional column** — no separate attributions table or join.

### §3.2 Plan 0 wire protocol consolidation

Three additions: extended Hello, subscription envelopes, error envelope. Module bumps to `0.2.0`.

#### Connection sequence (post-Plan 0)

```
Foundry module                      vtmtools desktop
─────────────                       ────────────────
  open wss://localhost:7424
  ────────────────────────────────►
  send Hello { protocol_version: 1,
               world_id, world_title,
               system_id, system_version,
               capabilities: ["actors"] }
  ────────────────────────────────►
                                    store SourceInfo in BridgeState
                                    emit bridge://foundry/connected
  send Actors { actors: [...] }     (module auto-subscribes itself to actors;
  ────────────────────────────────► matches today's always-send behavior)
```

#### Hello — extended

```rust
// src-tauri/src/bridge/foundry/types.rs
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    Hello {
        protocol_version: u32,
        world_id: Option<String>,
        world_title: Option<String>,
        system_id: Option<String>,
        system_version: Option<String>,
        capabilities: Vec<String>,         // Plan 0 ships ["actors"]
    },
    Actors      { actors: Vec<FoundryActor> },
    ActorUpdate { actor: FoundryActor },
    Error {
        refers_to: String,
        request_id: Option<String>,
        code: String,
        message: String,
    },
}
```

`protocol_version: 1` is forward-compat insurance — a future module shipping `protocol_version: 2` is detectable at handshake and refused with a clear error rather than producing mystery failures. `capabilities` is the flat list of subscriptions the module can serve; future plans extend it.

**Backward compatibility for old modules.** A pre-Plan-0 module (0.1.0) sends a Hello payload without `protocol_version` / `capabilities`. To avoid silent connection failures during the upgrade window, the desktop deserializes Hello with `protocol_version: Option<u32>` and `capabilities: Option<Vec<String>>`. Missing `protocol_version` is treated as `0` (legacy) and `capabilities` defaults to `["actors"]` (the always-send-actors behavior). Legacy connections still work; the source-attribution chip falls back to "source: FVTT" without world title. New module ships `protocol_version: 1` and is the default for all Phase 1+ behavior.

#### Subscription protocol

Outbound from desktop:

```json
{ "type": "bridge.subscribe",   "collection": "journal" }
{ "type": "bridge.unsubscribe", "collection": "journal" }
```

The module's auto-subscribe-actors-on-Hello behavior preserves today's always-send-actors semantics. Future tools that need other collections (`journal`, `scene`, etc.) send `bridge.subscribe`; the module's `subscribers` registry attaches the relevant Hooks.

#### Error envelope

```json
{
  "type": "error",
  "refers_to": "actor.update_field",
  "request_id": null,
  "code": "no_such_actor",
  "message": "actor not found: 9XaR3dFq..."
}
```

`refers_to` is the wire `type` of the message that caused the error. `request_id` is reserved for future correlation (additive — no version bump needed). `code` is a stable machine-readable tag; `message` is human prose. Desktop routes errors to a new `bridge://foundry/error` event.

#### Module-side scope (`vtmtools-bridge/`)

- `module.json` — bump version to `0.2.0`.
- `scripts/bridge.js` — extend Hello payload; route `Error` envelope on handler exceptions.
- `scripts/foundry-actions/index.js` — register `bridge.*` umbrella in handler-map.
- `scripts/foundry-actions/bridge.js` (new) — `subscribers` registry; subscribe/unsubscribe handlers.
- `scripts/foundry-actions/actor.js` — refactor to expose `attach()`/`detach()` so the registry can manage it. **Internal change only — wire shape unchanged.**

#### Desktop-side scope (`src-tauri/`)

- `bridge/foundry/types.rs` — extend `FoundryInbound` (Hello fields + Error variant).
- `bridge/foundry/translate.rs` — populate `BridgeState.source_info` on Hello; emit `bridge://foundry/error` on Error.
- `bridge/foundry/actions/bridge.rs` (new) — outbound subscribe/unsubscribe builders.
- `bridge/types.rs` — add `SourceInfo` struct.
- `bridge/mod.rs` — extend `BridgeState` with `source_info: HashMap<SourceKind, SourceInfo>`.
- `bridge/commands.rs` — new `bridge_get_source_info` Tauri command.
- `lib.rs` — register the new command.

### §3.3 Tauri command surface

Six new commands across the four plans. Total surface grows from 31 → 37.

#### Plan 0

```rust
#[tauri::command]
async fn bridge_get_source_info(state: State<'_, Arc<BridgeState>>, source: SourceKind)
    -> Result<Option<SourceInfo>, String>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub world_id: Option<String>,
    pub world_title: Option<String>,
    pub system_id: Option<String>,
    pub system_version: Option<String>,
    pub protocol_version: u32,
    pub capabilities: Vec<String>,
}
```

#### Plan 1

```rust
#[tauri::command]
async fn save_character(state: State<'_, AppState>, canonical: CanonicalCharacter, foundry_world: Option<String>)
    -> Result<i64, String>;
// INSERT only; (source, source_id) collision returns "db/saved_character.save: already saved; use update"

#[tauri::command]
async fn list_saved_characters(state: State<'_, AppState>) -> Result<Vec<SavedCharacter>, String>;

#[tauri::command]
async fn update_saved_character(state: State<'_, AppState>, id: i64, canonical: CanonicalCharacter)
    -> Result<(), String>;
// Overwrites canonical_json; bumps last_updated_at; saved_at unchanged.

#[tauri::command]
async fn delete_saved_character(state: State<'_, AppState>, id: i64) -> Result<(), String>;
```

The save/update split (vs. UPSERT) gives cleaner error semantics: re-clicking Save when a saved row exists is a programming error, not a feature. The `last_updated_at` bump is per-update only; `saved_at` records the original save timestamp.

#### Plan 2

**No new Tauri commands.** The diff is computed in TypeScript — both sides of the comparison are already on the frontend (`SavedCharacter` from `list_saved_characters`, `BridgeCharacter` from `bridge://characters-updated`). No IPC roundtrip per compare-button click; the diff projection is a static config in `src/lib/saved-characters/diff.ts`.

#### Plan 3

```rust
#[tauri::command]
fn roll_skill_check(input: SkillCheckInput) -> SkillCheckResult;
// Sync — no I/O. Wraps shared::v5::skill_check::skill_check with thread_rng().
```

All six commands register in `src-tauri/src/lib.rs` via `invoke_handler(tauri::generate_handler![...])`. Each gets a typed wrapper in `src/lib/**/api.ts`; components never call `invoke(...)` directly (ARCHITECTURE.md §5).

### §3.4 V5 dice helper library (Plan 3)

#### Module structure

```
src-tauri/src/shared/v5/
  mod.rs          re-exports + crate-level docs citing v5-combat-rules.md
  types.rs        all shared types
  pool.rs         build_pool       — assemble PoolSpec from input
  dice.rs         roll_pool        — pure RNG → dice values
  interpret.rs    interpret        — dice → Tally
  difficulty.rs   compare          — Tally + difficulty → Outcome
  message.rs      format_skill_check — Outcome → human string
  skill_check.rs  skill_check      — orchestrator (5 sequential calls)
```

Each leaf is pure. RNG is injected via `&mut impl rand::Rng` (matching the existing pattern in `shared/dice.rs`).

#### Types (in `shared/v5/types.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct PoolPart { pub name: String, pub level: u8 }

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DieKind { Regular, Hunger }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct Die { pub kind: DieKind, pub value: u8 }   // value: 1..=10

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct PoolSpec {
    pub parts: Vec<PoolPart>,
    pub regular_count: u8,
    pub hunger_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct RollResult {
    pub parts: Vec<PoolPart>,
    pub dice: Vec<Die>,                             // pool-order: regulars first, then hunger
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct Tally {
    pub successes: u8,                              // dice ≥6 + 2*crit_pairs
    pub crit_pairs: u8,                             // tens / 2
    pub is_critical: bool,                          // crit_pairs ≥ 1
    pub is_messy_critical: bool,                    // critical AND ≥1 hunger 10
    pub has_hunger_one: bool,                       // ≥1 hunger 1
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub successes: u8,
    pub difficulty: u8,
    pub margin: i32,                                // successes - difficulty
    pub passed: bool,
    pub flags: OutcomeFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct OutcomeFlags {
    pub critical: bool,
    pub messy: bool,
    pub bestial_failure: bool,                      // !passed AND tally.has_hunger_one
    pub total_failure: bool,                        // successes == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct SkillCheckInput {
    pub character_name: Option<String>,             // for message; None → "Strength + Brawl check"
    pub attribute: PoolPart,
    pub skill: PoolPart,
    pub hunger: u8,                                 // 0..=5; 0 = mortal/non-vampire path
    pub specialty: Option<String>,                  // Some → adds 1 die labeled "Specialty: <name>"
    pub difficulty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct SkillCheckResult {
    pub spec: PoolSpec,
    pub roll: RollResult,
    pub tally: Tally,
    pub outcome: Outcome,
    pub message: String,
}
```

#### Function signatures

```rust
// pool.rs
pub fn build_pool(input: &SkillCheckInput) -> PoolSpec;
//   parts = [attribute, skill] + ["Specialty: …" if Some]
//   pool_size = sum of part.level
//   hunger_count = min(input.hunger, pool_size)
//   regular_count = pool_size - hunger_count

// dice.rs
pub fn roll_pool<R: rand::Rng + ?Sized>(spec: &PoolSpec, rng: &mut R) -> RollResult;

// interpret.rs
pub fn interpret(result: &RollResult) -> Tally;
//   successes  = #(dice ≥ 6) + (tens / 2) * 2
//   crit_pairs = tens / 2
//   is_critical = crit_pairs ≥ 1
//   is_messy_critical = is_critical AND #(hunger 10s) ≥ 1
//   has_hunger_one = #(hunger 1s) ≥ 1

// difficulty.rs
pub fn compare(tally: &Tally, difficulty: u8) -> Outcome;
//   margin = successes - difficulty
//   passed = margin ≥ 0
//   total_failure   = successes == 0
//   bestial_failure = !passed AND has_hunger_one

// message.rs
pub fn format_skill_check(input: &SkillCheckInput, result: &RollResult, outcome: &Outcome) -> String;
//   "Charlotte's Strength + Brawl check · Messy Critical · Success +3 (5 vs DV 2)"
//   "Strength + Brawl check · Bestial Failure · -1 (1 vs DV 2)"
```

#### Orchestrator

```rust
pub fn skill_check<R: rand::Rng + ?Sized>(input: &SkillCheckInput, rng: &mut R) -> SkillCheckResult {
    let spec    = build_pool(input);
    let roll    = roll_pool(&spec, rng);
    let tally   = interpret(&roll);
    let outcome = compare(&tally, input.difficulty);
    let message = format_skill_check(input, &roll, &outcome);
    SkillCheckResult { spec, roll, tally, outcome, message }
}
```

Two layers max: orchestrator → leaves. Future helpers (hunger rolls, remorse rolls, contested rolls) compose the same primitives directly without going through `skill_check`.

#### V5 mechanics encoded

These rules are the source of truth for the helper implementations. Source: `docs/reference/v5-combat-rules.md`.

- Each die showing 6+ = 1 success.
- Each *pair* of 10s = 2 *additional* successes (so 2× 10 totals 4; 4× 10 totals 8; 3× 10 totals 5 — one pair plus a lone 10).
- Critical = at least one pair of 10s **and** the test passes.
- Messy critical = critical with ≥1 hunger die showing 10.
- Bestial failure = failed test with ≥1 hunger die showing 1.
- Total failure = 0 successes (can co-occur with bestial failure).
- Hunger dice never exceed pool size.

### §3.5 Frontend integration

#### File inventory

| Plan | New files | Modified files |
|---|---|---|
| 0 | — | `src/lib/bridge/api.ts`, `src/store/bridge.svelte.ts` |
| 1 | `src/lib/saved-characters/api.ts`, `src/store/savedCharacters.svelte.ts`, `src/components/SourceAttributionChip.svelte` | `src/tools/Campaign.svelte` |
| 2 | `src/lib/saved-characters/diff.ts`, `src/components/CompareModal.svelte` | `src/tools/Campaign.svelte` (wire Compare button into modal) |
| 3 | `src/lib/v5/api.ts` | — (no UI in Phase 1) |

#### Campaign view structure (post-Plan 1)

Twin sections (Layout B):

- **Live · N characters** — existing card grid, augmented with: Save locally button (no saved match), Update saved button + Compare button (saved match exists), drift badge (saved match exists and JSON differs), source attribution chip.
- **Saved · M characters** — new card grid below. Each card: drift badge (live counterpart differs) or offline badge (no live counterpart), Delete button, source attribution chip including saved-on date.

A character that is both live and saved appears in both sections.

#### Diff projection (`src/lib/saved-characters/diff.ts`)

```ts
interface DiffablePath {
  key: string;
  label: string;
  read: (c: BridgeCharacter | SavedCharacter['canonical']) => string | number | null;
}

const CANONICAL_PATHS: DiffablePath[] = [
  { key: 'name',                  label: 'Name',                   read: c => c.name },
  { key: 'hunger',                label: 'Hunger',                 read: c => c.hunger ?? null },
  { key: 'humanity',              label: 'Humanity',               read: c => c.humanity ?? null },
  { key: 'humanity.stains',       label: 'Stains',                 read: c => c.humanityStains ?? null },
  { key: 'health.max',            label: 'Health (max)',           read: c => c.health?.max ?? null },
  { key: 'health.superficial',    label: 'Health (superficial)',   read: c => c.health?.superficial ?? null },
  { key: 'health.aggravated',     label: 'Health (aggravated)',    read: c => c.health?.aggravated ?? null },
  { key: 'willpower.max',         label: 'Willpower (max)',        read: c => c.willpower?.max ?? null },
  { key: 'willpower.superficial', label: 'Willpower (superficial)', read: c => c.willpower?.superficial ?? null },
  { key: 'bloodPotency',          label: 'Blood Potency',          read: c => c.bloodPotency ?? null },
];

// Curated paths off the Foundry raw.system blob; programmatically generated from
// the WoD5e skill/attribute lists in docs/reference/foundry-vtm5e-paths.md.
const FOUNDRY_PATHS: DiffablePath[] = [/* …skills.<k>.value, attributes.<k>.value… */];

export const DIFFABLE_PATHS = [...CANONICAL_PATHS, ...FOUNDRY_PATHS];

export function diffCharacter(saved, live): DiffEntry[] {
  return DIFFABLE_PATHS
    .map(p => ({ before: p.read(saved), after: p.read(live), label: p.label, key: p.key }))
    .filter(({ before, after }) => before !== after)
    .map(({ before, after, label, key }) => ({
      key, label,
      before: before == null ? '—' : String(before),
      after:  after  == null ? '—' : String(after),
    }));
}
```

#### Specialty diffing (list comparator)

Specialties are stored on Foundry actors as Item documents with `type === 'speciality'`. Each item carries `name` (the specialty, e.g. `"small knives"`) and `system.skill` (the parent skill, e.g. `"melee"`). One actor can have multiple specialties per skill. Plan 2 ships a list comparator alongside the path-based diff:

```ts
function collectSpecialties(raw): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  for (const item of raw?.items ?? []) {
    if (item.type !== 'speciality') continue;
    const skill = item.system?.skill;
    if (!skill) continue;
    (out[skill] ??= []).push(item.name);
  }
  return out;
}

function diffSpecialties(saved, live): DiffEntry[] {
  if (saved.source !== 'foundry') return [];
  const savedMap = collectSpecialties(saved.raw);
  const liveMap  = collectSpecialties(live.raw);
  const skills = new Set([...Object.keys(savedMap), ...Object.keys(liveMap)]);
  const entries: DiffEntry[] = [];
  for (const skill of skills) {
    const before = (savedMap[skill] ?? []).sort().join(', ') || '—';
    const after  = (liveMap[skill]  ?? []).sort().join(', ') || '—';
    if (before !== after) {
      entries.push({ key: `specialty.${skill}`, label: `Specialty: ${cap(skill)}`, before, after });
    }
  }
  return entries;
}

// diffCharacter composes path-based and list-based:
export function diffCharacter(saved, live): DiffEntry[] {
  return [
    ...DIFFABLE_PATHS
      .map(p => ({ before: p.read(saved), after: p.read(live), label: p.label, key: p.key }))
      .filter(({ before, after }) => before !== after)
      .map(({ before, after, label, key }) => ({
        key, label,
        before: before == null ? '—' : String(before),
        after:  after  == null ? '—' : String(after),
      })),
    ...diffSpecialties(saved, live),
  ];
}
```

Two functions, two layers max (`diffSpecialties` → `collectSpecialties` → return); pure; no IPC. Roll20 saved characters skip specialty diffing via the `source !== 'foundry'` guard.

#### Still out of Phase 1 diff coverage

Embedded merits/flaws, discipline powers, and other Item-document-based character features remain deferred to Phase 2. They're list comparisons of the same shape as `diffSpecialties` — adding them later is one new `diffXxx` function composed into `diffCharacter`. Their requirements need character-editing scope to be settled first.

#### Per-source path applicability

`CANONICAL_PATHS` apply to both Roll20 and Foundry sources (both populate the canonical fields). `FOUNDRY_PATHS` reads guard on `c.source === 'foundry'` and return `null` for Roll20 saved characters; the diff filter treats `null === null` as no-difference, so they're silently filtered out. Practically, a Roll20 saved character diffs only against the 10 canonical paths. A `ROLL20_PATHS` array can be added the same way if Roll20 attribute paths ever need diff coverage — no code structure change.

#### Drift detection

Computed in TypeScript at render time: for each live character that has a saved match, run `diffCharacter` and check `length > 0`. Cheap with current `DIFFABLE_PATHS` size (~30 entries). If the list grows past ~50 entries, memoize on `(saved.id, live.lastSeen)`.

### §3.6 Error handling, dependencies, verification

#### Error handling per plan

Follows ARCHITECTURE.md §7: Rust commands return `Result<T, String>` with module-stable prefixes; frontend catches in API wrappers and surfaces via toast / inline state.

| Plan | Failure | Surfaces as |
|---|---|---|
| 0 | `Hello.protocol_version != 1` | desktop drops connection, logs warning, no event to frontend |
| 0 | `Error` envelope from module | new event `bridge://foundry/error` with `{ refers_to, code, message }` → toast |
| 0 | `bridge_get_source_info` for disconnected source | `Ok(None)` (absence is data) |
| 1 | Save existing `(source, source_id)` | `Err("db/saved_character.save: already saved; use update")` |
| 1 | Update missing `id` | `Err("db/saved_character.update: not found")` |
| 1 | Delete missing `id` | `Err("db/saved_character.delete: not found")` |
| 1 | `canonical` deserialize failure at IPC boundary | Tauri returns `Err(serde error)` automatically |
| 2 | Reader function throws | `try/catch` per-path; log + skip the entry, never fail the whole modal |
| 3 | Pure functions; can't fail at runtime | no `Result` types in helper signatures |

#### Plan dependencies

```
Plan 0 ────► Plan 1 ────► Plan 2          bridge proto → saved chars → compare
Plan 3                                    v5 helpers (independent)
```

- **Plan 0 → Plan 1**: Plan 1's `foundry_world` value comes from Plan 0's Hello extension. Plan 1's *schema* can be designed in advance (contract-only stub of `SourceInfo`); only the runtime population needs Plan 0 landed.
- **Plan 1 → Plan 2**: Plan 2 reads the `SavedCharacter` shape and the typed API wrappers from Plan 1.
- **Plan 3 ⫫ all others**: zero file overlap. Touches only `src-tauri/src/shared/v5/`, `src-tauri/src/tools/skill_check.rs`, `src/lib/v5/api.ts`, and `src-tauri/src/lib.rs`'s command list.

**Dispatch shape:** Plan 0 ships first. Plans 1 and 3 ship in parallel after Plan 0. Plan 2 ships after Plan 1. With `superpowers:using-git-worktrees`, Plan 1 and Plan 3 are truly independent worktrees.

The shared edit point is `lib.rs`'s `generate_handler![...]` macro — handle by line-level anti-scope per plan, with each plan adding only its own command identifiers.

#### Anti-scope (per ARCHITECTURE.md §11)

| Plan | MUST NOT touch |
|---|---|
| 0 | `db/saved_character.rs`, `shared/v5/`, `tools/Campaign.svelte`, any Plan 1/2/3 frontend file |
| 1 | `shared/v5/`, `bridge/foundry/types.rs` (frozen by Plan 0), `CompareModal.svelte`, `diff.ts` |
| 2 | `db/saved_character.rs` (frozen by Plan 1), `shared/v5/`, `bridge/`, any new Tauri command registration |
| 3 | `bridge/`, `db/`, any `.svelte` component, `tools/Campaign.svelte` |

#### Invariants cited

- **Plan 0**: §4 Bridge WebSocket protocol, §5 module boundaries, §6 (one connection per source), §7 (error handling), §8 (network surface stays loopback-only)
- **Plan 1**: §3 (storage strategy — SQLite, `tokio::fs`), §4 (Tauri IPC + frontend API wrappers), §5 (only `db/*` talks to SQLite), §6 (`PRAGMA foreign_keys = ON`)
- **Plan 2**: §4 (typed frontend API wrappers — components never call `invoke()`), §6 (CSS uses `:root` tokens, no hardcoded hex)
- **Plan 3**: §3 (no new storage), §10 (`#[cfg(test)] mod tests` per file)

#### Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

| Plan | `verify.sh` covers | Manual verification |
|---|---|---|
| 0 | `cargo test` (Hello-version + error-envelope routing); `npm run check`; `npm run build` | Boot dev app + Foundry world with new module → green pip + character cards show "source: FVTT — [world]"; trigger an error from Foundry → toast surfaces |
| 1 | `cargo test` (save/list/update/delete + UNIQUE); `npm run check`; `npm run build` | Save a live character → it appears in Saved section; click Save again → "already saved" toast; click Update saved → drift badge clears |
| 2 | `npm run check`; `npm run build` (no Rust changes) | Modify a Foundry actor → "drift" badge appears on the matching live card; click Compare → modal lists exactly the changed paths AND any specialty additions/removals/renames |
| 3 | `cargo test` (per-primitive test modules: pool, dice, interpret, difficulty, message, skill_check); `npm run check`; `npm run build` | None — no UI in Phase 1; correctness is unit-test-only |

---

## §4 Plan inventory (Phase 1)

| Plan | Title | Scope | Depends on |
|---|---|---|---|
| 0 | Bridge protocol consolidation | Hello extension; subscription protocol; error envelope; protocol version. Both `vtmtools-bridge/` (module) and `src-tauri/src/bridge/foundry/` (desktop). ~150 LoC across both sides. | none |
| 1 | Saved characters | New `saved_characters` table + migration; 4 new Tauri commands; new `savedCharacters.svelte.ts` store; Campaign view "Saved" section; save/update/delete buttons; source-attribution chip | Plan 0 (`SourceInfo` struct contract; `BridgeState.source_info` runtime population) |
| 2 | Compare modal | `src/lib/saved-characters/diff.ts` with `DIFFABLE_PATHS` + `diffSpecialties` list comparator; `CompareModal.svelte`; Compare button wiring in Campaign live cards. Pure-frontend plan — zero Rust changes. | Plan 1 (`SavedCharacter` shape; typed API wrappers) |
| 3 | V5 dice helper library | New `src-tauri/src/shared/v5/` module (7 files); `roll_skill_check` Tauri command; `src/lib/v5/api.ts` typed wrapper. No UI consumer in Phase 1 (Phase 2+ tools consume it) | none (parallel with Plans 0/1) |

---

## §5 Phases 2–5 sketches

### Phase 2 — Character editing

Components:

- **`character::set_field` router** — single helper, takes a character ref + path + value. Routes: saved-only → DB write; live-only → bridge `actor.update_field`; both → both. Same shape used for any future sheet type.
- **Add/remove advantage** — composes `actor.create_feature` / `actor.delete_item_by_id` (already in foundry helper roadmap) plus a desktop-only DB path.
- **Stat editing UI** — increment/decrement attributes & skills on a saved or live character. Reuses Phase 1's character cards.

Open questions resolved during Phase 1 brainstorm:
- ✅ Live characters are editable (during-play changes are required).

### Phase 3 — Roll mirroring

Components:

- **Roll-source toggle** per roll: "roll here" (uses Phase 1's `v5/skill_check`) vs. "roll in Foundry" (sends `game.roll_v5_pool` from foundry helper roadmap).
- **Foundry → tool result mirroring** — read back the resulting `ChatMessage` via Foundry `createChatMessage` hook. Reference: `docs/reference/foundry-vtm5e-rolls.md`. Requires a new `chat.*` umbrella subscription (uses Plan 0's subscription protocol).
- **Hunger roll, remorse roll, contested checks** — composites over the same Phase 1 primitives.
- **Roll log UI** — running list of recent rolls regardless of where they happened.

Open questions remaining:
- Passive vs. active mirror: when a player rolls in Foundry without the tool requesting it, do we still mirror? (Probably yes — passive mirror.)

### Phase 4 — Library sync

Components:

- **Push merit/dyscrasia → Foundry world** — one-click "send to active world" using `actor.create_feature`-class helpers.
- **Pull merits ← Foundry world** — scan the active world's items and actors' embedded items; surface non-canonical entries as importable. Requires new `item.*` umbrella subscription.
- **Source attribution surfacing** — imported items carry a `source_attribution` JSON column; UI shows "source: FVTT — [world]" badge.
- **Dedup & conflict** — what if a merit with the same name already exists locally? Skip / overwrite / version?

Open questions remaining:
- Generalized vs. Foundry-only: does library sync go through the same `BridgeSource`-style abstraction, or is it Foundry-only for v1?

### Phase 5+ — Foundry data surface (Level A — reserved by name only)

Future Foundry collections that vtmtools tools may consume. **No code, no plans, no commitments** — each is activated only when its consumer feature lands. The activating plan adds the wire variant, helper umbrella, hook subscription, and desktop consumer in one scope.

| Reserved umbrella | Likely consumer | Notes |
|---|---|---|
| `journal.*` | Domains Manager (link nodes ↔ journal pages); NPC notes | Pages are HTML strings with embedded `@UUID[…]` refs — round-tripping needs an HTML-or-markdown bridge |
| `scene.*` | Domains Manager (places linked to scenes) | Bridge metadata only (id, name, thumbnail); full scene blobs are too large |
| `item.*` | Library sync (Phase 4); Advantages library | World-level Item docs; embedded-on-actor items already arrive via `game.actors` |
| `chat.*` | Roll mirroring (Phase 3); roll log | `createChatMessage` hook is the entry point |
| `combat.*` | Future combat / conflict tool | Initiative, turn state, round counter |

The hooks for all of these follow the same pattern as the existing actor hooks; the typed-per-helper wire convention from the foundry helper roadmap means each new umbrella is additive.

---

## §6 Open questions

### Resolved during this brainstorm

- Layout for live ∩ saved characters → **B (twin cards)** with stat-comparer button on the live card.
- V5 helper library shape → **B (`shared/v5/` with primitives + composite)**.
- Diff implementation → **pure TS** with curated `DIFFABLE_PATHS` projection over canonical fields + Foundry skill/attribute paths.
- Helper composition style → **shallow / two-layer max**: orchestrator → leaves, no nested branching > 2 levels deep within a function body.
- "Listening but dead until used" → **Level A (reserved by name only)** — umbrellas named in this roadmap; no code until a consumer lands.
- Spec packaging → **one umbrella roadmap, four plans for Phase 1**.
- Phase 1 includes Plan 0 (bridge protocol consolidation) — front-load the protocol-level work before the eventual repo split.
- Live characters are editable in Phase 2 (during-play changes required).
- Specialty diffing pulled forward into Plan 2 (per advisor review) — covered in Phase 1 via a `diffSpecialties` list comparator alongside the path-based diff.
- Backward-compat for old (pre-Plan-0) modules: Hello deserializes with `Option<u32>`/`Option<Vec<String>>`; missing `protocol_version` → treated as `0` (legacy); missing `capabilities` → defaults to `["actors"]`. Avoids silent connection failures during the upgrade window.

### Outstanding (deferred to later phases)

- Phase 3: passive vs. active roll mirror.
- Phase 4: generalized vs. Foundry-only library sync abstraction.
- Phase 4: dedup strategy for imported items (skip / overwrite / version).

---

## §7 Repo split

The `vtmtools-bridge/` Foundry module is structurally independent of the desktop app — nothing in vtmtools imports from it; it's only consumed by Foundry at install time. Splitting it into its own repo for marketplace submission is **not blocked on any phase** and is reversible/postponable.

**Mechanism when split happens:** mirror-release pattern. Development continues in this monorepo; CI mirrors the `vtmtools-bridge/` directory to a published repo on tag. The marketplace `manifest`/`download` URLs point at the published repo. Editing one tree, releasing two.

**Why front-load the protocol (Plan 0) regardless of split timing:** post-split, every protocol-level change is a coordinated 2-repo PR; additive changes within existing umbrellas (new helpers, new wire variants) stay cheap. Plan 0 captures the protocol-level mechanisms (subscription, versioning, error envelope) so future cross-repo PRs are limited to additive changes.

The split timing itself is **TBD** — triggered by a real driver (marketplace listing intent, license divergence, or external contributor), not by a phase milestone.
