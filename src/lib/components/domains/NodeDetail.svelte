<script lang="ts">
  import PropertyEditor from '../properties/PropertyEditor.svelte';
  import NodeForm from './NodeForm.svelte';
  import * as api from '../../domains/api';
  import type { ChronicleNode } from '../../../types';
  import { session, cache, refreshNodes, refreshEdges, selectNode } from '../../../store/domains.svelte';

  const node = $derived(cache.nodes.find(n => n.id === session.nodeId) ?? null);

  const childCount = $derived(
    node == null
      ? 0
      : cache.edges.filter(e => e.edge_type === 'contains' && e.from_node_id === node.id).length
  );

  // Walk the contains edges to build an ancestor list (top-down).
  const pathToRoot = $derived(computePath());

  function computePath(): ChronicleNode[] {
    if (!node) return [];
    const byId = new Map(cache.nodes.map(n => [n.id, n]));
    const byChild = new Map<number, number>();
    for (const e of cache.edges) {
      if (e.edge_type === 'contains') byChild.set(e.to_node_id, e.from_node_id);
    }
    const path: ChronicleNode[] = [];
    let cur: number | undefined = byChild.get(node.id);
    let guard = 0;
    while (cur != null && guard++ < 64) {
      const p = byId.get(cur);
      if (!p) break;
      path.unshift(p);
      cur = byChild.get(cur);
    }
    return path;
  }

  let editing = $state(false);
  let deleting = $state(false);

  $effect(() => {
    session.nodeId;
    editing = false;
  });

  async function handleDelete() {
    if (!node) return;
    const msg = childCount > 0
      ? `Delete "${node.label}"? ${childCount} child node${childCount === 1 ? '' : 's'} will also be removed (cascade). This cannot be undone.`
      : `Delete "${node.label}"? This cannot be undone.`;
    if (!confirm(msg)) return;
    deleting = true;
    try {
      await api.deleteNode(node.id);
      selectNode(null);
      await refreshNodes();
      await refreshEdges();
    } catch (e) {
      alert(`Delete failed: ${e}`);
    } finally {
      deleting = false;
    }
  }
</script>

<section class="detail">
  {#if !node}
    <p class="muted">No node selected. Click one in the tree to view its details.</p>
  {:else if editing}
    <NodeForm
      node={node}
      oncancel={() => editing = false}
      onsave={() => editing = false}
    />
  {:else}
    {#if pathToRoot.length > 0}
      <nav class="crumbs" aria-label="breadcrumb">
        {#each pathToRoot as p, i (p.id)}
          <button class="crumb" onclick={() => selectNode(p.id)}>{p.label}</button>
          {#if i < pathToRoot.length}<span class="sep">▸</span>{/if}
        {/each}
      </nav>
    {/if}
    <header class="head">
      <span class="title">{node.label}</span>
      <span class="type-chip">{node.type}</span>
      <span class="spacer"></span>
      <button class="btn" onclick={() => editing = true}>✎ Edit</button>
      <button class="btn danger" onclick={handleDelete} disabled={deleting}>
        {deleting ? 'Deleting…' : '✕ Delete'}
      </button>
    </header>

    {#if node.tags.length > 0}
      <div class="tags">
        {#each node.tags as tag (tag)}
          <span class="tag">{tag}</span>
        {/each}
      </div>
    {/if}

    {#if node.description}
      <p class="desc">{node.description}</p>
    {:else}
      <p class="desc muted">(no description)</p>
    {/if}

    <div class="props">
      <div class="props-label">Properties</div>
      {#if node.properties.length === 0}
        <p class="muted small">(none)</p>
      {:else}
        {#each node.properties as f (f.name)}
          <PropertyEditor field={f} readonly={true} />
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .detail { padding: 0.9rem 1rem; display: flex; flex-direction: column; gap: 0.55rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.8rem; }
  .muted.small { font-size: 0.7rem; }
  .crumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.65rem;
    color: var(--text-ghost);
  }
  .crumb {
    background: none;
    border: 0;
    color: var(--text-muted);
    font-size: inherit;
    padding: 0;
    cursor: pointer;
  }
  .crumb:hover { color: var(--text-primary); text-decoration: underline; }
  .sep { color: var(--text-ghost); }
  .head { display: flex; align-items: center; gap: 0.5rem; }
  .title { font-size: 1.1rem; font-weight: 600; color: var(--text-primary); }
  .type-chip {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
    background: var(--bg-raised);
    border: 1px solid var(--border-card);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
  }
  .spacer { flex: 1; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent-amber);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .btn:hover { box-shadow: 0 0 8px #cc992255; }
  .btn.danger { color: var(--accent); }
  .btn.danger:hover { box-shadow: 0 0 8px #cc222255; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .tags { display: flex; gap: 0.3rem; flex-wrap: wrap; }
  .tag {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    font-size: 0.6rem;
    padding: 0.1rem 0.4rem;
    color: var(--text-label);
  }
  .desc {
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.55;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
    white-space: pre-wrap;
  }
  .props { border-top: 1px solid var(--border-faint); padding-top: 0.5rem; }
  .props-label {
    font-size: 0.55rem;
    color: #7c9;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.25rem;
  }
</style>
