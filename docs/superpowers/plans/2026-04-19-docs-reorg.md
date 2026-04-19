# Docs Reorg + ARCHITECTURE.md Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the docs reorg defined in `docs/superpowers/specs/2026-04-19-docs-reorg-design.md` — introduce ARCHITECTURE.md, slim CLAUDE.md, rename `docs/design/` to `docs/reference/`, establish `docs/adr/` with template + 5 retroactive ADRs, update cross-refs, verify green.

**Architecture:** Parallel-first decomposition. Phase A (7 tasks) is independent — renames + 6 new ADR/template files — and can run fully concurrently. Phase B (ARCHITECTURE.md) blocks on Phase A because §13 indexes the ADRs and prose cites `docs/reference/` paths. Phase C (slim CLAUDE.md) blocks on Phase B because its read-scope guide cites ARCHITECTURE.md sections. Phase D (cross-ref sweep) blocks on Phase A. Phase E (verify) blocks on everything.

**Tech Stack:** Plain Markdown files, `git mv` for renames, `Grep` for path sweeps, `./scripts/verify.sh` as the correctness gate. No code changes, no runtime impact.

---

## Conventions for Every Task

Each task declares:

- **Files (create / modify / delete):** Exact paths.
- **Anti-scope:** Files the task MUST NOT touch (prevents parallel collisions).
- **Depends on:** Predecessor task IDs or `none`.
- **Invariants cited:** References into the spec or ARCHITECTURE.md section the task must honor.

Every commit message ends with the standard co-author trailer:

```
Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
```

`docs/superpowers/` is gitignored; use `git add -f` when staging files under it (mirrors the repo's existing pattern — see earlier spec/plan commits).

No task writes to `src/`, `src-tauri/`, `extension/`, `migrations/`, `scripts/`, `package.json`, `Cargo.toml`, or any runtime code. This is a docs-only reorg.

---

## Phase A — Independent Prep (7 parallel-safe tasks)

These 7 tasks have no dependencies on each other and touch disjoint file sets. Dispatch in parallel.

---

### Task 1: Rename docs/design → docs/reference + rename internal files

**Files:**

- Move (with `git mv`):
  - `docs/design/data-sources.md` → `docs/reference/data-sources.md`
  - `docs/design/data-sources-kumu.json` → `docs/reference/data-sources.kumu.json`
  - `docs/design/roll20-vtt-bundle-analysis.md` → `docs/reference/roll20-bundle.md`
  - `docs/design/v5-combat-rules.md` → `docs/reference/v5-combat-rules.md`

**Anti-scope:** Do NOT touch CLAUDE.md, ARCHITECTURE.md (it doesn't exist yet), `docs/adr/`, `docs/superpowers/`, or any source file.

**Depends on:** none.

**Invariants cited:** Spec §Target Directory Structure.

- [ ] **Step 1: Verify current state of `docs/design/`**

Run: `ls docs/design/`
Expected: lists exactly `data-sources-kumu.json`, `data-sources.md`, `roll20-vtt-bundle-analysis.md`, `v5-combat-rules.md`.

If any file is missing or extra files are present, stop and raise the discrepancy before proceeding.

- [ ] **Step 2: Create the new directory and move files with `git mv`**

Run:

```bash
mkdir -p docs/reference
git mv docs/design/data-sources.md docs/reference/data-sources.md
git mv docs/design/data-sources-kumu.json docs/reference/data-sources.kumu.json
git mv docs/design/roll20-vtt-bundle-analysis.md docs/reference/roll20-bundle.md
git mv docs/design/v5-combat-rules.md docs/reference/v5-combat-rules.md
rmdir docs/design
```

Expected: no output; files moved; old directory removed.

- [ ] **Step 3: Verify new state**

Run:

```bash
ls docs/reference/
test ! -d docs/design && echo "old dir gone"
```

Expected: new dir lists 4 files; "old dir gone" printed.

- [ ] **Step 4: Check inbound references inside moved files**

Run:

```bash
grep -n "docs/design" docs/reference/*.md docs/reference/*.json || echo "no self-refs"
```

If matches are found, they are old-path self-references in the moved files — fix them in-place with `Edit`, replacing `docs/design/` with `docs/reference/` and `roll20-vtt-bundle-analysis.md` with `roll20-bundle.md` and `data-sources-kumu.json` with `data-sources.kumu.json`.

Note: the Kumu JSON may include a public URL or path-like string referring to its own old name. Update only textual references, not opaque IDs.

- [ ] **Step 5: Commit**

```bash
git add -A docs/reference docs/design
git commit -m "$(cat <<'EOF'
docs: rename docs/design to docs/reference (step 1 of docs reorg)

Rename reflects the actual role of these files — stable, topic-gated
reference knowledge, not design artifacts. Also renames
roll20-vtt-bundle-analysis.md to roll20-bundle.md and
data-sources-kumu.json to data-sources.kumu.json for naming consistency.

See docs/superpowers/specs/2026-04-19-docs-reorg-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created on the current branch.

---

### Task 2: Create docs/adr/template.md

**Files:**

- Create: `docs/adr/template.md`

**Anti-scope:** Do NOT touch CLAUDE.md, ARCHITECTURE.md, `docs/reference/`, `docs/superpowers/`, source code, or any other ADR file.

**Depends on:** none.

**Invariants cited:** Spec §ADR Conventions — Template.

- [ ] **Step 1: Create the directory and template file**

Create `docs/adr/template.md` with this exact content:

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

- [ ] **Step 2: Verify**

Run: `test -f docs/adr/template.md && wc -l docs/adr/template.md`
Expected: file exists; around 18 lines.

- [ ] **Step 3: Commit**

```bash
git add docs/adr/template.md
git commit -m "$(cat <<'EOF'
docs(adr): add ADR template

Establishes the authoring format for docs/adr/NNNN-<slug>.md files.
ADRs are append-only; when a decision changes, write a new ADR and mark
the superseded one accordingly.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

### Task 3: Write ADR 0001 — Tauri 2 stack

**Files:**

- Create: `docs/adr/0001-tauri-2-stack.md`

**Anti-scope:** Do NOT touch any other ADR file, CLAUDE.md, ARCHITECTURE.md, `docs/reference/`, `docs/superpowers/`, or source code.

**Depends on:** none.

**Invariants cited:** Spec §ADR Conventions — Retroactive ADRs #1.

- [ ] **Step 1: Write the ADR**

Create `docs/adr/0001-tauri-2-stack.md` with this exact content:

```markdown
# 0001: Tauri 2 + SvelteKit + SQLite stack

**Status:** accepted
**Date:** 2026-04-19

## Context

vtmtools is a GM-facing desktop tool for Vampire: The Masquerade 5th
Edition. Requirements: offline-capable, native filesystem access (for
Markdown exports and a local SQLite database), modest binary size, single-
user operation, no hosted backend, strong types shared between frontend
and backend, and a small maintenance surface. We evaluated Electron,
Tauri 1, Tauri 2, and a web-only deployment.

## Decision

- Frontend: SvelteKit with the static adapter, running as a single-page
  app (no SSR, no server routes).
- Backend: Rust, compiled into the Tauri 2 shell.
- Database: SQLite via `sqlx`, stored in the Tauri app data directory.
- IPC: Tauri 2 commands + events, with a capability allowlist.

## Consequences

- Binary size is a fraction of an equivalent Electron app (native webview
  on each OS, not a bundled Chromium).
- Serde types in Rust can be mirrored in TypeScript with minimal drift;
  the IPC boundary is strongly typed in both directions.
- Tauri 2's capabilities system gives a default-deny IPC surface —
  adding a new command requires listing it in a capability JSON.
- SvelteKit's SPA mode avoids SSR complexity that is irrelevant for a
  desktop app.
- Trade: smaller plugin ecosystem than Electron, and occasional platform
  webview differences require targeted workarounds.

## Alternatives considered

- **Electron.** Rejected for binary size, resource overhead, and lack of
  fit with a Rust backend.
- **Tauri 1.** Rejected in favor of Tauri 2 for its capabilities system,
  improved plugin architecture, and active direction of the project.
- **Web-only SPA.** Rejected — no filesystem access for Markdown exports
  or SQLite, and no way to host the localhost WebSocket listener the
  Roll20 integration requires (see ADR 0005).
```

- [ ] **Step 2: Verify**

Run: `test -f docs/adr/0001-tauri-2-stack.md && head -4 docs/adr/0001-tauri-2-stack.md`
Expected: file exists; header shows "# 0001: Tauri 2 + SvelteKit + SQLite stack" and "**Status:** accepted".

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0001-tauri-2-stack.md
git commit -m "$(cat <<'EOF'
docs(adr): 0001 Tauri 2 + SvelteKit + SQLite stack

Retroactive ADR capturing the stack decision. Desktop-first, single-user,
native FS access, strong typed IPC — Tauri 2 + SvelteKit (static) + Rust
+ SQLite. See ADR for alternatives considered and consequences.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

### Task 4: Write ADR 0002 — Destructive reseed of non-custom dyscrasias

**Files:**

- Create: `docs/adr/0002-destructive-reseed.md`

**Anti-scope:** Do NOT touch any other ADR file, CLAUDE.md, ARCHITECTURE.md, `docs/reference/`, `docs/superpowers/`, or source code.

**Depends on:** none.

**Invariants cited:** Spec §ADR Conventions — Retroactive ADRs #2.

- [ ] **Step 1: Write the ADR**

Create `docs/adr/0002-destructive-reseed.md` with this exact content:

```markdown
# 0002: Destructive reseed of non-custom dyscrasias on startup

**Status:** accepted
**Date:** 2026-04-19

## Context

Built-in dyscrasias (canonical V5 content) ship baked into the binary via
`src-tauri/src/db/seed.rs`. Users can also create custom dyscrasias
(flagged `is_custom = 1` in the `dyscrasias` table). The canonical seed
changes over time as descriptions are refined or entries are added. The
question: how do we reconcile seed-file changes with the live SQLite DB
on the user's machine without stomping their custom work?

## Decision

On every app start, `seed.rs` deletes all rows where `is_custom = 0` and
reinserts the full canonical set from the seed source. Rows with
`is_custom = 1` are never touched.

## Consequences

- Seed changes land automatically on next launch; users always see the
  current canonical dyscrasia data shipped with the build.
- Any edit a user makes to a built-in entry is discarded on next
  launch. This is intentional: edits should be made by forking the entry
  into a custom copy (new row with `is_custom = 1`), not by mutating the
  built-in.
- No migration is required when the seed changes — the next boot
  reconciles automatically.
- Foreign-key dependencies on built-in dyscrasias are stable because IDs
  are deterministic within a seed version.

## Alternatives considered

- **Count-check guard (seed only when `dyscrasias` is empty).** Rejected:
  stale data after first run; every future seed change becomes invisible
  without manual DB surgery.
- **Per-seed-change migration.** Rejected: high engineering cost, easy to
  forget, brittle across app versions.
- **Merge-by-name with conflict resolution.** Rejected: "same name"
  semantics are ambiguous; makes the system harder to reason about for
  negligible user-facing benefit.
```

- [ ] **Step 2: Verify**

Run: `test -f docs/adr/0002-destructive-reseed.md && head -4 docs/adr/0002-destructive-reseed.md`
Expected: file exists; header shows "# 0002: Destructive reseed of non-custom dyscrasias on startup".

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0002-destructive-reseed.md
git commit -m "$(cat <<'EOF'
docs(adr): 0002 destructive reseed of non-custom dyscrasias

Retroactive ADR capturing why seed.rs deletes is_custom=0 rows on every
start rather than using a count-check guard. Prevents stale built-ins
and keeps canonical data fresh across app updates without migrations.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

### Task 5: Write ADR 0003 — Freeform strings for node and edge types

**Files:**

- Create: `docs/adr/0003-freeform-node-edge-types.md`

**Anti-scope:** Do NOT touch any other ADR file, CLAUDE.md, ARCHITECTURE.md, `docs/reference/`, `docs/superpowers/`, or source code.

**Depends on:** none.

**Invariants cited:** Spec §ADR Conventions — Retroactive ADRs #3.

- [ ] **Step 1: Write the ADR**

Create `docs/adr/0003-freeform-node-edge-types.md` with this exact content:

```markdown
# 0003: Freeform strings for nodes.type and edges.edge_type

**Status:** accepted
**Date:** 2026-04-19

## Context

The Domains Manager stores a chronicle graph of nodes (areas, characters,
institutions, businesses, merits, and anything else a Storyteller wants
to track) and edges (contains, owns, knows, member-of, leads, and so on).
The schema question is whether `nodes.type` and `edges.edge_type` should
be enumerated (CHECK constraint or Rust enum) or left as freeform user-
authored strings.

## Decision

Both fields are freeform `TEXT` columns with no enumeration. The UI
derives autocomplete suggestions from the distinct values already present
in the current chronicle.

The sole exception is the `"contains"` edge type, which the UI uses as
its hierarchy/drilldown convention. A partial unique index
(`ON edges(chronicle_id, to_node_id) WHERE edge_type = 'contains'`)
enforces at most one `contains` parent per node. All other edge types
are unconstrained.

## Consequences

- Users can invent new node and edge types without a code change, a
  migration, or a release.
- The domain grammar of the Storyteller's world is authored by the
  Storyteller, not pre-committed by the tool.
- Typos (`"carachter"` vs `"character"`) can create phantom types —
  mitigated by the autocomplete UI, which surfaces existing types first.
- Pattern is consistent with the project's extensibility preference
  (pluggable over locked-in).

## Alternatives considered

- **Fixed Rust enum + CHECK constraint.** Rejected: every new type
  requires a migration and a code change; undermines the "Storyteller-
  authored world" posture.
- **Enum with an `Other(String)` fallback.** Rejected: adds complexity
  without meaningful benefit. Autocomplete over freeform strings gives
  the ergonomic wins of an enum without the schema rigidity.
- **JSON-schema-validated type taxonomy.** Rejected for v1 scope;
  potentially revisitable later as user-defined schemas per chronicle.
```

- [ ] **Step 2: Verify**

Run: `test -f docs/adr/0003-freeform-node-edge-types.md && head -4 docs/adr/0003-freeform-node-edge-types.md`
Expected: file exists; header shows "# 0003: Freeform strings for nodes.type and edges.edge_type".

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0003-freeform-node-edge-types.md
git commit -m "$(cat <<'EOF'
docs(adr): 0003 freeform strings for node and edge types

Retroactive ADR capturing why the chronicle graph uses freeform TEXT for
nodes.type and edges.edge_type instead of an enum. Supports user-authored
world grammars without migrations; 'contains' gets a partial unique index
for the hierarchy convention.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

### Task 6: Write ADR 0004 — Dark-only theming

**Files:**

- Create: `docs/adr/0004-dark-only-theming.md`

**Anti-scope:** Do NOT touch any other ADR file, CLAUDE.md, ARCHITECTURE.md, `docs/reference/`, `docs/superpowers/`, or source code.

**Depends on:** none.

**Invariants cited:** Spec §ADR Conventions — Retroactive ADRs #4.

- [ ] **Step 1: Write the ADR**

Create `docs/adr/0004-dark-only-theming.md` with this exact content:

```markdown
# 0004: Dark-only theming

**Status:** accepted
**Date:** 2026-04-19

## Context

vtmtools targets Storytellers running V5 chronicles — a domain with a
strong aesthetic lean toward low-light, subdued UI. Supporting both light
and dark themes doubles the design and regression-test surface for every
component. The question is whether to invest in theming or commit to a
single palette.

## Decision

Dark-only. No theme toggle exists or will be added. All colors are CSS
custom properties defined once in `:global(:root)` inside
`src/routes/+layout.svelte`. Components reference tokens
(`--bg-base`, `--text-primary`, `--accent`, etc.) — hardcoded hex is
permitted only for transient states with no semantic equivalent (hover
intermediates, glow shadows).

## Consequences

- One palette to tune; design effort concentrates on a single
  well-calibrated look.
- Components are cheaper to author and review — no dual-theme reasoning.
- Users who prefer light UI are not served. Acceptable given the audience
  and use context (GMs running chronicles, often in dim settings).
- Future reversal would require retrofitting light-mode token variants
  and a runtime theme switch; cost is proportional to the component
  surface at that time.

## Alternatives considered

- **Dual-mode toggle.** Rejected — 2× design/test surface, misaligned
  with product posture.
- **System-followed theme (`prefers-color-scheme`).** Rejected — same
  2× surface cost; saves only the toggle UI.
- **User-configurable palette.** Rejected — scope creep; users can't
  tune four colors well, and "just make it dark" is the stated need.
```

- [ ] **Step 2: Verify**

Run: `test -f docs/adr/0004-dark-only-theming.md && head -4 docs/adr/0004-dark-only-theming.md`
Expected: file exists; header shows "# 0004: Dark-only theming".

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0004-dark-only-theming.md
git commit -m "$(cat <<'EOF'
docs(adr): 0004 dark-only theming

Retroactive ADR capturing why vtmtools ships with a single dark palette
and no theme toggle. Aligns with audience and use context; keeps design/
test surface half the size of a dual-mode app.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

### Task 7: Write ADR 0005 — Roll20 integration via local WS + browser extension

**Files:**

- Create: `docs/adr/0005-roll20-ws-extension-bridge.md`

**Anti-scope:** Do NOT touch any other ADR file, CLAUDE.md, ARCHITECTURE.md, `docs/reference/`, `docs/superpowers/`, or source code.

**Depends on:** none.

**Invariants cited:** Spec §ADR Conventions — Retroactive ADRs #5.

- [ ] **Step 1: Write the ADR**

Create `docs/adr/0005-roll20-ws-extension-bridge.md` with this exact content:

```markdown
# 0005: Roll20 integration via localhost WebSocket + browser extension

**Status:** accepted
**Date:** 2026-04-19

## Context

Several tools (Campaign viewer, Resonance Roll20 write-back, future
features) depend on live Roll20 character data — health, willpower,
hunger, humanity, blood potency, and attribute write-back for rolls.
We evaluated ingestion paths: Roll20's public API, direct character-sheet
export, a relay server, and a localhost bridge fed by a browser
extension.

## Decision

A Chrome extension (`extension/`) injects a content script into Roll20
pages. The extension reads Roll20's DOM for character state (which the
logged-in user is already authorized to see) and opens a WebSocket
connection to `127.0.0.1:7423`, a listener owned by the Tauri app
(`src-tauri/src/roll20/`). The Tauri app accepts one connection at a
time, maintains the character cache in `Arc<Roll20State>`, and emits
`roll20://connected`, `roll20://disconnected`, and
`roll20://characters-updated` events to the Svelte frontend.

## Consequences

- Data is live ground-truth — whatever the user sees on the sheet is
  what the app sees.
- No Roll20 API key, rate limits, or API dependency.
- No cloud relay; nothing leaves the user's machine.
- Security posture: localhost-only listener; any process running as the
  user can connect. This is equivalent to trusting the user and is the
  stated security posture for a single-user desktop tool (see
  ARCHITECTURE.md §8).
- Roll20 DOM changes can break attribute resolution. Mitigated by
  `docs/reference/roll20-bundle.md`, which documents the current DOM
  paths and Jumpgate sheet attribute names.
- Single-session only — one extension WS connection at a time.
  Sufficient for single-user use; revisit if multi-seat scenarios arise.

## Alternatives considered

- **Roll20 public API.** Rejected — not designed for live sheet state,
  rate-limited, and requires an API key per user.
- **Cloud relay / hosted backend.** Rejected — contradicts the project's
  "no cloud, no server" posture (see ARCHITECTURE.md §12 Non-goals) and
  adds an external dependency.
- **Manual character-sheet export.** Rejected — stale, manual, and
  incompatible with the write-back direction (attribute updates from
  vtmtools to Roll20).
```

- [ ] **Step 2: Verify**

Run: `test -f docs/adr/0005-roll20-ws-extension-bridge.md && head -4 docs/adr/0005-roll20-ws-extension-bridge.md`
Expected: file exists; header shows "# 0005: Roll20 integration via localhost WebSocket + browser extension".

- [ ] **Step 3: Commit**

```bash
git add docs/adr/0005-roll20-ws-extension-bridge.md
git commit -m "$(cat <<'EOF'
docs(adr): 0005 Roll20 integration via localhost WS + browser extension

Retroactive ADR capturing why vtmtools uses a browser extension + local
WebSocket listener on 127.0.0.1:7423 for Roll20 integration rather than
the Roll20 API or a cloud relay. Live data, no API dependency, localhost-
only security surface.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

## Phase B — ARCHITECTURE.md

### Task 8: Write ARCHITECTURE.md

**Files:**

- Create: `ARCHITECTURE.md` (at repo root).

**Anti-scope:** Do NOT touch CLAUDE.md, any ADR file, any `docs/reference/` file, any `docs/superpowers/` file, or any source code.

**Depends on:** Tasks 1–7 (renames in place; ADRs exist so §13 index entries resolve).

**Invariants cited:** Spec §ARCHITECTURE.md Structure (sections 1–13).

- [ ] **Step 1: Gather current-state inputs**

Read the following files to ground the content in current reality. Do not proceed to Step 2 before reading them.

- `src-tauri/src/shared/types.rs` — for §2 Domain model type definitions.
- `src-tauri/src/shared/dice.rs` — for §2 (dice/resonance types surface).
- `src-tauri/src/shared/resonance.rs` — for §2 (resonance surface).
- `src-tauri/src/lib.rs` — for §4 (command registration, capability wiring).
- `src-tauri/capabilities/*.json` — for §8 (Tauri ACL reference).
- `src-tauri/migrations/0001_initial.sql` — for §3 schema.
- `src-tauri/migrations/0002_chronicle_graph.sql` — for §3 schema.
- `src/tools.ts` — for §9 (tools registry shape).
- `docs/superpowers/specs/2026-04-19-docs-reorg-design.md` — the authoritative spec for this plan.

Enumerate Tauri commands via:

```bash
grep -rn "^#\[tauri::command\]" src-tauri/src --include='*.rs'
```

Expected: 32 matches grouped as 5 in `roll20/commands.rs`, 5 in `db/chronicle.rs`, 10 in `db/node.rs`, 5 in `db/edge.rs`, 5 in `db/dyscrasia.rs`, 1 in `tools/resonance.rs`, 1 in `tools/export.rs`.

- [ ] **Step 2: Write ARCHITECTURE.md in full**

Create `ARCHITECTURE.md` at the repo root. Follow the structure below exactly. Where the instruction says "inline <source>", copy the relevant definitions from the file verbatim under a fenced code block and then add one short prose line per item.

```markdown
# ARCHITECTURE.md

> Canonical cross-feature reference for vtmtools. Load this file for any
> code change. Feature-specific details live in `docs/superpowers/specs/`
> and `docs/superpowers/plans/`; historical decisions live in
> `docs/adr/`; stable reference knowledge lives in `docs/reference/`.

---

## §1 Overview & stack

[2–4 sentences. State: desktop-first single-GM tool for VTM 5e;
Tauri 2 + SvelteKit (static SPA) + Rust backend + SQLite; offline-
capable; dark-only; no cloud, no multi-user. Cite ADR 0001 for the
stack decision and ADR 0004 for the theming posture.]

## §2 Domain model

The canonical data shapes. Features that produce or consume any of
these must honor the shape defined here.

### Dyscrasia domain

[Inline the `DyscrasiaEntry` Rust struct (from
`src-tauri/src/shared/types.rs`) and the `dyscrasias` CREATE TABLE
statement from `src-tauri/migrations/0001_initial.sql` verbatim under
fenced code blocks. One short prose line after each describing its
role.]

### Dice / resonance domain

[Inline `Temperament`, `ResonanceType`, `SliderLevel`,
`TemperamentConfig`, `ResonanceWeights`, `RollConfig`,
`ResonanceRollResult` from `shared/types.rs`. Prose: each is shared
between the Rust roller and the frontend display.]

### Chronicle graph domain

[Inline `Chronicle`, `StringFieldValue`, `NumberFieldValue`,
`FieldValue`, `Field`, `Node`, `Edge`, `EdgeDirection` from
`shared/types.rs`. Inline the SQL schema for `chronicles`, `nodes`,
`edges` from `migrations/0002_chronicle_graph.sql`. Call out:
`nodes.type` and `edges.edge_type` are freeform strings (ADR 0003).]

### Roll20 domain

[Inline the `Character` struct (or equivalent) from the Roll20
module. Describe the character cache payload shape that flows over
the `roll20://characters-updated` event.]

### Mirror layer

The frontend `src/types.ts` mirrors these shapes in TypeScript.
Drift is not tolerated — changing a Rust struct requires updating
the TS mirror in the same commit.

## §3 Storage strategy

- **SQLite** at `app.path().app_data_dir()`. All durable application
  state lives here: `dyscrasias`, `chronicles`, `nodes`, `edges`.
- **Migrations** in `src-tauri/migrations/NNNN_*.sql`, applied on
  every startup via `sqlx::migrate!`. `PRAGMA foreign_keys = ON` is
  enabled on the pool via `SqliteConnectOptions` so `ON DELETE
  CASCADE` actually fires.
- **Seed policy:** `src-tauri/src/db/seed.rs` deletes all
  `is_custom = 0` rows in `dyscrasias` and reinserts the canonical
  set on every start. Intentional; see ADR 0002.
- **Ephemeral state:**
  - Svelte runes stores in `src/store/*` (e.g.
    `domains.svelte.ts` for chronicle UI state, `toolEvents.ts` for
    cross-tool pub/sub).
  - `Arc<Roll20State>` on the Rust side holds the Roll20 character
    cache and WebSocket connection state.
- **Export artifacts:** Markdown written to `~/Documents/vtmtools/`
  via `tokio::fs` in `src-tauri/src/tools/export.rs`.
- **No cloud, no remote sync, no network storage** (see §12 Non-goals).

## §4 I/O contracts

Named surfaces at module boundaries. Feature tasks that cross any of
these must honor the contract shapes declared here and must not bypass
the wrapper layers described below.

### Tauri IPC commands

Inventoried by module. A command's request/response shape (from its
Rust signature + the types it references in §2) is the stable contract.

- **`src-tauri/src/db/chronicle.rs`** (5):
  `list_chronicles`, `get_chronicle`, `create_chronicle`,
  `update_chronicle`, `delete_chronicle`.
- **`src-tauri/src/db/node.rs`** (10):
  `list_nodes`, `get_node`, `create_node`, `update_node`,
  `delete_node`, `get_parent_of`, `get_children_of`,
  `get_siblings_of`, `get_path_to_root`, `get_subtree`.
- **`src-tauri/src/db/edge.rs`** (5):
  `list_edges`, `list_edges_for_node`, `create_edge`,
  `update_edge`, `delete_edge`.
- **`src-tauri/src/db/dyscrasia.rs`** (5):
  `list_dyscrasias`, `add_dyscrasia`, `update_dyscrasia`,
  `delete_dyscrasia`, `roll_random_dyscrasia`.
- **`src-tauri/src/tools/resonance.rs`** (1): `roll_resonance`.
- **`src-tauri/src/tools/export.rs`** (1): `export_result_to_md`.
- **`src-tauri/src/roll20/commands.rs`** (5):
  `get_roll20_characters`, `get_roll20_status`,
  `refresh_roll20_data`, `send_roll20_chat`,
  `set_roll20_attribute`.

Total: 32 commands. New commands are registered in
`src-tauri/src/lib.rs` and listed in `src-tauri/capabilities/*.json`
(see §8).

### Typed frontend API wrapper modules

Frontend components never call `invoke(...)` directly. IPC goes
through typed wrapper modules in `src/lib/**/api.ts` (see
`src/lib/domains/api.ts` for the reference implementation: one
exported function per Tauri command, return type matching the Rust
response). New tools adopt the same pattern.

### Roll20 WebSocket protocol

- Binding: `127.0.0.1:7423`, localhost-only (see §8 Security model,
  ADR 0005).
- At most one active extension session. Connection owner is
  `src-tauri/src/roll20/mod.rs`.
- Message shapes: [inline the in-use message variants from
  `src-tauri/src/roll20/types.rs` / `mod.rs`, or summarize in 3–5
  lines if the protocol is documented elsewhere].

### Tauri events (backend → frontend)

| Event | Payload | Emitted when |
|---|---|---|
| `roll20://connected` | none | Extension opens WS connection |
| `roll20://disconnected` | none | Extension closes WS connection |
| `roll20://characters-updated` | `Vec<Character>` | Character cache refreshed |

### Svelte cross-tool pub/sub

`src/store/toolEvents.ts` exposes `publishEvent(event)` and a
`toolEvents` writable store. Subscribers are loose — document event
names + payload shapes near the publisher.

### Tools registry

`src/tools.ts` is THE add-a-tool seam. Adding an entry auto-wires the
sidebar and lazy-loaded component. See §9 Extensibility seams.

## §5 Module boundaries

Forbidding rules. Violations are merge blockers.

- Only `src-tauri/src/db/*` talks to SQLite. No component or other
  backend module invokes `sqlx` or opens a connection.
- Only `src-tauri/src/roll20/*` talks to the WebSocket server. No
  other backend module binds a socket.
- Frontend components never import SQL drivers or call the database
  directly. Database access is exclusively via Tauri `invoke(...)`
  calls, and those go through the typed API wrappers in
  `src/lib/**/api.ts`.
- All Tauri commands that do I/O are `async` and use `tokio::fs`,
  never `std::fs`.
- Frontend tools (`src/tools/*.svelte`) never import another tool
  directly. Cross-tool coordination goes through `toolEvents` or
  shared stores (e.g. `src/store/domains.svelte.ts` for chronicle-
  aware tools).

## §6 Invariants

Properties that must hold across all features.

- `PRAGMA foreign_keys = ON` is enabled on the pool via
  `SqliteConnectOptions`. `ON DELETE CASCADE` depends on this.
- `resonance_type` uses the spelling `'Melancholy'` (not
  `'Melancholic'`). The DB CHECK constraint enforces this.
- Seed reconciliation on app start is destructive for
  `is_custom = 0` rows. Do not revert to a count-check guard
  (ADR 0002).
- CSS colors use tokens from `:global(:root)` in
  `src/routes/+layout.svelte`. Hex is allowed only for transient
  states with no semantic token (hover intermediates, glow shadows).
  Token groups (as of this writing):
  - Text: `--text-primary`, `--text-label`, `--text-secondary`,
    `--text-muted`, `--text-ghost`.
  - Surfaces: `--bg-base`, `--bg-card`, `--bg-raised`, `--bg-input`,
    `--bg-sunken`, `--bg-active`.
  - Borders: `--border-faint`, `--border-card`, `--border-surface`,
    `--border-active`.
  - Accents: `--accent`, `--accent-bright`, `--accent-amber`.
  - Temperament: `--temp-negligible`, `--temp-negligible-dim`,
    `--temp-fleeting-dim`, `--temp-intense-dim`.
- Layout and typography sizes are in `rem`. Root font-size is
  `clamp(16px, 1.0vw, 32px)`; `rem` scales automatically. Never use
  `px` for font sizes or layout widths.
- Card grids use CSS Grid with `align-items: start`. Never use CSS
  multi-column — incompatible with `animate:flip`.
- Any element combining `width: 100%` with `padding` must set
  `box-sizing: border-box` (there is no global reset).
- In Svelte 5 runes mode, `in:` / `out:` transitions are placed on
  elements whose lifecycle is controlled by the enclosing `{#each}`
  or `{#if}`, not on runes-mode component roots. Use a plain wrapper
  `<div in:scale out:fade>` in the parent's `{#each}` block.
- Only one Roll20 extension session is active at a time. State is
  held in `Arc<Roll20State>` shared between the WS loop and the
  Tauri command handlers.
- Dark-only. No theme toggle exists or will be added (ADR 0004).

## §7 Error handling

How failures propagate across the Rust ↔ Tauri IPC ↔ Svelte boundary.

- Rust commands return `Result<T, E>`. At the Tauri IPC boundary,
  the error type is serialized as `String` (Tauri rejects the
  frontend promise with that string). Prefix stable per-command
  identifiers where useful (e.g. `"db/dyscrasia.create: …"`) so the
  frontend can categorize without parsing free-form prose.
- Frontend catches rejected `invoke` promises in the typed API
  wrapper (`src/lib/**/api.ts`) or at the call site, and surfaces
  user-visible errors via toast / inline error state. Raw errors
  are logged to the console.
- Panics in command paths are bugs, not error flow. No `unwrap()`
  in production code; use `?` or explicit error mapping.
- WebSocket disconnect is expected flow, not an error. The
  `roll20://disconnected` event fires, the UI shifts to "not
  connected" state, and the next extension reconnect restores
  service.
- Database errors from `sqlx` propagate as `Err(String)` with
  module-stable prefixes. Migration failures on startup are fatal
  (the app exits with a user-visible error).

## §8 Security model

Trust boundaries and assumptions for a single-user local desktop tool.

- **Trust posture.** Single user, single machine. No authentication,
  no authorization, no user-level access control. These are non-
  goals (§12).
- **Network surface.** Exactly one listener: the Roll20 WebSocket
  on `127.0.0.1:7423`. It must never bind to `0.0.0.0` or any
  routable interface. No other external network call is made by
  the app.
- **Localhost WS trust model.** Any process running as the user can
  connect to `127.0.0.1:7423`. This is equivalent to trusting the
  user and is the intended posture. Do not add authentication to
  the WS without a specific threat that justifies it.
- **Tauri capabilities / ACL.** `src-tauri/capabilities/*.json` is
  the effective allowlist for which Tauri commands the frontend
  may call. Adding a new command requires listing it here;
  otherwise `invoke` fails closed.
- **Filesystem write scope.** Writes are limited to
  `app.path().app_data_dir()` (for SQLite) and
  `~/Documents/vtmtools/` (for exports). Any new write path must
  be added to this list and justified.
- **Browser extension DOM-read surface.** The extension reads
  Roll20 DOM nodes documented in `docs/reference/roll20-bundle.md`.
  It never sends data outside the localhost WebSocket.
- **Secrets.** None. No API keys, tokens, or credentials in the
  app. If a future feature introduces one, this section is updated
  and an ADR is filed.

## §9 Extensibility seams

Named places to add things. Feature specs cite a seam instead of
inventing a new hook.

- **Add a tool.** Add one entry to `src/tools.ts`. Sidebar +
  lazy-loaded component wiring is automatic. Existing examples:
  `Resonance.svelte`, `DyscrasiaManager.svelte`,
  `DomainsManager.svelte` (three examples — the pattern is stable).
- **Add a schema change.** Add a new
  `src-tauri/migrations/NNNN_*.sql` file; migrations run on app
  start. Mirror the shape change in `shared/types.rs` and
  `src/types.ts` in the same commit.
- **Add a node or edge type.** No code change. The chronicle graph
  uses freeform strings (ADR 0003); the UI derives autocomplete
  from existing distinct values.
- **Add a Tauri command.** Declare in the relevant
  `src-tauri/src/**/commands.rs` (or module-level `commands.rs`
  equivalent), register in `src-tauri/src/lib.rs`, list in the
  capability JSON (§8), then add a typed wrapper in
  `src/lib/**/api.ts`. Components call the wrapper, never
  `invoke(...)` directly.
- **Add a cross-tool event.** Publish via `src/store/toolEvents.ts`.
  Document the event name + payload shape near the publisher.
- **Add a property field type.** Extend the `FieldValue` enum in
  `src-tauri/src/shared/types.rs`, mirror it in `src/types.ts`,
  and register a widget in the Domains Manager property-editor
  registry. Existing variants: `string`, `text`, `number`,
  `date`, `url`, `email`, `bool`, `reference`. v1 UI widgets
  ship for `string`, `text`, `number`, `bool`; the other variants
  are extensibility seams whose widgets will follow.

## §10 Testing & verification

- Rust unit tests live as `#[cfg(test)] mod tests` inside each
  source file. Current test modules: `shared/dice.rs`,
  `shared/resonance.rs`, `db/dyscrasia.rs`, `db/chronicle.rs`,
  `db/node.rs`, `db/edge.rs`, `tools/export.rs`. (Run the grep
  `grep -rn "#\[cfg(test)\]" src-tauri/src` to confirm current
  state before editing.)
- No frontend test framework is installed. This is a deliberate
  current choice, not an oversight. Introducing one is a scope
  change to be raised explicitly.
- `./scripts/verify.sh` is the aggregate gate: runs `npm run
  check`, `cargo test`, and `npm run build`. All claims of "done"
  must be backed by a green run.
- Expected (non-regression) warnings — do NOT "fix" these:
  - `npm run build`: unused `listen` import in
    `src/tools/Campaign.svelte` and `src/tools/Resonance.svelte`.
    In-progress surface, not dead code.
  - `shared/types.rs`: `FieldValue` variants `Date`, `Url`,
    `Email`, and `Reference` may surface "never constructed" —
    the v1 Domains UI uses only `String`, `Text`, `Number`, and
    `Bool` widgets. The unused variants ship as extensibility
    seams for future property widgets (see §9). Do not remove.

## §11 Plan & execution conventions

Plans produced against this architecture are structured for parallel
sub-agent-driven execution (`superpowers:subagent-driven-development`,
`superpowers:dispatching-parallel-agents`).

Each plan task declares:

- **Files (create / modify / delete):** tight, explicit scope.
- **Anti-scope:** files the task MUST NOT touch. Prevents silent
  collisions when two sub-agents run concurrently.
- **Depends on:** predecessor task IDs, or `none` if independent.
- **Invariants cited:** pointers to specific ARCHITECTURE.md
  sections the task must honor.

Seams between parallel tasks are drawn along §4 I/O contracts. If
two tasks share a contract, the contract shape is settled in a
preliminary task before either implementation task starts; both
parallel tasks then work behind the frozen contract.

Verification gate: every sub-agent runs `./scripts/verify.sh`
before reporting success. Green output is required; self-reports
without verification are not accepted.

Isolation: prefer `superpowers:using-git-worktrees` for multi-agent
dispatch so concurrent edits don't collide in a shared working tree.

## §12 Non-goals

Explicitly out of scope. A feature spec that proposes any of these
must first raise a scope change; do not assume it's allowed.

- No multi-user or multi-tenant operation.
- No cloud sync, remote storage, or server-backed state.
- No light-mode, theme toggle, or configurable color scheme (ADR 0004).
- No authentication or authorization of any kind.
- No network surface beyond the single `127.0.0.1:7423` WebSocket listener.
- No ingestion path for Roll20 data other than the browser extension
  bridge (ADR 0005).
- No multi-session Roll20 support. One extension session at a time.
- No frontend testing framework.

## §13 ADR index

| # | Title | Status |
|---|---|---|
| 0001 | [Tauri 2 + SvelteKit + SQLite stack](docs/adr/0001-tauri-2-stack.md) | accepted |
| 0002 | [Destructive reseed of non-custom dyscrasias on startup](docs/adr/0002-destructive-reseed.md) | accepted |
| 0003 | [Freeform strings for nodes.type and edges.edge_type](docs/adr/0003-freeform-node-edge-types.md) | accepted |
| 0004 | [Dark-only theming](docs/adr/0004-dark-only-theming.md) | accepted |
| 0005 | [Roll20 integration via localhost WebSocket + browser extension](docs/adr/0005-roll20-ws-extension-bridge.md) | accepted |

Add new rows here as ADRs are written. When an ADR is superseded,
update its Status column to `superseded by NNNN`.
```

- [ ] **Step 3: Verify ARCHITECTURE.md**

Run:

```bash
grep -c "^## §" ARCHITECTURE.md
```

Expected: `13`.

Run:

```bash
grep -n "TBD\|TODO\|FIXME\|XXX" ARCHITECTURE.md || echo "no placeholders"
```

Expected: "no placeholders" (no lingering TBD / TODO markers).

Run:

```bash
for adr in docs/adr/000[1-5]-*.md; do
  label=$(basename "$adr" .md)
  grep -q "$adr" ARCHITECTURE.md && echo "ok: $label linked" || echo "MISSING LINK: $label"
done
```

Expected: 5 "ok:" lines, no "MISSING LINK".

- [ ] **Step 4: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
docs: add ARCHITECTURE.md — canonical cross-feature reference

Introduces the stable architecture doc: domain model, storage,
I/O contracts, module boundaries, invariants, error handling,
security model, extensibility seams, testing/verification,
plan/execution conventions, non-goals, and ADR index.

Follows the spec in docs/superpowers/specs/2026-04-19-docs-reorg-design.md.
Downstream work (slim CLAUDE.md, cross-ref sweep, final verify)
follows in subsequent commits.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

## Phase C — Slim CLAUDE.md

### Task 9: Rewrite CLAUDE.md with read-scope guide and slim content

**Files:**

- Modify: `CLAUDE.md`

**Anti-scope:** Do NOT touch ARCHITECTURE.md, any ADR file, any `docs/reference/` file, any `docs/superpowers/` file, or any source code.

**Depends on:** Task 8 (pointers to ARCHITECTURE.md sections need ARCHITECTURE.md to exist).

**Invariants cited:** Spec §CLAUDE.md — New Shape.

- [ ] **Step 1: Read current CLAUDE.md**

Read the full current `CLAUDE.md` so the rewrite preserves the daily-operational content (Commands block, Claude-specific rules) while removing architectural prose that now lives in ARCHITECTURE.md.

- [ ] **Step 2: Rewrite CLAUDE.md**

Replace the entire contents of `CLAUDE.md` with the following. Do not merge or append — this is a full replacement:

```markdown
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
```

- [ ] **Step 3: Verify length and content**

Run:

```bash
wc -l CLAUDE.md
```

Expected: ≤100 lines.

Run:

```bash
grep -q "^## Read-scope guide" CLAUDE.md && echo "read-scope: ok"
grep -q "^## Commands" CLAUDE.md && echo "commands: ok"
grep -q "^## Claude-specific rules" CLAUDE.md && echo "rules: ok"
grep -q "ARCHITECTURE.md" CLAUDE.md && echo "architecture pointer: ok"
grep -q "docs/design" CLAUDE.md && echo "STALE PATH FOUND" || echo "no stale paths"
```

Expected: four "ok" lines and "no stale paths".

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "$(cat <<'EOF'
docs: slim CLAUDE.md; architecture prose moved to ARCHITECTURE.md

Rewrite CLAUDE.md as a thin Claude-facing operational doc: read-scope
guide (which files to load when), commands, Claude-specific rules, and
pointers. Architectural detail now lives in ARCHITECTURE.md; historical
decisions in docs/adr/; topic-gated reference in docs/reference/.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

Expected: commit created.

---

## Phase D — Cross-reference sweep

### Task 10: Purge `docs/design/` references from non-archive files

**Files:**

- Modify: any non-archive file that still mentions `docs/design/`, `roll20-vtt-bundle-analysis.md`, or `data-sources-kumu.json` with a dash. Expected candidates: none, but possibly `docs/reference/data-sources.md`, `docs/reference/data-sources.kumu.json`, or minor mentions in source files.

**Anti-scope:** Do NOT modify any file under `docs/superpowers/specs/`, `docs/superpowers/plans/`, or `docs/adr/`. These archives are frozen per spec §Scope — Out of Scope.

**Depends on:** Task 1 (rename must be in place) and Task 9 (CLAUDE.md already rewritten with new paths; Task 9 should have zero stale refs, but confirm here).

**Invariants cited:** Spec §Acceptance Criteria #6.

- [ ] **Step 1: Find remaining references**

Run:

```bash
git grep -nE "docs/design|roll20-vtt-bundle-analysis|data-sources-kumu\.json" \
  -- \
  ':!docs/superpowers/specs/' \
  ':!docs/superpowers/plans/' \
  ':!docs/adr/'
```

Record each match. If there are zero matches, skip to Step 3.

- [ ] **Step 2: Fix each match in place**

For each match found in Step 1, use `Edit` to replace:

- `docs/design/` → `docs/reference/`
- `roll20-vtt-bundle-analysis.md` → `roll20-bundle.md`
- `data-sources-kumu.json` → `data-sources.kumu.json`

Do not rewrite file structure; only substitute the paths. If a match is
in a code comment (source file), replace only the comment; do not touch
surrounding logic.

Do NOT edit files in `docs/superpowers/specs/`,
`docs/superpowers/plans/`, or `docs/adr/` even if they contain
matches — the archive rule in the spec overrides.

- [ ] **Step 3: Re-run the sweep to confirm zero matches**

Run:

```bash
git grep -nE "docs/design|roll20-vtt-bundle-analysis|data-sources-kumu\.json" \
  -- \
  ':!docs/superpowers/specs/' \
  ':!docs/superpowers/plans/' \
  ':!docs/adr/'
```

Expected: no output (exit code 1).

- [ ] **Step 4: Commit (only if Step 2 produced changes)**

If Step 2 modified any files, commit:

```bash
git add -A
git commit -m "$(cat <<'EOF'
docs: sweep stale docs/design paths to docs/reference

Updates cross-references outside the frozen archives
(docs/superpowers/specs, docs/superpowers/plans, docs/adr) to the
renamed docs/reference/ layout. Renames of inner files applied:
roll20-vtt-bundle-analysis.md -> roll20-bundle.md,
data-sources-kumu.json -> data-sources.kumu.json.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

If Step 2 produced no changes, record "no sweep commit needed" in your
task report and move on — the rename was already clean.

---

## Phase E — Verification

### Task 11: Final verification pass

**Files:**

- Read: all relevant files to confirm acceptance criteria.
- Create or modify: none, unless a check fails.

**Anti-scope:** No file changes unless a failure surfaces. If any check fails, stop and report — do not "fix by hand" without coordination.

**Depends on:** Tasks 1–10 (all preceding work must be committed).

**Invariants cited:** Spec §Acceptance Criteria (#1–#7).

- [ ] **Step 1: ARCHITECTURE.md exists with 13 sections and no placeholders** (AC #1)

```bash
test -f ARCHITECTURE.md && echo "AC1a: file exists"
test $(grep -c "^## §" ARCHITECTURE.md) -eq 13 && echo "AC1b: 13 sections"
grep -nE "TBD|TODO|FIXME" ARCHITECTURE.md && echo "AC1c: PLACEHOLDERS FOUND" || echo "AC1c: no placeholders"
```

Expected: three "AC1*" success lines, no "PLACEHOLDERS FOUND".

- [ ] **Step 2: CLAUDE.md ≤ 100 lines with read-scope guide at top** (AC #2)

```bash
test $(wc -l < CLAUDE.md) -le 100 && echo "AC2a: line count ok" || echo "AC2a: TOO LONG"
head -20 CLAUDE.md | grep -q "## Read-scope guide" && echo "AC2b: read-scope early" || echo "AC2b: read-scope buried"
```

Expected: both "ok / early" lines.

- [ ] **Step 3: `docs/reference/` contains renamed files, `docs/design/` is gone** (AC #3)

```bash
test -f docs/reference/data-sources.md && echo "AC3a"
test -f docs/reference/data-sources.kumu.json && echo "AC3b"
test -f docs/reference/roll20-bundle.md && echo "AC3c"
test -f docs/reference/v5-combat-rules.md && echo "AC3d"
test ! -d docs/design && echo "AC3e: old dir gone" || echo "AC3e: DOCS/DESIGN STILL EXISTS"
```

Expected: 5 success lines (AC3a–AC3e).

- [ ] **Step 4: `docs/adr/` contains template + 5 retroactive ADRs** (AC #4)

```bash
test -f docs/adr/template.md && echo "AC4t"
for n in 1 2 3 4 5; do
  f=$(ls docs/adr/000${n}-*.md 2>/dev/null)
  test -n "$f" && echo "AC4-000${n}: $f" || echo "AC4-000${n}: MISSING"
done
```

Expected: 6 success lines.

- [ ] **Step 5: `./scripts/verify.sh` is green** (AC #5)

Run: `./scripts/verify.sh`
Expected: exit code 0, no cargo test failures, no Svelte type errors, frontend build succeeds. Expected warnings from ARCHITECTURE.md §10 are acceptable; actual regressions are not.

If `verify.sh` fails: record the failure precisely, do not "fix" source code in this task — this is a docs-only reorg, and any verify regression is caused by a broken path somewhere (most likely in Task 10's sweep). Re-run Task 10's Step 1 and inspect.

- [ ] **Step 6: Grep check — no stale `docs/design` outside archives** (AC #6)

```bash
git grep -nE "docs/design|roll20-vtt-bundle-analysis|data-sources-kumu\.json" \
  -- \
  ':!docs/superpowers/specs/' \
  ':!docs/superpowers/plans/' \
  ':!docs/adr/' \
  && echo "AC6: STALE REFS FOUND" \
  || echo "AC6: clean"
```

Expected: "AC6: clean".

- [ ] **Step 7: ARCHITECTURE.md §11 describes plan/execution conventions** (AC #7)

```bash
awk '/^## §11 /{found=1; next} found && /^## §/{exit} found{print}' ARCHITECTURE.md \
  | grep -qE "subagent-driven-development|dispatching-parallel-agents|Files \(create|Anti-scope|Depends on|Invariants cited" \
  && echo "AC7: plan conventions present" || echo "AC7: MISSING"
```

Expected: "AC7: plan conventions present".

- [ ] **Step 8: Final commit marker (only if any earlier step produced changes)**

This plan intentionally does not add a marker commit if Steps 1–7 all pass cleanly. The reorg is already captured by the commits from Tasks 1–10. Record a final status report as part of the task output: which ACs passed, any follow-up items surfaced during verification.

If Steps 1–7 surfaced a correctable failure and Task 10 was re-run, commit those fixes with a clear message; then re-run Steps 1–7.

---

## Notes for subagent-driven execution

- **Phase A parallelism.** Tasks 1–7 have no file overlap. Dispatch all 7 in parallel worktrees via `superpowers:using-git-worktrees` for maximum throughput. Each task produces one commit.
- **Phase B/C/D/E are sequential.** Each blocks on the previous phase — do not dispatch in parallel.
- **Task 8 (ARCHITECTURE.md) has inline extraction instructions.** The sub-agent must read the named source files (Step 1 of Task 8) and copy type definitions verbatim into the relevant sections. Do not paraphrase type shapes.
- **Archives are frozen.** No task in this plan modifies files under `docs/superpowers/specs/`, `docs/superpowers/plans/`, or `docs/adr/` (except the ADR writer tasks that create their own file). The sweep in Task 10 explicitly excludes these paths.
- **Verification gate.** Task 11 is the merge gate. A green Task 11 is required before the reorg branch is considered complete.
