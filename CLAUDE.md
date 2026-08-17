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

# Run Rust tests only (faster than verify.sh for backend iteration)
cargo test --manifest-path src-tauri/Cargo.toml

# Run all correctness gates (type-check, cargo tests, frontend build)
./scripts/verify.sh
```

`./scripts/verify.sh` is the aggregate gate. Claims of "done" must be
backed by a green run. Expected (non-regression) warnings are documented
in `ARCHITECTURE.md` §10 — do not "fix" them.

**This repo runs one gate, not two — a deliberate override of the global
workflow.** `~/.claude/CLAUDE.md` splits gates in two: a fast item gate
after every item, the full suite once per pack. Here `verify.sh` is
*both*. It runs before every commit, per the rule in Claude-specific
rules below, and no lighter substitute counts.

Why this repo opts out, so nobody re-derives it:

- **It costs 57 seconds warm** (measured 2026-08-18: `npm run check` +
  `cargo test` + `npm run build`, all green). The two-tier split exists
  to keep slow suites off the per-item path; at under a minute there is
  not enough to save for a second gate concept to pay for itself.
- **There is no `scripts/check.sh` here**, so adopting the split would
  mean inventing one and maintaining it.
- **Most items here touch logic** — dice mechanics, V5 rules, bridge
  protocol decoding, character transforms. The global item gate already
  requires the targeted test module for exactly that kind of work, and
  `cargo test` for this crate *is* that module. The lighter gate would
  save almost nothing on the items where it matters.

**Nothing enforces this.** A commit-time hook used to flag violations
(one was caught on 2026-04-26, when a plan step specified only
`cargo check`); it is gone, and `.claude/settings.json` no longer carries
anything that checks. The rule now depends on being read. When writing a
manifest for this repo, put the `verify.sh` step in each item's steps
explicitly rather than leaving it implied.

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

These rules override `~/.claude/CLAUDE.md` §Workflow for this repo.
Everything the global workflow says still applies — scope verdict,
manifests, the hard stop for an explicit go — except where a rule below
says otherwise. The gate override lives up in Commands: one gate, not
two.

- **A spec is an approved design.** If `docs/superpowers/specs/*.md`
  already holds a spec matching the feature, the design gate is
  already passed — do not re-open it. Read the spec, write a
  five-bullet "what I understood" recap, ask at most three
  ambiguity-only questions about things the spec genuinely does not
  answer, and then go to the manifest. Do not present sections for
  re-approval and do not offer alternative approaches: the spec is
  the decision. (That directory is the archive from the previous
  planning system; the specs in it are still authoritative designs,
  even though new planning docs are manifests.)
- **One review per pack, and one implementer per item.** The global
  workflow already puts the code review at pack level, not item
  level; this repo additionally rules out per-item reviewer
  subagents of any kind. Dispatch ONE implementer per item with the
  full item text plus scene-setting context, run `./scripts/verify.sh`
  after it commits, and move on. After every item in the pack is
  committed, run a SINGLE `code-review:code-review` against the full
  branch diff. Item precision, `verify.sh` and the final review are
  the quality gates — per-item reviewers triple-bill for marginal
  catch on the mostly-mechanical work here (router wiring, typed
  wrappers, schema migrations, IPC command registration). When a
  pack's items are tightly coupled, execute them in one session
  instead of fanning out.
- **Tests on demand, not by default.** The manifest item says whether
  tests are required — look for "tests: required" or an explicit test
  step. Default for wiring, refactor, IPC-router and typed-wrapper
  items is no new tests; `verify.sh` is the gate. Write tests first
  for genuine logic: dice mechanics, V5 combat rules, bridge protocol
  decoding, character data transforms. This is the same line the gate
  override draws — those are the items whose `cargo test` coverage is
  the reason the full suite runs per commit here.

## Pointers

- Architecture: `ARCHITECTURE.md`
- Decisions (historical): `docs/adr/`
- Reference material (topic-gated): `docs/reference/`
- Per-feature archive: `docs/superpowers/specs/` and
  `docs/superpowers/plans/`
