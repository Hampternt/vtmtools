<script lang="ts">
  import { onMount } from 'svelte';
  import ChronicleHeader from '$lib/components/domains/ChronicleHeader.svelte';
  import DomainTree from '$lib/components/domains/DomainTree.svelte';
  import { session, cache, status, refreshChronicles, setChronicle } from '../store/domains.svelte';

  onMount(async () => {
    await refreshChronicles();
    if (session.chronicleId == null && cache.chronicles.length > 0) {
      await setChronicle(cache.chronicles[0].id);
    }
  });
</script>

<div class="tool">
  <ChronicleHeader />

  {#if status.loading}
    <p class="muted">Loading…</p>
  {:else if cache.chronicles.length === 0}
    <p class="muted">No chronicles yet. Click "+ New" above to create one.</p>
  {:else if session.chronicleId == null}
    <p class="muted">Select a chronicle to start.</p>
  {:else}
    <div class="grid">
      <DomainTree />
      <main class="detail-placeholder">
        {#if session.nodeId == null}
          <p class="muted">Select a node from the tree to view its details.</p>
        {:else}
          <p class="muted">
            Selected node id: {session.nodeId}
            — (detail pane not implemented yet; see Task 6)
          </p>
        {/if}
      </main>
      <aside class="edges-placeholder">
        <p class="muted">Relationships panel (Task 11).</p>
      </aside>
    </div>
  {/if}
</div>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100%;
  }
  .grid {
    display: grid;
    grid-template-columns: 18rem 1fr 17rem;
    flex: 1;
    min-height: 0;
  }
  .detail-placeholder, .edges-placeholder { padding: 1rem; overflow: auto; }
  .edges-placeholder { border-left: 1px solid var(--border-surface); background: var(--bg-sunken); }
  .muted { color: var(--text-ghost); font-size: 0.82rem; }
</style>
