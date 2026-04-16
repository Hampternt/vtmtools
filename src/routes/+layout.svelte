<script lang="ts">
  import { onMount } from 'svelte';
  import type { Snippet } from 'svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { tools } from '../tools';
  import type { Component } from 'svelte';

  const { children }: { children?: Snippet } = $props();

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
    {#if children}{@render children()}{/if}
  </main>
</div>

<style>
  :global(html) {
    /* Fluid root font — all rem values scale with this.
       Grows from 16px at ~1600px width up to 32px at 4K.
       Tune the vw coefficient to taste; cap prevents runaway on ultrawide. */
    font-size: clamp(16px, 1.0vw, 32px);
  }

  :global(:root) {
    /* ── Text hierarchy ─────────────────────────────── */
    --text-primary:   #d4c5a9;  /* values, main content */
    --text-label:     #a09070;  /* field labels, section headers */
    --text-secondary: #6a5a3a;  /* hints, timestamps, empty states */
    --text-muted:     #686868;  /* dice info, die tags, sub-labels */
    --text-ghost:     #585050;  /* near-invisible contextual */

    /* ── Surfaces ───────────────────────────────────── */
    --bg-base:   #0d0d0d;  /* global body */
    --bg-card:   #120808;  /* step cards, panels */
    --bg-raised: #160808;  /* buttons, die chips */
    --bg-input:  #1a0d0d;  /* inputs, result card */
    --bg-sunken: #0f0606;  /* history entries */
    --bg-active: #380808;  /* selected / pressed state */

    /* ── Borders ────────────────────────────────────── */
    --border-faint:   #5a3030;  /* separators  — DEBUG: was #1e0e0e */
    --border-card:    #6a3535;  /* card edges  — DEBUG: was #2a1010 */
    --border-surface: #7a4545;  /* raised element edges  — DEBUG: was #3a1a1a */
    --border-active:  #cc2222;  /* focused / active ring */

    /* ── Accent colors ──────────────────────────────── */
    --accent:        #cc2222;
    --accent-bright: #ff4444;
    --accent-amber:  #cc9922;

    /* ── Temperament outcome tints ──────────────────── */
    --temp-negligible:     #909090;  /* result value text */
    --temp-negligible-dim: #686868;  /* zone labels, prob bars */
    --temp-fleeting-dim:   #907220;
    --temp-intense-dim:    #902020;
  }

  @font-face {
    font-family: 'Last Rites';
    src: url('/fonts/LastRites.ttf') format('truetype');
    font-weight: normal;
    font-style: normal;
    font-display: swap;
  }

  :global(body) {
    margin: 0;
    background: var(--bg-base);
    color: var(--text-primary);
    font-family: 'Georgia', serif;
  }
  .shell {
    display: flex;
    min-height: 100vh;
  }
  /* Very narrow windows: stack nav above content */
  @media (max-width: 28rem) {
    .shell { flex-direction: column; }
  }
  .content {
    flex: 1;
    padding: 1.5rem;
    overflow-y: auto;
    min-height: 0; /* prevent flex child from overflowing */
  }
  .loading {
    color: var(--text-muted);
    font-style: italic;
  }
</style>
