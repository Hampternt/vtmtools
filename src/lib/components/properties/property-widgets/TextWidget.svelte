<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  const text = $derived(field.type === 'text' ? field.value : '');
</script>

{#if readonly}
  <span class="value">{text}</span>
{:else}
  <textarea
    class="input"
    rows={3}
    value={text}
    oninput={(e) => onchange?.({ name: field.name, type: 'text', value: (e.target as HTMLTextAreaElement).value })}
  ></textarea>
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; white-space: pre-wrap; }
  .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.28rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    width: 100%;
    resize: vertical;
    outline: none;
    font-family: inherit;
  }
  .input:focus { border-color: var(--accent); }
</style>
