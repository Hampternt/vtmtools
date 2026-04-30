<!--
  Small "source: [origin]" badge used on character cards and similar.
  Reads worldTitle from the bridge store's sourceInfo for live characters,
  or accepts an explicit `worldTitle` prop for saved characters where the
  world is captured-at-save-time.
-->
<script lang="ts">
  import { bridge } from '../../store/bridge.svelte';
  import type { SourceKind } from '$lib/bridge/api';

  let {
    source,
    worldTitle = null,
  }: {
    source: SourceKind;
    worldTitle?: string | null;
  } = $props();

  // For live characters, prefer the live world title from the bridge store.
  // For saved characters, prefer the captured worldTitle prop.
  const displayWorld = $derived(
    worldTitle ?? bridge.sourceInfo[source]?.worldTitle ?? null
  );
</script>

<span class="chip">
  source:
  {#if source === 'foundry'}
    FVTT{#if displayWorld} — {displayWorld}{/if}
  {:else if source === 'roll20'}
    Roll20
  {/if}
</span>

<style>
  .chip {
    font-size: 0.7em;
    color: var(--text-ghost);
    opacity: 0.85;
  }
</style>
