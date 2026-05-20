<script lang="ts">
  import type { StatusTemplate } from '../../../types';

  /**
   * Discriminated input: lets the same card render both saved library
   * rows and unsaved builder previews. Saved rows carry a DB id plus
   * the wip flag; previews omit id (the builder hasn't issued one yet)
   * and pass `wip` alongside the payload.
   */
  type SavedInput   = { kind: 'saved';   data: StatusTemplate };
  type PreviewInput = {
    kind: 'preview';
    data: Omit<StatusTemplate, 'id'>;
    wip?: boolean;
  };

  interface Props {
    input: SavedInput | PreviewInput;
    /** Saved-mode "summary" line under the name (e.g. effect digest). */
    summary?: string;
    /** Saved-mode primary action — apply the template. */
    onapply?: () => void;
    /** Whether the apply button is disabled (e.g. no focused character). */
    applyDisabled?: boolean;
    /** Tooltip for the apply button. */
    applyTitle?: string;
    /** Saved-mode edit callback. Card has no IPC knowledge. */
    onedit?: () => void;
    /** Preview-mode click-to-edit. Ignored in saved mode. */
    onClick?: (target: HTMLElement) => void;
  }

  let {
    input,
    summary = '',
    onapply,
    applyDisabled = false,
    applyTitle = '',
    onedit,
    onClick,
  }: Props = $props();

  const data = $derived(input.data);
  const isPreview = $derived(input.kind === 'preview');
  const isWip = $derived(input.kind === 'saved' ? input.data.wip : (input.wip ?? false));
</script>

<article
  class="status-template-card"
  class:preview={isPreview}
  class:wip={isWip}
  onclick={isPreview && onClick ? (e) => onClick(e.target as HTMLElement) : undefined}
>
  {#if input.kind === 'saved'}
    <button
      class="apply"
      onclick={onapply}
      disabled={applyDisabled}
      title={applyTitle}
    >
      <span class="name">{data.name}</span>
      <span class="summary">{summary}</span>
      {#if data.tags.length > 0}
        <span class="tags">
          {#each data.tags as tag}<span class="tag">#{tag}</span>{/each}
        </span>
      {/if}
    </button>
    <button class="edit" title="Edit template" onclick={onedit}>✎</button>
  {:else}
    <div class="apply preview-body">
      <span class="name">{data.name}</span>
      {#if summary}<span class="summary">{summary}</span>{/if}
      {#if data.tags.length > 0}
        <span class="tags">
          {#each data.tags as tag}<span class="tag">#{tag}</span>{/each}
        </span>
      {/if}
    </div>
  {/if}
</article>

<style>
  .status-template-card {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.3rem;
    align-items: stretch;
  }
  .apply {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 0.4rem;
    padding: 0.45rem 0.6rem;
    text-align: left;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    transition: border-color 120ms ease, background 120ms ease;
  }
  .apply:hover:not(:disabled) { border-color: var(--accent-bright); background: var(--bg-active); }
  .apply:disabled { opacity: 0.5; cursor: not-allowed; }
  .apply .name { font-size: 0.8rem; font-weight: 500; }
  .apply .summary { font-size: 0.7rem; color: var(--text-secondary); }
  .apply .tags { display: flex; flex-wrap: wrap; gap: 0.2rem; margin-top: 0.15rem; }
  .apply .tag { font-size: 0.6rem; color: var(--text-muted); }
  .preview-body { cursor: default; }
  .edit {
    background: transparent;
    border: 1px solid var(--border-faint);
    color: var(--text-muted);
    border-radius: 0.4rem;
    padding: 0 0.4rem;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .edit:hover { color: var(--text-primary); border-color: var(--border-surface); }

  /* Preview-mode highlight: amber border + warm glow makes builder
     previews visually distinct from saved library rows. */
  .status-template-card.preview {
    border-color: var(--accent-amber);
    box-shadow: 0 0 16px rgba(217, 165, 96, 0.22);
  }
  /* WIP draft indicator — subtle dimming. */
  .status-template-card.wip {
    opacity: 0.92;
  }
</style>
