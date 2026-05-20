<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    open: boolean;
    title: string;
    saveLabel?: string;
    valid?: boolean;
    busy?: boolean;
    error?: string | null;
    onSave: () => void | Promise<void>;
    onCancel: () => void;
    onDelete?: () => void | Promise<void>;
    onBack?: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let {
    open, title, saveLabel = 'Save',
    valid = $bindable(true), busy = false, error = null,
    onSave, onCancel, onDelete, onBack,
    children, footer,
  }: Props = $props();

  let dialogEl: HTMLDialogElement;

  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) dialogEl.showModal();
    else if (!open && dialogEl.open) dialogEl.close();
  });

  function onBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) onCancel();
  }

  function onContextMenu(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const editable =
      target.tagName === 'INPUT' ||
      target.tagName === 'TEXTAREA' ||
      target.isContentEditable;
    if (editable) return;
    e.preventDefault();
    onBack?.();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      onSave();
    }
  }
</script>

<dialog
  bind:this={dialogEl}
  class="shell"
  oncancel={(e) => { e.preventDefault(); onCancel(); }}
  onclick={onBackdropClick}
  oncontextmenu={onContextMenu}
  onkeydown={onKeyDown}
>
  <header>
    <h3>{title}</h3>
    <button class="close" onclick={onCancel} aria-label="Cancel">×</button>
  </header>

  <div class="body">{@render children()}</div>

  {#if error}<p class="error">{error}</p>{/if}

  {#if footer}
    {@render footer()}
  {:else}
    <footer>
      {#if onDelete}<button class="danger" onclick={onDelete} disabled={busy}>Delete</button>{/if}
      <span class="spacer"></span>
      <button class="secondary" onclick={onCancel}>Cancel</button>
      <button class="primary" onclick={onSave} disabled={!valid || busy}>
        {busy ? 'Saving…' : saveLabel}
      </button>
    </footer>
  {/if}
</dialog>

<style>
  .shell {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.5rem;
    padding: 0;
    width: min(36rem, calc(100vw - 2rem));
    max-height: calc(100vh - 4rem);
    color: var(--text-primary);
    box-shadow: 0 0.75rem 2rem -0.25rem rgba(0,0,0,0.7);
    box-sizing: border-box;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .shell::backdrop { background: rgba(0,0,0,0.55); }
  @starting-style { .shell[open] { transform: scale(0.96); } }
  .shell[open] { transition: transform 140ms ease-out; }
  header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.7rem 0.85rem;
    border-bottom: 1px solid var(--border-faint);
  }
  header h3 { margin: 0; font-size: 0.9rem; color: var(--text-primary); }
  .close { background: transparent; border: none; color: var(--text-muted); font-size: 1rem; cursor: pointer; padding: 0; }
  .body { padding: 0; flex: 1; overflow-y: auto; }
  footer { display: flex; align-items: center; gap: 0.4rem; padding: 0.6rem 0.85rem; border-top: 1px solid var(--border-faint); background: rgba(0,0,0,0.15); }
  footer .spacer { flex: 1; }
  footer button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.25rem;
    padding: 0.3rem 0.7rem; font-size: 0.72rem; cursor: pointer;
    font-family: inherit;
  }
  footer button.primary { background: var(--accent); color: #fff; border-color: var(--accent-bright); }
  footer button.danger { background: transparent; color: var(--accent-amber); border-color: var(--accent-amber); }
  footer button[disabled] { opacity: 0.5; cursor: not-allowed; }
  .error { color: var(--accent-amber); font-size: 0.75rem; margin: 0 0.85rem; }
</style>
