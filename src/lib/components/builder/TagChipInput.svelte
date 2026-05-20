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

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      commit();
    } else if (e.key === 'Backspace' && draft === '' && tags.length > 0) {
      e.preventDefault();
      remove(tags[tags.length - 1]);
    }
  }

  function onBlur() {
    if (draft.trim()) commit();
  }
</script>

<div class="chips">
  {#each tags as t}
    <span class="chip">
      {t}
      <button type="button" onclick={() => remove(t)} aria-label="Remove tag {t}">×</button>
    </span>
  {/each}
  <input
    bind:this={inputEl}
    type="text"
    {placeholder}
    {autofocus}
    bind:value={draft}
    onkeydown={onKey}
    onblur={onBlur}
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
  .chip {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.7rem;
    display: inline-flex; align-items: center; gap: 0.25rem;
  }
  .chip button {
    background: transparent; border: none; color: var(--text-muted);
    cursor: pointer; padding: 0; font-size: 0.7rem;
  }
  .chip button:hover { color: var(--accent); }
  input {
    flex: 1; min-width: 6rem;
    background: transparent; border: none;
    color: var(--text-primary); font-size: 0.7rem;
    padding: 0.2rem;
    outline: none;
  }
  input::placeholder { color: var(--text-muted); }
</style>
