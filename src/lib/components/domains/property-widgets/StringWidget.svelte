<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  function currentString(): string {
    if (field.type !== 'string') return '';
    return Array.isArray(field.value) ? (field.value[0] ?? '') : field.value;
  }

  function emit(next: string) {
    onchange?.({ name: field.name, type: 'string', value: next });
  }
</script>

{#if readonly}
  <span class="value">{currentString()}</span>
{:else}
  <input
    class="input"
    type="text"
    value={currentString()}
    oninput={(e) => emit((e.target as HTMLInputElement).value)}
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
    width: 100%;
    outline: none;
    font-family: inherit;
  }
  .input:focus { border-color: var(--accent); }
</style>
