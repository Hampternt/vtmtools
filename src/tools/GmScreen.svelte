<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge, initBridge } from '../store/bridge.svelte';
  import { savedCharacters } from '../store/savedCharacters.svelte';
  import { modifiers } from '../store/modifiers.svelte';
  import { statusTemplates } from '../store/statusTemplates.svelte';
  import TagFilterBar from '$lib/components/gm-screen/TagFilterBar.svelte';
  import CharacterRow from '$lib/components/gm-screen/CharacterRow.svelte';
  import StatusPaletteDock from '$lib/components/gm-screen/StatusPaletteDock.svelte';
  import type { BridgeCharacter, SourceKind } from '../types';

  onMount(() => {
    void initBridge();
    void savedCharacters.ensureLoaded();
    void modifiers.ensureLoaded();
    void statusTemplates.ensureLoaded();
  });

  // Focused character drives the palette dock's apply-to target. Set by
  // clicking a row; cleared only by clicking another row (sticky focus).
  let focusedCharacter = $state<BridgeCharacter | null>(null);

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

  <div class="layout">
    <div class="rows">
      {#if displayCharacters.length === 0}
        <p class="empty">No characters available. Connect Foundry or Roll20, or load a saved character.</p>
      {:else}
        {#each displayCharacters as char (`${char.source}:${char.source_id}`)}
          <!--
            role="button" instead of a real <button> — CharacterRow renders
            modifier-card buttons (cog/push/reset/hide/toggle) inside, and
            <button> nested inside <button> is invalid HTML (the browser
            auto-closes the outer one at the first nested button, breaking
            layout). div + role="button" + keyboard handler preserves
            keyboard a11y without violating nesting rules. Inner button
            clicks bubble up and ALSO set focus — that's intended.
          -->
          <div
            class="row-focus-wrap"
            class:focused={focusedCharacter && focusedCharacter.source === char.source && focusedCharacter.source_id === char.source_id}
            role="button"
            tabindex="0"
            onclick={() => focusedCharacter = char}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); focusedCharacter = char; } }}
          >
            <CharacterRow
              character={char}
              activeFilterTags={modifiers.activeFilterTags}
              showHidden={modifiers.showHidden}
            />
          </div>
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

    <StatusPaletteDock {focusedCharacter} />
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
  .layout { display: flex; flex: 1; min-height: 0; }
  .rows { flex: 1; overflow-y: auto; padding: 0.75rem 1rem; }
  .row-focus-wrap {
    display: block;
    width: 100%;
    /* 2px transparent border reserves the same box on unfocused rows so
       toggling .focused does NOT shift surrounding layout. */
    border: 2px solid transparent;
    border-radius: 0.55rem;
    padding: 0;
    margin-bottom: 0.6rem;
    cursor: pointer;
    text-align: left;
  }
  .row-focus-wrap.focused { border-color: var(--accent-bright); }
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
