# GM Screen — Modifier Zones + Drag-and-Drop Primitive

**Date:** 2026-05-14
**Status:** Design — pending user review
**Phase scope:** This spec covers **v1** only. v2 and v3 are designed-for, not built.

## Overview

The GM Screen currently renders all of a character's modifiers (advantage-bound merits/flaws from Foundry, plus free-bound user-authored modifiers and status-template applications) into one mixed overlapping carousel per character row. The GM cannot distinguish "this character has the Beautiful merit" from "this character is currently affected by Slippery Floor" except by reading each card. Free-bound modifiers can only be hidden, never deleted.

This spec introduces:

1. A per-modifier **zone** (`character` | `situational`) that classifies what a modifier semantically is.
2. A **three-box per-character layout** — `[Active effects][Character carousel][Situational carousel]` — driven by zone.
3. **Green visual treatment** for situational cards (border, bg tint, "Situational" pill chip).
4. A **hard-delete** action on free-bound cards alongside the existing hide toggle.
5. A **drag-and-drop primitive** with a pickup-and-place interaction model and a `getActionsFor(source, target)` permission matrix. v1 uses it only for free-bound zone reclassification within a single character row.

The architecture is built so v2 (cross-row drag) and v3 (Status Template palette as drag source) plug in by adding rows to `getActionsFor` and entries to the source/target type unions — no rewrites.

## Decisions made on the agent's recommendation (flag during review if wrong)

The user did not explicitly approve these — the agent picked sensible defaults. Override during spec review if any feels wrong:

- **Free-bound cards keep BOTH the × hide button AND the 🗑 delete button.** Hide is useful when the GM wants to temporarily de-clutter without losing data ("bring it back when the scene changes"); delete is for "this was wrong, gone forever". Advantage-bound cards keep only ×.
- **Hard-delete prompts a `confirm()` dialog**, matching the existing reset-card pattern (`CharacterRow.svelte:300-311`). Yes/cancel only; no soft-trash or undo.
- **Esc key cancels DnD pickup** alongside right-click — cheap to support; matches common UX expectations.
- **Window blur (alt-tab while held) auto-cancels** the pickup.
- **Status Template apply via click (current flow) creates the modifier with `zone='situational'`** by default. Templates are inherently scene-mod-shaped per user intent.
- **The "Situational" pill chip on green cards is derived from `zone`, not added to the `tags` array.** Keeps the tags array user-owned; the chip is system-rendered.

## Domain shape changes

### Rust (`src-tauri/src/shared/modifier.rs`)

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModifierZone {
    #[default]
    Character,
    Situational,
}

// Existing CharacterModifier gains:
pub struct CharacterModifier {
    // …existing fields…
    #[serde(default)]
    pub zone: ModifierZone,
}

// Existing NewCharacterModifier gains:
pub struct NewCharacterModifier {
    // …existing fields…
    #[serde(default)]
    pub zone: ModifierZone,
}
```

The `Default` impl + `#[serde(default)]` matches the project's existing tolerance for legacy/pre-migration payloads (see `foundryCapturedLabels` handling in the same file — missing field deserializes to the default).

### TS mirror (`src/types.ts`)

```ts
export type ModifierZone = 'character' | 'situational';

export interface CharacterModifier {
  // …existing fields…
  zone: ModifierZone;
}
```

### DB migration (`src-tauri/migrations/0007_add_modifier_zone.sql`)

```sql
-- Add zone column; default existing rows to 'character'.
ALTER TABLE character_modifiers
  ADD COLUMN zone TEXT NOT NULL DEFAULT 'character'
  CHECK(zone IN ('character', 'situational'));

-- Backfill: existing template-applied modifiers semantically belong
-- in Situational. The origin_template_id signal is preserved for any
-- future feature; the zone field is the new canonical box-placement
-- source of truth.
UPDATE character_modifiers
   SET zone = 'situational'
 WHERE origin_template_id IS NOT NULL;
```

The backfill is critical — without it, every existing template-applied modifier (slippery, etc.) lands in the Character box after upgrade, requiring the GM to manually drag each one back. The backfill makes the upgrade transparent.

### DB layer (`src-tauri/src/db/modifier.rs`)

- `db_list` SELECT adds `zone` to the column list; row mapper reads it into `ModifierZone`.
- `db_add` INSERT includes the zone value from the input.
- `db_upsert_advantage_binding` (the materialize-advantage path) hard-codes `zone = 'character'` — advantage-bound modifiers are zone-locked.
- New `db_set_zone(pool, id, zone) -> Result<CharacterModifier, String>` for IPC. **Reads the target row's binding first; returns `Err("db/modifier.set_zone: cannot reclassify advantage-bound modifier")` if the binding kind is `Advantage`.** Defensive — the UI matrix already prevents the call, but the backend enforces invariant independently per the project's stable-prefix error idiom (`ARCHITECTURE.md` §7).
- Rust unit test mirroring `character_modifier_captured_labels_round_trip_json`: round-trip a JSON blob with `"zone": "situational"` and assert it deserializes correctly + the missing-field default produces `Character`.

### IPC (`src-tauri/src/db/modifier.rs` → `src-tauri/src/lib.rs`)

```rust
#[tauri::command]
pub async fn set_modifier_zone(
    pool: State<'_, DbState>,
    id: i64,
    zone: ModifierZone,
) -> Result<CharacterModifier, String> {
    db_set_zone(&pool.0, id, zone).await
}
```

Register in `invoke_handler` in `lib.rs`. Adds to the running total in `ARCHITECTURE.md` §4.

### Typed wrapper (`src/lib/modifiers/api.ts`)

```ts
export async function setModifierZone(id: number, zone: ModifierZone): Promise<CharacterModifier>;
```

### Store (`src/store/modifiers.svelte.ts`)

```ts
async setZone(id: number, zone: ModifierZone): Promise<void> {
  const row = await setModifierZone(id, zone);
  mergeRow(row);
}
```

## Layout

`CharacterRow.svelte` body becomes three flex columns instead of two. The current structure:

```
[ActiveEffectsSummary][.modifier-row (one carousel, all cards)]
```

becomes:

```
[ActiveEffectsSummary][.modifier-row.character][.modifier-row.situational]
```

Each `.modifier-row` is the existing absolutely-positioned z-stacked carousel (`CharacterRow.svelte:524-539`), unchanged in mechanics — just two of them, one filtered to `zone === 'character'`, one to `zone === 'situational'`. Each gets its own `--cards: N` inline variable for the z-stack centering math. Each gets its own "+ Add" button defaulting to that zone.

**`sibling-index()` continues to work** because each carousel is a separate DOM parent — sibling-index resets per carousel. The implementer must not introduce shared `--card-i` scopes.

The `visibleCards` derivation in `CharacterRow.svelte:184-193` becomes two derivations, one filtered per zone, applied after the existing tag/hidden filters. The card-entry mapping logic upstream is unchanged.

## Visual treatment

### New tokens (`src/routes/+layout.svelte` `:global(:root)`)

Per `ARCHITECTURE.md` §6 — new accent and surface tokens for situational treatment:

```css
--accent-situational: #4a8a4a;       /* mid green, dark-mode legible */
--accent-situational-bright: #6ab26a;
--bg-situational-card: #182218;       /* greener tint of --bg-card */
--border-situational: #3d6a3d;
```

Hex values are tokens here, not transient inline colors — fine per §6.

### Card rules (`src/lib/components/gm-screen/ModifierCard.svelte`)

The card already exposes `data-active` and `data-hidden` attributes. Add `data-zone`:

```svelte
<div class="modifier-card" data-zone={modifier.zone} …>
```

CSS:

```css
.modifier-card[data-zone="situational"] {
  background: var(--bg-situational-card);
  border-color: var(--border-situational);
}
.modifier-card[data-zone="situational"][data-active="true"] {
  border-color: var(--accent-situational-bright);
}
```

### "Situational" pill chip

Rendered in the card head (`ModifierCard.svelte:100-110`) only when `modifier.zone === 'situational'`:

```svelte
{#if modifier.zone === 'situational'}
  <span class="zone-chip" aria-label="Situational modifier">Situational</span>
{/if}
```

CSS uses `--accent-situational` background, small uppercase, low-vis. Chip sits above the name line.

## Delete UX

- **Trash button** (🗑) rendered on free-bound cards only (`modifier.binding.kind === 'free'`). Triggers a `confirm()` dialog: `"Delete \"{name}\" permanently? This cannot be undone."`. On confirm, calls `modifiers.delete(id)`.
- **× hide button** stays on all cards (current behavior).
- For advantage-bound cards, no trash button — they shouldn't be deletable from the GM screen (a backend `modifiers://rows-reaped` event reaps the row when the underlying Foundry item is deleted; see `bridge/foundry/translate.js` and the existing `db_delete_by_advantage_binding`).

The trash button uses the same opacity-on-hover pattern as the existing push/reset/hide buttons (`ModifierCard.svelte:362-417`).

**Card foot layout.** With trash added, a free-bound advantage-style card's foot can hold up to 6 elements (ON/OFF toggle, save-override, push, reset, hide, trash). In v1 the trash button only renders on free-bound cards — which can never carry the push/reset/save-override set (those are advantage-only). So free-bound foot is at most 3 elements (toggle, hide, trash). Advantage-bound foot is unchanged. The trash sits immediately left of the hide × button in the foot's right-aligned cluster. The smoke checklist includes a step (16) verifying no overflow at the 9rem card width.

## Drag-and-drop primitive

### Interaction model — pickup and place

| State | Trigger | Effect |
|---|---|---|
| `idle` | — | Normal carousel behavior; cards rest. |
| `idle → held` | `pointerdown` (left button) on the `.card-body` element of a free-bound card | Card detaches: position:fixed, follows cursor via `pointermove`. Original slot shows a faded ghost. Valid drop zones get a green dashed outline. Cursor becomes `grabbing`. GM screen root gains `.dnd-active`. A `contextmenu` listener is installed for the duration of the held state — calls `preventDefault()` and `cancel()`. |

The pickup target is **the `.card-body` element only**, not the whole `.modifier-card`. `ModifierCard.svelte` is restructured so the body content (name, bonuses, effects, tags, "Situational" chip) lives inside a single `.card-body` div, and the `.foot` (toggle / push / reset / hide / trash / save-override buttons) lives outside that div as a sibling. `DragSource` wraps `.card-body`. This is enforced structurally, not by a `e.target` predicate — adding new foot buttons in the future will not accidentally break pickup gating.
| `held → dropped` | `pointerdown` (left button) while cursor is over a valid drop zone | Resolve `getActionsFor(source, target)`: if 1 action, execute immediately; if 2+, open DropMenu at cursor with the action list. |
| `held → cancelled` | Right-click anywhere; OR Esc key; OR window blur; OR left-click outside any valid drop zone | Card returns to origin; no IPC call; state machine resets to idle. |

Advantage-bound cards: pointerdown on them is allowed (per advisor item 5), but `getActionsFor` returns `[]` for `kind: 'advantage'`, so every drop is invalid and the card just snaps back. This keeps the matrix as the single source of truth for "what's draggable to where" — no second hard-coded rule on the source side.

### Source / Target contracts

Pinned now so v2/v3 extend without breaking. In `src/lib/dnd/types.ts`:

```ts
export type DragSource =
  | { kind: 'free-mod'; mod: CharacterModifier }
  | { kind: 'advantage'; mod: CharacterModifier }     // v1: always rejected
  | { kind: 'template'; template: StatusTemplate };   // v3: not used in v1, listed for completeness

export type DropTarget =
  | { kind: 'character-zone'; character: BridgeCharacter }
  | { kind: 'situational-zone'; character: BridgeCharacter };

export type Action =
  | { id: 'move-zone';      label: string; newZone: ModifierZone }   // v1
  | { id: 'move-character'; label: string; newSourceId: string }     // v2
  | { id: 'copy-character'; label: string; newSourceId: string }     // v2
  | { id: 'apply-template'; label: string; zone: ModifierZone };     // v3
```

The `Action` union is also pinned now — `DropMenu` will be written generically against this contract so v2/v3 actions render without changes to the menu component.

### Permission matrix (`src/lib/dnd/actions.ts`)

```ts
export function getActionsFor(source: DragSource, target: DropTarget): Action[];
```

**v1 matrix** (everything not listed returns `[]`):

| Source | Target | Returns |
|---|---|---|
| `free-mod` with `mod.zone === 'character'` | `situational-zone` with same `character.source_id` | `[{ id: 'move-zone', label: 'Move to Situational', newZone: 'situational' }]` |
| `free-mod` with `mod.zone === 'situational'` | `character-zone` with same `character.source_id` | `[{ id: 'move-zone', label: 'Move to Character', newZone: 'character' }]` |
| all others | all | `[]` |

The "same `character.source_id`" check enforces v1's same-row-only scope. v2 simply removes that constraint and adds `move-character`/`copy-character` rows. v3 adds rows for `template`-kind sources.

### Drop flow

- `0 actions` → auto-cancel (card snaps back, state → idle).
- `1 action` → execute the action's effect immediately; no menu.
- `≥2 actions` → open `DropMenu` at cursor; user clicks a row to execute, or right-clicks / Escs / clicks-outside to cancel.

For v1, only the 1-action branch fires. The DropMenu component is built generically against the `Action` union so v2/v3 plug in without changes; v1 verifies its rendering via a manual smoke step (see Testing step 15) using a temporary 2-action input — the project has no frontend test framework (`ARCHITECTURE.md` §10).

### Cleanup edges

- **Window blur** (`window.addEventListener('blur', …)`): auto-cancel.
- **Esc** (`keydown` listener while held): cancel.
- **Right-click** (`contextmenu` listener while held): preventDefault + cancel.
- **Left-click outside valid drop zone**: pointerdown's hit-test runs `document.elementFromPoint`; if no DropZone ancestor → cancel.
- **`pointercancel`** event (browser issues this when e.g. a system gesture intercepts): cancel.

### Focus-handler interference

`GmScreen.svelte:115` currently sets `focusedCharacter` on any click within a row's `.row-focus-wrap`. With DnD active, this would double-fire: pickup-click → focus changes; drop-click → focus changes again, potentially to a different row in v2.

Resolution: in `DragSource.svelte`, the pointerdown handler that initiates pickup calls `e.stopPropagation()`. The drop-click in `DropZone.svelte` also stops propagation. Row click handlers run only when no DnD is in progress. **Row focus is unchanged by any DnD interaction in v1.**

### Visual feedback during held state

- `.dnd-active` class on the GM screen root suppresses the per-card hover transform (`ModifierCard.svelte:223-241`) and the neighbor-shift cascade (`:has()` rules at lines 233-241).
- Valid drop zones get `data-drop-valid="true"` set by the held-state effect, styled with a green dashed outline.
- The held card is rendered by a top-level `HeldCardOverlay` component mounted in `GmScreen.svelte`, reading position from the DnD store. Using a top-level component (rather than the carousel's child card) keeps it outside the z-stack and lets it visually overlay anything on the screen.
- Cursor style: `body { cursor: grabbing }` while held.

### Component additions

- `src/lib/dnd/types.ts` — the unions above.
- `src/lib/dnd/actions.ts` — `getActionsFor`.
- `src/lib/dnd/store.svelte.ts` — singleton runes store: held source, current target, drop position, lifecycle methods (`pickup(source, originRect)`, `setTarget(target | null)`, `drop()`, `cancel()`).
- `src/lib/components/dnd/DragSource.svelte` — wraps a draggable element; pointerdown handler.
- `src/lib/components/dnd/DropZone.svelte` — wraps a drop target; reads from the store to highlight; pointerdown handler when held.
- `src/lib/components/dnd/DropMenu.svelte` — rendered at cursor position when an action list of 2+ resolves. Dormant in v1 except for unit coverage.
- `src/lib/components/dnd/HeldCardOverlay.svelte` — the position:fixed clone that follows the cursor.

### Wiring into existing components

- `ModifierCard.svelte` wrapped in `<DragSource>` providing the `DragSource` discriminated union value.
- Each of the two `.modifier-row` carousels in `CharacterRow.svelte` wrapped in `<DropZone>` providing the `DropTarget` value.
- The GM screen root in `GmScreen.svelte` reads `dndStore.held` reactively and toggles the `.dnd-active` class accordingly; renders `<HeldCardOverlay>` and `<DropMenu>` at top level.

## Files to create / modify

**Create:**

- `src-tauri/migrations/NNNN_add_modifier_zone.sql` — migration with backfill.
- `src/lib/dnd/types.ts` — DnD discriminated unions.
- `src/lib/dnd/actions.ts` — `getActionsFor`.
- `src/lib/dnd/store.svelte.ts` — DnD state machine.
- `src/lib/components/dnd/DragSource.svelte`.
- `src/lib/components/dnd/DropZone.svelte`.
- `src/lib/components/dnd/DropMenu.svelte`.
- `src/lib/components/dnd/HeldCardOverlay.svelte`.

**Modify:**

- `src-tauri/src/shared/modifier.rs` — add `ModifierZone` enum, `zone` field on `CharacterModifier` and `NewCharacterModifier`, round-trip test.
- `src-tauri/src/db/modifier.rs` — read/write zone; new `db_set_zone`; advantage upsert hard-codes `zone='character'`.
- `src-tauri/src/lib.rs` — register `set_modifier_zone` command.
- `src/types.ts` — mirror `ModifierZone` and the field.
- `src/lib/modifiers/api.ts` — add `setModifierZone`.
- `src/store/modifiers.svelte.ts` — add `.setZone`.
- `src/routes/+layout.svelte` — new situational color tokens.
- `src/lib/components/gm-screen/ModifierCard.svelte` — `data-zone`, situational styles, "Situational" chip, trash button.
- `src/lib/components/gm-screen/CharacterRow.svelte` — split carousel into character/situational rows; replace single `addFreeModifier()` (line 343-355) with `addFreeModifier(zone: ModifierZone)` taking the zone from each carousel's "+ Add" button; wire DragSource/DropZone. The existing tag-filter and `showHidden` predicates (`passesTagFilter`, `passesHiddenFilter` at lines 165-176) apply identically to both carousels — they filter the per-zone-filtered card list, not the original.
- `src/lib/components/gm-screen/StatusPaletteDock.svelte` — add `zone: 'situational'` to the `modifiers.add({...})` call at lines 38-48 so click-apply produces situational-zone modifiers.
- `src/tools/GmScreen.svelte` — render `HeldCardOverlay` + `DropMenu` at root; toggle `.dnd-active`.
- `ARCHITECTURE.md` — §4 IPC commands inventory: add a `db/modifier.rs` entry (if not already listed) with `set_modifier_zone` and the other modifier commands; update the §4 IPC running total accordingly.

**Delete:** none.

## Anti-scope (v1)

- Cross-row drag (free-bound card from one character's box → another character's box). Designed-for in matrix; not implemented.
- Status Template palette becoming a drag source. Designed-for; not implemented.
- Foundry merit library DnD (Phase 4). Separate spec, gated on Foundry helper library roadmap.
- Undo on hard delete.
- Touch / keyboard a11y for DnD. Pointer events handle stylus, but real mobile/touch UX is a separate design.
- Reclassifying advantage-bound modifiers — zone-locked to `character`.
- Light-mode green variant (ADR 0004 — dark only).
- Animations beyond cursor-follow (no fancy "card flies back to origin on cancel"; just snap).

## Testing & verification

The project has no frontend test framework (`ARCHITECTURE.md` §10) — frontend correctness is verified manually via the dev server. The Rust side does have unit tests.

**Rust tests (`src-tauri/src/shared/modifier.rs`):**

- New test `character_modifier_zone_round_trips_json`: serialize a `CharacterModifier` with `zone: Situational`, deserialize, assert round-trip.
- New test `character_modifier_missing_zone_defaults_to_character`: deserialize a JSON blob without the `zone` field, assert it produces `ModifierZone::Character`.
- New test (in `db/modifier.rs`) `db_set_zone_updates_and_returns_row`: insert a row, call `db_set_zone(id, Situational)`, assert the returned row has the new zone and a fresh `updated_at`.
- New test (in `db/modifier.rs`) `db_upsert_advantage_binding_locks_zone_to_character`: upsert an advantage binding, assert the resulting row has `zone = Character` regardless of any input.

**Migration verification:**

- Manual: run the migration against a test DB containing one modifier with `origin_template_id IS NOT NULL` and one without. Verify the first row's `zone` flips to `situational` and the second stays `character`. Document in the PR description.

**Frontend manual smoke checklist (verify.sh + dev server):**

1. Connect Foundry, see the three-box layout per character row.
2. Click "+ Add" in Character box → new card appears in Character zone, gray theme.
3. Click "+ Add" in Situational box → new card appears in Situational zone, green theme + "Situational" chip.
4. Pickup a free-bound character-zone card; drop on situational zone of same row → card moves, theme switches.
5. Same in reverse (situ → char).
6. Drop on the same zone the card already lives in (no-op): card snaps back.
7. Right-click while held → cancel.
8. Esc while held → cancel.
9. Alt-tab while held → cancel.
10. Try to pickup an advantage-bound card → drop anywhere → invalid, snaps back.
11. Click 🗑 on a free-bound card → confirm dialog → on yes, card disappears.
12. Click × on a free-bound card → hidden (existing behavior preserved).
13. Restart the app → verify all zone changes from steps 4-5 persisted.
14. Apply a status template via the dock click flow → new card lands in Situational zone with the green theme.
15. DropMenu render sanity: temporarily edit `getActionsFor` to return two actions for any v1 same-row drop, perform a drop, verify the menu appears at the cursor with both rows; revert the edit before commit.
16. Apply tag filters with both carousels populated; verify the filter applies identically to both zones. Toggle "Show hidden"; verify hidden cards reappear in whichever zone they belong to.
17. Free-bound card foot overflow check: create a free-bound card; verify the foot row (toggle, hide, trash) fits within the 9rem card width with no overflow at all card-state combinations (on/off, hidden/visible).

**Aggregate gate:** `./scripts/verify.sh` (per `CLAUDE.md`) before any commit.

## Phases — what comes after v1

- **v2 (cross-row drag).** Spec'd separately when prioritized. Adds rows to `getActionsFor` for cross-`character.source_id` targets; `move-character` and `copy-character` actions get IPC support (`set_modifier_source` or a duplicate-row helper). Same DropMenu component, same gesture, same source/target unions.
- **v3 (Status Template palette as drag source).** Spec'd separately. `StatusPaletteDock.svelte` wraps each template chip in `DragSource` with `{ kind: 'template', template }`. New rows in `getActionsFor` for `template`-source × `situational-zone` target → `apply-template` action. The chip remains click-applicable for keyboard / non-mouse users.
- **Phase 4 (Foundry merit library DnD).** Gated on the Foundry helper library roadmap (`docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`). When the bridge has an action for "create item on actor from template payload", the merit library palette becomes a drag source whose drop creates a Foundry item via the bridge. Same primitive.

## Invariants cited

- `ARCHITECTURE.md` §2 (`CharacterModifier` shape — adding a field, not changing existing ones).
- `ARCHITECTURE.md` §4 (IPC commands inventory — `set_modifier_zone` adds to the modifier list).
- `ARCHITECTURE.md` §5 (only `src-tauri/src/db/*` talks to SQLite; modifier IPC stays in `db/modifier.rs`).
- `ARCHITECTURE.md` §6 (CSS tokens from `:root`; new `--accent-situational*` tokens added there, not inlined).
- `ARCHITECTURE.md` §9 (Tauri command extension seam — register in `lib.rs::invoke_handler`; typed wrapper in `api.ts`).
- `ARCHITECTURE.md` §10 (no frontend test framework — manual smoke + Rust units only).
- `CLAUDE.md` workflow override: every plan task ending in a commit lists `./scripts/verify.sh` immediately before the commit.
