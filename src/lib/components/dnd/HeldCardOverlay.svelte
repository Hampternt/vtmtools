<script lang="ts">
  /**
   * Top-level overlay rendered by GmScreen.svelte. While dndStore.held is
   * non-null, this fixed-position element follows the cursor with a small
   * miniature of the dragged card. Listens to global pointermove on the
   * document to update store cursor coords; the listener installs only
   * while held is non-null.
   */
  import { dndStore } from '../../dnd/store.svelte';

  function handleMove(e: PointerEvent) {
    if (!dndStore.held) return;
    dndStore.moveCursor(e.clientX, e.clientY);
  }

  $effect(() => {
    if (dndStore.held) {
      document.addEventListener('pointermove', handleMove);
      return () => document.removeEventListener('pointermove', handleMove);
    }
  });

  function labelOf(): string {
    const h = dndStore.held;
    if (!h) return '';
    if (h.source.kind === 'free-mod' || h.source.kind === 'advantage') return h.source.mod.name;
    if (h.source.kind === 'template') return h.source.template.name;
    return '';
  }
</script>

{#if dndStore.held && !dndStore.held.menuOpenAt}
  {@const h = dndStore.held}
  <div
    class="held-overlay"
    style="left: {h.cursorX + 8}px; top: {h.cursorY + 8}px;"
    aria-hidden="true"
  >
    {labelOf()}
  </div>
{/if}

<style>
  .held-overlay {
    position: fixed;
    z-index: 1500;
    pointer-events: none;
    background: var(--bg-card);
    border: 1px solid var(--accent-bright);
    border-radius: 0.4rem;
    padding: 0.3rem 0.6rem;
    font-size: 0.75rem;
    color: var(--text-primary);
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.5);
    max-width: 14rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
