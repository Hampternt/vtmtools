<script lang="ts">
  interface Props {
    tags: string[];
    onChange: (tags: string[]) => void;
    placeholder?: string;
    autofocus?: boolean;
  }

  let { tags, onChange, placeholder = '+ tag', autofocus = false }: Props = $props();
  let draft = $state('');
  let inputEl: HTMLInputElement;

  function commit() {
    const t = draft.trim();
    if (!t || tags.includes(t)) { draft = ''; return; }
    onChange([...tags, t]);
    draft = '';
  }

  function remove(t: string) { onChange(tags.filter(x => x !== t)); }

  function removeChipAt(i: number) {
    const next = tags.filter((_, idx) => idx !== i);
    onChange(next);
  }

  function onInputKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ',') {
      if (draft.trim().length > 0) {
        commit();
      }
      e.preventDefault();
    } else if (e.key === 'Backspace' && draft === '' && tags.length > 0) {
      e.preventDefault();
      removeChipAt(tags.length - 1);
    } else if (e.key === 'Escape') {
      draft = '';
      e.preventDefault();
    }
  }

  function onBlur() {
    if (draft.trim()) commit();
  }
</script>

<div role="combobox" aria-haspopup="listbox" aria-expanded="false" aria-controls="chips-list" class="chips">
  <ul role="list" id="chips-list" class="chips-list">
    {#each tags as t, i (t)}
      <li role="listitem" class="chip-item">
        <span class="chip">
          {t}
          <button
            type="button"
            tabindex="0"
            class="chip-remove"
            aria-label={`Remove ${t}`}
            onclick={() => removeChipAt(i)}
          >×</button>
        </span>
      </li>
    {/each}
  </ul>
  <input
    bind:this={inputEl}
    type="text"
    {placeholder}
    {autofocus}
    bind:value={draft}
    onkeydown={onInputKeydown}
    onblur={onBlur}
    aria-label="Add tag"
  />
</div>

<style>
  .chips {
    display: flex; flex-wrap: wrap; align-items: center; gap: 0.3rem;
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.3rem;
    box-sizing: border-box;
  }
  .chips-list {
    display: contents;
  }
  .chip-item {
    display: contents;
  }
  .chip {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.7rem;
    display: inline-flex; align-items: center; gap: 0.25rem;
  }
  .chip-remove {
    background: transparent; border: none; color: var(--text-muted);
    cursor: pointer; padding: 0; font-size: 0.7rem;
  }
  .chip-remove:hover { color: var(--accent); }
  input {
    flex: 1; min-width: 6rem;
    background: transparent; border: none;
    color: var(--text-primary); font-size: 0.7rem;
    padding: 0.2rem;
    outline: none;
  }
  input::placeholder { color: var(--text-muted); }
</style>
