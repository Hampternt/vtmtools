<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { Roll20Character, Roll20Attribute } from '../types';

  // ── Attribute name constants ────────────────────────────────────────────
  // The Roll20 Jumpgate VTM 5e sheet stores the max value as the `.max`
  // field on the same attribute (e.g. health.max = 5), not as a separate
  // `health_max` attribute. So healthMax and willpowerMax point to the
  // same attribute name as health/willpower — attrMax() reads .max from it.
  const ATTR = {
    hunger:       'hunger',
    health:       'health',            // .max = total health boxes
    healthMax:    'health',            // attrMax reads .max field
    healthSup:    'health_superficial',
    healthAgg:    'health_aggravated',
    willpower:    'willpower',         // .current = remaining WP pool
    willpowerMax: 'willpower',         // attrMax reads .max field
    willpowerSup: 'willpower_superficial',
    willpowerAgg: 'willpower_aggravated',
    humanity:     'humanity',
    bloodPotency: 'blood_potency',
  } as const;

  // ── State ───────────────────────────────────────────────────────────────
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

  // ── Helpers ─────────────────────────────────────────────────────────────

  // Returns the integer value of attribute.current, or 0 if missing/empty.
  // Use for damage-style attrs (health): empty current = 0 damage = healthy.
  function attr(attributes: Roll20Attribute[], name: string): number {
    const a = attributes.find(a => a.name === name);
    return a ? (parseInt(a.current, 10) || 0) : 0;
  }

  // Reads the .max field of the named attribute. Falls back to `fallback`
  // if the attribute is missing or its max is unset/unparseable.
  function attrMax(attributes: Roll20Attribute[], name: string, fallback: number): number {
    const a = attributes.find(a => a.name === name);
    if (a && a.max) return parseInt(a.max, 10) || fallback;
    return fallback;
  }

  // Returns current value; if current is empty/unset, returns `max` instead.
  // Use for pool-style attrs (willpower): empty current = not yet set = full.
  function attrCurrentOrMax(attributes: Roll20Attribute[], name: string, max: number): number {
    const a = attributes.find(a => a.name === name);
    if (!a || a.current === '') return max;
    return parseInt(a.current, 10) || 0;
  }

  // Returns an array of booleans for rendering dot tracks.
  function dots(filled: number, total: number): boolean[] {
    return Array.from({ length: total }, (_, i) => i < filled);
  }

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
        {@const healthMax   = attrMax(char.attributes, ATTR.healthMax, 5)}
        {@const healthSup   = attr(char.attributes, ATTR.healthSup)}
        {@const healthAgg   = attr(char.attributes, ATTR.healthAgg)}
        {@const healthOk    = Math.max(0, healthMax - healthSup - healthAgg)}
        {@const wpMax       = attrMax(char.attributes, ATTR.willpowerMax, 5)}
        {@const wpSup       = attr(char.attributes, ATTR.willpowerSup)}
        {@const wpAgg       = attr(char.attributes, ATTR.willpowerAgg)}
        {@const wpOk        = Math.max(0, wpMax - wpSup - wpAgg)}
        {@const hunger      = attr(char.attributes, ATTR.hunger)}
        {@const humanity    = attr(char.attributes, ATTR.humanity)}
        {@const bp          = attr(char.attributes, ATTR.bloodPotency)}

        <div class="char-card">
          <div class="card-header">
            <span class="char-name">{char.name}</span>
            <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
              {isPC(char) ? 'PC' : 'NPC'}
            </span>
          </div>

          <div class="card-body">
            <!-- Hunger (0–5 dots, crimson) -->
            <div class="stat-row">
              <span class="stat-label">Hunger</span>
              <div class="track">
                {#each dots(hunger, 5) as filled}
                  <div class="dot" class:hunger={filled}></div>
                {/each}
              </div>
            </div>

            <!-- Health: [healthy][superficial][aggravated] left→right -->
            <div class="stat-row">
              <span class="stat-label">Health</span>
              <div class="track">
                {#each Array.from({ length: healthMax }, (_, i) => i) as i}
                  <div
                    class="box"
                    class:filled={i >= healthOk && i < healthOk + healthSup}
                    class:aggravated={i >= healthOk + healthSup}
                  ></div>
                {/each}
              </div>
            </div>

            <!-- Willpower: [remaining][superficial][aggravated] left→right -->
            <div class="stat-row">
              <span class="stat-label">Willpower</span>
              <div class="track">
                {#each Array.from({ length: wpMax }, (_, i) => i) as i}
                  <div
                    class="box willpower"
                    class:filled={i < wpOk}
                    class:superficial={i >= wpOk && i < wpOk + wpSup}
                    class:aggravated={i >= wpOk + wpSup}
                  ></div>
                {/each}
              </div>
            </div>

            <!-- Humanity (0–10 dots, indigo) -->
            <div class="stat-row">
              <span class="stat-label">Humanity</span>
              <div class="track">
                {#each dots(humanity, 10) as filled}
                  <div class="dot" class:humanity={filled}></div>
                {/each}
              </div>
            </div>

            <!-- Blood Potency (single number, amber) -->
            <div class="stat-row">
              <span class="stat-label">Blood Potency</span>
              <span class="bp-value">{bp}</span>
            </div>
          </div>

          <div class="card-footer">
            <button class="raw-toggle" onclick={() => toggleRaw(char.id)}>
              raw attrs {expandedRaw.has(char.id) ? '▴' : '▾'}
            </button>
          </div>

          {#if expandedRaw.has(char.id)}
            <div class="raw-panel">
              {#each char.attributes as a}
                <div class="raw-row">
                  <span class="raw-name">{a.name}</span>
                  <span class="raw-val">{a.current}{a.max ? ' / ' + a.max : ''}</span>
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
    grid-template-columns: repeat(auto-fill, minmax(280px, 340px));
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
    align-items: flex-start;
    gap: 0.5rem;
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .char-name {
    flex: 1;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
    overflow: hidden;
    word-break: break-word;
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

  /* Stat rows */
  .card-body {
    padding: 0.65rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .stat-row {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .stat-label {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-ghost);
    font-weight: 600;
  }
  .track {
    display: flex;
    gap: 3px;
    flex-wrap: wrap;
  }

  /* Dots (Hunger, Humanity) */
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1px solid var(--border-surface);
    background: transparent;
  }
  .dot.hunger {
    background: var(--accent);
    border-color: var(--accent);
    box-shadow: 0 0 4px color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .dot.humanity {
    background: #7986cb;
    border-color: #7986cb;
  }

  /* Boxes (Health, Willpower) */
  .box {
    width: 11px;
    height: 11px;
    border: 1px solid var(--border-surface);
    border-radius: 2px;
    background: transparent;
  }
  .box.filled {
    background: var(--accent);
    border-color: var(--accent);
  }
  .box.willpower.filled {
    background: #7986cb;
    border-color: #7986cb;
  }
  .box.willpower.superficial {
    background: #7986cb40;
    border-color: #7986cb;
  }
  .box.willpower.aggravated {
    background-image: repeating-linear-gradient(
      45deg,
      #7986cb 0,
      #7986cb 1px,
      transparent 0,
      transparent 50%
    );
    background-size: 4px 4px;
  }
  .box.aggravated {
    border-color: var(--border-surface);
    background-image: repeating-linear-gradient(
      45deg,
      var(--accent) 0,
      var(--accent) 1px,
      transparent 0,
      transparent 50%
    );
    background-size: 4px 4px;
  }

  /* Blood Potency */
  .bp-value {
    font-size: 1.05rem;
    font-weight: 700;
    color: var(--accent-amber);
  }

  .card-footer {
    padding: 0.3rem 0.9rem;
    display: flex;
    justify-content: flex-end;
    border-top: 1px solid var(--border-faint);
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
