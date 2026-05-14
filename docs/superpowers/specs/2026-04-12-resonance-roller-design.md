# vtmtools — Resonance Roller Design Spec
Date: 2026-04-12

## Stack
Tauri 2 + SvelteKit + Rust + SQLite (sqlx) + tauri-plugin-updater

## Architecture
- Shared Rust modules in src-tauri/src/shared/ (dice.rs, resonance.rs, types.rs)
- Tool-specific Tauri commands in src-tauri/src/tools/
- DB access in src-tauri/src/db/ (dyscrasia.rs, seed.rs)
- Tool registry in src/tools.ts — add a new entry to add a new tool
- Inter-tool pub/sub via src/store/toolEvents.ts

## Dice Mechanics
1. Roll d10 for temperament: 1-5 Negligible, 6-8 Fleeting, 9-10 Intense
2. If Fleeting or Intense: roll resonance type (weighted by GM sliders)
   - Default: Phlegmatic 1-3, Melancholy 4-6, Choleric 7-8, Sanguine 9-10
3. If Intense: roll acute check — 9-10 = Acute
4. If Acute: roll or GM-pick from Dyscrasia table for that resonance type

## Temperament Modifiers
- Advantage/disadvantage: roll N dice (1-5), take highest or lowest
- Threshold shift: GM adjusts Negligible/Fleeting/Intense band boundaries directly

## Resonance Type Weighting
7-step slider per type: Impossible/ExtremelyUnlikely/Unlikely/Neutral/Likely/ExtremelyLikely/Guaranteed
Multipliers: 0 / 0.1× / 0.5× / 1× / 2× / 4× / locks to 100%
Applied against base probabilities (30/30/20/20), then normalised.

## Dyscrasia Tables
One table per resonance type in SQLite. Canonical entries seeded on first run.
Custom entries: add/edit/delete supported. Roll random or GM-pick both exposed.

## UI Layout
Step wizard (left panel) + live summary (right panel) + result card (replaces wizard after roll).
Gothic VTM aesthetic: dark background, blood-red accents, parchment result cards.

## Export
format_to_md(json) — pure Rust function, no DB access. Saves .md to ~/Documents/vtmtools/.

## Auto-update
tauri-plugin-updater pointed at GitHub releases. Checks on app launch.
CI: .github/workflows/release.yml builds Linux + Windows on tag push.
