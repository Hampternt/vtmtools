<script lang="ts">
  // Roll feed — reverse-chronological view of the bridge's roll-history ring.
  // Subscribes to the rolls store from Plan B; filters per-actor and per-splat.
  //
  // Color token mapping (all reuse — no new :root tokens added):
  //   vampire        → --alert-card-dossier
  //   werewolf       → --accent-amber
  //   hunter         → --accent
  //   mortal/unknown → --text-muted
  //   success/6-9    → --text-primary
  //   critical/10    → --alert-card-dossier
  //   failure/1-5    → --text-muted
  //   bestial 1      → --alert-card-dossier (dashed outline)
  //   brutal 1       → --accent-amber (dashed outline)
  //   messy (hunger 10) → --alert-card-dossier (saturated bg)
  //   desperation-fail (hunter 1) → --accent (dashed outline)
  //
  // Per-row rendering lives in src/lib/components/RollEntry.svelte (split per
  // plan-c Step 4 — RollFeed.svelte was projected to exceed ~250 lines inline).

  import { onMount } from 'svelte';
  import { rolls } from '../store/rolls.svelte';
  import type { RollSplat } from '../types';
  import RollEntry from '$lib/components/RollEntry.svelte';

  onMount(() => { void rolls.ensureLoaded(); });

  // ── Filter state ────────────────────────────────────────────────────────
  let actorFilter = $state<string>('');   // empty = all actors
  let splatFilter = $state<RollSplat | ''>('');

  const actorOptions = $derived(
    Array.from(new Set(rolls.list.map(r => r.actor_name).filter((n): n is string => !!n))).sort()
  );
  const splatOptions: RollSplat[] = ['mortal', 'vampire', 'werewolf', 'hunter'];

  const filteredRolls = $derived(
    rolls.list.filter(r => {
      if (actorFilter && r.actor_name !== actorFilter) return false;
      if (splatFilter && r.splat !== splatFilter) return false;
      return true;
    })
  );

  function splatLabel(s: RollSplat): string {
    if (s === 'unknown') return '?';
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  function clearFilters() {
    actorFilter = '';
    splatFilter = '';
  }
</script>

<div class="roll-feed">
  <div class="toolbar">
    <span class="title">Rolls</span>
    <span class="count" class:dim={filteredRolls.length === 0}>{filteredRolls.length}</span>
    <span class="spacer"></span>
    <select class="filter" bind:value={splatFilter}>
      <option value="">All splats</option>
      {#each splatOptions as s}
        <option value={s}>{splatLabel(s)}</option>
      {/each}
    </select>
    <select class="filter" bind:value={actorFilter}>
      <option value="">All actors</option>
      {#each actorOptions as a}
        <option value={a}>{a}</option>
      {/each}
    </select>
    {#if actorFilter || splatFilter}
      <button class="btn-clear" onclick={clearFilters}>clear</button>
    {/if}
  </div>

  {#if rolls.list.length === 0}
    <div class="empty">
      No rolls yet — when a roll resolves in Foundry, it appears here.
    </div>
  {:else if filteredRolls.length === 0}
    <div class="empty">
      No rolls match the current filter.
    </div>
  {:else}
    <div class="entries">
      {#each filteredRolls as roll (roll.source_id)}
        <RollEntry {roll} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .roll-feed {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    height: 100%;
    padding: 0.75rem;
    box-sizing: border-box;
  }

  .toolbar {
    display: flex; align-items: center; gap: 0.5rem;
    padding-bottom: 0.5rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .title {
    font-size: 0.85rem;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    color: var(--text-label);
    font-weight: 600;
  }
  .count {
    font-size: 0.65rem;
    color: var(--text-secondary);
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 10px;
    padding: 0 0.5rem;
    line-height: 1.6;
  }
  .count.dim { opacity: 0.4; }
  .spacer { flex: 1; }
  .filter {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
  }
  .btn-clear {
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.25rem 0.5rem;
    font-size: 0.7rem;
    cursor: pointer;
    text-transform: lowercase;
  }
  .btn-clear:hover { color: var(--text-primary); }

  .empty {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    font-style: italic;
    font-size: 0.85rem;
    text-align: center;
    padding: 2rem 1rem;
  }

  .entries {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    overflow-y: auto;
    flex: 1;
    padding-right: 0.25rem;
  }
  .entries::-webkit-scrollbar { width: 4px; }
  .entries::-webkit-scrollbar-track { background: transparent; }
  .entries::-webkit-scrollbar-thumb { background: var(--border-faint); border-radius: 2px; }
</style>
