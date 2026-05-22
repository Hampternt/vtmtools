<script lang="ts">
  // Transfer modal: liquid ↔ banked with optional delay hint surfaced from
  // wallet.transferDelay (stored but not enforced in V1).
  // No invoke() — caller threads via onApply(from, to, amount).

  import type { Bucket, Wallet } from '../../../types';

  let {
    open = $bindable(false),
    wallet,
    onApply,
  }: {
    open: boolean;
    wallet: Wallet;
    onApply: (from: Bucket, to: Bucket, amount: number) => void | Promise<void>;
  } = $props();

  let from: Bucket = $state('liquid');
  let amount = $state(0);

  // Derived target = the other bucket.
  let to = $derived<Bucket>(from === 'liquid' ? 'banked' : 'liquid');

  async function onConfirm() {
    await onApply(from, to, amount);
    open = false;
    amount = 0;
    from = 'liquid';
  }

  function onCancel() {
    open = false;
    amount = 0;
    from = 'liquid';
  }
</script>

{#if open}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Transfer funds</h3>
    <p class="current">
      Liquid: {wallet.currencyName}{wallet.liquid.toLocaleString()}<br/>
      Banked: {wallet.currencyName}{wallet.banked.toLocaleString()}
    </p>

    <fieldset class="direction">
      <legend>From</legend>
      <label><input type="radio" bind:group={from} value="liquid" /> Liquid → Banked</label>
      <label><input type="radio" bind:group={from} value="banked" /> Banked → Liquid</label>
    </fieldset>

    <label class="amount-field">
      <span>Amount</span>
      <input type="number" step="any" min="0" bind:value={amount} autofocus />
    </label>

    {#if wallet.transferDelay}
      <p class="delay-hint">
        Transfer delay configured: {wallet.transferDelay.value} {wallet.transferDelay.unit}.
        Tool executes instantly in V1.
      </p>
    {/if}

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm} disabled={amount <= 0}>
        Transfer
      </button>
    </div>
  </dialog>
{/if}

<style>
  /* Token substitutions: see AdjustModal.svelte / WealthManager.svelte. */

  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.6);
    z-index: 100;
  }
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 101;
    display: grid;
    align-items: start;
    gap: 0.75rem;
    padding: 1rem;
    background: var(--bg-card);
    color: var(--text-primary);
    border: 1px solid var(--border-card);
    border-radius: 0.3rem;
    min-width: 24rem;
  }
  h3 {
    margin: 0;
    color: var(--accent);
  }
  .current {
    color: var(--text-muted);
    margin: 0;
    font-size: 0.82rem;
  }
  .direction {
    display: grid;
    align-items: start;
    gap: 0.5rem;
    border: 0;
    padding: 0;
    color: var(--text-primary);
  }
  .direction legend {
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  .direction label {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.82rem;
  }
  .amount-field {
    display: grid;
    align-items: start;
    gap: 0.25rem;
    font-size: 0.82rem;
    color: var(--text-muted);
  }
  .amount-field input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.4rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.82rem;
  }
  .delay-hint {
    color: var(--text-muted);
    font-size: 0.82rem;
    margin: 0;
    padding: 0.5rem;
    border-left: 2px solid var(--accent);
    background: var(--bg-active);
    border-radius: 0.2rem;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .actions button {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    color: var(--text-primary);
    border-radius: 0.3rem;
    padding: 0.4rem 0.75rem;
    font-size: 0.82rem;
    cursor: pointer;
  }
  .actions button:hover {
    background: var(--bg-active);
  }
  .primary {
    background: var(--bg-active) !important;
    border-color: var(--accent) !important;
    color: var(--accent) !important;
  }
  .primary:hover:not(:disabled) {
    color: var(--accent-bright) !important;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
