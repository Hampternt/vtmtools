<script lang="ts">
  // Shop panel: tabbed item-slot catalog surface. Tabs are the five asset
  // categories; the active tab drives a fresh listItemSlots() fetch.
  // Each row offers a "Buy…" affordance that bubbles the chosen slot up to
  // WealthManager via onBuy() — WealthManager owns the modal-open state.
  //
  // CSS tokens substituted per the actual repo theme (see WealthManager.svelte
  // for the same mapping table). Active tab + Buy button use the muted-action
  // convention from Tasks 8 / 9 (var(--bg-active) bg, accent border, accent
  // text) rather than a saturated accent fill.

  import type { ItemSlot, AssetCategory, Wallet } from '../../../types';
  import { listItemSlots } from '$lib/wealth/api';

  let {
    wallet,
    onBuy,
  }: {
    wallet: Wallet;
    onBuy: (slot: ItemSlot) => void;
  } = $props();

  const categories: AssetCategory[] = ['weapon', 'vehicle', 'property', 'investment', 'misc'];
  let active: AssetCategory = $state<AssetCategory>('weapon');
  let slots: ItemSlot[] = $state([]);
  let loading = $state(false);
  let error: string | null = $state(null);

  $effect(() => {
    const cat = active;
    loading = true;
    error = null;
    listItemSlots(cat)
      .then((s) => { slots = s; })
      .catch((e) => { error = String(e); })
      .finally(() => { loading = false; });
  });

  function fmt(n: number): string {
    return wallet.currencyName + n.toLocaleString();
  }
</script>

<section class="shop-panel">
  <header>
    <h3>Shop</h3>
    <nav class="cat-tabs" role="tablist">
      {#each categories as c}
        <button
          type="button"
          role="tab"
          aria-selected={c === active}
          class:active={c === active}
          onclick={() => (active = c)}
        >{c}</button>
      {/each}
    </nav>
  </header>

  {#if loading}
    <p class="empty">Loading…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if slots.length === 0}
    <p class="empty">No slots in this category.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Slot</th>
          <th>Price band</th>
          <th>Examples</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each slots as s (s.id)}
          <tr>
            <td>{s.label}</td>
            <td class="num">{fmt(s.basePriceMin)} – {fmt(s.basePriceMax)}</td>
            <td class="examples">{s.examples ?? '—'}</td>
            <td><button type="button" class="action" onclick={() => onBuy(s)}>Buy…</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  /* Token substitutions: see WealthManager.svelte mapping table. */

  .shop-panel {
    display: grid;
    align-items: start;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.3rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--accent);
  }
  .cat-tabs {
    display: flex;
    gap: 0.25rem;
    flex-wrap: wrap;
  }
  .cat-tabs button {
    background: transparent;
    border: 1px solid var(--border-card);
    padding: 0.3rem 0.6rem;
    border-radius: 0.2rem;
    color: var(--text-muted);
    text-transform: capitalize;
    cursor: pointer;
    font-size: 0.78rem;
  }
  .cat-tabs button:hover {
    color: var(--text-primary);
  }
  .cat-tabs button.active {
    background: var(--bg-active);
    border-color: var(--accent);
    color: var(--accent);
  }
  .empty,
  .error {
    text-align: center;
    color: var(--text-muted);
    padding: 0.75rem;
    margin: 0;
    font-size: 0.82rem;
  }
  .error {
    color: var(--accent);
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: left;
    padding: 0.5rem;
    border-bottom: 1px solid var(--border-card);
    font-size: 0.82rem;
    color: var(--text-primary);
    vertical-align: top;
  }
  th {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    font-weight: 600;
  }
  td.num {
    font-variant-numeric: tabular-nums;
  }
  .examples {
    color: var(--text-muted);
    font-style: italic;
    font-size: 0.78rem;
  }
  .action {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 0.3rem;
    padding: 0.3rem 0.6rem;
    font-size: 0.78rem;
    cursor: pointer;
  }
  .action:hover {
    color: var(--accent-bright);
  }
</style>
