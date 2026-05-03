<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge, initBridge } from '../store/bridge.svelte';
  import { savedCharacters } from '../store/savedCharacters.svelte';
  import { modifiers } from '../store/modifiers.svelte';
  import TagFilterBar from '$lib/components/gm-screen/TagFilterBar.svelte';
  import CharacterRow from '$lib/components/gm-screen/CharacterRow.svelte';
  import type { BridgeCharacter, SourceKind } from '../types';

  onMount(() => {
    void initBridge();
    void savedCharacters.ensureLoaded();
    void modifiers.ensureLoaded();
  });

  // All tags currently in use across all materialized modifiers — drives the chip bar.
  let allTags = $derived(
    [...new Set(modifiers.list.flatMap(m => m.tags))].sort()
  );

  function toggleTag(t: string): void {
    const next = new Set(modifiers.activeFilterTags);
    if (next.has(t)) next.delete(t); else next.add(t);
    modifiers.setActiveFilterTags(next);
  }

  function clearFilter(): void {
    modifiers.setActiveFilterTags(new Set());
  }

  // Orphans = modifier rows whose (source, source_id) matches no live or saved char.
  let orphans = $derived(
    modifiers.list.filter(m => {
      const liveMatch = bridge.characters.some(
        c => c.source === m.source && c.source_id === m.sourceId
      );
      const savedMatch = savedCharacters.list.some(
        s => s.source === m.source && s.sourceId === m.sourceId
      );
      return !liveMatch && !savedMatch;
    })
  );

  // Synthesize a BridgeCharacter shell for a saved-only character so CharacterRow renders it.
  function savedAsBridge(s: { source: SourceKind; sourceId: string; canonical: BridgeCharacter }): BridgeCharacter {
    return s.canonical;
  }

  // Combined character list: live first, then saved-only (no live match).
  let displayCharacters = $derived.by((): BridgeCharacter[] => {
    const live = bridge.characters;
    const liveKeys = new Set(live.map(c => `${c.source}:${c.source_id}`));
    const savedOnly = savedCharacters.list
      .filter(s => !liveKeys.has(`${s.source}:${s.sourceId}`))
      .map(savedAsBridge);
    return [...live, ...savedOnly];
  });
</script>

<div class="gm-screen">
  <header class="title-bar">
    <h1>🛡 GM Screen</h1>
    <div class="toggles">
      <label>
        <input
          type="checkbox"
          checked={modifiers.showHidden}
          onchange={(e) => modifiers.showHidden = (e.currentTarget as HTMLInputElement).checked}
        /> Show hidden
      </label>
      <label>
        <input
          type="checkbox"
          checked={modifiers.showOrphans}
          onchange={(e) => modifiers.showOrphans = (e.currentTarget as HTMLInputElement).checked}
        /> Show orphans
      </label>
    </div>
  </header>

  <TagFilterBar
    {allTags}
    activeTags={modifiers.activeFilterTags}
    onToggleTag={toggleTag}
    onClearAll={clearFilter}
  />

  <div class="rows">
    {#if displayCharacters.length === 0}
      <p class="empty">No characters available. Connect Foundry or Roll20, or load a saved character.</p>
    {:else}
      {#each displayCharacters as char (`${char.source}:${char.source_id}`)}
        <CharacterRow
          character={char}
          activeFilterTags={modifiers.activeFilterTags}
          showHidden={modifiers.showHidden}
        />
      {/each}
    {/if}

    {#if modifiers.showOrphans && orphans.length > 0}
      <section class="orphans">
        <h2>Orphans ({orphans.length})</h2>
        <p class="hint">Modifier rows whose character isn't currently live or saved.</p>
        {#each orphans as o}
          <div class="orphan-row">
            <span>{o.name}</span>
            <span class="meta">{o.source}:{o.sourceId}</span>
            <button onclick={() => modifiers.delete(o.id)}>Delete</button>
          </div>
        {/each}
      </section>
    {/if}
  </div>
</div>

<style>
  .gm-screen {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-base);
    color: var(--text-primary);
    box-sizing: border-box;
  }
  .title-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .title-bar h1 { margin: 0; font-size: 1.05rem; }
  .toggles { display: flex; gap: 1rem; font-size: 0.8rem; color: var(--text-secondary); }
  .toggles label { display: inline-flex; gap: 0.3rem; align-items: center; cursor: pointer; }
  .rows { flex: 1; overflow-y: auto; padding: 0.75rem 1rem; }
  .empty { color: var(--text-muted); font-style: italic; }
  .orphans { margin-top: 1rem; padding-top: 0.75rem; border-top: 1px solid var(--border-faint); }
  .orphans h2 { font-size: 0.85rem; margin: 0 0 0.4rem 0; color: var(--text-label); }
  .orphans .hint { font-size: 0.7rem; color: var(--text-muted); margin: 0 0 0.5rem 0; }
  .orphan-row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.4rem 0;
    font-size: 0.8rem;
  }
  .meta { color: var(--text-muted); font-size: 0.7rem; font-family: monospace; }
</style>
