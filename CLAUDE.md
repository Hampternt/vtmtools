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

# Run all existing correctness gates (type-check, cargo tests, frontend build)
./scripts/verify.sh
```

`./scripts/verify.sh` is the aggregate gate: it runs `npm run check`, `cargo test`, and `npm run build`. Rust unit tests live as `#[cfg(test)] mod tests` inside each source file (currently `shared/dice.rs`, `shared/resonance.rs`, `db/dyscrasia.rs`, `tools/export.rs`). There is no frontend test framework.

Expected `verify.sh` warnings (not regressions): `shared/types.rs` types for the Domains Manager (`Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`) trigger "never constructed / never used" — they back migration `0002_chronicle_graph.sql` but aren't yet wired into Tauri commands. `npm run build` also reports an unused `listen` import in `Campaign.svelte` and `Resonance.svelte`. Don't remove any of these without checking with the user — in-progress surface, not dead code.

## Architecture

This is a **Tauri 2 + SvelteKit + TypeScript** desktop app. The frontend is a SPA (static adapter, no SSR). All GM logic lives in the Rust backend; the frontend is display + input only.

### Frontend (`src/`)

- **`tools.ts`** — registry of all tools. Adding a new tool means adding one entry here; the sidebar and lazy-loading are automatic.
- **`routes/+layout.svelte`** — shell layout: sidebar + lazy-loaded tool component. Active tool component is loaded on selection.
- **`src/tools/*.svelte`** — one file per tool (e.g. `Resonance.svelte`). Each tool is an independent page-level component. `DyscrasiaManager.svelte` handles dyscrasia CRUD and random rolling. `Campaign.svelte` is the Roll20 viewer: it listens to `roll20://` events and uses a hardcoded `ATTR` constants map to resolve sheet attribute names from Roll20 Jumpgate character sheets.
- **`src/lib/components/`** — shared UI components used by tools.
- **`src/types.ts`** — all shared TypeScript interfaces (kept thin; mirrors Rust structs).
- **`src/store/toolEvents.ts`** — Svelte writable store for cross-tool event broadcasting (`publishEvent` / `toolEvents`).

Svelte 5 runes are used throughout (`$state`, `$derived`, `$props`, `$effect`). Use `untrack()` when initializing `$state` from a prop to avoid reactive-capture warnings.

### Backend (`src-tauri/src/`)

- **`lib.rs`** — app entry point: registers the Tauri updater plugin, sets up SQLite pool, runs migrations, seeds dyscrasias, registers all Tauri commands, and spawns the Roll20 WebSocket server.
- **`db/`** — SQLite access via `sqlx`. `dyscrasia.rs` has all CRUD + random-roll commands. `seed.rs` populates built-in dyscrasias on first run.
- **`shared/`** — dice logic (`dice.rs`), resonance probability types (`resonance.rs`), and shared serde types (`types.rs`).
- **`tools/`** — one module per tool. `resonance.rs` implements `roll_resonance`. `export.rs` implements `export_result_to_md` (writes Markdown to `~/Documents/vtmtools/`).

All Tauri commands that do I/O must be `async` and use `tokio::fs`, not `std::fs`.

### Roll20 Integration (`src-tauri/src/roll20/`)

A WebSocket server bridges the Tauri app to a browser extension running inside Roll20.

- **`mod.rs`** — `start_ws_server()`: binds `127.0.0.1:7423`, accepts one connection at a time, emits Tauri events to the frontend.
- **`types.rs`** — shared state (`Roll20State`): `characters` (HashMap), `connected` (bool), `outbound_tx` (mpsc channel).
- **`commands.rs`** — Tauri commands: `get_roll20_characters`, `get_roll20_status`, `refresh_roll20_data`, `send_roll20_chat`, `set_roll20_attribute`.

Tauri events emitted to the frontend:

| Event | Payload | When |
|---|---|---|
| `roll20://connected` | none | Extension connects |
| `roll20://disconnected` | none | Extension disconnects |
| `roll20://characters-updated` | `Vec<Character>` | Characters refreshed |

The browser extension connects to port **7423**. Only one extension session is active at a time; state is held in `Arc<Roll20State>` shared between the WS loop and Tauri command handlers.

### Browser Extension (`extension/`)

Chrome extension that runs inside Roll20 and connects to the Tauri WebSocket server on `127.0.0.1:7423`. Contains `content.js` (injected into Roll20 pages), `background.js` (service worker), and `manifest.json`. This is the client side of the Roll20 integration — it reads character data from the Roll20 DOM and relays it to the desktop app.

### Database

SQLite, stored in the Tauri app data directory (`app.path().app_data_dir()`). Migrations are in `src-tauri/migrations/`. `PRAGMA foreign_keys = ON` is enabled on the pool via `SqliteConnectOptions` so `ON DELETE CASCADE` actually fires. Migration `0001_initial.sql` creates `dyscrasias` (id, resonance_type, name, description, bonus, is_custom).

`seed.rs` deletes all `is_custom = 0` rows and reinserts canonical entries on every app start (not "only if empty"). This is intentional — it keeps built-in data fresh when the seed changes. Do not revert to the count-check guard.

`resonance_type` CHECK constraint uses `'Melancholy'` (not `'Melancholic'`). Source material uses both spellings; the DB enforces the former.

In `DyscrasiaCard`: `description` = full effect text shown in the card body; `bonus` = short one-line mechanical tag shown in the card footer.

Migration `0002_chronicle_graph.sql` adds three tables for the Domains Manager tool: `chronicles` (one row per running game), `nodes` (any discrete thing — area, character, institution, business, merit), and `edges` (typed directional relationships between nodes). `nodes.type` and `edges.edge_type` are freeform user-authored strings; no enum enforcement. Custom fields live in `nodes.properties_json` / `edges.properties_json` as a JSON array of typed Field records (each `{name, type, value}`). Deleting a chronicle cascades to its nodes and edges; deleting a node cascades to its edges. The `"contains"` edge type is the UI's convention for hierarchy/drilldown (and a partial unique index enforces at most one `contains` parent per node) but other edge types are unrestricted.

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

### Design docs

- **`docs/design/data-sources.md`** — names and describes every data source in the app (GM Roll Config, Dice Engine, Dyscrasia Store, Roll20 Live Feed, etc.). Use these names when referencing data flows in other docs or discussions.
- **`docs/design/data-sources-kumu.json`** — Kumu-importable JSON map of all data sources, tools, and their connections. Linked as a remote blueprint in Kumu so the visual map stays in sync with the repo. **When you add a new tool, data source, or integration, update this JSON to reflect the change.**
