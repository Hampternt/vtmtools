# Docs Reorg & ARCHITECTURE.md Design

**Date:** 2026-04-19
**Status:** Draft
**Scope:** Documentation structure only. No code changes, no runtime behavior changes, no new policy beyond what is already enforced in `CLAUDE.md` and the codebase.

---

## Goal

Consolidate project documentation into a clear four-tier layer cake so future feature specs/plans have a stable reference, Claude knows deterministically which files to load per task, and plans are structured for parallel sub-agent-driven execution.

The current state has a working feature-plan pipeline (`docs/superpowers/specs/` + `docs/superpowers/plans/`) but no stable cross-feature "constitution" — architecture is scattered across a fat `CLAUDE.md` and a mixed-purpose `docs/design/` directory. Every new spec re-derives the domain model, contracts, and invariants, and sessions spend tokens guessing which files to read.

This reorg introduces `ARCHITECTURE.md` as the canonical cross-feature reference, slims `CLAUDE.md` to Claude-specific operational guidance plus a deterministic read-scope guide, renames `docs/design/` to `docs/reference/` to match its actual function, and establishes `docs/adr/` as an append-only decision log.

---

## Scope

### In Scope

- Create `ARCHITECTURE.md` at repo root with 13 sections (see §"ARCHITECTURE.md structure" below).
- Slim `CLAUDE.md` to a thin operational file (target ~80 lines) with a read-scope guide at the top, commands, and Claude-specific rules. Architecture prose moves to `ARCHITECTURE.md`.
- Rename `docs/design/` to `docs/reference/`. Rename `docs/reference/roll20-vtt-bundle-analysis.md` to `docs/reference/roll20-bundle.md`. The Kumu JSON file keeps its role; rename to `data-sources.kumu.json` for naming consistency.
- Create `docs/adr/` with `template.md` and five retroactive ADRs.
- Update all cross-references in existing files (`CLAUDE.md`, existing specs/plans that cite `docs/design/…`, source comments if any) to the new paths.
- Update `docs/reference/data-sources.md` and the Kumu JSON if they reference the old path layout.

### Out of Scope

- No content edits to existing files in `docs/superpowers/specs/` or `docs/superpowers/plans/` — the archive stays frozen as a historical record.
- No changes to source code, migrations, tests, or runtime behavior.
- No new rules or policy beyond what already exists. This is reorganization, not drift.
- No hooks, settings, or permission changes — those are a separate follow-on task that will be able to reference the conventions this reorg establishes.
- No new retroactive ADRs beyond the five listed. Further backfill can be added later as individual ADRs.
- No changes to the `superpowers:brainstorming` → `superpowers:writing-plans` → `superpowers:executing-plans` pipeline itself.

---

## Target Directory Structure

```
/ (repo root)
├── CLAUDE.md                    # slim: commands, read-scope guide, claude-specific rules
├── ARCHITECTURE.md              # canonical cross-feature reference
├── docs/
│   ├── reference/               # renamed from design/; stable domain knowledge
│   │   ├── v5-combat-rules.md
│   │   ├── roll20-bundle.md     # renamed from roll20-vtt-bundle-analysis.md
│   │   ├── data-sources.md
│   │   └── data-sources.kumu.json
│   ├── adr/                     # append-only decision log
│   │   ├── template.md
│   │   ├── 0001-tauri-2-stack.md
│   │   ├── 0002-destructive-reseed.md
│   │   ├── 0003-freeform-node-edge-types.md
│   │   ├── 0004-dark-only-theming.md
│   │   └── 0005-roll20-ws-extension-bridge.md
│   └── superpowers/
│       ├── specs/               # unchanged archive
│       └── plans/               # unchanged archive
```

---

## ARCHITECTURE.md Structure

Thirteen sections, in this order. Each section is a stable cross-feature reference; feature-specific details belong in specs, not here.

### §1 Overview & stack

One short paragraph naming the stack (Tauri 2 + SvelteKit SPA + Rust + SQLite + Roll20 WebSocket bridge) and the product posture (single-GM desktop tool, offline-capable, dark-only). Links to relevant ADRs for stack decisions.

### §2 Domain model

Core data shapes in real syntax, not prose. Includes the shared Rust structs from `src-tauri/src/shared/types.rs` and the mirrored TypeScript interfaces from `src/types.ts`:

- Dyscrasia domain: `Dyscrasia` struct + SQL schema reference.
- Chronicle graph domain: `Chronicle`, `Node`, `Edge`, `Field`, `FieldValue` (including `StringFieldValue`, `NumberFieldValue`), `EdgeDirection`.
- Dice/resonance domain: temperament/resonance/acute types from `shared/resonance.rs` and `shared/dice.rs`.
- Roll20 domain: `Character` and related payload shapes.

Each shape is the canonical contract — features that consume or mutate the shape must cite this section.

### §3 Storage strategy

Where each piece of data lives and under what policy:

- SQLite at `app.path().app_data_dir()` as the durable store for dyscrasias and the chronicle graph.
- Migration policy: sequential `src-tauri/migrations/NNNN_*.sql`; `PRAGMA foreign_keys = ON` at pool init.
- Seed policy: destructive reseed of non-custom rows on every start (intentional — see ADR 0002).
- Ephemeral state: Svelte runes stores (`src/store/*`), `Arc<Roll20State>` for the WS bridge.
- Export artifacts: Markdown to `~/Documents/vtmtools/` via `tokio::fs`.
- No cloud, no remote sync (see §11 Non-goals).

### §4 I/O contracts

Named surfaces at module boundaries. Feature tasks that cross any of these boundaries must honor the contract shape declared here.

- **Tauri IPC commands** — organized by module (`db/dyscrasia`, `tools/resonance`, `tools/export`, `roll20/commands`, domains-manager commands). Each command's request/response shape is the contract.
- **Roll20 WebSocket protocol** — localhost-only `127.0.0.1:7423`, single session, message shapes between extension and `src-tauri/src/roll20/*`.
- **Tauri events (backend → frontend)** — `roll20://connected`, `roll20://disconnected`, `roll20://characters-updated`.
- **Svelte cross-tool pub/sub** — `src/store/toolEvents.ts` (`publishEvent` / subscribe).
- **Tools registry** — `src/tools.ts` is THE add-a-tool seam; sidebar + lazy-loading auto-wire off this registry.

### §5 Module boundaries

Forbidding rules that must hold across features:

- Only `src-tauri/src/db/*` talks to SQLite.
- Only `src-tauri/src/roll20/*` talks to the WebSocket server.
- Frontend components never import SQL drivers or call the DB directly — only via Tauri `invoke(...)`.
- All Tauri commands doing I/O are `async` and use `tokio::fs` (never `std::fs`).
- Frontend imports only cross component → component via `src/lib/components/*` and `src/tools/*`; tools never import other tools directly — cross-tool coordination goes through `toolEvents` or shared stores.

### §6 Invariants

Properties that must hold across features. Violations caught in review are merge blockers.

- `PRAGMA foreign_keys = ON` is enabled on the pool via `SqliteConnectOptions`.
- `ON DELETE CASCADE` is the chosen integrity model for chronicle graph and dyscrasia relationships.
- `resonance_type` uses `'Melancholy'` spelling (not `'Melancholic'`) — DB CHECK constraint enforces this.
- Seed on every start is destructive for `is_custom = 0` rows (ADR 0002).
- CSS colors use tokens from `:global(:root)` in `src/routes/+layout.svelte`; hex is allowed only for one-off transition/glow states with no semantic token.
- Layout and typography sizes are `rem`, never `px` (fluid typography via `html { font-size: clamp(…) }`).
- Grid card layouts use CSS Grid with `align-items: start`, never CSS multi-column (incompatible with `animate:flip`).
- In Svelte 5 runes mode, `in:`/`out:` transitions are placed on elements whose lifecycle is controlled by the enclosing `{#each}` / `{#if}`, not on runes-mode component roots.
- Only one Roll20 extension session is active at a time; WS state lives in `Arc<Roll20State>`.
- Dark-only — no theme toggle exists or will be added (ADR 0004).

### §7 Error handling

How failures propagate across the Rust ↔ Tauri IPC ↔ Svelte boundary.

- Rust: `Result<T, E>` with specific error types per module; `thiserror`-style or string-mapped as each module already does — consistency within a module is required, cross-module harmonization is not.
- Tauri commands return `Result<T, String>` at the IPC boundary (Tauri serializes `Err(String)` to the frontend as a rejected promise).
- Frontend catches rejected `invoke` promises, surfaces user-visible errors via toast or inline error state, logs raw error to console.
- Panics in commands are bugs, not error flow; never `unwrap()` in production command paths — use `?` or explicit error mapping.
- WebSocket disconnect is expected flow: `roll20://disconnected` event fires, UI falls back to "not connected" state, next extension connect restores session.
- Database errors from `sqlx` propagate as `Err(String)` with a stable message prefix per command (e.g. `"db/dyscrasia.create: "`) so the frontend can categorize without parsing.

### §8 Security model

Trust boundaries and assumptions for a single-user local desktop tool.

- **Trust posture**: single user, single machine, no multi-user scenario. No authentication, no authorization, no user-level access control — these are non-goals (see §11).
- **Network surface**: exactly one listener — `127.0.0.1:7423` WebSocket. Must never bind to `0.0.0.0` or any routable interface. Any other external network call is out of scope for this app.
- **Roll20 WS trust**: the WS protocol trusts any localhost client. Rationale: only processes on the user's machine can reach it, which is equivalent to trusting the user. Do not add authentication to the WS without a specific threat that justifies it.
- **Tauri capabilities/ACL**: `src-tauri/capabilities/*.json` is the effective frontend-callable command allowlist. Adding a new Tauri command requires the capability JSON to list it; otherwise the IPC call fails closed.
- **Filesystem write scope**: Rust side writes only to `app.path().app_data_dir()` (for SQLite) and `~/Documents/vtmtools/` (for exports). New write paths must be justified and added here.
- **Browser extension DOM-read surface**: the extension reads Roll20 DOM nodes as documented in `docs/reference/roll20-bundle.md`. Extension never sends data outside the localhost WS.
- **Secrets**: none. The app has no API keys, tokens, or credentials. If a future feature introduces one, this section is updated and an ADR filed.

### §9 Extensibility seams

Named places to add things, so future feature specs can cite the seam instead of re-deriving where to hook in.

- **Add a tool**: add one entry to `src/tools.ts`; sidebar + lazy-loaded component wiring is automatic.
- **Add a schema change**: add a new `src-tauri/migrations/NNNN_*.sql` file; migrations run on app start. Update corresponding `shared/types.rs` + `src/types.ts` shapes in the same change.
- **Add a node/edge type**: chronicle graph types are freeform user-authored strings (ADR 0003) — no code change required for new types, only the UI's autocomplete lists update.
- **Add a cross-tool event**: publish via `toolEvents.ts` store. Subscribers are loose; document the event name + payload shape near the publisher.
- **Add a Tauri command**: declare in the relevant `src-tauri/src/**/commands.rs`, register in `lib.rs`, list in the capability JSON, then add the typed wrapper in `src/lib/**/api.ts`.
- **Add a property field type**: extend `FieldValue` enum in `shared/types.rs`, add corresponding TS mirror in `src/types.ts`, register a widget in the property-editor registry.

### §10 Testing & verification

- Rust unit tests live as `#[cfg(test)] mod tests` inside each source file (`shared/dice.rs`, `shared/resonance.rs`, `db/dyscrasia.rs`, `tools/export.rs`, plus newer domains-manager test modules).
- No frontend test framework is installed — this is a deliberate current choice. If a feature requires frontend tests, that's a scope change to be raised explicitly, not assumed.
- `./scripts/verify.sh` is the aggregate gate: runs `npm run check`, `cargo test`, `npm run build`. All claims of "done" must be backed by a green `verify.sh` run.
- Expected (non-regression) warnings are listed here so sub-agents don't "fix" them:
  - `shared/types.rs`: the Domains Manager's core types (`Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`) are fully wired into Tauri commands and the Svelte UI as of 2026-04-19, so they no longer trigger "never constructed / never used."
  - `shared/types.rs`: `FieldValue` variants `date`, `url`, `email`, and `reference` may still surface "never constructed" — the v1 Domains UI uses only `string`, `text`, `number`, and `bool` widgets. The unused variants ship with the backend by design, as extensibility seams for future property widgets (see §9); do not remove them.
  - `npm run build`: unused `listen` import in `Campaign.svelte` and `Resonance.svelte`.

### §11 Plan & execution conventions

Plans produced against this architecture are structured for parallel sub-agent-driven execution (`superpowers:subagent-driven-development`, `superpowers:dispatching-parallel-agents`). Each plan task declares:

- **Files touched** — tight scope, one coherent area per task.
- **Files NOT touched** — the anti-scope boundary. Prevents silent collisions when two sub-agents run concurrently.
- **Depends on** — explicit predecessor task IDs, or `none` if independent.
- **Invariants cited** — pointer to specific ARCHITECTURE.md sections the task must honor (e.g. "§4 I/O contracts: Tauri IPC — shape of `get_dyscrasias` is stable").

Seams between parallel tasks are drawn along §4 (I/O contracts). If two tasks share a contract, the contract shape is settled in a preliminary task before either implementation task starts; both parallel tasks then work behind the frozen contract.

Verification gate: every sub-agent runs `./scripts/verify.sh` before reporting success. Green output is required; self-reports without verification are not accepted.

Isolation: prefer `superpowers:using-git-worktrees` for multi-agent dispatch so concurrent edits don't collide in a shared working tree.

### §12 Non-goals

Explicitly out of scope for this application. Future specs that propose any of these must first raise a scope change.

- No multi-user or multi-tenant operation.
- No cloud sync, remote storage, or server-backed state.
- No light-mode / theme toggle / configurable color scheme (ADR 0004).
- No authentication or authorization of any kind.
- No network surface beyond the single `127.0.0.1:7423` WebSocket listener.
- No ingestion path for Roll20 data other than the browser extension bridge (ADR 0005).
- No live multi-session Roll20 support — one extension session at a time.
- No frontend testing framework.

### §13 ADR index

Linked table of all decision records with status (`accepted` | `superseded by NNNN`). The table is the canonical entry point; ADR files are the detail.

---

## CLAUDE.md — New Shape

Target length ~80 lines, hard cap ≤100 lines. The file becomes a Claude-facing operational doc, not an architecture doc.

**Top of file (new):**

```markdown
## Read-scope guide

- **Always, for any code change**: load `ARCHITECTURE.md`. It is the canonical cross-feature reference.
- **Topic-gated**:
  - Roll20 bridge work → `docs/reference/roll20-bundle.md`.
  - Dice / combat mechanics → `docs/reference/v5-combat-rules.md`.
  - Data-flow / Kumu map work → `docs/reference/data-sources.md`.
- **On-demand only** (do NOT load unless the task names one):
  - `docs/superpowers/specs/*.md` — per-feature design archive.
  - `docs/superpowers/plans/*.md` — per-feature implementation archive.
- **On-demand only**: `docs/adr/*.md` — load when researching historical decisions or proposing a change that would supersede one.
```

**Kept in CLAUDE.md:**

- Commands block (daily operational; not stable architecture).
- Claude-specific "don't do this" rules (no `-uall` flag, no hex in CSS, no theme toggle, always use tokens, always run `verify.sh` before claiming done).
- Pointer section at bottom: "For architecture, see `ARCHITECTURE.md`. For decision history, see `docs/adr/`."

**Moved out of CLAUDE.md to ARCHITECTURE.md:**

- Frontend/Backend/Roll20/Database architecture prose → §1–§5.
- Dice mechanics detail → §2 (domain model).
- Theming token table, CSS layout gotchas, Svelte 5 transition gotcha → §6 (invariants).
- Expected `verify.sh` warnings list → §10 (testing & verification).
- Design-doc references → §9 (extensibility seams) + pointers into `docs/reference/`.

---

## ADR Conventions

### Template

`docs/adr/template.md` is the authoring template:

```markdown
# NNNN: <Title>

**Status:** accepted | superseded by NNNN
**Date:** YYYY-MM-DD

## Context

What was the situation that forced a choice?

## Decision

What was chosen?

## Consequences

What changes as a result? What trade-offs are accepted?

## Alternatives considered

Briefly, what else was on the table and why it was not chosen.
```

### Numbering & supersession

- ADRs are numbered sequentially starting at `0001`. Numbers never reused.
- ADRs are append-only. When a decision changes, write a new ADR; mark the old one `superseded by NNNN`.
- `ARCHITECTURE.md` reflects the *current* state only; the ADR log is where history lives.

### Retroactive ADRs (written as part of this reorg)

1. **0001 — Tauri 2 stack.** Why Tauri 2 + SvelteKit + SQLite over Electron or a web-only deployment. Desktop-first, single-user GM tool, small binary, native FS access.
2. **0002 — Destructive reseed of non-custom dyscrasias.** Why `seed.rs` deletes all `is_custom = 0` rows on every start rather than using a count-check guard. Keeps built-in data fresh when the seed changes. Do not revert to the count-check.
3. **0003 — Freeform strings for `nodes.type` and `edges.edge_type`.** Why the chronicle graph does not enforce an enum. Supports extensibility without migrations; `"contains"` is the one convention enforced by a partial unique index.
4. **0004 — Dark-only theming.** Why no toggle. Single product posture, no light-mode work budget, token-only palette.
5. **0005 — Roll20 integration via local WebSocket + browser extension.** Why a WS bridge on `127.0.0.1:7423` rather than API scraping or direct character-sheet import. Live feed, extension reads DOM the user is already authorized to see, no Roll20 API dependency.

---

## Migration Plan Summary

The full implementation plan (produced by `superpowers:writing-plans` after spec approval) will break this into independent, parallel-safe tasks along these seams:

- ARCHITECTURE.md authoring (independent; 13 sections can be distributed across sub-agents if desired).
- CLAUDE.md slim + read-scope guide (depends on ARCHITECTURE.md skeleton existing so pointers resolve).
- `docs/design/` → `docs/reference/` rename + file renames (independent; grep-and-replace for old paths is a follow-up task).
- `docs/adr/template.md` + 5 retroactive ADRs (independent; each ADR can be written in parallel).
- Cross-reference sweep: update any `docs/design/…` mention in existing files to the new path.
- Final verification: `./scripts/verify.sh` green + `grep -r "docs/design"` returns zero matches outside of historical ADRs / specs.

This is a summary; the plan is authoritative.

---

## Acceptance Criteria

1. `ARCHITECTURE.md` exists at repo root; all 13 sections populated; no `TBD` / `TODO` placeholders.
2. `CLAUDE.md` is ≤ ~100 lines and opens with the read-scope guide.
3. `docs/reference/` exists and contains the renamed files; `docs/design/` no longer exists.
4. `docs/adr/` contains `template.md` and ADRs `0001`–`0005`; each ADR follows the template and has status `accepted`.
5. `./scripts/verify.sh` exits green after the reorg.
6. `grep -r "docs/design" -- ':!docs/superpowers/specs' ':!docs/superpowers/plans' ':!docs/adr'` returns zero matches (historical archives are exempt from the rename sweep; ADRs document history and are allowed to reference the old path in `Context:` if justified).
7. ARCHITECTURE.md §11 describes plan/execution conventions; the next plan produced from this spec follows the declared-file-scope + invariant-citation format.

---

## Non-goals for this Reorg

- Not a CLAUDE.md policy rewrite — same rules, reorganized location.
- Not a frontend-test introduction — §10 documents the current no-framework state.
- Not a hooks/settings update — tracked as a separate follow-on task that will reference these conventions.
- Not a content edit to `docs/superpowers/specs/*` or `docs/superpowers/plans/*` — the archive is frozen.
- Not a dependency update or runtime-code change.
