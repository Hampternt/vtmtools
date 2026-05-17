<script lang="ts">
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { scale, fade } from 'svelte/transition';
  import { cubicOut } from 'svelte/easing';
  import AdvantageCard from '$lib/components/AdvantageCard.svelte';
  import AdvantageForm from '$lib/components/AdvantageForm.svelte';
  import { listAdvantages, deleteAdvantage } from '$lib/advantages/api';
  import type { Advantage, Field } from '../types';

  type SortKey = 'name-asc' | 'name-desc' | 'level-asc' | 'level-desc' | 'recent';

  let allEntries: Advantage[] = $state([]);
  let loading       = $state(true);
  let loadError     = $state('');
  let activeTags: Set<string> = $state(new Set(['__all__']));
  let rawSearch     = $state('');
  let searchQuery   = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let sortKey: SortKey = $state('name-asc');
  let showAddForm   = $state(false);
  let editingId: number | null = $state(null);

  // ---- helpers --------------------------------------------------------------

  function findField(adv: Advantage, name: string): Field | undefined {
    return adv.properties.find(p => p.name === name);
  }

  function levelNumber(adv: Advantage): number | null {
    const levelField = findField(adv, 'level');
    if (levelField && levelField.type === 'number') {
      return Array.isArray(levelField.value) ? levelField.value[0] ?? null : levelField.value;
    }
    const minField = findField(adv, 'min_level');
    if (minField && minField.type === 'number') {
      return Array.isArray(minField.value) ? minField.value[0] ?? null : minField.value;
    }
    return null;
  }

  function matchesTags(adv: Advantage): boolean {
    if (activeTags.has('__all__')) return true;
    return adv.tags.some(t => activeTags.has(t));
  }

  function matchesQuery(adv: Advantage): boolean {
    const q = searchQuery.toLowerCase();
    if (!q) return true;
    if (adv.name.toLowerCase().includes(q)) return true;
    if (adv.description.toLowerCase().includes(q)) return true;
    if (adv.tags.some(t => t.toLowerCase().includes(q))) return true;
    return false;
  }

  function sortRows(rows: Advantage[]): Advantage[] {
    const copy = [...rows];
    switch (sortKey) {
      case 'name-asc':  return copy.sort((a, b) => a.name.localeCompare(b.name));
      case 'name-desc': return copy.sort((a, b) => b.name.localeCompare(a.name));
      case 'recent':    return copy.sort((a, b) => b.id - a.id);
      case 'level-asc':
      case 'level-desc': {
        const dir = sortKey === 'level-asc' ? 1 : -1;
        return copy.sort((a, b) => {
          const la = levelNumber(a);
          const lb = levelNumber(b);
          if (la === null && lb === null) return 0;
          if (la === null) return 1;   // missing-level rows always last
          if (lb === null) return -1;
          return dir * (la - lb);
        });
      }
    }
  }

  // ---- derived state --------------------------------------------------------

  const distinctTags = $derived(
    [...new Set(allEntries.flatMap(e => e.tags))].sort((a, b) => a.localeCompare(b))
  );

  /**
   * Filter-row Option A (per Phase 4 Library Sync plan): kind tags
   * already filter implicitly via the existing tag-chip mechanism.
   * We just give those specific filter chips a `data-kind-tag`
   * attribute so CSS can color-code them, matching the per-row
   * kind-chip palette in AdvantageCard.svelte. No new filter state.
   */
  const KIND_TAG_LOOKUP: Record<string, string> = {
    Merit:      'merit',
    Flaw:       'flaw',
    Background: 'background',
    Boon:       'boon',
  };
  function kindTagValue(tag: string): string | undefined {
    return KIND_TAG_LOOKUP[tag];
  }

  const visible = $derived(
    sortRows(allEntries.filter(e => matchesTags(e) && matchesQuery(e)))
  );

  // ---- actions --------------------------------------------------------------

  async function loadAll() {
    loading = true;
    loadError = '';
    try {
      allEntries = await listAdvantages();
    } catch (e) {
      loadError = String(e);
    } finally {
      loading = false;
    }
  }

  function toggleTag(tag: string) {
    if (tag === '__all__') {
      activeTags = new Set(['__all__']);
      return;
    }
    const next = new Set(activeTags);
    next.delete('__all__');
    if (next.has(tag)) next.delete(tag);
    else next.add(tag);
    if (next.size === 0) next.add('__all__');
    activeTags = next;
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

  async function handleDelete(id: number) {
    try {
      await deleteAdvantage(id);
      loadAll();
    } catch (e) {
      loadError = String(e);
    }
  }

  $effect(() => { untrack(() => loadAll()); });
  $effect(() => { return () => { if (searchTimer) clearTimeout(searchTimer); }; });
</script>

<div class="page">
  <h1 class="title">Advantages</h1>

  <div class="controls">
    <input
      class="search"
      type="text"
      value={rawSearch}
      oninput={onSearchInput}
      placeholder="Search by name, description, or tag…"
    />
    <select class="sort" bind:value={sortKey}>
      <option value="name-asc">Name A–Z</option>
      <option value="name-desc">Name Z–A</option>
      <option value="level-asc">Level ↑</option>
      <option value="level-desc">Level ↓</option>
      <option value="recent">Recently added</option>
    </select>
    <button class="add-btn" onclick={() => { showAddForm = !showAddForm; editingId = null; }}>
      {showAddForm ? '✕ Cancel' : '+ Add Custom'}
    </button>
  </div>

  <div class="chips">
    <button
      class="chip"
      class:active={activeTags.has('__all__')}
      onclick={() => toggleTag('__all__')}
    >All</button>
    {#each distinctTags as tag}
      <button
        class="chip"
        class:active={activeTags.has(tag)}
        data-kind-tag={kindTagValue(tag)}
        onclick={() => toggleTag(tag)}
      >{tag}</button>
    {/each}
  </div>

  <p class="results-label">Showing {visible.length} advantages</p>

  {#if loading}
    <p class="loading-text">Loading…</p>
  {:else if loadError}
    <p class="error-text">{loadError}</p>
  {:else}
    <div class="grid">
      {#if showAddForm}
        <div
          in:scale={{ start: 0.9, duration: 200, easing: cubicOut }}
          out:fade={{ duration: 150 }}
        >
          <AdvantageForm
            oncancel={() => { showAddForm = false; }}
            onsave={handleSave}
          />
        </div>
      {/if}

      {#each visible as entry (entry.id)}
        <div
          animate:flip={{ duration: 300, easing: cubicOut }}
          transition:fade={{ duration: 120 }}
        >
          {#if editingId === entry.id}
            <div
              in:scale={{ start: 0.9, duration: 200, easing: cubicOut }}
              out:fade={{ duration: 150 }}
            >
              <AdvantageForm
                {entry}
                oncancel={() => { editingId = null; }}
                onsave={handleSave}
              />
            </div>
          {:else}
            <AdvantageCard
              {entry}
              onedit={() => { editingId = entry.id; showAddForm = false; }}
              ondelete={() => handleDelete(entry.id)}
            />
          {/if}
        </div>
      {/each}

      {#if visible.length === 0 && !showAddForm}
        <p class="empty" transition:fade>No advantages match your filters.</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .page   { padding: 1rem 1.25rem; }
  .title  { color: var(--accent); font-size: 1.4rem; margin-bottom: 1rem; }

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
    box-sizing: border-box;
  }
  .search:focus { border-color: var(--accent); }
  .sort {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 5px;
    padding: 0.45rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
  }
  .add-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 5px;
    padding: 0.5rem 0.9rem;
    font-size: 0.78rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .chips { display: flex; flex-wrap: wrap; gap: 0.4rem; margin-bottom: 0.75rem; }
  .chip {
    padding: 0.28rem 0.7rem;
    border-radius: 20px;
    font-size: 0.72rem;
    border: 1px solid var(--border-card);
    color: var(--text-label);
    background: var(--bg-card);
    cursor: pointer;
  }
  .chip:hover  { border-color: var(--border-surface); color: var(--text-primary); }
  .chip.active { border-color: var(--text-label); color: var(--text-primary); background: var(--bg-raised); }

  /* Kind-tag filter chips (Option A): visually distinguish the four
     V5 kind tags from generic free-form tags via a colored left
     border. Color palette matches the per-row kind chip in
     AdvantageCard.svelte, using existing accent tokens as fallbacks
     since dedicated --accent-* kind tokens are not (yet) defined. */
  .chip[data-kind-tag]                              { border-left-width: 3px; padding-left: calc(0.7rem - 2px); }
  .chip[data-kind-tag="merit"]                      { border-left-color: var(--accent-merit,      var(--accent)); }
  .chip[data-kind-tag="flaw"]                       { border-left-color: var(--accent-flaw,       var(--accent-bright)); }
  .chip[data-kind-tag="background"]                 { border-left-color: var(--accent-background, var(--accent-card-dossier)); }
  .chip[data-kind-tag="boon"]                       { border-left-color: var(--accent-boon,       var(--accent-amber)); }

  .results-label { font-size: 0.68rem; color: var(--text-ghost); margin-bottom: 0.75rem; }
  .loading-text  { color: var(--text-ghost); font-size: 0.8rem; }
  .error-text    { color: var(--accent); font-size: 0.8rem; padding: 1rem 0; }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(12.5rem, 1fr));
    gap: 0.75rem;
    align-items: start;
  }
  .empty { color: var(--text-ghost); font-size: 0.8rem; text-align: center; padding: 2rem; grid-column: 1 / -1; }
</style>
