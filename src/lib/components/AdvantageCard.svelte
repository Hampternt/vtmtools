<script lang="ts">
  import type { Advantage, Field } from '../../types';
  import { fade } from 'svelte/transition';

  const { entry, onedit, ondelete }: {
    entry: Advantage;
    onedit?: () => void;
    ondelete?: () => void;
  } = $props();

  function findField(name: string): Field | undefined {
    return entry.properties.find(p => p.name === name);
  }

  function numberValue(f: Field | undefined): number | null {
    if (!f) return null;
    if (f.type !== 'number') return null;
    return Array.isArray(f.value) ? f.value[0] ?? null : f.value;
  }

  const level      = $derived(numberValue(findField('level')));
  const minLevel   = $derived(numberValue(findField('min_level')));
  const maxLevel   = $derived(numberValue(findField('max_level')));
  const dotCeiling = 5;

  // Other properties — everything except the level/min/max triple, rendered as key: value.
  const otherProps = $derived(
    entry.properties.filter(p => !['level', 'min_level', 'max_level'].includes(p.name))
  );

  function displayValue(f: Field): string {
    switch (f.type) {
      case 'string':  return Array.isArray(f.value) ? f.value.join(', ') : String(f.value);
      case 'text':    return f.value;
      case 'number':  return Array.isArray(f.value) ? f.value.join(', ') : String(f.value);
      case 'bool':    return f.value ? 'yes' : 'no';
      case 'date':
      case 'url':
      case 'email':   return f.value;
      case 'reference': return `#${f.value}`;
      default:        return '';
    }
  }
</script>

<article class="card" transition:fade={{ duration: 120 }}>
  <header class="head">
    <h3 class="name">{entry.name}</h3>
    {#if entry.tags.length > 0}
      <div class="tags">
        {#each entry.tags as t}
          <span class="tag">{t}</span>
        {/each}
      </div>
    {/if}
  </header>

  {#if entry.description}
    <p class="desc">{entry.description}</p>
  {/if}

  {#if level !== null || (minLevel !== null && maxLevel !== null)}
    <div class="dots" aria-label="dot cost">
      {#if level !== null}
        {#each Array(dotCeiling) as _, i}
          <span class:filled={i < level}>●</span>
        {/each}
      {:else if minLevel !== null && maxLevel !== null}
        <span class="range">{minLevel}–{maxLevel} dots</span>
      {/if}
    </div>
  {/if}

  {#if otherProps.length > 0}
    <ul class="props">
      {#each otherProps as p}
        <li><span class="k">{p.name}:</span> {displayValue(p)}</li>
      {/each}
    </ul>
  {/if}

  <footer class="foot">
    {#if entry.isCustom}
      <button class="btn edit"   onclick={onedit}   aria-label="Edit">✎</button>
      <button class="btn delete" onclick={ondelete} aria-label="Delete">✕</button>
    {:else}
      <span class="builtin">built-in</span>
    {/if}
  </footer>
</article>

<style>
  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 6px;
    padding: 0.65rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    box-sizing: border-box;
  }
  .head { display: flex; flex-direction: column; gap: 0.3rem; }
  .name { font-size: 0.92rem; color: var(--text-primary); margin: 0; }
  .tags { display: flex; flex-wrap: wrap; gap: 0.25rem; }
  .tag {
    background: var(--bg-sunken);
    color: var(--text-muted);
    border-radius: 10px;
    padding: 0.06rem 0.45rem;
    font-size: 0.62rem;
  }
  .desc { color: var(--text-secondary); font-size: 0.74rem; margin: 0; line-height: 1.4; }
  .dots { color: var(--text-ghost); letter-spacing: 0.08em; font-size: 0.85rem; }
  .dots .filled { color: var(--accent); }
  .dots .range  { font-size: 0.72rem; font-style: italic; }
  .props { list-style: none; margin: 0; padding: 0; font-size: 0.7rem; color: var(--text-muted); }
  .props li { padding: 0.08rem 0; }
  .props .k { color: var(--text-label); }
  .foot { display: flex; justify-content: flex-end; gap: 0.3rem; }
  .btn {
    background: none;
    border: 1px solid var(--border-faint);
    color: var(--text-ghost);
    border-radius: 3px;
    padding: 0.1rem 0.45rem;
    font-size: 0.68rem;
    cursor: pointer;
    transition: color 0.15s, border-color 0.15s;
  }
  .btn:hover { color: var(--accent); border-color: var(--accent); }
  .builtin { color: var(--text-ghost); font-size: 0.62rem; font-style: italic; }
</style>
