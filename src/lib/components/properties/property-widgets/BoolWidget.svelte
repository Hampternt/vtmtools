<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  const bool = $derived(field.type === 'bool' ? field.value : false);
</script>

{#if readonly}
  <span class="value">{bool ? 'true' : 'false'}</span>
{:else}
  <input
    type="checkbox"
    checked={bool}
    onchange={(e) => onchange?.({ name: field.name, type: 'bool', value: (e.target as HTMLInputElement).checked })}
  />
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; }
</style>
