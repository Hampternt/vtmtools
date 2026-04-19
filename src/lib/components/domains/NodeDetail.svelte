<script lang="ts">
  import { session, cache } from '../../../store/domains.svelte';

  const node = $derived(cache.nodes.find(n => n.id === session.nodeId) ?? null);
</script>

<section class="detail">
  {#if !node}
    <p class="muted">No node selected. Click one in the tree to view its details.</p>
  {:else}
    <header class="head">
      <span class="title">{node.label}</span>
      <span class="type-chip">{node.type}</span>
      <span class="spacer"></span>
      <button class="btn" disabled title="Edit mode lands in Task 9">✎ Edit</button>
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

    <!-- Properties panel lands in Task 8. -->
  {/if}
</section>

<style>
  .detail { padding: 0.9rem 1rem; display: flex; flex-direction: column; gap: 0.55rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.8rem; }
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
    color: var(--text-muted);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
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
</style>
