<script lang="ts">
  import type { ModifierZone } from '../../../types';

  interface Props {
    /** Drives the active-dot color/glow. */
    isActive: boolean;
    /** Zone — selects which accent token the dot uses when active. */
    zone: ModifierZone;
  }
  let { isActive, zone }: Props = $props();
</script>

<div class="drag-handle" data-zone={zone} aria-hidden="true">
  <span class="grip">⠿</span>
  <span class="active-dot" class:on={isActive}></span>
</div>

<style>
  .drag-handle {
    height: 1.1rem;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.55rem;
    cursor: grab;
    user-select: none;
    background: linear-gradient(180deg, rgba(255,255,255,0.025), transparent);
    border-bottom: 1px solid rgba(255,255,255,0.04);
  }
  .drag-handle:active { cursor: grabbing; }

  .grip {
    color: var(--text-muted);
    opacity: 0.4;
    font-size: 0.7rem;
    letter-spacing: -1px;
    line-height: 1;
    transition: opacity 120ms ease;
  }
  /* Reveal fully on card hover OR handle hover. The :global() escapes
     Svelte's per-component scoping for the ancestor selector. */
  :global(.modifier-card:hover) .grip,
  .drag-handle:hover .grip { opacity: 1; }

  .active-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: transparent;
    transition: background 200ms ease, box-shadow 200ms ease;
  }
  .active-dot.on {
    background: var(--accent-bright);
    box-shadow: 0 0 6px var(--accent-bright);
  }
  .drag-handle[data-zone="situational"] .active-dot.on {
    background: var(--accent-situational-bright);
    box-shadow: 0 0 6px var(--accent-situational-bright);
  }
</style>
