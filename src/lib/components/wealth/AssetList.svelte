<script lang="ts">
  // Asset list: table-of-owned-assets with Sell + Remove row actions and a
  // "+ Add" header button. Pure display + button callbacks — no IPC. Caller
  // owns state + refreshes.
  //
  // CSS tokens substituted per the actual repo theme (see WealthManager.svelte
  // for the same mapping table). Buttons match the muted-action style used by
  // WalletPanel + Task 8's modals — no saturated accent fill.

  import type { Asset } from '../../../types';

  let {
    assets,
    currencyName,
    onAdd,
    onSell,
    onRemove,
  }: {
    assets: Asset[];
    currencyName: string;
    onAdd: () => void;
    onSell: (a: Asset) => void;
    onRemove: (a: Asset) => void;
  } = $props();

  function fmt(n: number): string {
    return currencyName + n.toLocaleString();
  }
</script>

<section class="asset-list">
  <header>
    <h3>Assets</h3>
    <button type="button" class="action" onclick={onAdd}>+ Add</button>
  </header>

  {#if assets.length === 0}
    <p class="empty">No assets yet. Buy one from the shop, or add a manual entry.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Category</th>
          <th>Condition</th>
          <th class="num">Value</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each assets as a (a.id)}
          <tr>
            <td>{a.name}</td>
            <td>{a.category ?? '—'}</td>
            <td>{a.condition ?? '—'}</td>
            <td class="num">{fmt(a.currentValue)}</td>
            <td class="row-actions">
              <button type="button" class="action" onclick={() => onSell(a)}>Sell</button>
              <button type="button" class="action danger" onclick={() => onRemove(a)}>Remove</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  /* Token substitutions: see WealthManager.svelte mapping table. */

  .asset-list {
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
  }
  h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--accent);
  }
  .empty {
    color: var(--text-muted);
    text-align: center;
    padding: 0.75rem;
    margin: 0;
    font-size: 0.82rem;
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
  }
  th {
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    font-weight: 600;
  }
  td.num,
  th.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .row-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
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
  .danger {
    border-color: var(--accent);
  }
</style>
