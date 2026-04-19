<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  function currentNumber(): number {
    if (field.type !== 'number') return 0;
    return Array.isArray(field.value) ? (field.value[0] ?? 0) : field.value;
  }
</script>

{#if readonly}
  <span class="value">{currentNumber()}</span>
{:else}
  <input
    class="input"
    type="number"
    value={currentNumber()}
    oninput={(e) => {
      const v = Number((e.target as HTMLInputElement).value);
      onchange?.({ name: field.name, type: 'number', value: Number.isFinite(v) ? v : 0 });
    }}
  />
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; }
  .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.28rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    width: 6rem;
    outline: none;
    font-family: inherit;
  }
  .input:focus { border-color: var(--accent); }
</style>
