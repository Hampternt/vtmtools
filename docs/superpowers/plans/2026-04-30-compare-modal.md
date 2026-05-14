# Compare Modal Implementation Plan (Plan 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show GMs exactly what changed between a saved character snapshot and the current live version. Adds a "Compare" button to live cards (when a saved match exists) and a "drift" badge that signals "saved version is stale" at a glance.

**Architecture:** Pure-frontend plan — zero Rust changes, zero new Tauri commands. Diff computed in TypeScript via `diffCharacter(saved, live)`, which composes a path-based projection (`DIFFABLE_PATHS` over canonical fields + Foundry skill/attribute paths) with a list-based comparator (`diffSpecialties` walking `raw.items`). Modal renders the resulting `DiffEntry[]` as a `before → after` table.

**Tech Stack:** TypeScript / Svelte 5 runes; no test framework (per ARCHITECTURE.md §10).

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md`

**Depends on:** Plan 1 (Saved Characters) — reads `SavedCharacter` shape and the typed API wrappers from `src/lib/saved-characters/api.ts` and `src/store/savedCharacters.svelte.ts`.

---

## File structure

### New files
- `src/lib/saved-characters/diff.ts` — `DIFFABLE_PATHS`, `collectSpecialties`, `diffSpecialties`, `diffCharacter`, `DiffEntry` type
- `src/components/CompareModal.svelte` — modal component rendering `before → after`

### Modified files
- `src/tools/Campaign.svelte` — wire Compare button into live cards (only when saved match exists); add drift badge derivation

### Files explicitly NOT touched
- All Rust files (Plan 2 is pure-frontend)
- Plan 1's API wrapper / store / migration / commands
- Plan 0's bridge files
- `vtmtools-bridge/**`

---

## Task overview

| # | Task | Depends on |
|---|---|---|
| 1 | Create `diff.ts` with `DIFFABLE_PATHS` (canonical paths only) | none |
| 2 | Extend `diff.ts` with Foundry skill + attribute paths | 1 |
| 3 | Add `diffSpecialties` list comparator to `diff.ts` | 1 |
| 4 | Add `diffCharacter` composer (path diff + specialty diff) | 2, 3 |
| 5 | Create `CompareModal.svelte` rendering `DiffEntry[]` | 4 |
| 6 | Wire Compare button + drift badge into `Campaign.svelte` | 5 |
| 7 | Final verification gate | all |

Tasks 2 and 3 are independent of each other (each adds a non-overlapping export to `diff.ts`).

---

## Task 1: Create `diff.ts` with canonical paths

**Files:**
- Create: `src/lib/saved-characters/diff.ts`

**Anti-scope:** No Foundry-specific paths yet. No specialty diffing. No `diffCharacter` composer.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §4 (frontend pure-TS lib reading types from `src/lib/**/api.ts`).

- [ ] **Step 1: Create the file with canonical paths**

```ts
import type { SavedCharacter } from '$lib/saved-characters/api';
import type { BridgeCharacter } from '$lib/bridge/api';

/** A single difference between saved and live. Identity by `key`. */
export interface DiffEntry {
  key: string;
  label: string;
  before: string;
  after: string;
}

/** A diffable path: how to read it, what to call it, stable identity. */
interface DiffablePath {
  key: string;
  label: string;
  read: (c: BridgeCharacter) => string | number | null;
}

/** Canonical fields — apply to all sources (Roll20 + Foundry). */
const CANONICAL_PATHS: DiffablePath[] = [
  { key: 'name',                  label: 'Name',                   read: c => c.name },
  { key: 'hunger',                label: 'Hunger',                 read: c => c.hunger ?? null },
  { key: 'humanity',              label: 'Humanity',               read: c => c.humanity ?? null },
  { key: 'humanity.stains',       label: 'Stains',                 read: c => c.humanityStains ?? null },
  { key: 'health.max',            label: 'Health (max)',           read: c => c.health?.max ?? null },
  { key: 'health.superficial',    label: 'Health (superficial)',   read: c => c.health?.superficial ?? null },
  { key: 'health.aggravated',     label: 'Health (aggravated)',    read: c => c.health?.aggravated ?? null },
  { key: 'willpower.max',         label: 'Willpower (max)',        read: c => c.willpower?.max ?? null },
  { key: 'willpower.superficial', label: 'Willpower (superficial)', read: c => c.willpower?.superficial ?? null },
  { key: 'bloodPotency',          label: 'Blood Potency',          read: c => c.bloodPotency ?? null },
];

// FOUNDRY_PATHS, diffSpecialties, diffCharacter follow in subsequent tasks.

/** Internal: the consolidated path list — exported for use in Task 4's diffCharacter. */
export const DIFFABLE_PATHS: DiffablePath[] = [...CANONICAL_PATHS];
```

If `BridgeCharacter` is not exported from `$lib/bridge/api`, check the actual export name (could be `Character`, `CanonicalCharacter`, etc.) and use that.

- [ ] **Step 2: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/lib/saved-characters/diff.ts
git commit -m "feat(saved-characters): add diff.ts with canonical paths (Plan 2 task 1)"
```

---

## Task 2: Add Foundry skill + attribute paths

**Files:**
- Modify: `src/lib/saved-characters/diff.ts`

**Anti-scope:** No specialty diffing in this task.

**Depends on:** Task 1.

**Invariants cited:** ARCHITECTURE.md §4. **Reference:** `docs/reference/foundry-vtm5e-paths.md` is the source of truth for the WoD5e skill/attribute key list.

- [ ] **Step 1: Read `docs/reference/foundry-vtm5e-paths.md`**

Identify the canonical lists of WoD5e skill keys and attribute keys (the names used at `system.skills.<key>.value` and `system.attributes.<key>.value`).

- [ ] **Step 2: Add the Foundry path arrays + helper**

Above the `export const DIFFABLE_PATHS = [...CANONICAL_PATHS];` line in `diff.ts`, add:

```ts
const FOUNDRY_SKILL_KEYS = [
  // Per docs/reference/foundry-vtm5e-paths.md — the V5 17-skill list:
  'athletics', 'brawl', 'craft', 'drive', 'firearms', 'larceny',
  'melee', 'stealth', 'survival',
  'animal_ken', 'etiquette', 'insight', 'intimidation', 'leadership',
  'performance', 'persuasion', 'streetwise', 'subterfuge',
  'academics', 'awareness', 'finance', 'investigation', 'medicine',
  'occult', 'politics', 'science', 'technology',
];

const FOUNDRY_ATTR_KEYS = [
  'strength', 'dexterity', 'stamina',
  'charisma', 'manipulation', 'composure',
  'intelligence', 'wits', 'resolve',
];

function cap(s: string): string {
  return s.replace(/_/g, ' ').replace(/\b\w/g, ch => ch.toUpperCase());
}

const FOUNDRY_PATHS: DiffablePath[] = [
  ...FOUNDRY_SKILL_KEYS.map(k => ({
    key:   `skills.${k}`,
    label: `${cap(k)} (skill)`,
    read:  (c: BridgeCharacter) =>
      c.source === 'foundry'
        ? ((c.raw as any)?.system?.skills?.[k]?.value ?? null)
        : null,
  })),
  ...FOUNDRY_ATTR_KEYS.map(k => ({
    key:   `attrs.${k}`,
    label: `${cap(k)} (attribute)`,
    read:  (c: BridgeCharacter) =>
      c.source === 'foundry'
        ? ((c.raw as any)?.system?.attributes?.[k]?.value ?? null)
        : null,
  })),
];
```

Replace the existing `export const DIFFABLE_PATHS = [...CANONICAL_PATHS];` line with:

```ts
export const DIFFABLE_PATHS: DiffablePath[] = [...CANONICAL_PATHS, ...FOUNDRY_PATHS];
```

If the spec's reference doc lists fewer or differently-named skills, **use the doc's authoritative list, not the placeholder above**. The exact names matter — they go directly into the `system.skills.<k>` lookup.

- [ ] **Step 3: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/lib/saved-characters/diff.ts
git commit -m "feat(saved-characters): add Foundry skill + attribute path readers to diff (Plan 2 task 2)"
```

---

## Task 3: Add `diffSpecialties` list comparator

**Files:**
- Modify: `src/lib/saved-characters/diff.ts`

**Anti-scope:** Do NOT add the `diffCharacter` composer in this task — Task 4.

**Depends on:** Task 1 (uses `DiffEntry`, `BridgeCharacter`).

**Invariants cited:** ARCHITECTURE.md §4. **Reference:** specialties live as Item documents on the actor with `type === 'speciality'` and `system.skill = '<skill_key>'`.

- [ ] **Step 1: Append to `diff.ts`**

```ts
/** Build a map from skill key → list of specialty names on that skill. */
function collectSpecialties(raw: unknown): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  const items = (raw as any)?.items;
  if (!Array.isArray(items)) return out;
  for (const item of items) {
    if (item?.type !== 'speciality') continue;
    const skill = item?.system?.skill;
    if (typeof skill !== 'string' || !skill) continue;
    if (!out[skill]) out[skill] = [];
    out[skill].push(String(item?.name ?? ''));
  }
  return out;
}

/**
 * List comparator for specialty Items. Roll20 saves skip this entirely.
 * Returns one DiffEntry per skill where the set of specialty names changed,
 * with comma-joined sorted names so order doesn't produce false positives.
 */
export function diffSpecialties(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  if (saved.source !== 'foundry') return [];
  const savedMap = collectSpecialties(saved.raw);
  const liveMap  = collectSpecialties(live.raw);
  const skills = new Set([...Object.keys(savedMap), ...Object.keys(liveMap)]);
  const entries: DiffEntry[] = [];
  for (const skill of skills) {
    const before = (savedMap[skill] ?? []).slice().sort().join(', ') || '—';
    const after  = (liveMap[skill]  ?? []).slice().sort().join(', ') || '—';
    if (before !== after) {
      entries.push({
        key:   `specialty.${skill}`,
        label: `Specialty: ${cap(skill)}`,
        before,
        after,
      });
    }
  }
  return entries;
}
```

- [ ] **Step 2: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/lib/saved-characters/diff.ts
git commit -m "feat(saved-characters): add diffSpecialties list comparator (Plan 2 task 3)"
```

---

## Task 4: Add `diffCharacter` composer

**Files:**
- Modify: `src/lib/saved-characters/diff.ts`

**Anti-scope:** No additional path classes (merits/flaws/disciplines deferred to Phase 2).

**Depends on:** Tasks 2, 3.

**Invariants cited:** ARCHITECTURE.md §4.

- [ ] **Step 1: Append the composer**

```ts
/**
 * Diff a saved character against a live one. Returns the changed entries
 * across canonical fields, Foundry skills/attributes, and specialties.
 *
 * Pure function. Caller is responsible for ensuring the two inputs refer
 * to the same character (typically by matching (source, source_id)).
 */
export function diffCharacter(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  const pathDiffs: DiffEntry[] = DIFFABLE_PATHS
    .map(p => ({ key: p.key, label: p.label, before: p.read(saved), after: p.read(live) }))
    .filter(({ before, after }) => before !== after)
    .map(({ key, label, before, after }) => ({
      key,
      label,
      before: before == null ? '—' : String(before),
      after:  after  == null ? '—' : String(after),
    }));
  return [...pathDiffs, ...diffSpecialties(saved, live)];
}
```

- [ ] **Step 2: Verify type-check + a manual smoke**

Run: `npm run check`
Expected: clean.

Run a quick browser-console smoke (in `npm run tauri dev` if Plan 1 is up):

```js
// In the dev-tools console with Campaign view open:
const live = bridgeStore.characters[0];   // adjust to actual store name
const saved = savedCharacters.list[0];
const { diffCharacter } = await import('/src/lib/saved-characters/diff.ts');
console.log(diffCharacter(saved.canonical, live));
```

Expected: an array of `DiffEntry` if the saved is stale, `[]` if in sync. (Smoke is informational — not a gating step.)

- [ ] **Step 3: Commit**

```bash
git add src/lib/saved-characters/diff.ts
git commit -m "feat(saved-characters): add diffCharacter composer (Plan 2 task 4)"
```

---

## Task 5: Create `CompareModal.svelte`

**Files:**
- Create: `src/components/CompareModal.svelte`

**Anti-scope:** No drift-badge logic here (Task 6 handles that in Campaign).

**Depends on:** Task 4.

**Invariants cited:** ARCHITECTURE.md §6 (CSS uses `:root` tokens, no hardcoded hex; `box-sizing: border-box` for `width:100%` + padding combos).

- [ ] **Step 1: Create the component**

```svelte
<!--
  Modal dialog showing the diff between a saved character snapshot and
  the live bridge view. Closes on Escape or backdrop click. Pure-presentation;
  the diff itself is computed by the caller via diffCharacter().
-->
<script lang="ts">
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import type { BridgeCharacter } from '$lib/bridge/api';
  import { diffCharacter, type DiffEntry } from '$lib/saved-characters/diff';

  let {
    saved,
    live,
    onClose,
  }: {
    saved: SavedCharacter;
    live: BridgeCharacter;
    onClose: () => void;
  } = $props();

  const entries: DiffEntry[] = $derived(diffCharacter(saved.canonical, live));

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div
  class="backdrop"
  onclick={handleBackdropClick}
  role="presentation"
>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="cmp-title">
    <header>
      <h3 id="cmp-title">{saved.name} — saved vs. live</h3>
      <button type="button" class="close" onclick={onClose} aria-label="Close">×</button>
    </header>

    <div class="meta">
      Saved {saved.savedAt}{#if saved.lastUpdatedAt && saved.lastUpdatedAt !== saved.savedAt}, last updated {saved.lastUpdatedAt}{/if}
    </div>

    {#if entries.length === 0}
      <p class="empty">No differences detected — saved snapshot matches the live character.</p>
    {:else}
      <p class="summary">{entries.length} difference{entries.length === 1 ? '' : 's'}</p>
      <table>
        <thead>
          <tr>
            <th>Field</th>
            <th>Saved</th>
            <th></th>
            <th>Live</th>
          </tr>
        </thead>
        <tbody>
          {#each entries as entry (entry.key)}
            <tr>
              <td class="label">{entry.label}</td>
              <td class="before"><code>{entry.before}</code></td>
              <td class="arrow">→</td>
              <td class="after"><code>{entry.after}</code></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <footer>
      <button type="button" onclick={onClose}>Close</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--bg-raised);
    color: var(--text-primary);
    border: 1px solid var(--border-card);
    border-radius: 0.5rem;
    box-sizing: border-box;
    padding: 1rem 1.25rem;
    max-width: 36rem;
    width: 90vw;
    max-height: 80vh;
    overflow: auto;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-faint);
    padding-bottom: 0.5rem;
    margin-bottom: 0.5rem;
  }
  h3 { margin: 0; font-size: 1rem; }
  .close {
    background: transparent;
    border: 0;
    color: var(--text-muted);
    font-size: 1.25rem;
    cursor: pointer;
  }
  .meta { font-size: 0.75rem; color: var(--text-muted); margin-bottom: 0.75rem; }
  .summary { font-size: 0.85rem; color: var(--text-label); margin: 0 0 0.5rem; }
  .empty { color: var(--text-muted); font-style: italic; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th, td { padding: 0.4rem 0.5rem; text-align: left; }
  th { color: var(--text-label); border-bottom: 1px solid var(--border-faint); }
  td.label { color: var(--text-secondary); }
  td.arrow { color: var(--text-ghost); width: 1rem; }
  td.before code { color: var(--text-muted); }
  td.after code { color: var(--text-primary); }
  footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.75rem;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
  }
</style>
```

- [ ] **Step 2: Verify type-check + build**

Run: `npm run check`
Expected: clean.

Run: `npm run build`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/components/CompareModal.svelte
git commit -m "feat(components): add CompareModal showing diff entries (Plan 2 task 5)"
```

---

## Task 6: Wire Compare button + drift badge into Campaign

**Files:**
- Modify: `src/tools/Campaign.svelte`

**Anti-scope:** Do NOT change Plan 1's Save / Update / Delete buttons. Do NOT change saved-card markup beyond optional drift styling.

**Depends on:** Task 5.

**Invariants cited:** ARCHITECTURE.md §5 (cross-tool boundary — components don't `invoke()` directly), §6 (Svelte 5 `{#each}` lifecycle for transitions).

- [ ] **Step 1: Import `diffCharacter` + `CompareModal`**

In `src/tools/Campaign.svelte`'s `<script lang="ts">`:

```ts
import { diffCharacter } from '$lib/saved-characters/diff';
import CompareModal from '$components/CompareModal.svelte';
```

- [ ] **Step 2: Add modal-open state and helper**

```ts
let comparing = $state<{ saved: SavedCharacter; live: BridgeCharacter } | null>(null);

function openCompare(saved: SavedCharacter, live: BridgeCharacter) {
  comparing = { saved, live };
}
function closeCompare() { comparing = null; }
```

(Import `SavedCharacter` from `$lib/saved-characters/api` and `BridgeCharacter` from `$lib/bridge/api` if not already imported.)

- [ ] **Step 3: Add drift badge derivation**

In `<script lang="ts">`:

```ts
const drifts = $derived(new Map<string, boolean>(
  liveWithMatches
    .filter(({ saved }) => saved !== undefined)
    .map(({ live, saved }) => [
      `${live.source}:${live.sourceId}`,
      diffCharacter(saved!.canonical, live).length > 0,
    ])
));

function hasDrift(live: BridgeCharacter): boolean {
  return drifts.get(`${live.source}:${live.sourceId}`) ?? false;
}
```

- [ ] **Step 4: Add Compare button + drift badge to live cards**

Inside the `{#each liveWithMatches as item ...}` block, at the live card's button cluster (added in Plan 1 Task 11), insert before the existing Save / Update button:

```svelte
{#if item.saved}
  <button
    type="button"
    onclick={() => openCompare(item.saved!, item.live)}
  >Compare</button>
{/if}
```

And in the live card's badge area, add:

```svelte
{#if item.saved && hasDrift(item.live)}
  <span class="drift-badge">drift</span>
{/if}
```

- [ ] **Step 5: Render the modal at the bottom of the template**

After the existing live and saved sections, add:

```svelte
{#if comparing}
  <CompareModal
    saved={comparing.saved}
    live={comparing.live}
    onClose={closeCompare}
  />
{/if}
```

- [ ] **Step 6: Add drift-badge styles**

Inside the existing `<style>` block:

```css
  .drift-badge {
    font-size: 0.65em;
    padding: 0.1em 0.5em;
    border-radius: 0.5rem;
    background: color-mix(in oklab, var(--accent-amber) 20%, transparent);
    color: var(--accent-amber);
  }
```

(If `color-mix` isn't supported in the target browser scope, use a semi-transparent `var(--accent-amber)` directly with `opacity: 0.85;`.)

- [ ] **Step 7: Verify type-check + build**

Run: `npm run check`
Expected: clean.

Run: `npm run build`
Expected: clean.

- [ ] **Step 8: Manual verification (dev app)**

Run `npm run tauri dev` with Plan 0 module installed and at least one saved character:

1. Save a live character (Plan 1 path).
2. Modify the same character in Foundry — change hunger, edit a skill rating, or add a specialty.
3. Within ~1s, the live card shows a yellow "drift" badge.
4. Click "Compare" — modal opens listing exactly the changed paths.
5. Add or remove a specialty in Foundry — modal shows a `Specialty: <Skill>` row with the comma-joined names changed.
6. Press Escape or click outside — modal closes.
7. Click "Update saved" — drift badge clears within ~1s; reopening Compare shows "No differences detected."

- [ ] **Step 9: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): add Compare button + drift badge wiring (Plan 2 task 6)"
```

---

## Task 7: Final verification gate

**Files:** none — verification only.

**Depends on:** all previous.

- [ ] **Step 1: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 2: Re-run Task 6 Step 8 manual flow** end-to-end. All scenarios green.

- [ ] **Step 3: Commit any fixups**

```bash
git status --short
```

If clean, no commit. Otherwise:

```bash
git add -A
git commit -m "chore: Plan 2 verification fixups"
```

---

## Self-review checklist

- [x] Spec § 3.5 diff projection (`DIFFABLE_PATHS` with canonical + Foundry paths) — Tasks 1, 2.
- [x] Spec § 3.5 specialty diffing (pulled forward per advisor review) — Task 3.
- [x] Spec § 3.5 `diffCharacter` composing path-based + list-based — Task 4.
- [x] Spec § 3.5 Roll20 fallback (`source !== 'foundry'` returns `null` from FOUNDRY_PATHS readers; specialty diff also no-ops) — covered by Task 2 readers and Task 3 guard.
- [x] Spec § 3.5 drift detection at render time — Task 6 derived state.
- [x] Spec § 3.5 CompareModal with `before → after` table — Task 5.
- [x] Spec § 3.5 Compare button on live cards (only when saved match exists) — Task 6.
- [x] Spec § 3.6 verification gate — Task 7.
- [x] Plan 2 ships **zero Rust changes** — confirmed by file inventory.
- [x] No placeholders / TBDs.
- [x] Anti-scope on every task.
