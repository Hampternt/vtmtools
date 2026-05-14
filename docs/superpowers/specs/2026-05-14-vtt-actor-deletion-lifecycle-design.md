# VTT Actor Deletion Lifecycle

> Design for propagating Foundry actor deletions through the bridge, the
> saved-character store, and the Campaign + GM Screen UIs. Closes the gap
> where deleted-in-Foundry actors persisted in vtmtools after refresh and
> across app restart.

---

## Background

Deleting an actor in Foundry left the actor visible in vtmtools' Campaign
and GM Screen, both during the session and across `bridge_refresh` /
app-restart. Investigation surfaced three compounding causes:

1. **`deleteActor` is mis-routed.**
   `vtmtools-bridge/scripts/translate.js:41-55` registers the same handler
   for `updateActor`, `createActor`, AND `deleteActor`, all of which send
   `{ type: "actor_update", actor: <…> }`. On delete, Foundry hands the
   hook the just-deleted actor object; that object gets serialized and
   sent to the desktop as if it were a regular update. The desktop has
   no way to tell the difference.

2. **Bridge cache is merge-only.**
   `src-tauri/src/bridge/mod.rs:273-282` handles `CharactersUpdated` by
   `chars.insert(c.key(), c)` for every entry. There is no remove path.
   Even when Foundry sends a fresh `{ type: "actors", actors: [...] }`
   snapshot via `bridge_refresh` (omitting deleted actors), the cache
   retains the stale entries because the receiver can't distinguish a
   bulk snapshot from a single-actor update — both produce the same
   `InboundEvent::CharactersUpdated`.

3. **"Saved locally" has no delete UI.**
   `CharacterCardShell.svelte:33-45` only exposes *Save locally* /
   *Update saved* / *Compare*. The `delete_saved_character` IPC exists
   but no UI surface calls it. Once a row is saved, it persists in
   Campaign and GM Screen forever, even if the source VTT has long since
   removed the actor.

Effects 1 and 2 together explain why unsaved deletions persist; effect 3
explains why saved deletions persist; the combination is the bug.

---

## Goals

- Live bridge cache reflects each connected source's authoritative actor
  set (no monotonic growth across a session or across reconnects).
- Saved-character snapshots remain durable, but the UI clearly surfaces
  when a saved character has been deleted in its source VTT.
- One-click forget for saved-only rows, no confirm prompt.
- Defense-in-depth: explicit per-event deletion AND snapshot
  reconciliation, so a missed event doesn't silently desync state.

## Anti-scope (explicitly NOT in this spec)

- No "Restore from VTT" or "Recreate in Foundry" action. Forgetting is
  destructive on the saved side; recreating is a Foundry user action.
- No deletion audit log / "recently deleted" panel.
- No Roll20 extension changes. Roll20 saved characters are NOT
  snapshot-reconciled (no per-campaign disambiguator in the saved
  schema; reconciling across campaigns would falsely flag rows from
  other Roll20 campaigns). Roll20 deletion flagging waits on a future
  spec that either adds a `roll20_campaign` column or wires explicit
  `CharacterRemoved` events from the extension.
- No change to `bridge://characters-updated` event payload shape — still
  carries the merged `Vec<CanonicalCharacter>` it does today.
- No new Tauri commands. All new DB helpers are internal Rust-module
  exports called from `bridge/mod.rs`.
- No re-fetch debouncing on the saved-characters store. The store
  re-fetches on every `bridge://characters-updated` event (one local
  SQL query). If this becomes hot during combat, a 100ms debounce is
  cheap to add in a follow-up — not part of this spec.
- **No handling of Foundry world renames.** Reconciliation matches on
  exact string equality between `saved_characters.foundry_world` and
  the current Hello's `world_title`. If the user renames their world,
  pre-rename saved rows keep the old title and become invisible to
  reconciliation — they show as saved-only with no badge even when
  their actors are deleted in Foundry. This is a degradation (missing
  badges, not false positives) and strictly better than the
  cross-world false-positive it replaces. Long-term fix: migrate
  `foundry_world` to store `world_id` instead of `world_title` —
  separate spec.

## Invariants honored

- `ARCHITECTURE.md` §5 — only `db/*` talks to SQLite. New DB helpers
  live in `db/saved_character.rs` and are called from `bridge/mod.rs`;
  no `sqlx` invocation outside `db/*`.
- `ARCHITECTURE.md` §6 — `PRAGMA foreign_keys = ON`; the new column
  has no FK implications.
- `ARCHITECTURE.md` §10 — every plan task ending in a commit runs
  `./scripts/verify.sh` first.
- CLAUDE.md — every new `#[tauri::command]` updates §4 IPC inventory
  in the same commit. (This spec adds no new commands, so §4 IPC
  inventory is unchanged. §2 Bridge domain DOES need updating for
  the new `InboundEvent` variants.)

---

## Architecture

### §1 Bridge layer: wire protocol + event semantics

**JS wire (`vtmtools-bridge/scripts/translate.js`):**

Split `deleteActor` out of the shared `for (const ev of [...])` loop in
`hookActorChanges`. The `updateActor` and `createActor` hooks continue
to emit `{ type: "actor_update", actor: actorToWire(actor) }` as today.
The `deleteActor` hook emits a new shape:

```js
Hooks.on("deleteActor", (actor) => {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  socket.send(JSON.stringify({
    type: "actor_deleted",
    actor_id: actor.id,
  }));
});
```

Only `actor_id` ships — the deleted actor's body would be stale and
misleading. The shape matches the existing `item_deleted` precedent at
`translate.js:75-81`.

**Foundry inbound type (`src-tauri/src/bridge/foundry/types.rs`):**

New variant on `FoundryInbound`:

```rust
/// A Foundry actor was deleted. Triggered by the `deleteActor` hook in
/// `vtmtools-bridge/scripts/translate.js`. `actor_id` is the Foundry
/// actor `_id` (canonical `source_id`).
ActorDeleted {
    actor_id: String,
},
```

Mirrors `ItemDeleted`. No `actor` body — the deletion is the message.

**Canonical event (`src-tauri/src/bridge/source.rs`):**

`InboundEvent::CharactersUpdated(Vec<CanonicalCharacter>)` is *replaced*
by three more-precise variants:

```rust
pub enum InboundEvent {
    /// Bulk truth from one source — cache replaces this source's slice.
    CharactersSnapshot {
        source: SourceKind,
        characters: Vec<CanonicalCharacter>,
    },
    /// Single character changed — cache inserts/updates one entry.
    CharacterUpdated(CanonicalCharacter),
    /// Character removed from its source — cache evicts one entry.
    CharacterRemoved {
        source: SourceKind,
        source_id: String,
    },
    RollReceived(CanonicalRoll),
    ItemDeleted { /* unchanged */ },
}
```

`CharacterRemoved` is source-generic from day one. Foundry produces it
now; Roll20 will produce it later when its extension grows a delete
signal — no further canonical-layer change required.

**Source impl wiring:**

| Source variant in | Produces canonical event |
|---|---|
| Foundry `Actors { actors }` | `CharactersSnapshot { source: Foundry, characters: ... }` |
| Foundry `ActorUpdate { actor }` | `CharacterUpdated(...)` |
| Foundry `ActorDeleted { actor_id }` | `CharacterRemoved { source: Foundry, source_id: actor_id }` |
| Roll20 `Characters { characters }` | `CharactersSnapshot { source: Roll20, characters: ... }` |
| Roll20 `CharacterUpdate { character }` | `CharacterUpdated(...)` |

**Cache handler (`bridge/mod.rs::accept_loop` match):**

```text
CharactersSnapshot { source, characters }:
    chars.retain(|_, c| c.source != source)
    for c in characters: chars.insert(c.key(), c)
    emit bridge://characters-updated (full snapshot)
    if source == Foundry:
        // saved_characters.foundry_world stores what CharacterCardShell
        // writes on save, which today is SourceInfo.world_title (see
        // CharacterCardShell.svelte:18-21). Match the same column the
        // saver writes to keep the comparison consistent.
        world = state.source_info[Foundry].world_title  // captured from Hello
        if world.is_some():
            db_reconcile_vtt_presence(&world, &characters' source_ids)
        // else: world unknown — skip reconciliation (fail-safe; would
        // otherwise stamp every saved-foundry row from every world)
    // Roll20: no reconciliation (see anti-scope)

CharacterUpdated(c):
    chars.insert(c.key(), c)
    emit bridge://characters-updated
    side-effect: db_clear_deleted_in_vtt(c.source, c.source_id)
    // Note: largely cosmetic. Foundry's undo regenerates a new actor _id
    // in most cases, so the source_id rarely matches a previously-stamped
    // saved record. Snapshot reconciliation is the primary clear path.

CharacterRemoved { source, source_id }:
    chars.remove(&key(source, source_id))
    emit bridge://characters-updated
    side-effect: db_mark_deleted_in_vtt(source, source_id)
```

`db_mark_deleted_in_vtt` is not world-scoped — it operates on the
exact `(source, source_id)` of the deletion event, which is unambiguous
(only one saved row can match per the `UNIQUE(source, source_id)`
constraint). World-scoping matters only for the bulk reconciliation
path, where "absence from a snapshot" depends on which world the
snapshot came from.

**Defense-in-depth.** Two redundant deletion paths exist on purpose:

- **Explicit event** (`CharacterRemoved`) — fired by Foundry's
  `deleteActor` hook in real time. Precise, timestamped.
- **Snapshot reconciliation** — when a fresh `CharactersSnapshot`
  arrives and a previously-cached actor is absent, the source-slice
  replace drops it. Catches deletions that happened while vtmtools was
  closed, or any explicit event that was dropped.

The same redundancy applies to the saved-character stamp:
`db_reconcile_vtt_presence` stamps `deleted_in_vtt_at` for any saved
character whose source matches the snapshot but whose `source_id`
isn't in the snapshot's id set.

### §2 Saved-character backend: schema + DB helpers

**Migration (`src-tauri/migrations/0008_saved_character_deleted_in_vtt.sql`):**

```sql
ALTER TABLE saved_characters
    ADD COLUMN deleted_in_vtt_at TEXT;
```

Nullable, no default, no backfill. Existing rows get `NULL` — the
correct "not known to be deleted" state.

**Type mirror:**

| File | Change |
|---|---|
| `src-tauri/src/db/saved_character.rs` `SavedCharacter` | Add `pub deleted_in_vtt_at: Option<String>`. |
| `src/lib/saved-characters/api.ts` `SavedCharacter` interface | Add `deletedInVttAt: string \| null`. Serde rename via `#[serde(rename_all = "camelCase")]` on the Rust struct (verify present; add if missing). |

`save_character` and `update_saved_character` do NOT touch this field.
The field is owned exclusively by the bridge reconciliation paths.

**New DB-module helpers (`db/saved_character.rs`, internal — no `#[tauri::command]`):**

```rust
/// Set deleted_in_vtt_at = datetime('now') for the saved record matching
/// (source, source_id). Idempotent — if already set, the timestamp is
/// refreshed to the latest deletion. Returns Ok(false) if no row matched.
pub async fn db_mark_deleted_in_vtt(
    pool: &SqlitePool,
    source: SourceKind,
    source_id: &str,
) -> Result<bool, String>;

/// Set deleted_in_vtt_at = NULL for the saved record matching
/// (source, source_id). No-op if already NULL or row absent.
pub async fn db_clear_deleted_in_vtt(
    pool: &SqlitePool,
    source: SourceKind,
    source_id: &str,
) -> Result<bool, String>;

/// Foundry-only. For saved rows with source = 'foundry' AND
/// foundry_world = `foundry_world`:
///   - clear deleted_in_vtt_at if source_id is in `present_source_ids`
///   - set deleted_in_vtt_at = datetime('now') otherwise
/// One transaction, two UPDATEs regardless of N.
///
/// World-scoped because Foundry actor IDs are world-scoped — a snapshot
/// from world B must not affect saved characters from world A. If
/// `foundry_world` is empty or unknown, the caller MUST skip this
/// helper (we'd otherwise stamp every saved row from every other world).
/// The `foundry_world` value is whatever is in
/// `saved_characters.foundry_world` (today: `SourceInfo.world_title`).
/// Saves whose `foundry_world` is NULL — older rows or world-less saves —
/// are exempt: SQL `=` excludes NULL by definition.
///
/// **Empty `present_source_ids` MUST be handled explicitly.** SQLite's
/// `WHERE source_id NOT IN ()` is a syntax error (empty IN-list invalid).
/// Implementation guards: if `present_source_ids.is_empty()`, run the
/// "stamp all matching" branch only (`UPDATE ... SET deleted_in_vtt_at
/// = datetime('now') WHERE source = 'foundry' AND foundry_world = ? AND
/// deleted_in_vtt_at IS NULL`); skip the clear branch entirely.
pub async fn db_reconcile_vtt_presence(
    pool: &SqlitePool,
    foundry_world: &str,
    present_source_ids: &[String],
) -> Result<ReconcileStats, String>;

pub struct ReconcileStats {
    pub stamped: u64,  // rows where deleted_in_vtt_at was newly set
    pub cleared: u64,  // rows where deleted_in_vtt_at was cleared
}
```

Roll20 has no per-campaign disambiguator on `saved_characters` and no
delete signal on the wire, so Roll20 `CharactersSnapshot` events do NOT
trigger reconciliation. Roll20 snapshots still drive cache replacement
(the live-data bug fix applies to both sources); only the
saved-character flagging is Foundry-only.

**Bridge → DB call sites** in `bridge/mod.rs::accept_loop` mirror the
existing `ItemDeleted` precedent (mod.rs:288-322):
`handle.try_state::<DbState>().map(|s| s.0.clone())`, then call the
helper. Errors logged via `eprintln!`, never propagated — a missed
stamp is recoverable; a crashed accept loop is not.

### §3 Frontend UX: badge + Forget button

**`savedCharacters` store (`src/store/savedCharacters.svelte.ts`):**

- `delete(id)`: confirm already wired (calls `delete_saved_character`,
  removes from `list`).
- `ensureLoaded()`: after the initial `listSavedCharacters()` fetch,
  subscribe to `bridge://characters-updated` and call `this.refresh()`
  (re-fetch full list) on every event. Cheap — local SQLite, dozen-row
  query. This is how the badge appears in real time without a manual
  reload.

**`CharacterCardShell.svelte` rail:**

Current (saved branch): `[Compare] [Update saved]`.
New: `[Compare] [Update saved] [Forget saved]`.

```svelte
{#if saved}
  …existing buttons…
  <button type="button" class="btn-save btn-destructive"
    onclick={() => savedCharacters.delete(saved.id)}
    disabled={savedCharacters.loading}>Forget saved</button>
{/if}
```

No confirm prompt — single-user offline tool, one-click forget. The
button uses a `btn-destructive` modifier composed from existing
color tokens (no new tokens, no hardcoded hex — §6 invariant).

**"Deleted" badge:**

Shown in the rail next to the existing `drift` badge when
`saved?.deletedInVttAt != null`:

```svelte
{#if saved?.deletedInVttAt}
  <span class="vtt-deleted-badge"
    title="Deleted in {sourceLabel(saved.source)}">deleted</span>
{/if}
```

Same chip shape as the `drift` badge. Color derived from existing
tokens (a muted accent — implementation will pick the specific
composition; no new tokens introduced).

**No card-body dimming.** Per design decision: visual differentiation
is the badge alone. The card body renders unchanged.

**`CharacterRow.svelte` (GM Screen):**

The row header strip needs the same `{#if saved?.deletedInVttAt}`
badge. `CharacterRow` currently receives `BridgeCharacter`; the
implementation will add a `saved?: SavedCharacter | null` prop and
read the badge from there. The `GmScreen.svelte` derivation already
has access to saved-character data via `savedAsBridge` for saved-only
rows; pairing with live rows requires a small lookup (`savedCharacters.findMatch(char)`)
analogous to `Campaign.svelte:23`.

**Campaign + GM Screen:** No logic changes to the list-derivation code
(`liveWithMatches`, `savedOnly`, `displayCharacters`). The badge is a
property of the rendered card/row only.

### §4 ARCHITECTURE.md updates

- §2 Bridge domain: replace the documented `InboundEvent` enum with the
  new 3-variant + RollReceived + ItemDeleted shape. Add a sentence
  describing the snapshot-replace semantic.
- §4 IPC commands: NO change (no new `#[tauri::command]`).
- §6 Invariants: add bullet — "The merged characters cache is
  source-slice-authoritative on `CharactersSnapshot`; the cache never
  carries entries from a source whose latest snapshot omitted them."
- §10 Testing: list the new test modules.

---

## Implementation order

Replacing `InboundEvent::CharactersUpdated` with three new variants is
not a separable change — it touches the enum in `bridge/source.rs`, both
source impls (`bridge/foundry/mod.rs`, `bridge/roll20/mod.rs`), and the
cache handler (`bridge/mod.rs::accept_loop`) simultaneously. Any partial
state fails to compile. The order below groups these into the one
atomic commit, per `feedback_atomic_cluster_commits`.

The plan that follows codifies this in plan-task form; listed here for
completeness:

1. **Migration 0008 + Rust type-only additions.**
   - `0008_saved_character_deleted_in_vtt.sql`.
   - `SavedCharacter.deleted_in_vtt_at: Option<String>` field.
   - `FoundryInbound::ActorDeleted { actor_id: String }` variant.
   These compile cleanly without touching anything else.
2. **`db/saved_character.rs` helpers + tests.**
   `db_mark_deleted_in_vtt`, `db_clear_deleted_in_vtt`,
   `db_reconcile_vtt_presence`. Standalone — no callers yet.
3. **Bridge event-shape cutover (atomic combined commit).**
   - Replace `InboundEvent::CharactersUpdated(Vec<…>)` with
     `CharactersSnapshot { source, characters }`,
     `CharacterUpdated(…)`, `CharacterRemoved { source, source_id }`.
   - Update `bridge/foundry/mod.rs::handle_inbound` to emit the new
     events (including the new `ActorDeleted` arm).
   - Update `bridge/roll20/mod.rs::handle_inbound` analogously
     (Snapshot for `Characters`, Updated for `CharacterUpdate`; no
     CharacterRemoved emitter on Roll20 today).
   - Update `bridge/mod.rs::accept_loop` to handle the three new
     variants, including the DB-helper side-effects from §1.
   - Add `handle_inbound` unit tests in `bridge/foundry/mod.rs` for
     the `actor_deleted` arm; add the `actor_deleted` round-trip
     deserialize test in `bridge/foundry/types.rs`.
   This is one commit. Splitting it fails compile or produces a
   runtime-broken intermediate.
4. **JS wire change.** `vtmtools-bridge/scripts/translate.js` splits
   `deleteActor` out, emits `actor_deleted`.
5. **Frontend type mirror.** `src/types.ts`,
   `src/lib/saved-characters/api.ts` — add `deletedInVttAt`.
6. **`savedCharacters` store wiring.** Subscribe to
   `bridge://characters-updated`, refresh the list on event.
7. **`CharacterCardShell.svelte`.** Forget button + badge.
8. **`CharacterRow.svelte`.** Badge surface for GM Screen rows.
9. **`ARCHITECTURE.md`.** §2, §6, §10 updates as listed above.

---

## Verification

`./scripts/verify.sh` is the gate (per CLAUDE.md). New Rust tests:

- **`bridge/foundry/types.rs`** — `actor_deleted` JSON round-trips
  through `FoundryInbound` deserialize. Mirrors the existing
  `item_deleted` test at `types.rs:233`.
- **`bridge/foundry/mod.rs`** — `handle_inbound` on `actor_deleted`
  produces exactly one `InboundEvent::CharacterRemoved` with the
  expected `(source, source_id)`. Mirrors
  `item_deleted_inbound_produces_modifier_reap_event` at
  `mod.rs:99-117`.
- **`db/saved_character.rs`** — `db_mark_deleted_in_vtt`,
  `db_clear_deleted_in_vtt`, and `db_reconcile_vtt_presence` against
  an in-memory SQLite. Cover idempotency, no-row-match,
  reconcile-with-empty-snapshot, AND world-scoping: a reconcile call
  with `foundry_world = "world_b"` must NOT stamp rows whose
  `foundry_world = "world_a"`. This world-isolation test is the most
  important — it's the regression guard for the reconciliation bug.
- **`bridge/mod.rs`** — optional: unit-level test for the
  `CharactersSnapshot` handler replacing source slices correctly. The
  accept-loop body isn't trivially testable today; consider extracting
  the per-event match into a `handle_event(state, event)` helper to
  enable testing without spinning up a WS server.

No frontend tests (ARCHITECTURE.md §10 — no frontend test framework,
deliberate).

**Manual smoke test** (run as part of the plan's final verification):

1. With Foundry connected and one actor `[Save locally]`'d, delete the
   actor in Foundry. Expect: live row disappears from Campaign and
   GM Screen; saved row shows a "deleted" badge with a `[Forget saved]`
   button.
2. Click `[Forget saved]`. Expect: row vanishes immediately, no
   confirm prompt.
3. Delete an unsaved actor in Foundry. Expect: row disappears entirely
   from both tools, no residue, no badge.
4. Recreate an actor in Foundry that was previously saved + flagged
   deleted (same `_id` if Foundry preserves it on undo; otherwise the
   `CharacterUpdated` path won't match the saved key — that's
   acceptable, the saved record stays flagged deleted until forgotten).
5. Restart vtmtools with a previously-deleted-and-saved actor.
   Expect: badge is still present (persisted via the new DB column).
6. Open Campaign while Foundry is connecting (race). Expect: no
   transient false-positive badges — the initial `CharactersSnapshot`
   handler is the first event to reconcile saved presence.
7. **World-switch isolation.** Save characters from Foundry world A.
   Close Foundry, open Foundry to world B (a separate world), let
   it connect to vtmtools. Expect: saved characters from world A
   show NO "deleted" badge (their world doesn't match the snapshot's
   world; reconciliation correctly ignores them). The live list
   shows world B's actors only.

---

## Open questions

None remaining at spec time. Implementation may surface minor questions
(token color choice for the badge, exact `CharacterRow` insertion point)
— those are inside the plan's authority and don't reopen the spec.
