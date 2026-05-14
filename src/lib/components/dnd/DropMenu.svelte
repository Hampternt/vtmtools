<script lang="ts">
  /**
   * Contextual action menu rendered at the cursor when a drop resolves to
   * ≥2 actions. v1 never opens this — single-action drops execute immediately.
   * v2/v3 will surface multi-action drops here.
   *
   * Closes via:
   *   - click on an action → executeAction()
   *   - click outside the menu element → cancel()
   *   - right-click / Esc / blur → handled by global listeners in
   *     GmScreen.svelte that call dndStore.cancel().
   */
  import { dndStore } from '../../dnd/store.svelte';
  import type { Action } from '../../dnd/types';

  let menuEl: HTMLDivElement | undefined = $state();

  async function pick(action: Action) {
    await dndStore.executeAction(action);
  }

  function handleOutsidePointerDown(e: PointerEvent) {
    if (!menuEl) return;
    if (!menuEl.contains(e.target as Node)) {
      dndStore.cancel();
    }
  }

  $effect(() => {
    if (dndStore.held?.menuOpenAt) {
      document.addEventListener('pointerdown', handleOutsidePointerDown, true);
      return () => document.removeEventListener('pointerdown', handleOutsidePointerDown, true);
    }
  });
</script>

{#if dndStore.held?.menuOpenAt}
  {@const pos = dndStore.held.menuOpenAt}
  {@const actions = dndStore.held.actions}
  <div
    bind:this={menuEl}
    class="drop-menu"
    style="left: {pos.x}px; top: {pos.y}px;"
    role="menu"
  >
    {#each actions as action}
      <button class="action" role="menuitem" onclick={() => pick(action)}>
        {action.label}
      </button>
    {/each}
  </div>
{/if}

<style>
  .drop-menu {
    position: fixed;
    z-index: 2000;
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.4rem;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
    min-width: 13rem;
    padding: 0.25rem 0;
  }
  .action {
    display: block;
    width: 100%;
    box-sizing: border-box;
    text-align: left;
    background: transparent;
    color: var(--text-primary);
    border: none;
    padding: 0.4rem 0.75rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .action:hover {
    background: var(--bg-active);
  }
</style>
