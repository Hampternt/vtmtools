# Card Modifier Coverage Finish — Plan A — Vital + Discipline Deltas + Banner Nav

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Generalize the active-deltas path resolver, add stat-delta annotations to View 1 (vitals) and View 3 (discipline-level) of CharacterCard, extend ModifierEffectEditor's path autocomplete with vital + discipline paths, and wire the active-modifiers banner click → GmScreen scroll-to-row.

**Architecture:** Pure frontend — no Rust changes, no IPC additions, no schema migration. Replace the hardcoded `<head>.<tail>.value` reader in `active-deltas.ts` with a generic dot-path walker that returns either a direct number leaf or `.value` on an object leaf. View 1 and View 3 read the same `activeDeltas` map already wired in card-redesign Plan B and apply per-element annotations. Banner click publishes a `navigate-to-character` event on the existing `toolEvents` channel; GmScreen subscribes, switches active tool if needed, and scrolls the matching CharacterRow into view.

**Tech Stack:** Svelte 5 runes, TypeScript strict, existing `toolEvents` pub/sub.

---

## Required Reading

Before starting any task, read these files (NOT into context unless your task touches them):

- `docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md` — the spec being implemented; cite §3, §4, §5, §7, §8 in commits.
- `docs/superpowers/specs/2026-05-10-character-card-redesign-design.md` §6 — established stat-delta visualisation pattern on View 2.
- `ARCHITECTURE.md` §4 (cross-tool pub/sub), §6 (color tokens), §10 (testing posture).
- `src/lib/character/active-deltas.ts` — current `readPath` implementation (lines 44-55).
- `src/lib/components/CharacterCard.svelte` — current View 1 / View 3 / View 4 markup.
- `src/lib/components/gm-screen/ModifierEffectEditor.svelte` — current path-input autocomplete (added in commit `d280178`).
- `src/lib/foundry/canonical-names.ts` — existing `FOUNDRY_ATTRIBUTE_NAMES` / `FOUNDRY_SKILL_NAMES` constants.
- `src/store/toolEvents.ts` — existing event publisher / store.
- `src/tools/GmScreen.svelte` — host of CharacterRow rendering.

## File Structure

```
src/lib/character/
└── active-deltas.ts          (MODIFY — replace readPath body)

src/lib/foundry/
└── canonical-names.ts        (MODIFY — add FOUNDRY_VITAL_PATHS const + foundryDisciplineNames helper)

src/lib/components/
├── CharacterCard.svelte      (MODIFY — View 1 vital deltas + View 3 discipline deltas + banner click)
└── gm-screen/
    ├── ModifierEffectEditor.svelte  (MODIFY — extend autocomplete suggestions)
    └── CharacterRow.svelte           (MODIFY — add data-character-source / source-id attrs)

src/store/
└── toolEvents.ts             (MODIFY — add 'navigate-to-character' event variant)

src/tools/
└── GmScreen.svelte           (MODIFY — subscribe to navigate-to-character, scroll to row)
```

No new files. Three commits, each verified by `./scripts/verify.sh`.

---

## Task 1 — Generalize the path resolver in `active-deltas.ts`

**Files:**
- Modify: `src/lib/character/active-deltas.ts:44-55` (replace `readPath` body)
- Tests: none — frontend has no test framework (ARCH §10). Manual verification against the §3.3 coverage matrix.

- [ ] **Step 1:** Open `src/lib/character/active-deltas.ts`. Find the `readPath` function (lines 44-55). Replace its body. The function signature stays the same; only the implementation changes.

```ts
/**
 * Read the integer value at a canonical path on a character.
 * Returns 0 for non-existent paths or non-numeric values.
 *
 * Walks `path` (dot-separated) against `raw.system`. The leaf may be:
 *   - a number directly         (e.g. `health.max`     → system.health.max)
 *   - an object with `.value`   (e.g. `attributes.charisma` → system.attributes.charisma.value)
 *
 * Path coverage matrix (Foundry sources only — Roll20 returns 0):
 *
 *   | Canonical path           | Foundry path                          | Leaf shape       |
 *   |--------------------------|---------------------------------------|------------------|
 *   | attributes.<name>        | system.attributes.<name>              | { value: number }|
 *   | skills.<name>            | system.skills.<name>                  | { value: number }|
 *   | hunger                   | system.hunger.value                   | object → .value  |
 *   | humanity                 | system.humanity.value                 | object → .value  |
 *   | health.max               | system.health.max                     | number directly  |
 *   | health.superficial       | system.health.superficial             | number directly  |
 *   | health.aggravated        | system.health.aggravated              | number directly  |
 *   | willpower.max            | system.willpower.max                  | number directly  |
 *   | willpower.superficial    | system.willpower.superficial          | number directly  |
 *   | willpower.aggravated     | system.willpower.aggravated           | number directly  |
 *   | humanity.stains          | system.humanity.stains                | number directly  |
 *   | blood.potency            | system.blood.potency                  | number directly  |
 *   | disciplines.<name>       | system.disciplines.<name>             | object → .value (verify in plan task 1.5) |
 *
 * Roll20 sources: returns 0 (Roll20's `raw` is a flat attributes[] array,
 * not the `system.*` tree this resolver walks).
 *
 * See: docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md §3.
 */
function readPath(char: BridgeCharacter, path: string): number {
  if (char.source !== 'foundry') return 0;
  const raw = char.raw as { system?: unknown } | null;
  if (!raw?.system) return 0;

  let cur: unknown = raw.system;
  for (const seg of path.split('.')) {
    if (cur === null || typeof cur !== 'object') return 0;
    cur = (cur as Record<string, unknown>)[seg];
    if (cur === undefined) return 0;
  }

  // Leaf is a number directly (e.g. health.max).
  if (typeof cur === 'number') return cur;

  // Leaf is an object with .value (e.g. attributes.charisma → { value: 3 }).
  if (cur !== null && typeof cur === 'object') {
    const v = (cur as { value?: unknown }).value;
    if (typeof v === 'number') return v;
  }

  return 0;
}
```

- [ ] **Step 1.5 (verification gate):** Open `docs/reference/foundry-vtm5e-actor-sample.json` and search for `"disciplines"` to confirm the `system.disciplines.<name>` schema. The expected shape is either `{ value: number, ... }` per discipline OR a direct number per discipline.

  - If the schema is `system.disciplines.auspex.value` → resolver works as-is (object-with-.value branch). No further action.
  - If the schema is `system.disciplines.auspex` as a direct number → resolver works as-is (number-leaf branch).
  - If the schema is **non-conforming** (e.g. disciplines as an array, or per-discipline objects without `.value` and not numeric) → **STOP** and report. The spec §13.1 flagged this as needing verification before code lands. Do NOT proceed to subsequent tasks; raise the schema mismatch to the user.

  Document the verified schema shape as a one-line code comment above the path-coverage matrix in `readPath`'s JSDoc:

  ```
  // Verified 2026-05-10 against foundry-vtm5e-actor-sample.json: disciplines schema is <shape>.
  ```

- [ ] **Step 2:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 3:** Commit.

```bash
git add src/lib/character/active-deltas.ts
git commit -m "$(cat <<'EOF'
feat(active-deltas): generic dot-path walker in readPath

Replace hardcoded <head>.<tail>.value reader with a path walker that
descends raw.system segment-by-segment and unwraps .value when the leaf
is an object. Backward-compatible (attributes.charisma still resolves);
unlocks vital paths (hunger, health.max, humanity.stains, blood.potency,
willpower.max) and disciplines.<name> for upcoming View 1/3 deltas.

Per docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md §3.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2 — View 1 + View 3 annotations + ModifierEffectEditor autocomplete extension (combined)

Combined per `feedback_atomic_cluster_commits` — partial state would show vital deltas wired in some places (resolver) but not annotated in others (UI), which would feel broken to the GM mid-session.

**Files:**
- Modify: `src/lib/foundry/canonical-names.ts` (add `FOUNDRY_VITAL_PATHS` + `foundryDisciplineNames` helper)
- Modify: `src/lib/components/CharacterCard.svelte` (View 1 + View 3 annotation markup + helpers)
- Modify: `src/lib/components/gm-screen/ModifierEffectEditor.svelte` (extend autocomplete suggestions)

- [ ] **Step 1:** Open `src/lib/foundry/canonical-names.ts`. After the existing `FOUNDRY_SKILL_NAMES` declaration, add the vital-paths constant and discipline-names helper. Insert at the bottom of the file:

```ts
/**
 * Canonical path strings for V5 vital tracks. Used by:
 *   - active-deltas.readPath (path-resolver targets)
 *   - ModifierEffectEditor path-input autocomplete
 *
 * Path strings match the dot-path the resolver expects (see active-deltas.ts).
 * `health.superficial` / `health.aggravated` / `willpower.superficial` /
 * `willpower.aggravated` paths resolve correctly but are intentionally OMITTED
 * from autocomplete — uncommon modifier targets; user can type them by hand.
 */
export const FOUNDRY_VITAL_PATHS = [
  'hunger',
  'humanity',
  'humanity.stains',
  'health.max',
  'willpower.max',
  'blood.potency',
] as const;

/**
 * Per-character discipline path autocomplete. Reads the actor's current
 * disciplines map and returns canonical paths like 'disciplines.auspex'.
 * Returns [] for non-Foundry chars or chars with no disciplines field.
 */
export function foundryDisciplineNames(char: import('../../types').BridgeCharacter): string[] {
  if (char.source !== 'foundry') return [];
  const raw = char.raw as { system?: { disciplines?: Record<string, unknown> } } | null;
  const disciplines = raw?.system?.disciplines;
  if (!disciplines || typeof disciplines !== 'object') return [];
  return Object.keys(disciplines).map(name => `disciplines.${name}`);
}
```

- [ ] **Step 2:** Open `src/lib/components/gm-screen/ModifierEffectEditor.svelte`. Find the existing autocomplete-suggestions logic (added in commit `d280178`) — it currently combines `FOUNDRY_ATTRIBUTE_NAMES + FOUNDRY_SKILL_NAMES`. Update the imports and the suggestion-building to include the new constants. Locate the import of canonical-names; extend it:

```ts
import {
  FOUNDRY_ATTRIBUTE_NAMES,
  FOUNDRY_SKILL_NAMES,
  FOUNDRY_VITAL_PATHS,
  foundryDisciplineNames,
} from '$lib/foundry/canonical-names';
```

Find where suggestions are computed (likely a `$derived` expression named `suggestions` or `pathSuggestions`). Replace its body with:

```ts
const pathSuggestions = $derived([
  ...FOUNDRY_ATTRIBUTE_NAMES.map(n => `attributes.${n}`),
  ...FOUNDRY_SKILL_NAMES.map(n => `skills.${n}`),
  ...FOUNDRY_VITAL_PATHS,
  ...foundryDisciplineNames(character),
]);
```

The `character` reference must be in scope — the editor already receives a character/charProp in its props from existing wiring (verify it's the BridgeCharacter or destructure from the existing prop chain). If the editor doesn't currently receive the character, propagate it through from `CharacterCard.svelte`'s editor invocation.

- [ ] **Step 3:** Open `src/lib/components/CharacterCard.svelte`. Find the existing `deltaTooltip` helper (around line 68-74 from the post-card-redesign code). Below it, add view-specific delta-lookup helpers:

```ts
// View 1 / View 3 helpers — shared lookup shape mirrors the View 2 pattern.
function deltaFor(path: string) {
  return activeDeltas.get(path);
}
function deltaSign(delta: number): string {
  return delta >= 0 ? `+${delta}` : `${delta}`;
}
```

- [ ] **Step 4:** Find the View 1 (Basics) markup. Annotate each vital element. Specific patches below — use the existing class names and structure as anchors; keep the patch surgical.

  **4a. Hunger drops row.** Locate the hunger-drops cluster (5-cell drop row). Wrap it with a delta annotation. Replacement pattern:

  ```svelte
  {@const hd = deltaFor('hunger')}
  <div class="hunger-cluster">
    <div class="hunger-drops" title={hd ? deltaTooltip('hunger') : ''}>
      {#each Array(5) as _, i}
        {#if hd}
          <span class="blood-drop" class:filled={i < hd.modified}></span>
        {:else}
          <span class="blood-drop" class:filled={i < (character.hunger ?? 0)}></span>
        {/if}
      {/each}
    </div>
    {#if hd}
      <span class="delta-badge">{deltaSign(hd.delta)}</span>
    {/if}
  </div>
  ```

  Hunger renders are clamped at 5 max — the `Array(5)` loop bound enforces it regardless of `hd.modified` overshooting.

  **4b. BP pill.** Locate the BP pill markup. Replacement:

  ```svelte
  {@const bd = deltaFor('blood.potency')}
  <div class="bp-pill" title={bd ? deltaTooltip('blood.potency') : ''}>
    {#if bd}
      <span class="bp-baseline-strike">{bd.baseline}</span>
      <span class="bp-modified">{bd.modified}</span>
      <span class="delta-badge">{deltaSign(bd.delta)}</span>
    {:else}
      <span class="bp-value">{character.blood_potency ?? 0}</span>
    {/if}
  </div>
  ```

  **4c. Conscience block (humanity).** Locate the 10-letter `CONSCIENCE` row.

  ```svelte
  {@const huD = deltaFor('humanity')}
  {@const stainsD = deltaFor('humanity.stains')}
  {@const humanityVal = huD ? huD.modified : (character.humanity ?? 0)}
  {@const stainsVal = stainsD ? stainsD.modified : (character.humanity_stains ?? 0)}
  <div class="track-cluster">
    <div class="track-row">
      <span class="track-label">Conscience</span>
      {#if huD}
        <span class="delta-badge" title={deltaTooltip('humanity')}>{deltaSign(huD.delta)}</span>
      {/if}
      {#if stainsD}
        <span class="delta-badge stains" title={deltaTooltip('humanity.stains')}>stains {deltaSign(stainsD.delta)}</span>
      {/if}
      <!-- existing stepper renders here, unchanged -->
    </div>
    <div class="conscience-row">
      {#each 'CONSCIENCE'.split('') as letter, i}
        <span
          class="conscience-letter"
          class:filled={i < humanityVal && i >= stainsVal}
          class:stained={i < stainsVal}
        >{letter}</span>
      {/each}
    </div>
  </div>
  ```

  **4d. Health block.** Locate the Health track-row + box-row.

  ```svelte
  {@const hmD = deltaFor('health.max')}
  {@const hsD = deltaFor('health.superficial')}
  {@const haD = deltaFor('health.aggravated')}
  {@const healthMax = hmD ? hmD.modified : (character.health?.max ?? 5)}
  <div class="track-cluster">
    <div class="track-row">
      <span class="track-label">Health</span>
      {#if hmD}
        <span class="delta-badge" title={deltaTooltip('health.max')}>{deltaSign(hmD.delta)}</span>
      {/if}
      {#if hsD}
        <span class="delta-badge minor" title={deltaTooltip('health.superficial')}>sup {deltaSign(hsD.delta)}</span>
      {/if}
      {#if haD}
        <span class="delta-badge minor" title={deltaTooltip('health.aggravated')}>agg {deltaSign(haD.delta)}</span>
      {/if}
      <!-- existing stepper renders here -->
    </div>
    <div class="track-boxes">
      {#each Array(Math.max(0, healthMax)) as _, i}
        <span
          class="box"
          class:superficial={i < (character.health?.superficial ?? 0)}
          class:aggravated={i < (character.health?.aggravated ?? 0)}
        ></span>
      {/each}
    </div>
  </div>
  ```

  **4e. Willpower block.** Same structure as Health, with paths `willpower.max` / `willpower.superficial` / `willpower.aggravated`. Copy 4d, swap `health` → `willpower` and the path strings.

- [ ] **Step 5:** Find the View 3 (Disciplines) markup. Annotate the discipline-name dot indicator. Replacement pattern per discipline row:

```svelte
{#each disciplineList as disc (disc.name)}
  {@const discKey = `disciplines.${disc.name.toLowerCase()}`}
  {@const discD = deltaFor(discKey)}
  <div class="disc-section">
    <div class="disc-name-row">
      <span class="disc-name">{disc.label.toUpperCase()}</span>
      {#if discD}
        <span class="disc-dots-strike" title={deltaTooltip(discKey)}>
          {#each Array(disc.dots) as _, i}<span class="dot-baseline"></span>{/each}
        </span>
        <span class="disc-dots-modified">
          {#each Array(Math.max(0, discD.modified)) as _, i}<span class="dot"></span>{/each}
        </span>
        <span class="delta-badge">{deltaSign(discD.delta)}</span>
      {:else}
        <span class="disc-dots">
          {#each Array(disc.dots) as _, i}<span class="dot"></span>{/each}
        </span>
      {/if}
    </div>
    <!-- existing powers list renders here, unchanged -->
  </div>
{/each}
```

The `disc.name` is the canonical-name slug (lowercase, matching the autocomplete output from `foundryDisciplineNames`); use `.toLowerCase()` if the existing iteration variable uses display-cased names. Verify when reading the existing markup which casing is in scope.

- [ ] **Step 6:** Add the new CSS rules to the `<style>` block in `CharacterCard.svelte`. Token-only, no hex literals (per ARCH §6):

```css
.delta-badge {
  font-size: calc(0.55rem * var(--card-scale, 1));
  font-weight: 700;
  padding: 0 calc(0.3rem * var(--card-scale, 1));
  border-radius: 2px;
  background: var(--alert-card-dossier);
  color: var(--bg-card-dossier);
  letter-spacing: 0.05em;
}
.delta-badge.minor {
  background: transparent;
  color: var(--alert-card-dossier);
  border: 1px solid var(--alert-card-dossier);
  font-weight: 500;
}
.delta-badge.stains {
  background: transparent;
  color: var(--alert-card-dossier);
  border: 1px dashed var(--alert-card-dossier);
}
.bp-baseline-strike {
  text-decoration: line-through;
  color: var(--text-muted);
  margin-right: 0.25em;
  font-size: 0.85em;
}
.bp-modified {
  color: var(--alert-card-dossier);
  font-weight: 700;
}
.disc-dots-strike {
  display: inline-flex;
  text-decoration: line-through;
  opacity: 0.5;
  margin-right: 0.25em;
}
.disc-dots-modified {
  display: inline-flex;
  color: var(--alert-card-dossier);
  font-weight: 700;
  margin-right: 0.25em;
}
.dot-baseline {
  width: calc(0.4rem * var(--card-scale, 1));
  height: calc(0.4rem * var(--card-scale, 1));
  border-radius: 50%;
  background: var(--text-muted);
  margin-right: 1px;
  display: inline-block;
}
```

- [ ] **Step 7:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 8 (manual smoke):** Run `npm run tauri dev`. With Foundry connected and an actor with at least one merit:
  - Add a Stat-kind effect with path `health.max` and delta `+2` to a merit; toggle merit active. View 1's Health track should show 2 extra empty boxes and a `+2` delta badge.
  - Add a Stat-kind effect with path `hunger` and delta `-1`; toggle active. Hunger drops row should reflect the new count and show `−1` badge.
  - Add a Stat-kind effect with path `humanity.stains` and delta `+2`; toggle active. Conscience row's stained count should increase, badge shows `stains +2`.
  - Add a Stat-kind effect with path `disciplines.auspex` and delta `+1` (use a character with Auspex); toggle active. View 3's Auspex dots should show baseline struck-through + modified count + `+1` badge.
  - Open ModifierEffectEditor's path input on any chip; type `health` — autocomplete should suggest `health.max`. Type `dis` — should suggest the actor's actual discipline paths.
  - Toggle each modifier off; confirm annotations cleanly disappear.

- [ ] **Step 9:** Commit.

```bash
git add src/lib/foundry/canonical-names.ts src/lib/components/CharacterCard.svelte src/lib/components/gm-screen/ModifierEffectEditor.svelte
git commit -m "$(cat <<'EOF'
feat(character-card): View 1 vitals + View 3 discipline stat-deltas

View 1 hunger / BP / conscience(humanity) / health / willpower elements
gain delta annotations matching the View 2 pattern when the resolver
returns a hit. View 3 discipline rows gain dot-indicator delta annotation.
ModifierEffectEditor's path-input autocomplete extends to FOUNDRY_VITAL_PATHS
plus per-character disciplines.<name> derived from the actor data.

Health / willpower current-damage paths render badge-only (no fill recompute)
per spec §4.1. Per-power deltas remain deferred to Track 1.5.

Per docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md §4, §5, §7.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3 — Banner click → GM-screen navigation

**Files:**
- Modify: `src/store/toolEvents.ts` (add event variant)
- Modify: `src/lib/components/CharacterCard.svelte` (banner becomes interactive)
- Modify: `src/tools/GmScreen.svelte` (subscribe + scroll handler)
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (data attrs for selector)

- [ ] **Step 1 (seam discovery):** Open `src/components/Sidebar.svelte` (or wherever the sidebar lives — verify path). Find how it controls the active-tool state. The likely seam is one of:
  - A writable runes-mode store (e.g., `currentTool` in `src/store/`)
  - A prop callback up to `App.svelte` / `+page.svelte`
  - URL hash routing

  Note the exact seam name and how to programmatically `set` it. If it's a store, note the import path. Document inline:

  ```
  // Active-tool seam: <e.g. import { currentTool } from '$store/currentTool.svelte'>
  ```

  This becomes the helper invocation pattern in Step 4.

- [ ] **Step 2:** Open `src/store/toolEvents.ts`. Locate the `ToolEvent` type union (it's the discriminated-union type the publisher accepts). Add the new variant:

```ts
export type ToolEvent =
  | { type: 'navigate-to-character'; source: SourceKind; sourceId: string }
  | /* ...existing variants — preserve verbatim... */;
```

The exact existing variants must be preserved — locate the file and copy them, prepending the new variant. Import `SourceKind` from `'../types'` if not already imported.

- [ ] **Step 3:** Open `src/lib/components/CharacterCard.svelte`. Find the existing active-modifiers banner markup (rendered when `hasActiveModifiers === true`). Make it clickable. Replacement:

```svelte
{#if hasActiveModifiers}
  <div
    class="active-modifiers-banner"
    role="button"
    tabindex="0"
    onclick={onBannerClick}
    onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onBannerClick(); } }}
    title="Click to view in GM Screen"
  >
    <span class="banner-label">Active modifiers</span>
    <span class="banner-count">{characterModifiers.filter(m => m.isActive).length}</span>
  </div>
{/if}
```

Add the click handler at the top of the script block (alongside other helpers):

```ts
import { publishEvent } from '../../store/toolEvents';

function onBannerClick() {
  publishEvent({
    type: 'navigate-to-character',
    source: character.source,
    sourceId: character.source_id,
  });
}
```

Add CSS rules (no hex literals):

```css
.active-modifiers-banner {
  cursor: pointer;
  user-select: none;
  transition: background 0.15s, box-shadow 0.15s;
}
.active-modifiers-banner:hover,
.active-modifiers-banner:focus-visible {
  background: rgba(210, 69, 69, 0.18); /* alert-card-dossier @ 18% — stays as token override */
  box-shadow: 0 0 0 1px var(--alert-card-dossier);
  outline: none;
}
```

If the `0.18` rgba is awkward without a hex literal, an alternative is adding a token like `--alert-card-dossier-soft` to `:root` in `src/routes/+layout.svelte`; keep this as the default and only refactor if the inline is rejected by the existing color-token policy. The card-redesign spec already adds `rgba(...)` literals for a couple of dossier rules — in-scope precedent.

- [ ] **Step 4:** Open `src/tools/GmScreen.svelte`. Add a subscribe block on mount. After existing `onMount` content (or as a new `onMount`):

```ts
import { onMount, tick } from 'svelte';
import { toolEvents } from '../store/toolEvents';
// import { currentTool } from '<path identified in Step 1>';

onMount(() => {
  const unsub = toolEvents.subscribe(async ev => {
    if (!ev || ev.type !== 'navigate-to-character') return;

    // 1. Switch to GM Screen if not active.
    // currentTool.set('gm-screen');     // <-- Step 1's seam call

    // 2. Wait for tool-switch render to settle, then scroll target into view.
    await tick();
    const sel = `[data-character-source="${ev.source}"][data-character-source-id="${CSS.escape(ev.sourceId)}"]`;
    const row = document.querySelector(sel);
    row?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    row?.classList.add('flash-target');
    setTimeout(() => row?.classList.remove('flash-target'), 1500);
  });
  return () => unsub();
});
```

Replace the commented `currentTool.set(...)` with the actual call from Step 1's seam. If the seam is a callback, propagate appropriately.

- [ ] **Step 5:** Open `src/lib/components/gm-screen/CharacterRow.svelte`. Find the root element that wraps each row's content. Add data attributes:

```svelte
<div
  class="character-row ..."
  data-character-source={character.source}
  data-character-source-id={character.source_id}
  ...
>
```

The exact root element selector class name should match the existing markup — read the file and patch the existing root `<div>` rather than wrapping a new one.

- [ ] **Step 6:** Add the flash-target CSS rule to `GmScreen.svelte`'s `<style>` block (or `CharacterRow.svelte` if the style is row-scoped):

```css
:global(.character-row.flash-target) {
  animation: flash-pulse 1.5s ease-out;
}
@keyframes flash-pulse {
  0%   { box-shadow: 0 0 0 0   rgba(210, 69, 69, 0.6); }
  50%  { box-shadow: 0 0 0 6px rgba(210, 69, 69, 0); }
  100% { box-shadow: 0 0 0 0   rgba(210, 69, 69, 0); }
}
```

The `:global(...)` is needed because the class is added imperatively via `classList.add`, escaping Svelte's component-scoped CSS. If `CharacterRow.svelte`'s root already uses unscoped CSS modules or component-scoped works for its existing classes, follow the file's existing convention.

- [ ] **Step 7:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 8 (manual smoke):**
  - Open Campaign tool. Find a card with active modifiers (banner visible).
  - Click the banner. Tool switches to GM Screen, the matching character row scrolls into view, and the row briefly flashes red.
  - Test keyboard activation: Tab to the banner, press Enter — same behavior.
  - Test edge case: click banner on a card whose character isn't in the GM Screen filter. Tool switches; no row found; no error toast (silently no-ops, per spec §8.4).
  - Confirm GM Screen-already-active path: from GM Screen open a card overlay (if applicable), click banner — no tool-switch, just scroll.

- [ ] **Step 9:** Commit.

```bash
git add src/store/toolEvents.ts src/lib/components/CharacterCard.svelte src/tools/GmScreen.svelte src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
feat(character-card): banner click navigates to GM Screen row

Active-modifiers banner becomes clickable (role=button, keyboard-accessible)
and publishes a navigate-to-character event on toolEvents. GmScreen subscribes,
switches to its tool if not active, and scrolls the matching CharacterRow into
view with a brief red-pulse flash.

CharacterRow gains data-character-source / data-character-source-id attrs
for the cross-tool selector to land. Generic — any future tool can dispatch
the same event with no GmScreen change.

Per docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md §8.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Checklist (run after Task 3 commit)

- [ ] **Spec coverage:** Every §3 (path resolver), §4 (View 1 annotations per element), §5 (View 3 discipline-level), §7 (autocomplete extension), §8 (banner nav) requirement of the spec is implemented. Per-power deltas (§6 Track 1.5) explicitly NOT done — confirmed deferred. `health.superficial` / `health.aggravated` / `willpower.superficial` / `willpower.aggravated` resolve correctly but are NOT in autocomplete (intentional per §7).
- [ ] **Anti-scope respected:** No View 4 markup changes. No new `:root` tokens (the rgba literals on the banner stay inline as a tactical alpha-override; spec §3.1 of card-redesign already established that pattern). No bridge / IPC / Rust changes.
- [ ] **Token discipline:** `grep -E '#[0-9a-fA-F]{3,6}' src/lib/components/CharacterCard.svelte src/tools/GmScreen.svelte src/lib/components/gm-screen/CharacterRow.svelte src/lib/character/active-deltas.ts` — must return empty (any color via `var(--*)` or inline rgba alpha-overrides of an existing token's RGB). The hex-in-keyframes for the flash-pulse uses the existing `--alert-card-dossier` token color; if that token's hex changes, update the keyframe.
- [ ] **Path resolver coverage matrix verified:** Step 1.5 documented the live discipline schema; the JSDoc table in `readPath` matches reality.
- [ ] **No frontend test framework introduced.** ARCH §10 invariant.
- [ ] **`./scripts/verify.sh`** green for the final commit.

## Open questions (deferred from spec §13)

These do NOT block the plan:

- **Negative-modified-value clamp policy** — current implementation clamps in renderer (`Math.max(0, healthMax)` and `Array(Math.max(0, ...))`), keeping `computeActiveDeltas` arithmetic-pure. Confirmed in plan; revisit only if a future spec wants a single source of truth.
- **Autocomplete vs. lookup mismatch** — autocomplete derives per-character; the GM authoring `disciplines.fortitude` on a Fortitude-less character will type it manually. Acceptable v1 friction.
- **Discipline schema** — Step 1.5 is the verification gate. If non-conforming, the plan stops and the spec amends.
