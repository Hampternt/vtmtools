<script lang="ts">
  import type { Field } from '../../../types';
  import { WIDGETS } from './property-widgets';

  const { field, readonly, onchange, onremove }: {
    field: Field;
    readonly: boolean;
    onchange?: (updated: Field) => void;
    onremove?: () => void;
  } = $props();

  const Widget = $derived(WIDGETS[field.type] ?? null);
</script>

<div class="row">
  <span class="name">{field.name}</span>
  <div class="widget">
    {#if Widget}
      <Widget {field} {readonly} {onchange} />
    {:else}
      <span class="unsupported">Unsupported type: {field.type}</span>
    {/if}
  </div>
  {#if !readonly && onremove}
    <button class="remove" onclick={onremove} title="Remove field">✕</button>
  {/if}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 8rem 1fr auto;
    align-items: center;
    gap: 0.5rem;
    padding: 0.2rem 0;
  }
  .name { color: var(--text-muted); font-size: 0.68rem; }
  .widget { min-width: 0; }
  .unsupported { color: var(--text-ghost); font-size: 0.7rem; font-style: italic; }
  .remove {
    background: none;
    border: 1px solid var(--border-faint);
    color: var(--text-ghost);
    border-radius: 3px;
    padding: 0.05rem 0.25rem;
    font-size: 0.6rem;
    cursor: pointer;
  }
  .remove:hover { color: var(--accent); border-color: var(--accent); }
</style>
