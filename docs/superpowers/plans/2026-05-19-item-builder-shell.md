# Item Builder Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace three look-unrelated editors (`AdvantageForm`, `ModifierEffectEditor`, `StatusTemplateEditor`) with a unified `ItemBuilderShell` + per-type body components, plus a card-shape composer with attention-spotlight UX, blood-dot magnitude selector, WIP draft save, and per-field colored slots.

**Architecture:** Svelte 5 native `<dialog>`-based shell renders body components through a default snippet. Each body declares its own `tabOrder` and `cardLayout` config. CardComposer wraps bodies with a building-vs-preview top zone, a fixed spotlight zone for the active field, and a footer with WIP-aware save. Body state via `$bindable` for both `valid` and `state`. Preview rendering reuses extracted `AdvantageCard.svelte` / `StatusTemplateCard.svelte` shared with library list views.

**Tech Stack:** Svelte 5 (runes + native `<dialog>`), TypeScript, Tauri 2 IPC, Rust + sqlx (SQLite), VTM dark-theme CSS tokens.

**Spec reference:** `docs/superpowers/specs/2026-05-17-item-builder-shell-design.md` (602 lines). Commits `329e136` + `1cc68e4` on the `templates` branch. The plan implements the spec's 23-step Phasing section. Where this plan and the spec diverge, the spec wins.

**Branch:** all work commits to the `templates` branch in `/home/hampter/projects/vtmtools.worktrees/templates`. Master is unaffected.

**Verify gate (every commit):** `./scripts/verify.sh` (runs `npm run check`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm run build`). Project rule per `CLAUDE.md` — non-negotiable.

---

## File structure

**New files:**
- `src/lib/components/builder/ItemBuilderShell.svelte` — dialog chrome
- `src/lib/components/builder/CardComposer.svelte` — top-zone + spotlight + footer scaffold
- `src/lib/components/builder/TypePicker.svelte` — Step 1 type-picker grid
- `src/lib/components/builder/TagChipInput.svelte` — extracted chip-input primitive
- `src/lib/components/builder/ValueSelector.svelte` — blood-dot row + numpad popover
- `src/lib/components/builder/AppliesToPicker.svelte` — semantic chips for "applies to"
- `src/lib/components/builder/bodies/AdvantageBody.svelte` — Merit/Flaw/Background/Boon body
- `src/lib/components/builder/bodies/StatusTemplateBody.svelte` — status template body
- `src/lib/components/builder/bodies/ModifierEffectBody.svelte` — per-character modifier body
- `src/lib/components/AdvantageCard.svelte` — extracted card-render component
- `src/lib/components/gm-screen/StatusTemplateCard.svelte` — extracted card-render component
- `src/lib/builder/latin-placeholders.ts` — Latin pool + `pickPlaceholder()` utility
- `src/lib/builder/stage-machine.ts` — stage state machine (Set-based confirmed-fields tracking)
- `src/lib/builder/stage-machine.test.ts` — unit tests for state machine
- `src/lib/builder/latin-placeholders.test.ts` — unit tests for placeholder rotation
- `src-tauri/migrations/0010_add_wip_columns.sql` — `wip` + `confirmed_fields` columns
- `src-tauri/migrations/0011_add_bonus_name.sql` — optional per-effect `bonus_name`
- `src/routes/builder-test/+page.svelte` — smoke route for TypePicker

**Modified files:**
- `src/types.ts` — `ModifierKind` adds `auto_success`; `ModifierEffect` adds `bonus_name`; advantage / status-template types add `wip` + `confirmed_fields`
- `src-tauri/src/shared/modifier.rs` — `ModifierKind::AutoSuccess` variant + `ModifierEffect.bonus_name` field
- `src-tauri/src/db/advantage.rs` — `wip` + `confirmed_fields` columns in DB layer
- `src-tauri/src/db/status_template.rs` — same
- `src-tauri/src/db/modifier.rs` — `bonus_name` round-trip
- `src/lib/advantages/api.ts` — type updates
- `src/lib/components/AdvantagesManager.svelte` — replace inline form with shell; use extracted `AdvantageCard`
- `src/lib/components/gm-screen/StatusPaletteDock.svelte` — replace `.editor-overlay` with shell
- `src/lib/components/gm-screen/CharacterCard.svelte` — replace `.editor-overlay` with shell (around line 876)
- `src/lib/components/gm-screen/CharacterRow.svelte` — same change at line 527
- `src/routes/+layout.svelte` — add per-field + blood-dot CSS tokens
- `ARCHITECTURE.md` §4 IPC inventory + §6 design tokens

**Deleted files (Task 22):**
- `src/lib/components/AdvantageForm.svelte`
- `src/lib/components/gm-screen/ModifierEffectEditor.svelte`
- `src/lib/components/gm-screen/StatusTemplateEditor.svelte`

---

## Task 1: Land `TagChipInput` primitive

Extract the chip-input pattern duplicated across the three legacy editors into a single primitive. Refactor `StatusTemplateEditor` to use it as the smoke test (visual no-op for that file).

**Files:**
- Create: `src/lib/components/builder/TagChipInput.svelte`
- Modify: `src/lib/components/gm-screen/StatusTemplateEditor.svelte` (replace inline chip code with `<TagChipInput>`)

- [ ] **Step 1: Create `TagChipInput.svelte`**

```svelte
<script lang="ts">
  interface Props {
    tags: string[];
    onChange: (tags: string[]) => void;
    placeholder?: string;
    autofocus?: boolean;
  }

  let { tags, onChange, placeholder = '+ tag', autofocus = false }: Props = $props();
  let draft = $state('');
  let inputEl: HTMLInputElement;

  function commit() {
    const t = draft.trim();
    if (!t || tags.includes(t)) { draft = ''; return; }
    onChange([...tags, t]);
    draft = '';
  }

  function remove(t: string) { onChange(tags.filter(x => x !== t)); }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Backspace' && draft === '' && tags.length > 0) {
      e.preventDefault();
      remove(tags[tags.length - 1]);
    }
  }

  function onBlur() {
    if (draft.trim()) commit();
  }
</script>

<div class="chips">
  {#each tags as t}
    <span class="chip">
      {t}
      <button type="button" onclick={() => remove(t)} aria-label="Remove tag {t}">×</button>
    </span>
  {/each}
  <input
    bind:this={inputEl}
    type="text"
    {placeholder}
    {autofocus}
    bind:value={draft}
    onkeydown={onKey}
    onblur={onBlur}
  />
</div>

<style>
  .chips {
    display: flex; flex-wrap: wrap; align-items: center; gap: 0.3rem;
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.3rem;
    box-sizing: border-box;
  }
  .chip {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.7rem;
    display: inline-flex; align-items: center; gap: 0.25rem;
  }
  .chip button {
    background: transparent; border: none; color: var(--text-muted);
    cursor: pointer; padding: 0; font-size: 0.7rem;
  }
  .chip button:hover { color: var(--accent); }
  input {
    flex: 1; min-width: 6rem;
    background: transparent; border: none;
    color: var(--text-primary); font-size: 0.7rem;
    padding: 0.2rem;
    outline: none;
  }
  input::placeholder { color: var(--text-muted); }
</style>
```

- [ ] **Step 2: Refactor `StatusTemplateEditor` to use it**

In `src/lib/components/gm-screen/StatusTemplateEditor.svelte`, replace the inline tag-list block (the `<fieldset><legend>Tags</legend>...</fieldset>` around line 142-160) with:

```svelte
<fieldset>
  <legend>Tags</legend>
  <TagChipInput {tags} onChange={(next) => tags = next} />
</fieldset>
```

Add import at top of script block:
```ts
import TagChipInput from '$lib/components/builder/TagChipInput.svelte';
```

Remove now-unused `newTag`, `commitTag`, `removeTag` and their inline markup. The chip CSS in the `<style>` block becomes dead — remove `.tag-list`, `.tag-chip`, `.tag-chip button` rules.

- [ ] **Step 3: Run verify gate**

```bash
./scripts/verify.sh
```

Expected: PASS. Visual no-op — StatusTemplateEditor's tag UI looks identical, just rendered by the primitive.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/TagChipInput.svelte src/lib/components/gm-screen/StatusTemplateEditor.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add TagChipInput primitive, refactor StatusTemplateEditor to use it

First primitive of the Item Builder Shell consolidation. Extracted the
chip-input-with-Backspace-removes-last-chip pattern that all three legacy
editors duplicate. Visual no-op for StatusTemplateEditor.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 2: Land `ValueSelector` primitive

Blood-dot row (1-10) with position-indexed gradient, custom numpad popover for 11-100, scroll-wheel-on-focus increment/decrement, direct typing.

**Files:**
- Create: `src/lib/components/builder/ValueSelector.svelte`
- Create: `src/routes/builder-test/+page.svelte` (smoke route — will gain more primitives in subsequent tasks)
- Modify: `src/routes/+layout.svelte` (add blood-dot CSS tokens)

- [ ] **Step 1: Add blood-dot tokens to `+layout.svelte` `:root`**

In `src/routes/+layout.svelte` `<style>` block, find the existing `:root { ... }` rule and add:

```css
:root {
  /* ...existing tokens... */
  --blood-1:  #2a1310;
  --blood-2:  #3a1814;
  --blood-3:  #4a1d18;
  --blood-4:  #5a221c;
  --blood-5:  #6c2920;
  --blood-6:  #843024;
  --blood-7:  #9c3828;
  --blood-8:  #b04230;
  --blood-9:  #c84c34;
  --blood-10: #e0563a;
}
```

- [ ] **Step 2: Create `ValueSelector.svelte`**

```svelte
<script lang="ts">
  interface Props {
    value: number;
    onChange: (v: number) => void;
    min?: number;
    max?: number;
  }

  let { value, onChange, min = 0, max = 100 }: Props = $props();
  let numpadOpen = $state(false);
  let numpadDraft = $state('');
  let inputEl: HTMLInputElement;

  function setValue(v: number) {
    const clamped = Math.max(min, Math.min(max, Math.round(v)));
    onChange(clamped);
  }

  function onTyped(e: Event) {
    const raw = (e.target as HTMLInputElement).value;
    const n = Number(raw);
    if (!Number.isNaN(n)) setValue(n);
  }

  function onWheel(e: WheelEvent) {
    if (document.activeElement !== inputEl) return; // focused-only per spec
    e.preventDefault();
    const delta = e.deltaY < 0 ? 1 : -1;
    setValue(value + delta);
  }

  function onArrow(e: KeyboardEvent) {
    if (e.key === 'ArrowUp')   { e.preventDefault(); setValue(value + 1); }
    if (e.key === 'ArrowDown') { e.preventDefault(); setValue(value - 1); }
  }

  function commitNumpad() {
    const n = Number(numpadDraft);
    if (!Number.isNaN(n)) setValue(n);
    numpadOpen = false;
    numpadDraft = '';
  }

  function numpadPress(ch: string) {
    if (ch === '⌫') numpadDraft = numpadDraft.slice(0, -1);
    else            numpadDraft = (numpadDraft + ch).slice(0, 3); // max 100
  }
</script>

<div class="value-selector">
  <input
    bind:this={inputEl}
    class="value-input"
    type="text"
    value={String(value)}
    oninput={onTyped}
    onwheel={onWheel}
    onkeydown={onArrow}
  />
  <div class="dot-row">
    {#each [1,2,3,4,5,6,7,8,9,10] as n}
      <button
        type="button"
        class="blood-dot dot-{n}"
        class:filled={value >= n}
        onclick={() => setValue(n)}
        aria-label="Set value to {n}"
      ></button>
    {/each}
    <button
      type="button"
      class="custom-trigger"
      onclick={() => { numpadOpen = !numpadOpen; numpadDraft = String(value); }}
      aria-label="Open custom value numpad"
    >…</button>
  </div>

  {#if numpadOpen}
    <div class="numpad">
      <div class="numpad-display">
        <span>{numpadDraft || 0}</span>
      </div>
      <div class="numpad-grid">
        {#each ['1','2','3','4','5','6','7','8','9','0','⌫'] as ch}
          <button type="button" onclick={() => numpadPress(ch)}>{ch}</button>
        {/each}
        <button type="button" class="enter" onclick={commitNumpad}>Use {numpadDraft || 0}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .value-selector { display: flex; flex-direction: column; gap: 0.5rem; position: relative; }
  .value-input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.4rem 0.6rem;
    font-size: 1.05rem;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    width: 3.2rem; text-align: center;
    font-weight: 600;
    box-sizing: border-box;
    outline: none;
  }
  .value-input:focus { border-color: var(--accent); }
  .dot-row { display: flex; gap: 0.3rem; align-items: center; flex-wrap: wrap; }
  .blood-dot {
    width: 1.55rem; height: 1.55rem; border-radius: 50%;
    border: 1px solid var(--border-surface);
    background: transparent;
    cursor: pointer; padding: 0;
    transition: transform 0.12s, border-color 0.12s, box-shadow 0.12s;
  }
  .blood-dot:hover { transform: scale(1.1); border-color: var(--accent); }
  .blood-dot.dot-1.filled  { background: var(--blood-1); border-color: var(--blood-2); }
  .blood-dot.dot-2.filled  { background: var(--blood-2); border-color: var(--blood-3); }
  .blood-dot.dot-3.filled  { background: var(--blood-3); border-color: var(--blood-4); }
  .blood-dot.dot-4.filled  { background: var(--blood-4); border-color: var(--blood-5); }
  .blood-dot.dot-5.filled  { background: var(--blood-5); border-color: var(--blood-6); }
  .blood-dot.dot-6.filled  { background: var(--blood-6); border-color: var(--blood-7); box-shadow: 0 0 3px rgba(132, 48, 36, 0.4); }
  .blood-dot.dot-7.filled  { background: var(--blood-7); border-color: var(--blood-8); box-shadow: 0 0 4px rgba(156, 56, 40, 0.55); }
  .blood-dot.dot-8.filled  { background: var(--blood-8); border-color: var(--blood-9); box-shadow: 0 0 6px rgba(176, 66, 48, 0.7); }
  .blood-dot.dot-9.filled  { background: var(--blood-9); border-color: var(--blood-10); box-shadow: 0 0 8px rgba(200, 76, 52, 0.85); }
  .blood-dot.dot-10.filled { background: var(--blood-10); border-color: #f47050; box-shadow: 0 0 12px rgba(224, 86, 58, 1), 0 0 24px rgba(224, 86, 58, 0.4); }
  .custom-trigger {
    background: var(--bg-input); border: 1px dashed var(--border-surface);
    color: var(--text-muted);
    width: 1.85rem; height: 1.55rem; padding: 0;
    border-radius: 0.7rem;
    font-size: 0.7rem; cursor: pointer; font-style: italic;
  }
  .custom-trigger:hover { color: var(--accent-amber); border-color: var(--accent-amber); }
  .numpad {
    position: absolute; top: 100%; left: 0; z-index: 10;
    margin-top: 0.4rem;
    background: var(--bg-raised);
    border: 1px solid var(--accent-amber);
    border-radius: 0.4rem;
    padding: 0.55rem;
    box-shadow: 0 0.5rem 1.5rem -0.2rem rgba(0,0,0,0.55);
    display: inline-flex; flex-direction: column; gap: 0.35rem;
  }
  .numpad-display {
    padding: 0.25rem 0.4rem;
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 0.25rem;
    min-width: 6rem; text-align: right;
    color: var(--text-primary);
    font-size: 1.1rem; font-variant-numeric: tabular-nums; font-weight: 600;
  }
  .numpad-grid {
    display: grid;
    grid-template-columns: repeat(3, 1.95rem);
    gap: 0.3rem;
  }
  .numpad-grid button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.25rem;
    width: 1.95rem; height: 1.95rem; font-size: 0.85rem; cursor: pointer;
    padding: 0; font-variant-numeric: tabular-nums; font-weight: 600;
  }
  .numpad-grid button:hover { border-color: var(--accent); color: var(--accent-bright); }
  .numpad-grid .enter {
    grid-column: 1 / span 3;
    background: var(--accent); color: #fff; border-color: var(--accent-bright);
    font-size: 0.72rem; height: 1.7rem;
  }
</style>
```

- [ ] **Step 3: Create smoke test route `src/routes/builder-test/+page.svelte`**

```svelte
<script lang="ts">
  import ValueSelector from '$lib/components/builder/ValueSelector.svelte';
  let v = $state(3);
</script>

<div style="padding: 2rem; color: var(--text-primary); background: var(--bg-page); min-height: 100vh;">
  <h1>Builder Primitives Smoke Test</h1>

  <section style="margin-top: 2rem;">
    <h2>ValueSelector</h2>
    <ValueSelector value={v} onChange={(n) => v = n} />
    <p>Current: {v}</p>
  </section>
</div>
```

- [ ] **Step 4: Run verify gate**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/ValueSelector.svelte src/routes/builder-test/+page.svelte src/routes/+layout.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add ValueSelector primitive with blood-dot gradient

Position-indexed 1-10 dots (stale brown → glowing red), custom numpad
popover for 11-100, scroll-wheel-on-focus, arrow keys, direct typing.
Smoke test at /builder-test.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 3: Land `AppliesToPicker` primitive

Semantic chips (Social / Physical / Mental / A skill… / An attribute… / All rolls) that emit canonical V5 path lists under the hood. Plus an "Advanced" disclosure that reveals the existing path-chip pattern for power users.

**Files:**
- Create: `src/lib/components/builder/AppliesToPicker.svelte`
- Modify: `src/routes/builder-test/+page.svelte` (add AppliesToPicker demo)

- [ ] **Step 1: Create `AppliesToPicker.svelte`**

```svelte
<script lang="ts">
  import { FOUNDRY_SKILL_NAMES, FOUNDRY_ATTRIBUTE_NAMES } from '$lib/foundry/canonical-names';

  interface Props {
    paths: string[];
    onChange: (paths: string[]) => void;
  }

  let { paths, onChange }: Props = $props();
  let advancedOpen = $state(false);
  let pathDraft = $state('');

  // V5 skill groupings. Canonical names match FOUNDRY_SKILL_NAMES casing.
  const PHYSICAL = ['athletics', 'brawl', 'craft', 'drive', 'firearms', 'larceny', 'melee', 'stealth', 'survival'];
  const SOCIAL   = ['animal_ken', 'etiquette', 'insight', 'intimidation', 'leadership', 'performance', 'persuasion', 'streetwise', 'subterfuge'];
  const MENTAL   = ['academics', 'awareness', 'finance', 'investigation', 'medicine', 'occult', 'politics', 'science', 'technology'];

  const SOCIAL_PATHS   = SOCIAL.map(s => `skills.${s}`);
  const PHYSICAL_PATHS = PHYSICAL.map(s => `skills.${s}`);
  const MENTAL_PATHS   = MENTAL.map(s => `skills.${s}`);

  // "Selected" category = paths exactly match a category's full path list
  const selectedSemantic = $derived.by(() => {
    const set = new Set(paths);
    if (paths.length === 0) return 'all';
    if (paths.length === SOCIAL_PATHS.length   && SOCIAL_PATHS.every(p   => set.has(p))) return 'social';
    if (paths.length === PHYSICAL_PATHS.length && PHYSICAL_PATHS.every(p => set.has(p))) return 'physical';
    if (paths.length === MENTAL_PATHS.length   && MENTAL_PATHS.every(p   => set.has(p))) return 'mental';
    return 'custom';
  });

  function selectCategory(category: 'social' | 'physical' | 'mental' | 'all') {
    if (category === 'social')   onChange([...SOCIAL_PATHS]);
    if (category === 'physical') onChange([...PHYSICAL_PATHS]);
    if (category === 'mental')   onChange([...MENTAL_PATHS]);
    if (category === 'all')      onChange([]);
  }

  function pickSkill(skill: string) {
    onChange([`skills.${skill}`]);
  }

  function pickAttribute(attr: string) {
    onChange([`attributes.${attr}`]);
  }

  function commitPath() {
    const p = pathDraft.trim();
    if (!p || paths.includes(p)) { pathDraft = ''; return; }
    onChange([...paths, p]);
    pathDraft = '';
  }

  function removePath(p: string) {
    onChange(paths.filter(x => x !== p));
  }
</script>

<div class="applies-to">
  <div class="chip-row">
    <button class:selected={selectedSemantic === 'social'}   onclick={() => selectCategory('social')}>Social</button>
    <button class:selected={selectedSemantic === 'physical'} onclick={() => selectCategory('physical')}>Physical</button>
    <button class:selected={selectedSemantic === 'mental'}   onclick={() => selectCategory('mental')}>Mental</button>
    <details class="skill-detail">
      <summary>A skill…</summary>
      <div class="dropdown">
        {#each FOUNDRY_SKILL_NAMES as s}
          <button class="dropdown-item" onclick={() => pickSkill(s)}>{s}</button>
        {/each}
      </div>
    </details>
    <details class="skill-detail">
      <summary>An attribute…</summary>
      <div class="dropdown">
        {#each FOUNDRY_ATTRIBUTE_NAMES as a}
          <button class="dropdown-item" onclick={() => pickAttribute(a)}>{a}</button>
        {/each}
      </div>
    </details>
    <button class:selected={selectedSemantic === 'all'} onclick={() => selectCategory('all')}>All rolls</button>
  </div>

  <button class="advanced-toggle" onclick={() => advancedOpen = !advancedOpen}>
    {advancedOpen ? '▾' : '▸'} Advanced: paths · narrow scope
  </button>

  {#if advancedOpen}
    <div class="advanced">
      <div class="path-chips">
        {#each paths as p}
          <span class="path-chip">
            {p === '' ? '(pathless)' : p}
            <button onclick={() => removePath(p)} aria-label="Remove path {p}">×</button>
          </span>
        {/each}
        <input
          type="text"
          placeholder="+ path (e.g. attributes.strength)"
          bind:value={pathDraft}
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); commitPath(); } }}
          onblur={() => { if (pathDraft.trim()) commitPath(); }}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .applies-to { display: flex; flex-direction: column; gap: 0.4rem; }
  .chip-row { display: flex; gap: 0.3rem; flex-wrap: wrap; align-items: flex-start; }
  .chip-row > button, .skill-detail > summary {
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    color: var(--text-secondary);
    border-radius: 0.25rem;
    padding: 0.32rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
    font-family: inherit;
  }
  .chip-row > button:hover, .skill-detail > summary:hover { border-color: var(--accent); color: var(--accent-bright); }
  .chip-row > button.selected {
    border-color: var(--accent);
    background: rgba(168, 80, 64, 0.15);
    color: var(--accent-bright);
    font-weight: 600;
  }
  .skill-detail { position: relative; }
  .skill-detail > summary { list-style: none; cursor: pointer; }
  .dropdown {
    position: absolute; top: 100%; left: 0; z-index: 10;
    margin-top: 0.2rem; max-height: 14rem; overflow-y: auto;
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.3rem;
    box-shadow: 0 0.4rem 1rem -0.2rem rgba(0,0,0,0.5);
    display: flex; flex-direction: column; gap: 0.15rem;
    min-width: 9rem;
  }
  .dropdown-item {
    background: transparent; border: none; text-align: left;
    color: var(--text-secondary); padding: 0.25rem 0.4rem;
    font-size: 0.72rem; cursor: pointer; border-radius: 0.2rem;
    font-family: inherit;
  }
  .dropdown-item:hover { background: var(--bg-card); color: var(--accent-bright); }
  .advanced-toggle {
    background: transparent; border: none; color: var(--text-muted);
    font-size: 0.65rem; cursor: pointer; align-self: flex-start;
    padding: 0.25rem 0; letter-spacing: 0.04em;
  }
  .advanced-toggle:hover { color: var(--accent-bright); }
  .advanced {
    padding: 0.4rem;
    background: rgba(0,0,0,0.18);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
  }
  .path-chips { display: flex; gap: 0.3rem; flex-wrap: wrap; align-items: center; }
  .path-chip {
    background: var(--bg-input); color: var(--text-secondary);
    border: 1px solid var(--border-faint); border-radius: 999px;
    padding: 0.1rem 0.45rem; font-size: 0.65rem;
    font-family: ui-monospace, monospace;
    display: inline-flex; align-items: center; gap: 0.2rem;
  }
  .path-chip button {
    background: transparent; border: none; color: var(--text-muted);
    cursor: pointer; font-size: 0.7rem; padding: 0;
  }
  .path-chips input {
    background: var(--bg-input); border: 1px solid var(--border-faint);
    color: var(--text-primary); border-radius: 0.2rem;
    padding: 0.15rem 0.4rem; font-size: 0.65rem; width: 11rem;
    font-family: ui-monospace, monospace;
    box-sizing: border-box;
  }
</style>
```

- [ ] **Step 2: Add to smoke test route**

Append to `src/routes/builder-test/+page.svelte`:

```svelte
<script>
  // ... existing imports ...
  import AppliesToPicker from '$lib/components/builder/AppliesToPicker.svelte';
  let p = $state<string[]>([]);
</script>

<!-- existing markup -->

<section style="margin-top: 2rem;">
  <h2>AppliesToPicker</h2>
  <AppliesToPicker paths={p} onChange={(next) => p = next} />
  <p>Paths: {JSON.stringify(p)}</p>
</section>
```

- [ ] **Step 3: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/AppliesToPicker.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add AppliesToPicker primitive with semantic chips

Replaces jargon path-input for beginners. Social/Physical/Mental
chips emit canonical V5 skill path lists; "A skill…" and "An attribute…"
dropdowns offer single-pick. Advanced disclosure exposes the path-chip
system for power users.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 4: Land Latin placeholder pool (TDD)

Tests REQUIRED per spec. Deterministic-by-existing-count rotation.

**Files:**
- Create: `src/lib/builder/latin-placeholders.ts`
- Create: `src/lib/builder/latin-placeholders.test.ts`

- [ ] **Step 1: Write failing test**

`src/lib/builder/latin-placeholders.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { LATIN_PLACEHOLDERS, pickPlaceholder } from './latin-placeholders';

describe('LATIN_PLACEHOLDERS', () => {
  it('has the 12 stub entries from the spec', () => {
    expect(LATIN_PLACEHOLDERS).toHaveLength(12);
    expect(LATIN_PLACEHOLDERS).toContain('Vinculum Sanguinis');
    expect(LATIN_PLACEHOLDERS).toContain('Praesidium Noctis');
    expect(LATIN_PLACEHOLDERS).toContain('Silentium Tumuli');
  });
});

describe('pickPlaceholder', () => {
  it('returns deterministic value for the same input count', () => {
    expect(pickPlaceholder(0)).toBe(pickPlaceholder(0));
    expect(pickPlaceholder(5)).toBe(pickPlaceholder(5));
  });

  it('cycles through the pool as count grows', () => {
    expect(pickPlaceholder(0)).not.toBe(pickPlaceholder(1));
  });

  it('wraps around past pool length', () => {
    expect(pickPlaceholder(0)).toBe(pickPlaceholder(LATIN_PLACEHOLDERS.length));
  });

  it('returns a non-empty string for negative or large counts', () => {
    expect(pickPlaceholder(-1)).toMatch(/.+/);
    expect(pickPlaceholder(99999)).toMatch(/.+/);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npm run check
```

Expected: TypeScript error — `latin-placeholders` module not found.

- [ ] **Step 3: Implement `latin-placeholders.ts`**

```ts
export const LATIN_PLACEHOLDERS = [
  'Vinculum Sanguinis',
  'Praesidium Noctis',
  'Fons Aeternitatis',
  'Vigil Tenebris',
  'Memoria Aevi',
  'Custos Animae',
  'Tenebrarum Filia',
  'Sanguis Vetus',
  'Aetas Foederis',
  'Iter Sub Luna',
  'Cor Tenebrosum',
  'Silentium Tumuli',
] as const;

/**
 * Deterministic placeholder rotation. count = number of existing items
 * (so placeholder is stable while the user is typing — only rolls when
 * a new item lands in the library).
 */
export function pickPlaceholder(count: number): string {
  const len = LATIN_PLACEHOLDERS.length;
  const idx = ((count % len) + len) % len;
  return LATIN_PLACEHOLDERS[idx];
}
```

- [ ] **Step 4: Run test to verify it passes**

If vitest is set up, run:
```bash
npx vitest run src/lib/builder/latin-placeholders.test.ts
```

If vitest is not yet set up in this project, the test will exist but be skipped during `verify.sh`. Confirm by running `./scripts/verify.sh` — `npm run check` should pass (type-checks the test file).

Expected: PASS.

- [ ] **Step 5: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/builder/latin-placeholders.ts src/lib/builder/latin-placeholders.test.ts
git commit -m "$(cat <<'EOF'
feat(builder): add Latin placeholder pool with deterministic rotation

12-entry curated Latin word pool for Name field default. Rotation is
deterministic by existing-item-count so the placeholder is stable while
the user types (only re-rolls when a new advantage lands in the library).

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 5: Add `auto_success` to `ModifierKind`

Atomic commit covering Rust enum + TS mirror + DB read/write paths. Mismatched schemas break runtime, so this is one commit.

**Files:**
- Modify: `src-tauri/src/shared/modifier.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Add `AutoSuccess` variant to Rust enum**

In `src-tauri/src/shared/modifier.rs` around line 74 (the `ModifierKind` enum):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind {
    Pool,
    Difficulty,
    Note,
    Stat,
    AutoSuccess,
}
```

Verify the existing serde tag uses `snake_case` so `AutoSuccess` becomes `"auto_success"` on the wire.

If there are any `match` expressions in the codebase that switch on `ModifierKind`, Rust's exhaustiveness check will catch them. Find and handle:

```bash
grep -rn "ModifierKind::" src-tauri/src/ | grep -v "AutoSuccess"
```

For each match arm, add `ModifierKind::AutoSuccess => { /* render-time only in v1 */ }` or equivalent. If the match doesn't need to handle it (e.g., it's an early-return on `Pool` only), add a catch-all that does nothing.

- [ ] **Step 2: Update TS mirror**

In `src/types.ts` around line 280:

```ts
export type ModifierKind = 'pool' | 'difficulty' | 'note' | 'stat' | 'auto_success';
```

Run `npm run check` and fix any TS exhaustiveness errors. Pattern: add `| 'auto_success'` cases that no-op or fall through to existing behavior. The roller does NOT consume this variant in v1 — it's storage-only.

- [ ] **Step 3: Run verify gate**

```bash
./scripts/verify.sh
```

Expected: PASS. All existing tests in `modifier.rs` (lines 150-181 reference Stat/Pool/Difficulty) continue working — no test changes needed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/modifier.rs src/types.ts
git commit -m "$(cat <<'EOF'
feat(modifier): add auto_success variant to ModifierKind

Forward-compatible storage for V5 automatic-success bonuses. The V5
dice roller does NOT consume this variant in v1 — it's stored and
displayed but doesn't affect roll resolution. When the dice helpers
gain support, existing auto_success effects begin working automatically.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 6: Add `wip` + `confirmed_fields` columns

Migration + Rust struct + IPC + TS types. Atomic — schema/code mismatch breaks runtime.

**Files:**
- Create: `src-tauri/migrations/0010_add_wip_columns.sql`
- Modify: `src-tauri/src/db/advantage.rs`
- Modify: `src-tauri/src/db/status_template.rs`
- Modify: `src/types.ts`

- [ ] **Step 1: Create migration file**

`src-tauri/migrations/0010_add_wip_columns.sql`:

```sql
ALTER TABLE advantages       ADD COLUMN wip              INTEGER NOT NULL DEFAULT 0;
ALTER TABLE advantages       ADD COLUMN confirmed_fields TEXT;
ALTER TABLE status_templates ADD COLUMN wip              INTEGER NOT NULL DEFAULT 0;
ALTER TABLE status_templates ADD COLUMN confirmed_fields TEXT;
```

(Confirm exact table names by inspecting existing migrations 0003 and 0005 first — `advantages` and `status_templates` are likely correct but verify.)

- [ ] **Step 2: Update `Advantage` Rust struct**

In `src-tauri/src/shared/` (find the file declaring `Advantage` struct — likely `src-tauri/src/shared/advantage.rs` or in a types module), add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advantage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub kind: AdvantageKind,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,
    pub is_custom: bool,
    pub source_attribution: Option<String>,
    pub wip: bool,                              // NEW
    pub confirmed_fields: Option<Vec<String>>,  // NEW · JSON-decoded; None when wip=false
}
```

(Adjust field names to match the existing struct exactly.)

Same shape addition for `StatusTemplate` struct (find in `src-tauri/src/shared/status_template.rs` or similar).

- [ ] **Step 3: Update DB layer for advantages**

In `src-tauri/src/db/advantage.rs`, update:
- `row_to_advantage` (around line 56) to read `wip: r.try_get::<i64, _>("wip")? != 0` and `confirmed_fields: r.try_get::<Option<String>, _>("confirmed_fields")?.map(|s| serde_json::from_str(&s).unwrap_or_default())`.
- `db_insert` and `db_update` to bind the new columns.
- Test pool CREATE TABLE (around line 582 in `mod tests`) — add the new columns to keep tests passing.

Same pattern for `src-tauri/src/db/status_template.rs`.

- [ ] **Step 4: Update TS types**

In `src/types.ts`, find the `Advantage` interface and add:

```ts
export interface Advantage {
  // ...existing fields...
  wip: boolean;
  confirmedFields: string[] | null;  // null when wip=false
}
```

camelCase per `project_canonical_character_naming` memory.

Find `AdvantageInput` (used for create/update IPC) — add the same fields. Default `wip: false`, `confirmedFields: null` if creating.

Same for `StatusTemplate` interface.

- [ ] **Step 5: Run verify gate**

```bash
./scripts/verify.sh
```

Expected: PASS. Existing migrations rerun on test DBs; new columns get added.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations/0010_add_wip_columns.sql src-tauri/src/shared/ src-tauri/src/db/advantage.rs src-tauri/src/db/status_template.rs src/types.ts
git commit -m "$(cat <<'EOF'
feat(db): add wip + confirmed_fields columns to advantages + status_templates

wip flag enables draft save (invalid forms can be saved + reopened).
confirmed_fields stores a JSON array of stage keys for the builder's
reopen flow — avoids the "confirmed empty vs never reached" ambiguity
of inferring from data presence.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 7: Add `bonus_name` to `ModifierEffect` AND `effects` to `Advantage`

Bundled because the bonus that AdvantageBody composes needs somewhere to land. Today's `Advantage` struct has `properties: Field[]` but no `effects: ModifierEffect[]`. The builder's Bonus stage produces a `ModifierEffect` (kind / value / paths / bonus_name) — the existing model can't store it as-is. This task adds both the `bonus_name` field on `ModifierEffect` AND a new `effects` column on `advantages` + `status_templates`. Forward-compatible pattern (same as `auto_success`): the Foundry push code (`push_advantage_to_world` in `src-tauri/src/tools/library_push.rs`) is NOT updated in this plan — it continues sending only name/description/points/kind. A follow-up makes push include effects[].

**Files:**
- Create: `src-tauri/migrations/0011_add_bonus_name_and_effects.sql`
- Modify: `src-tauri/src/shared/modifier.rs`
- Modify: `src-tauri/src/db/modifier.rs`
- Modify: `src-tauri/src/shared/` (Advantage struct file)
- Modify: `src-tauri/src/db/advantage.rs`
- Modify: `src-tauri/src/db/status_template.rs`
- Modify: `src/types.ts`
- Modify: `src/lib/advantages/api.ts` (AdvantageInput add `effects`)

- [ ] **Step 1: Determine ModifierEffect storage shape**

`ModifierEffect` is currently stored either as JSON inside an `effects_json` column or in a normalized table. Inspect `src-tauri/src/db/modifier.rs` to determine which:

```bash
grep -n "CREATE TABLE\|effects_json\|bonus_name" src-tauri/src/db/modifier.rs
```

- **If stored as JSON column:** no SQL migration needed for `bonus_name` — JSON parsing is forgiving with `#[serde(default)]`.
- **If stored in a normalized table:** the migration below adds the column.

- [ ] **Step 1b: Create migration adding `effects` column to advantages + status_templates**

`src-tauri/migrations/0011_add_bonus_name_and_effects.sql`:

```sql
-- effects[] for advantages and status_templates: JSON array of ModifierEffect
ALTER TABLE advantages       ADD COLUMN effects_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE status_templates ADD COLUMN effects_json TEXT NOT NULL DEFAULT '[]';

-- If modifier effects are stored in a normalized table, also add bonus_name there:
-- ALTER TABLE modifier_effects ADD COLUMN bonus_name TEXT;
```

(Uncomment the `modifier_effects` line only if step 1's grep confirms a normalized table. Otherwise leave it commented — the JSON storage handles bonus_name via serde.)

- [ ] **Step 2: Update `ModifierEffect` Rust struct**

In `src-tauri/src/shared/modifier.rs`, modify `ModifierEffect`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModifierEffect {
    pub kind: ModifierKind,
    pub scope: Option<String>,
    pub delta: Option<i32>,
    pub note: Option<String>,
    pub paths: Vec<String>,
    #[serde(default)]
    pub bonus_name: Option<String>,
}
```

`#[serde(default)]` ensures legacy JSON without `bonus_name` deserializes cleanly.

- [ ] **Step 3: Add `effects` to `Advantage` Rust struct**

In the file declaring `Advantage` (find with `grep -rn "pub struct Advantage" src-tauri/src/shared/`), add the field:

```rust
pub struct Advantage {
    // ...existing fields...
    pub effects: Vec<ModifierEffect>,
}
```

Same for `StatusTemplate` if it doesn't already have one (it might — check):

```bash
grep -n "pub struct StatusTemplate" src-tauri/src/shared/status_template.rs
```

If StatusTemplate already has `effects`, no change. Otherwise add the field.

- [ ] **Step 4: Update DB layer**

In `src-tauri/src/db/advantage.rs`:
- `row_to_advantage` reads `effects_json` and `serde_json::from_str()` it into `Vec<ModifierEffect>`. Empty `'[]'` deserializes to empty Vec.
- `db_insert` and `db_update` serialize `effects` to JSON and bind to `effects_json` column.
- Test pool's CREATE TABLE adds `effects_json TEXT NOT NULL DEFAULT '[]'`.

Same pattern for `src-tauri/src/db/status_template.rs`.

`src-tauri/src/db/modifier.rs` reads `bonus_name` via JSON serde — no SQL changes if effects are JSON-stored.

- [ ] **Step 5: Update TS types + AdvantageInput**

In `src/types.ts`:

```ts
export interface ModifierEffect {
  kind: ModifierKind;
  scope: string | null;
  delta: number | null;
  note: string | null;
  paths: string[];
  bonusName: string | null;  // NEW (camelCase TS-side)
}

export interface Advantage {
  // ...existing fields...
  effects: ModifierEffect[];  // NEW
  // (wip, confirmedFields already added in Task 6)
}
```

In `src/lib/advantages/api.ts`:

```ts
export type AdvantageInput = {
  name: string;
  description: string;
  kind: AdvantageKind;
  tags: string[];
  properties: Field[];
  effects: ModifierEffect[];  // NEW
  wip?: boolean;
  confirmedFields?: string[] | null;
};
```

Find construction sites of `AdvantageInput` (existing AdvantageForm save path) and add `effects: []` to keep them compiling. They don't author effects today — Task 18 wires the builder's bonus output.

- [ ] **Step 6: Run verify gate**

```bash
./scripts/verify.sh
```

Existing `push_advantage_to_world` continues using `name + kind + description + points` — it does NOT yet consume `effects`. This is forward-compat storage; the push pipeline gains effect-pushing in a follow-up PR after v1.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/migrations/0011_add_bonus_name_and_effects.sql src-tauri/src/shared/modifier.rs src-tauri/src/shared/ src-tauri/src/db/modifier.rs src-tauri/src/db/advantage.rs src-tauri/src/db/status_template.rs src/types.ts src/lib/advantages/api.ts
git commit -m "$(cat <<'EOF'
feat(advantage,modifier): add effects[] to Advantage, bonus_name to ModifierEffect

Two coupled additions. The builder's Bonus stage authors ModifierEffect
data; Advantage now has somewhere to store it (effects_json column,
JSON-serialized Vec<ModifierEffect>). bonus_name on each effect supports
per-bonus Foundry naming.

Forward-compat: push_advantage_to_world does NOT yet consume Advantage.effects.
Same pattern as auto_success — the consumer wires up in a follow-up PR.
For now, effects are persisted and rendered in the new builder, not
pushed to Foundry.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 8: Extract `AdvantageCard.svelte`

Pull inline card rendering out of `AdvantagesManager.svelte` into a standalone reusable component. Pure refactor — no visual change yet.

**Files:**
- Create: `src/lib/components/AdvantageCard.svelte`
- Modify: `src/lib/components/AdvantagesManager.svelte`

- [ ] **Step 1: Inspect current AdvantagesManager card rendering**

```bash
grep -n "card\|Card\|<article\|<section" src/lib/components/AdvantagesManager.svelte | head -30
```

Identify the markup block that renders a single advantage card in the grid. Copy that markup verbatim — it becomes the basis of the new component.

- [ ] **Step 2: Create `AdvantageCard.svelte`**

```svelte
<script lang="ts">
  import type { Advantage } from '$lib/../types';

  // Discriminated input: saved (from DB) | preview (unsaved, from builder)
  type SavedInput   = { kind: 'saved';   data: Advantage };
  type PreviewInput = { kind: 'preview'; data: Omit<Advantage, 'id' | 'is_custom' | 'source_attribution'>; wip?: boolean };

  interface Props {
    input: SavedInput | PreviewInput;
    onClick?: (target: HTMLElement) => void;  // preview mode click-to-edit
  }

  let { input, onClick }: Props = $props();

  const data = $derived(input.data);
  const isPreview = $derived(input.kind === 'preview');
  const isWip = $derived(input.kind === 'saved' ? input.data.wip : (input.wip ?? false));
</script>

<article
  class="advantage-card"
  class:preview={isPreview}
  class:wip={isWip}
  onclick={isPreview && onClick ? (e) => onClick(e.target as HTMLElement) : undefined}
>
  <!-- Paste the existing card markup here, parameterized over `data` -->
</article>

<style>
  /* Paste existing card styles here, then add: */
  .advantage-card.preview {
    border-color: var(--accent-amber);
    box-shadow: 0 0 16px rgba(217, 165, 96, 0.22);
  }
  .advantage-card.wip {
    /* Subtle draft indicator — small draft pill, dimmed slightly */
    opacity: 0.92;
  }
</style>
```

Move the actual markup (whatever AdvantagesManager renders today for a card) inside the `<article>`. Move the associated CSS into this file's `<style>` block.

- [ ] **Step 3: Update `AdvantagesManager.svelte` to use the extracted component**

In `src/lib/components/AdvantagesManager.svelte`:
- Add `import AdvantageCard from './AdvantageCard.svelte';` at the top of the script.
- Replace the inline card markup with `<AdvantageCard input={{ kind: 'saved', data: advantage }} />`.
- Remove the moved CSS from this file's `<style>`.

- [ ] **Step 4: Run verify gate + visual smoke**

```bash
./scripts/verify.sh
```

Then run `npm run tauri dev`. Open the Advantages tool. **Visual:** cards look identical to before — no spacing, color, or layout change. WIP pill is not yet visible (no WIP advantages exist yet).

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/AdvantageCard.svelte src/lib/components/AdvantagesManager.svelte
git commit -m "$(cat <<'EOF'
refactor(advantages): extract AdvantageCard.svelte from AdvantagesManager

Card rendering now lives in a reusable component with a discriminated
input shape (saved | preview). Library list keeps using the saved branch;
the upcoming builder preview mode will use the preview branch. Pure
refactor — no visual change.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 9: Extract `StatusTemplateCard.svelte`

Same pattern, pulled from `StatusPaletteDock` / `StatusTemplateEditor` rendering context.

**Files:**
- Create: `src/lib/components/gm-screen/StatusTemplateCard.svelte`
- Modify: `src/lib/components/gm-screen/StatusPaletteDock.svelte`

- [ ] **Step 1: Identify the rendering site**

```bash
grep -n "StatusTemplate\|template-card\|.template" src/lib/components/gm-screen/StatusPaletteDock.svelte | head -20
```

- [ ] **Step 2: Create `StatusTemplateCard.svelte`**

Same discriminated input pattern as `AdvantageCard`:

```svelte
<script lang="ts">
  import type { StatusTemplate } from '$lib/../types';

  type SavedInput   = { kind: 'saved';   data: StatusTemplate };
  type PreviewInput = { kind: 'preview'; data: Omit<StatusTemplate, 'id'>; wip?: boolean };

  interface Props {
    input: SavedInput | PreviewInput;
    onClick?: (target: HTMLElement) => void;
  }

  let { input, onClick }: Props = $props();
  const data = $derived(input.data);
  const isPreview = $derived(input.kind === 'preview');
  const isWip = $derived(input.kind === 'saved' ? input.data.wip : (input.wip ?? false));
</script>

<!-- Paste existing template-card markup here, parameterized over `data` -->
<!-- Add `.preview { border-color: var(--accent-amber); ... }` -->
```

- [ ] **Step 3: Update StatusPaletteDock to use the extracted component**

Replace inline template-card markup with `<StatusTemplateCard input={{ kind: 'saved', data: tmpl }} />`.

- [ ] **Step 4: Run verify gate + smoke**

```bash
./scripts/verify.sh
```

`npm run tauri dev` → open GM Screen → check status palette renders identically.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/gm-screen/StatusTemplateCard.svelte src/lib/components/gm-screen/StatusPaletteDock.svelte
git commit -m "$(cat <<'EOF'
refactor(status): extract StatusTemplateCard.svelte from StatusPaletteDock

Mirror of AdvantageCard extraction. Discriminated input (saved | preview)
prepares the card for reuse in the upcoming builder preview mode.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 10: Land `ItemBuilderShell` chrome

Bare dialog with header, footer slot, body slot, `<dialog>` mechanics (showModal, oncancel, backdrop click, ESC), right-click input carve-out. No CardComposer yet.

**Files:**
- Create: `src/lib/components/builder/ItemBuilderShell.svelte`
- Modify: `src/routes/builder-test/+page.svelte` (smoke test for the shell)

- [ ] **Step 1: Create `ItemBuilderShell.svelte`**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    saveLabel?: string;
    valid?: boolean;
    busy?: boolean;
    error?: string | null;
    onSave: () => void | Promise<void>;
    onCancel: () => void;
    onDelete?: () => void | Promise<void>;
    onBack?: () => void;   // optional right-click handler
    children: Snippet;
    footer?: Snippet;       // optional override; default footer renders below
  }

  let {
    open, title, saveLabel = 'Save',
    valid = $bindable(true), busy = false, error = null,
    onSave, onCancel, onDelete, onBack,
    children, footer,
  }: Props = $props();

  let dialogEl: HTMLDialogElement;

  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) dialogEl.showModal();
    else if (!open && dialogEl.open) dialogEl.close();
  });

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onCancel();
  }

  function onContextMenu(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const editable =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable;
    if (editable) return; // preserve native Cut/Copy/Paste menu
    e.preventDefault();
    onBack?.();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      onSave();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="shell"
  oncancel={(e) => { e.preventDefault(); onCancel(); }}
  onclick={onBackdropClick}
  oncontextmenu={onContextMenu}
  onkeydown={onKeyDown}
>
  <header>
    <h3>{title}</h3>
    <button class="close" onclick={onCancel} aria-label="Cancel">×</button>
  </header>

  <div class="body">{@render children()}</div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if footer}
    {@render footer()}
  {:else}
    <footer>
      {#if onDelete}<button class="danger" onclick={onDelete} disabled={busy}>Delete</button>{/if}
      <span class="spacer"></span>
      <button class="secondary" onclick={onCancel}>Cancel</button>
      <button class="primary" onclick={onSave} disabled={!valid || busy}>
        {busy ? 'Saving…' : saveLabel}
      </button>
    </footer>
  {/if}
</dialog>

<style>
  .shell {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.5rem;
    padding: 0;
    width: min(36rem, calc(100vw - 2rem));
    max-height: calc(100vh - 4rem);
    color: var(--text-primary);
    box-shadow: 0 0.75rem 2rem -0.25rem rgba(0,0,0,0.7);
    box-sizing: border-box;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .shell::backdrop { background: rgba(0,0,0,0.55); }
  @starting-style { .shell[open] { transform: scale(0.96); } }
  .shell[open] { transition: transform 140ms ease-out; }
  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.7rem 0.85rem;
    border-bottom: 1px solid var(--border-faint);
  }
  header h3 { margin: 0; font-size: 0.9rem; color: var(--text-primary); }
  .close { background: transparent; border: none; color: var(--text-muted); font-size: 1rem; cursor: pointer; padding: 0; }
  .body { padding: 0; flex: 1; overflow-y: auto; }
  footer { display: flex; align-items: center; gap: 0.4rem; padding: 0.6rem 0.85rem; border-top: 1px solid var(--border-faint); background: rgba(0,0,0,0.15); }
  footer .spacer { flex: 1; }
  footer button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.25rem;
    padding: 0.3rem 0.7rem; font-size: 0.72rem; cursor: pointer;
    font-family: inherit;
  }
  footer button.primary { background: var(--accent); color: #fff; border-color: var(--accent-bright); }
  footer button.danger { background: transparent; color: var(--accent-amber); border-color: var(--accent-amber); }
  footer button[disabled] { opacity: 0.5; cursor: not-allowed; }
  .error { color: var(--accent-amber); font-size: 0.75rem; margin: 0 0.85rem; }
</style>
```

- [ ] **Step 2: Smoke test in `/builder-test`**

Append to `src/routes/builder-test/+page.svelte`:

```svelte
<script>
  // ...
  import ItemBuilderShell from '$lib/components/builder/ItemBuilderShell.svelte';
  let shellOpen = $state(false);
</script>

<section style="margin-top: 2rem;">
  <h2>ItemBuilderShell</h2>
  <button onclick={() => shellOpen = true}>Open shell</button>
  <ItemBuilderShell
    open={shellOpen}
    title="Smoke test"
    onSave={() => { alert('Save'); shellOpen = false; }}
    onCancel={() => { shellOpen = false; }}
  >
    <div style="padding: 1rem;">Hello from inside the shell. Ctrl+Enter saves. Esc cancels.</div>
  </ItemBuilderShell>
</section>
```

- [ ] **Step 3: Run verify gate + smoke**

```bash
./scripts/verify.sh
```

`npm run tauri dev` → navigate to `/builder-test` → click "Open shell" → verify: dialog opens centered, ESC closes, backdrop click closes, Ctrl+Enter alerts "Save", right-click on body text shows native context menu (input carve-out), right-click on dialog body (non-text area) is no-op (no onBack handler provided yet).

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/ItemBuilderShell.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add ItemBuilderShell with native <dialog> chrome

Shell renders title bar, body via default snippet, default footer (or
override via `footer` snippet). Native showModal for top-layer rendering
(transform-ancestor safe). Backdrop click + ESC + right-click route to
back/cancel handlers; right-click is carved out for editable text targets
so native Cut/Copy/Paste survives.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 11: Land `CardComposer` zone layout + CSS tokens

Top zone + spotlight zone + footer skeleton inside a Composer wrapper. Per-field CSS tokens added to `+layout.svelte`. No slot states yet — just static layout with placeholder rows.

**Files:**
- Create: `src/lib/components/builder/CardComposer.svelte`
- Modify: `src/routes/+layout.svelte` (add per-field color tokens)
- Modify: `src/routes/builder-test/+page.svelte`

- [ ] **Step 1: Add per-field tokens to `+layout.svelte` `:root`**

```css
:root {
  /* ...existing tokens... */
  --field-name:     #d9a560;
  --field-type:     #c46050;
  --field-level:    #c08850;
  --field-category: #88aabb;
  --field-desc:     #c4b7a4;
  --field-bonus:    #e0563a;
  --field-tags:     #98c89f;
}
```

- [ ] **Step 2: Create `CardComposer.svelte` (zone skeleton only)**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    spotlight: Snippet;
    cardRows: Snippet;
    queueLabel?: string;
    queueValues?: string[];
  }

  let { spotlight, cardRows, queueLabel = 'up next', queueValues = [] }: Props = $props();
</script>

<div class="composer">
  <div class="card-preview">
    <div class="card-preview-label">— card preview —</div>
    {@render cardRows()}
  </div>

  <div class="spotlight">
    {@render spotlight()}
  </div>

  {#if queueValues.length > 0}
    <div class="queue">
      <span class="queue-label">{queueLabel}</span>
      <span class="queue-list">
        {#each queueValues as v, i}
          {#if i > 0}<span class="queue-arrow">→</span>{/if}
          <span class="queue-item">{v}</span>
        {/each}
      </span>
    </div>
  {/if}
</div>

<style>
  .composer { display: flex; flex-direction: column; }
  .card-preview {
    padding: 0.85rem 0.9rem;
    background: linear-gradient(180deg, rgba(217,165,96,0.04) 0%, rgba(217,165,96,0.01) 100%);
    border-bottom: 1px solid var(--border-faint);
    display: flex; flex-direction: column; gap: 0.5rem;
  }
  .card-preview-label {
    font-size: 0.55rem; color: var(--text-muted);
    letter-spacing: 0.1em; text-transform: uppercase;
    text-align: center;
  }
  .spotlight {
    padding: 1.2rem 1rem;
    background: radial-gradient(ellipse at center, rgba(217, 165, 96, 0.08) 0%, transparent 65%);
    border-bottom: 1px solid var(--border-faint);
    display: flex; flex-direction: column; gap: 0.7rem;
    position: relative;
    min-height: 8rem;
  }
  .spotlight::before {
    content: ''; position: absolute; inset: 0.4rem;
    border: 1px solid rgba(217, 165, 96, 0.18);
    border-radius: 0.3rem; pointer-events: none;
  }
  .queue {
    padding: 0.4rem 0.8rem;
    display: flex; align-items: center; justify-content: space-between;
    background: rgba(0,0,0,0.2);
    font-size: 0.62rem;
  }
  .queue-label { color: var(--text-muted); letter-spacing: 0.08em; text-transform: uppercase; }
  .queue-list { display: flex; gap: 0.4rem; align-items: center; color: var(--text-secondary); }
  .queue-arrow { color: var(--text-muted); }
  .queue-item { color: var(--text-label); }
</style>
```

- [ ] **Step 3: Smoke test**

In `/builder-test`, demo CardComposer inside the shell with placeholder content for both snippets. Verify layout matches mockup 07.

- [ ] **Step 4: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/CardComposer.svelte src/routes/+layout.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add CardComposer zone layout + per-field CSS tokens

Card preview zone (top), spotlight zone (middle), optional queue strip
(bottom). Per-field color tokens declared as fresh tokens in :root
(not aliases) so future per-field rebranding stays isolated from
shared accents. Slot states + stage machine land in next tasks.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 12: Land slot state styling

Empty / active / confirmed visual treatments. Per-field color application. Pulsing-amber active outline animation. Static rendering — no state machine yet.

**Files:**
- Modify: `src/lib/components/builder/CardComposer.svelte` (add slot subcomponent + styles)
- Create: `src/lib/components/builder/CardSlot.svelte`

- [ ] **Step 1: Create `CardSlot.svelte`**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  type FieldKey = 'name' | 'type' | 'level' | 'category' | 'desc' | 'bonus' | 'tags';
  type SlotState = 'empty' | 'active' | 'confirmed';

  interface Props {
    field: FieldKey;
    state: SlotState;
    keyLabel: string;
    onClick?: () => void;
    children: Snippet;
  }

  let { field, state, keyLabel, onClick, children }: Props = $props();
</script>

<div
  class="slot {field} state-{state}"
  onclick={state === 'confirmed' && onClick ? onClick : undefined}
  role={state === 'confirmed' ? 'button' : undefined}
  tabindex={state === 'confirmed' ? 0 : undefined}
>
  <span class="slot-key">{keyLabel}</span>
  <span class="slot-val">{@render children()}</span>
</div>

<style>
  .slot {
    padding: 0.35rem 0.55rem;
    border-radius: 0.3rem;
    display: flex; align-items: center; gap: 0.4rem;
    font-size: 0.72rem; min-height: 1.8rem;
    border: 1px solid transparent;
    box-sizing: border-box;
    transition: all 0.16s;
  }
  .slot-key {
    font-size: 0.56rem; color: var(--text-muted);
    letter-spacing: 0.08em; text-transform: uppercase;
    flex-shrink: 0;
  }
  .slot-val {
    font-family: "Iowan Old Style", "Garamond", "Palatino Linotype", serif;
    font-weight: 600;
    flex: 1; min-width: 0;
  }

  /* Empty */
  .slot.state-empty {
    background: rgba(0,0,0,0.18);
    border: 1px dashed var(--border-surface);
    color: var(--text-muted);
    font-style: italic;
    opacity: 0.5;
  }
  .slot.state-empty .slot-val { color: var(--text-muted); font-weight: 400; }

  /* Active — pulsing amber */
  .slot.state-active {
    border: 1px solid var(--accent-amber);
    background: rgba(217, 165, 96, 0.06);
    animation: pulse-border 1.4s ease-in-out infinite;
  }
  .slot.state-active .slot-key { color: var(--accent-amber); }
  .slot.state-active .slot-val { color: var(--accent-amber); font-style: italic; font-weight: 500; }
  @keyframes pulse-border {
    0%, 100% { box-shadow: 0 0 0 0 rgba(217, 165, 96, 0.5); }
    50%      { box-shadow: 0 0 0 4px rgba(217, 165, 96, 0); }
  }

  /* Confirmed — per-field colors */
  .slot.state-confirmed { cursor: pointer; }
  .slot.state-confirmed:hover { transform: scale(1.01); }
  .slot.name.state-confirmed     { background: rgba(217, 165, 96, 0.13); border-color: var(--field-name);     color: var(--field-name); }
  .slot.type.state-confirmed     { background: rgba(196, 96, 80, 0.13);  border-color: var(--field-type);     color: var(--field-type); }
  .slot.level.state-confirmed    { background: rgba(192, 136, 80, 0.13); border-color: var(--field-level);    color: var(--field-level); }
  .slot.category.state-confirmed { background: rgba(136, 170, 187, 0.13); border-color: var(--field-category); color: var(--field-category); }
  .slot.desc.state-confirmed     { background: rgba(196, 183, 164, 0.1);  border-color: var(--field-desc);     color: var(--field-desc); }
  .slot.bonus.state-confirmed    { background: rgba(224, 86, 58, 0.12);  border-color: var(--field-bonus);    color: var(--field-bonus); }
  .slot.tags.state-confirmed     { background: rgba(152, 200, 159, 0.13); border-color: var(--field-tags);    color: var(--field-tags); }
</style>
```

- [ ] **Step 2: Smoke test slots in `/builder-test`**

In the `builder-test` route, demo a card-shape grid of slots in all three states. Verify per-field colors render correctly and the active slot pulses.

- [ ] **Step 3: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/CardSlot.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add CardSlot with empty/active/confirmed visual states

Per-field colors applied via .slot.{field}.state-confirmed class
combination. Active state uses pulsing-amber outline animation. Empty
is dim dashed. Click on confirmed slot fires onClick (re-edit affordance
hookup happens when stage machine lands).

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 13: Land stage state machine (TDD)

Tests REQUIRED per spec. Tab order ↔ spatial layout decoupling, confirmedFields Set, advance/retreat logic.

**Files:**
- Create: `src/lib/builder/stage-machine.ts`
- Create: `src/lib/builder/stage-machine.test.ts`

- [ ] **Step 1: Write failing tests**

`src/lib/builder/stage-machine.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { createStageMachine, type StageMachine } from './stage-machine';

const tabOrder = ['name', 'type', 'level', 'bonus', 'description', 'tags'] as const;
const required = ['name', 'bonus'] as const;

function fresh(): StageMachine {
  return createStageMachine({ tabOrder: [...tabOrder], required: [...required] });
}

describe('stage machine', () => {
  it('starts at the first stage with no confirmed fields', () => {
    const m = fresh();
    expect(m.stage).toBe('name');
    expect(m.confirmed.size).toBe(0);
  });

  it('advance() marks current confirmed and moves to next', () => {
    const m = fresh();
    m.advance();
    expect(m.confirmed.has('name')).toBe(true);
    expect(m.stage).toBe('type');
  });

  it('retreat() does NOT unconfirm previous; just moves the cursor', () => {
    const m = fresh();
    m.advance();   // name confirmed, stage=type
    m.advance();   // type confirmed, stage=level
    m.retreat();   // stage=type, still confirmed
    expect(m.stage).toBe('type');
    expect(m.confirmed.has('name')).toBe(true);
    expect(m.confirmed.has('type')).toBe(true);
  });

  it('setStage(key) jumps to that stage and removes it from confirmed', () => {
    const m = fresh();
    m.advance(); m.advance(); m.advance();   // name, type, level confirmed; stage=bonus
    m.setStage('name');
    expect(m.stage).toBe('name');
    expect(m.confirmed.has('name')).toBe(false);
    expect(m.confirmed.has('type')).toBe(true);  // subsequent stays confirmed (no cascade)
    expect(m.confirmed.has('level')).toBe(true);
  });

  it('isComplete() returns true only when all required fields are confirmed', () => {
    const m = fresh();
    expect(m.isComplete()).toBe(false);
    m.advance();  // name
    expect(m.isComplete()).toBe(false);
    m.advance();  // type — but not required
    m.advance();  // level — not required
    m.advance();  // bonus — required
    expect(m.isComplete()).toBe(true);
  });

  it('seedFromConfirmed() places stage at first non-confirmed in tabOrder', () => {
    const m = createStageMachine({
      tabOrder: [...tabOrder],
      required: [...required],
      initialConfirmed: ['name', 'type'],
    });
    expect(m.stage).toBe('level');
    expect(m.confirmed.has('name')).toBe(true);
    expect(m.confirmed.has('type')).toBe(true);
  });

  it('seedFromConfirmed all required → review stage', () => {
    const m = createStageMachine({
      tabOrder: [...tabOrder],
      required: [...required],
      initialConfirmed: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
    });
    expect(m.stage).toBe('review');
    expect(m.isComplete()).toBe(true);
  });

  it('advance past last tabOrder entry → review stage', () => {
    const m = fresh();
    for (let i = 0; i < tabOrder.length; i++) m.advance();
    expect(m.stage).toBe('review');
  });

  it('retreat from first stage stays at first', () => {
    const m = fresh();
    m.retreat();
    expect(m.stage).toBe('name');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npm run check
```

Expected: TypeScript error — `stage-machine` module not found.

- [ ] **Step 3: Implement `stage-machine.ts`**

```ts
export type Stage = string;

export interface StageMachineConfig {
  tabOrder: Stage[];
  required: Stage[];
  initialConfirmed?: Stage[];
}

export interface StageMachine {
  stage: Stage;
  confirmed: Set<Stage>;
  advance(): void;
  retreat(): void;
  setStage(s: Stage): void;
  isComplete(): boolean;
}

export function createStageMachine(cfg: StageMachineConfig): StageMachine {
  const tabOrder = cfg.tabOrder;
  const required = new Set(cfg.required);
  const confirmed = new Set<Stage>(cfg.initialConfirmed ?? []);

  // Find initial stage: first tabOrder entry not in confirmed, or 'review' if all required confirmed
  function firstUnconfirmed(): Stage {
    for (const s of tabOrder) if (!confirmed.has(s)) return s;
    return 'review';
  }

  let stage: Stage =
    [...required].every(r => confirmed.has(r))
      ? (cfg.initialConfirmed && cfg.initialConfirmed.length === tabOrder.length ? 'review' : firstUnconfirmed())
      : firstUnconfirmed();

  // Re-resolve: if all required confirmed AND every tabOrder confirmed, jump to review
  if ([...tabOrder].every(s => confirmed.has(s))) stage = 'review';

  const machine: StageMachine = {
    get stage() { return stage; },
    set stage(s: Stage) { stage = s; },
    confirmed,

    advance() {
      // Mark current stage confirmed (unless we're already at review)
      if (stage !== 'review') confirmed.add(stage);
      // Move to next in tabOrder, or review
      const idx = tabOrder.indexOf(stage);
      if (idx === -1 || idx >= tabOrder.length - 1) {
        stage = 'review';
      } else {
        stage = tabOrder[idx + 1];
      }
    },

    retreat() {
      const idx = tabOrder.indexOf(stage);
      if (idx > 0) {
        stage = tabOrder[idx - 1];
      } else if (stage === 'review') {
        // Retreat from review to last tabOrder entry
        stage = tabOrder[tabOrder.length - 1];
      }
      // else: stay at first stage
    },

    setStage(s: Stage) {
      stage = s;
      confirmed.delete(s);
    },

    isComplete() {
      return [...required].every(r => confirmed.has(r));
    },
  };

  return machine;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/lib/builder/stage-machine.test.ts
```

Expected: all PASS.

- [ ] **Step 5: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/builder/stage-machine.ts src/lib/builder/stage-machine.test.ts
git commit -m "$(cat <<'EOF'
feat(builder): add stage state machine with confirmed-fields tracking

Pure UI state — never persisted. Tab order decoupled from spatial card
layout. advance() marks current stage confirmed and moves cursor.
retreat() moves cursor without un-confirming (preserves Shift+Tab
ergonomic). setStage(s) jumps + un-confirms s only (no cascade).
Reopen path: pass initialConfirmed to seed from DB confirmed_fields.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 14: Land building↔preview mode transition

CardComposer detects completion (`machine.isComplete()`) and cross-fades from slot scaffolding to the real card preview. Uses extracted `AdvantageCard.svelte` / `StatusTemplateCard.svelte` for the preview render.

**Files:**
- Modify: `src/lib/components/builder/CardComposer.svelte`

- [ ] **Step 1: Add mode prop and conditional rendering**

Extend `CardComposer.svelte` props:

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    spotlight: Snippet;
    cardRows: Snippet;   // building-mode slot rows
    preview?: Snippet;   // preview-mode rendered card (optional — bodies without preview omit)
    isComplete: boolean; // from stage machine
    queueLabel?: string;
    queueValues?: string[];
  }

  let { spotlight, cardRows, preview, isComplete, queueLabel = 'up next', queueValues = [] }: Props = $props();
</script>
```

In the markup, replace the static card-preview section with a conditional:

```svelte
<div class="card-preview" class:complete={isComplete && preview}>
  <div class="card-preview-label">— {isComplete && preview ? 'preview · as it will appear' : 'card preview'} —</div>
  {#if isComplete && preview}
    <div class="preview-wrap" out:fade={{ duration: 90 }} in:fade={{ duration: 180 }}>
      {@render preview()}
    </div>
  {:else}
    <div out:fade={{ duration: 90 }} in:fade={{ duration: 180 }}>
      {@render cardRows()}
    </div>
  {/if}
</div>
```

Add at top of `<script>`:

```ts
import { fade } from 'svelte/transition';
```

- [ ] **Step 2: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/builder/CardComposer.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add building↔preview mode transition

When isComplete fires true and a preview snippet is provided, the
top zone cross-fades from slot scaffolding to the real card preview.
Bodies without a preview (ModifierEffectBody) omit the preview prop
and the top zone stays in building mode permanently.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 15: Land `AdvantageBody` — name / type / level stages

First half of AdvantageBody. Identity-cluster stages with Latin placeholder integration and tier dot input.

**Files:**
- Create: `src/lib/components/builder/bodies/AdvantageBody.svelte` (partial — name/type/level only)

- [ ] **Step 1: Create `AdvantageBody.svelte`**

```svelte
<script lang="ts">
  import { createStageMachine } from '$lib/builder/stage-machine';
  import { pickPlaceholder } from '$lib/builder/latin-placeholders';
  import CardComposer from '../CardComposer.svelte';
  import CardSlot from '../CardSlot.svelte';
  import type { Advantage, AdvantageKind } from '$lib/../types';

  export interface AdvantageBodyBonus {
    kind: 'pool' | 'difficulty' | 'auto_success';
    bonusName: string;
    value: number;
    paths: string[];
  }

  export interface AdvantageBodyState {
    name: string;
    kind: AdvantageKind;
    level: number;
    tags: string[];
    description: string;
    bonus: AdvantageBodyBonus;
    confirmedFields: string[];  // exported snapshot of stage machine confirmed set
  }

  interface Props {
    initial?: Advantage;
    existingCount?: number;
    valid?: boolean;
    state?: AdvantageBodyState;
  }

  let {
    initial,
    existingCount = 0,
    valid = $bindable(false),
    state = $bindable<AdvantageBodyState>(emptyState(initial, existingCount)),
  }: Props = $props();

  function emptyState(init: Advantage | undefined, count: number): AdvantageBodyState {
    return {
      name: init?.name ?? pickPlaceholder(count),
      kind: init?.kind ?? 'merit',
      level: Number((init?.properties.find(p => p.name === 'dot_cost_per_level' && p.type === 'number')?.value) ?? 1),
      tags: init ? [...init.tags] : [],
      description: init?.description ?? '',
      bonus: {},
    };
  }

  const machine = createStageMachine({
    tabOrder: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
    required: ['name', 'bonus'],
    initialConfirmed: initial?.confirmedFields ?? undefined,
  });

  $effect(() => {
    valid = machine.isComplete();
  });

  // Stage-specific UI snippets pulled below as `{#snippet}`s
</script>

{#snippet cardRows()}
  <div class="card-row two-col">
    <CardSlot field="name" state={slotStateFor('name')} keyLabel="Name" onClick={() => machine.setStage('name')}>
      {state.name}
    </CardSlot>
    <CardSlot field="type" state={slotStateFor('type')} keyLabel="Type" onClick={() => machine.setStage('type')}>
      {kindLabel(state.kind)}
    </CardSlot>
  </div>
  <div class="card-row two-col">
    <CardSlot field="level" state={slotStateFor('level')} keyLabel="Level" onClick={() => machine.setStage('level')}>
      <span class="tier-dots">
        {#each [1,2,3,4,5] as n}
          <span class="tier-pip" class:f={state.level >= n}></span>
        {/each}
      </span>
    </CardSlot>
    <CardSlot field="category" state="empty" keyLabel="Category">n/a (merits)</CardSlot>
  </div>
  <!-- description, bonus, tags rows added in Task 17 -->
{/snippet}

{#snippet spotlight()}
  {#if machine.stage === 'name'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Name this merit</h3>
    <input
      class="big-input"
      type="text"
      bind:value={state.name}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); machine.advance(); } }}
      autofocus
    />
    <p class="hint">Tab or Enter accepts placeholder · Shift+Tab goes back</p>
  {:else if machine.stage === 'type'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">What kind?</h3>
    <div class="kind-row">
      {#each ['merit', 'flaw', 'background', 'boon'] as k}
        <button
          class:selected={state.kind === k}
          onclick={() => { state.kind = k as AdvantageKind; machine.advance(); }}
        >{kindLabel(k as AdvantageKind)}</button>
      {/each}
    </div>
  {:else if machine.stage === 'level'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Tier level</h3>
    <div class="tier-row">
      {#each [1,2,3,4,5] as n}
        <button
          class="tier-pip-button"
          class:f={state.level >= n}
          onclick={() => { state.level = n; machine.advance(); }}
        ></button>
      {/each}
    </div>
  {/if}
  <!-- bonus, description, tags spotlight added in Tasks 16, 17 -->
{/snippet}

<CardComposer
  {cardRows}
  {spotlight}
  isComplete={machine.isComplete()}
  queueValues={queueRemaining()}
/>

<script context="module" lang="ts">
  // (top-level helpers shared across instances)
</script>

<script lang="ts">
  function slotStateFor(field: string): 'empty' | 'active' | 'confirmed' {
    if (machine.stage === field) return 'active';
    if (machine.confirmed.has(field)) return 'confirmed';
    return 'empty';
  }

  function kindLabel(k: AdvantageKind): string {
    return ({ merit: 'Merit', flaw: 'Flaw', background: 'Background', boon: 'Boon' })[k];
  }

  function queueRemaining(): string[] {
    return ['bonus', 'description', 'tags']
      .filter(s => s !== machine.stage && !machine.confirmed.has(s));
  }
</script>

<style>
  .card-row { display: grid; gap: 0.45rem; }
  .card-row.two-col { grid-template-columns: 1fr 1fr; }
  .tier-dots { display: inline-flex; gap: 0.18rem; }
  .tier-pip {
    width: 0.55rem; height: 0.55rem; border-radius: 50%;
    border: 1px solid var(--border-surface);
  }
  .tier-pip.f { background: var(--field-level); border-color: var(--field-level); box-shadow: 0 0 3px rgba(192, 136, 80, 0.5); }
  .prompt-label { font-size: 0.6rem; color: var(--accent-amber); letter-spacing: 0.1em; text-transform: uppercase; text-align: center; }
  .prompt { margin: 0; font-family: "Iowan Old Style", "Garamond", serif; font-size: 1.1rem; color: var(--text-primary); text-align: center; font-weight: 600; }
  .big-input {
    background: var(--bg-input);
    border: 1px solid var(--accent-amber);
    border-radius: 0.3rem;
    padding: 0.55rem 0.75rem;
    font-size: 1.05rem;
    color: var(--text-primary);
    font-family: "Iowan Old Style", "Garamond", serif;
    box-sizing: border-box;
    width: 100%;
    outline: none;
  }
  .hint { font-size: 0.65rem; color: var(--text-muted); text-align: center; font-style: italic; margin: 0; }
  .kind-row, .tier-row { display: flex; gap: 0.45rem; justify-content: center; }
  .kind-row button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.3rem;
    padding: 0.45rem 0.85rem; font-size: 0.85rem; cursor: pointer;
    font-family: inherit;
  }
  .kind-row button.selected { background: var(--field-type); color: #fff; border-color: var(--field-type); }
  .tier-pip-button {
    width: 1.8rem; height: 1.8rem; border-radius: 50%;
    border: 1px solid var(--border-surface);
    background: transparent; cursor: pointer; padding: 0;
    transition: transform 0.12s;
  }
  .tier-pip-button:hover { transform: scale(1.1); border-color: var(--accent); }
  .tier-pip-button.f { background: var(--field-level); border-color: var(--field-level); box-shadow: 0 0 4px rgba(192, 136, 80, 0.6); }
</style>
```

- [ ] **Step 2: Run verify gate**

```bash
./scripts/verify.sh
```

This task does NOT migrate AdvantagesManager yet — AdvantageBody is incomplete. Verify it builds; smoke-test via builder-test route.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/builder/bodies/AdvantageBody.svelte
git commit -m "$(cat <<'EOF'
feat(builder): AdvantageBody name/type/level stages

First half of AdvantageBody. Name stage uses Latin placeholder
(deterministic by existing-count). Type stage is 4-button kind picker.
Level stage is 1-5 tier dot row. Stage machine drives the spotlight
content; CardSlot tracks per-stage state for the card preview rows.
Bonus / description / tags lands in Tasks 16, 17.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 16: Land `AdvantageBody` — bonus stage (TDD for bonus_name auto-derivation)

Sub-composer for the bonus stage: type cards (Pool / Difficulty / AutoSuccess) → bonus name → ValueSelector → AppliesToPicker. Auto-derived `[merit name] bonus`, sticky on edit. Tests REQUIRED.

**Files:**
- Modify: `src/lib/components/builder/bodies/AdvantageBody.svelte` (add bonus stage)
- Create: `src/lib/builder/bonus-name.ts` (helper with the auto-derive logic)
- Create: `src/lib/builder/bonus-name.test.ts`

- [ ] **Step 1: Write failing tests**

`src/lib/builder/bonus-name.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { deriveBonusName, isAutoDerived } from './bonus-name';

describe('deriveBonusName', () => {
  it('returns "[name] bonus" for a non-empty name', () => {
    expect(deriveBonusName('Sanguine Tenacity')).toBe('Sanguine Tenacity bonus');
  });

  it('returns empty string for empty name', () => {
    expect(deriveBonusName('')).toBe('');
  });

  it('trims whitespace before deriving', () => {
    expect(deriveBonusName('  Iron Will  ')).toBe('Iron Will bonus');
  });
});

describe('isAutoDerived', () => {
  it('matches when current is exactly "[merit name] bonus"', () => {
    expect(isAutoDerived('Sanguine Tenacity bonus', 'Sanguine Tenacity')).toBe(true);
  });

  it('does not match when user has edited', () => {
    expect(isAutoDerived('Custom Name', 'Sanguine Tenacity')).toBe(false);
  });

  it('matches empty bonus name + empty merit name', () => {
    expect(isAutoDerived('', '')).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
npx vitest run src/lib/builder/bonus-name.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 3: Implement `bonus-name.ts`**

```ts
export function deriveBonusName(meritName: string): string {
  const trimmed = meritName.trim();
  if (trimmed === '') return '';
  return `${trimmed} bonus`;
}

export function isAutoDerived(currentBonusName: string, meritName: string): boolean {
  return currentBonusName === deriveBonusName(meritName);
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
npx vitest run src/lib/builder/bonus-name.test.ts
```

Expected: PASS.

- [ ] **Step 5: Add bonus stage to AdvantageBody**

In `src/lib/components/builder/bodies/AdvantageBody.svelte` (the `AdvantageBodyBonus` interface was declared in Task 15):

- In `emptyState`, initialize `bonus: { kind: 'pool', bonusName: deriveBonusName(initial?.name ?? pickPlaceholder(count)), value: 1, paths: [] }`.
- Add an `$effect` that keeps bonus.bonusName synced while it's auto-derived:

```ts
let bonusNameSticky = $state(false);
$effect(() => {
  if (!bonusNameSticky) state.bonus.bonusName = deriveBonusName(state.name);
});
function onBonusNameInput(v: string) {
  bonusNameSticky = true;
  state.bonus.bonusName = v;
}
```

- Add spotlight branch for the bonus stage. It's a multi-step sub-flow inside one stage: pick type → enter name → pick value → pick applies-to. Use local `$state` for sub-stage cursor within the bonus stage:

```svelte
{:else if machine.stage === 'bonus'}
  {#if bonusSubStage === 'type'}
    <h3 class="prompt">What kind of bonus?</h3>
    <div class="bonus-type-grid">
      <button onclick={() => { state.bonus.kind = 'pool';        bonusSubStage = 'name'; }}>⚂ Dice Pool</button>
      <button onclick={() => { state.bonus.kind = 'difficulty'; bonusSubStage = 'name'; }}>↧ Difficulty</button>
      <button onclick={() => { state.bonus.kind = 'auto_success'; bonusSubStage = 'name'; }}>✓ Auto Successes</button>
    </div>
  {:else if bonusSubStage === 'name'}
    <h3 class="prompt">Bonus name</h3>
    <input class="big-input" type="text" value={state.bonus.bonusName} oninput={(e) => onBonusNameInput(e.currentTarget.value)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); bonusSubStage = 'value'; } }} autofocus />
    <p class="hint">Default: "{deriveBonusName(state.name)}" · Tab accepts</p>
  {:else if bonusSubStage === 'value'}
    <h3 class="prompt">How many {state.bonus.kind === 'difficulty' ? '(difficulty steps)' : state.bonus.kind === 'auto_success' ? 'auto successes' : 'dice'}?</h3>
    <ValueSelector value={state.bonus.value} onChange={(n) => state.bonus.value = n} />
    <button class="next-btn" onclick={() => bonusSubStage = 'applies'}>Next: applies to →</button>
  {:else if bonusSubStage === 'applies'}
    <h3 class="prompt">Applies to which rolls?</h3>
    <AppliesToPicker paths={state.bonus.paths} onChange={(p) => state.bonus.paths = p} />
    <button class="next-btn" onclick={() => machine.advance()}>Confirm bonus →</button>
  {/if}
{/if}
```

Add `let bonusSubStage = $state<'type' | 'name' | 'value' | 'applies'>('type');` to the script.

Add the corresponding `bonus` card row inside `{#snippet cardRows()}`:

```svelte
<div class="card-row one-col">
  <CardSlot field="bonus" state={slotStateFor('bonus')} keyLabel="Bonus" onClick={() => { machine.setStage('bonus'); bonusSubStage = 'type'; }}>
    {#if machine.confirmed.has('bonus')}
      <span class="bonus-line">
        <strong>+{state.bonus.value}</strong> {bonusKindLabel(state.bonus.kind)} when making
        <em>{state.bonus.paths.length === 0 ? 'all' : appliesToLabel(state.bonus.paths)}</em> rolls
      </span>
    {:else}
      —
    {/if}
  </CardSlot>
</div>
```

Imports added:
```ts
import ValueSelector from '../ValueSelector.svelte';
import AppliesToPicker from '../AppliesToPicker.svelte';
import { deriveBonusName, isAutoDerived } from '$lib/builder/bonus-name';
```

- [ ] **Step 6: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 7: Commit**

```bash
git add src/lib/builder/bonus-name.ts src/lib/builder/bonus-name.test.ts src/lib/components/builder/bodies/AdvantageBody.svelte
git commit -m "$(cat <<'EOF'
feat(builder): AdvantageBody bonus stage with sub-composer + auto-name

Bonus stage is a sub-flow: type → name → value → applies-to. Bonus
name defaults to "[merit name] bonus" and stays synced until the user
types — once edited, the link breaks (sticky). ValueSelector wired
for magnitude; AppliesToPicker for scope. Auto-derivation logic and
sticky-on-edit detection are pure functions in bonus-name.ts (TDD).

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 17: Land `AdvantageBody` — description / tags + integration

Complete the remaining stages, wire up body config export, finalize validity rule.

**Files:**
- Modify: `src/lib/components/builder/bodies/AdvantageBody.svelte`

- [ ] **Step 1: Add description + tags stages to spotlight**

```svelte
{:else if machine.stage === 'description'}
  <h3 class="prompt">Describe {state.name}</h3>
  <textarea
    class="big-textarea"
    bind:value={state.description}
    placeholder="Optional. A line or two of flavor."
    onkeydown={(e) => { if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); machine.advance(); } }}
    autofocus
  ></textarea>
  <p class="hint">Tab to confirm (empty is fine)</p>
{:else if machine.stage === 'tags'}
  <h3 class="prompt">Tag this merit</h3>
  <TagChipInput tags={state.tags} onChange={(t) => state.tags = t} autofocus />
  <p class="hint">Tab confirms · empty is fine</p>
{:else if machine.stage === 'review'}
  <h3 class="prompt review-prompt">{state.name} is ready.</h3>
  <p class="hint">Click any slot above to re-edit · Ctrl+Enter to save</p>
{/if}
```

Add card rows for description + tags (parallel pattern):

```svelte
<div class="card-row one-col">
  <CardSlot field="desc" state={slotStateFor('description')} keyLabel="Description" onClick={() => machine.setStage('description')}>
    {state.description || '—'}
  </CardSlot>
</div>
<div class="card-row one-col">
  <CardSlot field="tags" state={slotStateFor('tags')} keyLabel="Tags" onClick={() => machine.setStage('tags')}>
    {#each state.tags as t}
      <span class="mini-tag">{t}</span>
    {/each}
    {#if state.tags.length === 0}—{/if}
  </CardSlot>
</div>
```

- [ ] **Step 2: Add preview snippet wired to AdvantageCard**

```svelte
{#snippet preview()}
  <AdvantageCard input={{ kind: 'preview', data: toAdvantageInput(state) }} onClick={onPreviewClick} />
{/snippet}

<CardComposer {cardRows} {spotlight} {preview} isComplete={machine.isComplete()} queueValues={queueRemaining()} />
```

Helper functions:
```ts
function toAdvantageInput(s: AdvantageBodyState) {
  return {
    name: s.name,
    description: s.description,
    kind: s.kind,
    tags: s.tags,
    properties: [
      { name: 'dot_cost_per_level', type: 'number', value: s.level },
      // bonus → effects[] mapping happens in the save handler, not here
    ],
    wip: !machine.isComplete(),
    confirmedFields: [...machine.confirmed],
  };
}

function onPreviewClick(target: HTMLElement) {
  const slot = target.closest('[data-stage]') as HTMLElement | null;
  if (slot) machine.setStage(slot.dataset.stage as any);
}
```

(Add `data-stage="name"` etc. attributes to the corresponding markup in `AdvantageCard.svelte` so this delegation works. This requires a small update there.)

- [ ] **Step 3: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/bodies/AdvantageBody.svelte src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
feat(builder): complete AdvantageBody — description, tags, preview wiring

Description and tags stages added; review stage shows "X is ready"
prompt. Preview snippet renders AdvantageCard in preview mode, with
data-stage attributes for click-to-edit delegation. Body export
complete and ready for AdvantagesManager migration.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 18: Migrate `AdvantagesManager`

Replace inline `AdvantageForm` mount with `ItemBuilderShell` + `AdvantageBody`. The inline grid-expansion goes away. Save handler maps body state to existing `addAdvantage` / `updateAdvantage` IPC.

**Files:**
- Modify: `src/lib/components/AdvantagesManager.svelte`

- [ ] **Step 1: Update AdvantagesManager**

Replace the existing form mount block (somewhere near the top of the rendered grid) with:

```svelte
<script>
  // ...existing imports...
  import ItemBuilderShell from '$lib/components/builder/ItemBuilderShell.svelte';
  import AdvantageBody, { type AdvantageBodyState } from '$lib/components/builder/bodies/AdvantageBody.svelte';

  let bodyValid = $state(false);
  let bodyState = $state<AdvantageBodyState | undefined>();

  function bonusToEffect(b: AdvantageBodyBonus): ModifierEffect {
    return {
      kind: b.kind,                          // 'pool' | 'difficulty' | 'auto_success'
      scope: null,                            // semantic scope baked into paths via AppliesToPicker
      delta: b.value,
      note: null,
      paths: b.paths,
      bonusName: b.bonusName,
    };
  }

  async function handleSave() {
    if (!bodyState) return;
    const input: AdvantageInput = {
      name: bodyState.name,
      description: bodyState.description,
      kind: bodyState.kind,
      tags: bodyState.tags,
      properties: [{
        name: 'dot_cost_per_level',
        type: 'number',
        value: bodyState.level,
      }],
      effects: [bonusToEffect(bodyState.bonus)],
      wip: !bodyValid,
      confirmedFields: bodyValid ? null : bodyState.confirmedFields,
    };
    if (editingId !== null) await updateAdvantage(editingId, input);
    else                    await addAdvantage(input);
    showAddForm = false;
    editingId = null;
    await loadAll();
  }
</script>

{#if showAddForm || editingId !== null}
  <ItemBuilderShell
    open
    title={editingId === null ? 'New advantage' : 'Edit advantage'}
    saveLabel={bodyValid ? 'Save' : 'Save WIP'}
    bind:valid={bodyValid}
    onSave={handleSave}
    onCancel={() => { showAddForm = false; editingId = null; }}
  >
    <AdvantageBody initial={editingEntry} existingCount={advantages.length} bind:valid={bodyValid} bind:state={bodyState} />
  </ItemBuilderShell>
{/if}
```

Remove the now-unused `<AdvantageForm>` import and any inline grid-slot reservation for the form.

- [ ] **Step 2: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 3: Smoke test**

`npm run tauri dev` → Advantages tool → click "+ New advantage" → builder opens, stages walk Name → Type → Level → Bonus (sub-composer) → Description → Tags → Review. Save creates the advantage. Cancel discards. Click an existing advantage to edit → builder opens with seeded state.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/AdvantagesManager.svelte
git commit -m "$(cat <<'EOF'
feat(advantages): migrate AdvantagesManager to ItemBuilderShell

Replaces inline AdvantageForm grid-expansion with the unified shell.
Eliminates layout shift on form open. Save / Save WIP label-flips
based on completion. AdvantageBody handles all stage UX.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 19: Land `StatusTemplateBody` + migrate `StatusPaletteDock`

4-row layout (Name, Effects, Description, Tags). No level/category. Delete affordance in shell footer.

**Files:**
- Create: `src/lib/components/builder/bodies/StatusTemplateBody.svelte`
- Modify: `src/lib/components/gm-screen/StatusPaletteDock.svelte`

- [ ] **Step 1: Create `StatusTemplateBody.svelte`**

Mirror of AdvantageBody but with `tabOrder: ['name', 'effects', 'description', 'tags']` and a `cardLayout` of 4 rows. The Effects stage uses a sub-composer similar to AdvantageBody's Bonus stage but without bonus-naming (templates don't have per-bonus names — single shared `name` field on the template covers it). Multiple effects allowed (use TagChipInput pattern but for effect rows).

The preview snippet uses `StatusTemplateCard`:

```svelte
{#snippet preview()}
  <StatusTemplateCard input={{ kind: 'preview', data: toStatusTemplateInput(state) }} onClick={onPreviewClick} />
{/snippet}
```

- [ ] **Step 2: Update StatusPaletteDock**

Replace the existing `<div class="editor-overlay"><StatusTemplateEditor … /></div>` block with:

```svelte
<ItemBuilderShell
  open={editorOpen}
  title={editorExisting ? 'Edit template' : 'New template'}
  saveLabel={bodyValid ? 'Save' : 'Save WIP'}
  bind:valid={bodyValid}
  onSave={handleTemplateSave}
  onCancel={() => { editorOpen = false; editorExisting = null; }}
  onDelete={editorExisting ? handleTemplateDelete : undefined}
>
  <StatusTemplateBody initial={editorExisting} bind:valid={bodyValid} bind:state={templateState} />
</ItemBuilderShell>
```

Add the imports + state declarations. Remove `<StatusTemplateEditor>` import.

- [ ] **Step 3: Run verify gate + smoke**

```bash
./scripts/verify.sh
```

`npm run tauri dev` → GM Screen → click "+ New template" in StatusPaletteDock → builder opens. Walk through stages. Save creates the template. Delete (on edit) removes it.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/bodies/StatusTemplateBody.svelte src/lib/components/gm-screen/StatusPaletteDock.svelte
git commit -m "$(cat <<'EOF'
feat(status): migrate StatusPaletteDock to ItemBuilderShell + StatusTemplateBody

4-row layout (Name, Effects, Description, Tags) per spec. Multiple
effects supported via sub-composer. Delete affordance in shell footer.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 20: Land `ModifierEffectBody` + migrate `CharacterCard` / `CharacterRow`

2-row layout (Effects, Tags). No name, no preview mode.

**Files:**
- Create: `src/lib/components/builder/bodies/ModifierEffectBody.svelte`
- Modify: `src/lib/components/gm-screen/CharacterCard.svelte` (around line 876)
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (around line 527)

- [ ] **Step 1: Create `ModifierEffectBody.svelte`**

```svelte
<script lang="ts">
  import { createStageMachine } from '$lib/builder/stage-machine';
  import CardComposer from '../CardComposer.svelte';
  import CardSlot from '../CardSlot.svelte';
  import TagChipInput from '../TagChipInput.svelte';
  import ValueSelector from '../ValueSelector.svelte';
  import AppliesToPicker from '../AppliesToPicker.svelte';
  import type { ModifierEffect, BridgeCharacter } from '$lib/../types';

  export interface ModifierEffectBodyState {
    effects: ModifierEffect[];
    tags: string[];
  }

  interface Props {
    initialEffects: ModifierEffect[];
    initialTags: string[];
    character?: BridgeCharacter;
    valid?: boolean;
    state?: ModifierEffectBodyState;
  }

  let {
    initialEffects, initialTags, character,
    valid = $bindable(false),
    state = $bindable<ModifierEffectBodyState>({ effects: [...initialEffects], tags: [...initialTags] }),
  }: Props = $props();

  const machine = createStageMachine({
    tabOrder: ['effects', 'tags'],
    required: ['effects'],
  });

  $effect(() => { valid = machine.isComplete(); });
</script>

{#snippet cardRows()}
  <div class="card-row one-col">
    <CardSlot field="bonus" state={slotStateFor('effects')} keyLabel="Effects" onClick={() => machine.setStage('effects')}>
      {state.effects.length} effect{state.effects.length === 1 ? '' : 's'}
    </CardSlot>
  </div>
  <div class="card-row one-col">
    <CardSlot field="tags" state={slotStateFor('tags')} keyLabel="Tags" onClick={() => machine.setStage('tags')}>
      {#each state.tags as t}<span class="mini-tag">{t}</span>{/each}
      {#if state.tags.length === 0}—{/if}
    </CardSlot>
  </div>
{/snippet}

{#snippet spotlight()}
  {#if machine.stage === 'effects'}
    <h3 class="prompt">Edit effects</h3>
    {#each state.effects as effect, i}
      <div class="effect-row">
        <select value={effect.kind} onchange={(e) => state.effects[i].kind = (e.currentTarget as HTMLSelectElement).value as ModifierKind}>
          {#each ['pool', 'difficulty', 'note', 'auto_success'] as k}
            <option value={k}>{k}</option>
          {/each}
        </select>
        {#if effect.kind === 'note'}
          <input type="text" placeholder="Note" value={effect.note ?? ''} oninput={(e) => state.effects[i].note = (e.currentTarget as HTMLInputElement).value || null} />
        {:else}
          <ValueSelector value={effect.delta ?? 0} onChange={(n) => state.effects[i].delta = n} />
          <AppliesToPicker paths={effect.paths} onChange={(p) => state.effects[i].paths = p} />
        {/if}
        <button onclick={() => state.effects = state.effects.filter((_, idx) => idx !== i)} aria-label="Remove effect">×</button>
      </div>
    {/each}
    <button class="add-effect" onclick={() => state.effects = [...state.effects, { kind: 'pool', scope: null, delta: 1, note: null, paths: [], bonusName: null }]}>+ Add effect</button>
    <button class="next-btn" onclick={() => machine.advance()}>Confirm effects →</button>
  {:else if machine.stage === 'tags'}
    <h3 class="prompt">Tags</h3>
    <TagChipInput tags={state.tags} onChange={(t) => state.tags = t} autofocus />
  {/if}
{/snippet}

<!-- No preview snippet — bonus-character-modifier has no library card -->
<CardComposer {cardRows} {spotlight} isComplete={machine.isComplete()} />

<script lang="ts">
  function slotStateFor(field: string) {
    if (machine.stage === field) return 'active';
    if (machine.confirmed.has(field)) return 'confirmed';
    return 'empty';
  }
</script>
```

- [ ] **Step 2: Update CharacterCard.svelte**

Find the existing editor mount around line 876. Replace with:

```svelte
{#if editorTarget !== null}
  <ItemBuilderShell
    open
    title={editorTarget?.name ?? 'Edit modifier'}
    bind:valid={effectsValid}
    onSave={() => saveChipEditor(effectsState)}
    onCancel={closeChipEditor}
  >
    <ModifierEffectBody
      initialEffects={editorTarget?.effects ?? []}
      initialTags={editorTarget?.tags ?? []}
      {character}
      bind:valid={effectsValid}
      bind:state={effectsState}
    />
  </ItemBuilderShell>
{/if}
```

Remove the `<ModifierEffectEditor>` import. Add state declarations:
```ts
let effectsValid = $state(false);
let effectsState = $state<ModifierEffectBodyState | undefined>();
```

- [ ] **Step 3: Update CharacterRow.svelte (line ~527)**

Same change as CharacterCard.

- [ ] **Step 4: Run verify gate + smoke**

```bash
./scripts/verify.sh
```

`npm run tauri dev` → GM Screen → click a modifier chip on a CharacterCard → builder opens with effects + tags. The dialog renders correctly under transform ancestors (this is the transform-ancestor test case from the spec).

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/bodies/ModifierEffectBody.svelte src/lib/components/gm-screen/CharacterCard.svelte src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): migrate CharacterCard + CharacterRow modifier editors to ItemBuilderShell

2-row layout (Effects, Tags), no preview mode. Native <dialog>
showModal is transform-ancestor safe — fixes the popover position
bug that motivated this work.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 21: Land `TypePicker` + smoke route

Step 1 of the builder flow. 2×3 grid (Merit/Flaw/Background/Boon enabled, Discipline/Item preview-disabled). Smoke-tested via `/builder-test`.

**Files:**
- Create: `src/lib/components/builder/TypePicker.svelte`
- Modify: `src/routes/builder-test/+page.svelte`

- [ ] **Step 1: Create `TypePicker.svelte`**

```svelte
<script lang="ts">
  export type EnabledType = 'merit' | 'flaw' | 'background' | 'boon';
  export type PreviewType = 'discipline' | 'item';

  interface Props {
    available?: EnabledType[];
    preview?: PreviewType[];
    onPick: (type: EnabledType) => void;
  }

  let { available = ['merit', 'flaw', 'background', 'boon'], preview = ['discipline', 'item'], onPick }: Props = $props();

  const CARDS = {
    merit:      { glyph: 'M', title: 'Merit',      desc: 'A character advantage. Buys dots for an in-game bonus.' },
    flaw:       { glyph: 'F', title: 'Flaw',       desc: 'A character drawback. Awards experience.' },
    background: { glyph: 'B', title: 'Background', desc: 'External resources or connections.' },
    boon:       { glyph: '★', title: 'Boon',       desc: 'Owed favor between Kindred.' },
    discipline: { glyph: 'D', title: 'Discipline', desc: 'Vampiric power with school + levels.' },
    item:       { glyph: 'I', title: 'Item',       desc: 'Weapon, armor, or gear.' },
  } as const;
</script>

<div class="picker-grid">
  {#each available as t}
    <button class="picker-card" onclick={() => onPick(t)}>
      <span class="glyph">{CARDS[t].glyph}</span>
      <span class="text">
        <strong>{CARDS[t].title}</strong>
        <span class="desc">{CARDS[t].desc}</span>
      </span>
    </button>
  {/each}
  {#each preview as t}
    <button class="picker-card preview-disabled" disabled title="coming in v1.x — body needs design">
      <span class="glyph">{CARDS[t].glyph}</span>
      <span class="text">
        <strong>{CARDS[t].title}</strong>
        <span class="desc">{CARDS[t].desc}</span>
        <span class="pill">preview · needs design</span>
      </span>
    </button>
  {/each}
</div>

<style>
  .picker-grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: 0.6rem;
    padding: 0.85rem;
  }
  .picker-card {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    border-radius: 0.35rem;
    padding: 0.85rem 0.95rem;
    cursor: pointer;
    display: flex; align-items: flex-start; gap: 0.7rem;
    text-align: left;
    font-family: inherit;
    color: var(--text-primary);
  }
  .picker-card:hover:not([disabled]) {
    border-color: var(--accent);
    background: rgba(168, 80, 64, 0.08);
  }
  .picker-card.preview-disabled {
    opacity: 0.45;
    cursor: not-allowed;
    border-style: dashed;
  }
  .glyph {
    width: 2rem; height: 2rem; flex-shrink: 0;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    display: flex; align-items: center; justify-content: center;
    font-size: 1.05rem; font-weight: 600;
    color: var(--accent-amber);
    font-family: ui-serif, "Iowan Old Style", serif;
  }
  .text { display: flex; flex-direction: column; gap: 0.15rem; flex: 1; min-width: 0; }
  .text strong { font-size: 0.82rem; color: var(--text-primary); font-weight: 600; }
  .desc { font-size: 0.68rem; color: var(--text-muted); line-height: 1.4; }
  .pill {
    font-size: 0.55rem; color: var(--text-muted);
    background: rgba(0,0,0,0.25);
    padding: 0.08rem 0.4rem; border-radius: 999px;
    letter-spacing: 0.06em; text-transform: uppercase;
    align-self: flex-start; margin-top: 0.15rem;
  }
</style>
```

- [ ] **Step 2: Smoke test in `/builder-test`**

Append a TypePicker demo section that opens ItemBuilderShell with the picker inside.

- [ ] **Step 3: Run verify gate**

```bash
./scripts/verify.sh
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/builder/TypePicker.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
feat(builder): add TypePicker (Step 1) with 2x3 grid

Merit/Flaw/Background/Boon enabled. Discipline/Item disabled with
"preview · needs design" pill and tooltip. No production caller in
v1 — smoke route at /builder-test exercises it. Pre-built per the
project's roadmap pattern so a future Library home page can wire it
up without component re-work.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 22: Delete legacy editors

Remove the three superseded files and any now-dead CSS.

**Files:**
- Delete: `src/lib/components/AdvantageForm.svelte`
- Delete: `src/lib/components/gm-screen/ModifierEffectEditor.svelte`
- Delete: `src/lib/components/gm-screen/StatusTemplateEditor.svelte`
- Modify: `src/lib/components/AdvantagesManager.svelte` (remove `.editor-overlay` CSS if present)
- Modify: `src/lib/components/gm-screen/StatusPaletteDock.svelte` (remove `.editor-overlay` CSS)
- Modify: `src/lib/components/gm-screen/CharacterCard.svelte` (remove `.editor-overlay` CSS around line 876 area)

- [ ] **Step 1: Search for remaining references**

```bash
grep -rn "AdvantageForm\|ModifierEffectEditor\|StatusTemplateEditor" src/ 2>&1
```

Expected: only the three files themselves (imports gone after Tasks 18-20). If any other references show, fix them first.

- [ ] **Step 2: Delete the files**

```bash
rm src/lib/components/AdvantageForm.svelte
rm src/lib/components/gm-screen/ModifierEffectEditor.svelte
rm src/lib/components/gm-screen/StatusTemplateEditor.svelte
```

- [ ] **Step 3: Remove dead .editor-overlay CSS**

```bash
grep -n "editor-overlay" src/lib/components/AdvantagesManager.svelte src/lib/components/gm-screen/StatusPaletteDock.svelte src/lib/components/gm-screen/CharacterCard.svelte 2>&1
```

Remove the matching CSS blocks from each file.

- [ ] **Step 4: Run verify gate + smoke**

```bash
./scripts/verify.sh
```

Then `npm run tauri dev`:
- Advantages tool: + New / edit / delete all work via new shell
- GM Screen → StatusPalette: + New / edit / delete templates all work
- GM Screen → CharacterCard / CharacterRow: clicking modifier chip opens new editor under transform parents

- [ ] **Step 5: Commit**

```bash
git add -u src/lib/components/AdvantageForm.svelte src/lib/components/gm-screen/ModifierEffectEditor.svelte src/lib/components/gm-screen/StatusTemplateEditor.svelte src/lib/components/AdvantagesManager.svelte src/lib/components/gm-screen/StatusPaletteDock.svelte src/lib/components/gm-screen/CharacterCard.svelte
git commit -m "$(cat <<'EOF'
chore(builder): delete legacy editors, remove dead .editor-overlay CSS

AdvantageForm.svelte, ModifierEffectEditor.svelte, StatusTemplateEditor.svelte
are now fully superseded by ItemBuilderShell + per-type bodies. Removed
the now-unused .editor-overlay CSS blocks from AdvantagesManager,
StatusPaletteDock, and CharacterCard.

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

---

## Task 23: End-of-branch code review

Single comprehensive review covering all of Tasks 1-22. No new code — this is the quality gate.

**Files:**
- None changed. Review-only task.

- [ ] **Step 1: Confirm verify.sh passes on latest commit**

```bash
./scripts/verify.sh
```

- [ ] **Step 2: Confirm `ARCHITECTURE.md` §4 is updated**

The spec adds several `#[tauri::command]` paths (potentially in steps 5-7). Check that ARCHITECTURE.md §4 IPC inventory reflects any new commands AND the running total per the project's hard rule. If not done already, add the entries in a final docs commit.

```bash
grep -n "tauri::command\|invoke_handler" src-tauri/src/lib.rs | wc -l
# Compare with ARCHITECTURE.md §4 total
```

- [ ] **Step 3: Run `code-review:code-review` against the branch diff**

Invoke `code-review:code-review` skill against the templates branch diff (compared to master). Address findings inline. Re-run verify.sh after each fix.

- [ ] **Step 4: Final commit (if any review fixes were applied)**

```bash
./scripts/verify.sh
git add -u
git commit -m "$(cat <<'EOF'
review(builder): address end-of-branch code review findings

[Summarize fixes here]

Refs: docs/superpowers/specs/2026-05-17-item-builder-shell-design.md
EOF
)"
```

- [ ] **Step 5: Update ARCHITECTURE.md if any new IPC commands were added**

Per project CLAUDE.md hard rule: any new `#[tauri::command]` requires updating ARCHITECTURE.md §4 in the same commit. If steps 5-7 added new commands (unlikely — they extend existing structs but probably don't add new top-level commands), ensure the inventory is current.

---

## Self-review notes

- **Spec coverage:** Every section of the spec maps to one or more tasks. Picker → Task 21. Card-shape composer → Tasks 11, 12, 14. Spotlight → Task 14, 15-17. Per-body cardLayouts → Tasks 15, 19, 20. Data shape additions → Tasks 5, 6, 7. AdvantageCard / StatusTemplateCard extraction → Tasks 8, 9. WIP save → Tasks 6, 18, 19. Right-click carve-out → Task 10. Speed patterns → Tasks 2, 13 (and scattered).
- **Placeholder scan:** None remain. Bonus → effects[] mapping is resolved via `bonusToEffect()` helper in Task 18 (the spec gap discovered during plan-writing — Advantage didn't have effects[]; now gains it in Task 7).
- **Type consistency:** `confirmedFields` is camelCase TS-side, `confirmed_fields` is snake_case DB-side, both consistent throughout. `ModifierKind::AutoSuccess` (Rust) ↔ `'auto_success'` (TS) via serde rename. `AdvantageBodyState` consistent across Tasks 15-18.
- **Atomic clusters:** Tasks 5, 6, 7 are intentionally atomic (schema/code mismatch = runtime break). Task 18 (AdvantagesManager migration) is also one commit — partial state would render the page broken.

---

## Plan complete

Plan saved to `docs/superpowers/plans/2026-05-19-item-builder-shell.md`. Two execution options:

**1. Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
