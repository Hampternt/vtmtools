<!--
  Modal dialog showing the diff between a saved character snapshot and
  the live bridge view. Closes on Escape or backdrop click. Pure-presentation;
  the diff itself is computed by the caller via diffCharacter().
-->
<script lang="ts">
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import type { BridgeCharacter } from '$lib/bridge/api';
  import { diffCharacter, type DiffEntry } from '$lib/saved-characters/diff';

  let {
    saved,
    live,
    onClose,
  }: {
    saved: SavedCharacter;
    live: BridgeCharacter;
    onClose: () => void;
  } = $props();

  const entries: DiffEntry[] = $derived(diffCharacter(saved.canonical, live));

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) onClose();
  }
  function handleKey(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }
</script>

<svelte:window onkeydown={handleKey} />

<div
  class="backdrop"
  onclick={handleBackdropClick}
  role="presentation"
>
  <div class="modal" role="dialog" aria-modal="true" aria-labelledby="cmp-title">
    <header>
      <h3 id="cmp-title">{saved.name} — saved vs. live</h3>
      <button type="button" class="close" onclick={onClose} aria-label="Close">×</button>
    </header>

    <div class="meta">
      Saved {saved.savedAt}{#if saved.lastUpdatedAt && saved.lastUpdatedAt !== saved.savedAt}, last updated {saved.lastUpdatedAt}{/if}
    </div>

    {#if entries.length === 0}
      <p class="empty">No differences detected — saved snapshot matches the live character.</p>
    {:else}
      <p class="summary">{entries.length} difference{entries.length === 1 ? '' : 's'}</p>
      <table>
        <thead>
          <tr>
            <th>Field</th>
            <th>Saved</th>
            <th></th>
            <th>Live</th>
          </tr>
        </thead>
        <tbody>
          {#each entries as entry (entry.key)}
            <tr>
              <td class="label">{entry.label}</td>
              <td class="before"><code>{entry.before}</code></td>
              <td class="arrow">→</td>
              <td class="after"><code>{entry.after}</code></td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <footer>
      <button type="button" onclick={onClose}>Close</button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
  }
  .modal {
    background: var(--bg-raised);
    color: var(--text-primary);
    border: 1px solid var(--border-card);
    border-radius: 0.5rem;
    box-sizing: border-box;
    padding: 1rem 1.25rem;
    max-width: 36rem;
    width: 90vw;
    max-height: 80vh;
    overflow: auto;
  }
  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-faint);
    padding-bottom: 0.5rem;
    margin-bottom: 0.5rem;
  }
  h3 { margin: 0; font-size: 1rem; }
  .close {
    background: transparent;
    border: 0;
    color: var(--text-muted);
    font-size: 1.25rem;
    cursor: pointer;
  }
  .meta { font-size: 0.75rem; color: var(--text-muted); margin-bottom: 0.75rem; }
  .summary { font-size: 0.85rem; color: var(--text-label); margin: 0 0 0.5rem; }
  .empty { color: var(--text-muted); font-style: italic; }
  table { width: 100%; border-collapse: collapse; font-size: 0.85rem; }
  th, td { padding: 0.4rem 0.5rem; text-align: left; }
  th { color: var(--text-label); border-bottom: 1px solid var(--border-faint); }
  td.label { color: var(--text-secondary); }
  td.arrow { color: var(--text-ghost); width: 1rem; }
  td.before code { color: var(--text-muted); }
  td.after code { color: var(--text-primary); }
  footer {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.75rem;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
  }
</style>
