# Stat editor UI (#7) — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship issue [#7](https://github.com/Hampternt/vtmtools/issues/7) — inline ±1 stat-editor controls on live Foundry character cards for 7 of 8 canonical fields. Pure frontend; composes the already-shipped `character_set_field` router (#6).

**Architecture:** A single parameterized Svelte 5 `{#snippet}` (`stepper`) renders a `−`/`+` pair against any canonical field. A `tweakField` handler clamps to per-field ranges and dispatches `characterSetField('live', ...)`. A `busyKey` reactive scalar disables the active button while the IPC round-trip is in flight (`aria-busy="true"`). A `liveEditAllowed(char)` guard disables and tooltips the buttons on Roll20 live cards. The existing visualizations — hunger drops, conscience track, health/willpower boxes — gain adjacent stepper instances. **Offline-saved-only editing is anti-scope** (see §Anti-scope; the saved-card render path doesn't have stat panels yet — that's Phase 2.5 territory).

**Tech Stack:** Svelte 5 (runes mode), TypeScript.

**Spec:** `docs/superpowers/specs/2026-05-02-phase-2-character-editing-design.md` (covers #7 and #8; this plan is the #7 half)

**Hard rules** (from `CLAUDE.md`):
- Every task ending in a commit MUST run `./scripts/verify.sh` first and produce a green result.
- Frontend components NEVER call `invoke(...)` directly — go through `src/lib/character/api.ts` (already shipped by #6).
- Never run `git status -uall` (memory issues on large trees).
- Never hardcode hex colors — use tokens from `:root` in `src/routes/+layout.svelte`.

---

## Fresh-session bootstrap

If you are picking this plan up in a new session, here's everything you need:

**Read first (in order):**
1. `CLAUDE.md` — auto-loaded; defines verify gate + frontend-wrapper rule.
2. **This plan** — has full code for every task. Spec is referenced but not strictly required for execution.
3. (Optional) `docs/superpowers/specs/2026-05-02-phase-2-character-editing-design.md` §2.1, §2.4, §2.9, §4.4 — adds rationale (why 7/8 fields, why wait-for-bridge UX, etc.).

**Recommended dispatch shape (subagent-driven):** all tasks here are sequential — each task modifies the same file (`Campaign.svelte`) and the next task layers atop the previous one. Single implementer, one task at a time, `verify.sh` after each. No parallelism.

**Suggested first message in the new session:**

> "Execute the plan at `docs/superpowers/plans/2026-05-02-stat-editor-ui.md` using `superpowers:subagent-driven-development`. Sequential — Task 0 → 1 → 2 → 3 → 4 → 5 → 6. Final commit footer: `Closes #7`."

---

## File map

| Action | Path | Purpose | Task |
|---|---|---|---|
| Modify | `src/tools/Campaign.svelte` | Add stepper snippet, helpers, per-track wiring, Roll20 guard, CSS | 1–6 |

Total: 1 file modified. No Rust changes, no IPC, no schema, no migrations, no new wire variants. Tauri command surface unchanged (`character_set_field` was registered by #6).

---

### Task 0: Pre-flight green build

Verifies the workspace is green before starting; surfaces any unrelated issues before they get attributed to this work.

**Files:** none

- [ ] **Step 1: Run the aggregate gate.**

```bash
./scripts/verify.sh
```

Expected: green. If it fails, stop and resolve before starting Task 1.

---

### Task 1: Add helpers + stepper snippet + hunger wiring

The smallest end-to-end vertical slice: add the typed helpers, the parameterized `stepper` snippet, the CSS, and wire them up to the hunger drops as the first consumer. Future tasks instantiate the same snippet against other fields.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Add the typed import.**

In the script block, after the existing imports (around line 16, after the `foundryEffectIsActive` import), add:

```ts
  import { characterSetField } from '$lib/character/api';
  import type { CanonicalFieldName } from '$lib/character/api';
```

- [ ] **Step 2: Add helpers and reactive busy state.**

After the `expandedFeats` declaration (around line 60) and before the `urlCopied` declaration, insert:

```ts
  // ── Stat editor (#7) ────────────────────────────────────────────────────

  /// Per-field clamp ranges. Mirrors src-tauri/src/shared/canonical_fields.rs
  /// expect_u8_in_range() bounds; keep the two in sync.
  const FIELD_RANGES: Record<CanonicalFieldName, [number, number]> = {
    hunger:                [0, 5],
    humanity:              [0, 10],
    humanity_stains:       [0, 10],
    blood_potency:         [0, 10],
    health_superficial:    [0, 20],
    health_aggravated:     [0, 20],
    willpower_superficial: [0, 20],
    willpower_aggravated:  [0, 20],
  };

  /// True when the live card supports inline ±1 editing. Roll20 live editing
  /// of canonical names is deferred to Phase 2.5 (router spec §2.8).
  function liveEditAllowed(char: BridgeCharacter): boolean {
    return char.source === 'foundry';
  }

  /// Identity for a per-field stepper: card key plus field name. Used to
  /// scope the busy-disabled state to one button at a time.
  function stepperKey(char: BridgeCharacter, field: CanonicalFieldName): string {
    return `${char.source}:${char.source_id}:${field}`;
  }

  /// Which stepper is currently mid-IPC. Null when idle.
  let busyKey = $state<string | null>(null);

  async function tweakField(
    char: BridgeCharacter,
    field: CanonicalFieldName,
    delta: number,
    current: number,
  ) {
    const range = FIELD_RANGES[field];
    const next  = Math.max(range[0], Math.min(range[1], current + delta));
    if (next === current) return;
    const key = stepperKey(char, field);
    busyKey = key;
    try {
      await characterSetField('live', char.source, char.source_id, field, next);
    } catch (e) {
      console.error('[Campaign] characterSetField failed:', e);
      window.alert(String(e));
    } finally {
      if (busyKey === key) busyKey = null;
    }
  }
```

- [ ] **Step 3: Add the `stepper` snippet definition.**

Snippets in Svelte 5 must live inside the markup, not the script block. Add this snippet block immediately after the closing `</script>` tag (before the opening `<div class="campaign">`):

```svelte
{#snippet stepper(char: BridgeCharacter, field: CanonicalFieldName, current: number)}
  {@const allowed   = liveEditAllowed(char)}
  {@const key       = stepperKey(char, field)}
  {@const busy      = busyKey === key}
  {@const range     = FIELD_RANGES[field]}
  {@const atFloor   = current <= range[0]}
  {@const atCeiling = current >= range[1]}
  {@const tooltip   = allowed
    ? ''
    : 'Roll20 live editing not supported (Phase 2.5)'}
  <span class="stat-stepper" class:roll20-blocked={!allowed}>
    <button
      type="button"
      class="step-btn"
      onclick={() => tweakField(char, field, -1, current)}
      disabled={!allowed || busy || atFloor}
      aria-busy={busy}
      title={tooltip}
      aria-label={`Decrease ${field}`}
    >−</button>
    <button
      type="button"
      class="step-btn"
      onclick={() => tweakField(char, field, +1, current)}
      disabled={!allowed || busy || atCeiling}
      aria-busy={busy}
      title={tooltip}
      aria-label={`Increase ${field}`}
    >+</button>
  </span>
{/snippet}
```

- [ ] **Step 4: Wire the hunger drops with the stepper.**

Find the `<div class="hunger-drops">` block (around line 349) inside the `header-vitals` div:

```svelte
                <div class="hunger-drops">
                  {#each dots(hunger, 5) as filled}
                    <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
                      <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
                    </svg>
                  {/each}
                </div>
```

Replace it with this version that wraps the drops in a flex row plus the stepper:

```svelte
                <div class="hunger-cluster">
                  <div class="hunger-drops">
                    {#each dots(hunger, 5) as filled}
                      <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
                        <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
                      </svg>
                    {/each}
                  </div>
                  {@render stepper(char, 'hunger', hunger)}
                </div>
```

- [ ] **Step 5: Add the stepper CSS + the cluster wrapper CSS.**

Inside the `<style>` block, after the existing `.hunger-drops { ... }` rule (around line 1046), insert:

```css
  .hunger-cluster {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  /* ── Stat-editor stepper (#7) ────────────────────────────────────────── */
  .stat-stepper {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
  }
  .step-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    font-size: 0.95rem;
    font-weight: 700;
    line-height: 1;
    color: var(--text-muted);
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    cursor: pointer;
    transition: color 0.1s, border-color 0.1s, background 0.1s, opacity 0.1s;
    user-select: none;
  }
  .step-btn:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }
  .step-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .step-btn[aria-busy="true"] {
    opacity: 0.55;
    cursor: wait;
  }
  .stat-stepper.roll20-blocked .step-btn {
    border-style: dashed;
  }
```

- [ ] **Step 6: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS — `cargo test`, `npm run check`, `npm run build` all green. If `npm run check` reports an unresolved import for `CanonicalFieldName`, confirm it's exported from `src/lib/character/api.ts` (it should be — re-exported by #6).

- [ ] **Step 7: Commit.**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(tools/campaign): add stepper snippet + hunger ±1 controls"
```

---

### Task 2: Wire conscience track (humanity + stains)

Two stepper instances above/below the conscience word-track. Humanity steps the `humanity` field; stains steps the `humanity_stains` field.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Replace the conscience row.**

Find the `<!-- ── Conscience ──── -->` block (around line 364–378):

```svelte
          <!-- ── Conscience ──────────────────────────────────────────────── -->
          <div class="conscience-row">
            <div class="conscience-track">
              {#each 'CONSCIENCE'.split('') as letter, i}
                {@const pos = i + 1}
                {@const isFilled = pos <= humanity}
                {@const isStained = pos > 10 - stains}
                <span
                  class="conscience-letter"
                  class:filled={isFilled}
                  class:stained={isStained && !isFilled}
                >{letter}</span>
              {/each}
            </div>
          </div>
```

Replace with:

```svelte
          <!-- ── Conscience ──────────────────────────────────────────────── -->
          <div class="conscience-row">
            <div class="conscience-track">
              {#each 'CONSCIENCE'.split('') as letter, i}
                {@const pos = i + 1}
                {@const isFilled = pos <= humanity}
                {@const isStained = pos > 10 - stains}
                <span
                  class="conscience-letter"
                  class:filled={isFilled}
                  class:stained={isStained && !isFilled}
                >{letter}</span>
              {/each}
            </div>
          </div>
          <div class="conscience-controls">
            <div class="ctrl-row">
              <span class="ctrl-label">Humanity</span>
              {@render stepper(char, 'humanity', humanity)}
            </div>
            <div class="ctrl-row">
              <span class="ctrl-label">Stains</span>
              {@render stepper(char, 'humanity_stains', stains)}
            </div>
          </div>
```

- [ ] **Step 2: Add the conscience-controls CSS.**

Inside the `<style>` block, immediately after the `.conscience-letter.stained::after` rule (around line 1103), insert:

```css
  .conscience-controls {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0.25rem var(--card-pad, 0.6rem) 0.4rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .ctrl-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .ctrl-label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-ghost);
    font-weight: 600;
    flex: 1;
  }
```

- [ ] **Step 3: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit.**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(tools/campaign): humanity + stains ±1 controls"
```

---

### Task 3: Wire health track (superficial + aggravated)

Two stepper instances next to the health track. Each one steps a different damage column.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Replace the health track row.**

Find the `<!-- ── Health track ──── -->` block (around line 380–391):

```svelte
          <!-- ── Health track ────────────────────────────────────────────── -->
          <div class="track-row">
            <div class="track-boxes">
              {#each Array.from({ length: healthMax }, (_, i) => i) as i}
                <div
                  class="box"
                  class:superficial={i >= healthOk && i < healthOk + healthSup}
                  class:aggravated={i >= healthOk + healthSup}
                ></div>
              {/each}
            </div>
          </div>
```

Replace with:

```svelte
          <!-- ── Health track ────────────────────────────────────────────── -->
          <div class="track-row">
            <div class="track-cluster">
              <div class="track-boxes">
                {#each Array.from({ length: healthMax }, (_, i) => i) as i}
                  <div
                    class="box"
                    class:superficial={i >= healthOk && i < healthOk + healthSup}
                    class:aggravated={i >= healthOk + healthSup}
                  ></div>
                {/each}
              </div>
              <div class="track-controls">
                <div class="ctrl-row">
                  <span class="ctrl-label" title="Superficial">Sup</span>
                  {@render stepper(char, 'health_superficial', healthSup)}
                </div>
                <div class="ctrl-row">
                  <span class="ctrl-label" title="Aggravated">Agg</span>
                  {@render stepper(char, 'health_aggravated', healthAgg)}
                </div>
              </div>
            </div>
          </div>
```

- [ ] **Step 2: Add the track-cluster CSS.**

Inside the `<style>` block, immediately after the `.track-row { ... }` rule (around line 1158), insert:

```css
  .track-cluster {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .track-cluster .track-boxes { flex: 1; }
  .track-controls {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    flex-shrink: 0;
  }
```

- [ ] **Step 3: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit.**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(tools/campaign): health track ±1 controls (superficial + aggravated)"
```

---

### Task 4: Wire willpower track (superficial + aggravated)

Mirrors Task 3 against the willpower track.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Replace the willpower track row.**

Find the `<!-- ── Willpower track ──── -->` block (around line 393–405):

```svelte
          <!-- ── Willpower track ─────────────────────────────────────────── -->
          <div class="track-row">
            <div class="track-boxes">
              {#each Array.from({ length: wpMax }, (_, i) => i) as i}
                <div
                  class="box willpower"
                  class:filled={i < wpOk}
                  class:superficial={i >= wpOk && i < wpOk + wpSup}
                  class:aggravated={i >= wpOk + wpSup}
                ></div>
              {/each}
            </div>
          </div>
```

Replace with:

```svelte
          <!-- ── Willpower track ─────────────────────────────────────────── -->
          <div class="track-row">
            <div class="track-cluster">
              <div class="track-boxes">
                {#each Array.from({ length: wpMax }, (_, i) => i) as i}
                  <div
                    class="box willpower"
                    class:filled={i < wpOk}
                    class:superficial={i >= wpOk && i < wpOk + wpSup}
                    class:aggravated={i >= wpOk + wpSup}
                  ></div>
                {/each}
              </div>
              <div class="track-controls">
                <div class="ctrl-row">
                  <span class="ctrl-label" title="Superficial">Sup</span>
                  {@render stepper(char, 'willpower_superficial', wpSup)}
                </div>
                <div class="ctrl-row">
                  <span class="ctrl-label" title="Aggravated">Agg</span>
                  {@render stepper(char, 'willpower_aggravated', wpAgg)}
                </div>
              </div>
            </div>
          </div>
```

- [ ] **Step 2: Run the verification gate.**

(No CSS changes needed — Task 3 already added `.track-cluster` and `.track-controls`.)

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 3: Commit.**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(tools/campaign): willpower track ±1 controls (superficial + aggravated)"
```

---

### Task 5: Roll20 disabled-state polish

The stepper snippet already gates on `liveEditAllowed(char)` and tooltips when blocked, but visually a Roll20 card should signal "this won't work" without the user needing to hover. This task tightens the disabled state.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Confirm Roll20 cards already render disabled steppers.**

The stepper from Task 1 already passes `disabled={!allowed || busy || atFloor}` and adds `class:roll20-blocked={!allowed}`. Run the dev server briefly to visually confirm — when a Roll20 character is connected, every stepper on its card should be greyed out with a dashed border.

```bash
npm run dev
```

Open the app, connect a Roll20 game (or use the existing dev fixture data if you have one), and verify the stepper buttons render with `opacity: 0.35` and a dashed border.

If no Roll20 fixture is available, skip the visual check and rely on the disabled-attribute logic from Task 1.

- [ ] **Step 2: Add a one-line aria hint at the cluster level for accessibility.**

Update the `<span class="stat-stepper" ...>` element inside the `stepper` snippet (added in Task 1, Step 3). The current opening tag is:

```svelte
  <span class="stat-stepper" class:roll20-blocked={!allowed}>
```

Replace it with:

```svelte
  <span
    class="stat-stepper"
    class:roll20-blocked={!allowed}
    aria-disabled={!allowed}
  >
```

- [ ] **Step 3: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit (with issue-closure footer).**

```bash
git add src/tools/Campaign.svelte
git commit -m "$(cat <<'EOF'
feat(tools/campaign): aria-disabled on Roll20-blocked steppers

Closes #7
EOF
)"
```

---

### Task 6: Final verification + manual smoke

Closes the loop with the manual verification path from spec §7.

**Files:** none

- [ ] **Step 1: Final aggregate verification.**

```bash
./scripts/verify.sh
```

Expected: PASS — `cargo test`, `npm run check`, `npm run build` all green.

- [ ] **Step 2: Manual smoke (recommended before announcing done).**

From a Foundry-connected dev session against a character with a saved counterpart:

1. Start the dev app:
   ```bash
   npm run tauri dev
   ```
2. Connect Foundry world (browser, accept cert, GM login, enable module).
3. Open the Campaign tool; pick a character with a saved counterpart.
4. **Hunger:** click `+` next to the hunger drops → live card re-renders within a tick (drops fill); drift badge appears on the live card pointing to the saved snapshot.
5. **Hunger floor:** click `−` until hunger=0; verify the `−` button is then disabled (greyed out).
6. **Hunger ceiling:** click `+` until hunger=5; verify the `+` button is then disabled.
7. **Humanity:** click `+`/`−` next to "Humanity" in the conscience-controls row; conscience word-track grows/shrinks accordingly.
8. **Stains:** click `+` next to "Stains"; the rightmost letters get the stained styling.
9. **Health track:** click `+ Sup` and `+ Agg`; track boxes update.
10. **Willpower track:** same — `+ Sup` and `+ Agg`.
11. **Roll20:** if a Roll20 character is also connected, hover any stepper on the Roll20 card → tooltip "Roll20 live editing not supported (Phase 2.5)"; clicking does nothing because the buttons are `disabled`.
12. **In-flight state:** click any `+` and watch the button briefly show `aria-busy="true"` styling (cursor changes to `wait`, opacity drops to 0.55) until the bridge round-trip resolves.

Skipping the manual smoke is acceptable if `verify.sh` is green; flag it in the PR description so the user knows.

The `Closes #7` footer is already in Task 5's commit message — merging the resulting branch / PR will auto-close the issue.

---

## Dependency graph

```
Task 0 (pre-flight)
  ▼
Task 1 (helpers + snippet + hunger)
  ▼
Task 2 (humanity + stains)
  ▼
Task 3 (health track)
  ▼
Task 4 (willpower track)
  ▼
Task 5 (Roll20 polish)
  ▼
Task 6 (final verify + smoke)
```

Strictly sequential — every task modifies `src/tools/Campaign.svelte`. No parallel-safe partition.

---

## Anti-scope (sub-agents must not touch these files)

| Anti-scope file/area | Why |
|---|---|
| Saved cards (`Campaign.svelte:670-686`) | v1 deferral per Plan A scope decision — saved-card stat panels don't exist yet; building them is a Phase 2.5 follow-up |
| Blood Potency editor | Per spec §2.4 — BP edits are book-keeping events, deserve a separate editor, not inline ±1 |
| Skills / attributes editing | Out of scope per spec §2.1 — Phase 2.5 with Roll20 mappings |
| Optimistic UI updates | Spec §2.9 — wait-for-bridge with `aria-busy` is sufficient |
| Toast component | The spec mentions a "toast pattern" — none exists; this plan uses `console.error` + `window.alert(String(e))`. Building a toast component is its own task |
| Any Rust file | Pure-frontend plan |
| Any other `.svelte` file | Plan A only touches Campaign.svelte |

---

## Verification gate summary

Per CLAUDE.md hard rule: every task ending in a commit runs `./scripts/verify.sh` first.

| Task | `cargo test` impact | `npm run check` impact | `npm run build` impact |
|---|---|---|---|
| 0 | (pre-flight only) | (pre-flight only) | (pre-flight only) |
| 1 | none | new types referenced — must resolve `CanonicalFieldName` import | Campaign.svelte compiles with new snippet |
| 2 | none | none | recompiles |
| 3 | none | none | recompiles |
| 4 | none | none | recompiles |
| 5 | none | none | recompiles |
| 6 | aggregate green | aggregate green | aggregate green |

No new Tauri commands. No new tests. The IPC layer was test-covered by #6.

---

## Pointers

- Spec: `docs/superpowers/specs/2026-05-02-phase-2-character-editing-design.md`
- Sibling plan (Plan B = #8): `docs/superpowers/plans/2026-05-02-advantage-editor.md`
- Router (#6, shipped): `docs/superpowers/plans/2026-05-02-character-set-field-router.md`
- ARCHITECTURE.md §6 (color tokens), §7 (errors)
- Issue: https://github.com/Hampternt/vtmtools/issues/7
