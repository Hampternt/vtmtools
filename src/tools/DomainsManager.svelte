<script lang="ts">
  import { onMount } from 'svelte';
  import ChronicleHeader from '$lib/components/domains/ChronicleHeader.svelte';
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

  <div class="body">
    {#if status.loading}
      <p class="muted">Loading…</p>
    {:else if cache.chronicles.length === 0}
      <p class="muted">No chronicles yet. Click "+ New" above to create one.</p>
    {:else if session.chronicleId == null}
      <p class="muted">Select a chronicle to start.</p>
    {:else}
      <p class="muted">
        Chronicle loaded: {cache.nodes.length} nodes, {cache.edges.length} edges.
      </p>
    {/if}
  </div>
</div>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100%;
  }
  .body { flex: 1; padding: 1rem 1.25rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.82rem; }
</style>
