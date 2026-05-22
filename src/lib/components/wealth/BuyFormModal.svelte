<script lang="ts">
  // Buy form modal: slot purchase form with band slider, condition radio,
  // stackable custom modifiers, and a live final-price preview.
  //
  // The price preview recomputes via $derived from `computeFinalPrice`. The
  // [Buy] action is disabled when liquid funds are insufficient. Out-of-band
  // prices are warned (prose, no glyph) but not blocked, per the spec.
  //
  // CSS tokens substituted per the actual repo theme (see WealthManager.svelte
  // for the same mapping table). Buttons follow the muted-action convention
  // from Tasks 8 / 9 (background var(--bg-active), accent border, accent text)
  // rather than a saturated accent fill.
  //
  // No invoke() — caller threads a PurchaseRequest back via onApply().

  import type {
    ItemSlot, Wallet, PurchaseRequest, AssetCondition, CustomModifier,
  } from '../../../types';
  import {
    computeFinalPrice, defaultPickInBand,
  } from '$lib/wealth/pricing';

  let {
    open = $bindable(false),
    slot,
    wallet,
    onApply,
  }: {
    open: boolean;
    slot: ItemSlot | null;
    wallet: Wallet;
    onApply: (req: PurchaseRequest) => void | Promise<void>;
  } = $props();

  let pickInBand = $state(0);
  let condition: AssetCondition = $state<AssetCondition>('new');
  let customMult = $state(1.0);
  let customMods: CustomModifier[] = $state([]);
  let newModLabel = $state('');
  let newModMult = $state(1.0);
  let assetName = $state('');
  let notes = $state('');

  // Reset form whenever the slot changes (modal re-opens for a different slot).
  $effect(() => {
    if (slot) {
      pickInBand = defaultPickInBand(slot.basePriceMin, slot.basePriceMax);
      condition = 'new';
      customMult = 1.0;
      customMods = [];
      assetName = slot.label;
      notes = '';
    }
  });

  let finalPrice = $derived(
    slot
      ? computeFinalPrice(
          pickInBand,
          condition,
          condition === 'custom' ? customMult : null,
          customMods,
        )
      : 0
  );

  let canAfford = $derived(wallet.liquid >= finalPrice);
  let outOfBand = $derived(
    slot != null && (pickInBand < slot.basePriceMin || pickInBand > slot.basePriceMax)
  );

  function addModifier() {
    if (newModLabel.trim() === '') return;
    customMods = [...customMods, { label: newModLabel, mult: newModMult }];
    newModLabel = '';
    newModMult = 1.0;
  }
  function removeModifier(i: number) {
    customMods = customMods.filter((_, idx) => idx !== i);
  }

  async function onConfirm() {
    if (!slot) return;
    await onApply({
      characterId: wallet.characterId,
      slotId: slot.id,
      price: finalPrice,
      condition,
      customModifiers: customMods,
      nameOverride: assetName.trim() === '' ? null : assetName,
      notes: notes === '' ? null : notes,
    });
    open = false;
  }

  function onCancel() {
    open = false;
  }

  function fmt(n: number): string {
    return wallet.currencyName + n.toLocaleString(undefined, { maximumFractionDigits: 2 });
  }
</script>

{#if open && slot}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Buy: {slot.label}</h3>
    {#if slot.examples}
      <p class="examples"><em>e.g.</em> {slot.examples}</p>
    {/if}

    <fieldset class="band">
      <legend>Price (band: {fmt(slot.basePriceMin)} – {fmt(slot.basePriceMax)})</legend>
      <input
        type="range"
        min={slot.basePriceMin}
        max={slot.basePriceMax}
        step={Math.max((slot.basePriceMax - slot.basePriceMin) / 100, 1)}
        bind:value={pickInBand}
      />
      <input type="number" step="any" bind:value={pickInBand} />
      {#if outOfBand}
        <span class="warn">Warning: out of band — allowed, but verify intent.</span>
      {/if}
    </fieldset>

    <fieldset class="cond">
      <legend>Condition</legend>
      <label><input type="radio" bind:group={condition} value="new" /> New ×1.0</label>
      <label><input type="radio" bind:group={condition} value="used" /> Used ×0.6</label>
      <label><input type="radio" bind:group={condition} value="very_used" /> Very used ×0.3</label>
      <label>
        <input type="radio" bind:group={condition} value="custom" />
        Custom ×
        <input
          type="number"
          step="any"
          bind:value={customMult}
          disabled={condition !== 'custom'}
          class="cust-mult"
        />
      </label>
    </fieldset>

    <fieldset class="mods">
      <legend>Custom modifiers (stack multiplicatively)</legend>
      <ul>
        {#each customMods as m, i (i)}
          <li>
            <span>{m.label} ×{m.mult}</span>
            <button type="button" class="mod-remove" onclick={() => removeModifier(i)}>
              Remove
            </button>
          </li>
        {/each}
      </ul>
      <div class="mod-add">
        <input type="text" placeholder="label" bind:value={newModLabel} />
        <input type="number" step="any" bind:value={newModMult} class="cust-mult" />
        <button type="button" class="mod-add-btn" onclick={addModifier}>Add modifier</button>
      </div>
    </fieldset>

    <label class="field">
      <span>Asset name</span>
      <input type="text" bind:value={assetName} />
    </label>

    <label class="field">
      <span>Notes (optional)</span>
      <textarea bind:value={notes} rows="2"></textarea>
    </label>

    <p class="final">
      <strong>Final price: {fmt(finalPrice)}</strong>
      <span class="wallet">(liquid: {fmt(wallet.liquid)})</span>
    </p>
    {#if !canAfford}
      <p class="warn">Insufficient liquid funds.</p>
    {/if}

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm} disabled={!canAfford}>
        Buy for {fmt(finalPrice)}
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
    gap: 0.75rem;
    padding: 1rem;
    background: var(--bg-card);
    color: var(--text-primary);
    border: 1px solid var(--border-card);
    border-radius: 0.3rem;
    min-width: 32rem;
    max-width: 90vw;
    max-height: 90vh;
    overflow-y: auto;
  }
  h3 {
    margin: 0;
    color: var(--accent);
  }
  .examples {
    color: var(--text-muted);
    margin: 0;
    font-size: 0.82rem;
  }
  fieldset {
    border: 0;
    padding: 0;
    display: grid;
    gap: 0.5rem;
    align-items: start;
  }
  fieldset legend {
    font-size: 0.82rem;
    color: var(--text-muted);
    padding: 0;
  }
  .band input[type="range"] {
    width: 100%;
  }
  .cust-mult {
    width: 6rem;
  }
  .cond label {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    margin-right: 0.75rem;
    font-size: 0.82rem;
    color: var(--text-primary);
  }
  .mods ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: grid;
    gap: 0.25rem;
  }
  .mods li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.82rem;
    color: var(--text-primary);
  }
  .mod-add {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }
  .field {
    display: grid;
    align-items: start;
    gap: 0.25rem;
    font-size: 0.82rem;
    color: var(--text-muted);
  }
  .field input,
  .field textarea,
  .band input[type="number"],
  .mod-add input[type="text"],
  .mod-add input[type="number"],
  .cond input[type="number"] {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.4rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.82rem;
    font-family: inherit;
  }
  .final {
    margin: 0;
    padding: 0.5rem;
    background: var(--bg-active);
    border-radius: 0.2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    color: var(--text-primary);
    font-size: 0.82rem;
  }
  .wallet {
    color: var(--text-muted);
    font-size: 0.82rem;
  }
  .warn {
    color: var(--accent);
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
  .primary:hover:not(:disabled) {
    color: var(--accent-bright) !important;
  }
  .primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .mod-remove,
  .mod-add-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 0.3rem;
    padding: 0.3rem 0.6rem;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .mod-remove:hover,
  .mod-add-btn:hover {
    color: var(--accent-bright);
  }
</style>
