# Dyscrasia Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the existing backend CRUD commands into a Dyscrasia Manager sidebar tool and an Acute Panel inside the Resonance Roller.

**Architecture:** Four new Svelte components. `ResultCard.svelte` has existing dyscrasia UI (select dropdown + roll button) that must be fully replaced by `AcutePanel`. All five Tauri commands already exist — this is frontend-only work. Masonry layout uses CSS `column-width` with Svelte `in:scale` / `out:fade` transitions.

**Tech Stack:** Svelte 5 runes (`$state`, `$props`, `$effect`, `$derived`), TypeScript, `@tauri-apps/api/core` invoke, `svelte/transition` (fade, scale), `svelte/easing` (cubicOut).

---

## File Map

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `src/lib/components/DyscrasiaCard.svelte` | Single card for both manager and acute surfaces |
| Create | `src/lib/components/DyscrasiaForm.svelte` | Inline add/edit form card |
| Create | `src/lib/components/AcutePanel.svelte` | Acute dyscrasia picker with re-roll/confirm |
| Create | `src/tools/DyscrasiaManager.svelte` | Manager tool: search, chips, masonry grid |
| Modify | `src/tools.ts` | Register new tool |
| Modify | `src/lib/components/ResultCard.svelte` | Replace existing dyscrasia UI with AcutePanel |

---

### Task 1: DyscrasiaCard component

**Files:**
- Create: `src/lib/components/DyscrasiaCard.svelte`

- [ ] **Step 1: Create `src/lib/components/DyscrasiaCard.svelte`**

```svelte
<script lang="ts">
  import type { DyscrasiaEntry } from '../../types';

  const {
    entry,
    mode,
    state = null,
    onselect,
    onedit,
    ondelete,
  }: {
    entry: DyscrasiaEntry;
    mode: 'manager' | 'acute';
    state?: 'rolled' | 'selected' | null;
    onselect?: () => void;
    onedit?: () => void;
    ondelete?: () => void;
  } = $props();

  let descEl: HTMLElement | undefined = $state(undefined);
  let overflows = $state(false);
  let expanded = $state(false);

  $effect(() => {
    if (!descEl) return;
    const check = () => {
      overflows = (descEl?.scrollHeight ?? 0) > (descEl?.offsetHeight ?? 0) + 10;
    };
    check();
    const ro = new ResizeObserver(check);
    ro.observe(descEl);
    return () => ro.disconnect();
  });
</script>

<div
  class="card"
  class:rolled={state === 'rolled'}
  class:selected={state === 'selected'}
  class:clickable={mode === 'acute'}
  onclick={mode === 'acute' ? onselect : undefined}
  role={mode === 'acute' ? 'button' : undefined}
  tabindex={mode === 'acute' ? 0 : undefined}
>
  {#if state === 'rolled'}
    <span class="state-badge">rolled</span>
  {:else if state === 'selected'}
    <span class="state-badge selected-badge">selected ✓</span>
  {/if}

  {#if mode === 'manager'}
    <div class="type-badge type-{entry.resonanceType.toLowerCase()}">{entry.resonanceType}</div>
  {/if}

  <div class="name">{entry.name}</div>

  <div
    class="desc"
    class:clipped={overflows && !expanded}
    bind:this={descEl}
  >
    {entry.description}
  </div>

  {#if overflows && !expanded}
    <button
      class="show-more"
      onclick={(e) => { e.stopPropagation(); expanded = true; }}
    >
      show more ▾
    </button>
  {/if}

  <div class="footer">
    <span class="bonus">{entry.bonus}</span>
    {#if mode === 'manager'}
      {#if entry.isCustom}
        <div class="actions">
          <button class="action-btn edit" onclick={(e) => { e.stopPropagation(); onedit?.(); }}>✎ Edit</button>
          <button class="action-btn del"  onclick={(e) => { e.stopPropagation(); ondelete?.(); }}>✕</button>
        </div>
      {:else}
        <span class="builtin-badge">built-in</span>
      {/if}
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    padding: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    break-inside: avoid;
    margin-bottom: 0.75rem;
    width: 100%;
    position: relative;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .card.clickable { cursor: pointer; }
  .card.clickable:hover { border-color: var(--border-surface); box-shadow: 0 2px 14px #0009; }
  .card:not(.clickable):hover { border-color: var(--border-surface); }

  .card.rolled {
    border-color: var(--accent);
    background: #1a0808;
    box-shadow: 0 0 12px #cc222233, inset 0 0 18px #cc22220a;
  }
  .card.selected {
    border-color: var(--accent-amber);
    background: #1a1206;
    box-shadow: 0 0 12px #cc992233, inset 0 0 18px #cc99220a;
  }

  .state-badge {
    position: absolute;
    top: 0.5rem;
    right: 0.6rem;
    font-size: 0.52rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--accent);
    opacity: 0.8;
  }
  .state-badge.selected-badge { color: var(--accent-amber); opacity: 0.9; }

  .type-badge {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    font-weight: 600;
  }
  .type-badge.type-phlegmatic { color: #7090c0; }
  .type-badge.type-melancholy { color: #9070b0; }
  .type-badge.type-choleric   { color: var(--accent); }
  .type-badge.type-sanguine   { color: var(--accent-amber); }

  .name {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.2;
    padding-right: 2rem;
  }

  .desc {
    font-size: 0.72rem;
    color: var(--text-secondary);
    line-height: 1.5;
    flex: 1;
  }
  .desc.clipped {
    max-height: 260px;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, black 70%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, black 70%, transparent 100%);
  }

  .show-more {
    align-self: flex-start;
    font-size: 0.62rem;
    color: var(--text-ghost);
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
    transition: color 0.15s;
    margin-top: -0.1rem;
  }
  .show-more:hover { color: var(--text-label); }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.2rem;
    padding-top: 0.4rem;
    border-top: 1px solid var(--border-faint);
    gap: 0.5rem;
  }
  .bonus { font-size: 0.64rem; color: var(--text-label); flex: 1; }

  .actions { display: flex; gap: 0.3rem; flex-shrink: 0; }
  .action-btn {
    font-size: 0.6rem;
    padding: 0.18rem 0.38rem;
    border-radius: 3px;
    border: 1px solid;
    cursor: pointer;
    background: none;
    transition: background 0.15s, box-shadow 0.15s, transform 0.1s;
  }
  .action-btn:active { transform: scale(0.87); }
  .action-btn.edit { border-color: #4a3a1a; color: var(--accent-amber); }
  .action-btn.edit:hover { background: #1a1206; box-shadow: 0 0 6px #cc992244; }
  .action-btn.del  { border-color: #3a1010; color: var(--accent); }
  .action-btn.del:hover  { background: #1a0505; box-shadow: 0 0 6px #cc222244; }

  .builtin-badge {
    font-size: 0.55rem;
    color: var(--text-ghost);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.1rem 0.3rem;
    flex-shrink: 0;
  }
</style>
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/DyscrasiaCard.svelte
git commit -m "feat: add DyscrasiaCard component"
```

---

### Task 2: DyscrasiaForm component

**Files:**
- Create: `src/lib/components/DyscrasiaForm.svelte`

- [ ] **Step 1: Create `src/lib/components/DyscrasiaForm.svelte`**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { DyscrasiaEntry } from '../../types';

  const {
    entry = null,
    oncancel,
    onsave,
  }: {
    entry?: DyscrasiaEntry | null;
    oncancel: () => void;
    onsave: (saved: DyscrasiaEntry) => void;
  } = $props();

  const TYPES = ['Phlegmatic', 'Melancholy', 'Choleric', 'Sanguine'] as const;

  let resonanceType = $state(entry?.resonanceType ?? 'Phlegmatic');
  let name         = $state(entry?.name ?? '');
  let description  = $state(entry?.description ?? '');
  let bonus        = $state(entry?.bonus ?? '');
  let saving = $state(false);
  let error  = $state('');

  async function save() {
    if (!name.trim() || !description.trim() || !bonus.trim()) {
      error = 'All fields are required.';
      return;
    }
    saving = true;
    error = '';
    try {
      let saved: DyscrasiaEntry;
      if (entry) {
        await invoke<void>('update_dyscrasia', {
          id: entry.id,
          name: name.trim(),
          description: description.trim(),
          bonus: bonus.trim(),
        });
        saved = { ...entry, name: name.trim(), description: description.trim(), bonus: bonus.trim() };
      } else {
        saved = await invoke<DyscrasiaEntry>('add_dyscrasia', {
          resonanceType,
          name: name.trim(),
          description: description.trim(),
          bonus: bonus.trim(),
        });
      }
      onsave(saved);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form-card">
  <div class="form-title">{entry ? 'Edit Dyscrasia' : 'Add Custom Dyscrasia'}</div>

  <div class="field">
    <label for="rtype">Resonance Type</label>
    <select id="rtype" bind:value={resonanceType} disabled={!!entry}>
      {#each TYPES as t}
        <option value={t}>{t}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="dname">Name</label>
    <input id="dname" type="text" bind:value={name} placeholder="e.g. Unshakeable Calm" />
  </div>

  <div class="field">
    <label for="ddesc">Description</label>
    <textarea id="ddesc" bind:value={description} rows={3} placeholder="Flavour text…"></textarea>
  </div>

  <div class="field">
    <label for="dbonus">Bonus</label>
    <input id="dbonus" type="text" bind:value={bonus} placeholder="e.g. +2 dice to Fortitude rolls" />
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="form-actions">
    <button class="cancel-btn" onclick={oncancel} disabled={saving}>Cancel</button>
    <button class="save-btn"   onclick={save}     disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>

<style>
  .form-card {
    background: var(--bg-card);
    border: 1px solid var(--border-active);
    border-radius: 8px;
    padding: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    break-inside: avoid;
    margin-bottom: 0.75rem;
  }
  .form-title {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
  }
  .field { display: flex; flex-direction: column; gap: 0.2rem; }
  label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  input, select, textarea {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.35rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
  }
  input:focus, select:focus, textarea:focus { border-color: var(--accent); }
  textarea { resize: vertical; min-height: 4rem; }
  select:disabled { opacity: 0.6; cursor: not-allowed; }
  .error { font-size: 0.68rem; color: var(--accent-bright); }
  .form-actions { display: flex; gap: 0.4rem; justify-content: flex-end; }
  .save-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 4px;
    padding: 0.3rem 0.8rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: box-shadow 0.15s, transform 0.1s;
  }
  .save-btn:hover:not(:disabled) { box-shadow: 0 0 8px #cc222255; }
  .save-btn:active { transform: scale(0.93); }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .cancel-btn {
    background: none;
    border: 1px solid var(--border-surface);
    color: var(--text-muted);
    border-radius: 4px;
    padding: 0.3rem 0.8rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.1s;
  }
  .cancel-btn:hover:not(:disabled) { border-color: var(--border-active); color: var(--text-label); }
  .cancel-btn:active { transform: scale(0.93); }
  .cancel-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/DyscrasiaForm.svelte
git commit -m "feat: add DyscrasiaForm component"
```

---

### Task 3: DyscrasiaManager tool

**Files:**
- Create: `src/tools/DyscrasiaManager.svelte`
- Modify: `src/tools.ts`

- [ ] **Step 1: Create `src/tools/DyscrasiaManager.svelte`**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import DyscrasiaCard from '$lib/components/DyscrasiaCard.svelte';
  import DyscrasiaForm from '$lib/components/DyscrasiaForm.svelte';
  import type { DyscrasiaEntry } from '../types';

  const TYPES = ['Phlegmatic', 'Melancholy', 'Choleric', 'Sanguine'] as const;

  // Per-type active colours — hex acceptable per CLAUDE.md for non-semantic states
  const CHIP_ACTIVE: Record<string, { border: string; text: string; bg: string }> = {
    Phlegmatic: { border: '#5080c0', text: '#a0c0f0', bg: '#0a0f1a' },
    Melancholy: { border: '#7050a0', text: '#c0a0e0', bg: '#0f0a18' },
    Choleric:   { border: 'var(--accent)', text: '#f09090', bg: '#1a0505' },
    Sanguine:   { border: 'var(--accent-amber)', text: '#f0cc70', bg: '#1a1206' },
  };

  let allEntries: DyscrasiaEntry[] = $state([]);
  let loading = $state(true);
  let activeTypes: Set<string> = $state(new Set(['all']));
  let rawSearch = $state('');
  let searchQuery = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let showAddForm = $state(false);
  let editingId: number | null = $state(null);

  const filteredEntries = $derived(
    allEntries.filter(e => {
      const typeMatch = activeTypes.has('all') || activeTypes.has(e.resonanceType);
      const q = searchQuery.toLowerCase();
      const textMatch = !q ||
        e.name.toLowerCase().includes(q) ||
        e.description.toLowerCase().includes(q) ||
        e.bonus.toLowerCase().includes(q);
      return typeMatch && textMatch;
    })
  );

  async function loadAll() {
    loading = true;
    const results = await Promise.all(
      TYPES.map(t => invoke<DyscrasiaEntry[]>('list_dyscrasias', { resonanceType: t }))
    );
    allEntries = results.flat();
    loading = false;
  }

  function toggleType(type: string) {
    if (type === 'all') {
      activeTypes = new Set(['all']);
      return;
    }
    const next = new Set(activeTypes);
    next.delete('all');
    if (next.has(type)) next.delete(type);
    else next.add(type);
    if (next.size === 0) next.add('all');
    activeTypes = next;
  }

  function onSearchInput(e: Event) {
    rawSearch = (e.target as HTMLInputElement).value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = rawSearch; }, 110);
  }

  function handleSave() {
    showAddForm = false;
    editingId = null;
    loadAll();
  }

  function handleDelete(id: number) {
    invoke<void>('delete_dyscrasia', { id }).then(() => loadAll());
  }

  $effect(() => { loadAll(); });
</script>

<div class="page">
  <h1 class="title">Dyscrasias</h1>

  <div class="controls">
    <input
      class="search"
      type="text"
      value={rawSearch}
      oninput={onSearchInput}
      placeholder="Search by name or description…"
    />
    <button
      class="add-btn"
      onclick={() => { showAddForm = !showAddForm; editingId = null; }}
    >
      {showAddForm ? '✕ Cancel' : '+ Add Custom'}
    </button>
  </div>

  <div class="chips">
    <button
      class="chip"
      class:active={activeTypes.has('all')}
      onclick={() => toggleType('all')}
    >All</button>
    {#each TYPES as type}
      {@const colors = CHIP_ACTIVE[type]}
      {@const isActive = activeTypes.has(type)}
      <button
        class="chip"
        class:active={isActive}
        style={isActive
          ? `border-color:${colors.border};color:${colors.text};background:${colors.bg};box-shadow:0 0 8px ${colors.border}44`
          : ''}
        onclick={() => toggleType(type)}
      >{type}</button>
    {/each}
  </div>

  <p class="results-label">Showing {filteredEntries.length} dyscrasias</p>

  {#if loading}
    <p class="loading-text">Loading…</p>
  {:else}
    <div class="masonry">
      {#if showAddForm}
        <div
          in:scale={{ start: 0.9, duration: 200, easing: cubicOut }}
          out:fade={{ duration: 150 }}
        >
          <DyscrasiaForm
            oncancel={() => { showAddForm = false; }}
            onsave={handleSave}
          />
        </div>
      {/if}

      {#each filteredEntries as entry, i (entry.id)}
        {#if editingId === entry.id}
          <div
            in:scale={{ start: 0.9, duration: 200, easing: cubicOut }}
            out:fade={{ duration: 150 }}
          >
            <DyscrasiaForm
              {entry}
              oncancel={() => { editingId = null; }}
              onsave={handleSave}
            />
          </div>
        {:else}
          <div
            in:scale={{ start: 0.88, duration: 220, delay: i * 30, easing: cubicOut }}
            out:fade={{ duration: 180 }}
          >
            <DyscrasiaCard
              {entry}
              mode="manager"
              onedit={() => { editingId = entry.id; showAddForm = false; }}
              ondelete={() => handleDelete(entry.id)}
            />
          </div>
        {/if}
      {/each}

      {#if filteredEntries.length === 0 && !showAddForm}
        <p class="empty" transition:fade>No dyscrasias match your search.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page { padding: 1rem 1.25rem; }
  .title { color: var(--accent); font-size: 1.4rem; margin-bottom: 1rem; }

  .controls { display: flex; gap: 0.6rem; margin-bottom: 0.75rem; align-items: center; }

  .search {
    flex: 1;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 5px;
    padding: 0.5rem 0.75rem;
    color: var(--text-primary);
    font-size: 0.82rem;
    outline: none;
    transition: border-color 0.2s, box-shadow 0.2s;
  }
  .search:focus { border-color: var(--accent); box-shadow: 0 0 0 2px #cc222218; }
  .search::placeholder { color: var(--text-ghost); }

  .add-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 5px;
    padding: 0.5rem 0.9rem;
    font-size: 0.78rem;
    cursor: pointer;
    white-space: nowrap;
    transition: box-shadow 0.15s, transform 0.12s;
  }
  .add-btn:hover { box-shadow: 0 0 10px #cc222255; }
  .add-btn:active { transform: scale(0.93); }

  .chips { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-bottom: 0.75rem; }

  .chip {
    padding: 0.28rem 0.7rem;
    border-radius: 20px;
    font-size: 0.72rem;
    border: 1px solid var(--border-card);
    color: var(--text-label);
    background: var(--bg-card);
    cursor: pointer;
    transition: border-color 0.18s, color 0.18s, background 0.18s, box-shadow 0.15s, transform 0.12s;
  }
  .chip:hover { border-color: var(--border-surface); color: var(--text-primary); }
  .chip:active { transform: scale(0.87); }
  .chip.active { border-color: var(--text-label); color: var(--text-primary); background: #1a1006; }

  .results-label { font-size: 0.68rem; color: var(--text-ghost); margin-bottom: 0.75rem; }
  .loading-text  { color: var(--text-ghost); font-size: 0.8rem; }

  .masonry {
    column-width: 200px;
    column-gap: 0.75rem;
  }
  /* transition wrapper divs must not break across columns */
  .masonry > div { break-inside: avoid; }

  .empty {
    color: var(--text-ghost);
    font-size: 0.8rem;
    text-align: center;
    padding: 2rem;
    column-span: all;
  }
</style>
```

- [ ] **Step 2: Register tool in `src/tools.ts`**

Add after the existing resonance entry:

```ts
  {
    id: 'dyscrasias',
    label: 'Dyscrasias',
    icon: '📋',
    component: () => import('./tools/DyscrasiaManager.svelte'),
  },
```

The full updated `tools` array:

```ts
export const tools: Tool[] = [
  {
    id: 'resonance',
    label: 'Resonance Roller',
    icon: '🩸',
    component: () => import('./tools/Resonance.svelte'),
  },
  {
    id: 'dyscrasias',
    label: 'Dyscrasias',
    icon: '📋',
    component: () => import('./tools/DyscrasiaManager.svelte'),
  },
];
```

- [ ] **Step 3: Type-check**

```bash
npm run check
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/tools/DyscrasiaManager.svelte src/tools.ts
git commit -m "feat: add Dyscrasia Manager tool"
```

---

### Task 4: AcutePanel component

**Files:**
- Create: `src/lib/components/AcutePanel.svelte`

- [ ] **Step 1: Create `src/lib/components/AcutePanel.svelte`**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import DyscrasiaCard from '$lib/components/DyscrasiaCard.svelte';
  import type { DyscrasiaEntry } from '../../types';

  const {
    resonanceType,
    initialDyscrasia,
    onconfirm,
  }: {
    resonanceType: string;
    initialDyscrasia: DyscrasiaEntry | null;
    onconfirm: (d: DyscrasiaEntry) => void;
  } = $props();

  let entries: DyscrasiaEntry[] = $state([]);
  let rolledId: number | null = $state(initialDyscrasia?.id ?? null);
  let selectedId: number | null = $state(initialDyscrasia?.id ?? null);
  let rawSearch = $state('');
  let searchQuery = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  const selectedEntry = $derived(entries.find(e => e.id === selectedId) ?? null);

  const filteredEntries = $derived(
    entries.filter(e => {
      const q = searchQuery.toLowerCase();
      return !q ||
        e.name.toLowerCase().includes(q) ||
        e.description.toLowerCase().includes(q);
    })
  );

  $effect(() => {
    invoke<DyscrasiaEntry[]>('list_dyscrasias', { resonanceType }).then(list => {
      entries = list;
    });
  });

  async function reroll() {
    const rolled = await invoke<DyscrasiaEntry | null>('roll_random_dyscrasia', { resonanceType });
    if (rolled) {
      rolledId   = rolled.id;
      selectedId = rolled.id;
    }
  }

  function onSearchInput(e: Event) {
    rawSearch = (e.target as HTMLInputElement).value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = rawSearch; }, 110);
  }

  function cardState(entry: DyscrasiaEntry): 'rolled' | 'selected' | null {
    if (entry.id === rolledId && entry.id === selectedId) return 'rolled';
    if (entry.id === selectedId) return 'selected';
    return null;
  }
</script>

<div class="acute-panel">
  <div class="panel-header">
    <span class="panel-title">
      Dyscrasias — <span class="type-label">{resonanceType}</span>
    </span>
    <div class="panel-controls">
      <input
        class="search"
        type="text"
        value={rawSearch}
        oninput={onSearchInput}
        placeholder="filter…"
      />
      <button class="reroll-btn" onclick={reroll}>⟳ Re-roll</button>
    </div>
  </div>

  <div class="masonry">
    {#each filteredEntries as entry (entry.id)}
      <div
        in:scale={{ start: 0.88, duration: 200, easing: cubicOut }}
        out:fade={{ duration: 150 }}
      >
        <DyscrasiaCard
          {entry}
          mode="acute"
          state={cardState(entry)}
          onselect={() => { selectedId = entry.id; }}
        />
      </div>
    {/each}
  </div>

  <div class="panel-footer">
    <span class="summary">
      {#if selectedEntry}
        {selectedId === rolledId ? 'Auto-rolled:' : 'Selected:'}
        <strong>{selectedEntry.name}</strong>
      {:else}
        No dyscrasia selected.
      {/if}
    </span>
    <button
      class="confirm-btn"
      disabled={!selectedEntry}
      onclick={() => selectedEntry && onconfirm(selectedEntry)}
    >
      Confirm
    </button>
  </div>
</div>

<style>
  .acute-panel {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 6px;
    padding: 1rem 1.1rem;
    margin-top: 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .panel-title {
    font-size: 0.78rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
  }
  .type-label { color: var(--accent); }

  .panel-controls { display: flex; align-items: center; gap: 0.5rem; }

  .search {
    background: var(--bg-sunken);
    border: 1px solid var(--border-card);
    border-radius: 4px;
    padding: 0.3rem 0.6rem;
    color: var(--text-primary);
    font-size: 0.72rem;
    width: 140px;
    outline: none;
    transition: border-color 0.15s;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-ghost); }

  .reroll-btn {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.65rem;
    font-size: 0.72rem;
    cursor: pointer;
    white-space: nowrap;
    transition: border-color 0.15s, color 0.15s, box-shadow 0.15s, transform 0.1s;
  }
  .reroll-btn:hover { border-color: var(--accent); color: var(--text-primary); box-shadow: 0 0 6px #cc222233; }
  .reroll-btn:active { transform: scale(0.93); }

  .masonry { column-width: 180px; column-gap: 0.6rem; }
  .masonry > div { break-inside: avoid; }

  .panel-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 0.65rem;
    border-top: 1px solid var(--border-card);
    gap: 0.5rem;
  }
  .summary { font-size: 0.72rem; color: var(--text-label); }
  .summary strong { color: var(--accent-amber); }

  .confirm-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 4px;
    padding: 0.35rem 0.8rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: box-shadow 0.15s, transform 0.1s;
  }
  .confirm-btn:hover:not(:disabled) { box-shadow: 0 0 8px #cc222255; }
  .confirm-btn:active { transform: scale(0.93); }
  .confirm-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/AcutePanel.svelte
git commit -m "feat: add AcutePanel component"
```

---

### Task 5: Wire AcutePanel into ResultCard

**Files:**
- Modify: `src/lib/components/ResultCard.svelte`

This task fully replaces the existing dyscrasia UI in ResultCard (select dropdown, "Roll randomly" button, `loadDyscrasias`/`rollRandomDyscrasia` functions) with `AcutePanel`. It also changes the temperament display to show "ACUTE" when `result.isAcute`, and removes the separate "Acute check" row.

- [ ] **Step 1: Replace `src/lib/components/ResultCard.svelte` with the following**

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import AcutePanel from '$lib/components/AcutePanel.svelte';
  import type { ResonanceRollResult, DyscrasiaEntry } from '../../types';

  const { result }: { result: ResonanceRollResult } = $props();

  let confirmedDyscrasia: DyscrasiaEntry | null = $state(null);
  let acuteConfirmed = $state(false);

  // Reset when a new result arrives
  $effect(() => {
    // Reading result.isAcute makes result a dependency; assignments below
    // don't create a cycle because confirmedDyscrasia/acuteConfirmed
    // are not read in this effect.
    if (result) {
      confirmedDyscrasia = null;
      acuteConfirmed = false;
    }
  });

  async function exportToMd() {
    await invoke('export_result_to_md', { result, dyscrasia: confirmedDyscrasia });
  }
</script>

<div class="result-card">
  <div class="result-row">
    <span class="label">Temperament</span>
    <span class="value {result.isAcute ? 'acute' : result.temperament}">
      {result.isAcute ? 'ACUTE' : result.temperament.toUpperCase()}
      <span class="dice-info">
        (rolled {result.temperamentDie}
        {#if result.temperamentDice.length > 1}
          from [{result.temperamentDice.join(', ')}]
        {/if})
      </span>
    </span>
  </div>

  {#if result.resonanceType}
    <div class="result-row">
      <span class="label">Resonance</span>
      <span class="value">{result.resonanceType}</span>
    </div>
  {/if}

  {#if acuteConfirmed && confirmedDyscrasia}
    <div class="result-row">
      <span class="label">Dyscrasia</span>
      <span class="value" style="color: var(--accent-amber)">{confirmedDyscrasia.name}</span>
    </div>
  {/if}

  <div class="card-footer">
    <button class="export-btn" onclick={exportToMd}>Export to .md</button>
  </div>
</div>

{#if result.isAcute && !acuteConfirmed && result.resonanceType}
  <AcutePanel
    resonanceType={result.resonanceType}
    initialDyscrasia={result.dyscrasia}
    onconfirm={(d) => { confirmedDyscrasia = d; acuteConfirmed = true; }}
  />
{/if}

<style>
  .result-card {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 6px;
    padding: 1.25rem;
    margin-top: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .result-row { display: flex; align-items: baseline; gap: 1rem; }
  .label { width: 6.875rem; color: var(--text-label); font-size: 0.85rem; flex-shrink: 0; }
  .value { font-size: 1rem; color: var(--text-primary); font-weight: 600; }
  .value.negligible { color: var(--temp-negligible); }
  .value.fleeting   { color: var(--accent-amber); }
  .value.intense    { color: var(--accent); }
  .value.acute      { color: var(--accent-bright); text-shadow: 0 0 8px #cc222288; }
  .dice-info { font-size: 0.8rem; color: var(--text-muted); font-weight: 400; margin-left: 0.5rem; }
  .card-footer { display: flex; justify-content: flex-end; border-top: 1px solid var(--border-surface); padding-top: 0.75rem; }
  .export-btn {
    padding: 0.3rem 0.8rem;
    background: #1a1a0d;
    border: 1px solid #6a5a20;
    color: var(--accent-amber);
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.8rem;
  }
</style>
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/ResultCard.svelte
git commit -m "feat: replace dyscrasia UI with AcutePanel in ResultCard"
```

---

## Verification

Run the app and walk through every surface:

```bash
npm run tauri dev
```

**Dyscrasia Manager:**
1. Click "Dyscrasias" in the sidebar — 8 built-in cards appear in masonry columns, staggered fade-in.
2. Click "Phlegmatic" chip — other cards fade out, Phlegmatic cards remain.
3. Click "Choleric" — both Phlegmatic and Choleric visible (multi-select).
4. Click "All" — all cards return.
5. Type in the search box — cards filter in real time (110ms debounce).
6. Click "+ Add Custom" — inline form card appears at top of grid.
7. Fill all fields and Save — new card appears in the correct resonance type section, form disappears.
8. Click "✎ Edit" on the new card — form appears pre-filled, resonance type select is disabled.
9. Edit name/description/bonus and Save — card updates.
10. Click "✕" on the custom card — card disappears.

**Resonance Roller — Acute:**
11. Roll until Acute fires (may take several attempts). If impatient, temporarily lower the acute threshold in the Rust code to force it, then revert.
12. Temperament row shows **"ACUTE"** in bright red glow. No separate "Acute check" row.
13. AcutePanel appears below the result card, showing cards for the rolled resonance type.
14. The auto-rolled dyscrasia card has a red border glow and "rolled" badge.
15. Click a different card — it turns gold with "selected ✓" badge. Summary line updates.
16. Click "⟳ Re-roll" — rolled highlight moves to a new randomly chosen card.
17. Click "Confirm" — AcutePanel disappears. Result card shows "Dyscrasia: [name]" in amber.
18. Export to .md — includes the confirmed dyscrasia.

**Type-check:**
```bash
npm run check
```
Expected: no errors.
