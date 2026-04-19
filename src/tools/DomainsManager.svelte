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
      <div class="pane pane-tree"><DomainTree /></div>
      <div class="pane pane-detail"><NodeDetail /></div>
      <div class="pane pane-edges"><EdgesPanel /></div>
    </div>
  {/if}
</div>

<style>
  /* `.tool` fills the flex-column `.content` in +layout.svelte instead of
     using 100vh (which overflowed by the content's padding). The container
     query is driven by this element's inline (width) size, so breakpoints
     respond to the actual available width — independent of the sidebar. */
  .tool {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    min-width: 0;
    width: 100%;
    container-type: inline-size;
    container-name: domains-tool;
  }
  /* Default (wide): 3 columns. `minmax` lets sidebars shrink gracefully
     before the layout swaps to a stacked variant at the breakpoints below. */
  .grid {
    display: grid;
    grid-template-columns: minmax(14rem, 18rem) minmax(18rem, 1fr) minmax(13rem, 17rem);
    flex: 1;
    min-height: 0;
    min-width: 0;
  }
  /* Each pane wraps one child component. Flex column + overflow hidden
     gives the inner component a finite height to resolve `flex: 1`
     against, while `min-*: 0` lets it shrink below content size. */
  .pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  .pane > :global(*) {
    flex: 1;
    min-height: 0;
    min-width: 0;
  }

  /* Medium (portrait/smaller windows): 2 columns, relationships panel
     drops to a bottom strip spanning both columns. */
  @container domains-tool (max-width: 58rem) {
    .grid {
      grid-template-columns: minmax(14rem, 18rem) 1fr;
      grid-template-rows: 1fr minmax(6rem, 12rem);
    }
    .grid > .pane-edges {
      grid-column: 1 / -1;
      grid-row: 2;
      border-top: 1px solid var(--border-surface);
    }
  }

  /* Narrow (phone-width / vertical split): single column, all three panes
     stacked. Tree and relationships get height caps so the detail pane
     keeps breathing room. */
  @container domains-tool (max-width: 36rem) {
    .grid {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(6rem, 11rem) 1fr minmax(5rem, 9rem);
    }
    .grid > .pane-tree {
      grid-column: 1;
      grid-row: 1;
      border-right: 0;
      border-bottom: 1px solid var(--border-surface);
    }
    .grid > .pane-detail { grid-column: 1; grid-row: 2; }
    .grid > .pane-edges { grid-column: 1; grid-row: 3; }
  }

  .muted { color: var(--text-ghost); font-size: 0.82rem; }
</style>
