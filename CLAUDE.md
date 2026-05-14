# CLAUDE.md

vtmtools is a single-GM offline desktop tool for running Vampire: The
Masquerade 5e — Tauri 2 + SvelteKit + Rust/SQLite, dark-only, with two
localhost bridges to Roll20 and Foundry. Single-machine, single-user;
no cloud, no auth, no network beyond loopback.

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
- **On-demand only** (do NOT load by default; **`docs/superpowers/`
  is gitignored** — files exist locally but are never committed; do
  not investigate why they're missing from git history, and never
  `git add -f` to override the ignore):
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

# Run Rust tests only (faster than verify.sh for backend iteration)
cargo test --manifest-path src-tauri/Cargo.toml

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
- Never add a `#[tauri::command]` without updating `ARCHITECTURE.md` §4
  IPC inventory in the same commit — both the module's per-file list
  AND the running total. Drift here is silent and accumulates fast
  (a recent audit caught 31 unregistered commands across 8 modules).
- Roadmap tracking lives in GitHub Issues + the "vtmtools roadmap"
  Project board (https://github.com/users/Hampternt/projects/3).
  Create / modify issues only when the user explicitly asks ("add a
  feature for X to the roadmap", "open an issue for this task").
  Never auto-create issues from detecting completed work.
- Project-board granularity: only **feature-level** parent issues go
  on the board. Subtask issues created during plan execution stay
  off the board — link them from the parent feature's body as a
  ` - [ ] #N` task list instead. The board's job is the wide-angle
  view; per-task tracking happens inside the parent issue.
- When committing work that maps to an open GitHub issue, propose
  `Closes #N` in the commit-message footer. The user closes the
  issue or merges the PR.

## Workflow overrides

These rules override the default behavior of corresponding superpowers
skills for this repo. The `using-superpowers` priority chain says
"User's explicit instructions" are higher priority than skill
defaults — these instructions ARE those user instructions, so they
win. When a workflow override conflicts with a skill's checklist,
follow the override.

- **Spec supersedes brainstorming.** If a spec for the feature
  already exists at `docs/superpowers/specs/*.md` (any file whose
  name matches the feature topic), do **not** invoke
  `superpowers:brainstorming`. Read the spec, write a 5-bullet
  "what I understood" recap, list at most 3 ambiguity-only questions
  grounded in things the spec genuinely doesn't answer, and once
  resolved invoke `superpowers:writing-plans` directly. The spec
  IS the approved design. Do not present sections for re-approval,
  do not propose 2-3 alternative approaches, do not run the spec
  self-review or user spec-review gates a second time. Skip the
  brainstorming `<HARD-GATE>` — it exists to force *some* approved
  design, and the spec already is one.
- **Lean plan execution.** When executing a plan in this repo, do
  **not** invoke `superpowers:subagent-driven-development`'s
  per-task spec-compliance reviewer or per-task code-quality
  reviewer subagents. Per task: dispatch ONE implementer subagent
  with full task text + scene-setting context, run
  `./scripts/verify.sh` after the implementer commits, then move
  on. After ALL plan tasks are committed, run a SINGLE
  `code-review:code-review` against the full branch diff. Plan
  precision + `verify.sh` + final review provide the quality gates;
  per-task reviewer subagents triple-bill for marginal catch on
  the mostly-mechanical work in this repo (router wiring, typed
  wrappers, schema migrations, IPC command registration). Use
  `superpowers:executing-plans` (single-session, no per-task
  fan-out) for plans whose tasks are tightly coupled, and the
  one-implementer-per-task pattern above when tasks are
  independent.
- **TDD on demand.** Subagents executing plan tasks should NOT
  auto-invoke `superpowers:test-driven-development`. The plan
  task text itself states whether tests are required (look for
  "tests: required" or an explicit test step). Default for
  wiring / refactor / IPC-router / typed-wrapper tasks is no new
  tests — `verify.sh` (`npm run check` + `cargo check` +
  `cargo test` + frontend build) is the gate. Reserve TDD for
  genuine logic: dice mechanics, V5 combat rules, bridge protocol
  decoding, character data transforms.
- **Plan tasks include `verify.sh` before any commit.** Already
  in feedback memory; restating here so the rule lives next to
  its enforcement context. Every plan task ending in a commit
  must list `./scripts/verify.sh` as the step immediately before
  the commit step.

## Pointers

- Architecture: `ARCHITECTURE.md`
- Decisions (historical): `docs/adr/`
- Reference material (topic-gated): `docs/reference/`
- Per-feature archive: `docs/superpowers/specs/` and
  `docs/superpowers/plans/`
