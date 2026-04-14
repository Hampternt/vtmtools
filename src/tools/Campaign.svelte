<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { Roll20Character } from '../types';

  let connected = $state(false);
  let characters = $state<Roll20Character[]>([]);
  let lastSync = $state<Date | null>(null);
  let expandedRaw = $state<Set<string>>(new Set());

  $effect(() => {
    invoke<boolean>('get_roll20_status').then(s => { connected = s; });
    invoke<Roll20Character[]>('get_roll20_characters').then(c => { characters = c; });

    const unlisteners = [
      listen<void>('roll20://connected', () => { connected = true; }),
      listen<void>('roll20://disconnected', () => { connected = false; }),
      listen<Roll20Character[]>('roll20://characters-updated', (e) => {
        characters = e.payload;
        lastSync = new Date();
      }),
    ];

    return () => { unlisteners.forEach(p => p.then(u => u())); };
  });

  function toggleRaw(id: string) {
    const next = new Set(expandedRaw);
    if (next.has(id)) { next.delete(id); } else { next.add(id); }
    expandedRaw = next;
  }

  function isPC(char: Roll20Character): boolean {
    return char.controlled_by.trim() !== '';
  }

  function timeSince(d: Date): string {
    const s = Math.floor((Date.now() - d.getTime()) / 1000);
    return s < 60 ? `${s}s ago` : `${Math.floor(s / 60)}m ago`;
  }

  function refresh() {
    invoke('refresh_roll20_data');
  }
</script>

<div class="campaign">
  <!-- Toolbar -->
  <div class="toolbar">
    <div class="status">
      <div class="status-dot" class:connected></div>
      {connected ? 'Connected to Roll20' : 'Not connected'}
    </div>
    {#if connected && lastSync}
      <span class="sync-time">last sync {timeSince(lastSync)}</span>
    {/if}
    <div class="spacer"></div>
    <button class="btn-refresh" onclick={refresh} disabled={!connected}>↺ Refresh</button>
  </div>

  {#if !connected}
    <!-- Disconnected banner -->
    <div class="disconnected-banner">
      <p class="banner-title">No Roll20 session detected</p>
      <p class="banner-body">
        Open your Roll20 game in Chrome with the vtmtools extension enabled.
        This panel connects automatically.
      </p>
    </div>
  {:else if characters.length === 0}
    <div class="disconnected-banner">
      <p class="banner-title">Connected — waiting for characters</p>
      <p class="banner-body">The extension is connected but no character data has arrived yet. Try clicking Refresh.</p>
    </div>
  {:else}
    <!-- Character grid -->
    <div class="char-grid">
      {#each characters as char (char.id)}
        <div class="char-card">
          <div class="card-header">
            <span class="char-name">{char.name}</span>
            <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
              {isPC(char) ? 'PC' : 'NPC'}
            </span>
          </div>

          <div class="card-footer">
            <button class="raw-toggle" onclick={() => toggleRaw(char.id)}>
              raw attrs {expandedRaw.has(char.id) ? '▴' : '▾'}
            </button>
          </div>

          {#if expandedRaw.has(char.id)}
            <div class="raw-panel">
              {#each char.attributes as attr}
                <div class="raw-row">
                  <span class="raw-name">{attr.name}</span>
                  <span class="raw-val">{attr.current}{attr.max ? ' / ' + attr.max : ''}</span>
                </div>
              {/each}
              {#if char.attributes.length === 0}
                <span class="raw-empty">No attributes loaded</span>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
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

  /* Toolbar */
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
    font-size: 0.82rem;
    color: var(--text-secondary);
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-ghost);
    flex-shrink: 0;
  }
  .status-dot.connected {
    background: #4caf50;
    box-shadow: 0 0 5px #4caf5066;
  }
  .sync-time {
    font-size: 0.72rem;
    color: var(--text-ghost);
  }
  .spacer { flex: 1; }
  .btn-refresh {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    color: var(--text-secondary);
    padding: 0.3rem 0.8rem;
    border-radius: 5px;
    font-size: 0.78rem;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .btn-refresh:hover:not(:disabled) { border-color: var(--accent); color: var(--text-primary); }
  .btn-refresh:disabled { opacity: 0.4; cursor: default; }

  /* Disconnected banner */
  .disconnected-banner {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 3rem 1rem;
    color: var(--text-ghost);
    text-align: center;
  }
  .banner-title { font-size: 0.9rem; color: var(--text-muted); }
  .banner-body { font-size: 0.78rem; line-height: 1.6; max-width: 280px; }

  /* Character grid */
  .char-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
    align-items: start;
  }

  /* Character card */
  .char-card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 7px;
    overflow: hidden;
  }
  .char-card:hover { border-color: var(--border-surface); }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .char-name {
    flex: 1;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .badge.pc { background: #2a1515; color: var(--accent); border: 1px solid #3a1e1e; }
  .badge.npc { background: #151528; color: #7986cb; border: 1px solid #1e1e3a; }

  .card-footer {
    padding: 0.3rem 0.9rem;
    display: flex;
    justify-content: flex-end;
  }
  .raw-toggle {
    font-size: 0.65rem;
    color: var(--text-ghost);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.1rem 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .raw-toggle:hover { color: var(--text-muted); }

  /* Raw attribute dump */
  .raw-panel {
    border-top: 1px solid var(--border-faint);
    padding: 0.5rem 0.9rem;
    max-height: 180px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .raw-row {
    display: flex;
    gap: 0.5rem;
    font-size: 0.7rem;
    font-family: monospace;
    line-height: 1.7;
  }
  .raw-name { color: var(--text-muted); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .raw-val { color: var(--accent); flex-shrink: 0; }
  .raw-empty { font-size: 0.7rem; color: var(--text-ghost); font-style: italic; }
</style>
