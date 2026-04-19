<script lang="ts">
  import type { ChronicleEdge } from '../../../types';
  import * as api from '../../domains/api';
  import { session, cache, selectNode, refreshEdges } from '../../../store/domains.svelte';
  import EdgePicker from './EdgePicker.svelte';

  let picking = $state(false);
  let localError = $state('');

  const outgoing = $derived(
    session.nodeId == null
      ? []
      : cache.edges.filter(e => e.from_node_id === session.nodeId && e.edge_type !== 'contains')
  );
  const incoming = $derived(
    session.nodeId == null
      ? []
      : cache.edges.filter(e => e.to_node_id === session.nodeId && e.edge_type !== 'contains')
  );

  function labelOf(id: number): string {
    return cache.nodes.find(n => n.id === id)?.label ?? `#${id}`;
  }

  async function removeEdge(e: ChronicleEdge) {
    if (!confirm(`Remove relationship "${e.edge_type}"?`)) return;
    try {
      await api.deleteEdge(e.id);
      await refreshEdges();
    } catch (err) {
      localError = String(err);
    }
  }
</script>

<aside class="panel">
  <div class="panel-header">Relationships</div>

  {#if session.nodeId == null}
    <p class="empty">Select a node to see its relationships.</p>
  {:else}
    {#if localError}<p class="error">{localError}</p>{/if}

    <div class="group">
      <div class="group-label">→ Outgoing</div>
      {#if outgoing.length === 0}
        <p class="empty small">(none)</p>
      {:else}
        {#each outgoing as e (e.id)}
          <div class="edge">
            <span class="edge-type">{e.edge_type}</span>
            <button class="linked" onclick={() => selectNode(e.to_node_id)}>{labelOf(e.to_node_id)}</button>
            <button class="remove" title="Remove" onclick={() => removeEdge(e)}>✕</button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="group">
      <div class="group-label">← Incoming</div>
      {#if incoming.length === 0}
        <p class="empty small">(none)</p>
      {:else}
        {#each incoming as e (e.id)}
          <div class="edge">
            <span class="edge-type">{e.edge_type}</span>
            <button class="linked" onclick={() => selectNode(e.from_node_id)}>{labelOf(e.from_node_id)}</button>
            <button class="remove" title="Remove" onclick={() => removeEdge(e)}>✕</button>
          </div>
        {/each}
      {/if}
    </div>

    {#if picking}
      <EdgePicker
        oncancel={() => picking = false}
        onsave={() => picking = false}
      />
    {:else}
      <button class="add-btn" onclick={() => picking = true}>+ Add relationship</button>
    {/if}
  {/if}
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.65rem 0.7rem;
    border-left: 1px solid var(--border-surface);
    background: var(--bg-sunken);
    overflow: auto;
  }
  .panel-header {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #7c9;
  }
  .group { display: flex; flex-direction: column; gap: 0.15rem; }
  .group-label {
    font-size: 0.55rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .empty { color: var(--text-ghost); font-size: 0.72rem; padding: 0.2rem 0; }
  .empty.small { font-size: 0.65rem; }
  .edge {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.4rem;
    align-items: center;
    padding: 0.15rem 0;
    font-size: 0.72rem;
    color: var(--text-secondary);
  }
  .edge-type { color: var(--text-muted); font-style: italic; }
  .linked {
    background: none;
    border: 0;
    padding: 0;
    color: #9cf;
    font-size: inherit;
    cursor: pointer;
    text-align: left;
  }
  .linked:hover { text-decoration: underline; }
  .remove {
    background: none;
    border: 1px solid var(--border-faint);
    color: var(--text-ghost);
    border-radius: 3px;
    padding: 0 0.25rem;
    font-size: 0.6rem;
    cursor: pointer;
  }
  .remove:hover { color: var(--accent); border-color: var(--accent); }
  .add-btn {
    background: none;
    border: 0;
    color: var(--text-ghost);
    font-size: 0.7rem;
    padding: 0.3rem 0;
    cursor: pointer;
    text-align: left;
    margin-top: auto;
  }
  .add-btn:hover { color: var(--accent); }
  .error { font-size: 0.65rem; color: var(--accent-bright); }
</style>
