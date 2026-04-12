<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { untrack } from 'svelte';
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import DyscrasiaCard from '$lib/components/DyscrasiaCard.svelte';
  import DyscrasiaForm from '$lib/components/DyscrasiaForm.svelte';
  import type { DyscrasiaEntry } from '../types';

  const TYPES = ['phlegmatic', 'melancholy', 'choleric', 'sanguine'] as const;

  // Per-type active colours — hex acceptable per CLAUDE.md for non-semantic states
  const CHIP_ACTIVE: Record<string, { border: string; text: string; bg: string }> = {
    phlegmatic: { border: '#5080c0', text: '#a0c0f0', bg: '#0a0f1a' },
    melancholy: { border: '#7050a0', text: '#c0a0e0', bg: '#0f0a18' },
    choleric:   { border: 'var(--accent)', text: '#f09090', bg: '#1a0505' },
    sanguine:   { border: 'var(--accent-amber)', text: '#f0cc70', bg: '#1a1206' },
  };

  let allEntries: DyscrasiaEntry[] = $state([]);
  let loading = $state(true);
  let loadError = $state('');
  let activeTypes: Set<string> = $state(new Set(['all']));
  let rawSearch = $state('');
  let searchQuery = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let showAddForm = $state(false);
  let editingId: number | null = $state(null);

  const filteredEntries = $derived(
    allEntries.filter(e => {
      const typeMatch = activeTypes.has('all') || activeTypes.has(e.resonanceType.toLowerCase());
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
    loadError = '';
    try {
      const results = await Promise.all(
        TYPES.map(t => invoke<DyscrasiaEntry[]>('list_dyscrasias', { resonanceType: t }))
      );
      allEntries = results.flat();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
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
    invoke<void>('delete_dyscrasia', { id })
      .then(() => loadAll())
      .catch(e => { loadError = String(e); });
  }

  $effect(() => { untrack(() => loadAll()); });

  $effect(() => {
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
    };
  });
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
      >{type.charAt(0).toUpperCase() + type.slice(1)}</button>
    {/each}
  </div>

  <p class="results-label">Showing {filteredEntries.length} dyscrasias</p>

  {#if loading}
    <p class="loading-text">Loading…</p>
  {:else if loadError}
    <p class="error-text">{loadError}</p>
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
  .chip.active { border-color: var(--text-label); color: var(--text-primary); background: var(--bg-raised); }

  .results-label { font-size: 0.68rem; color: var(--text-ghost); margin-bottom: 0.75rem; }
  .loading-text  { color: var(--text-ghost); font-size: 0.8rem; }
  .error-text { color: var(--accent); font-size: 0.8rem; padding: 1rem 0; }

  .masonry {
    column-width: 12.5rem;
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
