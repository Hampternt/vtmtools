# CLAUDE.md

Guidance for Claude Code when working in this repository. Load this file
plus `ARCHITECTURE.md` for every task. Load anything else only when the
read-scope guide below says to.

## Read-scope guide

- **Always, for any code change:** `ARCHITECTURE.md` — the canonical
  cross-feature reference.
- **Topic-gated** (load only when the topic matches):
  - `docs/reference/roll20-bundle.md` — Roll20 bridge work; Jumpgate
    sheet attribute names; DOM selectors.
  - `docs/reference/foundry-vtm5e-paths.md` — Foundry bridge work;
    WoD5e actor schema dot-paths; multi-splat shape.
  - `docs/reference/foundry-vtm5e-actor-sample.json` — Foundry
    bridge work; live-captured actor wire blob (ground truth).
  - `docs/reference/foundry-vtm5e-rolls.md` — Foundry roll API,
    V5 dice mechanics, ChatMessage shape, hooks for roll mirroring.
  - `docs/reference/foundry-vtm5e-roll-sample.json` — Foundry
    bridge work; live-captured roll ChatMessage.
  - `docs/reference/v5-combat-rules.md` — dice mechanics and combat
    rules.
  - `docs/reference/data-sources.md` and
    `docs/reference/data-sources.kumu.json` — named data sources and
    the Kumu map.
- **On-demand only** (do NOT load by default):
  - `docs/superpowers/specs/*.md` — per-feature design archive.
    Load only when the current task names a specific feature.
  - `docs/superpowers/plans/*.md` — per-feature implementation
    archive. Same rule.
- **On-demand only:** `docs/adr/*.md` — load when researching a
  historical decision or proposing a change that would supersede
  an existing ADR.

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

# Run all correctness gates (type-check, cargo tests, frontend build)
./scripts/verify.sh
```

`./scripts/verify.sh` is the aggregate gate. Claims of "done" must be
backed by a green run. Expected (non-regression) warnings are documented
in `ARCHITECTURE.md` §10 — do not "fix" them.

## Claude-specific rules

- Never run `git status -uall` on this repo — can cause memory issues
  on large working trees.
- Never hardcode hex colors in components — use tokens from `:root` in
  `src/routes/+layout.svelte` (see `ARCHITECTURE.md` §6).
- Never add a theme toggle or light-mode variant (ADR 0004).
- Never use `std::fs` in async command paths — always `tokio::fs`
  (`ARCHITECTURE.md` §5).
- Never revert the destructive-reseed logic in `src-tauri/src/db/seed.rs`
  to a count-check guard (ADR 0002).
- Never commit without running `./scripts/verify.sh` first.
- Never call `invoke(...)` directly from a Svelte component — use the
  typed wrapper in `src/lib/**/api.ts` (`ARCHITECTURE.md` §4).

## Pointers

- Architecture: `ARCHITECTURE.md`
- Decisions (historical): `docs/adr/`
- Reference material (topic-gated): `docs/reference/`
- Per-feature archive: `docs/superpowers/specs/` and
  `docs/superpowers/plans/`
