<script lang="ts" module>
  /**
   * Discriminated union — either a clickable item or a visual divider.
   * Using `kind` instead of optional fields prevents "divider with onActivate"
   * type errors at call sites.
   */
  export type CardAction =
    | {
        kind: 'item';
        label: string;
        /** Optional shortcut hint shown right-aligned (e.g. "H", "Del", "Enter"). */
        shortcut?: string;
        /** Marks destructive actions for amber styling. */
        destructive?: boolean;
        onActivate: () => void;
      }
    | { kind: 'divider' };
</script>

<script lang="ts">
  interface Props {
    /** Whether the menu is visible. */
    open: boolean;
    /** Viewport coords for the menu's top-left. Adjusted in $effect if off-screen. */
    anchor: { x: number; y: number };
    actions: CardAction[];
    /** Called on outside-pointerdown, Escape, or item activation. */
    onClose: () => void;
  }

  let { open, anchor, actions, onClose }: Props = $props();

  let menuEl: HTMLDivElement | undefined = $state();
  let position = $state<{ left: number; top: number }>({ left: 0, top: 0 });

  function handleActivate(action: CardAction) {
    if (action.kind !== 'item') return;
    action.onActivate();
    onClose();
  }

  function handleOutsidePointerDown(e: PointerEvent) {
    if (!menuEl) return;
    if (!menuEl.contains(e.target as Node)) onClose();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  // Position the menu when it opens; adjust if it'd fall off the right/bottom.
  $effect(() => {
    if (!open) return;
    // Start at the requested coords, then measure and clamp after layout.
    position = { left: anchor.x, top: anchor.y };
    requestAnimationFrame(() => {
      if (!menuEl) return;
      const r = menuEl.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let { left, top } = position;
      if (r.right > vw) left = Math.max(4, vw - r.width - 4);
      if (r.bottom > vh) top = Math.max(4, vh - r.height - 4);
      position = { left, top };
    });
  });

  // Outside-pointerdown + Esc listeners while open. Capture phase on pointerdown
  // to win the race with the next opener element's own pointerdown handler.
  $effect(() => {
    if (!open) return;
    document.addEventListener('pointerdown', handleOutsidePointerDown, true);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('pointerdown', handleOutsidePointerDown, true);
      document.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

{#if open}
  <div
    bind:this={menuEl}
    class="card-context-menu"
    style="left: {position.left}px; top: {position.top}px;"
    role="menu"
  >
    {#each actions as action}
      {#if action.kind === 'divider'}
        <div class="divider" aria-hidden="true"></div>
      {:else}
        <button
          type="button"
          class="item"
          class:destructive={action.destructive}
          role="menuitem"
          onclick={() => handleActivate(action)}
        >
          <span class="label">{action.label}</span>
          {#if action.shortcut}
            <span class="shortcut">{action.shortcut}</span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .card-context-menu {
    position: fixed;
    z-index: 2000;
    background: var(--bg-raised);
    border: 1px solid var(--border-faint);
    border-radius: 5px;
    box-shadow: 0 0.5rem 1.5rem var(--shadow-strong);
    padding: 0.25rem 0;
    min-width: 12rem;
    font-size: 0.8rem;
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    box-sizing: border-box;
    background: transparent;
    color: var(--text-primary);
    border: none;
    text-align: left;
    padding: 0.4rem 0.85rem;
    font-size: inherit;
    font-family: inherit;
    cursor: pointer;
    gap: 1rem;
  }
  .item:hover { background: var(--accent); color: var(--text-primary); }
  .item.destructive { color: var(--accent-amber); }
  .item.destructive:hover { background: rgba(204, 153, 34, 0.15); color: var(--accent-amber); }
  .shortcut {
    color: var(--text-muted);
    font-size: 0.7rem;
    font-family: monospace;
  }
  .item.destructive:hover .shortcut { color: var(--accent-amber); }
  .divider {
    height: 1px;
    background: var(--border-faint);
    margin: 0.25rem 0;
  }
</style>
