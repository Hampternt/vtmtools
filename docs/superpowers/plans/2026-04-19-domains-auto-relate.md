# Domains Auto-Relate on Child Creation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `NodeForm.svelte` so that creating a child node can optionally write a second, user-typed, directional edge to the parent in the same save, with a session-sticky preference.

**Architecture:** Approach X from brainstorming — pure frontend. One new `$state` export in the domains store (session-only, in-memory). One file extended with imports, local state, one derived, a new template section, save-logic changes, and scoped styles. No backend, schema, migration, or new Tauri command.

**Tech Stack:** Svelte 5 runes, SvelteKit, Tauri 2 (unchanged), TypeScript. Existing `api.createEdge` wrapper is sufficient.

**Spec:** `docs/superpowers/specs/2026-04-19-domains-auto-relate-design.md`

---

## File structure (decomposition)

- **`src/store/domains.svelte.ts`** — add one new `$state`-wrapped export (`autoRelatePref`) alongside the existing `session`/`cache`/`status`. Responsibility: session-only memory for the auto-relate UI's checkbox/type/direction.
- **`src/lib/components/domains/NodeForm.svelte`** — import the new preference; add form-local `autoRelateDescription` state; add `knownEdgeTypes` derived (mirrors `EdgePicker`); add a new template section gated on `node == null && parentId != null`; extend `save()` with empty-type validation, a `containsOk` gate, the auto-relate `createEdge` calls, and an `onsave` gate; add scoped styles (using `:root` tokens — no hex).

All other domain files (`DomainTree.svelte`, `EdgesPanel.svelte`, `EdgePicker.svelte`, `NodeDetail.svelte`, `PropertyEditor.svelte`, `api.ts`, backend) are **unchanged**.

Task dependencies preclude parallel sub-agent execution for this plan: Task 2 imports the symbol added in Task 1; Task 2's template and `save()` logic and styles all mutate the same file and interact too tightly to split. Plan is therefore strictly serial — Task 0 → 1 → 2 → 3.

---

## Task 0: Pre-flight — verify a clean starting point

**Files:** none (read-only preflight).

**Anti-scope:** No code changes.

**Depends on:** none.

**Invariants cited:**
- CLAUDE.md: "Never run `git status -uall`." (use plain `git status`).

- [ ] **Step 1: Check working tree state.**

  Run:
  ```bash
  git status --short
  ```

  Expected at time of plan writing (per the session's snapshot): two unrelated uncommitted files —
  ```
   M src/lib/components/domains/DomainTree.svelte
   M src/lib/components/domains/NodeForm.svelte
  ```
  These changes are **not** part of this plan (they are prior in-flight work: `SvelteSet` reactivity in the tree; `$state.snapshot`/flex-layout tweaks in the form). Commit them separately or stash them before starting Task 1 so this feature's commits are clean.

- [ ] **Step 2: Isolate pre-existing changes.**

  Option A — commit as-is (recommended if the changes are ready):
  ```bash
  git add src/lib/components/domains/DomainTree.svelte src/lib/components/domains/NodeForm.svelte
  git commit -m "chore(domains): SvelteSet for tree expansion + snapshot/flex form tweaks"
  ```

  Option B — stash until this plan is done:
  ```bash
  git stash push -m "pre-auto-relate NodeForm/DomainTree tweaks" -- \
    src/lib/components/domains/DomainTree.svelte \
    src/lib/components/domains/NodeForm.svelte
  ```

  After either option, re-run `git status --short` and confirm the working tree is clean before continuing.

- [ ] **Step 3: Confirm the spec and plan files exist.**

  Run:
  ```bash
  ls docs/superpowers/specs/2026-04-19-domains-auto-relate-design.md docs/superpowers/plans/2026-04-19-domains-auto-relate.md
  ```
  Expected: both files listed.

---

## Task 1: Add `autoRelatePref` to the domains store

**Files:**
- Modify: `src/store/domains.svelte.ts` (append near bottom, after existing exports).

**Anti-scope:** Do NOT modify `NodeForm.svelte`, any other component, or `api.ts`. Do NOT add persistence (localStorage, SQLite, settings file, Tauri store plugin). Do NOT add `description` to the export — description is intentionally form-local per the spec's "Resolved during review" section.

**Depends on:** Task 0.

**Invariants cited:**
- ARCHITECTURE.md §3: Svelte runes stores live in `src/store/*`; module-top-level `$state` requires the wrapper-object pattern (property mutation, not reassignment).
- ARCHITECTURE.md §4: no new Tauri IPC here.
- Spec §Architecture / §State module: exact export shape frozen as below.

- [ ] **Step 1: Read the current file.**

  Run the `Read` tool on `src/store/domains.svelte.ts` to confirm current structure — expected exports in order: `session`, `cache`, `status`, plus async helpers. The new export goes at the bottom, before any helper functions if present, or after the last export — follow the file's existing export ordering convention.

- [ ] **Step 2: Append the new export.**

  Add the following block at the appropriate insertion point (end of the module, after the last existing `export` but before any trailing helpers if they exist as a separate section):

  ```ts
  // Session-sticky preference for the NodeForm "Auto-relate to parent" UI.
  // In-memory only — resets on app restart. Description is deliberately NOT
  // persisted here (it's per-edge content, kept as form-local state in NodeForm).
  export const autoRelatePref = $state<{
    enabled: boolean;
    edgeType: string;
    direction: 'parent-to-child' | 'child-to-parent' | 'both';
  }>({
    enabled: false,
    edgeType: '',
    direction: 'child-to-parent',
  });
  ```

- [ ] **Step 3: Run the aggregate verification gate.**

  Run:
  ```bash
  ./scripts/verify.sh
  ```

  Expected: green. The only warnings allowed are the pre-existing ones listed in ARCHITECTURE.md §10 (unused `listen` imports in `Campaign.svelte` / `Resonance.svelte`; never-constructed `FieldValue` variants `Date`/`Url`/`Email`/`Reference`). If any new warning or error surfaces, fix inline before committing.

- [ ] **Step 4: Commit.**

  Run:
  ```bash
  git add src/store/domains.svelte.ts
  git commit -m "$(cat <<'EOF'
  feat(domains): add autoRelatePref session-sticky store

  Will be consumed by NodeForm's new auto-relate UI. In-memory only; no
  persistence, no schema. Description is intentionally NOT part of this
  state (form-local per spec).

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

  Expected: one commit added, one file changed.

---

## Task 2: Extend `NodeForm.svelte` with the auto-relate UI and save logic

**Files:**
- Modify: `src/lib/components/domains/NodeForm.svelte` (script block, template block, and `<style>` block).

**Anti-scope:** Do NOT modify any other component (`DomainTree.svelte`, `EdgesPanel.svelte`, `EdgePicker.svelte`, `NodeDetail.svelte`, `PropertyEditor.svelte`), any backend file, `api.ts`, or `domains.svelte.ts` (that's Task 1). Do NOT add a frontend test framework. Do NOT use hex colors — use `:root` tokens per ARCHITECTURE.md §6. Do NOT change the file's existing `properties`/`$state.snapshot` line or the existing `.add-prop` flex styles (those were addressed by Task 0's separate commit and are out of this feature's scope).

**Depends on:** Task 1 (imports `autoRelatePref`).

**Invariants cited:**
- ARCHITECTURE.md §4: components go through `src/lib/**/api.ts` wrappers — never call `invoke()` directly. This task uses the existing `api.createEdge` only.
- ARCHITECTURE.md §6: `:root` color tokens only; no hex in components.
- ARCHITECTURE.md §7: inline-error partial-success posture — `localError` is the visible surface; raw IPC errors pass through `friendlyError`.
- Spec §Behavior step 7: `onsave(saved)` must be gated on `!localError` so partial-failure messages stay visible until the user closes the form via Cancel.
- Spec §Architecture / §Component: template structure, state field names, derived shape, and save-logic sequencing are frozen exactly as captured below.

- [ ] **Step 1: Update the store import line.**

  Find this line near the top of the `<script>` block:
  ```ts
  import { session, cache, refreshNodes, refreshEdges, selectNode } from '../../../store/domains.svelte';
  ```

  Replace with:
  ```ts
  import { session, cache, refreshNodes, refreshEdges, selectNode, autoRelatePref } from '../../../store/domains.svelte';
  ```

- [ ] **Step 2: Add form-local `autoRelateDescription` state.**

  Find this block (existing local state near top of `<script>`):
  ```ts
    let saving = $state(false);
    let localError = $state('');
  ```

  Immediately after it, add:
  ```ts
    let autoRelateDescription = $state('');
  ```

  This state is intentionally form-local: `NodeForm` mounts per "+ Add child" click, so this naturally resets to `''` each time the form opens. No sticky memory for descriptions.

- [ ] **Step 3: Add the `knownEdgeTypes` derived.**

  Find the existing `knownTypes` derived:
  ```ts
    const knownTypes = $derived(
      Array.from(new Set(cache.nodes.map(n => n.type))).sort()
    );
  ```

  Immediately after it, add:
  ```ts
    const knownEdgeTypes = $derived(
      Array.from(new Set(
        cache.edges.map(e => e.edge_type).filter(t => t !== 'contains')
      )).sort()
    );
  ```

  Identical shape to `EdgePicker.svelte`'s derivation — deliberate duplication (3 lines) rather than extracting a shared helper for 2 callers. Revisit if a third caller emerges.

- [ ] **Step 4: Replace the `save()` function.**

  Find the existing `async function save() { … }` (currently approx. lines 71–109 in the pre-task file). Replace the entire function body with:

  ```ts
    async function save() {
      if (!label.trim()) { localError = 'Label is required.'; return; }
      if (!nodeType.trim()) { localError = 'Type is required.'; return; }
      if (session.chronicleId == null) { localError = 'No chronicle selected.'; return; }
      if (node == null && parentId != null && autoRelatePref.enabled && !autoRelatePref.edgeType.trim()) {
        localError = 'Relationship type is required when auto-relate is on.';
        return;
      }

      const tags = tagsText.split(',').map(t => t.trim()).filter(Boolean);

      saving = true;
      localError = '';
      try {
        let saved: ChronicleNode;
        if (node) {
          saved = await api.updateNode(
            node.id, nodeType.trim(), label.trim(), description, tags, properties,
          );
        } else {
          saved = await api.createNode(
            session.chronicleId, nodeType.trim(), label.trim(), description, tags, properties,
          );
          if (parentId != null) {
            let containsOk = true;
            try {
              await api.createEdge(
                session.chronicleId, parentId, saved.id, 'contains', '', [],
              );
            } catch (e) {
              containsOk = false;
              localError = `Node created, but linking to parent failed: ${e}`;
            }

            if (containsOk && autoRelatePref.enabled && autoRelatePref.edgeType.trim()) {
              const type = autoRelatePref.edgeType.trim();
              const desc = autoRelateDescription;
              try {
                if (autoRelatePref.direction !== 'child-to-parent') {
                  await api.createEdge(session.chronicleId, parentId, saved.id, type, desc, []);
                }
                if (autoRelatePref.direction !== 'parent-to-child') {
                  await api.createEdge(session.chronicleId, saved.id, parentId, type, desc, []);
                }
              } catch (e) {
                localError = `Node created, but auto-relation failed: ${friendlyError(String(e))}`;
              }
            }
          }
        }
        await refreshNodes();
        await refreshEdges();
        selectNode(saved.id);
        if (!localError) onsave(saved);
      } catch (e) {
        localError = friendlyError(String(e));
      } finally {
        saving = false;
      }
    }
  ```

  Changes vs. previous `save()`:
  - **New validation** (lines 4–6): empty `edgeType` when auto-relate is enabled.
  - **New `containsOk` flag**: set to `false` on contains-edge failure to gate the auto-relate block.
  - **New auto-relate block**: two conditional `createEdge` calls using the `!==` guard pattern (both directions fire when direction is `'both'`; each single value fires exactly one).
  - **New `onsave` gate**: `if (!localError) onsave(saved);` — form stays open on any partial-failure so `localError` stays visible. Also retroactively fixes the pre-existing contains-auto flash-and-disappear behavior.

- [ ] **Step 5: Add the auto-relate template section.**

  Find the properties block in the template:
  ```svelte
    <div class="props">
      <div class="props-label">Properties</div>
      {#each properties as p, i (p.name)}
        <PropertyEditor
          field={p}
          readonly={false}
          onchange={(updated) => updateProperty(i, updated)}
          onremove={() => removeProperty(i)}
        />
      {/each}

      <div class="add-prop">
        <input class="small" bind:value={newPropName} placeholder="new property name" />
        <select class="small" bind:value={newPropType}>
          {#each SUPPORTED_TYPES as t (t)}
            <option value={t}>{t}</option>
          {/each}
        </select>
        <button class="btn" onclick={addProperty}>+ Add property</button>
      </div>
    </div>
  ```

  Immediately after the closing `</div>` of `.props` and before the `{#if localError}` block, insert:

  ```svelte
    {#if node == null && parentId != null}
      <div class="auto-relate">
        <label class="ar-checkbox">
          <input type="checkbox" bind:checked={autoRelatePref.enabled} />
          Also add a relationship to parent
        </label>

        {#if autoRelatePref.enabled}
          <div class="field">
            <label for="nf-ar-type">Relationship type</label>
            <input
              id="nf-ar-type"
              list="nf-ar-types"
              bind:value={autoRelatePref.edgeType}
              placeholder="member-of, located-in, allied-with…"
            />
            <datalist id="nf-ar-types">
              {#each knownEdgeTypes as t (t)}
                <option value={t}></option>
              {/each}
            </datalist>
          </div>

          <fieldset class="ar-direction">
            <legend>Direction</legend>
            <label class="ar-radio">
              <input type="radio" bind:group={autoRelatePref.direction} value="parent-to-child" />
              parent → new
            </label>
            <label class="ar-radio">
              <input type="radio" bind:group={autoRelatePref.direction} value="child-to-parent" />
              new → parent
            </label>
            <label class="ar-radio">
              <input type="radio" bind:group={autoRelatePref.direction} value="both" />
              both
            </label>
          </fieldset>

          <div class="field">
            <label for="nf-ar-desc">Description (optional)</label>
            <input id="nf-ar-desc" bind:value={autoRelateDescription} />
          </div>
        {/if}
      </div>
    {/if}
  ```

  Placement rationale: sits between the properties grid and the error/actions row, same vertical position as where an extra `field` section would naturally go. Rendered only for new-child creation; absent on edit mode and on root-add.

- [ ] **Step 6: Add scoped styles for the new section.**

  Find the existing `<style>` block in the file. Append the following rules before the closing `</style>` (insertion order in a `<style>` block doesn't affect specificity here — they're all scoped to this component):

  ```css
    .auto-relate {
      border-top: 1px solid var(--border-faint);
      padding-top: 0.45rem;
      display: flex;
      flex-direction: column;
      gap: 0.4rem;
    }
    .ar-checkbox {
      display: flex;
      align-items: center;
      gap: 0.4rem;
      font-size: 0.72rem;
      color: var(--text-secondary);
      cursor: pointer;
      text-transform: none;
      letter-spacing: normal;
    }
    .ar-checkbox input { margin: 0; }
    .ar-direction {
      border: 1px solid var(--border-surface);
      border-radius: 4px;
      padding: 0.3rem 0.55rem;
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      background: var(--bg-input);
      margin: 0;
    }
    .ar-direction legend {
      font-size: 0.55rem;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--text-muted);
      padding: 0 0.3rem;
    }
    .ar-radio {
      display: flex;
      align-items: center;
      gap: 0.3rem;
      font-size: 0.72rem;
      color: var(--text-secondary);
      cursor: pointer;
      text-transform: none;
      letter-spacing: normal;
    }
    .ar-radio input { margin: 0; }
  ```

  Notes:
  - All colors come from `:root` tokens (`--border-faint`, `--border-surface`, `--text-secondary`, `--text-muted`, `--bg-input`) — no hex, per ARCHITECTURE.md §6.
  - The `text-transform: none; letter-spacing: normal;` on `.ar-checkbox` and `.ar-radio` is an explicit override of the component-level `label { … }` rule (which applies uppercase/letter-spaced styling to field-row labels). Without the override, "Also add a relationship to parent" would render uppercased. This is intentional — checkbox/radio labels are sentences, not field titles.
  - `.ar-direction` uses `var(--bg-input)` to visually signal a grouped-input region, matching the input backgrounds in the rest of the form.

- [ ] **Step 7: Run the aggregate verification gate.**

  Run:
  ```bash
  ./scripts/verify.sh
  ```

  Expected: green. Allowed pre-existing warnings only (ARCHITECTURE.md §10). If any new TypeScript or Svelte error surfaces, fix inline before committing.

- [ ] **Step 8: Commit.**

  Run:
  ```bash
  git add src/lib/components/domains/NodeForm.svelte
  git commit -m "$(cat <<'EOF'
  feat(domains): auto-relate checkbox on child creation

  Add optional second edge (non-contains) at child-creation time, with a
  checkbox + free-text type + direction (parent→child | child→parent | both)
  + optional description. Session-sticky for enable/type/direction via
  autoRelatePref; description stays form-local.

  Also gates onsave(saved) on !localError so partial-failure errors stay
  visible instead of flashing-and-disappearing on form unmount — fixes the
  pre-existing contains-auto message-disappearance too.

  Spec: docs/superpowers/specs/2026-04-19-domains-auto-relate-design.md

  Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
  EOF
  )"
  ```

  Expected: one commit, one file changed.

---

## Task 3: Manual smoke test and final verification

**Files:** none (validation-only).

**Anti-scope:** No code changes in this task. If any test step fails, return to Task 2, fix inline, and rerun `./scripts/verify.sh` + this task's smoke walkthrough.

**Depends on:** Task 1 and Task 2.

**Invariants cited:**
- CLAUDE.md: "Never commit without running `./scripts/verify.sh` first." (already done in Tasks 1 & 2; re-run after any fix.)
- ARCHITECTURE.md §10: frontend correctness is verified manually since there is no frontend test framework. This is a deliberate project convention, not an oversight.

- [ ] **Step 1: Run the aggregate verification gate once more for cleanliness.**

  Run:
  ```bash
  ./scripts/verify.sh
  ```

  Expected: green, no new warnings beyond the documented baseline.

- [ ] **Step 2: Start the dev server.**

  Run:
  ```bash
  npm run tauri dev
  ```

  Expected: desktop window opens on the vtmtools app. Wait for initial compile to finish before interacting.

- [ ] **Step 3: Execute the 10-step smoke walkthrough from the spec.**

  Follow the "Testing Strategy / Manual smoke test" list in `docs/superpowers/specs/2026-04-19-domains-auto-relate-design.md`. Reproducing verbatim for convenience:

  1. Open Domains Manager tool. If no chronicle exists, create one. Add a root node labeled **"Anarch Coterie"**.
  2. Click "+ Add child" on Anarch Coterie. Confirm the new **"Auto-relate to parent"** section appears between the Properties grid and the Cancel/Save row. Confirm the checkbox is **unchecked** on first open.
  3. Tick the checkbox. The type input, direction radio, and description input appear. Type `member-of` into the type field. Leave direction at **new → parent** (the default). Leave description empty. Fill in the required node fields (label: `Nines Rodriguez`, type: `character`), then click Save.
  4. Verify that the new child "Nines Rodriguez" is selected. Its EdgesPanel shows exactly **one outgoing** edge labeled `member-of` pointing to Anarch Coterie.
  5. Click back onto Anarch Coterie in the tree. Its EdgesPanel shows exactly **one incoming** edge labeled `member-of` pointing from Nines Rodriguez.
  6. Click "+ Add child" on Anarch Coterie again. Confirm: checkbox is **checked**, type is pre-filled as `member-of`, direction is still **new → parent**, and the description input is **empty** (description is NOT sticky).
  7. Change direction to **both**, change type to `"allied-with"`, leave description empty, create a second child labeled `Jack Nine-Stakes` (type: `character`). Verify: Jack's EdgesPanel has one outgoing `allied-with` AND one incoming `allied-with` (two edges, opposite directions). Anarch Coterie's EdgesPanel also shows the mirror — one outgoing `allied-with` and one incoming `allied-with` pointing to/from Jack.
  8. Click "+ Add root node" at the bottom of the tree. Confirm: the auto-relate section does **NOT** render (no parent exists for a root).
  9. Select any existing node and click "✎ Edit" in the detail pane. Confirm: the auto-relate section does **NOT** render in edit mode.
  10. Click "+ Add child" on a parent, check the auto-relate box, leave type **empty**, fill in a label/type, click Save. Confirm: inline error "Relationship type is required when auto-relate is on." appears and **no node is created** (the tree should not gain a new row).

- [ ] **Step 4: Session-reset check.**

  Kill the dev server (Ctrl-C in the terminal running `npm run tauri dev`). Re-run `npm run tauri dev`. Open the Domains tool, click "+ Add child" on any parent. Confirm: the auto-relate checkbox is **unchecked again** — session memory was cleared on restart as designed.

- [ ] **Step 5: Partial-failure visibility check (optional but recommended).**

  To exercise the `onsave`-gate behavior manually: click "+ Add child", enable auto-relate with type `member-of`, pick direction **both**, and save. The first edge writes. Repeat "+ Add child" with the *same* target parent and the *same* settings — if direction=both happens to collide with an existing row, inline error appears; confirm the form stays open (does NOT unmount) so the error stays readable until the user clicks Cancel.

  If no collision arises naturally, skip this step — the gate is also covered by the non-collision paths passing through the same code path.

- [ ] **Step 6: Report done.**

  All ten smoke steps + session-reset check pass → the feature is complete. If anything failed, return to Task 2, fix inline, rerun `./scripts/verify.sh`, and resume Step 3.

---

## Self-review notes (done before handoff)

**Spec coverage scan.** Every spec section is covered by a concrete task step:
- Spec §Scope / In scope → Tasks 1 & 2 (store export + every template/save/styles element).
- Spec §Behavior (first open, subsequent opens, app-restart reset, save sequence) → Task 2 Steps 2–5 + Task 3 Steps 3–4.
- Spec §Architecture (state, component, data flow) → Tasks 1 & 2 with exact code.
- Spec §Error handling → Task 2 Step 4's save() rewrite (containsOk gate, onsave gate, friendlyError reuse).
- Spec §Testing → Task 3 Steps 1–5.
- Spec §File list matches Tasks 1 & 2 Files blocks (no extras, no omissions).
- Spec §Resolved during review → all three corrections (containsOk, form-local description, explicit validation) are present in Task 2 Step 4.

**Placeholder scan.** No "TBD", "TODO", "similar to Task N", "fill in details" strings anywhere in the plan. All code steps carry complete code blocks; all verification steps carry exact commands with expected outputs.

**Type consistency.** Field names `enabled`, `edgeType`, `direction` match between Task 1 (declaration) and Task 2 (consumption in template bindings and `save()` guards). Direction string values `'parent-to-child'`, `'child-to-parent'`, `'both'` are identical across state shape, radio values, and save-logic guards. Local state name `autoRelateDescription` is consistent across declaration (Step 2), binding (Step 5 template), and usage (Step 4 save).
