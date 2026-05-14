# Foundry Actions: Phase 0 + Phase 1 Design

> **Status:** implementable feature spec. Output of brainstorming for Phase 0 + Phase 1 of the Foundry helper library roadmap (`docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md` §7).
>
> **Goal:** scaffold the Foundry helper library (Phase 0) and ship the actor-umbrella primitives (Phase 1) as one combined infrastructure feature. No new user-visible features. Resonance and dyscrasia E2E flows must still work after the change.
>
> **Pre-locked by roadmap §6:** wire-type naming (`<umbrella>.<verb_noun>`), Rust builder signature, JS executor signature, error-prefix convention, JSDoc-or-equivalent contract requirement, composite helpers allowed.
>
> **Decided in this brainstorm:** A1 composition for `actor.apply_dyscrasia` (one wire message; JS executor composes primitives via direct function calls).

---

## §1 Architecture

### Phase 0 dispatch refactor

Refactor `vtmtools-bridge/scripts/bridge.js::handleInbound` from an `if/else` chain to a handler-map dispatch:

```js
import { handlers } from "./foundry-actions/index.js";

async function handleInbound(msg) {
  const handler = handlers[msg.type];
  if (!handler) {
    console.warn(`[${MODULE_ID}] unknown inbound type:`, msg.type);
    return;
  }
  try {
    await handler(msg);
  } catch (err) {
    console.error(`[${MODULE_ID}] handler ${msg.type} threw:`, err);
    ui.notifications?.error(`vtmtools: ${msg.type} failed — ${err.message}`);
  }
}
```

The `handlers` map is built by `foundry-actions/index.js` flattening per-umbrella `handlers` objects:

```js
import { handlers as actorHandlers } from "./actor.js";
import { handlers as gameHandlers } from "./game.js";
import { handlers as storytellerHandlers } from "./storyteller.js";

export const handlers = {
  ...actorHandlers,
  ...gameHandlers,
  ...storytellerHandlers,
};
```

Two properties of this shape:
- New helper = one entry in one umbrella file. `bridge.js` and `index.js` are not edited again after Phase 0.
- The `try/catch` wrapping in `handleInbound` applies uniformly. Currently each inline branch had ad-hoc handling (or none).

### Phase 0 wire-type renames

| Old wire type | New wire type |
|---|---|
| `update_actor` | `actor.update_field` |
| `create_item` | `actor.create_item_simple` |
| `apply_dyscrasia` | `actor.apply_dyscrasia` |

The Rust dispatch in `BridgeSource::build_set_attribute` (the `match name` inside `bridge/foundry/mod.rs`) is unchanged in shape — it still matches on `name` (the canonical attribute name). Only the `"type"` field of the JSON output changes.

### Phase 0 directory scaffolding

```
src-tauri/src/bridge/foundry/
├── mod.rs                    # build_set_attribute calls actions::actor::build_*
├── translate.rs              # unchanged
├── types.rs                  # gains 5 new payload structs
└── actions/                  # NEW
    ├── mod.rs                # pub mod actor; pub mod game; pub mod storyteller;
    ├── actor.rs              # 3 migrated builders + 5 new builders
    ├── game.rs               # empty stub (Phase 2 fills)
    └── storyteller.rs        # empty stub

vtmtools-bridge/scripts/
├── bridge.js                 # dispatch shell (handler-map + try/catch only)
├── translate.js              # unchanged
└── foundry-actions/          # NEW
    ├── index.js              # flattens handlers from all umbrellas
    ├── actor.js              # 8 executors (3 migrated + 5 new)
    ├── game.js               # empty stub: export const handlers = {};
    └── storyteller.js        # empty stub
```

### Phase 1 composition pattern (A1)

`apply_dyscrasia` keeps a single wire message (atomicity preserved within one event-loop tick) but its executor composes the new primitive functions:

```js
// vtmtools-bridge/scripts/foundry-actions/actor.js (sketch)

async function deleteItemsByPrefix(actor, { item_type, featuretype, name_prefix }) {
  const matches = actor.items.filter(
    (i) =>
      i.type === item_type &&
      (featuretype === undefined || i.system?.featuretype === featuretype) &&
      typeof i.name === "string" &&
      i.name.startsWith(name_prefix),
  );
  if (matches.length === 0) return 0;
  await actor.deleteEmbeddedDocuments("Item", matches.map((i) => i.id));
  return matches.length;
}

async function createFeature(actor, { featuretype, name, description, points }) {
  await actor.createEmbeddedDocuments("Item", [
    { type: "feature", name, system: { featuretype, description, points } },
  ]);
}

async function appendPrivateNotesLine(actor, { line }) {
  const current = actor.system?.privatenotes ?? "";
  const next = current.trim() === "" ? line : `${current}\n${line}`;
  await actor.update({ "system.privatenotes": next });
}

async function applyDyscrasia(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) return;
  if (msg.replace_existing) {
    await deleteItemsByPrefix(actor, {
      item_type: "feature",
      featuretype: "merit",
      name_prefix: "Dyscrasia: ",
    });
  }
  await createFeature(actor, {
    featuretype: "merit",
    name: `Dyscrasia: ${msg.dyscrasia_name}`,
    description: msg.merit_description_html,
    points: 0,
  });
  await appendPrivateNotesLine(actor, { line: msg.notes_line });
}

const wireExecutor = (fn) => async (msg) => {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[vtmtools-bridge] actor not found: ${msg.actor_id}`);
    return;
  }
  await fn(actor, msg);
};

export const handlers = {
  "actor.update_field": wireExecutor((actor, msg) =>
    actor.update({ [msg.path]: msg.value }),
  ),
  "actor.create_item_simple": wireExecutor(createItemSimple),
  "actor.append_private_notes_line": wireExecutor(appendPrivateNotesLine),
  "actor.replace_private_notes": wireExecutor(replacePrivateNotes),
  "actor.create_feature": wireExecutor(createFeature),
  "actor.delete_items_by_prefix": wireExecutor(deleteItemsByPrefix),
  "actor.delete_item_by_id": wireExecutor(deleteItemById),
  "actor.apply_dyscrasia": applyDyscrasia, // composite — manages own actor lookup
};
```

The `wireExecutor` HOF factors out the "look up actor by id, warn if missing" prelude that every primitive needs. `applyDyscrasia` opts out because it'd be wasteful to look up the actor three separate times for one composite operation.

## §2 Helper contracts

Each helper documented as: **wire shape** → **effect** → **failure modes** → **idempotency**.

### `actor.update_field` (migrated from `update_actor`)
- **Wire:** `{ type: "actor.update_field", actor_id: string, path: string, value: any }`
- **Effect:** `actor.update({ [path]: value })`. `path` is a Foundry dot-path like `"system.hunger.value"`.
- **Failure:** actor not found → console.warn + no-op. Invalid path silently fails inside Foundry.
- **Idempotency:** safe — same value yields same state.

### `actor.create_item_simple` (migrated from `create_item`)
- **Wire:** `{ type: "actor.create_item_simple", actor_id: string, item_type: string, item_name: string, replace_existing?: boolean }`
- **Effect:** Optionally delete all existing items where `i.type === item_type`; create one new Item with `{ type: item_type, name: item_name }`.
- **Failure:** actor not found → console.warn + no-op.
- **Idempotency:** with `replace_existing: true`, idempotent. Without, duplicates accumulate.

### `actor.apply_dyscrasia` (migrated from `apply_dyscrasia`, refactored to compose)
- **Wire:** `{ type: "actor.apply_dyscrasia", actor_id, dyscrasia_name, resonance_type, merit_description_html, notes_line, replace_existing }`
- **Effect:** Composite. Delete prior `Dyscrasia:`-prefixed merit Items + create new `Dyscrasia: <name>` merit + append notes line.
- **Failure:** actor not found → console.warn + no-op.
- **Idempotency:** with `replace_existing: true`, idempotent.
- **Composes:** `deleteItemsByPrefix` + `createFeature` + `appendPrivateNotesLine` (direct JS function calls, not wire round-trips).

### `actor.append_private_notes_line` (NEW)
- **Wire:** `{ type: "actor.append_private_notes_line", actor_id: string, line: string }`
- **Effect:** If `system.privatenotes` is empty (after trim), set to `line`. Otherwise append `\n<line>`.
- **Failure:** actor not found → console.warn + no-op.
- **Idempotency:** **NOT idempotent** — every call appends. Caller dedups if needed.

### `actor.replace_private_notes` (NEW)
- **Wire:** `{ type: "actor.replace_private_notes", actor_id: string, full_text: string }`
- **Effect:** `actor.update({ "system.privatenotes": full_text })`. Overwrites entirely.
- **Failure:** actor not found → console.warn + no-op.
- **Idempotency:** safe.

### `actor.create_feature` (NEW)
- **Wire:** `{ type: "actor.create_feature", actor_id, featuretype, name, description, points }`
- **Effect:** Create a `feature` Item with the given featuretype.
- **Failure:** actor not found → console.warn + no-op. Invalid `featuretype` (not in `merit`/`flaw`/`background`/`boon`) → Rust builder returns `Err("foundry/actor.create_feature: invalid featuretype: <given>")`.
- **Idempotency:** **NOT idempotent** — every call creates a new Item even if one with the same name exists. Caller dedups via `delete_items_by_prefix` first if needed.

### `actor.delete_items_by_prefix` (NEW)
- **Wire:** `{ type: "actor.delete_items_by_prefix", actor_id, item_type, featuretype?, name_prefix }`
- **Effect:** Delete all embedded Items where `i.type === item_type` AND (if `featuretype` given) `i.system.featuretype === featuretype` AND `i.name.startsWith(name_prefix)`. Case-sensitive prefix match.
- **Failure:** actor not found → console.warn + no-op. Empty `name_prefix` → Rust builder returns `Err("foundry/actor.delete_items_by_prefix: empty name_prefix is not allowed")` (refuse to match-all by accident).
- **Idempotency:** safe — running twice deletes once, then no-ops.

### `actor.delete_item_by_id` (NEW)
- **Wire:** `{ type: "actor.delete_item_by_id", actor_id: string, item_id: string }`
- **Effect:** `actor.deleteEmbeddedDocuments("Item", [item_id])`. Foundry no-ops if id doesn't exist.
- **Failure:** actor not found → console.warn + no-op. Bad item id → silent no-op (Foundry behavior).
- **Idempotency:** safe.

## §3 File plan

### Files created (Phase 0)
- `src-tauri/src/bridge/foundry/actions/mod.rs`
- `src-tauri/src/bridge/foundry/actions/actor.rs`
- `src-tauri/src/bridge/foundry/actions/game.rs` (empty stub)
- `src-tauri/src/bridge/foundry/actions/storyteller.rs` (empty stub)
- `vtmtools-bridge/scripts/foundry-actions/index.js`
- `vtmtools-bridge/scripts/foundry-actions/actor.js`
- `vtmtools-bridge/scripts/foundry-actions/game.js` (empty stub)
- `vtmtools-bridge/scripts/foundry-actions/storyteller.js` (empty stub)

### Files modified
- `src-tauri/src/bridge/foundry/mod.rs` — `build_set_attribute` calls into `actions::actor::build_*`; existing inline JSON moves into builder functions; the dyscrasia `#[cfg(test)]` block moves to `actions/actor.rs::tests` and asserts new wire-type strings.
- `src-tauri/src/bridge/foundry/types.rs` — add 5 new payload structs (`AppendPrivateNotesLinePayload`, `ReplacePrivateNotesPayload`, `CreateFeaturePayload`, `DeleteItemsByPrefixPayload`, `DeleteItemByIdPayload`).
- `vtmtools-bridge/scripts/bridge.js` — `handleInbound` becomes the handler-map dispatch shell; remove inline branches.

### Files explicitly unchanged
- All Svelte components. `Resonance.svelte`'s call to `bridge_set_attribute` with `name="dyscrasia"` (or `name="resonance"`) is unchanged; the wire-type rename happens below the Tauri command layer.
- All `src/lib/**/api.ts` typed wrappers.
- Roll20 source, `BridgeState`, Tauri command surface, `Cargo.toml`.
- Frontend stores (`bridge.svelte.ts`, `domains.svelte.ts`, etc.).
- `package.json`, `tauri.conf.json` (no version bump — see §6).

## §4 Testing strategy

- **Rust builders:** unit tests in `src-tauri/src/bridge/foundry/actions/actor.rs::tests`. Each builder gets at least one happy-path test plus error-case tests where applicable (`create_feature` invalid featuretype, `delete_items_by_prefix` empty prefix, `apply_dyscrasia` malformed payload).
- **JS executors:** no automated test infrastructure (per `ARCHITECTURE.md` §10 — introducing a JS test framework is a scope change requiring explicit user approval). Coverage relies on:
  - The Rust builder tests asserting the wire shape (one half of the contract).
  - Manual E2E verification of resonance + dyscrasia application after the refactor (the other half).
  - JSDoc contract blocks per executor as the source of truth for intended behavior.
- **Migration:** the existing 4 dyscrasia tests in `bridge/foundry/mod.rs::tests` move to `actions/actor.rs::tests`. They assert the NEW wire-type string `actor.apply_dyscrasia` (currently asserts `apply_dyscrasia`).

## §5 Verification gate

Phase 0+1 ships green when ALL of these pass:

1. `./scripts/verify.sh` exits 0 (npm check + cargo test + npm build).
2. The 4 migrated dyscrasia tests still pass with renamed wire-type assertions.
3. Each new actor primitive has at least one happy-path Rust unit test.
4. Manual E2E checklist passes:
   1. Start vtmtools, start Foundry world (vtmtools-bridge module installed and active), GM logs in.
   2. Resonance roll → "Apply to character" → resonance Item appears on the actor sheet.
   3. Dyscrasia roll → "Apply" → `Dyscrasia: <name>` merit Item appears, prior dyscrasia merit removed (if present), private notes shows new timestamped line.
   4. Foundry F12 console: no errors, no `unknown inbound type` warnings.
5. `git grep -nE 'update_actor|create_item|apply_dyscrasia' -- ':!docs/'` returns only the new namespaced names (`actor.update_field`, `actor.create_item_simple`, `actor.apply_dyscrasia`) in code and inline comments. (Excludes `docs/` because spec files, the roadmap, ADRs, and the implementation plan all reference old names historically.)

## §6 Anti-scope

Explicitly NOT in this feature:

- No new user-visible features. No Svelte components, no Tauri commands, no new API wrappers.
- No frontend changes whatsoever.
- No `game.*` or `storyteller.*` helpers (Phase 2+, separate spec).
- No JS test framework (separate scope-change discussion).
- No batch/transaction wire type for multi-op atomicity (YAGNI; A1 composition handles the dyscrasia case via a single inbound-handler tick).
- No changes to Roll20 source.
- No changes to the `BridgeSource` trait surface.
- No version bump. Phase 0+1 is library plumbing; the next feature release that consumes a new helper carries the version bump.
- No `ARCHITECTURE.md` updates yet. Deferred until first feature consuming the new library lands (per roadmap §10 — that's when the new "Add a Foundry helper" extensibility seam and the new pairing-convention invariant earn their place).

## §7 Open questions deferred to feature-time

These do NOT block Phase 0+1:

1. **`actor.create_power`** — discipline-power schema research. Deferred until first feature needs it (roadmap §9 Q2).
2. **Read-back surface** — does `CanonicalCharacter.raw.system` carry enough for "list current items" UIs? Deferred until first CRUD UI feature (roadmap §9 Q3).
3. **Permission model** — gating on actor ownership. Deferred until first feature touches non-GM-owned actors (roadmap §9 Q4).

## §8 Non-goals carried forward from roadmap

- No commitment to deliver any particular Phase 2+ helper as a result of Phase 0+1 landing.
- No commitment to migrate Roll20 source to a parallel helper-library structure.
- No `BridgeSource` trait widening to expose helper-style operations directly to Tauri commands.

## §9 Risks

| Risk | Mitigation |
|---|---|
| `apply_dyscrasia` refactor breaks the existing E2E flow | Manual E2E in §5 verification gate is mandatory; existing 4 unit tests catch wire-shape regressions |
| Foundry's `actor.update` and `createEmbeddedDocuments` behave differently when called rapidly in sequence (composition concern in A1) | Manual E2E specifically watches for this; if reproducible, fall back to A2 (self-contained `apply_dyscrasia`) and document the issue in this spec for Phase 2 to address |
| Renaming a wire type without updating both Rust and JS sides causes silent breakage | §5 step 5 (`git grep`) catches the most common version of this; manual E2E catches the rest |
| `wireExecutor` HOF abstraction turns out to be wrong shape (e.g., a future primitive needs the actor lookup conditional on a flag) | Cheap to undo — primitives can opt out the same way `applyDyscrasia` does. Pattern is convention, not lock-in |
