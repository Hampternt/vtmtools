<script lang="ts">
  import { onMount } from 'svelte';
  import ChronicleHeader from '$lib/components/domains/ChronicleHeader.svelte';
  import DomainTree from '$lib/components/domains/DomainTree.svelte';
  import NodeDetail from '$lib/components/domains/NodeDetail.svelte';
  import EdgesPanel from '$lib/components/domains/EdgesPanel.svelte';
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
      <NodeDetail />
      <EdgesPanel />
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
  .muted { color: var(--text-ghost); font-size: 0.82rem; }
</style>
