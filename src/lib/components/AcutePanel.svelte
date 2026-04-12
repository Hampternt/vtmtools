<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { untrack } from 'svelte';
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
  let rolledId: number | null = $state(untrack(() => initialDyscrasia?.id ?? null));
  let selectedId: number | null = $state(untrack(() => initialDyscrasia?.id ?? null));
  let rawSearch = $state('');
  let searchQuery = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let loadError = $state('');

  const selectedEntry = $derived(
    entries.find(e => e.id === selectedId) ??
    (selectedId === initialDyscrasia?.id ? initialDyscrasia : null)
  );

  const filteredEntries = $derived(
    entries.filter(e => {
      const q = searchQuery.toLowerCase();
      return !q ||
        e.name.toLowerCase().includes(q) ||
        e.description.toLowerCase().includes(q);
    })
  );

  $effect(() => {
    const rt = resonanceType.toLowerCase();
    invoke<DyscrasiaEntry[]>('list_dyscrasias', { resonanceType: rt }).then(list => {
      entries = list;
    }).catch(e => { console.error('list_dyscrasias failed:', e); loadError = String(e); });
  });

  $effect(() => {
    return () => {
      if (searchTimer) clearTimeout(searchTimer);
    };
  });

  async function reroll() {
    try {
      const rolled = await invoke<DyscrasiaEntry | null>('roll_random_dyscrasia', { resonanceType: resonanceType.toLowerCase() });
      if (rolled) {
        rolledId   = rolled.id;
        selectedId = rolled.id;
      }
    } catch (e) {
      console.error('reroll failed:', e);
    }
  }

  function onSearchInput(e: Event) {
    rawSearch = (e.target as HTMLInputElement).value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = rawSearch; }, 110);
  }

  function cardState(entry: DyscrasiaEntry): 'rolled' | 'selected' | null {
    if (entry.id === rolledId) return 'rolled';    // always show rolled (red)
    if (entry.id === selectedId) return 'selected'; // different selected card (gold)
    return null;
  }
</script>

<div class="acute-panel">
  <div class="panel-header">
    <span class="panel-title">
      Dyscrasias — <span class="type-label">{resonanceType.charAt(0).toUpperCase() + resonanceType.slice(1)}</span>
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

  {#if loadError}
    <p class="load-error">{loadError}</p>
  {/if}

  <div class="masonry">
    {#each filteredEntries as entry (entry.id)}
      <div
        in:scale={{ start: 0.88, duration: 200, easing: cubicOut }}
        out:fade={{ duration: 150 }}
      >
        <DyscrasiaCard
          {entry}
          mode="acute"
          cardstate={cardState(entry)}
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
    width: 8.75rem;
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

  .load-error { font-size: 0.72rem; color: var(--accent); padding: 0.5rem 0; }

  .masonry { column-width: 11.25rem; column-gap: 0.6rem; }
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
