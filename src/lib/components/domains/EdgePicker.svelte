<script lang="ts">
  import * as api from '../../domains/api';
  import { session, cache, refreshEdges } from '../../../store/domains.svelte';

  const { oncancel, onsave }: {
    oncancel: () => void;
    onsave: () => void;
  } = $props();

  let targetId = $state<number | ''>('');
  let edgeType = $state('');
  let description = $state('');
  let saving = $state(false);
  let localError = $state('');

  // Suggest edge-type strings from existing edges in the chronicle, excluding
  // 'contains' (contains is managed via the tree).
  const knownEdgeTypes = $derived(
    Array.from(new Set(
      cache.edges.map(e => e.edge_type).filter(t => t !== 'contains')
    )).sort()
  );

  // Candidate target nodes: every node except the current one.
  const candidates = $derived(
    cache.nodes
      .filter(n => n.id !== session.nodeId)
      .sort((a, b) => a.label.localeCompare(b.label))
  );

  async function save() {
    if (session.chronicleId == null || session.nodeId == null) { localError = 'No node selected.'; return; }
    if (targetId === '') { localError = 'Target node is required.'; return; }
    const et = edgeType.trim();
    if (!et) { localError = 'Edge type is required.'; return; }
    if (et === 'contains') { localError = 'Use the tree to manage contains relationships.'; return; }

    saving = true;
    localError = '';
    try {
      await api.createEdge(
        session.chronicleId, session.nodeId, Number(targetId), et, description, [],
      );
      await refreshEdges();
      onsave();
    } catch (e) {
      localError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form">
  <div class="form-title">Add relationship</div>

  <div class="field">
    <label for="ep-target">Target</label>
    <select id="ep-target" bind:value={targetId}>
      <option value="" disabled>Pick a node…</option>
      {#each candidates as c (c.id)}
        <option value={c.id}>{c.label} ({c.type})</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="ep-type">Type</label>
    <input id="ep-type" list="ep-types" bind:value={edgeType} placeholder="controls, allied-with…" />
    <datalist id="ep-types">
      {#each knownEdgeTypes as t (t)}
        <option value={t}></option>
      {/each}
    </datalist>
  </div>

  <div class="field">
    <label for="ep-desc">Description (optional)</label>
    <input id="ep-desc" bind:value={description} />
  </div>

  {#if localError}
    <p class="error">{localError}</p>
  {/if}

  <div class="actions">
    <button class="btn" onclick={oncancel} disabled={saving}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>

<style>
  .form {
    border: 1px solid var(--border-active);
    background: var(--bg-card);
    border-radius: 6px;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .form-title {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-label);
  }
  .field { display: flex; flex-direction: column; gap: 0.15rem; }
  label {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  input, select {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    color: var(--text-primary);
    font-size: 0.72rem;
    outline: none;
    font-family: inherit;
  }
  input:focus, select:focus { border-color: var(--accent); }
  .actions { display: flex; gap: 0.35rem; justify-content: flex-end; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.68rem;
    cursor: pointer;
  }
  .btn.primary { color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .error { font-size: 0.65rem; color: var(--accent-bright); }
</style>
