# Campaign Card Redesign — Compact Vertical Layout with Density Toggle

## Context

The character cards in `Campaign.svelte` have layout problems at narrow widths (~20rem). The CONSCIENCE track is crammed into the rightmost cell of a 3-column grid and gets crushed. Health and willpower share a `1fr 1fr` row that also cramps. The header leaves the right side empty while hunger/BP fight for space below. The card layout is too horizontal for what is fundamentally a vertical scanning tool.

**Goal:** Redesign the card body so every section gets full card width, the header uses both sides, and a density toggle lets the GM control how many cards fit on screen.

**Primary use case:** Quick glance scanning of vitals (hunger, health, willpower, humanity) across all characters during play. Speed of reading is paramount.

## Design Decisions

### Approach: Compact Header (B)

Selected over stacked rows (A) and labeled compact (C) for best balance of vertical density and readability.

### Card Structure

```
┌──────────────────────────────┐
│ Name                      PC │  ← line 1: name + badge
│ Clan       🩸🩸🩸🩸🩸  BP 3 │  ← line 2: clan + hunger + BP
├──────────────────────────────┤
│      C O N S C I E N C E    │  ← full-width conscience
├──────────────────────────────┤
│ ■ ■ ■ ■ ■ ■ ■               │  ← health (red, full-width)
├──────────────────────────────┤
│ ■ ■ ■ ■ ■                   │  ← willpower (blue, full-width)
├──────────────────────────────┤
│ Auspex••  Presence•••        │  ← disciplines (unchanged)
├──────────────────────────────┤
│ attrs ▾   info ▾      raw ▾ │  ← footer (unchanged)
└──────────────────────────────┘
```

Every row spans the full card width. Nothing shares horizontal space with another section.

### Header — Two Explicit Lines

The header becomes two rows inside a single flex-column container:

- **Line 1:** Character name (left) + PC/NPC badge (right). Both sides used.
- **Line 2:** Clan name italic (left) + hunger drops + BP pill (right). Fills the right side that was previously empty.

No `flex-wrap` trick — two explicit child divs, both `display: flex; justify-content: space-between`.

### Conscience — Full Width, Density-Responsive

- Own row with `container-type: inline-size` for `cqi`-based font sizing.
- At small density: smaller text, no glow.
- At large density: large Last Rites font with `text-shadow` glow.
- Font-size: `min(9cqi, <cap>)` where cap varies by density level.

### Health & Willpower — Separate Full-Width Rows

- Each track gets its own row (no more `1fr 1fr` side-by-side grid).
- Tracks are color-coded: red borders/fills for health, blue for willpower. No text labels needed (approach B).
- Small vertical padding between the two rows for visual separation.
- Box height scales with density level.

### Disciplines, Collapsible Sections, Footer

Unchanged from current implementation. These already work well as full-width rows.

## Density Toggle

### Toolbar Control

A segmented button in the existing toolbar, to the left of the Refresh button:

```
[Auto] [S] [M] [L]    ↺ Refresh
```

- **Auto** (default, active on load): Cards size themselves based on available container width. Uses CSS container queries or a reactive breakpoint on the grid container to pick S/M/L thresholds automatically.
- **S / M / L**: Manual override that locks the grid minmax and card styling to a fixed density.

Clicking any manual size deactivates Auto. Clicking Auto re-enables responsive sizing.

### Density Levels

| Property | S (compact) | M (comfortable) | L (spacious) |
|---|---|---|---|
| Grid `minmax` min | 16rem | 20rem | 28rem |
| Conscience font cap | 1.5rem | 2.5rem | 4rem |
| Conscience glow | none | subtle | full |
| Track box height | 1.4rem | 1.8rem | 2.4rem |
| Card padding | tight (0.4rem) | standard (0.6rem) | generous (0.8rem) |
| Blood drop size | 1.2rem | 1.6rem | 2rem |

### Implementation Approach

Use a CSS custom property on the `.char-grid` container (e.g. `--density: s | m | auto`) set by a Svelte `$state` variable. Card styles reference density via either:

- **Option A:** CSS custom properties per density (`--card-padding`, `--track-height`, etc.) set on `.char-grid` and inherited by cards.
- **Option B:** A class on `.char-grid` (`.density-s`, `.density-m`, `.density-l`) with scoped overrides.

Option A is preferred — it keeps the density logic in CSS and the Svelte code just toggles one variable.

For **Auto mode**: use a `ResizeObserver` on `.char-grid` to measure container width, compute the appropriate density level, and set the CSS custom properties accordingly. When a manual mode is active, skip the observer and use fixed values.

**Auto mode breakpoints** (based on grid container width):
- Container < 500px → S
- 500px ≤ container < 800px → M
- Container ≥ 800px → L

## Files to Modify

- **`src/tools/Campaign.svelte`** — template restructure + style rewrite + density toggle state + ResizeObserver logic

## What Stays Unchanged

- `.char-grid` outer layout (`display: grid`, `align-items: start`)
- Disciplines section markup and styles
- Collapsible attrs/info/raw panels
- Footer
- Blood-drop SVG shape and fill transitions
- Conscience letter `.filled` / `.stained` color logic and strikethrough
- Health/willpower box damage state classes (`.superficial`, `.aggravated`)
- All Tauri/Roll20 integration code (script block helpers, event listeners)

## Verification

1. `npm run check` — zero errors
2. Visual in dev server at multiple widths:
   - Auto mode: cards smoothly transition between S/M/L as window resizes
   - Manual S: many small cards, conscience compact, tracks thin
   - Manual L: fewer large cards, conscience with glow, tracks tall
   - Toggle between modes: no layout jumps or flash
3. CONSCIENCE letters readable at all density levels (no clipping/overflow)
4. Health/willpower tracks visually distinct by color at all sizes
5. Header fills both left and right sides at all widths
6. Variable-height cards don't stretch to match neighbors (`align-items: start`)
