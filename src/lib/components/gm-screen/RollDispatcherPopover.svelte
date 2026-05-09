<script lang="ts">
  import type { BridgeCharacter, CharacterModifier } from '../../../types';
  import {
    DEFAULT_PROVIDERS,
    type PathProvider,
  } from '../../gm-screen/path-providers';
  import {
    summarizeModifiers,
    dispatchRoll,
  } from '../../gm-screen/roll';

  interface Props {
    character: BridgeCharacter;
    modifiers: CharacterModifier[];
    /** Viewport-coord anchor from the trigger button's getBoundingClientRect. */
    anchor: { left: number; top: number };
    onclose: () => void;
  }
  let { character, modifiers, anchor, onclose }: Props = $props();

  // PathProvider state — keyed by provider.id, value is the chosen option.key.
  let selections = $state<Record<string, string>>({});

  // Difficulty + privacy state.
  let baseDifficulty = $state(0);
  let customFlavor = $state('');
  let rollMode = $state<'roll' | 'gmroll' | 'blindroll' | 'selfroll'>('roll');

  // Submission UI state.
  let submitting = $state(false);
  let errorMsg = $state<string | null>(null);

  // Derived: per-provider option lists (for the dropdowns).
  const providerOptions = $derived(
    DEFAULT_PROVIDERS.map((p) => ({
      provider: p,
      options: p.getOptions(character),
    })),
  );

  // Derived: modifier sums (recomputed if upstream modifier toggle changes).
  const sums = $derived(summarizeModifiers(modifiers));

  // Derived: validation — required providers must all have a selection.
  const requiredMissing = $derived(
    DEFAULT_PROVIDERS.filter(
      (p) => p.required && !selections[p.id],
    ).map((p) => p.label),
  );
  const canSubmit = $derived(requiredMissing.length === 0 && !submitting);

  // Derived: preview values shown to the GM before they roll.
  const baseFromStats = $derived.by(() => {
    let sum = 0;
    for (const { provider, options } of providerOptions) {
      const optKey = selections[provider.id];
      const opt = options.find((o) => o.key === optKey);
      if (opt) sum += opt.value;
    }
    return sum;
  });
  const finalPool = $derived(baseFromStats + sums.pool);
  const finalDifficulty = $derived(Math.max(0, baseDifficulty + sums.difficulty));

  async function onSubmit() {
    errorMsg = null;
    submitting = true;
    try {
      await dispatchRoll({
        char: character,
        providers: DEFAULT_PROVIDERS,
        selections,
        baseDifficulty,
        modifierSums: sums,
        customFlavor,
        rollMode,
      });
      onclose();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
    } finally {
      submitting = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />
<!-- Click-outside scrim. The popover content stops propagation. -->
<button class="scrim" onclick={onclose} aria-label="Close roll dispatcher"></button>

<div
  class="popover"
  role="dialog"
  aria-label="Roll for {character.name}"
  tabindex="-1"
  style:left="{anchor.left}px"
  style:top="{anchor.top}px"
  onclick={(e) => e.stopPropagation()}
  onkeydown={(e) => e.stopPropagation()}
>
  <div class="header">Roll for <strong>{character.name}</strong></div>

  <div class="body">
    {#each providerOptions as { provider, options } (provider.id)}
      <label class="row">
        <span class="label">{provider.label}{provider.required ? ' *' : ''}</span>
        <select bind:value={selections[provider.id]}>
          <option value="">— pick {provider.label.toLowerCase()} —</option>
          {#each options as opt (opt.key)}
            <option value={opt.key}>{opt.label} ({opt.value})</option>
          {/each}
        </select>
      </label>
    {/each}

    <label class="row">
      <span class="label">Base difficulty</span>
      <input
        type="number"
        min="0"
        max="20"
        bind:value={baseDifficulty}
      />
    </label>

    {#if sums.pool !== 0 || sums.difficulty !== 0 || sums.notes.length > 0}
      <div class="modifier-summary">
        {#if sums.pool !== 0}
          <div class="mod-line">
            <span class="mod-label">Pool modifiers:</span>
            <span class="mod-val" class:positive={sums.pool > 0} class:negative={sums.pool < 0}>
              {sums.pool > 0 ? '+' : ''}{sums.pool}
            </span>
          </div>
        {/if}
        {#if sums.difficulty !== 0}
          <div class="mod-line">
            <span class="mod-label">Difficulty modifiers:</span>
            <span class="mod-val" class:positive={sums.difficulty > 0} class:negative={sums.difficulty < 0}>
              {sums.difficulty > 0 ? '+' : ''}{sums.difficulty}
            </span>
          </div>
        {/if}
        {#each sums.notes as note (note)}
          <div class="mod-note">📝 {note}</div>
        {/each}
      </div>
    {/if}

    <div class="totals">
      <div class="total-line">
        <span class="total-label">Pool:</span>
        <span class="total-val">
          {baseFromStats}{sums.pool !== 0 ? ` ${sums.pool > 0 ? '+' : '−'} ${Math.abs(sums.pool)}` : ''}
          {sums.pool !== 0 ? ` = ${finalPool}` : ''}
        </span>
      </div>
      <div class="total-line">
        <span class="total-label">Difficulty:</span>
        <span class="total-val">
          {baseDifficulty}{sums.difficulty !== 0 ? ` ${sums.difficulty > 0 ? '+' : '−'} ${Math.abs(sums.difficulty)}` : ''}
          {sums.difficulty !== 0 ? ` = ${finalDifficulty}` : ''}
        </span>
      </div>
    </div>

    <details class="privacy">
      <summary>Privacy / flavor</summary>
      <label class="row">
        <span class="label">Custom flavor</span>
        <input
          type="text"
          placeholder="auto: {providerOptions.map(({ provider, options }) => options.find((o) => o.key === selections[provider.id])?.label).filter(Boolean).join(' + ') || 'Roll'}"
          bind:value={customFlavor}
        />
      </label>
      <label class="row">
        <span class="label">Roll mode</span>
        <select bind:value={rollMode}>
          <option value="roll">Public roll</option>
          <option value="gmroll">GM only</option>
          <option value="blindroll">Blind roll</option>
          <option value="selfroll">Self roll</option>
        </select>
      </label>
    </details>

    {#if errorMsg}
      <div class="error">⚠ {errorMsg}</div>
    {/if}
  </div>

  <div class="actions">
    <button class="btn-cancel" onclick={onclose}>Cancel</button>
    <button class="btn-submit" disabled={!canSubmit} onclick={onSubmit}>
      {submitting ? 'Rolling…' : '🎲 Roll in Foundry'}
    </button>
  </div>
  {#if requiredMissing.length > 0}
    <div class="missing-hint">
      Pick: {requiredMissing.join(', ')}
    </div>
  {/if}
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: transparent;
    border: none;
    padding: 0;
    cursor: default;
    z-index: 100;
  }

  .popover {
    position: fixed;
    z-index: 101;
    min-width: 320px;
    max-width: 420px;
    background: var(--bg-raised);
    border: 1px solid var(--border-faint);
    border-radius: 8px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    box-shadow: 0 6px 24px var(--shadow-strong);
    color: var(--text-primary);
  }

  .header {
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-faint);
    padding-bottom: 0.4rem;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .row {
    display: grid;
    grid-template-columns: 7rem 1fr;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .label {
    color: var(--text-ghost);
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.05em;
  }

  .row select,
  .row input {
    background: var(--bg-sunken);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    font-size: 0.85rem;
  }

  .modifier-summary {
    background: var(--bg-sunken);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.8rem;
  }

  .mod-line {
    display: flex;
    justify-content: space-between;
  }

  .mod-label {
    color: var(--text-ghost);
  }

  .mod-val.positive {
    color: var(--accent-bright);
  }

  .mod-val.negative {
    color: var(--accent-amber);
  }

  .mod-note {
    color: var(--text-secondary);
    font-style: italic;
  }

  .totals {
    border-top: 1px dashed var(--border-faint);
    padding-top: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.9rem;
  }

  .total-line {
    display: flex;
    justify-content: space-between;
  }

  .total-label {
    color: var(--text-ghost);
    font-weight: 600;
  }

  .total-val {
    font-weight: 700;
  }

  .privacy {
    background: var(--bg-sunken);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-size: 0.8rem;
  }

  .privacy summary {
    cursor: pointer;
    color: var(--text-ghost);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
    padding: 0.1rem 0;
  }

  .privacy .row {
    margin-top: 0.4rem;
  }

  .error {
    background: var(--bg-active);
    color: var(--accent-bright);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    font-size: 0.8rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
  }

  .btn-cancel,
  .btn-submit {
    padding: 0.35rem 0.8rem;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .btn-cancel {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
  }

  .btn-submit {
    background: var(--accent);
    color: var(--text-primary);
    border: 1px solid var(--accent);
    font-weight: 600;
  }

  .btn-submit:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .missing-hint {
    font-size: 0.7rem;
    color: var(--accent-amber);
    text-align: right;
    padding-top: 0.2rem;
  }
</style>
