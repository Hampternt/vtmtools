<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Two-way binding controls open state. Caller does `bind:open={...}`. */
    open?: boolean;
    /** Header title — usually the card name. */
    title: string;
    /** Body content. */
    children: Snippet;
    /** Optional foot — buttons supplied by caller. If omitted, no foot rendered. */
    foot?: Snippet;
    /** Called after the dialog actually closes (post-close cleanup, e.g. clearing parent state). */
    onClose?: () => void;
  }

  let { open = $bindable(false), title, children, foot, onClose }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  /** activeElement at open — restored manually on close because WebKitGTK's
   *  built-in <dialog>.close() focus restoration has shipped inconsistent
   *  behavior. See spec §9.1 / §12. */
  let openerFocus: HTMLElement | null = null;

  function close() {
    if (dialogEl?.open) dialogEl.close();
  }

  function handleClose() {
    open = false;
    onClose?.();
    if (openerFocus && openerFocus.isConnected) {
      openerFocus.focus();
    }
    openerFocus = null;
  }

  // Open/close the dialog when `open` prop changes.
  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) {
      openerFocus = (document.activeElement as HTMLElement | null) ?? null;
      dialogEl.showModal();
    } else if (!open && dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Backdrop click — event.target equals the dialog element only when the
   *  click landed on the backdrop, not on dialog content. Standard pattern. */
  function handleBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) close();
  }
</script>

<dialog
  bind:this={dialogEl}
  class="card-overlay"
  onclose={handleClose}
  onclick={handleBackdropClick}
>
  <header class="overlay-head">
    <h3>{title}</h3>
    <button
      type="button"
      class="overlay-close"
      aria-label="Close"
      onclick={close}
    >×</button>
  </header>
  <div class="overlay-body">
    {@render children()}
  </div>
  {#if foot}
    <footer class="overlay-foot">
      {@render foot()}
    </footer>
  {/if}
</dialog>

<style>
  .card-overlay {
    border: 1px solid var(--border-surface);
    background: var(--bg-card);
    color: var(--text-primary);
    border-radius: 0.75rem;
    padding: 0;
    max-width: 30rem;
    width: 90vw;
    box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.7);
  }
  .card-overlay::backdrop {
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
  }

  .overlay-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.1rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .overlay-head h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 500;
  }

  .overlay-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
  }
  .overlay-close:hover {
    color: var(--text-primary);
    background: var(--bg-raised);
  }

  .overlay-body {
    padding: 1rem 1.1rem 1.1rem;
  }

  .overlay-foot {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.75rem 1.1rem;
    border-top: 1px solid var(--border-faint);
  }
</style>
