<script lang="ts">
  /**
   * Wraps a draggable element. On left-button pointerdown, snapshots the
   * element's bounding rect and asks dndStore to enter the held state with
   * the configured source. Calls stopPropagation so the parent row's
   * focus-handler does not also fire.
   *
   * The pickup target is the wrapped element itself (e.g. `.card-body`).
   * Button children of the wrapped element should be siblings, not nested
   * descendants, so they don't initiate pickup when clicked.
   */
  import { dndStore } from '../../dnd/store.svelte';
  import type { DragSource } from '../../dnd/types';
  import type { Snippet } from 'svelte';

  interface Props {
    source: DragSource;
    /** When true, the element is rendered but pointerdown is ignored. */
    disabled?: boolean;
    children: Snippet;
  }

  let { source, disabled = false, children }: Props = $props();

  let wrapEl: HTMLDivElement | undefined = $state();

  function handlePointerDown(e: PointerEvent): void {
    if (disabled) return;
    if (e.button !== 0) return;             // left button only
    if (!wrapEl) return;
    e.stopPropagation();
    e.preventDefault();
    const rect = wrapEl.getBoundingClientRect();
    dndStore.pickup(source, rect, e.clientX, e.clientY);
  }
</script>

<div bind:this={wrapEl} onpointerdown={handlePointerDown}>
  {@render children()}
</div>

<style>
  div {
    display: contents;   /* wrapper is layout-transparent */
  }
</style>
