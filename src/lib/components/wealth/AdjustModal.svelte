<script lang="ts">
  // Adjust modal: +/-/set on a single wallet bucket (liquid or banked).
  // No invoke() here — caller threads the delta back via onApply().
  // CSS tokens substituted per the actual repo theme (see WealthManager.svelte
  // for the same mapping table).

  import type { Bucket, Wallet } from '../../../types';

  let {
    open = $bindable(false),
    bucket,
    wallet,
    onApply,
  }: {
    open: boolean;
    bucket: Bucket;
    wallet: Wallet;
    onApply: (delta: number) => void | Promise<void>;
  } = $props();

  let mode: 'add' | 'subtract' | 'set' = $state('add');
  let amount = $state(0);

  function currentValue(): number {
    return bucket === 'liquid' ? wallet.liquid : wallet.banked;
  }

  function deltaFromForm(): number {
    if (mode === 'add')      return amount;
    if (mode === 'subtract') return -amount;
    // 'set' — delta needed to reach the target
    return amount - currentValue();
  }

  async function onConfirm() {
    await onApply(deltaFromForm());
    open = false;
    amount = 0;
    mode = 'add';
  }

  function onCancel() {
    open = false;
    amount = 0;
    mode = 'add';
  }
</script>

{#if open}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Adjust {bucket}</h3>
    <p class="current">Current: {wallet.currencyName}{currentValue().toLocaleString()}</p>

    <fieldset class="modes">
      <legend>Mode</legend>
      <label><input type="radio" bind:group={mode} value="add" /> Add</label>
      <label><input type="radio" bind:group={mode} value="subtract" /> Subtract</label>
      <label><input type="radio" bind:group={mode} value="set" /> Set to</label>
    </fieldset>

    <label class="amount-field">
      <span>Amount</span>
      <input type="number" step="any" bind:value={amount} autofocus />
    </label>

    <p class="preview">
      New value: {wallet.currencyName}{(currentValue() + deltaFromForm()).toLocaleString()}
    </p>

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm}>Apply</button>
    </div>
  </dialog>
{/if}

<style>
  /* Token substitutions vs the plan's draft:
       --color-overlay     -> rgba(0, 0, 0, 0.6) literal
       --color-surface     -> --bg-card
       --color-text        -> --text-primary
       --color-text-muted  -> --text-muted
       --color-border      -> --border-card
       --color-accent      -> --accent
       --color-on-accent   -> --text-primary
       --space-*           -> rem literals (1=0.25, 2=0.5, 3=0.75, 4=1, 5=1.5)
       --radius-md         -> 0.3rem
     Per CLAUDE.md "rem not px" + colors via tokens only. */

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
    min-width: 22rem;
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
  .modes {
    display: flex;
    gap: 0.75rem;
    border: 0;
    padding: 0;
    color: var(--text-primary);
  }
  .modes legend {
    display: none;
  }
  .modes label {
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
  .preview {
    color: var(--text-muted);
    margin: 0;
    font-size: 0.82rem;
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
  .primary:hover {
    color: var(--accent-bright) !important;
  }
</style>
