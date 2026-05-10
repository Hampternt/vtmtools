<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge, anyConnected } from '../store/bridge.svelte';
  import { savedCharacters } from '../store/savedCharacters.svelte';
  import { refresh as bridgeRefresh } from '$lib/bridge/api';
  import CompareModal from '$lib/components/CompareModal.svelte';
  import CharacterCardShell from '$lib/components/CharacterCardShell.svelte';
  import { diffCharacter } from '$lib/saved-characters/diff';
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import type { BridgeCharacter } from '../types';

  // ── State ───────────────────────────────────────────────────────────────
  const connected  = $derived(anyConnected());
  const characters = $derived(bridge.characters);
  const lastSync   = $derived(bridge.lastSync);

  // Live characters paired with their saved-counterpart (if any). The
  // findMatch helper handles snake_case (live) vs camelCase (saved)
  // mapping internally, so we just pass the BridgeCharacter through.
  const liveWithMatches = $derived(
    characters.map(live => ({
      live,
      saved: savedCharacters.findMatch(live),
    })),
  );

  // Drift detection: for every live character with a saved match, compute
  // whether diffCharacter() reports any non-empty entries. Keyed by
  // `${source}:${source_id}` (BridgeCharacter is snake_case).
  const drifts = $derived(new Map<string, boolean>(
    liveWithMatches
      .filter(({ saved }) => saved !== undefined)
      .map(({ live, saved }) => [
        `${live.source}:${live.source_id}`,
        diffCharacter(saved!.canonical, live).length > 0,
      ]),
  ));

  function hasDrift(live: BridgeCharacter): boolean {
    return drifts.get(`${live.source}:${live.source_id}`) ?? false;
  }

  // Saved-only characters: those without a matching live counterpart.
  const savedOnly = $derived(
    savedCharacters.list.filter(
      s => !characters.some(c => c.source === s.source && c.source_id === s.sourceId),
    ),
  );

  // Compare-modal state.
  let comparing = $state<{ saved: SavedCharacter; live: BridgeCharacter } | null>(null);
  function openCompare(saved: SavedCharacter, live: BridgeCharacter) {
    comparing = { saved, live };
  }
  function closeCompare() { comparing = null; }

  onMount(() => { void savedCharacters.ensureLoaded(); });

  let urlCopied = $state(false);

  type Density = 'auto' | 's' | 'm' | 'l';
  let density = $state<Density>('auto');
  let resolvedDensity = $state<'s' | 'm' | 'l'>('m');
  let gridEl: HTMLDivElement | undefined = $state(undefined);

  function copyExtensionsUrl() {
    navigator.clipboard.writeText('chrome://extensions');
    urlCopied = true;
    setTimeout(() => { urlCopied = false; }, 1500);
  }

  $effect(() => {
    if (density !== 'auto') {
      resolvedDensity = density;
      return;
    }
    if (!gridEl) return;

    const ro = new ResizeObserver((entries) => {
      const w = entries[0]?.contentRect.width ?? 0;
      if (w < 500) resolvedDensity = 's';
      else if (w < 800) resolvedDensity = 'm';
      else resolvedDensity = 'l';
    });
    ro.observe(gridEl);
    return () => ro.disconnect();
  });

  const densityVars = $derived.by(() => {
    const d = resolvedDensity;
    const vals = {
      s: { minCol: '16rem', pad: '0.4rem', trackH: '1.4rem', conscienceCap: '1.5rem', dropSize: '1.2rem', conscienceGlow: 'none', cardScale: '0.7' },
      m: { minCol: '20rem', pad: '0.6rem', trackH: '1.8rem', conscienceCap: '2.5rem', dropSize: '1.6rem', conscienceGlow: '0 0 0.3rem color-mix(in srgb, var(--accent) 30%, transparent)', cardScale: '1.0' },
      l: { minCol: '28rem', pad: '0.8rem', trackH: '2.4rem', conscienceCap: '4rem', dropSize: '2rem', conscienceGlow: '0 0 0.5rem color-mix(in srgb, var(--accent) 50%, transparent)', cardScale: '1.4' },
    }[d];
    return `--col-min:${vals.minCol};--card-pad:${vals.pad};--track-h:${vals.trackH};--conscience-cap:${vals.conscienceCap};--drop-size:${vals.dropSize};--conscience-glow:${vals.conscienceGlow};--card-scale:${vals.cardScale}`;
  });

  function timeSince(d: Date): string {
    const s = Math.floor((Date.now() - d.getTime()) / 1000);
    return s < 60 ? `${s}s ago` : `${Math.floor(s / 60)}m ago`;
  }

  function refresh() {
    bridgeRefresh();
  }
</script>

<div class="campaign">
  <!-- Toolbar -->
  <div class="toolbar">
    <div class="status">
      <span class="source-pip" class:connected={bridge.connections.roll20}></span>
      <span class="source-label">R20</span>
      <span class="source-pip" class:connected={bridge.connections.foundry}></span>
      <span class="source-label">Foundry</span>
    </div>
    {#if connected && lastSync}
      <span class="sync-time">last sync {timeSince(lastSync)}</span>
    {/if}
    <div class="spacer"></div>
    <div class="density-toggle">
      {#each [['auto', 'Auto'], ['s', 'S'], ['m', 'M'], ['l', 'L']] as [val, label]}
        <button
          class="density-btn"
          class:active={density === val}
          onclick={() => { density = val as Density; }}
        >{label}</button>
      {/each}
    </div>
    <button class="btn-refresh" onclick={refresh} disabled={!connected}>↺ Refresh</button>
  </div>

  {#if !connected && characters.length === 0 && savedOnly.length === 0}
    <div class="setup-guide">
      <p class="guide-title">No bridge connected</p>
      <p class="guide-sub">Choose your VTT and follow the one-time setup. Once installed, both bridges run side by side — connect to either or both.</p>

      <details class="bridge-section" open>
        <summary><strong>Roll20</strong> — install browser extension</summary>
        <ol class="steps">
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Download the extension</span>
              <span class="step-text">Go to the vtmtools <strong>Releases</strong> page on GitHub and download <code>vtmtools-extension.zip</code>.</span>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Extract the zip</span>
              <span class="step-text">Right-click the downloaded file and choose <em>Extract All</em> (Windows) or double-click it (Mac/Linux). Remember where you put the folder.</span>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Open Chrome extensions</span>
              <span class="step-text">In Chrome, click the address bar, type this, and press Enter:</span>
              <div class="url-row">
                <code class="url-block">chrome://extensions</code>
                <button class="btn-copy" onclick={copyExtensionsUrl}>
                  {urlCopied ? 'copied!' : 'copy'}
                </button>
              </div>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Turn on Developer Mode</span>
              <span class="step-text">In the top-right corner of that page, flip the <em>Developer mode</em> toggle on. A new row of buttons will appear.</span>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Load the extension</span>
              <span class="step-text">Click <em>Load unpacked</em>, then select the folder you extracted in step 2.</span>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Open your Roll20 game</span>
              <span class="step-text">Navigate to your game on Roll20. Once the editor loads, this panel will connect on its own.</span>
            </div>
          </li>
        </ol>
      </details>

      <details class="bridge-section">
        <summary><strong>FoundryVTT</strong> — install module + accept cert</summary>
        <ol class="steps">
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Install the module</span>
              <span class="step-text">In Foundry's setup, <em>Add-on Modules → Install Module</em>, paste this manifest URL:</span>
              <div class="url-row">
                <code class="url-block">https://github.com/Hampternt/vtmtools/releases/latest/download/module.json</code>
              </div>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Accept the cert (one time)</span>
              <span class="step-text">In the same browser you use for Foundry, visit <code>https://localhost:7424</code>, click <em>Advanced → Proceed</em>. The browser remembers it.</span>
            </div>
          </li>
          <li class="step">
            <div class="step-body">
              <span class="step-heading">Enable in your world</span>
              <span class="step-text">Manage Modules → enable <em>vtmtools Desktop Bridge</em>. Reload the world. Only your GM browser opens the connection.</span>
            </div>
          </li>
        </ol>
      </details>

      <p class="guide-note">You only need to do this once per VTT.</p>
    </div>
  {:else if connected && characters.length === 0 && savedOnly.length === 0}
    <div class="disconnected-banner">
      <p class="banner-title">Connected — waiting for characters</p>
      <p class="banner-body">The extension is connected but no character data has arrived yet. Try clicking Refresh.</p>
    </div>
  {:else}
    <div class="char-grid" bind:this={gridEl} style={densityVars}>
      {#each liveWithMatches as { live, saved } (live.source + ':' + live.source_id)}
        <CharacterCardShell
          character={live}
          saved={saved ?? null}
          drift={hasDrift(live)}
          onCompare={openCompare}
        />
      {/each}
      {#each savedOnly as savedRow (savedRow.id)}
        <CharacterCardShell
          character={savedRow.canonical}
          saved={savedRow}
          drift={false}
          onCompare={openCompare}
        />
      {/each}
    </div>
  {/if}

  {#if comparing}
    <CompareModal
      saved={comparing.saved}
      live={comparing.live}
      onClose={closeCompare}
    />
  {/if}
</div>

<style>
  .campaign {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.25rem;
    height: 100%;
    box-sizing: border-box;
  }

  /* ── Toolbar ──────────────────────────────────────────────────────────── */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
  .source-pip {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-ghost);
    flex-shrink: 0;
  }
  .source-pip.connected {
    background: #4caf50;
    box-shadow: 0 0 5px #4caf5066;
  }
  .source-label {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-right: 0.4rem;
  }
  .bridge-section {
    margin: 0.75rem 0;
    border: 1px solid var(--border-faint);
    border-radius: 5px;
    padding: 0.65rem 0.85rem;
  }
  .bridge-section summary {
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--text-label);
    margin-bottom: 0.5rem;
    user-select: none;
  }
  .bridge-section summary strong {
    color: var(--accent);
  }
  .sync-time { font-size: 0.75rem; color: var(--text-ghost); }
  .spacer { flex: 1; }
  .btn-refresh {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    color: var(--text-secondary);
    padding: 0.3rem 0.8rem;
    border-radius: 5px;
    font-size: 0.8rem;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .btn-refresh:hover:not(:disabled) { border-color: var(--accent); color: var(--text-primary); }
  .btn-refresh:disabled { opacity: 0.4; cursor: default; }

  /* ── Density toggle ──────────────────────────────────────────────────── */
  .density-toggle {
    display: inline-flex;
    border: 1px solid var(--border-faint);
    border-radius: 5px;
    overflow: hidden;
  }
  .density-btn {
    background: var(--bg-card);
    color: var(--text-ghost);
    border: none;
    border-right: 1px solid var(--border-faint);
    padding: 0.2rem 0.55rem;
    font-size: 0.7rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }
  .density-btn:last-child { border-right: none; }
  .density-btn:hover { color: var(--text-secondary); }
  .density-btn.active {
    background: var(--bg-active);
    color: var(--accent);
  }

  /* ── Setup guide (shown when disconnected) ────────────────────────────── */
  .setup-guide {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    max-width: 480px;
    padding: 1.5rem;
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
  }
  .guide-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }
  .guide-sub {
    font-size: 0.82rem;
    color: var(--text-muted);
    line-height: 1.5;
    margin: 0;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0;
    counter-reset: step-counter;
    border: 1px solid var(--border-faint);
    border-radius: 6px;
    overflow: hidden;
  }
  .step {
    counter-increment: step-counter;
    display: flex;
    gap: 0.85rem;
    padding: 0.75rem 0.9rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .step:last-child { border-bottom: none; }
  .step::before {
    content: counter(step-counter);
    flex-shrink: 0;
    width: 1.4rem;
    height: 1.4rem;
    border-radius: 50%;
    background: var(--bg-sunken);
    border: 1px solid var(--border-surface);
    font-size: 0.68rem;
    font-weight: 700;
    color: var(--text-ghost);
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 0.05rem;
  }
  .step-body {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    min-width: 0;
  }
  .step-heading {
    font-size: 0.83rem;
    font-weight: 600;
    color: var(--text-secondary);
  }
  .step-text {
    font-size: 0.8rem;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .step-text code {
    font-family: monospace;
    font-size: 0.78rem;
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.05rem 0.3rem;
    color: var(--text-secondary);
  }
  .step-text strong { color: var(--text-secondary); font-weight: 600; }
  .step-text em     { color: var(--accent-amber); font-style: normal; }

  .url-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.2rem;
  }
  .url-block {
    font-family: monospace;
    font-size: 0.8rem;
    background: var(--bg-sunken);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    color: var(--accent);
    letter-spacing: 0.01em;
  }
  .btn-copy {
    font-size: 0.7rem;
    color: var(--text-ghost);
    background: none;
    border: 1px solid var(--border-faint);
    border-radius: 4px;
    padding: 0.15rem 0.45rem;
    cursor: pointer;
    transition: color 0.1s, border-color 0.1s;
    flex-shrink: 0;
  }
  .btn-copy:hover { color: var(--text-muted); border-color: var(--border-surface); }

  .guide-note {
    font-size: 0.75rem;
    color: var(--text-ghost);
    line-height: 1.5;
    margin: 0;
  }

  /* ── Character grid ───────────────────────────────────────────────────── */
  .char-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--col-min, 20rem), 1fr));
    max-width: 1100px;
    gap: 0.75rem;
    align-items: start;
  }
</style>
