<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { Roll20Character, Roll20Attribute } from '../types';

  // ── Attribute name constants ────────────────────────────────────────────
  const ATTR = {
    hunger:       'hunger',
    health:       'health',
    healthMax:    'health',
    healthSup:    'health_superficial',
    healthAgg:    'health_aggravated',
    willpower:    'willpower',
    willpowerMax: 'willpower',
    willpowerSup: 'willpower_superficial',
    willpowerAgg: 'willpower_aggravated',
    humanity:     'humanity',
    bloodPotency: 'blood_potency',
  } as const;

  // ── State ───────────────────────────────────────────────────────────────
  let connected    = $state(false);
  let characters   = $state<Roll20Character[]>([]);
  let lastSync     = $state<Date | null>(null);
  let expandedRaw   = $state<Set<string>>(new Set());
  let expandedAttrs = $state<Set<string>>(new Set());
  let expandedInfo  = $state<Set<string>>(new Set());
  let urlCopied     = $state(false);

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

  const densityVars = $derived(() => {
    const d = resolvedDensity;
    const vals = {
      s: { minCol: '16rem', pad: '0.4rem', trackH: '1.4rem', conscienceCap: '1.5rem', dropSize: '1.2rem', conscienceGlow: 'none' },
      m: { minCol: '20rem', pad: '0.6rem', trackH: '1.8rem', conscienceCap: '2.5rem', dropSize: '1.6rem', conscienceGlow: '0 0 0.3rem color-mix(in srgb, var(--accent) 30%, transparent)' },
      l: { minCol: '28rem', pad: '0.8rem', trackH: '2.4rem', conscienceCap: '4rem', dropSize: '2rem', conscienceGlow: '0 0 0.5rem color-mix(in srgb, var(--accent) 50%, transparent)' },
    }[d];
    return `--col-min:${vals.minCol};--card-pad:${vals.pad};--track-h:${vals.trackH};--conscience-cap:${vals.conscienceCap};--drop-size:${vals.dropSize};--conscience-glow:${vals.conscienceGlow}`;
  });

  // ── Helpers ─────────────────────────────────────────────────────────────

  function attr(attributes: Roll20Attribute[], name: string): number {
    const a = attributes.find(a => a.name === name);
    return a ? (parseInt(a.current, 10) || 0) : 0;
  }

  function attrMax(attributes: Roll20Attribute[], name: string, fallback: number): number {
    const a = attributes.find(a => a.name === name);
    if (a && a.max) return parseInt(a.max, 10) || fallback;
    return fallback;
  }

  function attrText(attributes: Roll20Attribute[], name: string): string {
    return attributes.find(a => a.name === name)?.current ?? '';
  }

  function parseDisciplines(attributes: Roll20Attribute[]): { type: string; level: number }[] {
    const prefix = 'repeating_disciplines_';
    const suffix = '_discipline';
    return attributes
      .filter(a =>
        a.name.startsWith(prefix) &&
        a.name.endsWith(suffix) &&
        !a.name.includes('_power_'),
      )
      .map(a => {
        const id = a.name.slice(prefix.length, -suffix.length);
        const nameAttr = attributes.find(x => x.name === `${prefix}${id}_discipline_name`);
        return {
          type: nameAttr?.current ?? '',
          level: parseInt(a.current, 10) || 0,
        };
      })
      .filter(d => d.type && d.level > 0);
  }

  function dots(filled: number, total: number): boolean[] {
    return Array.from({ length: total }, (_, i) => i < filled);
  }

  function toggleSet(s: Set<string>, id: string): Set<string> {
    const next = new Set(s);
    if (next.has(id)) { next.delete(id); } else { next.add(id); }
    return next;
  }

  function toggleRaw(id: string)   { expandedRaw   = toggleSet(expandedRaw,   id); }
  function toggleAttrs(id: string) { expandedAttrs = toggleSet(expandedAttrs, id); }
  function toggleInfo(id: string)  { expandedInfo  = toggleSet(expandedInfo,  id); }

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

  {#if !connected}
    <div class="setup-guide">
      <p class="guide-title">Browser extension not connected</p>
      <p class="guide-sub">Install the Roll20 bridge extension once, then open your game and this panel connects automatically.</p>

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

      <p class="guide-note">You only need to do this once. After that, just open Roll20 and vtmtools connects automatically.</p>
    </div>
  {:else if characters.length === 0}
    <div class="disconnected-banner">
      <p class="banner-title">Connected — waiting for characters</p>
      <p class="banner-body">The extension is connected but no character data has arrived yet. Try clicking Refresh.</p>
    </div>
  {:else}
    <div class="char-grid">
      {#each characters as char (char.id)}
        {@const healthMax    = attrMax(char.attributes, ATTR.healthMax, 5)}
        {@const healthSup    = attr(char.attributes, ATTR.healthSup)}
        {@const healthAgg    = attr(char.attributes, ATTR.healthAgg)}
        {@const healthOk     = Math.max(0, healthMax - healthSup - healthAgg)}
        {@const wpMax        = attrMax(char.attributes, ATTR.willpowerMax, 5)}
        {@const wpSup        = attr(char.attributes, ATTR.willpowerSup)}
        {@const wpAgg        = attr(char.attributes, ATTR.willpowerAgg)}
        {@const wpOk         = Math.max(0, wpMax - wpSup - wpAgg)}
        {@const hunger       = attr(char.attributes, ATTR.hunger)}
        {@const humanity     = attr(char.attributes, ATTR.humanity)}
        {@const bp           = attr(char.attributes, ATTR.bloodPotency)}
        {@const stains       = attr(char.attributes, 'humanity_stains')}
        {@const clan         = attrText(char.attributes, 'clan')}
        {@const disciplines  = parseDisciplines(char.attributes)}
        {@const strAttr      = attr(char.attributes, 'strength')}
        {@const dexAttr      = attr(char.attributes, 'dexterity')}
        {@const staAttr      = attr(char.attributes, 'stamina')}
        {@const chaAttr      = attr(char.attributes, 'charisma')}
        {@const manAttr      = attr(char.attributes, 'manipulation')}
        {@const comAttr      = attr(char.attributes, 'composure')}
        {@const intAttr      = attr(char.attributes, 'intelligence')}
        {@const witAttr      = attr(char.attributes, 'wits')}
        {@const resAttr      = attr(char.attributes, 'resolve')}
        {@const bane         = attrText(char.attributes, 'bane')}
        {@const baneSeverity = attrText(char.attributes, 'blood_bane_severity')}
        {@const ambition     = attrText(char.attributes, 'ambition')}
        {@const desire       = attrText(char.attributes, 'desire')}
        {@const predator     = attrText(char.attributes, 'predator')}
        {@const xpEarned     = attr(char.attributes, 'experience')}
        {@const xpSpent      = attr(char.attributes, 'experience_spent')}
        {@const sire         = attrText(char.attributes, 'sire')}
        {@const ageTrue      = attr(char.attributes, 'age_true')}
        {@const ageApparent  = attr(char.attributes, 'age_apparent')}
        {@const tenets       = attrText(char.attributes, 'tenets')}
        {@const notes        = attrText(char.attributes, 'notes')}
        {@const compulsions  = attrText(char.attributes, 'compulsions')}

        <div class="char-card">

          <!-- ── Header ──────────────────────────────────────────────────── -->
          <div class="card-header">
            <div class="name-clan">
              <span class="char-name">{char.name}</span>
              {#if clan}<span class="char-clan">{clan}</span>{/if}
            </div>
            <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
              {isPC(char) ? 'PC' : 'NPC'}
            </span>
            <div class="header-stats">
              <div class="hunger-drops">
                {#each dots(hunger, 5) as filled}
                  <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
                    <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
                  </svg>
                {/each}
              </div>
              <div class="bp-pill">
                <span class="qs-label">BP</span>
                <span class="bp-value">{bp}</span>
              </div>
            </div>
          </div>

          <!-- ── Conscience ──────────────────────────────────────────────── -->
          <div class="conscience-row">
            <div class="conscience-track">
              {#each 'CONSCIENCE'.split('') as letter, i}
                {@const pos = i + 1}
                {@const isFilled = pos <= humanity}
                {@const isStained = pos > 10 - stains}
                <span
                  class="conscience-letter"
                  class:filled={isFilled}
                  class:stained={isStained && !isFilled}
                >{letter}</span>
              {/each}
            </div>
          </div>

          <!-- ── Health track ────────────────────────────────────────────── -->
          <div class="track-row">
            <div class="track-boxes">
              {#each Array.from({ length: healthMax }, (_, i) => i) as i}
                <div
                  class="box"
                  class:superficial={i >= healthOk && i < healthOk + healthSup}
                  class:aggravated={i >= healthOk + healthSup}
                ></div>
              {/each}
            </div>
          </div>

          <!-- ── Willpower track ─────────────────────────────────────────── -->
          <div class="track-row">
            <div class="track-boxes">
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

          <!-- ── Disciplines ──────────────────────────────────────────────── -->
          {#if disciplines.length > 0}
            <div class="disc-section">
              <span class="stat-label">Disciplines</span>
              <div class="disc-chips">
                {#each disciplines as d}
                  <span class="disc-chip">
                    {d.type}<span class="disc-dots">{'•'.repeat(Math.min(d.level, 5))}</span>
                  </span>
                {/each}
              </div>
            </div>
          {/if}

          <!-- ── Collapsible: core attributes + bane ─────────────────────── -->
          {#if expandedAttrs.has(char.id)}
            <div class="card-section">
              <div class="attr-grid">
                <div class="attr-cell"><span class="attr-name">Str</span><span class="attr-val">{strAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Dex</span><span class="attr-val">{dexAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Sta</span><span class="attr-val">{staAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Cha</span><span class="attr-val">{chaAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Man</span><span class="attr-val">{manAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Com</span><span class="attr-val">{comAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Int</span><span class="attr-val">{intAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Wit</span><span class="attr-val">{witAttr}</span></div>
                <div class="attr-cell"><span class="attr-name">Res</span><span class="attr-val">{resAttr}</span></div>
              </div>
              {#if bane || baneSeverity}
                <div class="bane-row">
                  {#if baneSeverity}<span class="bane-severity">{baneSeverity}</span>{/if}
                  {#if bane}<span class="bane-text">{bane}</span>{/if}
                </div>
              {/if}
            </div>
          {/if}

          <!-- ── Collapsible: narrative info ─────────────────────────────── -->
          {#if expandedInfo.has(char.id)}
            <div class="card-section">
              {#if ambition}
                <div class="info-row">
                  <span class="stat-label">Ambition</span>
                  <span class="info-text">{ambition}</span>
                </div>
              {/if}
              {#if desire}
                <div class="info-row">
                  <span class="stat-label">Desire</span>
                  <span class="info-text">{desire}</span>
                </div>
              {/if}
              {#if predator}
                <div class="info-row">
                  <span class="stat-label">Predator Type</span>
                  <span class="info-text">{predator}</span>
                </div>
              {/if}
              {#if sire}
                <div class="info-row">
                  <span class="stat-label">Sire</span>
                  <span class="info-text">{sire}</span>
                </div>
              {/if}
              {#if ageTrue || ageApparent}
                <div class="info-row">
                  <span class="stat-label">Age</span>
                  <span class="info-text">
                    {#if ageTrue}{ageTrue} true{/if}{#if ageTrue && ageApparent} / {/if}{#if ageApparent}{ageApparent} apparent{/if}
                  </span>
                </div>
              {/if}
              {#if xpEarned || xpSpent}
                <div class="info-row">
                  <span class="stat-label">Experience</span>
                  <span class="info-text">{xpEarned} earned / {xpSpent} spent</span>
                </div>
              {/if}
              {#if tenets}
                <div class="info-row">
                  <span class="stat-label">Tenets</span>
                  <span class="info-text info-long">{tenets}</span>
                </div>
              {/if}
              {#if compulsions}
                <div class="info-row">
                  <span class="stat-label">Compulsions</span>
                  <span class="info-text info-long">{compulsions}</span>
                </div>
              {/if}
              {#if notes}
                <div class="info-row">
                  <span class="stat-label">Notes</span>
                  <span class="info-text info-long">{notes}</span>
                </div>
              {/if}
            </div>
          {/if}

          <!-- ── Footer ──────────────────────────────────────────────────── -->
          <div class="card-footer">
            <button class="section-toggle" onclick={() => toggleAttrs(char.id)}>
              attrs {expandedAttrs.has(char.id) ? '▴' : '▾'}
            </button>
            <button class="section-toggle" onclick={() => toggleInfo(char.id)}>
              info {expandedInfo.has(char.id) ? '▴' : '▾'}
            </button>
            <div class="footer-spacer"></div>
            <button class="raw-toggle" onclick={() => toggleRaw(char.id)}>
              raw {expandedRaw.has(char.id) ? '▴' : '▾'}
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
    grid-template-columns: repeat(auto-fill, minmax(20rem, 1fr));
    max-width: 1100px;
    gap: 0.75rem;
    align-items: start;
  }

  /* ── Character card shell ─────────────────────────────────────────────── */
  .char-card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 7px;
    overflow: hidden;
  }
  .char-card:hover { border-color: var(--border-surface); }

  /* ── Header ───────────────────────────────────────────────────────────── */
  .card-header {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 0.5rem;
    padding: 0.75rem 0.9rem 0.65rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .name-clan {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    min-width: 0;
  }
  .char-name {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    line-clamp: 2;
    overflow: hidden;
    word-break: break-word;
  }
  .char-clan {
    font-size: 0.78rem;
    color: var(--text-muted);
    font-style: italic;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .badge.pc  { background: #2a1515; color: var(--accent);  border: 1px solid #3a1e1e; }
  .badge.npc { background: #151528; color: #7986cb; border: 1px solid #1e1e3a; }

  /* ── Header stats sub-row (hunger + BP) ──────────────────────────────── */
  .header-stats {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-top: 0.25rem;
  }
  .bp-pill {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  /* ── Conscience row ──────────────────────────────────────────────────── */
  .conscience-row {
    container-type: inline-size;
    min-height: 3rem;
    display: flex;
    align-items: stretch;
    padding: 0 0.5rem;
    border-bottom: 1px solid var(--border-faint);
    overflow: hidden;
    box-sizing: border-box;
  }

  .qs-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-ghost);
    font-weight: 600;
  }

  /* ── Hunger blood drops ──────────────────────────────────────────────── */
  .hunger-drops {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .blood-drop {
    width: 1.6rem;
    height: 2.1rem;
    fill: none;
    stroke: var(--border-surface);
    stroke-width: 1.5;
    transition: fill 0.2s, stroke 0.2s, filter 0.2s;
  }
  .blood-drop.filled {
    fill: var(--accent);
    stroke: var(--accent);
    filter: drop-shadow(0 0 0.25rem color-mix(in srgb, var(--accent) 60%, transparent));
  }

  /* ── Conscience word-track (Humanity + Stains) ───────────────────────── */
  .conscience-track {
    display: flex;
    width: 100%;
    height: 100%;
    align-items: stretch;
    gap: 0;
  }
  .conscience-letter {
    flex: 1 1 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'Last Rites', cursive;
    font-size: min(9cqi, 4rem);
    font-weight: 400;
    color: var(--text-ghost);
    line-height: 0.7;
    overflow: hidden;
    transition: color 0.2s, text-shadow 0.2s;
    position: relative;
  }
  .conscience-letter.filled {
    color: var(--accent);
    text-shadow: 0 0 0.5rem color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .conscience-letter.stained {
    color: #e07b00;
  }
  .conscience-letter.stained::after {
    content: '';
    position: absolute;
    left: 5%;
    right: 5%;
    top: 50%;
    height: 1px;
    background: #e07b00;
    transform: rotate(-12deg);
  }

  /* ── Health / Willpower boxes ────────────────────────────────────────── */
  .track-boxes {
    display: flex;
    gap: 0.1rem;
  }
  .box {
    flex: 1;
    min-width: 0;
    height: 2.4rem;
    border: 1px solid var(--border-surface);
    border-radius: 0.2rem;
    background: transparent;
    box-sizing: border-box;
  }
  .box.filled           { background: var(--accent);  border-color: var(--accent); }
  .box.willpower.filled { background: #7986cb; border-color: #7986cb; }
  /* Superficial = slash through the box */
  .box.superficial {
    border-color: var(--border-surface);
    background-image: repeating-linear-gradient(
      45deg, var(--accent) 0, var(--accent) 1px, transparent 0, transparent 50%
    );
    background-size: 0.3rem 0.3rem;
  }
  .box.willpower.superficial {
    background-image: repeating-linear-gradient(
      45deg, #7986cb 0, #7986cb 1px, transparent 0, transparent 50%
    );
    background-size: 0.3rem 0.3rem;
    border-color: #7986cb;
  }
  /* Aggravated = fully filled */
  .box.aggravated {
    background: var(--accent);
    border-color: var(--accent);
  }
  .box.willpower.aggravated {
    background: #4a2848;
    border-color: #5c3458;
  }

  /* Blood Potency */
  .bp-value {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--accent-amber);
    line-height: 1;
  }

  /* ── Track row (one per track) ────────────────────────────────────────── */
  .track-row {
    border-bottom: 1px solid var(--border-faint);
  }


  /* ── Shared stat-row label ────────────────────────────────────────────── */
  .stat-label {
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-ghost);
    font-weight: 600;
  }

  /* ── Disciplines ──────────────────────────────────────────────────────── */
  .disc-section {
    padding: 0.6rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .disc-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .disc-chip {
    display: inline-flex;
    align-items: baseline;
    gap: 0.2rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 4px;
    padding: 0.12rem 0.45rem;
  }
  .disc-dots {
    color: #7986cb;
    letter-spacing: 0.05em;
    font-size: 0.75rem;
  }

  /* ── Collapsible sections (attrs, info) ───────────────────────────────── */
  .card-section {
    border-top: 1px solid var(--border-faint);
    padding: 0.7rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  /* 3×3 attribute grid */
  .attr-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.4rem 0.25rem;
    text-align: center;
  }
  .attr-cell {
    display: flex;
    flex-direction: column;
    gap: 0.05rem;
    background: var(--bg-sunken);
    border-radius: 4px;
    padding: 0.28rem 0;
  }
  .attr-name {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-ghost);
  }
  .attr-val {
    font-size: 1rem;
    font-weight: 700;
    color: var(--text-primary);
  }

  /* Bane */
  .bane-row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .bane-severity {
    font-size: 0.75rem;
    font-weight: 700;
    color: var(--accent);
    background: #2a1515;
    border: 1px solid #3a1e1e;
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
    flex-shrink: 0;
  }
  .bane-text {
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
    line-height: 1.4;
  }

  /* Info rows */
  .info-row {
    display: flex;
    flex-direction: column;
    gap: 0.18rem;
  }
  .info-text {
    font-size: 0.83rem;
    color: var(--text-secondary);
    line-height: 1.5;
  }
  .info-long {
    max-height: 4rem;
    overflow-y: auto;
  }

  /* ── Footer ───────────────────────────────────────────────────────────── */
  .card-footer {
    padding: 0.35rem 0.9rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-top: 1px solid var(--border-faint);
  }
  .footer-spacer { flex: 1; }
  .section-toggle,
  .raw-toggle {
    font-size: 0.7rem;
    color: var(--text-ghost);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.1rem 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .section-toggle:hover,
  .raw-toggle:hover { color: var(--text-muted); }

  /* ── Raw attribute dump ───────────────────────────────────────────────── */
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
    font-size: 0.72rem;
    font-family: monospace;
    line-height: 1.7;
  }
  .raw-name  { color: var(--text-muted); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .raw-val   { color: var(--accent); flex-shrink: 0; }
  .raw-empty { font-size: 0.72rem; color: var(--text-ghost); font-style: italic; }
</style>
