# Foundry Roll Mirroring — Plan C — RollFeed UI tool

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a "Rolls" entry to the tools registry, mounting a new `RollFeed.svelte` tool that renders the rolls store as a reverse-chronological feed. Each entry shows actor, splat, flavor, dice grid (color-coded per result class), totals, and outcome flags. Filter strip allows per-actor and per-splat filtering.

**Architecture:** One commit. Pure frontend, no IPC additions. Consumes the `rolls` store from Plan B. Uses existing site palette tokens (per ARCH §6) — no new tokens. Follows the dossier visual conventions established by `CharacterCard.svelte` for stylistic continuity. If `RollFeed.svelte` exceeds ~250 lines, split per-row rendering into `RollEntry.svelte`.

**Tech Stack:** Svelte 5 runes, existing color tokens, `slide` transition from Plan A's `RollHistory.svelte` for reference.

---

## Required Reading

- `docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md` §9, §13. Cite in commit.
- `docs/superpowers/plans/2026-05-10-foundry-roll-mirroring-plan-b-ring-and-store.md` — Plan B must be merged. The `rolls` store at `src/store/rolls.svelte.ts` must exist and be subscribed-and-primable.
- `docs/reference/foundry-vtm5e-rolls.md` §"per-die-class result tables" (lines 147-153) — the per-die-class result classification used in the dice grid.
- `ARCHITECTURE.md` §4 (tools registry seam), §6 (color tokens, no hex).
- `src/tools.ts` — existing tool entries; pattern to mirror.
- `src/lib/components/RollHistory.svelte` — existing resonance-roller history component for visual reference (DO NOT modify or reuse — it's bound to a different shape).
- `src/types.ts` — `CanonicalRoll` and `RollSplat` shapes from Plan A.1.

## File Structure

```
src/tools/
└── RollFeed.svelte           (CREATE — feed list + filter strip)

src/lib/components/
└── RollEntry.svelte          (CREATE — per-row component, only if RollFeed.svelte
                                grows past ~250 lines; otherwise inline)

src/
└── tools.ts                  (MODIFY — add 'rolls' entry)

src/routes/
└── +layout.svelte            (CONDITIONALLY MODIFY — only if a new token is genuinely
                                needed for splat-class colors that no existing token
                                covers; default is to reuse existing tokens)
```

One commit, verified by `./scripts/verify.sh`.

---

## Task 1 — RollFeed UI tool

**Files:** as listed in File Structure.

- [ ] **Step 1 (token survey):** Open `src/routes/+layout.svelte` and list the existing color tokens (per ARCH §6 group: text, surfaces, borders, accents, temperament, card-dossier). Map the per-splat / per-result-class palette:

  - Vampire: `--alert-card-dossier` (red, established for active modifiers + criticals)
  - Werewolf: `--accent-amber` (existing token; matches the umber/rage palette)
  - Hunter: `--accent` or a new dedicated token if neither serves
  - Mortal: `--text-secondary` or `--text-muted` (neutral)
  - Result classes: success → `--text-primary`, critical → `--alert-card-dossier`, failure → `--text-muted`, bestial (vampire 1) → `--alert-card-dossier` outline, brutal (werewolf 1) → `--accent-amber` outline
  - Messy crit (vampire 10 on hunger) → `--alert-card-dossier` saturated

  Document the token mapping in a code comment at the top of `RollFeed.svelte` (Step 3). Only add a new `:root` token if a class genuinely has no acceptable existing token — keep the default to "reuse existing".

- [ ] **Step 2:** Open `src/tools.ts`. Locate the existing tools array. Add the new entry — pick a position that fits the existing alphabetical or thematic ordering (likely alongside `Resonance` / `DyscrasiaManager` / `Campaign` / `GmScreen`):

```ts
{
  id: 'rolls',
  label: 'Rolls',
  icon: '🎲',
  component: () => import('./tools/RollFeed.svelte'),
},
```

  The `icon` may be an emoji per the existing convention; verify what other entries use (some may use SVG strings — match the file's convention).

- [ ] **Step 3:** Create `src/tools/RollFeed.svelte`. Full file content:

```svelte
<script lang="ts">
  // Roll feed — reverse-chronological view of the bridge's roll-history ring.
  // Subscribes to the rolls store from Plan B; filters per-actor and per-splat.
  //
  // Color token mapping (per Step 1 of plan-c):
  //   vampire        → --alert-card-dossier
  //   werewolf       → --accent-amber
  //   hunter         → --accent
  //   mortal/unknown → --text-secondary
  //   success/9-6    → --text-primary
  //   critical/10    → --alert-card-dossier (with saturation)
  //   failure/1-5    → --text-muted
  //   bestial 1      → --alert-card-dossier outline
  //   brutal 1       → --accent-amber outline
  //   messy (hunger 10) → --alert-card-dossier (saturated, bg)

  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import { rolls } from '../store/rolls.svelte';
  import type { CanonicalRoll, RollSplat } from '../types';

  onMount(() => { void rolls.ensureLoaded(); });

  // ── Filter state ────────────────────────────────────────────────────────
  let actorFilter = $state<string>('');   // empty = all actors
  let splatFilter = $state<RollSplat | ''>('');

  const actorOptions = $derived(
    Array.from(new Set(rolls.list.map(r => r.actor_name).filter((n): n is string => !!n))).sort()
  );
  const splatOptions: RollSplat[] = ['mortal', 'vampire', 'werewolf', 'hunter'];

  const filteredRolls = $derived(
    rolls.list.filter(r => {
      if (actorFilter && r.actor_name !== actorFilter) return false;
      if (splatFilter && r.splat !== splatFilter) return false;
      return true;
    })
  );

  // ── Render helpers ──────────────────────────────────────────────────────

  function timeAgo(iso: string | null): string {
    if (!iso) return 'just now';
    const t = Date.parse(iso);
    if (Number.isNaN(t)) return 'just now';
    const diff = Math.floor((Date.now() - t) / 1000);
    if (diff < 5) return 'just now';
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return new Date(t).toLocaleString();
  }

  function splatLabel(s: RollSplat): string {
    if (s === 'unknown') return '?';
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  /** Classification of a basic die per docs/reference/foundry-vtm5e-rolls.md:147. */
  function basicClass(d: number): 'critical' | 'success' | 'failure' {
    if (d === 10) return 'critical';
    if (d >= 6) return 'success';
    return 'failure';
  }

  /** Classification of an advanced die — depends on splat. */
  function advancedClass(d: number, splat: RollSplat): string {
    if (splat === 'vampire') {
      if (d === 10) return 'critical messy';
      if (d === 1) return 'bestial';
      if (d >= 6) return 'success';
      return 'failure';
    }
    if (splat === 'werewolf') {
      if (d === 10) return 'critical';
      if (d === 1) return 'brutal';
      if (d >= 6) return 'success';
      return 'failure';
    }
    if (splat === 'hunter') {
      if (d === 10) return 'critical';
      if (d === 1) return 'desperation-fail';
      if (d >= 6) return 'success';
      return 'failure';
    }
    // mortal / unknown → no advanced dice expected, but classify defensively.
    return basicClass(d);
  }

  function outcomeBadge(roll: CanonicalRoll): { label: string; cls: string } {
    if (roll.bestial) return { label: 'BESTIAL', cls: 'bestial' };
    if (roll.brutal) return { label: 'BRUTAL', cls: 'brutal' };
    if (roll.messy) return { label: 'MESSY', cls: 'messy' };
    if (roll.difficulty != null && roll.total < roll.difficulty) {
      return { label: `${roll.total} / ${roll.difficulty}`, cls: 'fail' };
    }
    return { label: `${roll.total}${roll.difficulty != null ? ` / ${roll.difficulty}` : ''}`, cls: 'pass' };
  }

  function clearFilters() {
    actorFilter = '';
    splatFilter = '';
  }
</script>

<div class="roll-feed">
  <div class="toolbar">
    <span class="title">Rolls</span>
    <span class="count" class:dim={filteredRolls.length === 0}>{filteredRolls.length}</span>
    <span class="spacer"></span>
    <select class="filter" bind:value={splatFilter}>
      <option value="">All splats</option>
      {#each splatOptions as s}
        <option value={s}>{splatLabel(s)}</option>
      {/each}
    </select>
    <select class="filter" bind:value={actorFilter}>
      <option value="">All actors</option>
      {#each actorOptions as a}
        <option value={a}>{a}</option>
      {/each}
    </select>
    {#if actorFilter || splatFilter}
      <button class="btn-clear" onclick={clearFilters}>clear</button>
    {/if}
  </div>

  {#if rolls.list.length === 0}
    <div class="empty">
      No rolls yet — when a roll resolves in Foundry, it appears here.
    </div>
  {:else if filteredRolls.length === 0}
    <div class="empty">
      No rolls match the current filter.
    </div>
  {:else}
    <div class="entries">
      {#each filteredRolls as roll (roll.source_id)}
        {@const out = outcomeBadge(roll)}
        <div class="entry splat-{roll.splat}" in:slide={{ duration: 180, axis: 'y' }}>
          <div class="gutter splat-{roll.splat}"></div>
          <div class="body">
            <div class="row-main">
              <span class="flavor">{roll.flavor || 'Roll'}</span>
              <span class="outcome {out.cls}">{out.label}</span>
            </div>
            <div class="row-meta">
              <span class="actor">{roll.actor_name ?? 'GM'}</span>
              <span class="splat-tag splat-{roll.splat}">{splatLabel(roll.splat)}</span>
              {#if roll.criticals > 0}<span class="meta-pill criticals">{roll.criticals} crit</span>{/if}
              <span class="time">{timeAgo(roll.timestamp)}</span>
            </div>
            <div class="dice-row">
              {#each roll.basic_results as d, i}
                <span class="die basic {basicClass(d)}" title="basic die">{d}</span>
              {/each}
              {#if roll.advanced_results.length > 0}
                <span class="dice-sep">+</span>
                {#each roll.advanced_results as d, i}
                  <span class="die advanced {advancedClass(d, roll.splat)}" title="advanced die ({roll.splat})">{d}</span>
                {/each}
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .roll-feed {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    padding: 0.75rem;
    box-sizing: border-box;
  }

  .toolbar {
    display: flex; align-items: center; gap: 0.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .title {
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-label);
    font-weight: 600;
  }
  .count {
    font-size: 0.65rem;
    color: var(--text-secondary);
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 10px;
    padding: 0 0.5rem;
    line-height: 1.6;
  }
  .count.dim { opacity: 0.4; }
  .spacer { flex: 1; }
  .filter {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }
  .btn-clear {
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.25rem 0.5rem;
    font-size: 0.7rem;
    cursor: pointer;
    text-transform: lowercase;
  }
  .btn-clear:hover { color: var(--text-primary); }

  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-style: italic;
    font-size: 0.85rem;
    text-align: center;
    padding: 2rem 1rem;
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    overflow-y: auto;
    flex: 1;
    padding-right: 0.25rem;
  }
  .entries::-webkit-scrollbar { width: 4px; }
  .entries::-webkit-scrollbar-track { background: transparent; }
  .entries::-webkit-scrollbar-thumb { background: var(--border-faint); border-radius: 2px; }

  .entry {
    display: flex;
    gap: 0.6rem;
    padding: 0.5rem 0.6rem;
    background: var(--bg-sunken);
    border-radius: 3px;
    border: 1px solid var(--border-faint);
  }
  .gutter {
    width: 3px;
    border-radius: 2px;
    flex-shrink: 0;
    align-self: stretch;
  }
  .gutter.splat-vampire   { background: var(--alert-card-dossier); }
  .gutter.splat-werewolf  { background: var(--accent-amber); }
  .gutter.splat-hunter    { background: var(--accent); }
  .gutter.splat-mortal,
  .gutter.splat-unknown   { background: var(--text-muted); }

  .body { display: flex; flex-direction: column; gap: 0.3rem; flex: 1; min-width: 0; }

  .row-main {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    justify-content: space-between;
  }
  .flavor {
    font-size: 0.85rem;
    color: var(--text-primary);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .outcome {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    padding: 0.1rem 0.5rem;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .outcome.pass    { background: var(--bg-active); color: var(--text-primary); }
  .outcome.fail    { background: transparent; color: var(--text-muted); border: 1px solid var(--border-faint); }
  .outcome.messy   { background: var(--alert-card-dossier); color: var(--text-primary); }
  .outcome.bestial { background: transparent; color: var(--alert-card-dossier); border: 1px solid var(--alert-card-dossier); }
  .outcome.brutal  { background: transparent; color: var(--accent-amber); border: 1px solid var(--accent-amber); }

  .row-meta {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    flex-wrap: wrap;
    font-size: 0.7rem;
  }
  .actor { color: var(--text-secondary); }
  .splat-tag {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    padding: 0 0.3rem;
    border-radius: 2px;
    border: 1px solid var(--border-faint);
  }
  .splat-tag.splat-vampire   { color: var(--alert-card-dossier); border-color: var(--alert-card-dossier); }
  .splat-tag.splat-werewolf  { color: var(--accent-amber);       border-color: var(--accent-amber); }
  .splat-tag.splat-hunter    { color: var(--accent);              border-color: var(--accent); }
  .meta-pill {
    font-size: 0.65rem;
    color: var(--text-muted);
    background: var(--bg-base);
    padding: 0 0.3rem;
    border-radius: 2px;
  }
  .meta-pill.criticals { color: var(--alert-card-dossier); }
  .time { margin-left: auto; color: var(--text-muted); }

  .dice-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
  }
  .dice-sep {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin: 0 0.15rem;
  }
  .die {
    width: 1.4rem;
    height: 1.4rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
    font-weight: 700;
    border-radius: 3px;
    border: 1px solid var(--border-faint);
    background: var(--bg-base);
    color: var(--text-primary);
  }
  .die.failure          { color: var(--text-muted); }
  .die.success          { color: var(--text-primary); }
  .die.critical         { color: var(--alert-card-dossier); border-color: var(--alert-card-dossier); }
  .die.critical.messy   { background: var(--alert-card-dossier); color: var(--bg-base); }
  .die.bestial          { color: var(--alert-card-dossier); border: 1px dashed var(--alert-card-dossier); }
  .die.brutal           { color: var(--accent-amber);       border: 1px dashed var(--accent-amber); }
  .die.desperation-fail { color: var(--accent);             border: 1px dashed var(--accent); }
</style>
```

  **Sanity-check the `.splat-tag.splat-hunter` rule** — if the Hunter token mapping in Step 1 used `--accent` (typically a red), it may visually conflict with vampire's `--alert-card-dossier`. If conflict is observed during smoke-testing, either swap Hunter to a different existing token or add a `--accent-blue` / `--accent-cyan` token to `:root` and document the addition.

- [ ] **Step 4:** If the file exceeds ~250 lines, split per-row rendering into `src/lib/components/RollEntry.svelte` consuming `roll: CanonicalRoll` as a prop. The split is mechanical: cut the `<div class="entry ...">...</div>` block out of `RollFeed.svelte`'s `{#each filteredRolls}` body, paste into `RollEntry.svelte` with the type imports + helper functions, and replace the cut location with `<RollEntry {roll} />`. Move the helper functions (`basicClass`, `advancedClass`, `outcomeBadge`, `timeAgo`, `splatLabel`) into `RollEntry.svelte` (or extract them into `src/lib/character/roll-helpers.ts` if they're useful elsewhere — default is move, not extract). The dice-row CSS rules move with the markup.

  Currently the file is ~280 lines with full styles, so the split likely IS warranted. Apply it.

  If splitting:

  ```svelte
  <!-- src/lib/components/RollEntry.svelte -->
  <script lang="ts">
    import type { CanonicalRoll, RollSplat } from '../../types';
    let { roll }: { roll: CanonicalRoll } = $props();

    // ... helper functions moved here ...
  </script>

  <!-- ... entry markup ... -->

  <style>
    /* ... entry-scoped styles moved here ... */
  </style>
  ```

  And in `RollFeed.svelte`:

  ```svelte
  import RollEntry from '$lib/components/RollEntry.svelte';
  <!-- ... -->
  {#each filteredRolls as roll (roll.source_id)}
    <RollEntry {roll} />
  {/each}
  ```

- [ ] **Step 5:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 6 (manual smoke):**
  1. `npm run tauri dev`. Sidebar shows the new "Rolls" tool with 🎲 icon.
  2. Click it. Empty state renders: "No rolls yet — when a roll resolves in Foundry, it appears here."
  3. Connect Foundry. In Foundry, roll a vampire skill+attribute (sheet button → confirm dialog → Roll). Within ~1 second the entry appears at the top of the feed.
  4. Verify the entry shows:
     - Correct flavor text
     - Actor name
     - Vampire splat tag (red text/border)
     - Outcome badge (pass/messy/bestial as appropriate)
     - Dice grid: basic dice (no border for failures, primary text for successes, red for criticals); hunger dice after a `+` separator (with bestial outline on natural 1, messy filled-red on natural 10).
     - Time-ago marker ("just now")
  5. Roll a couple more times across different splats. Confirm:
     - Reverse-chronological ordering (newest at top)
     - Splat filter dropdown filters correctly
     - Actor filter dropdown filters correctly
     - "clear" button shows when any filter is active and resets both
  6. Confirm the empty-when-filtered state ("No rolls match the current filter") displays correctly.
  7. Switch to Campaign tool, then back to Rolls. Confirm the feed persists (the store stays mounted via the listen subscription; ensureLoaded is idempotent on `#loaded` flag).
  8. Close the Tauri app and relaunch. Confirm the feed is empty (ephemeral ring per spec).
  9. **Stress check (optional):** Roll 10 times in quick succession. Confirm all entries appear; the slide transition doesn't visibly stall.

- [ ] **Step 7:** Update `ARCHITECTURE.md` §9 Extensibility seams "Add a tool" example list (line ~759 — find the list of existing tool examples). Add `RollFeed.svelte` to the list:

  ```
  - **Add a tool.** Add one entry to `src/tools.ts`. Sidebar +
    lazy-loaded component wiring is automatic. Existing examples:
    `Resonance.svelte`, `DyscrasiaManager.svelte`, `Campaign.svelte`,
    `DomainsManager.svelte`, `GmScreen.svelte`, `RollFeed.svelte` — the pattern is stable.
  ```

  Adjust the existing list if `GmScreen.svelte` isn't already in it (add it too — the pattern should reflect current reality).

- [ ] **Step 8:** Run `./scripts/verify.sh` once more after the doc edit. Expected: green.

- [ ] **Step 9:** Commit.

```bash
git add src/tools/RollFeed.svelte src/tools.ts ARCHITECTURE.md
# If the split into RollEntry.svelte was applied:
git add src/lib/components/RollEntry.svelte

git commit -m "$(cat <<'EOF'
feat(rolls): add Rolls tool — reverse-chronological roll feed

New "Rolls" entry in the tools registry mounts RollFeed.svelte, consuming
the rolls store from Plan B. Each entry renders with splat-coded gutter,
flavor + actor + outcome badge, and a dice grid colored per result class
(success/critical/messy/bestial/brutal). Filter strip supports per-actor
and per-splat selection.

Token-only — uses existing alert-card-dossier / accent-amber / accent /
text tokens; no new :root additions.

Per docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §9, §13.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Checklist (run after Task 1 commit)

- [ ] **Spec coverage:** §9.1 tools registry entry ✓. §9.2 RollFeed.svelte with toolbar / feed / dice grid / filters ✓. §9.3 RollEntry split conditional on size ✓. §13 manual UI verification across splats ✓.
- [ ] **Anti-scope respected:** No Rust changes. No bridge JS changes. No `rolls.svelte.ts` API surface change (frozen by Plan B).
- [ ] **Token discipline:** `grep -E '#[0-9a-fA-F]{3,6}' src/tools/RollFeed.svelte src/lib/components/RollEntry.svelte` — must return empty. Every color via `var(--*)`. If a Hunter-color conflict required a new token, the addition is documented in §6.
- [ ] **Type consistency:** `CanonicalRoll` and `RollSplat` imports match Plan A.1's TS exports. Helper-function classification matches the per-die-class table in `docs/reference/foundry-vtm5e-rolls.md:147-153`.
- [ ] **No frontend test framework introduced.** ARCH §10 invariant.
- [ ] **`./scripts/verify.sh`** green.

## Open questions

- **Outcome badge for partial-success / margin-of-success display** — current implementation shows `total / difficulty` for fail, just `total` for pass-without-difficulty, and named flags (MESSY/BESTIAL/BRUTAL) take precedence. A more granular V5-aware badge ("partial success", "exceptional success") could replace the simple totals — but that's a UX call best left to feedback after v1 ships.
- **Click-to-navigate on the actor section** — spec §14 reserves this seam. Future spec can wire the `navigate-to-character` event from card-modifier-coverage Plan A's banner, anchored to the actor name in each row.
