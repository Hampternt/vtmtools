# Phase 1 Execution Notes (Playbook for Fresh Session)

> **Read this first if resuming Phase 1 plan execution in a new session.**
>
> Three tasks of Plan 0 are already in master; the rest of Phase 1 is durable and can be picked up cleanly. This file captures four pacing optimizations derived from executing Tasks 1–3+4 the slow way, so the rest doesn't repeat the same overhead.

---

## §1 Status snapshot

**Already in master** (verified `./scripts/verify.sh` green at HEAD):

| Commit | Tasks | Notes |
|---|---|---|
| `a8cc4ff` + `a2c0a40` | Plan 0 Task 1 (`SourceInfo` struct + fix-up) | Doc comment correctness + `PartialEq` derive |
| `e82fe3d` | Plan 0 Task 2 (`BridgeState.source_info` field + disconnect cleanup) | |
| `0f88a93` + `9003861` | Plan 0 Tasks 3+4 combined (typed pre-trait Hello/Error dispatch) | Combined to keep master green; fix-up adopted typed deserialization |

**Remaining**:
- Plan 0 Tasks 5–15 (11 tasks)
- Plan 1 Tasks 1–12 (12 tasks)
- Plan 2 Tasks 1–7 (7 tasks)
- Plan 3 Tasks 1–12 (12 tasks)

Total: **42 tasks**.

Plans live at:
- `docs/superpowers/plans/2026-04-30-bridge-protocol-consolidation.md`
- `docs/superpowers/plans/2026-04-30-saved-characters.md`
- `docs/superpowers/plans/2026-04-30-compare-modal.md`
- `docs/superpowers/plans/2026-04-30-v5-dice-helpers.md`

Spec: `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md`.

---

## §2 The four pacing optimizations

### Opt 1 — Drop spec compliance review on mechanical tasks

The plan files contain verbatim code blocks for almost every task. Spec compliance review against verbatim-paste tasks found nothing across Tasks 1, 2, and 3+4. Keep it only for tasks where the plan is interpretive:

**Tasks deserving spec compliance review:**
- Plan 0 Task 12 (actor.js refactor — interpretive about hook timing)
- Plan 0 Task 13 (bridge.js extended Hello — interpretive about manual smoke verification)
- Plan 1 Task 11 (Campaign.svelte UI integration)
- Plan 2 Task 6 (Campaign.svelte Compare button wiring)
- Plan 3 Task 7 (`format_skill_check` — message format is presentational)
- All final verification gates (Plan 0 Task 15, etc.)

**Skip spec review on:** every other task.

### Opt 2 — Cluster trivial tasks into single dispatches

Trivial tasks across the plans cluster naturally. Each cluster lands as ONE implementer dispatch with one commit:

**Plan 0 clusters:**
- **Cluster A:** Tasks 5 + 6 — `bridge/foundry/actions/bridge.rs` (new file with subscribe/unsubscribe builders + tests) + `actions/mod.rs` 1-line export. Anti-scope: nothing else.
- **Cluster B:** Tasks 7 + 8 — `bridge_get_source_info` Tauri command in `bridge/commands.rs` + register in `lib.rs`. Anti-scope: nothing else; do NOT touch frontend yet.
- **Cluster C:** Tasks 9 + 10 — `bridgeGetSourceInfo` typed wrapper in `src/lib/bridge/api.ts` + extend `bridge.svelte.ts` with `sourceInfo` reactive state. Anti-scope: do NOT touch any tool component.
- **Cluster D:** Tasks 11 + 14 — `vtmtools-bridge/scripts/foundry-actions/bridge.js` (subscribers registry) + 1-line registration in `foundry-actions/index.js`. Anti-scope: do NOT touch `bridge.js` or `actor.js` yet (Cluster E owns those).
- **Cluster E (sequential, must be combined):** Tasks 12 + 13 — `actor.js` refactor to `actorsSubscriber.attach/detach` + `bridge.js` extended Hello payload + error envelope on handler exceptions. Combined because Task 12 alone breaks the always-send-actors behavior until Task 13 wires `bridgeUmbrella.handleSubscribe({collection: 'actors'})` in the open handler. Combined commit keeps master green.
- **Standalone:** Task 15 — `module.json` bump + final verification gate.

**Plan 1 clusters:**
- **Cluster F:** Tasks 1 + 2 — migration `0004_saved_characters.sql` + `db/saved_character.rs` skeleton + module declaration in `db/mod.rs`. Anti-scope: no commands yet.
- **Cluster G:** Tasks 3 + 4 + 5 + 6 — all four CRUD commands (`save`, `list`, `update`, `delete`) with their tests, in one commit. Plan calls them independent; in practice they share `db_*` helper conventions and one fixture, so one combined dispatch is cleaner than four. Anti-scope: do NOT register in `lib.rs` yet.
- **Cluster H:** Tasks 7 + 8 — register all 4 commands in `lib.rs` + create typed wrappers in `src/lib/saved-characters/api.ts`. Anti-scope: no frontend store/components yet.
- **Standalone:** Task 9 (runes store), Task 10 (`SourceAttributionChip`), Task 11 (Campaign integration — substantive, needs full review), Task 12 (final gate).

**Plan 2 clusters:**
- **Cluster I:** Tasks 1 + 2 + 3 + 4 — all of `diff.ts` (CANONICAL_PATHS + FOUNDRY_PATHS + diffSpecialties + diffCharacter) in one commit. Plan splits these for clarity, but they form one cohesive file. Anti-scope: no UI changes.
- **Standalone:** Task 5 (`CompareModal.svelte`), Task 6 (Campaign integration — substantive), Task 7 (final gate).

**Plan 3 clusters:**
- **Standalone:** Task 1 (scaffold) — already mostly mechanical; one dispatch.
- **Standalone:** Task 2 (types).
- **Cluster J:** Tasks 3 + 4 — `pool.rs` and `dice.rs` together (independent leaves; combining halves dispatch cost).
- **Cluster K:** Tasks 5 + 6 — `interpret.rs` and `difficulty.rs` together.
- **Standalone:** Task 7 (`message.rs` — presentational, needs spec review).
- **Standalone:** Task 8 (orchestrator — small, needs careful review).
- **Cluster L:** Tasks 9 + 10 + 11 — `tools/skill_check.rs` Tauri command + register in `lib.rs` + add `tools/mod.rs` declaration + `src/lib/v5/api.ts`. All small, all in one commit.
- **Standalone:** Task 12 (final gate).

**Net dispatches saved:** ~8 trivial-task dispatches across Phase 1.

### Opt 3 — Skip re-review when the fix is verbatim reviewer-suggested code

If a fix:
1. Adopts code the reviewer literally wrote in their review, AND
2. `./scripts/verify.sh` is green after the fix

…then re-review is process for process's sake. Mark the task complete after the fix lands. Keep re-review when the fix is interpretive (the implementer made design choices the reviewer didn't dictate).

In Tasks 1 and 3+4 fix-ups, the re-reviews approved verbatim-fix code with no changes. Skipping ~2 re-reviews per fix saves real time.

### Opt 4 — Run Plan 3 in a parallel worktree

Plan 3 (V5 dice helpers) has **zero file overlap** with Plans 0/1/2 — touches only `src-tauri/src/shared/v5/`, `src-tauri/src/tools/skill_check.rs`, `src-tauri/src/tools/mod.rs`, `src-tauri/src/lib.rs` (one new line), and `src/lib/v5/api.ts`.

The skill warning "never dispatch multiple implementers in parallel" is about *within-plan* conflicts. Truly disjoint plans in separate worktrees don't conflict.

**Setup commands** (run from main repo root `/home/hampter/projects/vtmtools/`):

```bash
git worktree add ../vtmtools-v5 -b plan-3-v5-helpers
cd ../vtmtools-v5
./scripts/verify.sh   # confirm green baseline
```

Then dispatch Plan 3 implementer subagents pointing at `/home/hampter/projects/vtmtools-v5/` while the main session continues Plan 0 in `/home/hampter/projects/vtmtools/`.

When Plan 3 finishes:

```bash
cd /home/hampter/projects/vtmtools
git merge plan-3-v5-helpers          # fast-forward or 3-way; lib.rs is the only potential conflict
git worktree remove ../vtmtools-v5
git branch -d plan-3-v5-helpers
```

If `lib.rs`'s `invoke_handler!` macro has a conflict, resolve by appending the Plan 3 line (`tools::skill_check::roll_skill_check`) to the existing list — that's the only intended overlap point.

**Net throughput:** Plan 3's ~12 tasks run concurrently with Plan 0's remainder (~11 tasks). The serial path was ~23 sequential task-cycles; the parallel path is ~12 (the longer of the two).

### Opt 5 — Inline-execute the most trivial tasks

The advisor noted: for tasks where the implementer would be copy-pasting verbatim code blocks anyway, inline editing in the controller session is faster than dispatching a subagent. Reserve subagent dispatch for tasks involving genuine implementation work.

**Inline-execute candidates:** Cluster D (1-line registration), Cluster B's task 8 (1-line registration), Cluster H's task 7 (4 lines in `lib.rs`), Plan 2 Cluster I (the diff projection — the plan has the full code).

**Subagent-dispatch candidates:** Plan 0 Cluster E (actor.js + bridge.js refactor — high regression risk), Plan 1 Task 11 (Campaign UI integration), Plan 2 Task 6 (Campaign Compare wiring), Plan 3 Tasks 5/7/8 (V5 mechanics — correctness-sensitive, want fresh-context implementer).

Roughly 8–10 substantive tasks deserve full subagent treatment; the rest can be inline edits with `cargo check` + `verify.sh` + commit, ~30–60 seconds each.

---

## §3 Cadence summary (post-optimization)

| Task class | Workflow | Approx wall time |
|---|---|---|
| Mechanical / verbatim-paste | Inline edit + verify.sh + commit | 1–2 min |
| Mechanical cluster (2–4 tasks combined) | Single subagent dispatch + code-quality review (no spec review) | 5–8 min |
| Substantive (UI / refactor / V5 mechanics) | Full cycle: implementer + spec review + code review + fix iterations | 15–30 min |
| Final verification gates | Inline `./scripts/verify.sh` + manual smoke + commit | 5–10 min |

**Estimated remaining work:** ~25–35 substantive units after clustering, vs. 42 tasks at the current per-task cadence. Roughly 4–6 hours of focused execution.

---

## §4 Pre-emptive plan-text fixes to apply when reading the plans

Each plan's "Self-review checklist" claims "no placeholders" — accurate. But the broken-then-fixed commit pattern in **Plan 0 Task 3** violates CLAUDE.md's hard rule. The same failure mode could exist in Plan 0 Tasks 12 → 13 (handled in this playbook by Cluster E) and in any plan that relies on a "structural change first, consumers next" sequence.

When executing, watch for: any task whose Step 2 says "Expected: compile error" or "broken — fixed in next task." That's a flag to combine with the next task.

---

## §5 First moves in the next session

1. `cd /home/hampter/projects/vtmtools && git status` — confirm clean tree at HEAD.
2. `./scripts/verify.sh` — confirm green baseline (should match — current HEAD is `9003861`).
3. Read `docs/superpowers/plans/2026-04-30-bridge-protocol-consolidation.md` once to refresh on Cluster A (Tasks 5+6).
4. Read this playbook (§2 — Cluster A entry) for the exact dispatch shape.
5. Set up Plan 3 worktree (Opt 4 commands above).
6. Dispatch Cluster A implementer (Plan 0 Tasks 5+6, single combined commit).
7. Dispatch Plan 3 Task 1 implementer in the parallel worktree.
8. From there, follow the cluster list in §2 sequentially for the main worktree, and continue Plan 3 sequentially in the parallel worktree.

---

## §6 What this conversation produced

Three commits in master, all passing `./scripts/verify.sh`:

```
9003861 fix(bridge): typed pre-trait Hello/Error dispatch + dedupe doc comment (Plan 0 task 3+4 review fix)
0f88a93 feat(bridge/foundry): extend FoundryInbound (Hello fields + Error variant) + capture Hello metadata + route Error envelope (Plan 0 tasks 3+4)
e82fe3d feat(bridge): extend BridgeState with source_info per source (Plan 0 task 2)
a2c0a40 fix(bridge): correct SourceInfo doc comment + add PartialEq derive (Plan 0 task 1 review fix)
a8cc4ff feat(bridge): add SourceInfo struct for per-source metadata (Plan 0 task 1)
```

The protocol-types-and-pre-trait-dispatch foundation is in place. Plan 0 Tasks 5–15 build on top of it without further breaking changes.
