<script lang="ts">
  // Add asset modal: manual asset entry (no catalog interaction — that's the
  // shop in Task 10). Useful for items the GM authors on the fly. No invoke()
  // — caller threads the payload back via onApply().
  //
  // CSS tokens substituted per the actual repo theme (see WealthManager.svelte
  // for the same mapping table). Buttons reuse the muted-action aesthetic from
  // AdjustModal / TransferModal (Task 8) — primary is var(--bg-active) with an
  // accent border + accent text, not a saturated accent fill.

  import type { NewAsset, AssetCategory, AssetCondition } from '../../../types';

  let {
    open = $bindable(false),
    onApply,
  }: {
    open: boolean;
    onApply: (payload: NewAsset) => void | Promise<void>;
  } = $props();

  let name = $state('');
  let category: AssetCategory | '' = $state('');
  let className = $state('');
  let currentValue = $state(0);
  let condition: AssetCondition | '' = $state('');
  let notes = $state('');

  const categories: AssetCategory[] = ['weapon', 'vehicle', 'property', 'investment', 'misc'];
  const conditions: AssetCondition[] = ['new', 'used', 'very_used', 'custom'];

  function reset() {
    name = '';
    category = '';
    className = '';
    currentValue = 0;
    condition = '';
    notes = '';
  }

  async function onConfirm() {
    const payload: NewAsset = {
      name,
      category: category === '' ? null : category,
      class: className === '' ? null : className,
      purchasePrice: null,
      currentValue,
      condition: condition === '' ? null : condition,
      customModifiers: [],
      notes: notes === '' ? null : notes,
    };
    await onApply(payload);
    open = false;
    reset();
  }

  function onCancel() {
    open = false;
    reset();
  }
</script>

{#if open}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Add asset (manual entry)</h3>

    <label class="field">
      <span>Name</span>
      <input type="text" bind:value={name} autofocus />
    </label>

    <label class="field">
      <span>Category</span>
      <select bind:value={category}>
        <option value="">— none —</option>
        {#each categories as c}<option value={c}>{c}</option>{/each}
      </select>
    </label>

    <label class="field">
      <span>Class (optional, e.g. "supercar")</span>
      <input type="text" bind:value={className} />
    </label>

    <label class="field">
      <span>Current value</span>
      <input type="number" step="any" bind:value={currentValue} />
    </label>

    <label class="field">
      <span>Condition</span>
      <select bind:value={condition}>
        <option value="">— none —</option>
        {#each conditions as c}<option value={c}>{c}</option>{/each}
      </select>
    </label>

    <label class="field">
      <span>Notes</span>
      <textarea bind:value={notes} rows="2"></textarea>
    </label>

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm} disabled={name.trim() === ''}>
        Add
      </button>
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
    gap: 0.5rem;
    padding: 1rem;
    background: var(--bg-card);
    color: var(--text-primary);
    border: 1px solid var(--border-card);
    border-radius: 0.3rem;
    min-width: 26rem;
    max-height: 90vh;
    overflow-y: auto;
  }
  h3 {
    margin: 0 0 0.5rem 0;
    color: var(--accent);
  }
  .field {
    display: grid;
    align-items: start;
    gap: 0.25rem;
    font-size: 0.82rem;
    color: var(--text-muted);
  }
  .field input,
  .field select,
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
    margin-top: 0.5rem;
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
