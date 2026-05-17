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

  // Show/hide via the native Popover API so the menu renders in the browser's
  // top layer, escaping the carousel's transform parents and overflow:hidden
  // safety net. Without this, position:fixed would be relative to the
  // transformed card and clipped to the card boundary — the menu would be
  // invisible. See spec §12 gotcha #2.
  $effect(() => {
    if (!menuEl) return;
    if (open) {
      // Position first so the very first paint after showPopover lands close
      // to the cursor; the rAF below then clamps to the viewport if needed.
      position = { left: anchor.x, top: anchor.y };
      if (!menuEl.matches(':popover-open')) {
        try {
          menuEl.showPopover();
        } catch {
          // Older WebKitGTK without Popover API support — fall back to a
          // visible-state CSS class. The menu is still positioned via
          // .style.left/.style.top + position: fixed; transform-parent
          // breakage may still affect placement on such builds, but a
          // partial menu beats no menu.
          menuEl.classList.add('fallback-open');
        }
      }
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
    } else {
      if (menuEl.matches(':popover-open')) {
        try {
          menuEl.hidePopover();
        } catch {
          /* no-op — popover wasn't open */
        }
      }
      menuEl.classList.remove('fallback-open');
    }
  });

  // Outside-pointerdown + Esc listeners while open. Capture phase on
  // pointerdown to win the race with the next opener element's own
  // pointerdown handler.
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

<div
  bind:this={menuEl}
  popover="manual"
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

<style>
  .card-context-menu {
    /* Popover elements are display:none by default until showPopover().
       When shown, the browser sets display:block automatically. The .fallback-open
       class is for the rare older-WebKitGTK code path where Popover API isn't
       supported and we manually toggle visibility. */
    position: fixed;
    z-index: 2000;
    background: var(--bg-raised);
    border: 1px solid var(--border-faint);
    border-radius: 5px;
    box-shadow: 0 0.5rem 1.5rem var(--shadow-strong);
    padding: 0.25rem 0;
    min-width: 12rem;
    font-size: 0.8rem;
    /* Reset the user-agent margin that popover elements receive by default. */
    margin: 0;
  }
  .card-context-menu.fallback-open {
    display: block;
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
