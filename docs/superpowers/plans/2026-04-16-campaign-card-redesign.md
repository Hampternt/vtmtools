# Campaign Card Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign Campaign.svelte character cards with a compact vertical layout and density toggle (Auto/S/M/L) for GM scanning.

**Architecture:** Single-file change to Campaign.svelte. Density state managed by a Svelte `$state` variable. CSS custom properties on `.char-grid` control all size-dependent values. A `ResizeObserver` drives Auto mode.

**Tech Stack:** Svelte 5 (runes), CSS custom properties, ResizeObserver API.

**Spec:** `docs/superpowers/specs/2026-04-16-campaign-card-redesign.md`

---

### Task 1: Add Density State and ResizeObserver

**Files:**
- Modify: `src/tools/Campaign.svelte` — script block (lines 1–115)

- [ ] **Step 1: Add density state variables after line 28**

Add these state declarations after the existing `urlCopied` state:

```typescript
type Density = 'auto' | 's' | 'm' | 'l';
let density = $state<Density>('auto');
let resolvedDensity = $state<'s' | 'm' | 'l'>('m');
let gridEl: HTMLDivElement | undefined = $state(undefined);
```

`density` is what the user picks (auto by default). `resolvedDensity` is the computed S/M/L used for CSS vars. `gridEl` is the bind target for the ResizeObserver.

- [ ] **Step 2: Add ResizeObserver effect after the existing `$effect` block (after line 50)**

```typescript
$effect(() => {
  if (density !== 'auto') {
    resolvedDensity = density;
    return;
  }
  if (!gridEl) return;

  const ro = new ResizeObserver((entries) => {
    const w = entries[0]?.contentRect.width ?? 0;
    if (w < 500) resolvedDensity = 's';
    else if (w < 800) resolvedDensity = 'm';
    else resolvedDensity = 'l';
  });
  ro.observe(gridEl);
  return () => ro.disconnect();
});
```

- [ ] **Step 3: Add density CSS variable helper**

```typescript
const densityVars = $derived(() => {
  const d = resolvedDensity;
  const vals = {
    s: { minCol: '16rem', pad: '0.4rem', trackH: '1.4rem', conscienceCap: '1.5rem', dropSize: '1.2rem', conscienceGlow: 'none' },
    m: { minCol: '20rem', pad: '0.6rem', trackH: '1.8rem', conscienceCap: '2.5rem', dropSize: '1.6rem', conscienceGlow: '0 0 0.3rem color-mix(in srgb, var(--accent) 30%, transparent)' },
    l: { minCol: '28rem', pad: '0.8rem', trackH: '2.4rem', conscienceCap: '4rem', dropSize: '2rem', conscienceGlow: '0 0 0.5rem color-mix(in srgb, var(--accent) 50%, transparent)' },
  }[d];
  return `--col-min:${vals.minCol};--card-pad:${vals.pad};--track-h:${vals.trackH};--conscience-cap:${vals.conscienceCap};--drop-size:${vals.dropSize};--conscience-glow:${vals.conscienceGlow}`;
});
```

- [ ] **Step 4: Run `npm run check`**

Expected: zero errors. The new state/derived values are not yet used in the template.

- [ ] **Step 5: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): add density state, ResizeObserver, and CSS var helper"
```

---

### Task 2: Add Density Toggle to Toolbar

**Files:**
- Modify: `src/tools/Campaign.svelte` — template (toolbar section, line 119–129) and styles

- [ ] **Step 1: Add the segmented toggle to the toolbar template**

Replace the toolbar div (lines 119–129) with:

```svelte
<div class="toolbar">
  <div class="status">
    <div class="status-dot" class:connected></div>
    {connected ? 'Connected to Roll20' : 'Not connected'}
  </div>
  {#if connected && lastSync}
    <span class="sync-time">last sync {timeSince(lastSync)}</span>
  {/if}
  <div class="spacer"></div>
  <div class="density-toggle">
    {#each [['auto', 'Auto'], ['s', 'S'], ['m', 'M'], ['l', 'L']] as [val, label]}
      <button
        class="density-btn"
        class:active={density === val}
        onclick={() => { density = val as Density; }}
      >{label}</button>
    {/each}
  </div>
  <button class="btn-refresh" onclick={refresh} disabled={!connected}>↺ Refresh</button>
</div>
```

- [ ] **Step 2: Add density toggle styles**

Add after the `.btn-refresh:disabled` rule (line 477):

```css
/* ── Density toggle ──────────────────────────────────────────────────── */
.density-toggle {
  display: inline-flex;
  border: 1px solid var(--border-faint);
  border-radius: 5px;
  overflow: hidden;
}
.density-btn {
  background: var(--bg-card);
  color: var(--text-ghost);
  border: none;
  border-right: 1px solid var(--border-faint);
  padding: 0.2rem 0.55rem;
  font-size: 0.7rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
}
.density-btn:last-child { border-right: none; }
.density-btn:hover { color: var(--text-secondary); }
.density-btn.active {
  background: var(--bg-active);
  color: var(--accent);
}
```

- [ ] **Step 3: Run `npm run check`**

Expected: zero errors. Toggle renders but doesn't yet affect card layout.

- [ ] **Step 4: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): add density toggle (Auto/S/M/L) to toolbar"
```

---

### Task 3: Wire Density CSS Variables to the Grid

**Files:**
- Modify: `src/tools/Campaign.svelte` — template (char-grid div, line 189) and styles

- [ ] **Step 1: Bind the grid element and apply density vars**

Change the `.char-grid` div (line 189) from:

```svelte
<div class="char-grid">
```

To:

```svelte
<div class="char-grid" bind:this={gridEl} style={densityVars()}>
```

- [ ] **Step 2: Update `.char-grid` CSS to use the variable**

Change the `.char-grid` rule (lines 606–612) from:

```css
.char-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
  max-width: 1100px;
  gap: 0.75rem;
  align-items: start;
}
```

To:

```css
.char-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(var(--col-min, 20rem), 1fr));
  max-width: 1100px;
  gap: 0.75rem;
  align-items: start;
}
```

- [ ] **Step 3: Run `npm run check`**

Expected: zero errors. Grid now responds to density toggle — column count changes when clicking S/M/L.

- [ ] **Step 4: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): wire density CSS vars to char-grid"
```

---

### Task 4: Restructure Card Header

**Files:**
- Modify: `src/tools/Campaign.svelte` — template (lines 230–270) and styles (lines 622–702)

- [ ] **Step 1: Replace the header + quick-stats template**

Replace the header block (lines 230–252, currently the `card-header` + `header-stats` divs from the earlier partial implementation) and the conscience row (lines 254–268) with:

```svelte
<!-- ── Header ──────────────────────────────────────────────────── -->
<div class="card-header">
  <div class="header-line">
    <span class="char-name">{char.name}</span>
    <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
      {isPC(char) ? 'PC' : 'NPC'}
    </span>
  </div>
  <div class="header-line">
    {#if clan}<span class="char-clan">{clan}</span>{:else}<span></span>{/if}
    <div class="header-vitals">
      <div class="hunger-drops">
        {#each dots(hunger, 5) as filled}
          <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
          </svg>
        {/each}
      </div>
      <div class="bp-pill">
        <span class="qs-label">BP</span>
        <span class="bp-value">{bp}</span>
      </div>
    </div>
  </div>
</div>
```

- [ ] **Step 2: Replace header + related styles**

Remove these CSS blocks entirely:
- `.card-header` (line 624)
- `.name-clan` (line 632)
- `.header-stats` (line 671)
- `.bp-pill` (line 678)

Replace with:

```css
/* ── Header ───────────────────────────────────────────────────────────── */
.card-header {
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
  padding: var(--card-pad, 0.6rem) var(--card-pad, 0.6rem) calc(var(--card-pad, 0.6rem) - 0.1rem);
  border-bottom: 1px solid var(--border-faint);
}
.header-line {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 0.5rem;
}
.header-vitals {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
}
.bp-pill {
  display: flex;
  align-items: center;
  gap: 0.25rem;
}
```

Keep `.char-name`, `.char-clan`, `.badge`, `.badge.pc`, `.badge.npc` styles unchanged.

Remove `.char-clan`'s parent `.name-clan` dependency — `.char-clan` just needs to live as a direct child of `.header-line` now. Its existing styles (italic, ellipsis, nowrap) still work.

- [ ] **Step 3: Update blood-drop size to use density var**

Change `.blood-drop` (line 710):

```css
.blood-drop {
  width: var(--drop-size, 1.6rem);
  height: calc(var(--drop-size, 1.6rem) * 1.3125);
  /* rest unchanged */
}
```

- [ ] **Step 4: Run `npm run check`**

Expected: zero errors.

- [ ] **Step 5: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): restructure card header as two-line layout"
```

---

### Task 5: Restructure Conscience Row

**Files:**
- Modify: `src/tools/Campaign.svelte` — template and styles

- [ ] **Step 1: Ensure conscience template is a standalone full-width row**

The conscience block should be (may already be close to this from the earlier partial work — verify and adjust):

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

- [ ] **Step 2: Update conscience styles to use density vars**

Replace the `.conscience-row`, `.conscience-track`, and `.conscience-letter` CSS blocks with:

```css
/* ── Conscience row ──────────────────────────────────────────────────── */
.conscience-row {
  container-type: inline-size;
  display: flex;
  align-items: stretch;
  padding: 0.2rem var(--card-pad, 0.6rem);
  border-bottom: 1px solid var(--border-faint);
  overflow: hidden;
  box-sizing: border-box;
}
.conscience-track {
  display: flex;
  width: 100%;
  align-items: stretch;
  gap: 0;
}
.conscience-letter {
  flex: 1 1 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-family: 'Last Rites', cursive;
  font-size: min(9cqi, var(--conscience-cap, 2.5rem));
  font-weight: 400;
  color: var(--text-ghost);
  line-height: 1;
  padding: 0.15rem 0;
  overflow: hidden;
  transition: color 0.2s, text-shadow 0.2s;
  position: relative;
}
.conscience-letter.filled {
  color: var(--accent);
  text-shadow: var(--conscience-glow, none);
}
```

Keep `.conscience-letter.stained` and `.conscience-letter.stained::after` unchanged.

- [ ] **Step 3: Run `npm run check`**

Expected: zero errors.

- [ ] **Step 4: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): density-responsive full-width conscience row"
```

---

### Task 6: Restructure Health and Willpower Tracks

**Files:**
- Modify: `src/tools/Campaign.svelte` — template and styles

- [ ] **Step 1: Ensure tracks are separate full-width rows**

The health and willpower blocks should be (may already be close from earlier work):

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

- [ ] **Step 2: Update track styles to use density vars**

Replace `.track-row`, `.track-boxes`, and `.box` CSS blocks with:

```css
/* ── Track row (one per track) ────────────────────────────────────────── */
.track-row {
  padding: 0.2rem var(--card-pad, 0.6rem);
  border-bottom: 1px solid var(--border-faint);
}
.track-boxes {
  display: flex;
  gap: 0.1rem;
}
.box {
  flex: 1;
  min-width: 0;
  height: var(--track-h, 1.8rem);
  border: 1px solid var(--border-surface);
  border-radius: 0.2rem;
  background: transparent;
  box-sizing: border-box;
}
```

Keep all `.box.filled`, `.box.superficial`, `.box.aggravated`, and `.box.willpower.*` variant styles unchanged.

- [ ] **Step 3: Run `npm run check`**

Expected: zero errors.

- [ ] **Step 4: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): full-width density-responsive health/willpower tracks"
```

---

### Task 7: Remove Dead CSS

**Files:**
- Modify: `src/tools/Campaign.svelte` — styles only

- [ ] **Step 1: Delete these CSS blocks that are no longer referenced**

Remove entirely:
- `.quick-stats` (if still present)
- `.qs-cell`
- `.hunger-cell`
- `.qs-center`
- `.qs-right`
- `.tracks-row`
- `.name-clan`
- `.header-stats` (old version)

Keep:
- `.qs-label` — still used in BP pill
- `.bp-value` — still used, but update to use density var:

```css
.bp-value {
  font-size: calc(var(--drop-size, 1.6rem) * 0.9375);
  font-weight: 700;
  color: var(--accent-amber);
  line-height: 1;
}
```

- [ ] **Step 2: Run `npm run check`**

Expected: zero errors, possibly svelte warnings about unused CSS if any dead selectors remain. Fix any that appear.

- [ ] **Step 3: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "refactor(campaign): remove dead CSS from old card layout"
```

---

### Task 8: Visual Verification

**Files:** None modified — verification only.

- [ ] **Step 1: Start dev server**

```bash
npm run dev
```

- [ ] **Step 2: Open in browser and verify Auto mode**

Resize the window and check:
- Cards transition between S/M/L column counts as width changes
- CONSCIENCE letters readable at all sizes, no clipping
- Health (red) and willpower (blue) visually distinct at all sizes
- Header fills both left and right sides
- No empty dead space in header or between sections
- Variable-height cards don't stretch (`align-items: start`)

- [ ] **Step 3: Verify manual density modes**

Click S, M, L buttons in toolbar:
- S: many small cards, compact conscience, thin tracks
- M: moderate cards, medium conscience
- L: fewer large cards, conscience with glow, tall tracks
- Auto: returns to responsive behavior

- [ ] **Step 4: Verify no regressions**

- Disciplines section renders correctly
- Collapsible attrs/info/raw panels still open/close
- Footer toggles work
- PC/NPC badge colors correct
- Blood drops fill/unfill correctly
- Damage states (superficial hatching, aggravated fill) render on both tracks

- [ ] **Step 5: Run `npm run check` one final time**

Expected: zero errors, zero warnings.

- [ ] **Step 6: Final commit if any visual tweaks were needed**

```bash
git add src/tools/Campaign.svelte
git commit -m "fix(campaign): visual polish for card redesign"
```
