# Foundry Helper Library Roadmap

> **Status:** directional roadmap — not an implementable spec. Concrete features that consume helpers from this library each get their own brainstorm + spec + plan cycle.
>
> **Audience:** anyone designing a vtmtools feature that needs to read from or write to a Foundry game.

---

## §1 What this is

A categorized library of **helper functions** that live on the bridge layer between vtmtools (Tauri/Rust) and Foundry (browser/JS), to be composed by future feature tools. Helpers are organized into three **umbrellas** by domain of effect:

- **`actor.*`** — single-actor edits (character sheet manipulation: merits, flaws, backgrounds, attributes, notes, items).
- **`game.*`** — in-game/table mechanics (rolls landing in Foundry chat, chat messages attributed to actors).
- **`storyteller.*`** — GM-facing operations not tied to a single actor (placeholder umbrella; no helpers in v1).

The library replaces the current pattern where each feature inlines its Foundry API calls in `vtmtools-bridge/scripts/bridge.js`. After v1 the library is the canonical interaction surface; new features compose helpers, not raw `actor.update` / `createEmbeddedDocuments` calls.

## §2 Why a library (not inline-per-feature)

Three concrete signals:

1. The dyscrasia-apply work (`bridge.js:101-145`) inlined three operations (delete prior dyscrasia merits, create new merit, append notes line) that are obvious reuse candidates for any future merit/flaw/background CRUD feature.
2. User-stated future scope includes merit/flaw/background CRUD and in-Foundry rolls — both will reuse the same primitives across multiple features.
3. The current `BridgeSource::build_set_attribute` `match` statement scales poorly past ~5 wire types. A handler-map dispatch + categorized helper modules is a one-shot refactor that prevents `match`-statement sprawl.

The library is also the natural seam for documentation: each helper has one canonical contract (input shape, effect, failure modes, idempotency) that both feature consumers and future maintainers can rely on without reading implementation.

## §3 Wire-protocol decision: typed-per-helper (Approach A)

Decided against generic dispatch. Each helper owns its own typed wire-message variant. Tradeoffs accepted:

- **Verbosity** — each new helper requires a Rust struct in `types.rs` + a Rust builder in `actions/<umbrella>.rs` + a JS executor in `foundry-actions/<umbrella>.js`. ~3 files touched per helper.
- **Discoverability gain** — consumers see typed contracts on both sides of the wire. The library's value comes from clear contracts; the verbosity tax buys discoverability.
- **Compile-time safety on the Rust side** — payload shape errors caught at `cargo check` time on the builder side. The JS side has no such check; the Rust↔JS contract is enforced by integration testing.

Generic dispatch (Approach B) was rejected because the marginal modularity it adds (JS-only additions for new helpers) is small — almost every new feature already crosses the Rust/JS boundary anyway via a Tauri command and a frontend wrapper, so the "JS-only addition" story rarely applies in practice.

## §4 Architecture

### File layout

```
src-tauri/src/bridge/foundry/
├── mod.rs                    # BridgeSource impl, dispatch shell
├── translate.rs              # inbound (Foundry→canonical) translator (unchanged)
├── types.rs                  # wire-message types (one struct per helper)
└── actions/                  # NEW: typed payload builders, organized by umbrella
    ├── mod.rs                # re-exports
    ├── actor.rs              # build_create_feature, build_append_notes_line, etc.
    ├── game.rs               # build_roll_v5_pool, build_post_chat_as_actor, etc.
    └── storyteller.rs        # placeholder — no helpers in v1

vtmtools-bridge/scripts/
├── bridge.js                 # connection lifecycle, hello, dispatch shell
├── translate.js              # outbound→inbound, hookActorChanges (unchanged)
└── foundry-actions/          # NEW: helper executors, organized by umbrella
    ├── index.js              # builds the handler map for handleInbound
    ├── actor.js              # appendPrivateNotesLine, createFeature, deleteItemsByPrefix, etc.
    ├── game.js               # rollV5Pool, postChatAsActor, etc.
    └── storyteller.js        # placeholder
```

### Wire-protocol layering

- Each helper owns one wire-message `type`, dot-namespaced as `<umbrella>.<verb_noun>`. Examples: `actor.create_feature`, `game.roll_v5_pool`. The dot makes umbrella visible on the wire, matches the directory layout on both sides, and lets future per-umbrella interception (logging, rate-limiting) route by string-prefix.
- `bridge.js::handleInbound` becomes a one-line dispatcher: looks up `msg.type` in a map populated by `foundry-actions/index.js`. Adding a helper does NOT require editing `handleInbound`.
- Existing wire messages migrate during Phase 0:
  - `update_actor` → `actor.update_field`
  - `create_item` → `actor.create_item_simple`
  - `apply_dyscrasia` → `actor.apply_dyscrasia`

### Contract surface (every helper documents)

1. **Wire-message shape** — JSON schema with field names, types, required/optional.
2. **Effect** — what changes in Foundry (which actor fields, which embedded documents, which chat messages).
3. **Failure modes** — what causes a no-op (actor not found, missing permission), what causes an explicit error (invalid payload).
4. **Idempotency** — re-running with same input: safe? destructive? appends?

### Cross-language pairing

The Rust `actions/` subdirectory mirrors the JS `foundry-actions/` directory **by convention only**. There is no compile-time check that a Rust builder and a JS executor agree on the wire shape. Pairing is enforced by:
- Naming convention (helper name is identical on both sides modulo case style).
- Integration test in `src-tauri/src/bridge/foundry/mod.rs` per helper that round-trips a payload through the builder and asserts the resulting JSON shape.
- Manual E2E verification when a feature first lands.

## §5 Helper inventory

Format: `wire_type` — effect summary — status (✅ existing, ➕ new in v1, ⏳ deferred to feature-time).

### `actor.*` umbrella (9 helpers)

| Wire type | Effect | Status |
|---|---|---|
| `actor.update_field` | `actor.update({[dot_path]: value})` for any system path | ✅ migrated from `update_actor` |
| `actor.append_private_notes_line` | Append `\n<line>` to `system.privatenotes` (or set bare if empty) | ➕ extracted from apply_dyscrasia |
| `actor.replace_private_notes` | Overwrite `system.privatenotes` entirely | ➕ |
| `actor.create_feature` | Create `feature` Item with given `featuretype` (`merit`, `flaw`, `background`, `boon`) | ➕ generalizes apply_dyscrasia's merit creation |
| `actor.delete_items_by_prefix` | Delete embedded Items matching `{type, featuretype?, name_prefix}` filter | ➕ generic version of dyscrasia's "delete prior" |
| `actor.delete_item_by_id` | Delete one embedded Item by Foundry document id | ➕ |
| `actor.create_item_simple` | Bare-name Item create (resonance pattern) | ✅ migrated from `create_item` |
| `actor.apply_dyscrasia` | Composite: delete prior dyscrasia merits + create new merit + append notes line | ✅ migrated from `apply_dyscrasia` |
| `actor.create_power` | Create a discipline-power Item under a parent discipline | ⏳ needs WoD5e schema research |

### `game.*` umbrella (4 helpers)

| Wire type | Effect | Status |
|---|---|---|
| `game.roll_v5_pool` | Invoke WoD5e's V5 roll machinery (rouse, hunger, messy critical, bestial fail handled by Foundry); post result to chat as actor | ➕ load-bearing — see decision below |
| `game.post_chat_as_actor` | Post a chat message attributed to actor (no dice) — useful for action descriptions, GM narration | ➕ |
| `game.rouse_check` | 1d10 rouse check, increments hunger on failure | ⏳ verify whether `roll_v5_pool` covers this or needs its own |
| `game.request_roll_from_player` | Whisper a player a "click to roll" prompt | ⏳ Foundry's roll-prompt API needs verification |

**Load-bearing decision for `game.roll_v5_pool`:** invoke WoD5e's existing roll API (the `WOD5eDice` namespace exposed via `game.system` per `docs/reference/foundry-vtm5e-rolls.md`) rather than constructing `Roll` + `ChatMessage` manually. This delegates V5 dice mechanics (hunger, messy criticals, rouse, bestial fails) to WoD5e's implementation. Single source of truth; vtmtools is a thin shim over WoD5e's roll function. Decided.

### `storyteller.*` umbrella (0 helpers in v1)

Placeholder umbrella. Listed in the directory structure so it is named, namespace-reserved on the wire, and ready when first concrete feature lands. Likely future candidates (deliberately speculative, NOT in v1 scope):

- `storyteller.broadcast_chat` — GM whisper to all
- `storyteller.create_journal_entry` — scene notes
- `storyteller.advance_combat_round` — combat tracker integration

## §6 Naming conventions

Locked rules so future helpers do not drift:

1. **Wire-message `type` field** — lowercase, dot-namespaced: `<umbrella>.<verb_noun>`. Underscore separates words within the verb-noun; dot only separates umbrella from verb.
2. **Rust builder function** — `build_<verb_noun>(args...) -> Result<Value, String>`, lives in `actions/<umbrella>.rs`. Returns the wire-message JSON. Errors on payload validation failures with module-prefixed messages (`"foundry/actor.create_feature: ..."`) per `ARCHITECTURE.md` §7.
3. **JS executor function** — `<verbNoun>` (camelCase JS convention), lives in `foundry-actions/<umbrella>.js`. Receives the parsed `msg` object, returns a Promise. Surfaces user-visible failures via `ui.notifications?.error`; logs non-actionable issues (e.g., actor not found) via `console.warn`.
4. **Handler registration** — each `foundry-actions/<umbrella>.js` exports a `handlers` object mapping `wire_type` → executor. `index.js` flattens these into a single map for `handleInbound`.
5. **Documentation** — every helper carries a JSDoc-or-equivalent block stating: wire shape, effect, failure modes, idempotency. This is the user-facing API contract.
6. **Composite helpers** (like `actor.apply_dyscrasia`) — allowed but must explicitly document which primitives they bundle, so a future maintainer can see whether a new feature should use the composite or compose primitives directly.

## §7 Build order

Each phase produces something usable; later phases extend rather than rework.

### Phase 0 — Wire-protocol scaffolding (prep, no new helpers)

- Introduce dot-namespace by renaming the three existing wire types: `update_actor` → `actor.update_field`, `create_item` → `actor.create_item_simple`, `apply_dyscrasia` → `actor.apply_dyscrasia`.
- Refactor `bridge.js::handleInbound` from `if/else` chain to a handler-map dispatch.
- Create empty `src-tauri/src/bridge/foundry/actions/` and `vtmtools-bridge/scripts/foundry-actions/` directories with module stubs and the `index.js` loader.
- Existing features (resonance apply, dyscrasia apply) keep working — pure refactor, no behavior change.
- **Verification:** existing E2E flows for resonance and dyscrasia application still succeed end-to-end.

### Phase 1 — Actor primitives (unblocks all CRUD features)

- `actor.append_private_notes_line`, `actor.replace_private_notes`
- `actor.create_feature`, `actor.delete_items_by_prefix`, `actor.delete_item_by_id`
- After Phase 1: `actor.apply_dyscrasia` could be reimplemented at the consumer (a vtmtools tool) as a 3-call composition. Kept as a composite for clarity and backward compat.
- **Unblocks:** merit CRUD UI, flaw CRUD UI, background CRUD UI features.

### Phase 2 — Game-roll helpers (unblocks in-Foundry rolling features)

- **Research spike first:** confirm WoD5e's `WOD5eDice` API surface (function name, args, async behavior, ChatMessage construction). Update `docs/reference/foundry-vtm5e-rolls.md` with findings.
- `game.roll_v5_pool`, `game.post_chat_as_actor`
- Optional: `game.rouse_check` (only if `roll_v5_pool` does not naturally cover the rouse case).
- **Unblocks:** any feature that wants vtmtools to make rolls in the Foundry chat.

### Phase 3+ — Feature-driven additions

- New feature specs add helpers to their umbrella as needed (e.g. discipline-power editor adds `actor.create_power`).
- Helpers added to the library when the **second consumer materializes**; the first feature can use a one-off until then. (Avoids the N=1 extraction trap — extracting based on a single use case tends to lock in the wrong interface.)
- New umbrellas added if/when concrete features need them.

### Critical paths

- **Merit/flaw/background CRUD feature:** Phase 0 → Phase 1.
- **In-Foundry rolling feature:** Phase 0 → Phase 2 (Phase 1 not required but probably overlaps in time).

## §8 What's out of scope (v1)

Listed so future work does not silently drift in:

- **Storyteller helpers** — umbrella reserved, directory created, zero helpers. Storyteller features each get their own design pass when concrete.
- **Read-back helpers** beyond what `CanonicalCharacter.raw` already exposes. If a feature needs to enumerate current Items on an actor, that is a future extension (likely a richer `FoundryActor` shape in `bridge/foundry/types.rs` or a new on-demand "fetch full actor" helper). Spec'd at feature time, not now.
- **Player-facing prompts** (`game.request_roll_from_player`) — listed in inventory as ⏳ pending verification, not committed in v1. Foundry's roll-prompt API is non-trivial; defer until a feature needs it.
- **Combat tracker integration**, **scene management**, **journal entries** — speculative storyteller-umbrella territory. Reserved on the wire (`storyteller.*`) but not built.
- **Cross-actor operations** (e.g. "for every PC in the world, do X"). Helpers operate on a single `actor_id`. Cross-actor work is a feature-level concern that loops over single-actor helpers.
- **Generic dispatch** (Approach B from §3). Decided against; helpers stay typed-per-helper.
- **Migrations of player-data** between actors. Not a typical V5 GM workflow; would need a much richer API.

## §9 Open questions for feature-time

These do NOT block the v1 library but will need answers when the first feature in each area is spec'd:

1. **WoD5e roll API surface** — exact function name, signature, return shape of `WOD5eDice` (or whatever the V5 roll entry point is named). Resolved during Phase 2 research spike.
2. **Discipline-power schema** — Foundry Item structure for V5 discipline powers (parent discipline reference, level, cost, dice pool descriptor). Resolved when `actor.create_power` is needed.
3. **Read-back surface** — does `CanonicalCharacter.raw.system` carry enough for "list current merits" UIs, or does the bridge need to expose `actor.items` separately? Resolved at the first feature that needs it.
4. **Permission model** — does any helper need to gate on actor ownership (e.g., should `actor.create_feature` refuse to act on actors the GM doesn't own)? Currently every helper assumes GM-runs-everything since the bridge module is GM-only (`bridge.js:16`). Worth re-examining if a feature ever runs on player-owned data.

## §10 Relationship to ARCHITECTURE.md

This roadmap sits under the §2 Bridge domain and §9 Extensibility seams of `ARCHITECTURE.md`. When the v1 library lands, ARCHITECTURE.md should grow:

- A new bullet under §9 "Add a Foundry helper" describing the umbrella subdirectory + builder + executor + handler-map registration pattern.
- An updated §2 Bridge domain note pointing readers to the helper inventory rather than the pre-library `match` statement.

The Rust↔JS pairing convention (no compile-time check, integration tests + manual E2E enforce it) is a new invariant; add it to `ARCHITECTURE.md` §6 Invariants when v1 lands.

## §11 What this roadmap does NOT do

- Does not constitute permission to start implementation. Each phase needs its own brainstorm + spec + implementation plan when scheduled.
- Does not freeze the helper inventory. Feature-time discoveries can add, rename, or split helpers; the inventory in §5 is a v1 starting set, not a contract.
- Does not commit to a delivery timeline. Phases are an ordering, not a schedule.
- Does not commit to building every ⏳ helper. Those are placeholders for "if/when a feature needs this".
