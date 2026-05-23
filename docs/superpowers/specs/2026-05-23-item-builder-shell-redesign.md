# Item Builder Shell — Redesign (Two-Form Architecture)
_2026-05-23 · supersedes the design intent of the 2026-05-17 spec; preserves its data-model additions_

## Context

The 2026-05-17 Item Builder Shell spec was executed across 23 plan tasks on the `templates-impl` branch and merged to master. Post-merge testing revealed visual + behavioral drift from the brainstorm mockups (`.superpowers/brainstorm/156015-1779148305/content/`) and from spec-stated interactions. A first remediation attempt produced a unilateral spec + plan; both were reverted at the user's request because they bypassed brainstorming.

This spec is the result of a fresh brainstorm session (`.superpowers/brainstorm/{227497,473959,75998}-*`) that re-grounded the design with the user. Key shifts from the original direction:

1. **Two complementary forms**, not one — inline-edit on the card itself for quick changes, plus the spotlight builder for creation and complex flows.
2. **Form #2 runs as a route, not a dialog modal** — the user explicitly wants it to be its own page during alpha for testing simplicity.
3. **Card visual identity inspired by the Witcher bestiary card** (CodePen `aditigaur/JjJgyrp`) — three-zone vertical layout, outline+offset frame, Spartan-thin uppercase titles, Oswald-light body, three-box footer with letter-spaced labels.
4. **No hero zone** on the card (the user removed it during brainstorm) — title sits at the top of the outlined frame; per-kind theming lives in the Type detail box.
5. **Required-field shape changed**: name is the only hard-required field. `kind` defaults to `'merit'`. `level` defaults to `0`. The bonus (`effects[0]`) is optional.
6. **Best practices applied** from 2026 form-UX research (NN/g, W3C WAI-ARIA APG, IxDF, PatternFly, Eleken, Anve, Venture Harbour). The original spec's behavioral guarantees gain explicit affordances: visible focus rings, inline-on-blur validation, autofocus on stage change, kbd-styled key hints, custom-widget keyboard maps per the W3C APG patterns.

The data-model additions from the original spec (`auto_success` ModifierKind, `wip` + `confirmed_fields` columns, `bonus_name` per `ModifierEffect`, `effects[]` on Advantage) stay as-is. No backend changes in this remediation.

## Goals

1. **Form #1 — Inline-edit card.** Click an existing card → it transforms in place into editable fields. Save / Cancel returns to read mode. No modal, no navigation. For fast edits to one or two fields on an existing item.
2. **Form #2 — Builder page.** "+ Add Custom" or "edit →" handoff from Form #1's bonus row opens a dedicated route. Stage-by-stage spotlight composer with progress signals across three zones (card preview + spotlight + queue). For creation and any deep restructuring.
3. **Shared data layer.** Both forms write through the existing `addAdvantage` / `updateAdvantage` Tauri commands. No new IPC. The backend already accepts every field the forms produce.
4. **Visual identity:** Witcher-inspired three-zone card (title / description + tags / three-box footer), outlined frame with offset, Spartan-thin uppercase titles, Oswald-light descriptions and footer values.
5. **Keyboard / mouse parity** across both forms, per the W3C ARIA APG patterns for custom widgets.
6. **Responsive from 800 px (Tauri default window) upward.** No fixed pixel widths; use `rem`, `min()`, `max()`, `clamp()`, `minmax()`, container queries.
7. **Save WIP always succeeds.** The save button is never disabled by validity state. Reopening a WIP draft seeds the stage cursor on the first unconfirmed stage.

## Non-goals

- **No hero / banner zone** on the card. The Witcher version had a colored monster-image zone at the top; we explicitly removed it during brainstorm.
- **No per-kind background theming.** Kind is communicated by the text in the Type footer box only. No hero gradient, no decorative stripe, no glyph icon.
- **No data-model changes.** The schema landed in migrations 0010 + 0011 from the original execution. No new fields, no schema migration.
- **No DisciplineBody or ItemBody.** TypePicker stubs them as preview-disabled. Bodies remain v1.x scope.
- **No Library home page** in v1. Form #2 entry points are the "+ Add Custom" buttons in `AdvantagesManager` and the "edit →" handoff from Form #1. The TypePicker (Step 1 of the original spec) is not exposed in v1 — Form #2 always opens directly on Step 2 (the body composer) because callers always know the type.
- **No Foundry-roller `auto_success` consumption** — data is stored, push code grows support later. Same as the original spec.
- **No backwards-compatibility shims for the original dialog form factor.** The dialog-based ItemBuilderShell is replaced by the route-based Form #2; the inline-edit Form #1 is new. The old shell is deleted at the end of the plan.

---

## Data model

The backend already accepts every field below. The front-end's job is to collect them. No schema migration in this remediation.

### Advantage

| Field | Type | Status | Source | Builder stage |
|---|---|---|---|---|
| `name` | `string` | **REQUIRED** | User types | `name` stage (Latin auto-placeholder pre-fills) |
| `kind` | `'merit' \| 'flaw' \| 'background' \| 'boon'` | Defaults to `'merit'` | User confirms (or accepts default) | `type` stage |
| `description` | `string` | Optional | User | `description` stage |
| `tags` | `string[]` | Optional | User | `tags` stage |
| `properties[dot_cost_per_level]` | `number 0–5` | Defaults to `0` | User | `level` stage |
| `effects` | `ModifierEffect[]` | Optional (empty array valid) | User | `bonus` stage (single bonus authored in v1; `effects[1..]` preserved on round-trip from FVTT imports) |
| `wip` | `boolean` | Auto | System | (derived from save action) |
| `confirmedFields` | `string[] \| null` | Auto when `wip = true` | System | (derived from stage machine state) |
| `id`, `isCustom`, `sourceAttribution` | — | Auto | DB / import path | n/a |

**Required-set for the stage machine:** `['name']`. Type and level have defaults; effect is optional. Save succeeds with just `name` filled.

### StatusTemplate

| Field | Type | Status |
|---|---|---|
| `name` | `string` | **REQUIRED** |
| `description` | `string` | Optional |
| `tags` | `string[]` | Optional |
| `effects` | `ModifierEffect[]` | **REQUIRED** (template's reason for existing) |
| `wip`, `confirmedFields`, `id`, `createdAt`, `updatedAt` | — | Auto |

Required-set: `['name', 'effects']`.

### ModifierEffect (sub-shape inside Advantage.effects and StatusTemplate.effects)

| Field | Type | Status | Where authored |
|---|---|---|---|
| `kind` | `'pool' \| 'difficulty' \| 'auto_success' \| 'note' \| 'stat'` | **REQUIRED** | Bonus sub-stage 1 (kind cards) |
| `bonusName` | `string?` | Optional (defaults to `"[name] bonus"`, sticky on edit) | Bonus sub-stage 2 |
| `delta` | `number?` | **REQUIRED** for pool/difficulty/auto_success | Bonus sub-stage 3 (blood-dot row + numpad) |
| `paths` | `string[]` (canonical V5) | Optional (empty = all) | Bonus sub-stage 4 (AppliesToPicker) |
| `scope` | `string?` | Optional (legacy free-text, no UI) | — |
| `note` | `string?` | Only for `kind = 'note'` | (storage-only in v1) |

### Per-character Modifier (ModifierEffectBody)

Authoring shape: `effects[]` + `tags`. Identity comes from the bound character. Required-set: `['effects']`.

---

## Two-form structure

### Form #1 — Inline-edit card

**Entry point:** click anywhere on an existing card in the library list. A hover-reveal ✎ icon at the top-right of the card communicates editability without cluttering the read view.

**Read mode → edit mode transition:** outline shifts from solid gray (`--outline-color: #83868a`) to dashed amber (`--accent-amber: #d9a560`). A top-of-card hint strip appears with kbd-styled key references ("click any field · `Tab` walks · `Esc` cancels · `Ctrl+S` saves"). Each display element becomes its corresponding input:

| Display | Edit-mode control |
|---|---|
| Title (h3 in Spartan-thin uppercase) | Text input (same typography) |
| Description (Oswald-light) | Textarea (same typography), min-height 3.5 rem |
| Tags chips | Chips + trailing `+ add tag` input (TagChipInput primitive) |
| Type pill in footer | `<select>` of the four kinds |
| Level dots in footer | Tier pips remain visible; the focused pip has an amber outline ring; `←` / `→` walk, `Space`/`Enter` commits |
| Bonus text in footer | Display value + "edit →" button that opens Form #2 on the bonus stage |

**Save / Cancel footer** appears below the edit-mode card with kbd hints inline:

```
[ Enter saves · Esc cancels ]        [Cancel]  [Save]
```

**Tab order inside edit mode:** name → description → tags chip-input → type select → tier pip group → bonus "edit →" button → Cancel → Save. Visual order matches tab order (top-to-bottom, then left-to-right in the footer).

**Validation:** inline on blur. If the user empties the name and tabs away, an orange warning line appears below the input ("⚠ name is empty — won't save as complete"). Save WIP still works; just informs the user.

**Hand-off to Form #2:** the "edit →" button on the bonus footer box closes Form #1's edit mode and routes to Form #2 with the bonus stage active. After Form #2's Save, the user returns to the library list (not back to Form #1 edit mode — Form #2's save is authoritative).

### Form #2 — Builder page

**Entry points:**
1. `+ Add Custom` button at the top of the Advantages tool — opens at `/builder/advantage/new` (or equivalent route).
2. `+ New template` button in the GM Screen status palette — opens at `/builder/template/new`.
3. "edit →" handoff from Form #1's bonus footer box — opens at `/builder/advantage/{id}/bonus` (or equivalent stage-deep-link).
4. Edit affordance on `ModifierEffectBody`-managed cards (per-character modifier cards) — opens at `/builder/modifier/{id}` with the corresponding body.

**Form factor:** SvelteKit route, NOT a `<dialog>` modal. Renders inside the main app shell with the standard navigation chrome above. The composer occupies the page's main content area, centered and width-capped at ~36 rem on wide screens, full-width-with-margins on narrow.

**Four zones, top to bottom:**

```
┌─────────────────────────────────────────────────┐
│ ① Step indicator · title bar · close (×)        │
├─────────────────────────────────────────────────┤
│ ② Card preview                                  │
│    Slot grid filling spatially as stages confirm│
├─────────────────────────────────────────────────┤
│ ③ Spotlight                                     │
│    Active stage's editor, fixed location        │
│    140ms cross-fade between stages              │
├─────────────────────────────────────────────────┤
│ ④ Queue + footer                                │
│    "up next: X → Y → Save" · WIP strip · btns   │
└─────────────────────────────────────────────────┘
```

**Behavior summary** (full keyboard map in §"Custom widget keyboard support"):

- `Tab` advances within a stage; `Enter` on the final field of a stage advances to the next stage.
- `Shift+Tab` retreats one field; right-click in the spotlight (outside any text input) retreats one stage.
- Click any slot in the card preview = random-access jump to that stage.
- `Esc` closes the builder; if there's unsaved work, prompts for confirmation.
- `Ctrl+Enter` saves from anywhere.
- Save WIP button is always enabled. Drafts always succeed.

---

## Card visual design

Three-zone vertical layout, no hero. Width capped at 22 rem, centered in its grid cell. Outline + offset gives the distinctive double-frame feel adapted from the Witcher card.

### Zone 1 — Title

```
┌─────────────────────────────────────┐
│                                     │
│         SANGUINE TENACITY           │  ← Spartan, weight 200, uppercase
│                                     │     letter-spacing 1.6px, 1.05rem
│                                     │     centered, line-height 1.25
```

`padding: 0.85rem 1rem 0.5rem`. Title is the first thing inside the outlined frame — no banner above.

### Zone 2 — Content (description + tags)

```
│ Reduce Blood Bond effects by 1      │  ← Oswald, weight 200
│ stage per dot, representing iron    │     0.82rem, letter-spacing 0.4px
│ control over imposed loyalty.       │     line-height 1.55, left-aligned
│                                     │     padding 0 1rem 0.55rem
│                                     │
│       loresheet · touchstone        │  ← Spartan lowercase chips
│                                     │     0.6rem, letter-spacing 0.4px
│                                     │     centered row, gap 0.25rem
│   ─────────────────────             │  ← Hairline rule
│                                     │     1px, fades to transparent at edges
│                                     │     margin: 0 1rem 0.45rem
```

### Zone 3 — Footer (three detail boxes)

```
│  ┌──────┬───────────┬──────────┐    │
│  │ TYPE │   LEVEL   │  BONUS   │    │  ← Spartan h6 labels
│  │      │           │          │    │     0.58rem, uppercase, 1.1px spacing
│  │ Merit│ ●●●○○     │ +5 Social│    │  ← Oswald 200 values
│  │      │           │          │    │     0.85rem, 0.7px spacing
│  └──────┴───────────┴──────────┘    │     centered
│                                     │
└─────────────────────────────────────┘
```

Footer container: `margin: 0 0.55rem 0.6rem`, `padding: 0 0.4rem`, `display: flex`, `gap: 0.35rem`, three boxes each `flex: 1`. Each detail box: `background: #302f30`, `border: 1px solid #403d3d`, `padding: 0.45rem 0.4rem 0.5rem`.

**Bonus magnitude treatment:** the numeric value (e.g. `+5`) renders in glowing crimson (`color: var(--blood-10)`, `text-shadow: 0 0 4px rgba(224, 86, 58, 0.4)`, `font-weight: 600`, `font-size: 0.95rem`). The scope (e.g. `Social`) renders in `--accent-blue`. When no bonus: the value cell shows `—` at `opacity: 0.45`.

**Tier dots** in the Level detail box: `0.48rem × 0.48rem` circles, `gap: 0.18rem`, filled dot uses `--accent-amber` with `0 0 3px rgba(217, 165, 96, 0.55)` glow.

### Hover-reveal edit affordance (read mode)

A small `✎` icon at the top-right corner of the card (`position: absolute; top: 0.5rem; right: 0.5rem`). `opacity: 0` by default; transitions to `0.7` on `.vamp-card:hover`. Click target is the entire card, not just the icon — the icon is a discoverability cue only.

### Typography stack (locked)

| Family | Weight | Use |
|---|---|---|
| `Spartan` | 200, 400, 600 | Titles, footer labels (h6), kind text |
| `Oswald` | 200, 300, 400, 600 | Description, footer values, kbd hints |
| `Iowan Old Style`, `Garamond` | inherit | Slot values inside the Form #2 card preview (for the "card-shape" feel) |
| `ui-monospace` | — | `.kbd` key references |

Spartan and Oswald loaded via Google Fonts `@import` in `+layout.svelte`. Iowan Old Style and Garamond are already part of the existing project font stack.

### CSS tokens (added to `+layout.svelte`)

```css
:root {
  /* Pre-existing tokens stay as-is — these are additions or confirmations */

  /* Card outline */
  --outline-color: #83868a;
  --shadow-color: 41deg 3% 15%;
  --shadow-medium:
    0.3px 0.5px 0.5px hsl(var(--shadow-color) / 0.68),
    0.6px 1.2px 1.2px -1.3px hsl(var(--shadow-color) / 0.55),
    1.9px 3.9px 3.9px -2.7px hsl(var(--shadow-color) / 0.42),
    5.4px 10.8px 10.8px -4.1px hsl(var(--shadow-color) / 0.29);

  /* Detail box */
  --bg-detail: #302f30;
  --border-detail: #403d3d;
  --hairline: rgba(255, 255, 255, 0.06);

  /* WIP strip (already implicit in mockup 07; now an explicit token) */
  --wip: #c9a064;

  /* Validation (orange warning, not error red — for inline blur hints) */
  --validation-warn: #d9a560;
  --validation-error: #c46050;
}
```

---

## Form #2 zone specifications

### Title bar (zone 1)

Single-row strip at the top of the page's composer column:

```
┌─────────────────────────────────────────────────────┐
│ STEP 5 OF 6 · new merit                       [×]   │
└─────────────────────────────────────────────────────┘
```

`padding: 0.45rem 0.75rem`, `background: rgba(0, 0, 0, 0.3)`, `border-bottom: 1px solid var(--hairline)`. Step indicator in Spartan 200 uppercase, 0.56rem, 0.1em letter-spacing. The `STEP 5 OF 6` portion uses `--accent-amber`; the `· new merit` portion uses `--text-muted`. The `×` close button on the right routes back to the entry point (Advantages list, GM Screen, etc.) with a confirmation prompt if there's unsaved work.

### Card preview (zone 2)

Same row layout as the Form #1 card body, but each value is a **slot** rather than a finished display element. A slot has three visual states:

| State | Treatment |
|---|---|
| **Empty** | `background: rgba(0, 0, 0, 0.18)`, `border: 1px dashed var(--border-surface)`, italic placeholder text, `opacity: 0.55` |
| **Active** | `border: 1px solid var(--accent-amber)`, `background: rgba(217, 165, 96, 0.06)`, `animation: pulse-border 1.4s infinite`. Cursor indicator `▾` after the value, blink animation. |
| **Confirmed** | Filled in the field's unique hue per the per-field color tokens. `cursor: pointer` to enable re-edit. |

**Per-field colors** (each slot type gets its own when confirmed):

| Field | Token | Hex |
|---|---|---|
| Name | `--field-name` | `#d9a560` |
| Type | `--field-type` | `#c46050` |
| Level | `--field-level` | `#c08850` |
| Category | `--field-category` | `#88aabb` |
| Description | `--field-desc` | `#c4b7a4` |
| Bonus | `--field-bonus` | `#e0563a` |
| Tags | `--field-tags` | `#98c89f` |

These tokens already exist in `+layout.svelte` from the original execution — they don't need to be redeclared.

**Spatial layout** (for AdvantageBody):

```
Row 1 (col-2):  [Name]      [Type]
Row 2 (col-2):  [Level]     [Category]
Row 3 (col-1):  [Description]
Row 4 (col-1):  [Bonus]
Row 5 (col-1):  [Tags]
```

**This is the spec-stated layout** from the original 2026-05-17 spec at lines 285-290. The implementation's swap of Bonus and Description rows was the original D1 deviation; this spec re-asserts: Description = row 3, Bonus = row 4.

**Slot click behavior:** every slot (empty, active, or confirmed) is clickable via `onClick`. The CardSlot component wires `onclick` for all three states (not just confirmed). This satisfies the "click row label = focus first field" speed pattern.

**For StatusTemplateBody** (4 rows): name / effects / description / tags, all `col-1` full-width.

**For ModifierEffectBody** (2 rows): effects / tags, all `col-1` full-width.

### Spotlight (zone 3)

Fixed-location editor zone for the active stage. The spotlight's *position* never changes; only its *content* swaps via a 140 ms cross-fade keyed on stage.

```
┌─────────────────────────────────────────────────────┐
│  ┌─ inner amber-tinted border, inset 0.35rem ─┐    │
│  │                                              │    │
│  │            — SPOTLIGHT —                    │    │
│  │                                              │    │
│  │       Describe Sanguine Tenacity            │    │
│  │                                              │    │
│  │  ┌──────────────────────────────────────┐  │    │
│  │  │  Reduce Blood Bond effects by 1...   │  │    │
│  │  │  ▏                                   │  │    │  ← textarea with amber border
│  │  │                                      │  │    │     focus glow 3px amber tint
│  │  └──────────────────────────────────────┘  │    │
│  │                                              │    │
│  │   [Tab] confirms · [Shift+Tab] back · empty │    │
│  │                                              │    │
│  └──────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

`padding: clamp(0.6rem, 2vw, 1.2rem)`, `background: radial-gradient(ellipse at center, rgba(217, 165, 96, 0.1) 0%, transparent 65%)`, decorative inner border at `inset: 0.35rem`.

**Spotlight prompt** acts as the field label. Examples per stage:
- `name`: "Name this merit"
- `type`: "What kind?"
- `level`: "Tier level"
- `bonus` (sub-stage type): "What kind of bonus?"
- `description`: "Describe [name]"
- `tags`: "Tag this merit"
- `review`: "[Name] is ready."

**Hint line** below the editor uses `.kbd`-styled key references. Examples:
- name stage: "`Tab` or `Enter` accepts placeholder · `Shift+Tab` goes back"
- description stage: "`Tab` confirms · `Shift+Tab` or right-click goes back · empty is fine"
- review stage: "Click any slot above to re-edit · `Ctrl+Enter` to save"

**Inline validation** on blur appears between the editor and the hint line, in `--validation-warn` (gold), with a `⚠` glyph prefix. Example: "⚠ name is empty — won't save as complete." Save WIP still works.

**Autofocus on stage change:** when a stage becomes active, the primary control inside the spotlight receives focus immediately. The 140 ms cross-fade does NOT delay focus — focus is set on the new content's render, the visual fade is a transition. This satisfies the W3C APG "focus management when content changes" rule.

### Queue (zone 4 top half)

Single-row strip showing what's next:

```
┌─────────────────────────────────────────────────────┐
│ UP NEXT     Tags → Save                              │
└─────────────────────────────────────────────────────┘
```

`padding: 0.4rem 0.75rem`, `font-size: 0.58rem`. Label "up next" in Spartan 200 uppercase 0.08em letter-spacing, `--text-muted`. List of pending stages in `--text-faint`. Final "Save" terminator in `--accent-bright` weight 600.

The queue is always present when the form is not yet complete. When all required fields are confirmed, the queue collapses to just "Save" (still highlighted).

### Footer (zone 4 bottom half)

```
┌─────────────────────────────────────────────────────┐
│ [● 1 stage remaining]   [Cancel]   [Save WIP]       │
└─────────────────────────────────────────────────────┘
```

`padding: 0.5rem 0.75rem`, `background: rgba(0, 0, 0, 0.25)`, `display: flex`, `gap: 0.4rem`.

**WIP strip** (left, when invalid): amber pill with a small dot prefix and a concise missing-fields message. Examples: `"name still empty · saveable as draft"`, `"1 stage remaining · saveable as draft"`. The body computes the missing-fields message via `$effect` driven by `machine.confirmed` and exposes it via `$bindable wipMessage`.

**Buttons** (right):
- `Cancel` — closes the builder with confirmation prompt if there's unsaved work.
- `Save WIP` (when invalid) or `Save` (when all required confirmed). Label flips automatically. Color: muted button when WIP, accent-red when Save.

---

## Stage state machine

Lives in the body component. Reactivity via Svelte 5 `$state` + `SvelteSet` (the file extension is `.svelte.ts`).

```typescript
// src/lib/builder/stage-machine.svelte.ts
import { SvelteSet } from 'svelte/reactivity';

export interface StageMachine {
  stage: Stage;                    // reactive ($state)
  confirmed: Set<Stage>;           // reactive (SvelteSet)
  advance(): void;                 // confirms current, moves to next in tabOrder
  retreat(): void;                 // moves to prev in tabOrder (no un-confirm)
  setStage(s: Stage): void;        // jump (un-confirms target)
  isComplete(): boolean;           // required ⊆ confirmed
}

export function createStageMachine(cfg: {
  tabOrder: Stage[];
  required: Stage[];
  initialConfirmed?: Stage[];
}): StageMachine;
```

**Per body:**

| Body | `tabOrder` | `required` |
|---|---|---|
| AdvantageBody | `['name', 'type', 'level', 'bonus', 'description', 'tags']` | `['name']` |
| StatusTemplateBody | `['name', 'effects', 'description', 'tags']` | `['name', 'effects']` |
| ModifierEffectBody | `['effects', 'tags']` | `['effects']` |

Note the AdvantageBody `required` change vs the original spec: previously `['name', 'bonus']`, now `['name']` only. Per the data-model decisions.

**Tab order ≠ spatial order.** AdvantageBody tabs Bonus before Description, but Description sits visually above Bonus on the card. This decoupling is the spec's architectural invariant.

**Random-access via slot clicks:** clicking any slot calls `machine.setStage(field)`. `setStage` removes the target from `confirmed` so its editor reloads — subsequent stages stay confirmed (no cascade).

**Right-click on the CardComposer root (outside any text input):** calls `machine.retreat()`. Carve-out preserves native Cut/Copy/Paste menu inside `<input>` / `<textarea>` / `[contenteditable]` elements.

---

## Best-practice rules (research-applied)

These are derived from the 2026 form-UX research synthesis in the brainstorm (`.superpowers/brainstorm/75998-1779519814/content/09-best-practices.html`). Each rule is required.

1. **Visual order = tab order.** No positive `tabindex` values. `tabindex="0"` only for custom widgets that need to enter the natural flow. `tabindex="-1"` for programmatically-focusable-but-skipped elements.
2. **Native HTML first.** Use `<input>`, `<button>`, `<select>`, `<textarea>` wherever possible. Custom widgets only when no native equivalent fits.
3. **Visible focus rings on every interactive element.** Use the `--accent-amber` accent. Never `outline: none` without replacing.
4. **Single-column inside the spotlight.** The card preview's 2-column rows are OK for compact info display; they collapse to 1-column via container query when the composer is narrower than 28 rem.
5. **Inline validation on blur**, not on save submit. Field-level orange warning messages.
6. **Save & resume always works.** Save WIP button is never disabled. Reopening a WIP draft lands on the first unconfirmed stage.
7. **Autofocus on stage change.** When the spotlight's content swaps, the primary control inside receives focus immediately. Visual fade does not delay focus.
8. **Step count under 7.** AdvantageBody at 5-6 is well within the research sweet spot.
9. **Three layers of progress signaling:** step indicator (title bar) + filling slots (card preview) + queue line ("up next: X → Save").
10. **kbd-styled key references** in spotlight hints — every hint that mentions a key wraps it in `<span class="kbd">Tab</span>` styling. Keys are visible affordances, not hidden in a help modal.
11. **Specific WIP feedback.** The WIP strip shows what's missing concretely ("name still empty"), not vague ("complete the form").
12. **Custom widget keyboard handlers per W3C APG patterns** — see next section.

---

## Custom widget keyboard support (W3C ARIA APG patterns)

Browsers do NOT provide keyboard support for custom widgets. Each widget below explicitly implements its pattern's keyboard map.

| Widget | APG pattern | Tab behavior | Keys inside |
|---|---|---|---|
| **ValueSelector** (blood-dot 1-10 + numpad popover) | Slider | 1 tab stop on the widget | `←` / `→` change · `↑` / `↓` ±1 · `PgUp` / `PgDn` ±10 · `Home` min · `End` max · numeric direct typing OK · scroll wheel when focused |
| **TagChipInput** | Combobox + token list | 1 tab stop on the input; chips are NOT in tab order by default | `Enter` or `,` commits chip · `Backspace` removes last chip when input empty · `Esc` clears draft · chip × button focusable via Shift+Tab from input |
| **AppliesToPicker** (semantic chips) | Toolbar + Listbox | 1 tab stop on the chip group; arrow keys navigate within | `←` / `→` move between chips · `Space` toggle selection · `Enter` confirm and advance · "A skill…" disclosure opens a Listbox dropdown |
| **TypePicker** (kind cards) | Radio Group | 1 tab stop on the card group | `↑` / `↓` / `←` / `→` move between cards · `Space` or `Enter` select |
| **Tier dot pips** (Form #1 + level stage spotlight) | Slider (discrete) | 1 tab stop on the pip group | `←` / `→` walk · `Space` / `Enter` commit · `0` – `5` direct typing |
| **CardSlot** (Form #2 card preview) | Button | 1 tab stop per slot (all three states clickable) | `Space` / `Enter` activate (jump to stage) · `Shift+Tab` back to previous slot |

All widgets must implement `role`, `aria-label` / `aria-labelledby`, and the relevant `aria-*` state attributes from their APG entry. Focus management on group entry uses the roving-tabindex technique where appropriate.

---

## Responsive design

vtmtools is a Tauri desktop window. Target range: 800 × 600 (Tauri default) up to 4K monitors.

| Component | Width strategy |
|---|---|
| Inline-edit card (Form #1) | `width: 100%; max-width: 22rem; min-width: 14rem` |
| Library grid | `grid-template-columns: repeat(auto-fill, minmax(14rem, 22rem))` |
| Builder page (Form #2) | `width: min(36rem, 100% - 2rem); margin: 0 auto` |
| Card-preview 2-col rows | Collapse to 1-col via container query when container `< 28 rem` |
| Spotlight padding | `clamp(0.6rem, 2vw, 1.2rem)` |
| Footer row (level + category in card body) | `flex-wrap` instead of rigid grid |

**Rules:**
- No fixed pixel widths anywhere. Use `rem`, `%`, `min()`, `max()`, `clamp()`, `minmax()`, container queries.
- Min usable width = 800 px window. Below that, layout may look cramped but must still function (no horizontal scroll, no broken overflow).
- Container queries preferred over media queries for component-local responsiveness.
- Touch targets ≥ 32 × 32 px equivalent on tier dots and chip × buttons.
- Test matrix: 800 × 600 → 1280 × 800 → 1920 × 1080 → 2560 × 1440. Verify each.

---

## Save pipeline (no changes needed)

Already wired end-to-end. Both forms write through the same chain:

```
Form (Svelte component)
    │
    ▼ build AdvantageInput / StatusTemplatePatch / etc.
api.ts wrapper          ←  addAdvantage(input), updateAdvantage(id, input),
    │                       addStatusTemplate(...), addModifier(...), etc.
    ▼ invoke()
Tauri IPC handler       ←  src-tauri/src/db/advantage.rs:594 (add_advantage)
    │
    ▼ db_insert / db_update
SQLite                  ←  INSERT INTO / UPDATE advantages SET ...
```

Backend accepts every field both forms produce. No new IPC commands. No schema migration. The implementation work in this remediation is purely front-end refactoring.

---

## Component map

```
src/lib/components/builder/
  ItemBuilderShell.svelte           — DELETED (was the dialog; replaced by route + InlineEditCard)
  CardComposer.svelte               — kept; renders zones 2-4 of Form #2; takes `back` + `stage` props
  CardSlot.svelte                   — kept; all states clickable; cursor indicator on active
  TagChipInput.svelte               — kept; APG combobox + token list keyboard map applied
  ValueSelector.svelte              — kept; APG slider keyboard map applied
  AppliesToPicker.svelte            — kept; APG toolbar + listbox keyboard map applied
  TypePicker.svelte                 — kept; APG radio group keyboard map applied (smoke-only in v1)
  bodies/
    AdvantageBody.svelte            — refactored; required=['name'], level default 0, row order fixed
    StatusTemplateBody.svelte       — refactored; new buttons, kbd hints
    ModifierEffectBody.svelte       — refactored; new buttons, kbd hints

src/lib/components/
  InlineEditCard.svelte             — NEW; wraps AdvantageCard with edit-mode transformation
  AdvantageCard.svelte              — refactored; no hero, three-zone, hover-reveal ✎ icon
  StatusTemplateCard.svelte         — refactored; same three-zone treatment

src/lib/builder/
  stage-machine.svelte.ts           — kept (reactive); required-set changes per body
  latin-placeholders.ts             — kept
  bonus-name.ts                     — kept

src/routes/
  builder/
    advantage/
      new/+page.svelte              — NEW; Form #2 entry for new advantage
      [id]/+page.svelte             — NEW; Form #2 entry for edit (optional stage param)
    template/
      new/+page.svelte              — NEW
      [id]/+page.svelte             — NEW
    modifier/
      [id]/+page.svelte             — NEW (modifier needs character context — see implementation plan for details)
```

---

## Workflow safeguards for the implementation plan (one-off, this branch only)

Per the user's explicit choice in the brainstorm, these safeguards apply ONLY to this remediation. After merge, CLAUDE.md's standing lean-execution workflow override remains the default.

1. **Every plan task lists the mockup file + line numbers it must match.** Format: `.superpowers/brainstorm/<session>/content/<file>.html:<start>-<end>` or this spec's relevant section. The implementer must open the cited range and align their work against it.
2. **Visual smoke-test in `npm run tauri dev` is a gate before commit.** Each task includes an explicit step: open the affected route in a running dev session, perform the cited interaction, visually confirm the result matches the mockup. Before `./scripts/verify.sh` and before commit.
3. **Per-task spec-compliance reviewer subagent re-enabled for this remediation.** The CLAUDE.md lean-execution override drops both per-task reviewers; for this branch, re-enable the spec-compliance reviewer (the code-quality reviewer can stay dropped). The reviewer's job: open the cited mockup line range + this spec's relevant section, compare against the implementer's diff, confirm the change matches the design intent.
4. **End-of-branch holistic visual audit.** Before merge, walk all three flows (Advantage / StatusTemplate / ModifierEffect) in `npm run tauri dev` against this spec's mockups. Document any remaining drift as v1.x backlog rather than punting silently.

---

## Open questions (calibration, not blocking)

1. **Stage-machine `required` set for AdvantageBody — `['name']` vs `['name', 'type']`.** The user's exact wording during brainstorm was: _"the inputs that would need to be filled would be name, type and level, but level can be 0 so defaulting it to zero making it always have data unless otherwise changed. effect is not required."_ Level is explicitly defaulted-to-have-data. Effect is explicitly not required. Name is required. Type has a default of `'merit'` but the user listed it alongside name and level as an "input that needs to be filled." Two reasonable readings:
   - **(a)** `required: ['name']` — name is the only field the user must actively touch. Type and level pass through with defaults. Most permissive, fastest path to a saveable item.
   - **(b)** `required: ['name', 'type']` — the user must walk past the type stage even if they accept the default 'merit'. More deliberate.

   Spec currently defaults to **(a)** because it best matches the brainstorm's permissiveness. If you want (b), flip the constant — single-line change.
2. **Hover-reveal ✎ icon position.** Top-right of the card is the standard pattern. If a future asset library brings actual SVG edit icons, swap the Unicode glyph for the asset; behavior stays the same.
3. **Form #2 route shape.** `/builder/advantage/new` vs `/advantages/new` vs `/library/advantage/new`. Plan picks the routing convention based on existing project routing patterns (it's a minor naming detail).
4. **Latin placeholder pool expansion** from 12 → ~50 entries. Stated as a follow-up in the original spec; still a follow-up here.
5. **Cancel confirmation threshold.** Cancel always asks for confirmation if there's unsaved work? Or only if a non-default value was entered? Default: always ask if `confirmed` set is non-empty or any input has user-entered text different from initial.
6. **`scope` legacy field** on ModifierEffect (free-text predating `paths`). No UI in this redesign. Keep for round-tripping or drop entirely? Default: keep for round-tripping.

---

## What this spec does NOT change

- **Backend / IPC / SQL** — no changes. Migrations 0010 and 0011 from the original execution remain authoritative.
- **`auto_success` ModifierKind variant** — already added; not consumed by Foundry roller in v1; same as original spec.
- **`bonus_name` per ModifierEffect** — already added; default `"[name] bonus"`; sticky on user edit.
- **`wip` + `confirmed_fields` columns** — already added; reseed-on-reopen behavior unchanged.
- **Latin placeholder pool** — already added; 12 entries stay; expansion deferred.
- **Foundry push code** — does NOT consume `effects[]` in v1; out of scope per original spec.

This spec is a **front-end remediation** built on top of the original spec's data work. The implementation plan derived from this spec touches Svelte components and route files only — no Rust, no SQL.
