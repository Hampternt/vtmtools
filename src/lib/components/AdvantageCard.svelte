<script lang="ts">
  import type { Advantage, Field } from '../../types';
  import { bridge } from '../../store/bridge.svelte';
  import { pushAdvantageToWorld } from '$lib/library/api';

  /**
   * Discriminated input: lets the same card render both saved library
   * rows and unsaved builder previews. Saved rows carry a DB id plus the
   * provenance / wip flags; previews omit id + provenance (the builder
   * hasn't decided either yet) and pass `wip` alongside the payload.
   */
  type SavedInput   = { kind: 'saved';   data: Advantage };
  type PreviewInput = {
    kind: 'preview';
    data: Omit<Advantage, 'id' | 'isCustom' | 'sourceAttribution'>;
    wip?: boolean;
  };

  interface Props {
    input: SavedInput | PreviewInput;
    /** Preview-mode click-to-edit. Ignored in saved mode. */
    onClick?: (target: HTMLElement) => void;
    /** Saved-mode edit callback. Card has no IPC knowledge. */
    onedit?: () => void;
    /** Saved-mode delete callback. Card has no IPC knowledge. */
    ondelete?: () => void;
  }

  let { input, onClick, onedit, ondelete }: Props = $props();

  const data = $derived(input.data);
  const isPreview = $derived(input.kind === 'preview');
  const isWip = $derived(input.kind === 'saved' ? input.data.wip : (input.wip ?? false));

  let pushing = $state(false);
  let pushError = $state('');

  async function handlePush() {
    if (input.kind !== 'saved') return;
    pushing = true;
    pushError = '';
    try {
      await pushAdvantageToWorld(input.data.id);
    } catch (e) {
      pushError = String(e);
    } finally {
      pushing = false;
    }
  }

  function findField(name: string): Field | undefined {
    return data.properties.find(p => p.name === name);
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
    data.properties.filter(p => !['level', 'min_level', 'max_level'].includes(p.name))
  );

  function capitalize(s: string): string {
    return s.length === 0 ? s : s.charAt(0).toUpperCase() + s.slice(1);
  }

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

<article
  class="advantage-card"
  class:preview={isPreview}
  class:wip={isWip}
  onclick={isPreview && onClick ? (e) => onClick(e.target as HTMLElement) : undefined}
>
  <header class="head">
    <h3 class="name" data-stage="name">{data.name}</h3>
    <div class="tags">
      <span class="tag kind-chip" data-kind={data.kind} data-stage="type">{capitalize(data.kind)}</span>
      {#if input.kind === 'saved' && input.data.sourceAttribution}
        <span class="tag source-chip" data-source={(input.data.sourceAttribution.source as string | undefined) ?? 'foundry'}>
          FVTT · {(input.data.sourceAttribution.worldTitle as string | undefined) ?? '?'}
        </span>
      {/if}
      {#each data.tags as t}
        <span class="tag" data-stage="tags">{t}</span>
      {/each}
    </div>
  </header>

  {#if data.description}
    <p class="desc" data-stage="description">{data.description}</p>
  {/if}

  {#if level !== null || (minLevel !== null && maxLevel !== null)}
    <div class="dots" aria-label="dot cost" data-stage="level">
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

  {#if input.kind === 'saved'}
    <footer class="foot">
      {#if pushError}
        <span class="push-error" title={pushError}>!</span>
      {/if}
      {#if bridge.connections.foundry}
        <button
          class="btn push"
          onclick={handlePush}
          disabled={pushing}
          aria-label="Push to world"
          title="Push to Foundry world"
        >{pushing ? '…' : '⇡'}</button>
      {/if}
      {#if input.data.isCustom}
        <button class="btn edit"   onclick={onedit}   aria-label="Edit">✎</button>
        <button class="btn delete" onclick={ondelete} aria-label="Delete">✕</button>
      {:else}
        <span class="builtin">built-in</span>
      {/if}
    </footer>
  {/if}
</article>

<style>
  .advantage-card {
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
  /* Kind chip — color-coded by V5 advantage category. Uses token
     fallbacks since kind-specific tokens are not (yet) defined in
     :root; each kind falls through to a distinct existing accent so
     chips are visually distinguishable without hardcoded hex. */
  .kind-chip {
    color: var(--text-primary);
    text-transform: lowercase;
    font-variant: small-caps;
    letter-spacing: 0.04em;
  }
  .kind-chip[data-kind="merit"]      { background: var(--accent-merit,      var(--accent)); }
  .kind-chip[data-kind="flaw"]       { background: var(--accent-flaw,       var(--accent-bright)); }
  .kind-chip[data-kind="background"] { background: var(--accent-background, var(--accent-card-dossier)); }
  .kind-chip[data-kind="boon"]       { background: var(--accent-boon,       var(--accent-amber)); }
  /* Source-attribution chip — visually distinguishes imported (FVTT)
     rows from corebook + locally-authored ones. Mirrors the kind-chip
     shape so the two chips read as a paired strip. Uses token
     fallbacks since a dedicated --accent-foundry token isn't defined
     in :root yet. */
  .source-chip {
    color: var(--text-primary);
    font-variant: small-caps;
    letter-spacing: 0.04em;
  }
  .source-chip[data-source="foundry"] { background: var(--accent-foundry, var(--accent-card-dossier)); }
  .source-chip[data-source="roll20"]  { background: var(--accent-roll20,  var(--accent-merit)); }
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
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .push-error {
    color: var(--accent);
    font-weight: bold;
    font-size: 0.72rem;
    align-self: center;
    cursor: help;
  }
  .builtin { color: var(--text-ghost); font-size: 0.62rem; font-style: italic; }

  /* Preview-mode highlight: amber border + warm glow makes builder
     previews visually distinct from saved library rows. */
  .advantage-card.preview {
    border-color: var(--accent-amber);
    box-shadow: 0 0 16px rgba(217, 165, 96, 0.22);
  }
  /* WIP draft indicator — subtle dimming. */
  .advantage-card.wip {
    opacity: 0.92;
  }
</style>
