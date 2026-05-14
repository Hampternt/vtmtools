# Character Card Redesign — Camarilla Dossier with Active-Modifier Projection

> **Status:** designed; ready for plan-writing.
> **Roadmap fit:** Phase 2 enhancement (during-play tooling polish). Sibling of Phase 3A (GM Screen modifier dashboard) — this spec extends the modifier dashboard's reach to the character card itself.
> **Audience:** anyone implementing the new card, refactoring `Campaign.svelte`, or extending modifier visualisation later.
> **Source roadmap:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 2 (character editing) and §3 Phase 1 polish layer.

---

## §1 What this is

A redesign of the character card currently inlined as ~400 lines of markup in `src/tools/Campaign.svelte` (lines 625–1021). The new card is a **fixed 2:3 aspect, four-view, flip-paged dossier** with a **Camarilla-Dossier** aesthetic — slate-blue institutional surface, blood-red accent, dashed inner rule, persistent file-label header. The card **scales proportionally** at S/M/L sizes (no internal reflow) and **subscribes to the GM-Screen modifier store**, so toggling a merit on the card flips its `is_active` and the modifier's stat deltas project visibly onto the Stats view.

Two structural extractions sit alongside the visual rework:

- **Card body** (`CharacterCard.svelte`) — the dossier frame, the four views, the flipper, and the modifier subscription. Reusable.
- **Card shell** (`CharacterCardShell.svelte`) — the wrapper that holds drag handle, source attribution chip, and save/compare/update buttons. Sits *outside* the dossier frame. The shell is Campaign-flavoured today; a future GM-screen-flavoured shell can wrap the same card body without forking it.

This split makes the card import-ready for a future modular GM screen *without that future tool being part of this spec*.

The spec also adds one **render-time** ModifierKind variant (`Stat { path, delta }`) to the existing `Pool / Difficulty / Note` set. The variant feeds visual deltas onto the card, never into roll math — see §6.2.

## §2 Composition — existing pieces this builds on

| Piece | What it provides | How this spec uses it |
|---|---|---|
| `2026-04-16-campaign-card-redesign.md` density toggle | S/M/L/Auto density via CSS custom properties on `.char-grid`; `ResizeObserver` for Auto mode | Card frame queries the same density custom properties; no new toggle UI; existing toolbar control unchanged. |
| `2026-05-03-gm-screen-design.md` modifier system | `character_modifiers` table; `is_active`; `materialize_advantage_modifier`; `set_modifier_active`; `modifiers.svelte.ts` Svelte store; `ModifierEffectEditor.svelte` cog popover | Card subscribes to `modifiers.list`; chip click materializes & toggles via existing commands; right-click chip opens existing `ModifierEffectEditor`. |
| `2026-04-30-character-tooling-roadmap.md` Phase 1 + `src/lib/saved-characters/diff.ts` | `BridgeCharacter` / `SavedCharacter` shapes; canonical path projection vocabulary (`DIFFABLE_PATHS`) | New `computeActiveDeltas()` mirrors `diff.ts`'s projection style and lives next to it (sibling, not subordinate). |
| existing `Campaign.svelte` inline markup | Steppers, save row, source attribution chip, drift badge, raw-JSON panel, expand/collapse state | Behavior preserved; visual treatment ported into the new component split. Steppers re-styled (slate-blue ± circles) but retain their `character::set_field`-router contract. |
| `src/components/SourceAttributionChip.svelte` | Source chip rendering | Lifted into the shell, unchanged. |
| existing canonical paths from `docs/reference/foundry-vtm5e-paths.md` | Attribute / skill / power locations on `raw.system` | Used by both `computeActiveDeltas` and View-2 / View-3 panels. |

## §3 Visual identity & shell

### §3.1 Camarilla-Dossier tokens

Added to `:root` in `src/routes/+layout.svelte` (per ARCH §6 — no hex literals in components):

- `--bg-card-dossier: #14181d`
- `--text-card-dossier: #c5cdd6`
- `--accent-card-dossier: #5c8aa8` (slate blue — institutional)
- `--alert-card-dossier: #d24545` (blood red — active state, deltas, warnings)
- `--label-card-dossier: rgba(92, 138, 168, 0.9)` (slate blue at 90% — for SUBJECT / file labels)
- `--rule-card-dossier: rgba(197, 205, 214, 0.08)` (subtle hairline)
- `--rule-card-dossier-dashed: rgba(197, 205, 214, 0.12)` (decorative inner dashed border)
- `--shadow-card-dossier: 0 8px 32px rgba(0, 0, 0, 0.45)`

### §3.2 Persistent shell within the card frame

Across all four views the dossier frame renders:

1. **File label** (top-left, small caps): `Camarilla file 0xC1A2`. Stable per character — derived from `source_id` (e.g., last 4 hex chars).
2. **Header line**: `SUBJECT  <name>` (left), PC/NPC badge (right). The `SUBJECT  ` prefix is rendered via CSS `::before` on the name span — keeps the name string clean for accessibility / copy.
3. **Clan line**: clan name uppercase letterspaced + `· <generation>th generation`.
4. **Body panel** (the swappable content) — see §5.
5. **Flipper footer**: `‹ <view name> · N/4 ›`. View name in red caps; pager in mono small.

A 1px dashed inner rule inset 8px from each side runs full-frame for the dossier evidence-board feel (`::before` on the frame, `pointer-events: none`).

### §3.3 Shell-rail (outside the card frame)

The wrapper component (`CharacterCardShell.svelte`) renders a thin row **above** the card containing:

- **Drag handle** (⋮⋮, `cursor: grab`) — currently inert; reserved for the future modular GM-screen drag-and-drop. v1 the handle visually exists but no drag listener is attached.
- **Source attribution chip** (existing `SourceAttributionChip.svelte`).
- **Action cluster** (right-aligned): Save / Update saved / Compare buttons with current Campaign.svelte semantics. Drift badge appears here when applicable.

Rationale: the shell-rail keeps GM/system actions visually outside the in-fiction dossier surface. When a future GM-screen widget shell replaces `CharacterCardShell`, it provides its own rail (e.g., pin / lock-view / remove-from-screen) without forking the dossier body.

## §4 Layout, sizing, density

### §4.1 Fixed 2:3 aspect ratio

The dossier frame is locked to a 2:3 portrait at every size. Internal layout proportions never reflow with size change — only scale. This is the load-bearing simplification: every layout decision in §5 can ignore overflow at base size, knowing other sizes are mathematically the same shape.

### §4.2 Density levels

Reuses the existing `--density: s | m | l | auto` mechanism on `.char-grid` from `2026-04-16-campaign-card-redesign.md`.

| Density | Card width × height |
|---|---|
| S (compact) | 196 × 294 |
| M (comfortable, default) | 280 × 420 |
| L (spacious) | 392 × 588 |

A new CSS custom property `--card-scale` is set on `.char-grid` per density (`0.7`, `1.0`, `1.4`). The card uses `--card-scale` as a multiplier on every internal dimension (font-size, padding, gap, track-box, drop-size). The card itself never queries `vw`/`vh` directly.

Auto mode (default): the existing `ResizeObserver` on `.char-grid` picks S/M/L by container width breakpoints (existing thresholds inherited, unchanged).

### §4.3 Skill-list overflow protection

View 2 (Stats) is the only panel likely to overflow at small densities (a maxed-out PC could have ~12+ non-zero skills). Two compaction strategies:

- **Skill filter**: only skills with `value > 0` OR a non-empty specialty list are rendered. A small `Hidden: N skills at zero` footnote summarizes the rest. The footnote is informational, not interactive.
- **Two-column flow at L density**: `column-count: 2` when card width ≥ 380px. Vertical envelope identical to single-column at S.

If after both compactions the panel would still overflow at S density (rare — a heavily-built PC), `overflow-y: auto` on the panel area lets it scroll within the panel. This is the **only** place internal scrolling is allowed in the card. Other views are designed to fit.

## §5 Four views

All views share the §3.2 shell. Body content:

### §5.1 View 1 — Basics

- **Vital row**: hunger drops (left, 5-cell) + BP pill (right, slate-blue outlined).
- **Conscience block**: track-label `Conscience` + 10-letter `CONSCIENCE` row, full-width-justified. Filled letters in `--text-card-dossier`, stained letters in `--alert-card-dossier` with strikethrough, empty letters in muted.
- **Health block**: track-label `Health` + box row. Empty box (1px slate-blue border), `.superficial` (red-fill), `.aggravated` (red border + inset shadow).
- **Willpower block**: same shape as Health but with slate-blue fills instead of red.

Steppers (existing `@render stepper` from Campaign.svelte): preserved as small ± circles to the right of each track-label. Visually re-styled to slate-blue but functionally identical (`character::set_field` router calls). Steppers obey `advantageEditAllowed(char)` — when not editable, render as static read-only.

### §5.2 View 2 — Stats

- **Attributes** section: `panel-title` (`Attributes`), then a 3×3 monospace grid (`STR 3` style). Modified attributes get the §6.4 treatment.
- `<hr>` rule.
- **Skills** section: `panel-title` (`Skills`), then a vertical list filtered per §4.3. Each row: name (left) + specialty in italic ochre (`--accent-secondary` — see ARCH §6 for that token; Foundry only) + value (right, red). Modified skills get the §6.4 treatment.
- **Filter footnote**: `Hidden: N skills at zero` in muted small-caps.

When ≥1 active modifier targets attributes/skills, the §6.6 active-modifiers banner appears above the panel.

### §5.3 View 3 — Disciplines

- For each discipline: `disc-name` row with discipline name (uppercase letterspaced) + dot indicator (●●●) + powers list below.
- Powers list: each power on its own line, prefixed with `›`, in `--text-card-dossier` at slightly muted alpha.

Source: `canonical.raw.items` filtered to `type === 'power'`, grouped by `system.discipline`. Discipline level = max power tier among learned powers (existing convention).

For Foundry-only data — Roll20 sources currently don't expose powers; the panel shows the discipline names + dots from `canonical.disciplines[]` and the powers list is empty. No special-case copy.

### §5.4 View 4 — Advantages

- **Section per advantage type**: Merits, Flaws, Backgrounds, Boons. Each section: `panel-title` label + horizontal chip flow.
- Each chip: name + dots (●● for value 2, etc.) + remove-X button (existing `chipRemoveBtn` snippet from Campaign.svelte). When `advantageEditAllowed(char)` is true, an `+ Add` chip ends each section's flow (existing `addBtn` / `addForm_` snippets).
- **Active state** (§6.5): chip has red fill, glow shadow, ◉ marker corner, dots inverted to dark.
- Chip kinds — color-coded borders: merit (slate-blue), flaw (red), background (ochre), boon (existing convention).

Active actor effects (existing `actorFx` filter from Campaign.svelte) render as a separate "Active modifiers (actor)" section below Boons, also as chips. These do **not** click-toggle — they're Foundry-managed and presented informationally only.

## §6 Modifier integration

### §6.1 New `ModifierKind::Stat` variant

```rust
// src-tauri/src/db/modifier.rs (extension to existing enum)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind {
    Pool,
    Difficulty,
    Note,
    Stat,  // NEW — render-time visual stat delta
}
```

`ModifierEffect` already has a `paths: Vec<String>` field today (used by the push-to-Foundry codepath on `pool` effects). The new `Stat` kind reuses this field for its target paths — **no struct shape change needed**. When `kind === Stat`:

- `paths` MUST contain ≥1 canonical attribute/skill path: `attributes.charisma`, `skills.brawl`, etc. (Same vocabulary as `DIFFABLE_PATHS` in `src/lib/saved-characters/diff.ts`.) Multiple paths in one effect mean the same `delta` applies to each path independently.
- `delta` is the integer stat delta (e.g., `+1`, `-2`).
- `scope` may carry an optional freeform context label (e.g., `"vs social rolls"`) used in the visual tooltip alongside the modifier name. May be `None`.
- `note` is unused (`None`).

**No schema migration required.** `effects_json` is a TEXT column with no `CHECK` constraint on contents; the new variant deserializes through the existing tagged-enum serde derives. Existing rows continue to deserialize.

### §6.2 Render-time vs roll-time semantics

**Render-time vs roll-time matrix (revised post-implementation per user feedback):**

| Kind | Card projection | Roll-time consumption |
|---|---|---|
| `Pool` (with `paths.length > 0`) | YES — projects as attribute/skill delta | YES — adds dice to scoped roll (when V5 dice helper lands) |
| `Pool` (with empty `paths`) | NO — pathless pool effect is scope-based, no specific stat target | YES — adds dice to scope-matched rolls |
| `Difficulty` | NO — affects roll target number, not a stat value | YES — adjusts roll difficulty |
| `Note` | NO — informational only | shown as chip in roll dispatch |
| `Stat` | YES — projects as attribute/skill delta | NO — never folded into rolls |

`Pool` is the common-case "+1 Charisma" merit kind: it shows on the card AND folds into rolls. `Stat` is the niche "show on card but don't fold into rolls" kind — useful for narrative-only buffs where the GM wants a visible indicator without mechanical effect on dice. The original spec had Pool roll-time-only and required separate Stat effects for card projection, but user feedback during implementation revealed that the typical mental model is "one effect, both behaviours" — so Pool was promoted to project on the card too. Stat survives but is no longer required for card projection.

The cog-wheel editor (`ModifierEffectEditor.svelte`) exposes a `paths[]` chip-input with autocomplete over `FOUNDRY_ATTRIBUTE_NAMES + FOUNDRY_SKILL_NAMES` (added post-implementation). Both `'stat'` and `'pool'` kinds are first-class options.

### §6.3 `computeActiveDeltas()`

```ts
// src/lib/character/active-deltas.ts  (NEW)

import type { BridgeCharacter, CharacterModifier, SavedCharacter } from '../../types';

export interface DeltaEntry {
  path: string;          // e.g., 'attributes.charisma'
  baseline: number;      // value read from char via canonical path vocabulary
  delta: number;         // sum of all active Stat-kind effects targeting this path
  modified: number;      // baseline + delta
  sources: { modifierId: number; modifierName: string }[]; // for tooltip
}

export function computeActiveDeltas(
  char: BridgeCharacter | SavedCharacter['canonical'],
  modifiers: CharacterModifier[],
): Map<string, DeltaEntry>;
```

Pure function. Iterates `modifiers` filtered to `isActive && (source, sourceId) match char`. For each Stat-kind effect on each matching modifier, walks `effect.paths[]` and adds `effect.delta` to the accumulator under each path. Reads baseline values from `char` via the same path-resolver used by `diff.ts` (factored into a shared helper if not already).

Edge cases:

- A path with `delta === 0` after summation (two opposing modifiers cancel) is **omitted from the map**.
- A `Stat` effect targeting a non-existent path (typo or character missing the field) treats baseline as 0; the entry renders as `0 → delta`.
- Modifiers with `is_active === false` are excluded from input.
- Modifiers belonging to other characters are excluded.

Lives next to `src/lib/saved-characters/diff.ts` in spirit — they share the path-projection vocabulary. If a refactor is convenient, both functions can live in a new `src/lib/character/` umbrella; otherwise keep `active-deltas.ts` next to `diff.ts` and re-export from a shared `paths.ts` if the resolver gets factored. Either is acceptable for Plan B.

### §6.4 Stat-delta visualization (View 2)

An attribute / skill entry whose `path` appears in `computeActiveDeltas`'s output renders with:

- **Original baseline value**: struck through, muted, smaller weight.
- **Modified value**: in `--alert-card-dossier`, bold.
- **Delta badge**: small red pill showing signed delta (`+1`, `-2`).
- **Tooltip on hover**: lists `sources[]` modifier names (newline-joined).

If a path's `delta === 0` after summation, it doesn't appear in the map and renders normally.

### §6.5 Chip click behaviour (View 4)

A merit/flaw/background/boon chip click:

1. Resolves chip's `_id` to `binding: { kind: 'advantage', item_id: <_id> }`.
2. Looks up an existing modifier with that binding from `modifiers.list`.
3. **If exists**: calls `modifiers.setActive(id, !is_active)` (existing API on the store, wraps `set_modifier_active`).
4. **If not exists**: calls `modifiers.materializeAdvantage(source, source_id, item_id, name, description)` (idempotent), then `modifiers.setActive(newRow.id, true)`.

Active chips render with the §5.4 treatment (red fill, glow, ◉ marker).

A **right-click** on the chip opens the existing `ModifierEffectEditor.svelte` popover anchored to the chip — letting the GM author or edit the chip's effects (Stat / Pool / Difficulty / Note) inline. Materialization happens then if needed.

### §6.6 Active-modifiers banner

When the character has ≥1 active modifier (any kind, any binding), the panel renders a banner at the top:

```
┌──────────────────────────────────────────┐
│ ACTIVE MODIFIERS                       2 │
└──────────────────────────────────────────┘
```

Banner styling: `--alert-card-dossier` border, red-tinted background (10% alpha of the alert color), uppercase letterspaced label, count pill at right.

Banner is informational only in v1 (clicking does nothing). A future spec may make it navigate to the GM Screen scrolled to the character's row, but cross-tool nav is out of v1 scope.

## §7 Component architecture

```
src/lib/components/
├── CharacterCard.svelte         (NEW — dossier body, view-aware, modifier-aware)
├── CharacterCardShell.svelte    (NEW — Campaign-flavoured wrapper: rail + slot)
└── (other existing components untouched)

src/lib/character/
└── active-deltas.ts             (NEW — computeActiveDeltas)
```

**`Campaign.svelte` refactor** (~400 lines removed, ~30 lines added):

- Inline card markup at lines 625–1021 deleted.
- Replaced with `<CharacterCardShell {char} {saved} />` per character in both the live grid and the saved grid.
- Collapsible-section state (`expandedAttrs`, `expandedSkills`, `expandedInfo`, `expandedFeats`, `expandedRaw`) removed entirely — Flip mechanic supersedes it.
- Save / Update saved / Compare buttons: lifted into the shell; their handlers (`savedCharacters.save`, `savedCharacters.update`, `openCompare`) move with them as imports.
- Density toolbar control unchanged.
- The `addBtn` / `chipRemoveBtn` / `addForm_` snippets and `stepper` snippet move into `CharacterCard.svelte` (or split into smaller components if the file grows past ~400 lines).

**`CharacterCard.svelte` props:**

```ts
interface Props {
  character: BridgeCharacter;
  viewIndex?: number;          // 1..4, default 1; controlled or uncontrolled
  onViewChange?: (i: number) => void;
}
```

Controlled mode: parent provides `viewIndex` + `onViewChange`. Uncontrolled mode (default): card holds internal `$state` for active view.

**`CharacterCardShell.svelte` props:**

```ts
interface Props {
  character: BridgeCharacter;
  saved: SavedCharacter | null;
}
```

The shell is responsible for: drag handle (inert), source chip, save/update/compare buttons, drift badge. It renders the rail row, then `<CharacterCard {character} />` below it. The shell does NOT pass `viewIndex` — the card stays uncontrolled in Campaign view.

## §8 Data flow

```
bridgeStore                ── live BridgeCharacter
savedCharactersStore       ── SavedCharacter[]
modifiersStore             ── CharacterModifier[]
        │
        ▼  (per-character iter)
Campaign.svelte
        │
        ▼  (props: character, saved)
CharacterCardShell ────── shell-rail (drag, source chip, actions, drift)
        │
        ▼  (slot)
CharacterCard
        │
        │  $derived activeDeltas      = computeActiveDeltas(character, modifiers.list)
        │  $derived activeChipIds     = Set<item_id> of active advantage modifiers for this char
        │  $derived hasActiveModifiers = modifiers.list.some(m => m.is_active && m matches char)
        │
        ├─ View 1 (Basics) — no delta integration
        ├─ View 2 (Stats) — annotates entries whose path is in activeDeltas
        ├─ View 3 (Disciplines) — no delta integration v1 (future seam §12)
        └─ View 4 (Advantages) — chip class .active iff item_id ∈ activeChipIds;
                                  click handler dispatches to modifiers store
```

The card subscribes to `modifiers.list` reactively. `computeActiveDeltas` recomputes per render — pure synchronous loop, fine for a grid of ~12 characters × ~5 modifiers each.

## §9 Error handling

Per ARCH §7. All Tauri commands return `Result<T, String>`; frontend catches in store wrappers and surfaces via toast.

| Failure | Surfaces as |
|---|---|
| `materialize_advantage_modifier` failure on chip click | toast + revert chip visual to inactive (Svelte `$state` rollback) |
| `set_modifier_active` failure on chip click | toast + revert chip visual |
| `computeActiveDeltas` malformed path | function returns the entry with `baseline: 0`; doesn't throw |
| Bridge / saved / modifier store loading | rendered as it is today (loading states inherited from existing stores) |

`computeActiveDeltas` is pure and defensive — never throws. Any internal try/catch logs to console and returns a partial map.

## §10 Plan packaging

Two plans (per ARCH §11 + lean-execution override):

### Plan A — Card body + Campaign refactor (no modifier integration)

Lands the visual redesign and the component split with **no changes to the modifier system**. The card's chip click does NOT dispatch — chips are inert clickable visuals; modifier subscription wiring is left for Plan B. This lets Plan A ship the Camarilla-Dossier visual on its own, validate scaling and view-flip mechanics under live conditions, and keep modifier-integration risk contained.

- New tokens in `:root` (`src/routes/+layout.svelte`).
- `CharacterCard.svelte` (body, all 4 views, flipper). Steppers, `addBtn` / `chipRemoveBtn` / `addForm_` snippets ported from Campaign.svelte. View 4 chips render with `data-active="false"` placeholder (Plan B will wire it).
- `CharacterCardShell.svelte` (Campaign-flavoured rail + slot).
- Refactor `Campaign.svelte`: remove inline markup; remove collapsible-section state; replace with shell components.
- Density toolbar control unchanged; new `--card-scale` custom property added to its density classes.
- `verify.sh` green; manual: cards render with all 4 views; flipper works; density toggle still scales correctly; save/update/compare buttons work; drift badge works; existing per-field stepper editing still hits `character::set_field` correctly.

### Plan B — Modifier integration

- Add `ModifierKind::Stat` variant to `src-tauri/src/db/modifier.rs` (and `src/types.ts` mirror).
- Schema migration: **none required** (verify `effects_json` has no CHECK constraint excluding "stat" — should be a no-op).
- New `src/lib/character/active-deltas.ts` with `computeActiveDeltas` + unit tests.
- Wire `modifiers.list` subscription into `CharacterCard.svelte`.
- Stat-delta annotation rendering on View 2.
- Chip click handler on View 4 (materialize + toggle).
- Right-click chip → existing `ModifierEffectEditor.svelte` popover, anchored to the chip rather than to the card.
- `ModifierEffectEditor.svelte`: add `'stat'` option to the `KINDS` array; add helper text below the kind dropdown explaining the §6.2 render-time vs roll-time asymmetry. The existing `paths[]` chip-input is reused as-is — no new path-picker UI.
- Active-modifiers banner rendering.
- `verify.sh` green; manual: click merit → chip lights up + stat delta appears on View 2; click again → both revert; right-click chip → editor popover; create a Stat effect via editor → delta appears.

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

### Anti-scope

| Plan | MUST NOT touch |
|---|---|
| A | `db/modifier.rs`, `src/types.ts` modifier types, `modifiers.svelte.ts` store, `ModifierEffectEditor.svelte`, `active-deltas.ts` (doesn't exist yet) |
| B | `CharacterCard.svelte` view layout (frozen by Plan A), `CharacterCardShell.svelte`, `Campaign.svelte` shell-rail markup, `:root` token additions (frozen by Plan A) |

### Invariants cited

- Plan A: ARCH §3 (no new storage), §4 (typed Tauri wrappers — components never call `invoke()`), §6 (`:root` color tokens, no hex literals, `box-sizing: border-box`).
- Plan B: same as Plan A plus ARCH §10 (`#[cfg(test)] mod tests` for the Rust enum extension; pure-function tests for `active-deltas.ts`).

### Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

## §11 Testing

Per ARCH §10: **no frontend test framework is installed**, and introducing one is an out-of-scope decision for this spec. Verification is therefore split between Rust-side and manual-UI:

- **Rust**: extend `db/modifier.rs` inline `#[cfg(test)] mod tests` to cover round-trip of the `Stat` kind through `effects_json` serde (insert → list → assert kind, scope, delta deserialize correctly). Add a regression test that re-serializing then deserializing an `effects_json` blob containing all four kinds yields an identical struct.
- **Manual UI verification of `computeActiveDeltas`**: ship the function with its semantics documented in a JSDoc block above its export. Plan B's manual-verification checklist must exercise every edge case from §6.3:
  - Two opposing modifiers cancel → no annotation appears on the affected attribute.
  - Inactive modifier on the character → no annotation.
  - Active modifier on a *different* character → no annotation on this card.
  - Active modifier with a non-existent path → entry renders as `0 → delta`.
  - Two stacking modifiers on the same path → annotation shows the summed delta with both modifier names in the tooltip.
- **No Svelte-component tests** (per ARCH §10).
- `./scripts/verify.sh` green before any commit (covers `npm run check`, `cargo test`, `npm run build`).

If a future spec adds Vitest or another frontend test runner, `computeActiveDeltas` is the natural first citizen for it — pure, no IO, deterministic. That decision is deferred.

## §12 Out of scope / future seams

| Future feature | How this spec accommodates it |
|---|---|
| **Full-page focused view** (clicking name) | `CharacterCard.svelte`'s `viewIndex` prop is already controlled-or-uncontrolled. A future spec adds a `'focused'` mode where the parent component opens a fullscreen overlay containing the same card body. No card body changes. |
| **Modular plug-and-play GM screen** | Card body has zero coupling to `CharacterCardShell`. A future `CharacterCardWidget.svelte` (GM-screen variant) wraps the same body with drag-handle wiring + lock-to-view behavior + remove-from-screen affordance. |
| **Discipline power deltas on View 3** | Stat-kind effects with `scope: powers.<id>.level` could light up specific powers. No data-model change required — `scope` is already freeform path. |
| **Hunger / Health / WP deltas on View 1** | Stat-kind effects with `scope: hunger`, `health.max`, `willpower.max`, etc. could project there. Out of v1 to keep scope tight; the projection logic in §6.3 already handles arbitrary canonical paths. |
| **Active-modifiers banner click navigation** | v1 banner is informational. A future cross-tool nav helper enables clicking it to jump to the GM Screen with the row scrolled into view. |
| **Roll20 chip click / advantage editing** | Phase 2.5 per existing roadmap. The card already supports the click visual; just needs the Roll20 advantage data path wired into the chip's `_id` resolver. |

## §13 Phase placement

This spec is a Phase 2 enhancement (per `2026-04-30-character-tooling-roadmap.md` §5 — "during-play tooling polish"). Plan A is a visual / refactor pass on Campaign view — pure frontend, no IPC change. Plan B is a small additive extension of the Phase 3A GM Screen modifier system, surfacing existing toggle state on the character card.

Goes on the GitHub Project board as **one feature-level parent issue**: "Character card redesign + modifier projection". Plans A and B render as two task-list checkboxes in the parent body, not separate board entries (per project's `feedback_issue_granularity` rule).

## §14 Open questions

These are flagged for plan-time resolution; none block writing the plans.

1. **Path-resolver factoring.** `diff.ts` already has a path resolver implicit in its `read` callbacks. `computeActiveDeltas` needs the same. Does Plan B factor a shared `src/lib/character/paths.ts` and re-export from both, or duplicate the resolver inline? Default: factor on first use.
2. **Steppers re-styling.** The existing steppers (± circles in slate-blue) match dossier accent. But the existing buttons use a generic `--accent` token. Should Plan A (a) introduce new `--accent-card-dossier-stepper` tokens, or (b) just reuse `--accent-card-dossier`? Default: (b) — fewer tokens, same visual outcome.
3. **`addBtn` / `addForm_` snippet location.** When the snippets move out of `Campaign.svelte`, do they live inline in `CharacterCard.svelte` (View 4 only uses them) or as exported helpers in a `chip-helpers.ts`? Default: inline in `CharacterCard.svelte` — co-locate with consumers; export later if the new GM-screen widget shell grows a need for them.
