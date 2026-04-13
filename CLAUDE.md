# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Type-check frontend (run after every Svelte/TS change)
npm run check

# Run dev server only (frontend, no Tauri desktop window)
npm run dev

# Run full Tauri desktop app in dev mode
npm run tauri dev

# Build Tauri desktop release
npm run tauri build

# Compile-check Rust only
cargo check --manifest-path src-tauri/Cargo.toml
```

There are no test suites. `npm run check` (svelte-check + tsc) is the primary correctness gate.

## Architecture

This is a **Tauri 2 + SvelteKit + TypeScript** desktop app. The frontend is a SPA (static adapter, no SSR). All GM logic lives in the Rust backend; the frontend is display + input only.

### Frontend (`src/`)

- **`tools.ts`** — registry of all tools. Adding a new tool means adding one entry here; the sidebar and lazy-loading are automatic.
- **`routes/+layout.svelte`** — shell layout: sidebar + lazy-loaded tool component. Active tool component is loaded on selection.
- **`src/tools/*.svelte`** — one file per tool (e.g. `Resonance.svelte`). Each tool is an independent page-level component.
- **`src/lib/components/`** — shared UI components used by tools.
- **`src/types.ts`** — all shared TypeScript interfaces (kept thin; mirrors Rust structs).
- **`src/store/toolEvents.ts`** — Svelte writable store for cross-tool event broadcasting (`publishEvent` / `toolEvents`).

Svelte 5 runes are used throughout (`$state`, `$derived`, `$props`, `$effect`). Use `untrack()` when initializing `$state` from a prop to avoid reactive-capture warnings.

### Backend (`src-tauri/src/`)

- **`lib.rs`** — app entry point: sets up SQLite pool, runs migrations, seeds dyscrasias, registers all Tauri commands.
- **`db/`** — SQLite access via `sqlx`. `dyscrasia.rs` has all CRUD + random-roll commands. `seed.rs` populates built-in dyscrasias on first run.
- **`shared/`** — dice logic (`dice.rs`), resonance probability types (`resonance.rs`), and shared serde types (`types.rs`).
- **`tools/`** — one module per tool. `resonance.rs` implements `roll_resonance`. `export.rs` implements `export_result_to_md` (writes Markdown to `~/Documents/vtmtools/`).

All Tauri commands that do I/O must be `async` and use `tokio::fs`, not `std::fs`.

### Database

SQLite, stored in the Tauri app data directory (`app.path().app_data_dir()`). Migrations are in `src-tauri/migrations/`. Schema currently has one table: `dyscrasias` (id, resonance_type, name, description, bonus, is_custom).

### VTM 5e Dice Mechanics

- **Temperament roll**: 1–5d10 pool, take best or worst die. Result bucketed into Negligible / Fleeting / Intense by configurable thresholds (default: ≤5 Negligible, ≤8 Fleeting, 9–10 Intense).
- **Resonance type**: weighted random selection across Phlegmatic / Melancholy / Choleric / Sanguine (weights: exclude / low / neutral / high).
- **Acute check**: separate d10 roll; 9–10 = Acute (triggers dyscrasia selection).

### Dark mode / theming

The app is dark-only — no theme toggle exists or should be added.

All colors are CSS custom properties defined in `:global(:root)` in `src/routes/+layout.svelte`. Always use tokens, never hardcode hex values in components:

| Token group | Tokens |
|---|---|
| Text (5 levels) | `--text-primary` `--text-label` `--text-secondary` `--text-muted` `--text-ghost` |
| Surfaces | `--bg-base` `--bg-card` `--bg-raised` `--bg-input` `--bg-sunken` `--bg-active` |
| Borders | `--border-faint` `--border-card` `--border-surface` `--border-active` |
| Accents | `--accent` `--accent-bright` `--accent-amber` |
| Temperament | `--temp-negligible` `--temp-negligible-dim` `--temp-fleeting-dim` `--temp-intense-dim` |

Fluid typography is set on `html` in the same file: `font-size: clamp(16px, 1.0vw, 32px)`. All component sizes use `rem`, so they scale automatically. Do not add `px`-based font sizes or layout widths — use `rem` so they scale with the root.

Hardcoded hex is acceptable only for one-off transition states with no semantic equivalent (e.g. hover intermediates, glow shadows).

### CSS layout gotchas

There is no global `box-sizing: border-box` reset. Any element using both `width: 100%` and `padding` must also set `box-sizing: border-box`, otherwise it overflows its container by `2×padding`.

Card grids (Dyscrasias, AcutePanel) use **CSS Grid** (`display: grid; grid-template-columns: repeat(auto-fill, minmax(..., 1fr)); align-items: start`). Grid fills left-to-right, row-by-row. `align-items: start` is required so variable-height cards don't stretch to fill their row. Do not use CSS multi-column — it is incompatible with `animate:flip` (FLIP translates elements by pixel offsets, which breaks across anonymous column-box boundaries and leaves visible empty spaces during animation).

In Svelte 5 runes mode, `in:`/`out:` transition directives must be placed on elements whose lifecycle is **directly controlled** by the same `{#each}` or `{#if}` block that creates them. Transitions on a runes-mode component's root element do not fire when the parent's `{#each}` adds the component — Svelte 5 does not propagate intro signals across runes-mode component boundaries. Use a plain wrapper `<div in:scale out:fade>` in the parent's `{#each}` block. In Grid, no `break-inside: avoid` is needed on wrapper divs.
