# Item Builder Shell — Design Spec
_2026-05-17 · revised 2026-05-19 after visual brainstorm_

## Context

vtmtools has three Svelte editors that all author "name-a-thing-and-give-it-tags-plus-effects" content. They were built at different times for different homes and look unrelated despite serving the same mental model:

- `src/lib/components/AdvantageForm.svelte` (322 lines) — inline grid-expansion editor for `Advantage`. Expanding it shifts the surrounding grid; the form sits awkwardly between cards.
- `src/lib/components/gm-screen/ModifierEffectEditor.svelte` (352 lines) — floating popover inside `CharacterCard` (`CharacterCard.svelte:876`) and `CharacterRow` (`CharacterRow.svelte:527`). 22rem fixed width.
- `src/lib/components/gm-screen/StatusTemplateEditor.svelte` (250 lines) — centered modal overlay launched from `StatusPaletteDock.svelte:109`.

Each editor reimplements its own dialog frame, chip-input, footer, and slot styling. Visual rhythm differs across all three. The current `position: fixed` overlay in `StatusPaletteDock` breaks under `transform` ancestors (per memory `feedback_portal_required_under_transform`).

**Beyond consolidation, the 2026-05-19 brainstorm expanded the design's ambition substantially.** The new shell is no longer a passive frame — it is a **picker-first, card-shape, attention-spotlight composer** built to be fast for keyboard-only and mouse-only users alike, intuitive for inexperienced authors, and structurally extensible to future entity types (Disciplines, Items).

## Goals

1. **One shared dialog shell** (`ItemBuilderShell`) for every "build a library entity" flow. Centered modal over a dimmed backdrop, native `<dialog>` for transform-ancestor safety.
2. **Picker-first wizard** (Step 1) when caller is generic; **skippable** when caller knows the type (`AdvantagesManager` already knows you're making an advantage).
3. **Card-shape composer** (Step 2) — the dialog body's spatial layout mirrors the resulting library card. As fields confirm, they slot into card positions. Two visual modes for the top zone: **building** (colored slot tags) and **preview** (real library card).
4. **Attention-spotlight active editor** — a fixed-location zone hosts the currently-active field's controls. User's eye stays put across stages.
5. **Keyboard / mouse parity** — every forward action has a back action in both modalities. Tab = click; Shift+Tab = right-click; Enter advances within a row; Ctrl+Enter saves; Esc cancels.
6. **WIP save always works** — invalid forms save as drafts and can be reopened to finish. Drafts can still be pushed to Foundry, marked as such.
7. **V5 thematic visual language** — blood-dot gradient for the magnitude selector (stale-blood → glowing-red), serif card-style display states, Latin auto-placeholder for names, "Applies to" semantic chips replacing jargon path-input.
8. **Extracted `TagChipInput` primitive** for the chip-input pattern currently duplicated across all three editors.

## Non-goals (v1)

- **No Discipline body in v1.** Picker shows Discipline with a "preview · needs design" pill; selecting it is disabled. Body lands in a follow-up.
- **No Item body in v1.** Same treatment as Disciplines. Foundry's `CanonicalWorldItem` already exists, but the authoring side needs a dedicated category-aware body (weapon/armor/gear).
- **No data-model unification.** `ModifierEffect` does not merge into `Field`. They answer different questions.
- **No drag-to-reorder, no command-palette quick-add, no keyboard chords** beyond Ctrl+Enter to save. These can ship as follow-ups against the polished shell.
- **No automatic Foundry roller support for `auto_success`** — the kind is added storage-side as forward-compat; the V5 dice helpers gain support later.

## Data-shape additions

The 2026-05-17 spec said "no data-shape changes." The 2026-05-19 brainstorm broke this in two places. Both additions are small and forward-compatible.

### 1. `ModifierKind` gains `'auto_success'` variant

```rust
// src-tauri/src/character/modifier.rs (or wherever ModifierKind lives)
pub enum ModifierKind {
  Pool,
  Difficulty,
  Note,
  Stat,
  AutoSuccess,  // NEW · stored as "auto_success"
}
```

- TypeScript mirror in `src/types.ts` updates to `'pool' | 'difficulty' | 'note' | 'stat' | 'auto_success'`.
- Existing roller code does **not** consume the new variant in v1. It is render-time-only on cards (similar to `Stat`) until the dice helpers gain support.
- DB migration: enum-as-text, no schema change required if the column is already `TEXT`. Verify with current advantage/template tables.

### 2. WIP / draft flag on advantage and status-template

```sql
-- migration: 0050_add_advantage_wip_flag.sql (or sequential)
ALTER TABLE advantage ADD COLUMN wip INTEGER NOT NULL DEFAULT 0;
ALTER TABLE status_template ADD COLUMN wip INTEGER NOT NULL DEFAULT 0;
```

- `wip = 1` means "saved as draft — fields may be invalid or incomplete."
- Library list view shows a small "draft" indicator on WIP rows.
- Foundry push action: drafts are skipped by default. The "Push to world" button gains an optional secondary "Push WIP too" affordance.
- WIP advantages reopen in the builder at the **first unconfirmed field's stage**; user finishes and re-saves.

### 3. Optional `bonus_name` per `ModifierEffect`

Foundry's bonus item shape supports per-bonus names. We persist user-provided names through to push.

```rust
pub struct ModifierEffect {
  pub kind: ModifierKind,
  pub scope: Option<String>,
  pub delta: Option<i32>,
  pub note: Option<String>,
  pub paths: Vec<String>,
  pub bonus_name: Option<String>,  // NEW
}
```

- Default at author time: `"[item name] bonus"` (live-derived until user customizes).
- Sticky once customized — re-naming the parent item does not retro-update.

### 4. "Applies to" semantic chips emit existing canonical paths

No data-shape change here — purely a UI translation. The "Social" chip emits the canonical path list for V5 Social skills (`skills.intimidation`, `skills.persuasion`, `skills.subterfuge`, etc.). "Physical" / "Mental" similarly. "A skill…" opens a dropdown of all V5 skills with type-ahead. The advanced disclosure reveals the existing path-chip system for power users.

---

## Architecture

### Component map

```
src/lib/components/builder/
  ItemBuilderShell.svelte         — dialog chrome (header, close, footer, <dialog> mechanics)
  TypePicker.svelte               — Step 1: type cards (Merit/Flaw/Background/Boon/StatusTemplate/ModifierEffect + future Discipline/Item)
  CardComposer.svelte             — Step 2: card-shape layout + spotlight + WIP footer
  TagChipInput.svelte             — extracted chip-input primitive
  ValueSelector.svelte            — 1-10 blood-dot row + custom numpad popover + scroll-wheel + typing
  AppliesToPicker.svelte          — semantic chips (Social/Physical/Mental/A skill…) + advanced path mode
  bodies/
    AdvantageBody.svelte          — Merit/Flaw/Background/Boon (was AdvantageForm)
    StatusTemplateBody.svelte     — (was StatusTemplateEditor)
    ModifierEffectBody.svelte     — (was ModifierEffectEditor)
    DisciplineBody.svelte         — v1.x stub (returns "needs design" placeholder)
    ItemBody.svelte               — v1.x stub
```

The three legacy editor components are deleted at the end of migration. Their call sites switch to mounting `ItemBuilderShell` with the appropriate body.

### Step 1: TypePicker

Conditional. Shown when caller does not provide `initialType`. Skipped when caller provides one.

**Picker contents:** Advantage kinds (Merit / Flaw / Background / Boon) as enabled cards + Discipline / Item as preview-disabled cards (with a "preview · needs design" pill). **StatusTemplate and ModifierEffect do NOT appear in the picker** — they always have direct entry points where the caller (`StatusPaletteDock`, `CharacterCard`, `CharacterRow`) knows the type, so picker is bypassed.

```svelte
<ItemBuilderShell open={true} title="What are you creating?">
  <TypePicker
    available={['merit', 'flaw', 'background', 'boon']}
    preview={['discipline', 'item']}
    onPick={(type) => parent.handlePick(type)}
  />
</ItemBuilderShell>
```

When user picks an `available` kind, the dialog transitions to Step 2 with `AdvantageBody` mounted and `kind` pre-set. When user clicks a `preview` card, button is disabled — hover shows tooltip "coming in v1.x — body needs design."

Picker uses a 2×3 grid of cards (glyph + title + 1-line description + optional "preview" pill).

**No user-visible entry point in v1** — no "Library home" page exists yet that calls the picker with no `initialType`. The picker ships in v1 anyway, smoke-tested via a temporary `/builder-test` route, so the Library home feature can land later without re-doing the picker.

### Step 2: CardComposer

The composer has four zones, top to bottom:

```
┌────────────────────────────────────────────────┐
│ ① Title bar (step indicator, close button)     │
├────────────────────────────────────────────────┤
│ ② Top zone — building OR preview mode          │
│   building: colored slot tags in card layout   │
│   preview:  real library card (when complete)  │
├────────────────────────────────────────────────┤
│ ③ Spotlight zone — active field's editor       │
│   fixed location regardless of which field     │
├────────────────────────────────────────────────┤
│ ④ Footer — WIP strip, Cancel, Save             │
└────────────────────────────────────────────────┘
```

#### Top zone — building mode

Card-shape layout, five rows:

```
Row 1:  [Name slot]              [Type slot]
Row 2:  [Level slot]             [Category slot]
Row 3:  [Description slot — full width]
Row 4:  [Bonus(es) — one slot per bonus, full width]
Row 5:  [Tags slot — full width]
```

Each slot has three visual states:

| State | Appearance | Trigger |
|-------|------------|---------|
| **Empty** | dim dashed border, italic placeholder text, low opacity | not yet reached in tab order |
| **Active** | pulsing amber border, blinking cursor indicator inside | current stage |
| **Confirmed** | filled colored pill in the field's unique hue, value visible | user pressed Tab/Enter/clicked elsewhere |

Per-field colors (each slot type gets its own to support glance-recognition):

| Field | Hex | Mood |
|-------|-----|------|
| Name | `#d9a560` warm gold | primary identity |
| Type | `#c46050` oxblood | the merit's nature |
| Level | `#c08850` copper | rank / rating |
| Category | `#88aabb` slate | classification |
| Description | `#c4b7a4` parchment | flavor |
| Bonus | `#e0563a` glowing crimson | mechanical power |
| Tags | `#98c89f` moss | taxonomy |

The Level slot shows V5 tier dots inline (1-5 cap). The Bonus slot shows formatted bonus text with the glowing crimson magnitude (`+5 dice`) and azure scope (`Social`). The Tags slot shows mini pill chips.

#### Top zone — preview mode

When `confirmedFields.size === requiredFields.size` (all required fields confirmed), the top zone transitions from building → preview. Building's colored slot scaffolding fades out; the real library card fades in.

The preview is rendered using the **same component the library list uses** to display saved items. This guarantees fidelity: what you see in preview = what appears in the AdvantagesManager grid after Save. Differences:
- Preview gets an amber border and amber glow halo to indicate "this is a render, not yet saved."
- Click anywhere on the preview returns to building mode focused on the clicked field.

Transition: 180ms cross-fade. The dialog height stays constant (both modes have similar footprint when complete).

#### Spotlight zone

Fixed location below the top zone. Hosts the active field's editor. When the stage changes, the spotlight's content swaps via a 140ms cross-fade (the user's eye stays at the same screen coordinate).

Spotlight content varies per stage:

| Stage | Spotlight content |
|-------|-------------------|
| `name` | Large text input, Latin auto-placeholder pre-filled |
| `type` | Skipped (set in Step 1 unless re-edit) |
| `level` | Tier dot-row (1-5), single-click to set |
| `category` | Type-specific picker (n/a for merits — auto-skipped) |
| `bonus` | Sub-composer: type cards (Pool/Difficulty/AutoSuccess) → name → value → applies-to |
| `description` | Multi-line textarea |
| `tags` | TagChipInput |
| `review` | "[Name] is ready" prompt + edit-by-click hint |

#### Footer

```
[WIP strip — when invalid]  [Cancel]  [Save | Save WIP]
```

- The WIP strip is visible only when at least one required field is unconfirmed. Shows count of missing required fields ("2 fields pending — saveable as draft").
- Save button label flips between `Save` (all valid) and `Save WIP` (invalid). Color shifts from accent-red to amber.
- `Save WIP` always succeeds; sets `wip = 1` on the saved row.

### Stage state machine

Lives in the body component. Pure UI state, never persisted.

```typescript
type Stage =
  | 'name' | 'type' | 'level' | 'category'
  | 'bonus' | 'description' | 'tags' | 'review';

interface ComposerState<T> {
  stage: Stage;
  confirmedFields: Set<Stage>;
  // ... body-specific payload fields
  data: T;  // T is body-specific (AdvantageBodyState, StatusTemplateBodyState, etc.)
}
```

Tab/Enter advances `stage` along the body's `tabOrder` array. Shift+Tab / right-click retreats. Each body declares its own `tabOrder` (since field sets vary per type) and `cardLayout` (spatial slot positions).

Click on a confirmed slot in the building top zone sets `stage` to that slot's stage and removes it from `confirmedFields` (so spotlight reloads its editor). Subsequent stages stay confirmed — no cascade.

### Tab order vs spatial order

Decoupled. Each body declares both:

```typescript
interface BodyConfig {
  tabOrder: Stage[];      // user's edit sequence
  cardLayout: CardSlot[]; // spatial positions on the card
}
```

For Advantage (merit/flaw/etc.):

```typescript
{
  tabOrder: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
  cardLayout: [
    { row: 1, cols: [{ stage: 'name' }, { stage: 'type' }] },
    { row: 2, cols: [{ stage: 'level' }, { stage: 'category' /* n/a */ }] },
    { row: 3, cols: [{ stage: 'description', span: 'full' }] },
    { row: 4, cols: [{ stage: 'bonus', span: 'full' }] },
    { row: 5, cols: [{ stage: 'tags', span: 'full' }] },
  ],
}
```

Note: `bonus` is row 4 (visually), but row 4 in tab order (i.e., before `description`). This is intentional — the user fills the mechanical bonus before the flavor description.

---

## Speed patterns (the keyboard ≡ mouse parity bundle)

All eight from the brainstorm, codified into shell behavior:

1. **Auto-focus on Add.** Clicking "+ Add" or pressing Enter on it creates the row AND focuses the row's first editable field.
2. **Enter advances within row.** In a multi-field row (e.g., bonus = kind+name+value+appliesTo), Enter walks to the next field within the row, then advances to the next stage.
3. **Backspace removes trailing chip.** In any chip-input, an empty draft + Backspace removes the previously-committed chip.
4. **Arrow keys in stepper.** `↑` / `↓` increment / decrement the focused value. Native pattern, applied to our custom selectors.
5. **Ctrl+Enter saves.** From anywhere in the dialog. Saves even invalid forms (as WIP).
6. **Smart defaults per kind.** New Dice Pool effect defaults to `+1` (not 0). New advantage defaults to kind `Merit`. New property defaults to type `number`, value `1`.
7. **Click row label = focus first field.** Large mouse target on row labels routes the click to focus the first input.
8. **Backdrop click + Esc = Cancel.** Native `<dialog>` semantics.

Additional patterns confirmed during the brainstorm:

9. **Scroll wheel on numbers.** Hover the value input + scroll wheel = increment/decrement. Range clamped 0-100. Only when the value input is focused (prevents accidental scroll-page modifications).
10. **Right-click = back.** Mirror of Shift+Tab. Dialog suppresses the browser context menu with `oncontextmenu={(e) => { e.preventDefault(); goBack(); }}`. Defined behaviors:
    - Right-click on the spotlight zone → retreat one stage
    - Right-click on a confirmed slot → re-edit that slot (same as left-click)
    - Right-click on a chip → remove that chip (same as click ×)

11. **Tab single-press advances.** No "Tab to confirm then Tab to advance" trap. Tab on a non-empty chip-input draft commits AND advances. Tab on a value input commits AND advances.
12. **Direct typing always works for value input.** The 1-10 blood-dot row is a mouse shortcut; the underlying value field accepts any 0-100 via keyboard.

---

## Visual language

### Dialog chrome

- Width: `min(36rem, calc(100vw - 2rem))`. Wider than the legacy 22/24rem to accommodate the card-shape composer.
- Max height: `calc(100vh - 4rem)` with `overflow-y: auto` on the spotlight zone only — top zone and footer stay pinned.
- Backdrop: `rgba(0, 0, 0, 0.55)`.
- Tokens: `--bg-raised` (panel base), `--bg-input` (controls), `--border-faint` / `--border-surface` (separators), `--text-primary` / `--text-secondary` / `--text-label` / `--text-muted` (hierarchy), per-field accent colors above.
- No new hex outside the per-field color palette declared above. Those land as new CSS variables in `+layout.svelte`:

```css
:root {
  --field-name: #d9a560;
  --field-type: #c46050;
  --field-level: #c08850;
  --field-category: #88aabb;
  --field-desc: #c4b7a4;
  --field-bonus: #e0563a;
  --field-tags: #98c89f;

  --blood-1: #2a1310;
  --blood-2: #3a1814;
  --blood-3: #4a1d18;
  --blood-4: #5a221c;
  --blood-5: #6c2920;
  --blood-6: #843024;
  --blood-7: #9c3828;
  --blood-8: #b04230;
  --blood-9: #c84c34;
  --blood-10: #e0563a;
}
```

### Blood-dot magnitude gradient

The Bonus stage's value selector uses a 10-dot row, position-indexed colors. Filled dot N has `background: var(--blood-N)` and a position-scaled `box-shadow` glow:

| Position | Fill | Glow |
|----------|------|------|
| 1 | `--blood-1` (stale brown) | none |
| 5 | `--blood-5` (mid red) | 3px subtle |
| 8 | `--blood-8` (vivid red) | 8px |
| 10 | `--blood-10` (incandescent) | 12px + 24px outer halo |

Range 1-10 inline. A `…` button at the end of the row opens the **custom numpad popover** for values 11-100 (or below 1, if useful). Direct typing into the value input bypasses both UIs.

### Latin auto-placeholder

Name field defaults to a randomly-selected entry from a curated Latin word pool. Examples: `Vinculum Sanguinis`, `Praesidium Noctis`, `Fons Aeternitatis`, `Vigil Tenebris`. The placeholder is a real default value (not just visible text) — Tab past Name without typing accepts the placeholder. Pool of ~50 entries lives in `src/lib/builder/latin-placeholders.ts`.

### Bonus naming default

Inside the Bonus stage, the bonus name input's default is `"[merit name] bonus"`. Updates live as the merit name changes UNTIL the user types into the bonus name field; then the link breaks and the bonus name is sticky.

---

## Component contracts

### `ItemBuilderShell.svelte`

```svelte
<script lang="ts">
  interface Props {
    open: boolean;
    title: string;
    saveLabel?: string;            // default 'Save', flips to 'Save WIP' when invalid
    valid?: boolean;               // bindable
    busy?: boolean;
    error?: string | null;
    onSave: () => void | Promise<void>;
    onCancel: () => void;
    onDelete?: () => void | Promise<void>;
    children: Snippet;
  }
</script>

<dialog
  oncancel={(e) => { e.preventDefault(); onCancel(); }}
  onclick={onBackdropClick}
  oncontextmenu={(e) => { e.preventDefault(); /* route to body's onBack */ }}
>
  <!-- header / body slot / footer with WIP strip / save with dynamic label -->
</dialog>
```

### Body contract

Each body component declares its config and exposes the standard bindables:

```svelte
<script lang="ts">
  interface Props<T> {
    initial?: T;
    valid?: boolean;       // bindable — derived from confirmedFields and required-field rule
    state?: ComposerState<T>;  // bindable — full state including stage and confirmedFields
  }

  export const config = {
    tabOrder: [...],
    cardLayout: [...],
    requiredFields: [...],   // for WIP / valid derivation
    latinPool: latinPlaceholders,
  };
</script>
```

The shell does NOT know about specific stages or fields. It only knows about the bindables and renders the body via a default snippet.

### `TagChipInput.svelte`, `ValueSelector.svelte`, `AppliesToPicker.svelte`

Pure primitives. Used by bodies. Same API as in the original spec — chip-input takes `tags` + `onChange`; value-selector takes `value` + `onChange` + `min` + `max`; applies-to takes `paths` + `onChange` + an optional `character` for per-actor discipline suggestions.

---

## Call site migration

Five call sites change. The first is new; the rest were in the original spec.

### 0. (NEW) Library home page entry point

A future "Library home" view (out of v1 scope) will have a single "+ New library item" button that opens `ItemBuilderShell` with no `initialType` → picker shown. **Not built in v1**; mention here so the architecture supports it.

### 1. `AdvantagesManager.svelte` — replace inline grid-expansion with modal

The current "+ New advantage" button opens the builder with `initialType: 'advantage'` (picker skipped). Same for the edit affordance.

```svelte
{#if showAddForm || editingId !== null}
  <ItemBuilderShell ...>
    <AdvantageBody initial={editingEntry} bind:valid bind:state />
  </ItemBuilderShell>
{/if}
```

The grid no longer reserves space for the inline form. Eliminates layout shift.

### 2. `StatusPaletteDock.svelte` — replace `.editor-overlay` div with shell

```svelte
<ItemBuilderShell open={editorOpen} ...>
  <StatusTemplateBody initial={editorExisting} bind:valid bind:state />
</ItemBuilderShell>
```

Delete moves from the StatusTemplateEditor's footer into the shell's footer (same position, same behavior).

### 3 & 4. `CharacterCard.svelte:876` and `CharacterRow.svelte:527`

Same shell, `ModifierEffectBody` mounted with `initialEffects`, `initialTags`, `character` (for discipline path autocomplete).

---

## Phasing (suggested implementation order)

Tasks are deliberately small and atomic so each can be a single commit with `./scripts/verify.sh` passing.

1. **Land primitives.** `TagChipInput.svelte`, `ValueSelector.svelte`, `AppliesToPicker.svelte`. Visual no-op for existing editors — refactor `StatusTemplateEditor` to use `TagChipInput` as the smoke test, leave the other two for later steps.
2. **Add Latin placeholder pool.** `src/lib/builder/latin-placeholders.ts` + utility function.
3. **Add `auto_success` to ModifierKind.** Rust enum + TS mirror + DB read/write paths.
4. **Add `wip` column.** Migration + Rust struct + IPC updates + TS types.
5. **Add `bonus_name` to ModifierEffect.** Same pattern as wip.
6. **Land `ItemBuilderShell`.** Bare dialog chrome, header/footer/body slot. Smoke-test with throwaway harness.
7. **Land `CardComposer` skeleton.** Zone layout (top building/preview, spotlight, footer), color tokens, slot states, stage state machine. Empty body bodies for now.
8. **Land `AdvantageBody`.** Full picker-skip path for Merit/Flaw/Background/Boon. Tab/spatial config, required-field rule.
9. **Migrate AdvantagesManager.** Replace inline form with the new shell + AdvantageBody. Visual review: building mode looks like the synthesis mockup; preview mode renders identically to the library card.
10. **Land `StatusTemplateBody`.** Migrate StatusPaletteDock. Delete on shell footer.
11. **Land `ModifierEffectBody`.** Migrate CharacterCard + CharacterRow. Verify transform-ancestor case.
12. **Land `TypePicker`.** Wire it up to the (currently-unused) generic entry point — placeholder route for testing. Discipline/Item show as preview-disabled.
13. **Delete legacy editors.** `AdvantageForm.svelte`, `ModifierEffectEditor.svelte`, `StatusTemplateEditor.svelte`. Remove old `.editor-overlay` CSS in `StatusPaletteDock` and `CharacterCard`.
14. **End-of-branch single code review.** Run `code-review:code-review` against the full branch diff. Address findings.

**Gates:**
- `./scripts/verify.sh` green before every commit.
- Smoke-test in `npm run tauri dev` after each migration step (9, 10, 11).
- No new tests required for wiring tasks. Tests REQUIRED for: Latin placeholder rotation (step 2), stage state machine logic (step 7), bonus_name auto-derivation behavior in AdvantageBody (step 8).

---

## Out of scope (future enhancements against the polished shell)

- **DisciplineBody and ItemBody** (v1.x). Picker stubs are in place; bodies land in follow-up sub-features.
- **Drag-to-reorder** for properties and effects.
- **Kind-aware preset suggestions** in AdvantageBody (Background presets ≠ Merit presets).
- **Command-palette quick-add** for power users.
- **Bonus type follow-on stages** beyond Pool/Difficulty/AutoSuccess (e.g., `note`, `stat` kinds need entry points or remain accessible via "Advanced" disclosure).
- **Mass-edit / bulk tag-add** across the library.
- **Animated slot transitions** beyond cross-fade (e.g., slide-from-right). v1 ships with cross-fade only.
- **Foundry roller support for `auto_success`** — data is stored; consumption lands when the V5 dice helpers add the case.

---

## Open questions

1. **Latin placeholder pool curation.** ~50 entries is the target. Source: real Latin words, lore-aware (e.g., `Vinculum` = bond, on-brand). Worth a separate research pass before locking content. Stub pool of 12 entries in v1 is acceptable.
2. **WIP push-to-Foundry default.** Skipped by default with a "push WIP too" toggle is the spec's current choice. Alternative considered: push with `[WIP]` name prefix. User confirmed "skip by default" is preferred but the toggle should be discoverable.
3. **Preview-mode click-routing precision.** When user clicks the preview's "+5 dice when making Social rolls" text, which slot do they re-edit — bonus magnitude or applies-to? Spec lands on: clicking opens the whole Bonus sub-composer at its first stage; user re-walks. Could be refined later.
4. **Animation tokens.** Cross-fade 140ms (spotlight content swap) and 180ms (building↔preview transition). These are stylistic choices; if vtmtools accumulates a motion system, hoist to CSS variables.
5. **GitHub issue tracking.** Per memory `project_roadmap_github_issues`, only the user creates issues. Spec proposes this as a single feature-level issue ("Unified item-builder shell + UI polish") with the 14 phasing steps as a `- [ ] #N` task list in the body — but the user makes that call.
