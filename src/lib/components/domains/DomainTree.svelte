<script lang="ts">
  import type { ChronicleNode } from '../../../types';
  import { session, cache, selectNode } from '../../../store/domains.svelte';

  // Nodes that appear in the tree root: those with no incoming 'contains' edge.
  const rootNodes = $derived(computeRoots());
  // Map parentId -> children[], so recursive rendering is O(n).
  const childrenByParent = $derived(computeChildrenMap());
  const expanded: Set<number> = $state(new Set());

  function computeRoots(): ChronicleNode[] {
    const containsTargets = new Set(
      cache.edges.filter(e => e.edge_type === 'contains').map(e => e.to_node_id)
    );
    return cache.nodes
      .filter(n => !containsTargets.has(n.id))
      .sort((a, b) => a.label.localeCompare(b.label));
  }

  function computeChildrenMap(): Map<number, ChronicleNode[]> {
    const byId = new Map<number, ChronicleNode>();
    for (const n of cache.nodes) byId.set(n.id, n);
    const m = new Map<number, ChronicleNode[]>();
    for (const e of cache.edges) {
      if (e.edge_type !== 'contains') continue;
      const child = byId.get(e.to_node_id);
      if (!child) continue;
      if (!m.has(e.from_node_id)) m.set(e.from_node_id, []);
      m.get(e.from_node_id)!.push(child);
    }
    for (const arr of m.values()) arr.sort((a, b) => a.label.localeCompare(b.label));
    return m;
  }

  function toggle(id: number) {
    // Svelte 5 proxies Set mutations, so .add/.delete trigger reactivity.
    if (expanded.has(id)) expanded.delete(id);
    else expanded.add(id);
  }
</script>

<aside class="tree">
  <div class="tree-header">Domains</div>

  {#if session.chronicleId == null}
    <p class="empty">No chronicle selected.</p>
  {:else if cache.nodes.length === 0}
    <p class="empty">This chronicle is empty. Create a node to begin.</p>
  {:else}
    <ul class="list">
      {#each rootNodes as node (node.id)}
        {@render treeRow(node, 0)}
      {/each}
    </ul>
  {/if}
</aside>

{#snippet treeRow(node: ChronicleNode, depth: number)}
  {@const kids = childrenByParent.get(node.id) ?? []}
  {@const open = expanded.has(node.id)}
  <li>
    <button
      class="row"
      class:active={session.nodeId === node.id}
      style="padding-left: calc(0.5rem + {depth * 0.8}rem)"
      onclick={() => selectNode(node.id)}
    >
      {#if kids.length > 0}
        <span
          class="caret"
          role="button"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); toggle(node.id); }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); toggle(node.id); } }}
        >{open ? '▾' : '▸'}</span>
      {:else}
        <span class="caret ghost">•</span>
      {/if}
      <span class="label">{node.label}</span>
      <span class="type">{node.type}</span>
    </button>
    {#if open && kids.length > 0}
      <ul class="list">
        {#each kids as child (child.id)}
          {@render treeRow(child, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
{/snippet}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: auto;
    border-right: 1px solid var(--border-surface);
    background: var(--bg-sunken);
  }
  .tree-header {
    padding: 0.45rem 0.65rem;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-faint);
  }
  .empty { color: var(--text-ghost); font-size: 0.74rem; padding: 0.6rem 0.8rem; }
  .list { list-style: none; margin: 0; padding: 0; }
  .row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    width: 100%;
    background: transparent;
    border: 0;
    border-left: 2px solid transparent;
    padding: 0.28rem 0.6rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
  }
  .row:hover { background: var(--bg-raised); color: var(--text-primary); }
  .row.active {
    background: var(--bg-active);
    color: var(--text-primary);
    border-left-color: var(--accent);
  }
  .caret {
    width: 0.85rem;
    color: var(--text-ghost);
    font-size: 0.65rem;
    user-select: none;
  }
  .caret.ghost { color: var(--border-faint); }
  .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .type {
    color: var(--text-ghost);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.7;
  }
</style>
