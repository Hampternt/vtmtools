<script lang="ts">
  /**
   * Wraps a drop target region. While dndStore.held is non-null and the
   * cursor is over this element, the store's target is set to our target.
   * On left-click pointerdown during held state, calls dndStore.drop().
   * Cursor leaving us calls setTarget(null).
   *
   * Renders a `data-drop-active` attribute when the current target equals
   * ours AND the resolved action list is non-empty, so the consumer can
   * style the "valid drop zone" outline.
   */
  import { dndStore } from '../../dnd/store.svelte';
  import type { DropTarget } from '../../dnd/types';
  import type { Snippet } from 'svelte';

  interface Props {
    target: DropTarget;
    children: Snippet;
  }

  let { target, children }: Props = $props();

  function isMine(): boolean {
    const h = dndStore.held;
    if (!h || !h.target) return false;
    if (h.target.kind !== target.kind) return false;
    return h.target.character.source === target.character.source
        && h.target.character.source_id === target.character.source_id;
  }

  let dropActive = $derived.by((): boolean => {
    const h = dndStore.held;
    if (!h) return false;
    return isMine() && h.actions.length > 0;
  });

  function handlePointerEnter(): void {
    if (!dndStore.held) return;
    dndStore.setTarget(target);
  }
  function handlePointerLeave(): void {
    if (!dndStore.held) return;
    if (isMine()) dndStore.setTarget(null);
  }
  async function handlePointerDown(e: PointerEvent): Promise<void> {
    if (!dndStore.held) return;
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    // Ensure target is set (pointerenter may not fire if cursor entered before pickup).
    dndStore.setTarget(target);
    await dndStore.drop();
  }
</script>

<div
  class="dnd-drop-zone"
  data-drop-active={dropActive ? 'true' : 'false'}
  onpointerenter={handlePointerEnter}
  onpointerleave={handlePointerLeave}
  onpointerdown={handlePointerDown}
>
  {@render children()}
</div>

<style>
  .dnd-drop-zone[data-drop-active="true"] {
    outline: 2px dashed var(--accent-situational-bright);
    outline-offset: 2px;
    border-radius: 0.625rem;
  }
</style>
