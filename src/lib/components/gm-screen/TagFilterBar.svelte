<script lang="ts">
  // GM Screen tag filter chip bar. Empty active set = no filter (spec §7.5).
  // OR semantics: a card matches when it has any of the active tags.
  //
  // Mirrors the chip pattern from src/tools/AdvantagesManager.svelte but
  // without the __all__ sentinel — empty Set is the unfiltered state here.

  interface Props {
    allTags: string[];
    activeTags: Set<string>;
    onToggleTag: (tag: string) => void;
    onClearAll: () => void;
  }

  let { allTags, activeTags, onToggleTag, onClearAll }: Props = $props();

  let sortedTags = $derived([...new Set(allTags)].sort());
</script>

<div class="filter-bar">
  <span class="label">Filter:</span>
  <button
    class="chip"
    class:active={activeTags.size === 0}
    onclick={onClearAll}
  >All</button>
  {#each sortedTags as tag}
    <button
      class="chip"
      class:active={activeTags.has(tag)}
      onclick={() => onToggleTag(tag)}
    >{tag}</button>
  {/each}
</div>

<style>
  .filter-bar {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border-faint);
  }
  .label {
    font-size: 0.75rem;
    color: var(--text-label);
    margin-right: 0.5rem;
  }
  .chip {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 999px;
    padding: 0.25rem 0.65rem;
    font-size: 0.75rem;
    cursor: pointer;
    transition: border-color 120ms ease, color 120ms ease, background 120ms ease;
  }
  .chip:hover  { border-color: var(--border-surface); color: var(--text-primary); }
  .chip.active { border-color: var(--text-label); color: var(--text-primary); background: var(--bg-raised); }
</style>
