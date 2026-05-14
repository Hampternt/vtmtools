# Domains Auto-Relate on Child Creation — Design

**Date:** 2026-04-19
**Status:** Draft
**Scope:** Frontend-only extension to the Domains Manager's "+ Add child" flow. No backend, schema, or Tauri-command changes. Depends on the v1 Domains Manager UI (spec: `2026-04-19-domains-manager-ui-design.md`).

---

## Overview

Extend `NodeForm.svelte` so that when a user creates a child node via the tree's "+ Add child" action, they can optionally write a **second, non-`contains`** edge between the new child and its parent in the same save. The user controls the edge type (free-text with autocomplete), the direction (parent→child, child→parent, or both), and an optional description. The enable/type/direction choices persist session-stickily — the form remembers the user's last pick and pre-fills it on the next "+ Add child" until the app restarts.

This is a quality-of-life collapse of what currently requires three clicks and a context-switch (create child → navigate to child → open EdgesPanel → "+ Add relationship" → pick parent as target → pick type) into one form.

---

## Scope

### In scope

- New sticky reactive state in `src/store/domains.svelte.ts`:
  - `autoRelatePref = $state({ enabled, edgeType, direction })` — session-only, module-top-level `$state` (same wrapper-object idiom the store already uses for `session`, `cache`, `status`).
- New UI section in `NodeForm.svelte`, rendered **only** when `node == null && parentId != null` (the "Add child node" case — not edit, not root-add):
  - Checkbox: "Also add a relationship to parent".
  - Edge-type input with `<datalist>` autocomplete populated from distinct non-`contains` `edge_type` values in `cache.edges` (same derivation as `EdgePicker`'s `knownEdgeTypes`).
  - Direction radio: **parent → new** | **new → parent** | **both**.
  - Optional description input (form-local state; not sticky).
- Extended `save()` in `NodeForm`: after a successful `createNode` and successful `contains` edge, emit 1 or 2 additional `createEdge` calls for the auto-relation, gated on a `containsOk` flag.
- Validation: checkbox on + empty type → inline error, same surface as the existing "Label is required." validation.
- Partial-failure handling: consistent with ARCHITECTURE.md §7 — raw IPC errors surface as inline `localError`; the form's existing `friendlyError` helper maps known cases.

### Deferred (explicitly out)

- **Per-parent persisted preference** (e.g., storing "children of Factions auto-relate as `member-of / child→parent`" on the node row). Would require a schema change. Revisit when the character-builder lands and drag-drop-from-library flows need a default-relation hook.
- **Asymmetric-label "both" mode** (different `edge_type` for each direction — e.g., `sired` parent→child paired with `is-childe-of` child→parent). Revisit if users routinely edit one of the two auto-created edges after creation to rename it.
- **Atomic transactional backend command** (approach Y from brainstorming). Matches the existing partial-success posture of the `contains`-auto case (NodeForm.svelte:95) and doesn't foreclose a future Y-shaped upgrade; the UI would remain unchanged when the transactional command lands.
- **Auto-relate on edit or on root creation.** No parent context exists in either case.
- **Best-effort rollback on auto-relate failure** (approach Z from brainstorming). Inverts the partial-success posture for one feature; rejected.

---

## Behavior

### First open of a session

User clicks "+ Add child" on a tree row. The form mounts with `parentId = parent.id`. The new section renders with:
- Checkbox unchecked.
- Direction radio pre-selected to **new → parent** (child-to-parent). Rationale: the dominant real-world patterns (member-of, located-in, allied-with, knows, reports-to) read most naturally as "new element → parent element." Callable out as a flagged decision for the user's spec-review step.
- Type and description inputs empty.

When the checkbox is unchecked, the type/direction/description rows render disabled or are hidden (visual decision during implementation — see Risks §).

### Subsequent opens in the same session

Form reads `autoRelatePref` for the checkbox/type/direction state and pre-fills accordingly. Description always resets to empty (it is form-local, not stored in `autoRelatePref`).

### After app restart

`autoRelatePref` re-initializes to its defaults (unchecked, empty type, `child-to-parent`). In-memory only; no SQLite or settings-file persistence.

### Save

1. Existing validations: `label.trim()`, `nodeType.trim()`, `session.chronicleId != null`.
2. **New validation:** if `node == null && parentId != null && autoRelatePref.enabled && !autoRelatePref.edgeType.trim()` → set `localError = 'Relationship type is required when auto-relate is on.'` and return. Consistent with the form's existing validation surface rather than silent-skip.
3. `createNode(...)` — unchanged.
4. If `parentId != null`: `createEdge(... 'contains' ...)` in a try/catch that sets `containsOk = false` on failure (existing behavior: `localError = 'Node created, but linking to parent failed: …'`).
5. If `containsOk && autoRelatePref.enabled && autoRelatePref.edgeType.trim()`:
   - Compute `type = autoRelatePref.edgeType.trim()` and `desc = autoRelateDescription` (form-local).
   - In a single try/catch around both directions:
     - If `autoRelatePref.direction !== 'child-to-parent'`: `createEdge(chronicleId, parentId, savedId, type, desc, [])`.
     - If `autoRelatePref.direction !== 'parent-to-child'`: `createEdge(chronicleId, savedId, parentId, type, desc, [])`.
   - On failure: `localError = 'Node created, but auto-relation failed: ' + friendlyError(String(e))`. Node and `contains` persist.
6. `refreshNodes()`, `refreshEdges()`, `selectNode(saved.id)` — unchanged.
7. **Only call `onsave(saved)` when `localError` is empty.** If any prior step set `localError` (failed `contains`, failed auto-relate), the form stays mounted so the error remains visible; the user reads it and clicks Cancel to close. This is a one-line tightening of the existing `save()` and also fixes the pre-existing `contains`-auto flash-and-disappear error (previously the form unmounted before the user could read "Node created, but linking to parent failed: …").

The `!== 'child-to-parent'` / `!== 'parent-to-child'` pair expresses "both" without a third branch: `'both'` passes both guards; each single value passes exactly one.

---

## Architecture

### State module — `src/store/domains.svelte.ts`

One new export:

```ts
export const autoRelatePref = $state<{
  enabled: boolean;
  edgeType: string;
  direction: 'parent-to-child' | 'child-to-parent' | 'both';
}>({
  enabled: false,
  edgeType: '',
  direction: 'child-to-parent',
});
```

Module-top-level `$state` follows the same wrapper-object pattern used by the existing `session`, `cache`, and `status` exports. Svelte 5 runes can't be reassigned across module boundaries, but property mutations on a `$state`-wrapped object propagate reactively — the form's `bind:` directives will read and write through this object.

No persistence layer: no SQLite column, no migration, no settings file, no localStorage. App restart clears to the initial values.

### Component — `src/lib/components/domains/NodeForm.svelte`

Imports:

```ts
import { session, cache, refreshNodes, refreshEdges, selectNode, autoRelatePref } from '../../../store/domains.svelte';
```

New form-local state (alongside `label`, `nodeType`, `description`, etc.):

```ts
let autoRelateDescription = $state('');
```

Description is intentionally form-local, not sticky: descriptions are per-edge content ("joined 1920", "defected 2019") and a pre-filled stale description on NPC #2 would be surprising.

New derived for the type autocomplete list (mirrors `EdgePicker`'s derivation):

```ts
const knownEdgeTypes = $derived(
  Array.from(new Set(
    cache.edges.map(e => e.edge_type).filter(t => t !== 'contains')
  )).sort()
);
```

Template addition, placed between the existing Properties section and the form-level error/action rows, rendered only when `node == null && parentId != null`:

```svelte
{#if node == null && parentId != null}
  <div class="auto-relate">
    <label class="checkbox">
      <input type="checkbox" bind:checked={autoRelatePref.enabled} />
      Also add a relationship to parent
    </label>

    {#if autoRelatePref.enabled}
      <div class="field">
        <label for="nf-ar-type">Relationship type</label>
        <input id="nf-ar-type" list="nf-ar-types" bind:value={autoRelatePref.edgeType}
               placeholder="member-of, located-in, allied-with…" />
        <datalist id="nf-ar-types">
          {#each knownEdgeTypes as t (t)}
            <option value={t}></option>
          {/each}
        </datalist>
      </div>

      <fieldset class="direction">
        <legend>Direction</legend>
        <label><input type="radio" bind:group={autoRelatePref.direction} value="parent-to-child" /> parent → new</label>
        <label><input type="radio" bind:group={autoRelatePref.direction} value="child-to-parent" /> new → parent</label>
        <label><input type="radio" bind:group={autoRelatePref.direction} value="both" /> both</label>
      </fieldset>

      <div class="field">
        <label for="nf-ar-desc">Description (optional)</label>
        <input id="nf-ar-desc" bind:value={autoRelateDescription} />
      </div>
    {/if}
  </div>
{/if}
```

Scoped styles follow the existing `.field`, `.form`, `.btn` aesthetic already in `NodeForm.svelte`; no hex colors (CLAUDE.md §6 / ARCHITECTURE.md §6 — use `:root` tokens).

Save logic extension as specified in **Behavior / Save** above.

### Data flow

1. User clicks "+ Add child" on parent row (`DomainTree.svelte`) → `NodeForm` mounts with `parentId = parent.id`.
2. Form renders auto-relate section; checkbox / type / direction read reactively from `autoRelatePref`.
3. User edits fields; `bind:` directives write straight through to `autoRelatePref` (persisting the memory for the next open) and to the form-local `autoRelateDescription`.
4. On Save: validation → `createNode` → `createEdge('contains')` (with `containsOk` flag) → auto-relate `createEdge` calls (gated on `containsOk && enabled && edgeType.trim()`) → refresh + select.
5. `EdgesPanel` on the now-selected new child reactively shows the auto-created relation(s) in the appropriate Outgoing/Incoming group. Navigating back to the parent row shows the mirrored entry in the parent's panel.

---

## Error Handling

- `createNode` failure → existing path: `localError = friendlyError(String(e))`; form stays open, no node exists.
- `contains` edge failure (existing case) → `containsOk = false`, `localError = 'Node created, but linking to parent failed: ...'`. Auto-relate block is skipped (guarded by `containsOk`).
- Auto-relate edge failure → `localError = 'Node created, but auto-relation failed: ' + friendlyError(...)'`. Node and `contains` persist. The form **stays open** on any non-empty `localError` (see Behavior step 7) so the user can actually read the message; closing is user-initiated via Cancel. Once closed, the user can retry the relation via `EdgesPanel`.
- UNIQUE constraint on auto-relate (e.g., user also manually created the same edge earlier) → `friendlyError` already maps `UNIQUE constraint failed` to `"That relationship already exists."` — reused without change.
- "Both" direction with one side colliding: the first failing `createEdge` throws and we exit the try/catch; one of the two edges may have been written, the other was not. User sees the error and can inspect via `EdgesPanel`. Acceptable for v1; the alternative (independent try/catch per direction, best-effort) is a future polish.

This posture (partial-success surfaced inline, user can retry) is deliberately aligned with ARCHITECTURE.md §7 and with the existing `contains`-auto handling (NodeForm.svelte:95).

---

## Testing Strategy

- **Rust tests:** none. No backend changes.
- **Frontend:** no test framework in the repo per ARCHITECTURE.md §10 — correctness verified via:
  - `npm run check` — TypeScript + Svelte checks (new store export, new component bindings type-check).
  - `npm run build` — production build proves no broken imports.
  - `./scripts/verify.sh` — aggregate gate. Must be green before declaring done.
- **Manual smoke test** (author walks through in dev):
  1. Create chronicle. Create root node "Anarch Coterie".
  2. "+ Add child" on Anarch Coterie. Confirm the auto-relate section appears. Confirm checkbox is **unchecked** on first open.
  3. Check the box. Type `member-of`. Leave direction at `new → parent`. Leave description empty. Save "Nines Rodriguez".
  4. Verify: Nines's `EdgesPanel` has 1 **outgoing** `member-of → Anarch Coterie`. Anarch Coterie's `EdgesPanel` has 1 **incoming** `member-of ← Nines Rodriguez`.
  5. "+ Add child" on Anarch Coterie again. Verify: checkbox is **checked**, type is pre-filled `member-of`, direction is `new → parent`. Description is **empty** (not sticky).
  6. Change direction to `both`, type `"allied-with"`, save "Jack Nine-Stakes". Verify: two rows appear — one outgoing, one incoming — both typed `allied-with`.
  7. Add a root node. Verify: auto-relate section does NOT render.
  8. Open an existing node and click "✎ Edit". Verify: auto-relate section does NOT render in edit mode.
  9. Check auto-relate, leave type empty, click Save. Verify: inline error "Relationship type is required when auto-relate is on." — no node created.
  10. Restart app. "+ Add child" on any parent. Verify: checkbox is **unchecked** again (session cleared).

---

## File List

**Modified:**

- `src/store/domains.svelte.ts` — add `autoRelatePref` `$state` export (~10 lines).
- `src/lib/components/domains/NodeForm.svelte` — import `autoRelatePref`, add `autoRelateDescription` local state and `knownEdgeTypes` derived, add templated auto-relate section, extend `save()` with validation + gated auto-relate block, add scoped styles (~70 lines total).

**Unchanged:**

- Backend (`src-tauri/**`) — no schema, no migrations, no new Tauri command, no test changes.
- `src/lib/domains/api.ts` — existing `createEdge` wrapper is sufficient.
- Other domain components: `DomainTree.svelte`, `EdgesPanel.svelte`, `EdgePicker.svelte`, `NodeDetail.svelte`, `PropertyEditor.svelte`.

---

## Risks / Open Questions

- **Sticky-after-context-switch surprise.** User bulk-adds 10 NPCs under "Anarch Coterie" with auto-relate on, then clicks "+ Add child" under a totally different parent (say a place node) and doesn't notice the checkbox is still on. Mitigation: the section is visually prominent, and a single unchecked click recovers. Not worth cross-parent heuristics for v1.
- **Inner fields are hidden, not disabled, when checkbox is unchecked.** The Architecture template snippet commits to `{#if autoRelatePref.enabled}` — less visual noise, cleaner when the user isn't intending to auto-relate. The direction radio's pre-selected value still exists in the store; it's just not rendered. Noted here (not ambiguous) so implementation doesn't second-guess it.
- **Default direction = `child-to-parent` is an explicit call.** Flagged for user spec-review. If `parent-to-child` or "no pre-selection, force the user to click" is preferred, swap the initial value in the store (or introduce a nullable direction).
- **Partial-failure inconsistency with the rest of the codebase.** Matches existing posture; not a new risk.

---

## Resolved during review

- **Auto-relate originally ran unconditionally after the `contains` try/catch.** Advisor flagged that a `contains`-fail followed by an auto-relate-success would write a non-`contains` relation to a "parent" the child isn't actually under, and a double-fail would overwrite the more severe `contains` error. Now gated on a `containsOk` flag.
- **`description` was originally included in `autoRelatePref` (sticky across form opens).** Advisor flagged that the user's Q2 choice was scoped to "checkbox + type + direction," and descriptions are per-edge content rather than pattern-level. Moved to form-local `$state`, resets on each mount.
- **Silent-skip when checkbox on + type empty was changed to explicit validation.** Advisor flagged inconsistency with the existing `label` / `type` validation posture. Now surfaces "Relationship type is required when auto-relate is on." inline.
- **Partial-failure errors were invisible (flash-and-disappear).** Advisor flagged that `onsave(saved)` unmounts the form unconditionally, destroying the form-local `localError`. Now gated: `if (!localError) onsave(saved)`. Form stays open on partial-failure; user reads the error and closes via Cancel. Also retroactively fixes the pre-existing `contains`-auto flash-and-disappear bug in the same one-line change.

---

## Implementation Order (for writing-plans)

The `superpowers:writing-plans` skill will structure this into concrete tasks. Rough sequence:

1. **Store change** — add `autoRelatePref` export in `src/store/domains.svelte.ts`. Pure plumbing, no UI yet.
2. **NodeForm — template + bindings** — import `autoRelatePref`, add `autoRelateDescription` local state and `knownEdgeTypes` derived, render the auto-relate section. Wire the `bind:` directives.
3. **NodeForm — `save()` extension** — add empty-type validation; add `containsOk` flag to existing `contains` try/catch; add the gated auto-relate block with the two `!== direction` guards.
4. **Scoped styles** — section wrapper, checkbox label, fieldset layout. No hex colors (use `:root` tokens per ARCHITECTURE.md §6).
5. **Manual smoke test** — the 10-step walkthrough above, plus `./scripts/verify.sh` green.
