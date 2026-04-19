<script lang="ts">
  import type { ChronicleNode } from '../../../types';
  import { session, cache, selectNode } from '../../../store/domains.svelte';
  import NodeForm from './NodeForm.svelte';

  // Nodes that appear in the tree root: those with no incoming 'contains' edge.
  const rootNodes = $derived(computeRoots());
  const childrenByParent = $derived(computeChildrenMap());
  const expanded: Set<number> = $state(new Set());

  let adding = $state<'root' | number | null>(null); // 'root' or parent node id

  let rawSearch = $state('');
  let searchQuery = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  function onSearchInput(e: Event) {
    rawSearch = (e.target as HTMLInputElement).value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = rawSearch; }, 110);
  }

  $effect(() => () => { if (searchTimer) clearTimeout(searchTimer); });

  // Filter: a node is visible if it matches or has a matching descendant.
  const visibleIds = $derived(computeVisibleIds());

  function computeVisibleIds(): Set<number> | null {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return null; // null = show all

    const matches = new Set<number>();
    for (const n of cache.nodes) {
      const hit =
        n.label.toLowerCase().includes(q) ||
        n.description.toLowerCase().includes(q) ||
        n.tags.some(t => t.toLowerCase().includes(q));
      if (hit) matches.add(n.id);
    }
    // Include all ancestors of matches so they stay visible.
    const keep = new Set<number>(matches);
    const byChild = new Map<number, number>();
    for (const e of cache.edges) {
      if (e.edge_type === 'contains') byChild.set(e.to_node_id, e.from_node_id);
    }
    for (const id of matches) {
      let cur: number | undefined = byChild.get(id);
      while (cur != null) {
        keep.add(cur);
        cur = byChild.get(cur);
      }
    }
    return keep;
  }

  function isVisible(id: number): boolean {
    return visibleIds == null || visibleIds.has(id);
  }

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
    if (expanded.has(id)) expanded.delete(id);
    else expanded.add(id);
  }

  function onFormDone() {
    adding = null;
  }
</script>

<aside class="tree">
  <div class="tree-header">Domains</div>

  <div class="search-wrap">
    <input
      class="search"
      type="text"
      value={rawSearch}
      oninput={onSearchInput}
      placeholder="🔍 Search this chronicle…"
    />
  </div>

  {#if session.chronicleId == null}
    <p class="empty">No chronicle selected.</p>
  {:else}
    {#if adding === 'root'}
      <div class="inline-form">
        <NodeForm parentId={null} oncancel={onFormDone} onsave={onFormDone} />
      </div>
    {/if}

    {#if cache.nodes.length === 0 && adding !== 'root'}
      <p class="empty">This chronicle is empty. Create a node to begin.</p>
    {:else}
      <ul class="list">
        {#each rootNodes as node (node.id)}
          {@render treeRow(node, 0)}
        {/each}
      </ul>
    {/if}

    <div class="tree-footer">
      <button class="add-btn" onclick={() => adding = 'root'} disabled={adding !== null}>
        + Add root node
      </button>
    </div>
  {/if}
</aside>

{#snippet treeRow(node: ChronicleNode, depth: number)}
  {@const kids = childrenByParent.get(node.id) ?? []}
  {@const open = expanded.has(node.id)}
  {#if isVisible(node.id)}
  <li>
    <div
      class="row-wrap"
      class:active={session.nodeId === node.id}
    >
      <button
        class="row"
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
      <button
        class="add-child"
        title="Add child"
        onclick={(e) => { e.stopPropagation(); adding = node.id; }}
      >+</button>
    </div>

    {#if adding === node.id}
      <div class="inline-form nested" style="padding-left: calc(0.5rem + {(depth + 1) * 0.8}rem)">
        <NodeForm parentId={node.id} oncancel={onFormDone} onsave={onFormDone} />
      </div>
    {/if}

    {#if open && kids.length > 0}
      <ul class="list">
        {#each kids as child (child.id)}
          {@render treeRow(child, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
  {/if}
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
  .search-wrap { padding: 0.35rem 0.5rem; border-bottom: 1px solid var(--border-faint); }
  .search {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.74rem;
    outline: none;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-ghost); }
  .empty { color: var(--text-ghost); font-size: 0.74rem; padding: 0.6rem 0.8rem; }
  .list { list-style: none; margin: 0; padding: 0; }
  .row-wrap {
    display: flex;
    align-items: center;
    gap: 0;
    border-left: 2px solid transparent;
  }
  .row-wrap.active {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .row-wrap:hover .add-child { opacity: 1; }
  .row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    padding: 0.28rem 0.6rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
    transition: color 0.12s;
  }
  .row-wrap:hover .row { color: var(--text-primary); }
  .row-wrap.active .row { color: var(--text-primary); }
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
  .add-child {
    background: transparent;
    border: 0;
    color: var(--text-ghost);
    font-size: 0.9rem;
    padding: 0 0.5rem;
    opacity: 0;
    cursor: pointer;
    transition: opacity 0.12s, color 0.12s;
  }
  .add-child:hover { color: var(--accent); }
  .inline-form {
    padding: 0.5rem 0.65rem;
  }
  .inline-form.nested { padding-right: 0.5rem; }
  .tree-footer {
    margin-top: auto;
    padding: 0.4rem 0.65rem;
    border-top: 1px solid var(--border-faint);
  }
  .add-btn {
    background: none;
    border: 0;
    color: var(--text-ghost);
    font-size: 0.72rem;
    padding: 0;
    cursor: pointer;
    transition: color 0.12s;
  }
  .add-btn:hover:not(:disabled) { color: var(--accent); }
  .add-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
