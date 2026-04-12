<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { tools } from '../tools';
  import type { Component } from 'svelte';

  let activeTool = $state(tools[0].id);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let ActiveComponent: Component<any> | null = $state(null);

  async function loadTool(id: string) {
    const tool = tools.find(t => t.id === id);
    if (!tool) return;
    activeTool = id;
    const mod = await tool.component();
    ActiveComponent = mod.default;
  }

  onMount(() => loadTool(activeTool));
</script>

<div class="shell">
  <Sidebar {activeTool} onSelect={loadTool} />
  <main class="content">
    {#if ActiveComponent}
      <ActiveComponent />
    {:else}
      <p class="loading">Loading…</p>
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    background: #0d0d0d;
    color: #d4c5a9;
    font-family: 'Georgia', serif;
  }
  .shell {
    display: flex;
    min-height: 100vh;
  }
  .content {
    flex: 1;
    padding: 1.5rem;
    overflow-y: auto;
  }
  .loading {
    color: #555;
    font-style: italic;
  }
</style>
