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
    {#if saved?.deletedInVttAt}
      <span class="vtt-deleted-badge"
        title="Deleted in {character.source === 'foundry' ? 'Foundry' : 'Roll20'} on {saved.deletedInVttAt}">deleted</span>
    {/if}
    <span class="rail-spacer"></span>
    <div class="actions">
      {#if saved}
        <button type="button" class="btn-save"
          onclick={() => onCompare(saved, character)}>Compare</button>
        <button type="button" class="btn-save"
          onclick={() => savedCharacters.update(saved.id, character)}
          disabled={savedCharacters.loading}>Update saved</button>
        <button type="button" class="btn-save btn-forget"
          onclick={() => savedCharacters.delete(saved.id)}
          disabled={savedCharacters.loading}>Forget saved</button>
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
  .vtt-deleted-badge {
    background: color-mix(in srgb, var(--text-muted) 40%, transparent);
    color: var(--text-primary);
    font-size: calc(0.6rem * var(--card-scale, 1));
    padding: 0 0.4em;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-weight: 600;
  }
  .btn-forget {
    color: var(--text-muted);
  }
  .btn-forget:hover:not(:disabled) {
    color: var(--accent-amber);
    border-color: var(--accent-amber);
  }
</style>
