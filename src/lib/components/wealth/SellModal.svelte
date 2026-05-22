<script lang="ts">
  // Sell modal: prompt for a sale price (pre-filled with currentValue via
  // $effect so the GM only has to type when haggling). No invoke() — caller
  // threads a SellRequest back via onApply().
  //
  // CSS tokens substituted per the actual repo theme (see WealthManager.svelte
  // for the same mapping table). Button styling matches AdjustModal /
  // TransferModal — muted action surface, not a saturated accent fill.

  import type { Asset, SellRequest } from '../../../types';

  let {
    open = $bindable(false),
    asset,
    currencyName,
    onApply,
  }: {
    open: boolean;
    asset: Asset | null;
    currencyName: string;
    onApply: (req: SellRequest) => void | Promise<void>;
  } = $props();

  let salePrice = $state(0);
  let notes = $state('');

  // Pre-fill the sale price from the asset's current value whenever the
  // modal's asset target changes. The GM can still overwrite for haggling.
  $effect(() => {
    if (asset) salePrice = asset.currentValue;
  });

  async function onConfirm() {
    if (!asset) return;
    await onApply({
      assetId: asset.id,
      salePrice,
      notes: notes === '' ? null : notes,
    });
    open = false;
    notes = '';
  }

  function onCancel() {
    open = false;
    notes = '';
  }
</script>

{#if open && asset}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Sell "{asset.name}"</h3>
    <p class="current">
      Current value: {currencyName}{asset.currentValue.toLocaleString()}
    </p>

    <label class="field">
      <span>Sale price ({currencyName})</span>
      <input type="number" step="any" bind:value={salePrice} autofocus />
    </label>

    <label class="field">
      <span>Notes (optional)</span>
      <textarea bind:value={notes} rows="2"></textarea>
    </label>

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm}>Sell</button>
    </div>
  </dialog>
{/if}

<style>
  /* Token substitutions: see WealthManager.svelte / AdjustModal.svelte. */

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
  .field {
    display: grid;
    align-items: start;
    gap: 0.25rem;
    font-size: 0.82rem;
    color: var(--text-muted);
  }
  .field input,
  .field textarea {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.4rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.82rem;
    font-family: inherit;
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
