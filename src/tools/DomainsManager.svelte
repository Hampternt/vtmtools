<script lang="ts">
  import { onMount } from 'svelte';
  import { session, cache, status, refreshChronicles, setChronicle } from '../store/domains.svelte';

  onMount(async () => {
    await refreshChronicles();
    if (session.chronicleId == null && cache.chronicles.length > 0) {
      await setChronicle(cache.chronicles[0].id);
    }
  });
</script>

<div class="page">
  <h1 class="title">Domains</h1>

  {#if status.loading}
    <p class="loading-text">Loading…</p>
  {:else if status.error}
    <p class="error-text">{status.error}</p>
  {:else if cache.chronicles.length === 0}
    <p class="empty">No chronicles yet. Create one to get started.</p>
  {:else}
    <p class="empty">
      Chronicle selected: {cache.chronicles.find(c => c.id === session.chronicleId)?.name ?? '(none)'}
      — nodes: {cache.nodes.length}, edges: {cache.edges.length}.
    </p>
  {/if}
</div>

<style>
  .page { padding: 1rem 1.25rem; }
  .title { color: var(--accent); font-size: 1.4rem; margin-bottom: 1rem; }
  .loading-text, .empty { color: var(--text-ghost); font-size: 0.8rem; }
  .error-text { color: var(--accent); font-size: 0.8rem; padding: 1rem 0; }
</style>
