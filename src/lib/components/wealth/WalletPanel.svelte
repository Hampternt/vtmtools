<script lang="ts">
  // Wallet panel: 3-column grid (liquid / banked / assets Σ).
  // Pure display + button callbacks — no IPC. Caller handles state +
  // refreshes via onOpenAdjust / onOpenTransfer.

  import type { Wallet, Asset } from '../../../types';

  let {
    wallet,
    assets,
    onOpenAdjust,
    onOpenTransfer,
  }: {
    wallet: Wallet;
    assets: Asset[];
    onOpenAdjust: (bucket: 'liquid' | 'banked') => void;
    onOpenTransfer: () => void;
  } = $props();

  let assetsTotal = $derived(
    assets.reduce((sum, a) => sum + a.currentValue, 0)
  );

  function fmt(n: number): string {
    return wallet.currencyName + n.toLocaleString();
  }
</script>

<section class="wallet-panel">
  <div class="bucket">
    <span class="label">Liquid</span>
    <span class="value">{fmt(wallet.liquid)}</span>
    <button type="button" class="action" onclick={() => onOpenAdjust('liquid')}>Adjust</button>
  </div>

  <div class="bucket">
    <span class="label">Banked</span>
    <span class="value">{fmt(wallet.banked)}</span>
    <div class="button-row">
      <button type="button" class="action" onclick={() => onOpenAdjust('banked')}>Adjust</button>
      <button type="button" class="action" onclick={onOpenTransfer}>Transfer</button>
    </div>
  </div>

  <div class="bucket">
    <span class="label">Assets Σ</span>
    <span class="value">{fmt(assetsTotal)}</span>
    <span class="hint">
      derived from current_value of {assets.length} asset{assets.length === 1 ? '' : 's'}
    </span>
  </div>
</section>

<style>
  /* Token substitutions: see WealthManager.svelte mapping table. */

  .wallet-panel {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
    align-items: start;
    padding: 0.75rem;
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.3rem;
  }
  .bucket {
    display: grid;
    align-items: start;
    gap: 0.25rem;
  }
  .label {
    font-size: 0.82rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .value {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }
  .button-row {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.25rem;
  }
  .action {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 0.3rem;
    padding: 0.35rem 0.75rem;
    font-size: 0.78rem;
    cursor: pointer;
    justify-self: start;
    margin-top: 0.25rem;
  }
  .action:hover {
    color: var(--accent-bright);
  }
  .button-row .action {
    margin-top: 0;
  }
</style>
