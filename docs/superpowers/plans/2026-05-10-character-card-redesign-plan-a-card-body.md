# Character Card Redesign — Plan A — Card Body & Campaign Refactor

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the ~400 lines of inline character-card markup in `src/tools/Campaign.svelte` with a new `CharacterCard.svelte` (Camarilla-Dossier styled, fixed 2:3 aspect, four views, flip mechanic) and a `CharacterCardShell.svelte` wrapper that holds drag handle / source chip / save buttons outside the card frame.

**Architecture:** Component-split refactor. Card body is a self-contained Svelte 5 component that takes a `BridgeCharacter` prop and renders one of four panels (Basics, Stats, Disciplines, Advantages) with a persistent header/footer. Shell wraps the card with Campaign-flavoured chrome. No data-flow changes — same stores, same IPC. Plan B (separate plan file) wires modifier integration on top.

**Tech Stack:** Svelte 5 (runes: `$state`, `$derived`, `$effect`, `$props`), TypeScript, vanilla CSS via existing `:root` tokens.

---

## Required Reading

Before starting any task, the implementer MUST read these in order. They are scoped to what Plan A touches:

1. **CLAUDE.md** — claude-specific rules (verify.sh gate, no `std::fs` in async paths, no theme toggles, no light mode, snake_case vs camelCase asymmetry).
2. **ARCHITECTURE.md** §3, §4, §6, §10 — storage / IPC / `:root` token convention / no-frontend-test-framework.
3. **`docs/superpowers/specs/2026-05-10-character-card-redesign-design.md`** §1 (intent), §3 (visual identity), §4 (sizing), §5 (four views), §7 (component architecture). **Skip §6 (modifier integration — Plan B).**
4. **`docs/superpowers/specs/2026-04-16-campaign-card-redesign.md`** — the existing density-toggle pattern this plan reuses.
5. **Visual mockup reference** (do NOT copy verbatim; use as styling reference): `.superpowers/brainstorm/557511-1778373454/content/views-and-scaling.html` (the four-view fixed-aspect mockup with the shell-rail).

The implementer is **not** to introduce a frontend test framework (ARCH §10 forbids it). Plan A has zero new tests.

The implementer is **not** to commit without first running `./scripts/verify.sh` and confirming green output (CLAUDE.md gate).

---

## File Structure

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src/routes/+layout.svelte` | Add new `--*-card-dossier` CSS tokens to the existing `:root` block (per ARCH §6). |
| Modify | `src/tools/Campaign.svelte` | Add `--card-scale` to existing `densityVars`; remove inline character-card markup (lines 625–1021); remove now-unused state/helpers/snippets/styles; replace per-character render with `<CharacterCardShell />`. |
| Create | `src/lib/components/CharacterCard.svelte` | Pure card body — dossier frame, persistent header (file label + SUBJECT name + PC/NPC badge + clan line), four view panels, flipper footer, all source-aware helpers (moved from Campaign.svelte), all four snippets (moved from Campaign.svelte). |
| Create | `src/lib/components/CharacterCardShell.svelte` | Campaign-flavoured wrapper — shell-rail row above card (drag handle ⋮⋮ inert + source chip + drift badge + Save / Update saved / Compare buttons) + slot for `<CharacterCard />`. |

After Plan A, `Campaign.svelte` shrinks dramatically (~2002 lines today → ~600 lines projected). The script block keeps only what the toolbar / setup-guide / grid container needs; the inline-card markup and its supporting code all moves into `CharacterCard.svelte`.

---

## Task 1 — Add tokens + density custom property

**Files:**
- Modify: `src/routes/+layout.svelte` (the `:root` block ends near line 84)
- Modify: `src/tools/Campaign.svelte:156-164` (the `densityVars` derived expression)

- [ ] **Step 1:** Open `src/routes/+layout.svelte`. Find the `:root` block (starts around line 50 with `--text-primary:`). After the `--shadow-strong:` line and before the closing `}` of `:root`, add the dossier tokens. Insert this block:

```css
    /* ── Camarilla-Dossier card variant ─────────────────────────────── */
    --bg-card-dossier:        #14181d;  /* slate institutional surface */
    --text-card-dossier:      #c5cdd6;  /* primary content */
    --accent-card-dossier:    #5c8aa8;  /* slate-blue accent / labels */
    --alert-card-dossier:     #d24545;  /* blood-red active / deltas */
    --label-card-dossier:     rgba(92, 138, 168, 0.9);  /* file labels */
    --rule-card-dossier:        rgba(197, 205, 214, 0.08);  /* hairline */
    --rule-card-dossier-dashed: rgba(197, 205, 214, 0.12);  /* decorative */
    --shadow-card-dossier:    0 8px 32px rgba(0, 0, 0, 0.45);
```

- [ ] **Step 2:** Open `src/tools/Campaign.svelte`. Find the `densityVars` derived expression at lines 156–164. Replace the body of the `vals` lookup so each density entry includes a `cardScale` property AND the returned string includes `--card-scale`. Final block:

```ts
  const densityVars = $derived.by(() => {
    const d = resolvedDensity;
    const vals = {
      s: { minCol: '16rem', pad: '0.4rem', trackH: '1.4rem', conscienceCap: '1.5rem', dropSize: '1.2rem', conscienceGlow: 'none', cardScale: '0.7' },
      m: { minCol: '20rem', pad: '0.6rem', trackH: '1.8rem', conscienceCap: '2.5rem', dropSize: '1.6rem', conscienceGlow: '0 0 0.3rem color-mix(in srgb, var(--accent) 30%, transparent)', cardScale: '1.0' },
      l: { minCol: '28rem', pad: '0.8rem', trackH: '2.4rem', conscienceCap: '4rem', dropSize: '2rem', conscienceGlow: '0 0 0.5rem color-mix(in srgb, var(--accent) 50%, transparent)', cardScale: '1.4' },
    }[d];
    return `--col-min:${vals.minCol};--card-pad:${vals.pad};--track-h:${vals.trackH};--conscience-cap:${vals.conscienceCap};--drop-size:${vals.dropSize};--conscience-glow:${vals.conscienceGlow};--card-scale:${vals.cardScale}`;
  });
```

- [ ] **Step 3:** Run `./scripts/verify.sh`. Expected: all three checks (`npm run check`, `cargo test`, `npm run build`) pass green.

- [ ] **Step 4:** Commit.

```bash
git add src/routes/+layout.svelte src/tools/Campaign.svelte
git commit -m "$(cat <<'EOF'
feat(tokens): add Camarilla-Dossier card tokens and --card-scale density var

Adds the slate-blue / blood-red token set used by the new CharacterCard,
plus a --card-scale custom property per density level so the new fixed-
aspect card can scale uniformly under the existing density toggle.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md
EOF
)"
```

---

## Task 2 — Create `CharacterCard.svelte`

The card is created in one task because intermediate states (header without panel content, panel without flipper, etc.) wouldn't render anything useful and `npm run check` is the only meaningful verification before consumption.

**Files:**
- Create: `src/lib/components/CharacterCard.svelte`

**Reference:** `.superpowers/brainstorm/557511-1778373454/content/views-and-scaling.html` shows the visual target. Translate the inline `<style>` rules in that file into scoped `<style>` rules on the Svelte component, swapping hex literals for the `--*-card-dossier` tokens added in Task 1.

- [ ] **Step 1:** Create the file `src/lib/components/CharacterCard.svelte` with the script block. The script imports source-aware helpers from existing libs and ports the four local helpers + state from `Campaign.svelte`. Paste this verbatim:

```svelte
<script lang="ts">
  import type { BridgeCharacter, FoundryItem, Roll20Raw, Roll20RawAttribute } from '../../types';
  import {
    foundryFeatures,
    foundryEffects,
    foundryItemEffects,
    foundryEffectIsActive,
    foundryAttrInt,
    foundrySkillInt,
  } from '$lib/foundry/raw';
  import { FOUNDRY_SKILL_NAMES } from '$lib/foundry/canonical-names';
  import {
    characterRemoveAdvantage,
    characterAddAdvantage,
    characterSetField,
  } from '$lib/character/api';
  import type { FeatureType, CanonicalFieldName } from '$lib/character/api';

  interface Props {
    character: BridgeCharacter;
    viewIndex?: number;
    onViewChange?: (i: number) => void;
  }
  let { character, viewIndex: viewIndexProp, onViewChange }: Props = $props();

  // Uncontrolled mode default; controlled when caller passes viewIndex.
  let internalView = $state(1);
  const viewIndex = $derived(viewIndexProp ?? internalView);

  function setView(next: number) {
    if (next < 1 || next > 4) return;
    if (onViewChange) onViewChange(next);
    else internalView = next;
  }
  function flip(dir: 1 | -1) {
    setView(((viewIndex - 1 + dir + 4) % 4) + 1);
  }

  const VIEW_LABELS = ['Basics', 'Stats', 'Disciplines', 'Advantages'] as const;

  // ── Stat editor (ported from Campaign.svelte) ─────────────────────────
  const FIELD_RANGES: Partial<Record<CanonicalFieldName, [number, number]>> = {
    hunger:                [0, 5],
    humanity:              [0, 10],
    humanity_stains:       [0, 10],
    blood_potency:         [0, 10],
    health_superficial:    [0, 20],
    health_aggravated:     [0, 20],
    willpower_superficial: [0, 20],
    willpower_aggravated:  [0, 20],
  };

  function liveEditAllowed(c: BridgeCharacter): boolean {
    return c.source === 'foundry';
  }
  function advantageEditAllowed(c: BridgeCharacter): boolean {
    return c.source === 'foundry';
  }

  let busyKey = $state<string | null>(null);
  function stepperKey(c: BridgeCharacter, field: CanonicalFieldName): string {
    return `${c.source}:${c.source_id}:${field}`;
  }

  async function tweakField(
    c: BridgeCharacter,
    field: CanonicalFieldName,
    delta: number,
    current: number,
  ) {
    const range = FIELD_RANGES[field];
    if (!range) return;
    const next = Math.max(range[0], Math.min(range[1], current + delta));
    if (next === current) return;
    const key = stepperKey(c, field);
    busyKey = key;
    try {
      await characterSetField('live', c.source, c.source_id, field, next);
    } catch (e) {
      console.error('[CharacterCard] characterSetField failed:', e);
      window.alert(String(e));
    } finally {
      if (busyKey === key) busyKey = null;
    }
  }

  // ── Source-aware readers (ported from Campaign.svelte) ────────────────
  function r20Attrs(c: BridgeCharacter): Roll20RawAttribute[] {
    if (c.source !== 'roll20') return [];
    const raw = c.raw as Roll20Raw | null;
    return raw?.attributes ?? [];
  }
  function r20AttrInt(c: BridgeCharacter, name: string): number {
    const a = r20Attrs(c).find(a => a.name === name);
    return a ? (parseInt(a.current, 10) || 0) : 0;
  }
  function r20AttrText(c: BridgeCharacter, name: string): string {
    return r20Attrs(c).find(a => a.name === name)?.current ?? '';
  }
  function attrInt(c: BridgeCharacter, name: string): number {
    if (c.source === 'foundry') return foundryAttrInt(c, name);
    return r20AttrInt(c, name);
  }
  function skillInt(c: BridgeCharacter, name: string): number {
    if (c.source === 'foundry') return foundrySkillInt(c, name);
    return 0;
  }
  function parseDisciplines(c: BridgeCharacter): { type: string; level: number }[] {
    const attrs = r20Attrs(c);
    const prefix = 'repeating_disciplines_';
    const suffix = '_discipline';
    return attrs
      .filter(a => a.name.startsWith(prefix) && a.name.endsWith(suffix) && !a.name.includes('_power_'))
      .map(a => {
        const id = a.name.slice(prefix.length, -suffix.length);
        const nameAttr = attrs.find(x => x.name === `${prefix}${id}_discipline_name`);
        return { type: nameAttr?.current ?? '', level: parseInt(a.current, 10) || 0 };
      })
      .filter(d => d.type && d.level > 0);
  }
  function isPC(c: BridgeCharacter): boolean {
    return c.controlled_by !== null && c.controlled_by.trim() !== '';
  }
  function fileLabel(c: BridgeCharacter): string {
    const tail = c.source_id.slice(-4).toUpperCase();
    return `Camarilla file 0x${tail}`;
  }
  function dots(filled: number, total: number): boolean[] {
    return Array.from({ length: total }, (_, i) => i < filled);
  }

  // ── Advantage add/remove (ported from Campaign.svelte) ────────────────
  let busyAdvantageKey = $state<string | null>(null);
  function advantageBusyKey(c: BridgeCharacter, itemId: string): string {
    return `${c.source}:${c.source_id}:${itemId}`;
  }
  async function removeAdvantage(c: BridgeCharacter, ft: FeatureType, item: FoundryItem) {
    if (!advantageEditAllowed(c)) return;
    if (!window.confirm(`Remove ${ft} '${item.name}'?`)) return;
    const key = advantageBusyKey(c, item._id);
    busyAdvantageKey = key;
    try {
      await characterRemoveAdvantage('live', c.source, c.source_id, ft, item._id);
    } catch (e) {
      console.error('[CharacterCard] removeAdvantage failed:', e);
      window.alert(String(e));
    } finally {
      if (busyAdvantageKey === key) busyAdvantageKey = null;
    }
  }

  type AddFormState = {
    charKey: string;
    featuretype: FeatureType;
    name: string;
    points: number;
    description: string;
    submitting: boolean;
  };
  let addForm = $state<AddFormState | null>(null);
  function startAdd(c: BridgeCharacter, ft: FeatureType) {
    if (!advantageEditAllowed(c)) return;
    addForm = {
      charKey: `${c.source}:${c.source_id}`, featuretype: ft,
      name: '', points: 0, description: '', submitting: false,
    };
  }
  function cancelAdd() { addForm = null; }
  function isAddActive(c: BridgeCharacter, ft: FeatureType): boolean {
    if (!addForm) return false;
    return addForm.charKey === `${c.source}:${c.source_id}` && addForm.featuretype === ft;
  }
  function addFormValid(f: AddFormState): boolean {
    return f.name.trim().length > 0 && f.points >= 0 && f.points <= 10;
  }
  async function submitAdd(c: BridgeCharacter) {
    if (!addForm) return;
    if (!isAddActive(c, addForm.featuretype)) return;
    if (!addFormValid(addForm)) return;
    addForm.submitting = true;
    try {
      await characterAddAdvantage(
        'live', c.source, c.source_id,
        addForm.featuretype, addForm.name.trim(),
        addForm.description, addForm.points,
      );
      addForm = null;
    } catch (e) {
      console.error('[CharacterCard] characterAddAdvantage failed:', e);
      window.alert(String(e));
      if (addForm) addForm.submitting = false;
    }
  }

  // ── Computed view-data ────────────────────────────────────────────────
  const hunger      = $derived(character.hunger ?? 0);
  const bp          = $derived(character.bloodPotency ?? 0);
  const humanity    = $derived(character.humanity ?? 0);
  const stains      = $derived(character.humanityStains ?? 0);
  const healthMax   = $derived(character.health?.max ?? 0);
  const healthSup   = $derived(character.health?.superficial ?? 0);
  const healthAgg   = $derived(character.health?.aggravated ?? 0);
  const wpMax       = $derived(character.willpower?.max ?? 0);
  const wpSup       = $derived(character.willpower?.superficial ?? 0);
  const wpAgg       = $derived(character.willpower?.aggravated ?? 0);
  const healthOk    = $derived(Math.max(0, healthMax - healthSup - healthAgg));
  const wpOk        = $derived(Math.max(0, wpMax - wpSup - wpAgg));
  const clan        = $derived(character.clan ?? '');
  const generation  = $derived(character.generation ?? null);
  const disciplines = $derived(
    character.source === 'foundry'
      ? (character.disciplines ?? [])
      : parseDisciplines(character),
  );
</script>
```

- [ ] **Step 2:** Below the `</script>` tag, add the four snippet definitions (ported with minor signature changes — the snippets used to take `char`, but now they live inside the component so they can read `character` from the component closure if simpler; keep the explicit `c` parameter for cleanliness so they remain self-contained):

```svelte
{#snippet stepper(c: BridgeCharacter, field: CanonicalFieldName, current: number)}
  {@const allowed   = liveEditAllowed(c)}
  {@const key       = stepperKey(c, field)}
  {@const busy      = busyKey === key}
  {@const range     = FIELD_RANGES[field]}
  {#if range}
    {@const atFloor   = current <= range[0]}
    {@const atCeiling = current >= range[1]}
    {@const tooltip   = allowed ? '' : 'Roll20 live editing not supported (Phase 2.5)'}
    <span class="stat-stepper" class:roll20-blocked={!allowed} aria-disabled={!allowed}>
      <button type="button" class="step-btn"
        onclick={() => tweakField(c, field, -1, current)}
        disabled={!allowed || busy || atFloor}
        aria-busy={busy} title={tooltip} aria-label={`Decrease ${field}`}>−</button>
      <button type="button" class="step-btn"
        onclick={() => tweakField(c, field, +1, current)}
        disabled={!allowed || busy || atCeiling}
        aria-busy={busy} title={tooltip} aria-label={`Increase ${field}`}>+</button>
    </span>
  {/if}
{/snippet}

{#snippet chipRemoveBtn(c: BridgeCharacter, ft: FeatureType, item: FoundryItem)}
  {@const allowed = advantageEditAllowed(c)}
  {@const busy    = busyAdvantageKey === advantageBusyKey(c, item._id)}
  {#if allowed}
    <button type="button" class="chip-remove-btn"
      onclick={() => removeAdvantage(c, ft, item)}
      disabled={busy} aria-busy={busy}
      title={`Remove ${ft}`} aria-label={`Remove ${ft} ${item.name}`}>×</button>
  {/if}
{/snippet}

{#snippet addBtn(c: BridgeCharacter, ft: FeatureType)}
  {#if advantageEditAllowed(c) && !isAddActive(c, ft)}
    <button type="button" class="feat-chip add-chip"
      onclick={() => startAdd(c, ft)}
      title={`Add ${ft}`}>+ Add {ft}</button>
  {/if}
{/snippet}

{#snippet addForm_(c: BridgeCharacter, ft: FeatureType)}
  {#if addForm && isAddActive(c, ft)}
    <form class="add-form" onsubmit={(e) => { e.preventDefault(); void submitAdd(c); }}>
      <div class="form-row">
        <label for={`add-name-${addForm.charKey}-${ft}`}>Name</label>
        <input id={`add-name-${addForm.charKey}-${ft}`} type="text"
          bind:value={addForm.name} maxlength="120" required autofocus />
      </div>
      <div class="form-row">
        <label for={`add-points-${addForm.charKey}-${ft}`}>Points</label>
        <input id={`add-points-${addForm.charKey}-${ft}`} type="number"
          min="0" max="10" bind:value={addForm.points} />
      </div>
      <div class="form-row">
        <label for={`add-desc-${addForm.charKey}-${ft}`}>Description</label>
        <textarea id={`add-desc-${addForm.charKey}-${ft}`}
          bind:value={addForm.description} rows="2"></textarea>
      </div>
      <div class="form-actions">
        <button type="submit" class="btn-save"
          disabled={!addFormValid(addForm) || addForm.submitting}
          aria-busy={addForm.submitting}>Add</button>
        <button type="button" class="btn-save"
          onclick={cancelAdd} disabled={addForm.submitting}>Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}
```

- [ ] **Step 3:** Below the snippets, add the dossier frame template. Refer to `views-and-scaling.html` for the visual structure. Skeleton:

```svelte
<div class="dossier" data-pc={isPC(character)}>
  <div class="file-label">{fileLabel(character)}</div>

  <header class="name-row">
    <span class="name">{character.name}</span>
    <span class="badge" class:pc={isPC(character)} class:npc={!isPC(character)}>
      {isPC(character) ? 'PC' : 'NPC'}
    </span>
  </header>

  <div class="clan-line">
    {#if clan}{clan}{/if}{#if clan && generation} · {/if}{#if generation}{generation}th generation{/if}
  </div>

  <div class="panel">
    {#if viewIndex === 1}
      <!-- View 1 — Basics: hunger, BP, conscience, health, willpower (with steppers) -->
    {:else if viewIndex === 2}
      <!-- View 2 — Stats: attributes grid + filtered skills list -->
    {:else if viewIndex === 3}
      <!-- View 3 — Disciplines: per-discipline rows with powers -->
    {:else if viewIndex === 4}
      <!-- View 4 — Advantages: chip rows for merits/flaws/backgrounds/boons + actor effects -->
    {/if}
  </div>

  <footer class="flipper">
    <button class="flip-arrow" type="button" aria-label="Previous view"
            onclick={() => flip(-1)}>‹</button>
    <span class="flip-current">{VIEW_LABELS[viewIndex - 1]}</span>
    <span class="flip-pager">{viewIndex} / 4</span>
    <button class="flip-arrow" type="button" aria-label="Next view"
            onclick={() => flip(+1)}>›</button>
  </footer>
</div>
```

- [ ] **Step 4:** Fill in **View 1 (Basics)**. Replace the `<!-- View 1 ... -->` placeholder with:

```svelte
<div class="basics">
  <div class="vital-row">
    <div class="hunger-cluster">
      <div class="hunger-drops">
        {#each dots(hunger, 5) as filled}
          <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
            <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
          </svg>
        {/each}
      </div>
      {@render stepper(character, 'hunger', hunger)}
    </div>
    <div class="bp-pill">
      <span class="qs-label">BP</span>
      <span class="bp-value">{bp}</span>
      {@render stepper(character, 'blood_potency', bp)}
    </div>
  </div>

  <div class="block">
    <div class="track-label">Conscience</div>
    <div class="conscience-track">
      {#each 'CONSCIENCE'.split('') as letter, i}
        {@const pos = i + 1}
        {@const isFilled = pos <= humanity}
        {@const isStained = pos > 10 - stains}
        <span class="conscience-letter"
              class:filled={isFilled}
              class:stained={isStained && !isFilled}>{letter}</span>
      {/each}
    </div>
    <div class="ctrl-grid">
      <span class="ctrl-label">Humanity</span>
      {@render stepper(character, 'humanity', humanity)}
      <span class="ctrl-label">Stains</span>
      {@render stepper(character, 'humanity_stains', stains)}
    </div>
  </div>

  <div class="block">
    <div class="track-label">Health</div>
    <div class="track-boxes">
      {#each Array.from({ length: healthMax }, (_, i) => i) as i}
        <div class="box health"
             class:superficial={i >= healthOk && i < healthOk + healthSup}
             class:aggravated={i >= healthOk + healthSup}></div>
      {/each}
    </div>
    <div class="ctrl-grid">
      <span class="ctrl-label">Sup</span>
      {@render stepper(character, 'health_superficial', healthSup)}
      <span class="ctrl-label">Agg</span>
      {@render stepper(character, 'health_aggravated', healthAgg)}
    </div>
  </div>

  <div class="block">
    <div class="track-label">Willpower</div>
    <div class="track-boxes">
      {#each Array.from({ length: wpMax }, (_, i) => i) as i}
        <div class="box willpower"
             class:filled={i < wpOk}
             class:superficial={i >= wpOk && i < wpOk + wpSup}
             class:aggravated={i >= wpOk + wpSup}></div>
      {/each}
    </div>
    <div class="ctrl-grid">
      <span class="ctrl-label">Sup</span>
      {@render stepper(character, 'willpower_superficial', wpSup)}
      <span class="ctrl-label">Agg</span>
      {@render stepper(character, 'willpower_aggravated', wpAgg)}
    </div>
  </div>
</div>
```

- [ ] **Step 5:** Fill in **View 2 (Stats)**. Replace the `<!-- View 2 ... -->` placeholder. The skill filter is the load-bearing piece — only render skills with `level > 0` OR a non-empty specialty list:

```svelte
{@const ATTR_NAMES = [
  ['strength', 'STR'], ['dexterity', 'DEX'], ['stamina', 'STA'],
  ['charisma', 'CHA'], ['manipulation', 'MAN'], ['composure', 'COM'],
  ['intelligence', 'INT'], ['wits', 'WIT'], ['resolve', 'RES'],
] as const}
{@const skills = character.source === 'foundry'
  ? FOUNDRY_SKILL_NAMES
      .map((name) => ({ name, value: foundrySkillInt(character, name), specialties: [] as string[] }))
      .filter((s) => s.value > 0 || s.specialties.length > 0)
  : []}
{@const hiddenSkillCount = (character.source === 'foundry' ? FOUNDRY_SKILL_NAMES.length : 0) - skills.length}
<div class="stats">
  <div class="panel-title">Attributes</div>
  <div class="attr-grid">
    {#each ATTR_NAMES as [n, abbr]}
      <div class="attr-cell" data-path={`attributes.${n}`}>
        <span class="attr-name">{abbr}</span>
        <span class="attr-val">{attrInt(character, n)}</span>
      </div>
    {/each}
  </div>
  <hr />
  <div class="panel-title">Skills</div>
  {#if skills.length === 0}
    <div class="skills-empty">No skills with non-zero levels.</div>
  {:else}
    <div class="skills">
      {#each skills as s}
        <div class="skill-row" data-path={`skills.${s.name}`}>
          <span class="skill-name">{s.name}</span>
          <span class="skill-val">{s.value}</span>
        </div>
      {/each}
    </div>
  {/if}
  {#if hiddenSkillCount > 0}
    <div class="skills-note">Hidden: {hiddenSkillCount} skills at zero</div>
  {/if}
</div>
```

(Specialties as a list need a Foundry helper that's out of scope for Plan A — the `specialties` field starts empty. Spec §5.2 keeps specialty rendering scoped to Plan B / a follow-up, since Foundry stores specialties as separate Item documents per `2026-04-30-character-tooling-roadmap.md` §3.5; wiring that read is Plan B / future scope. For Plan A, render `value > 0` only and skip the specialty/italic line.)

- [ ] **Step 6:** Fill in **View 3 (Disciplines)**. Replace `<!-- View 3 ... -->`:

```svelte
{@const powerItems = character.source === 'foundry'
  ? (character.raw?.items ?? []).filter((it: any) => it?.type === 'power')
  : []}
{@const grouped = (() => {
  const m: Record<string, { name: string; level: number; powers: string[] }> = {};
  for (const d of disciplines) {
    m[d.type.toLowerCase()] = { name: d.type, level: d.level, powers: [] };
  }
  for (const p of powerItems) {
    const key = (p?.system?.discipline ?? '').toLowerCase();
    if (m[key]) m[key].powers.push(p?.name ?? '');
  }
  return Object.values(m).sort((a, b) => a.name.localeCompare(b.name));
})()}
<div class="disc">
  {#if grouped.length === 0}
    <div class="disc-empty">No disciplines on this character.</div>
  {/if}
  {#each grouped as d, idx}
    {#if idx > 0}<hr />{/if}
    <div class="disc-row">
      <div class="disc-name">
        <span>{d.name}</span>
        <span class="dots">{'●'.repeat(Math.min(d.level, 5))}</span>
      </div>
      {#if d.powers.length > 0}
        <div class="powers">
          {#each d.powers as p}
            <div class="power">{p}</div>
          {/each}
        </div>
      {/if}
    </div>
  {/each}
</div>
```

- [ ] **Step 7:** Fill in **View 4 (Advantages)**. Replace `<!-- View 4 ... -->`:

```svelte
{@const merits      = character.source === 'foundry' ? foundryFeatures(character, 'merit')      : []}
{@const flaws       = character.source === 'foundry' ? foundryFeatures(character, 'flaw')       : []}
{@const backgrounds = character.source === 'foundry' ? foundryFeatures(character, 'background') : []}
{@const boons       = character.source === 'foundry' ? foundryFeatures(character, 'boon')       : []}
{@const actorFx     = character.source === 'foundry' ? foundryEffects(character)                : []}
<div class="adv">
  {#if merits.length > 0 || advantageEditAllowed(character)}
    <div class="adv-section">
      <div class="adv-label">Merits</div>
      <div class="chips">
        {#each merits as m}
          {@const points = (m.system?.points as number | undefined) ?? 0}
          <span class="chip merit" data-active="false" data-item-id={m._id}>
            <span class="feat-name">{m.name}</span>
            {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
            {@render chipRemoveBtn(character, 'merit', m)}
          </span>
        {/each}
        {@render addBtn(character, 'merit')}
      </div>
      {@render addForm_(character, 'merit')}
    </div>
  {/if}
  {#if flaws.length > 0 || advantageEditAllowed(character)}
    <div class="adv-section">
      <div class="adv-label">Flaws</div>
      <div class="chips">
        {#each flaws as f}
          {@const points = (f.system?.points as number | undefined) ?? 0}
          <span class="chip flaw" data-active="false" data-item-id={f._id}>
            <span class="feat-name">{f.name}</span>
            {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
            {@render chipRemoveBtn(character, 'flaw', f)}
          </span>
        {/each}
        {@render addBtn(character, 'flaw')}
      </div>
      {@render addForm_(character, 'flaw')}
    </div>
  {/if}
  {#if backgrounds.length > 0 || advantageEditAllowed(character)}
    <div class="adv-section">
      <div class="adv-label">Backgrounds</div>
      <div class="chips">
        {#each backgrounds as b}
          {@const points = (b.system?.points as number | undefined) ?? 0}
          <span class="chip bg" data-active="false" data-item-id={b._id}>
            <span class="feat-name">{b.name}</span>
            {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
            {@render chipRemoveBtn(character, 'background', b)}
          </span>
        {/each}
        {@render addBtn(character, 'background')}
      </div>
      {@render addForm_(character, 'background')}
    </div>
  {/if}
  {#if boons.length > 0 || advantageEditAllowed(character)}
    <div class="adv-section">
      <div class="adv-label">Boons</div>
      <div class="chips">
        {#each boons as bn}
          <span class="chip boon" data-active="false" data-item-id={bn._id}>
            <span class="feat-name">{bn.name}</span>
            {@render chipRemoveBtn(character, 'boon', bn)}
          </span>
        {/each}
        {@render addBtn(character, 'boon')}
      </div>
      {@render addForm_(character, 'boon')}
    </div>
  {/if}
  {#if actorFx.length > 0}
    <div class="adv-section">
      <div class="adv-label">Active modifiers (actor)</div>
      <div class="chips">
        {#each actorFx as e}
          {@const active = foundryEffectIsActive(e)}
          <span class="chip effect" class:disabled={!active}
                title={e.changes?.map(c => `${c.key} mode=${c.mode} value=${c.value}`).join('\n') ?? ''}>
            <span class="feat-name">{e.name}</span>
            <span class="fx-badge">{e.changes?.length ?? 0}</span>
          </span>
        {/each}
      </div>
    </div>
  {/if}
  {#if merits.length === 0 && flaws.length === 0 && backgrounds.length === 0 && boons.length === 0 && actorFx.length === 0}
    <div class="adv-empty">No merits, flaws, backgrounds, or modifiers.</div>
  {/if}
</div>
```

- [ ] **Step 8:** Add scoped styles. Translate the inline-styled mockup in `views-and-scaling.html` into scoped Svelte styles, swapping every hex literal for the `--*-card-dossier` token introduced in Task 1. The full styles block is large (~250 lines); the implementer writes them following these rules:

  1. The root `.dossier` element is the 2:3 aspect frame. Use `aspect-ratio: 2 / 3;` and `width: calc(280px * var(--card-scale, 1));` so size scales with the density custom property from Task 1.
  2. Every numeric inner dimension (font-size, padding, gap, drop size, track-box size, border-radius, etc.) multiplies by `var(--card-scale, 1)`. Use `calc()` and CSS custom properties — no `vw`/`vh`.
  3. Inner dashed border — implement as `&::before { content: ''; position: absolute; inset: calc(8px * var(--card-scale, 1)); border: 1px dashed var(--rule-card-dossier-dashed); pointer-events: none; }`.
  4. The `.name::before { content: 'SUBJECT  '; ... }` rule provides the SUBJECT prefix.
  5. Active-chip styling MUST exist for `.chip[data-active="true"]` — it never activates in Plan A (the attribute is hardcoded to `"false"`) but the CSS is shipped now to avoid re-touching the file in Plan B. Apply red fill, glow shadow, and a corner ◉ marker per spec §5.4.
  6. Steppers re-styled to slate-blue ± circles using `--accent-card-dossier`. Functionality unchanged.
  7. Box-sizing: `border-box` on `.dossier` and all descendants (ARCH §6).
  8. NO hex literals. Every color via `var(--*)`.
  9. Reduced-motion fallback: respect `@media (prefers-reduced-motion: reduce)` — disable any transitions added for hover/flip.

- [ ] **Step 9:** Run `./scripts/verify.sh`. Expected: green. The component is unused at this point, but `npm run check` validates the script block.

- [ ] **Step 10:** Commit.

```bash
git add src/lib/components/CharacterCard.svelte
git commit -m "$(cat <<'EOF'
feat(character-card): add CharacterCard.svelte with four views and flip mechanic

Camarilla-Dossier styled card body. Fixed 2:3 aspect, scales with --card-scale
custom property. Four views (Basics / Stats / Disciplines / Advantages) flipped
via footer arrow controls. View 4 chips ship with data-active="false" placeholder
and inert active-state CSS for Plan B to wire.

Source-aware helpers (r20Attrs, attrInt, skillInt, parseDisciplines) ported from
Campaign.svelte; the four template snippets (stepper, chipRemoveBtn, addBtn,
addForm_) ported as-is. No data-flow changes — same character::set_field router,
same characterAddAdvantage / characterRemoveAdvantage commands.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md
EOF
)"
```

---

## Task 3 — Create `CharacterCardShell.svelte`

**Files:**
- Create: `src/lib/components/CharacterCardShell.svelte`

The shell renders the rail row above the card with drag handle (inert), source attribution chip, drift indicator, and the save / update / compare buttons that used to live below the inline card markup in Campaign.svelte.

- [ ] **Step 1:** Create `src/lib/components/CharacterCardShell.svelte` with this content:

```svelte
<script lang="ts">
  import type { BridgeCharacter } from '../../types';
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import SourceAttributionChip from './SourceAttributionChip.svelte';
  import CharacterCard from './CharacterCard.svelte';
  import { savedCharacters } from '../../store/savedCharacters.svelte';
  import { bridge } from '../../store/bridge.svelte';

  interface Props {
    character: BridgeCharacter;
    saved: SavedCharacter | null;
    drift: boolean;
    onCompare: (saved: SavedCharacter, live: BridgeCharacter) => void;
  }
  let { character, saved, drift, onCompare }: Props = $props();

  function saveCharacter() {
    const world = character.source === 'foundry'
      ? (bridge.sourceInfo.foundry?.worldTitle ?? null)
      : null;
    void savedCharacters.save(character, world);
  }
</script>

<div class="card-shell">
  <div class="shell-rail">
    <span class="drag" aria-hidden="true" title="Drag (reserved for GM screen)">⋮⋮</span>
    <SourceAttributionChip source={character.source} />
    {#if drift}
      <span class="drift-badge" title="Live differs from saved snapshot">drift</span>
    {/if}
    <span class="rail-spacer"></span>
    <div class="actions">
      {#if saved}
        <button type="button" class="btn-save"
          onclick={() => onCompare(saved, character)}>Compare</button>
        <button type="button" class="btn-save"
          onclick={() => savedCharacters.update(saved.id, character)}
          disabled={savedCharacters.loading}>Update saved</button>
      {:else}
        <button type="button" class="btn-save"
          onclick={saveCharacter}
          disabled={savedCharacters.loading}>Save locally</button>
      {/if}
    </div>
  </div>
  <CharacterCard {character} />
</div>

<style>
  .card-shell {
    display: flex;
    flex-direction: column;
    gap: calc(0.4rem * var(--card-scale, 1));
  }
  .shell-rail {
    display: flex;
    align-items: center;
    gap: calc(0.5rem * var(--card-scale, 1));
    padding: 0 calc(0.4rem * var(--card-scale, 1));
    font-size: calc(0.75rem * var(--card-scale, 1));
    color: var(--text-secondary);
  }
  .drag {
    letter-spacing: 0.4em;
    cursor: grab;
    user-select: none;
    color: var(--text-muted);
  }
  .rail-spacer { flex: 1; }
  .actions { display: flex; gap: calc(0.4rem * var(--card-scale, 1)); }
  .btn-save {
    background: var(--bg-raised);
    border: 1px solid var(--border-card);
    color: var(--text-primary);
    padding: calc(0.25rem * var(--card-scale, 1)) calc(0.6rem * var(--card-scale, 1));
    border-radius: 0.25rem;
    cursor: pointer;
    font-size: calc(0.75rem * var(--card-scale, 1));
  }
  .btn-save:hover:not(:disabled) { border-color: var(--border-surface); }
  .btn-save:disabled { opacity: 0.5; cursor: default; }
  .drift-badge {
    background: var(--accent-amber);
    color: var(--bg-base);
    font-size: calc(0.6rem * var(--card-scale, 1));
    padding: 0 0.4em;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-weight: 600;
  }
</style>
```

- [ ] **Step 2:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 3:** Commit.

```bash
git add src/lib/components/CharacterCardShell.svelte
git commit -m "$(cat <<'EOF'
feat(character-card): add CharacterCardShell.svelte (Campaign-flavoured wrapper)

Wrapper renders shell-rail above the card holding drag handle (inert; reserved
for future GM-screen drag-and-drop), source attribution chip, drift badge, and
the Save / Update saved / Compare buttons that previously lived inside the
inline Campaign card markup.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md
EOF
)"
```

---

## Task 4 — Refactor `Campaign.svelte` to use the new components

This is the largest task — it removes ~1400 lines (template + script + styles) and replaces them with ~30 lines that delegate to the new components.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1:** Add imports for the new components at the top of the script block. Find the existing imports (lines 1–27); after the `import type { CanonicalFieldName } from '$lib/character/api';` line, add:

```ts
  import CharacterCardShell from '$lib/components/CharacterCardShell.svelte';
```

- [ ] **Step 2:** Remove now-redundant state. In the script block:
  - Delete `expandedRaw`, `expandedAttrs`, `expandedSkills`, `expandedInfo`, `expandedFeats` declarations (lines 68–72).
  - Delete `FIELD_RANGES` constant (lines 78–87).
  - Delete `liveEditAllowed` function (lines 91–93).
  - Delete `stepperKey` function (lines 97–99).
  - Delete `busyKey` state (line 102).
  - Delete `tweakField` function (lines 104–124).
  - Delete `r20Attrs`, `r20AttrInt`, `r20AttrText`, `attrInt`, `skillInt`, `parseDisciplines` helpers (lines 171–222).
  - Delete `dots`, `toggleSet`, `toggleRaw`, `toggleAttrs`, `toggleSkills`, `toggleInfo`, `toggleFeats`, `isPC` (lines 224–242). **Keep** `timeSince` (used by toolbar) and `refresh` (used by toolbar button).
  - Delete `advantageEditAllowed`, `busyAdvantageKey`, `removeAdvantage`, `AddFormState` type, `addForm` state, `startAdd`, `cancelAdd`, `isAddActive`, `addFormValid`, `submitAdd` (lines 255–344).

  After Step 2 the script block keeps: imports, `connected`, `characters`, `lastSync`, `liveWithMatches`, `drifts` map, `hasDrift`, `comparing` state + `openCompare` + `closeCompare`, `onMount`, `urlCopied` + `copyExtensionsUrl`, `density` + `resolvedDensity` + `gridEl`, `densityVars`, `timeSince`, `refresh`. It should be ~80 lines instead of ~345.

- [ ] **Step 3:** Remove the four snippet definitions at template scope (lines 347–463: `stepper`, `chipRemoveBtn`, `addBtn`, `addForm_`).

- [ ] **Step 4:** Replace the inline character-card markup. Find the iteration around line 580 (`{#each liveWithMatches as ...}`) — somewhere in there a per-character `<div class="char-card">` is opened at line 625 and closed near line 1021 (look for the `</div>` after the raw-panel markup). Replace the entire `<div class="char-card"> ... </div>` (and the surrounding `{@const ...}` declarations specific to its rendering — lines 597–623) with:

```svelte
{#each liveWithMatches as { live, saved } (live.source + ':' + live.source_id)}
  <CharacterCardShell
    character={live}
    {saved}
    drift={hasDrift(live)}
    onCompare={openCompare}
  />
{/each}
```

The `liveWithMatches` derived map already pairs each live character with its saved-match. The `drift` boolean is now derived once and passed in. The `openCompare` callback is unchanged.

  Find the saved-grid iteration (search for `<article class="saved-card">` around line 1038). The saved grid renders `SavedCharacter` records that have **no live counterpart** (offline characters). Each `SavedCharacter` carries its full canonical snapshot under `saved.canonical` — render those as cards too. Replace the `{#each ...}` loop containing the `<article class="saved-card">` block with:

```svelte
{#each savedCharacters.list.filter(s => !characters.some(c => c.source === s.source && c.source_id === s.sourceId)) as savedRow (savedRow.id)}
  <CharacterCardShell
    character={savedRow.canonical}
    saved={savedRow}
    drift={false}
    onCompare={openCompare}
  />
{/each}
```

  (The `savedRow.canonical` is shaped as a `BridgeCharacter` for cards that came from a saved-only source; the shell renders it identically to a live one. Drift is false because there's no live counterpart to compare against.)

- [ ] **Step 5:** Remove dead CSS. Open the `<style>` block in `Campaign.svelte`. Delete every rule whose selector starts with `.char-card`, `.card-header`, `.card-section`, `.card-footer`, `.char-name`, `.char-clan`, `.header-line`, `.header-badges`, `.header-vitals`, `.hunger-cluster`, `.hunger-drops`, `.blood-drop`, `.bp-pill`, `.qs-label`, `.bp-value`, `.conscience-row`, `.conscience-track`, `.conscience-letter`, `.conscience-controls`, `.ctrl-row`, `.ctrl-label`, `.track-row`, `.track-cluster`, `.track-boxes`, `.track-controls`, `.box`, `.disc-section`, `.disc-chips`, `.disc-chip`, `.disc-dots`, `.attr-grid`, `.attr-cell`, `.attr-name`, `.attr-val`, `.skill-grid`, `.bane-row`, `.bane-severity`, `.bane-text`, `.info-row`, `.info-text`, `.info-long`, `.feat-row`, `.feat-chips`, `.feat-chip`, `.feat-name`, `.feat-dots`, `.feat-fx-badge`, `.feat-empty`, `.add-chip`, `.add-form`, `.form-row`, `.form-actions`, `.btn-save`, `.section-toggle`, `.raw-toggle`, `.raw-panel`, `.raw-row`, `.raw-name`, `.raw-val`, `.raw-empty`, `.raw-json`, `.save-row`, `.save-actions`, `.saved-card`, `.drift-badge`, `.badge`, `.stat-stepper`, `.step-btn`, `.chip-remove-btn`, `.roll20-hint`. **Keep** rules for: `.campaign`, `.toolbar`, `.status`, `.source-pip`, `.source-label`, `.sync-time`, `.spacer`, `.density-toggle`, `.density-btn`, `.btn-refresh`, `.setup-guide`, `.guide-title`, `.guide-sub`, `.bridge-section`, `.steps`, `.step`, `.step-body`, `.char-grid`, `.live-section`, `.saved-section`, `.section-title`. (The selector list is conservative — when in doubt, search for the class in the post-refactor template; if it's not used, delete the rule.)

- [ ] **Step 6:** Verify the `.char-grid` rule still applies the `densityVars` style to the iteration container. The toolbar / grid wrapper structure should be:

```svelte
<div class="char-grid" bind:this={gridEl} style={densityVars}>
  {#each liveWithMatches as ...}
    <CharacterCardShell ... />
  {/each}
  {#each savedCharacters.list.filter(...) as savedRow ...}
    <CharacterCardShell ... />
  {/each}
</div>
```

  The `style={densityVars}` attribute on `.char-grid` propagates `--card-scale` (added in Task 1) down to the cards via CSS inheritance.

- [ ] **Step 7:** Run `./scripts/verify.sh`. Expected: green. If `npm run check` complains about unused imports, remove them — but do NOT remove `foundryFeatures`, `foundryEffects`, etc. unless certain they're not used by the toolbar / setup-guide block.

- [ ] **Step 8:** Manual smoke test. Run `npm run tauri dev` and open the app:
  1. With a Foundry actor connected: the live grid renders cards in the new dossier styling. Default view = Basics. Click ‹ and › to flip between all four views; pager shows `1 / 4` … `4 / 4`. Density toggle (Auto / S / M / L) scales the cards proportionally.
  2. Click Save locally → card persists; the drift badge stays absent until edits diverge.
  3. Click ± on a stepper (hunger, humanity, stains, health/willpower sup/agg, BP) — the value should change live in Foundry.
  4. Add a merit on View 4 — the chip appears. Remove it — chip disappears.
  5. Click Compare on a saved card with drift — the existing `CompareModal` opens and lists differing paths.
  6. Disconnect Foundry, leave a saved character — it appears in the grid via the saved-only render path.

  If any step fails, fix and re-verify before committing.

- [ ] **Step 9:** Commit.

```bash
git add src/tools/Campaign.svelte
git commit -m "$(cat <<'EOF'
refactor(campaign): replace inline character card with new card components

Campaign.svelte loses ~1400 lines (inline markup + supporting helpers + dead
CSS) and gains ~30 lines that delegate to <CharacterCardShell />. Behaviour
preserved: density toggle, save/update/compare, drift detection, stepper
editing, advantage add/remove, source-attribution chip. Visually new (dossier
frame + flip mechanic + four views); functionally identical.

The four advantage-chip placeholder data-active="false" attributes ship the
inert CSS active-state treatment; Plan B wires the modifier subscription that
flips them to "true".

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md
Closes-prep-for: Plan B — Modifier integration
EOF
)"
```

---

## Self-Review Checklist (run after Task 4 commit)

Before declaring Plan A complete, verify:

- [ ] **Spec coverage:** Every §3, §4, §5 requirement of the spec is implemented in `CharacterCard.svelte`. Specifically: dossier frame with persistent file label / SUBJECT name / clan line / panel / flipper; fixed 2:3 aspect; `--card-scale`-driven proportional sizing; four panels with the content described; skill filter on View 2; stepper preservation on View 1; chip kinds color-coded on View 4.
- [ ] **Anti-scope respected:** No changes to `db/modifier.rs`, `src/types.ts` modifier types, `modifiers.svelte.ts`, `ModifierEffectEditor.svelte`, or any modifier-store wiring. Plan B owns those.
- [ ] **Token discipline:** Search the new files for hex literals (`grep -E '#[0-9a-fA-F]{3,6}' src/lib/components/CharacterCard.svelte src/lib/components/CharacterCardShell.svelte`). Result must be empty (every color via `var(--*)`).
- [ ] **Card-scale propagation:** Inspect the rendered card under S / M / L densities. Internal proportions should remain identical — only size changes.
- [ ] **No frontend test framework introduced.** ARCH §10 invariant.
- [ ] **`./scripts/verify.sh`** green for the final commit.

If any check fails, fix inline and amend or add a new commit per the relevant CLAUDE.md rules.

---

## Out of scope (handled by Plan B)

- `ModifierKind::Stat` enum variant.
- `computeActiveDeltas()` projection.
- Card subscription to `modifiers.list`.
- Stat-delta annotations on View 2.
- Chip click handlers on View 4 (currently inert — clicking does nothing in Plan A).
- Right-click chip → `ModifierEffectEditor` popover.
- Active-modifiers banner.

These are all handled in `2026-05-10-character-card-redesign-plan-b-modifier-integration.md`.
