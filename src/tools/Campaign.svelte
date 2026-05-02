<script lang="ts">
  import { onMount } from 'svelte';
  import { bridge, anyConnected } from '../store/bridge.svelte';
  import { savedCharacters } from '../store/savedCharacters.svelte';
  import { refresh as bridgeRefresh } from '$lib/bridge/api';
  import SourceAttributionChip from '$lib/components/SourceAttributionChip.svelte';
  import CompareModal from '$lib/components/CompareModal.svelte';
  import { diffCharacter } from '$lib/saved-characters/diff';
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import type { BridgeCharacter, Roll20Raw, Roll20RawAttribute } from '../types';
  import {
    foundryFeatures,
    foundryEffects,
    foundryItemEffects,
    foundryEffectIsActive,
  } from '$lib/foundry/raw';
  import { characterSetField } from '$lib/character/api';
  import type { CanonicalFieldName } from '$lib/character/api';

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

  // Compare-modal state.
  let comparing = $state<{ saved: SavedCharacter; live: BridgeCharacter } | null>(null);
  function openCompare(saved: SavedCharacter, live: BridgeCharacter) {
    comparing = { saved, live };
  }
  function closeCompare() { comparing = null; }

  onMount(() => { void savedCharacters.ensureLoaded(); });
  let expandedRaw   = $state<Set<string>>(new Set());
  let expandedAttrs = $state<Set<string>>(new Set());
  let expandedInfo  = $state<Set<string>>(new Set());
  let expandedFeats = $state<Set<string>>(new Set());

  // ── Stat editor (#7) ────────────────────────────────────────────────────

  /// Per-field clamp ranges. Mirrors src-tauri/src/shared/canonical_fields.rs
  /// expect_u8_in_range() bounds; keep the two in sync.
  const FIELD_RANGES: Record<CanonicalFieldName, [number, number]> = {
    hunger:                [0, 5],
    humanity:              [0, 10],
    humanity_stains:       [0, 10],
    blood_potency:         [0, 10],
    health_superficial:    [0, 20],
    health_aggravated:     [0, 20],
    willpower_superficial: [0, 20],
    willpower_aggravated:  [0, 20],
  };

  /// True when the live card supports inline ±1 editing. Roll20 live editing
  /// of canonical names is deferred to Phase 2.5 (router spec §2.8).
  function liveEditAllowed(char: BridgeCharacter): boolean {
    return char.source === 'foundry';
  }

  /// Identity for a per-field stepper: card key plus field name. Used to
  /// scope the busy-disabled state to one button at a time.
  function stepperKey(char: BridgeCharacter, field: CanonicalFieldName): string {
    return `${char.source}:${char.source_id}:${field}`;
  }

  /// Which stepper is currently mid-IPC. Null when idle.
  let busyKey = $state<string | null>(null);

  async function tweakField(
    char: BridgeCharacter,
    field: CanonicalFieldName,
    delta: number,
    current: number,
  ) {
    const range = FIELD_RANGES[field];
    const next  = Math.max(range[0], Math.min(range[1], current + delta));
    if (next === current) return;
    const key = stepperKey(char, field);
    busyKey = key;
    try {
      await characterSetField('live', char.source, char.source_id, field, next);
    } catch (e) {
      console.error('[Campaign] characterSetField failed:', e);
      window.alert(String(e));
    } finally {
      if (busyKey === key) busyKey = null;
    }
  }

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
      s: { minCol: '16rem', pad: '0.4rem', trackH: '1.4rem', conscienceCap: '1.5rem', dropSize: '1.2rem', conscienceGlow: 'none' },
      m: { minCol: '20rem', pad: '0.6rem', trackH: '1.8rem', conscienceCap: '2.5rem', dropSize: '1.6rem', conscienceGlow: '0 0 0.3rem color-mix(in srgb, var(--accent) 30%, transparent)' },
      l: { minCol: '28rem', pad: '0.8rem', trackH: '2.4rem', conscienceCap: '4rem', dropSize: '2rem', conscienceGlow: '0 0 0.5rem color-mix(in srgb, var(--accent) 50%, transparent)' },
    }[d];
    return `--col-min:${vals.minCol};--card-pad:${vals.pad};--track-h:${vals.trackH};--conscience-cap:${vals.conscienceCap};--drop-size:${vals.dropSize};--conscience-glow:${vals.conscienceGlow}`;
  });

  // ── Helpers ─────────────────────────────────────────────────────────────

  /// Returns the raw Roll20 attribute list for a character, or [] for
  /// non-Roll20 chars or if the raw blob is missing. Roll20-specific
  /// helpers (clan, attributes, disciplines, etc.) read through this.
  function r20Attrs(char: BridgeCharacter): Roll20RawAttribute[] {
    if (char.source !== 'roll20') return [];
    const raw = char.raw as Roll20Raw | null;
    return raw?.attributes ?? [];
  }

  function r20AttrInt(char: BridgeCharacter, name: string): number {
    const a = r20Attrs(char).find(a => a.name === name);
    return a ? (parseInt(a.current, 10) || 0) : 0;
  }

  function r20AttrText(char: BridgeCharacter, name: string): string {
    return r20Attrs(char).find(a => a.name === name)?.current ?? '';
  }

  function parseDisciplines(char: BridgeCharacter): { type: string; level: number }[] {
    const attrs = r20Attrs(char);
    const prefix = 'repeating_disciplines_';
    const suffix = '_discipline';
    return attrs
      .filter(a =>
        a.name.startsWith(prefix) &&
        a.name.endsWith(suffix) &&
        !a.name.includes('_power_'),
      )
      .map(a => {
        const id = a.name.slice(prefix.length, -suffix.length);
        const nameAttr = attrs.find(x => x.name === `${prefix}${id}_discipline_name`);
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
  function toggleFeats(id: string) { expandedFeats = toggleSet(expandedFeats, id); }

  function isPC(char: BridgeCharacter): boolean {
    return char.controlled_by !== null && char.controlled_by.trim() !== '';
  }

  function timeSince(d: Date): string {
    const s = Math.floor((Date.now() - d.getTime()) / 1000);
    return s < 60 ? `${s}s ago` : `${Math.floor(s / 60)}m ago`;
  }

  function refresh() {
    bridgeRefresh();
  }
</script>

{#snippet stepper(char: BridgeCharacter, field: CanonicalFieldName, current: number)}
  {@const allowed   = liveEditAllowed(char)}
  {@const key       = stepperKey(char, field)}
  {@const busy      = busyKey === key}
  {@const range     = FIELD_RANGES[field]}
  {@const atFloor   = current <= range[0]}
  {@const atCeiling = current >= range[1]}
  {@const tooltip   = allowed
    ? ''
    : 'Roll20 live editing not supported (Phase 2.5)'}
  <span class="stat-stepper" class:roll20-blocked={!allowed}>
    <button
      type="button"
      class="step-btn"
      onclick={() => tweakField(char, field, -1, current)}
      disabled={!allowed || busy || atFloor}
      aria-busy={busy}
      title={tooltip}
      aria-label={`Decrease ${field}`}
    >−</button>
    <button
      type="button"
      class="step-btn"
      onclick={() => tweakField(char, field, +1, current)}
      disabled={!allowed || busy || atCeiling}
      aria-busy={busy}
      title={tooltip}
      aria-label={`Increase ${field}`}
    >+</button>
  </span>
{/snippet}

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

  {#if !connected}
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
  {:else if characters.length === 0}
    <div class="disconnected-banner">
      <p class="banner-title">Connected — waiting for characters</p>
      <p class="banner-body">The extension is connected but no character data has arrived yet. Try clicking Refresh.</p>
    </div>
  {:else}
    <div class="char-grid" bind:this={gridEl} style={densityVars}>
      {#each liveWithMatches as item (item.live.source + ':' + item.live.source_id)}
        {@const char         = item.live}
        {@const saved        = item.saved}
        {@const charKey      = char.source + ':' + char.source_id}
        {@const healthMax    = char.health?.max ?? 5}
        {@const healthSup    = char.health?.superficial ?? 0}
        {@const healthAgg    = char.health?.aggravated ?? 0}
        {@const healthOk     = Math.max(0, healthMax - healthSup - healthAgg)}
        {@const wpMax        = char.willpower?.max ?? 5}
        {@const wpSup        = char.willpower?.superficial ?? 0}
        {@const wpAgg        = char.willpower?.aggravated ?? 0}
        {@const wpOk         = Math.max(0, wpMax - wpSup - wpAgg)}
        {@const hunger       = char.hunger ?? 0}
        {@const humanity     = char.humanity ?? 0}
        {@const bp           = char.blood_potency ?? 0}
        {@const stains       = char.humanity_stains ?? 0}
        {@const clan         = r20AttrText(char, 'clan')}
        {@const disciplines  = parseDisciplines(char)}
        {@const strAttr      = r20AttrInt(char, 'strength')}
        {@const dexAttr      = r20AttrInt(char, 'dexterity')}
        {@const staAttr      = r20AttrInt(char, 'stamina')}
        {@const chaAttr      = r20AttrInt(char, 'charisma')}
        {@const manAttr      = r20AttrInt(char, 'manipulation')}
        {@const comAttr      = r20AttrInt(char, 'composure')}
        {@const intAttr      = r20AttrInt(char, 'intelligence')}
        {@const witAttr      = r20AttrInt(char, 'wits')}
        {@const resAttr      = r20AttrInt(char, 'resolve')}
        {@const bane         = r20AttrText(char, 'bane')}
        {@const baneSeverity = r20AttrText(char, 'blood_bane_severity')}
        {@const ambition     = r20AttrText(char, 'ambition')}
        {@const desire       = r20AttrText(char, 'desire')}
        {@const predator     = r20AttrText(char, 'predator')}
        {@const xpEarned     = r20AttrInt(char, 'experience')}
        {@const xpSpent      = r20AttrInt(char, 'experience_spent')}
        {@const sire         = r20AttrText(char, 'sire')}
        {@const ageTrue      = r20AttrInt(char, 'age_true')}
        {@const ageApparent  = r20AttrInt(char, 'age_apparent')}
        {@const tenets       = r20AttrText(char, 'tenets')}
        {@const notes        = r20AttrText(char, 'notes')}
        {@const compulsions  = r20AttrText(char, 'compulsions')}
        {@const merits       = foundryFeatures(char, 'merit')}
        {@const flaws        = foundryFeatures(char, 'flaw')}
        {@const backgrounds  = foundryFeatures(char, 'background')}
        {@const boons        = foundryFeatures(char, 'boon')}
        {@const actorFx      = foundryEffects(char)}

        <div class="char-card">

          <!-- ── Header ──────────────────────────────────────────────────── -->
          <div class="card-header">
            <div class="header-line">
              <span class="char-name">{char.name}</span>
              <div class="header-badges">
                {#if saved && hasDrift(char)}
                  <span class="drift-badge" title="Live differs from saved snapshot">drift</span>
                {/if}
                <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
                  {isPC(char) ? 'PC' : 'NPC'}
                </span>
              </div>
            </div>
            <div class="header-line">
              {#if clan}<span class="char-clan">{clan}</span>{:else}<span></span>{/if}
              <div class="header-vitals">
                <div class="hunger-cluster">
                  <div class="hunger-drops">
                    {#each dots(hunger, 5) as filled}
                      <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
                        <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
                      </svg>
                    {/each}
                  </div>
                  {@render stepper(char, 'hunger', hunger)}
                </div>
                <div class="bp-pill">
                  <span class="qs-label">BP</span>
                  <span class="bp-value">{bp}</span>
                </div>
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
          {#if expandedAttrs.has(charKey)}
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
          {#if expandedInfo.has(charKey)}
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

          <!-- ── Collapsible: feats (merits/flaws/backgrounds/boons + actor effects) ── -->
          {#if expandedFeats.has(charKey)}
            <div class="card-section">
              {#if merits.length > 0}
                <div class="feat-row">
                  <span class="stat-label">Merits</span>
                  <div class="feat-chips">
                    {#each merits as m}
                      {@const points = (m.system?.points as number | undefined) ?? 0}
                      {@const itemFx = foundryItemEffects(m).filter(foundryEffectIsActive)}
                      <span class="feat-chip merit" title={itemFx.length > 0 ? `${itemFx.length} active modifier(s)` : ''}>
                        <span class="feat-name">{m.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {#if itemFx.length > 0}<span class="feat-fx-badge">+{itemFx.length}</span>{/if}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
              {#if flaws.length > 0}
                <div class="feat-row">
                  <span class="stat-label">Flaws</span>
                  <div class="feat-chips">
                    {#each flaws as f}
                      {@const points = (f.system?.points as number | undefined) ?? 0}
                      {@const itemFx = foundryItemEffects(f).filter(foundryEffectIsActive)}
                      <span class="feat-chip flaw" title={itemFx.length > 0 ? `${itemFx.length} active modifier(s)` : ''}>
                        <span class="feat-name">{f.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {#if itemFx.length > 0}<span class="feat-fx-badge">+{itemFx.length}</span>{/if}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
              {#if backgrounds.length > 0}
                <div class="feat-row">
                  <span class="stat-label">Backgrounds</span>
                  <div class="feat-chips">
                    {#each backgrounds as b}
                      {@const points = (b.system?.points as number | undefined) ?? 0}
                      <span class="feat-chip background">
                        <span class="feat-name">{b.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
              {#if boons.length > 0}
                <div class="feat-row">
                  <span class="stat-label">Boons</span>
                  <div class="feat-chips">
                    {#each boons as bn}
                      <span class="feat-chip boon">
                        <span class="feat-name">{bn.name}</span>
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
              {#if actorFx.length > 0}
                <div class="feat-row">
                  <span class="stat-label">Active modifiers (actor)</span>
                  <div class="feat-chips">
                    {#each actorFx as e}
                      {@const active = foundryEffectIsActive(e)}
                      <span class="feat-chip effect" class:disabled={!active}
                            title={e.changes?.map(c => `${c.key} mode=${c.mode} value=${c.value}`).join('\n') ?? ''}>
                        <span class="feat-name">{e.name}</span>
                        <span class="feat-fx-badge">{e.changes?.length ?? 0}</span>
                      </span>
                    {/each}
                  </div>
                </div>
              {/if}
              {#if merits.length === 0 && flaws.length === 0 && backgrounds.length === 0 && boons.length === 0 && actorFx.length === 0}
                <span class="feat-empty">No merits, flaws, backgrounds, or modifiers on this character.</span>
              {/if}
            </div>
          {/if}

          <!-- ── Save row (source chip + Save/Update) ────────────────────── -->
          <div class="save-row">
            <SourceAttributionChip source={char.source} />
            <div class="save-actions">
              {#if saved}
                <button
                  type="button"
                  class="btn-save"
                  onclick={() => openCompare(saved, char)}
                >Compare</button>
                <button
                  type="button"
                  class="btn-save"
                  onclick={() => savedCharacters.update(saved.id, char)}
                  disabled={savedCharacters.loading}
                >Update saved</button>
              {:else}
                <button
                  type="button"
                  class="btn-save"
                  onclick={() => savedCharacters.save(
                    char,
                    char.source === 'foundry' ? (bridge.sourceInfo.foundry?.worldTitle ?? null) : null,
                  )}
                  disabled={savedCharacters.loading}
                >Save locally</button>
              {/if}
            </div>
          </div>

          <!-- ── Footer ──────────────────────────────────────────────────── -->
          <div class="card-footer">
            <button class="section-toggle" onclick={() => toggleAttrs(charKey)}>
              attrs {expandedAttrs.has(charKey) ? '▴' : '▾'}
            </button>
            <button class="section-toggle" onclick={() => toggleInfo(charKey)}>
              info {expandedInfo.has(charKey) ? '▴' : '▾'}
            </button>
            <button class="section-toggle" onclick={() => toggleFeats(charKey)}>
              feats {expandedFeats.has(charKey) ? '▴' : '▾'}
            </button>
            <div class="footer-spacer"></div>
            <button class="raw-toggle" onclick={() => toggleRaw(charKey)}>
              raw {expandedRaw.has(charKey) ? '▴' : '▾'}
            </button>
          </div>

          {#if expandedRaw.has(charKey)}
            <div class="raw-panel">
              {#if char.source === 'roll20'}
                {@const r20 = r20Attrs(char)}
                {#each r20 as a}
                  <div class="raw-row">
                    <span class="raw-name">{a.name}</span>
                    <span class="raw-val">{a.current}{a.max ? ' / ' + a.max : ''}</span>
                  </div>
                {/each}
                {#if r20.length === 0}
                  <span class="raw-empty">No attributes loaded</span>
                {/if}
              {:else}
                <pre class="raw-json">{JSON.stringify(char.raw, null, 2)}</pre>
              {/if}
            </div>
          {/if}

        </div>
      {/each}
    </div>
  {/if}

  <!-- ── Saved section ────────────────────────────────────────────────── -->
  <section class="saved-section">
    <h2 class="section-title">Saved · {savedCharacters.list.length} characters</h2>
    {#if savedCharacters.loading}
      <p class="muted">Loading…</p>
    {:else if savedCharacters.error}
      <p class="err">{savedCharacters.error}</p>
    {:else if savedCharacters.list.length === 0}
      <p class="muted">No saved characters yet. Click "Save locally" on a live character to save a snapshot.</p>
    {:else}
      <div class="char-grid">
        {#each savedCharacters.list as saved (saved.id)}
          <article class="saved-card">
            <header class="saved-header">
              <strong class="saved-name">{saved.name}</strong>
            </header>
            <SourceAttributionChip source={saved.source} worldTitle={saved.foundryWorld} />
            <div class="saved-meta">saved {saved.savedAt}</div>
            <div class="saved-actions">
              <button
                type="button"
                class="btn-save"
                onclick={() => savedCharacters.delete(saved.id)}
                disabled={savedCharacters.loading}
              >Delete</button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>

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
  .raw-json {
    font-family: 'Consolas', 'Monaco', monospace;
    font-size: 0.7rem;
    color: var(--text-secondary);
    background: var(--bg-sunken);
    border-radius: 3px;
    padding: 0.5rem;
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
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
    flex-direction: column;
    gap: 0.15rem;
    padding: var(--card-pad, 0.6rem) var(--card-pad, 0.6rem) calc(var(--card-pad, 0.6rem) - 0.1rem);
    border-bottom: 1px solid var(--border-faint);
  }
  .header-line {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }
  .header-vitals {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
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

  .header-badges {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    flex-shrink: 0;
  }
  .drift-badge {
    font-size: 0.65rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    background: color-mix(in oklab, var(--accent-amber) 20%, transparent);
    color: var(--accent-amber);
    flex-shrink: 0;
  }

  .bp-pill {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  /* ── Conscience row ──────────────────────────────────────────────────── */
  .conscience-row {
    container-type: inline-size;
    display: flex;
    align-items: stretch;
    padding: 0.2rem var(--card-pad, 0.6rem);
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
  .hunger-cluster {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
  }

  /* ── Stat-editor stepper (#7) ────────────────────────────────────────── */
  .stat-stepper {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
  }
  .step-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.25rem;
    height: 1.25rem;
    padding: 0;
    font-size: 0.95rem;
    font-weight: 700;
    line-height: 1;
    color: var(--text-muted);
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    cursor: pointer;
    transition: color 0.1s, border-color 0.1s, background 0.1s, opacity 0.1s;
    user-select: none;
  }
  .step-btn:hover:not(:disabled) {
    color: var(--accent);
    border-color: var(--accent);
  }
  .step-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }
  .step-btn[aria-busy="true"] {
    opacity: 0.55;
    cursor: wait;
  }
  .stat-stepper.roll20-blocked .step-btn {
    border-style: dashed;
  }
  .blood-drop {
    width: var(--drop-size, 1.6rem);
    height: calc(var(--drop-size, 1.6rem) * 1.3125);
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
    align-items: stretch;
    gap: 0;
  }
  .conscience-letter {
    flex: 1 1 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: 'Last Rites', cursive;
    font-size: min(9cqi, var(--conscience-cap, 2.5rem));
    font-weight: 400;
    color: var(--text-ghost);
    line-height: 1;
    padding: 0.15rem 0;
    overflow: hidden;
    transition: color 0.2s, text-shadow 0.2s;
    position: relative;
  }
  .conscience-letter.filled {
    color: var(--accent);
    text-shadow: var(--conscience-glow, none);
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
    height: var(--track-h, 1.8rem);
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
    font-size: calc(var(--drop-size, 1.6rem) * 0.9375);
    font-weight: 700;
    color: var(--accent-amber);
    line-height: 1;
  }

  /* ── Track row (one per track) ────────────────────────────────────────── */
  .track-row {
    padding: 0.2rem var(--card-pad, 0.6rem);
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

  /* ── Feats (merits/flaws/backgrounds/boons + actor effects) ───────────── */
  .feat-row {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .feat-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .feat-chip {
    display: inline-flex;
    align-items: baseline;
    gap: 0.3rem;
    font-size: 0.78rem;
    color: var(--text-secondary);
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 4px;
    padding: 0.15rem 0.5rem;
  }
  .feat-chip.merit      { border-color: color-mix(in srgb, var(--accent) 40%, var(--border-faint)); }
  .feat-chip.flaw       { border-color: color-mix(in srgb, var(--accent-amber) 40%, var(--border-faint)); color: var(--accent-amber); }
  .feat-chip.background { border-color: var(--border-surface); }
  .feat-chip.boon       { border-color: color-mix(in srgb, var(--accent-bright) 30%, var(--border-faint)); }
  .feat-chip.effect.disabled { opacity: 0.45; }
  .feat-name {
    font-weight: 500;
  }
  .feat-dots {
    color: var(--accent);
    letter-spacing: 0.05em;
    font-size: 0.7rem;
  }
  .feat-fx-badge {
    font-size: 0.6rem;
    font-weight: 700;
    color: var(--accent-bright);
    padding: 0.05rem 0.25rem;
    border-radius: 2px;
    background: color-mix(in srgb, var(--accent-bright) 10%, transparent);
  }
  .feat-empty {
    font-size: 0.78rem;
    color: var(--text-ghost);
    font-style: italic;
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

  /* ── Save row on live cards ───────────────────────────────────────────── */
  .save-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.9rem;
    border-top: 1px solid var(--border-faint);
  }
  .save-actions {
    margin-left: auto;
    display: flex;
    gap: 0.4rem;
  }
  .btn-save {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    color: var(--text-secondary);
    padding: 0.25rem 0.65rem;
    border-radius: 4px;
    font-size: 0.75rem;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }
  .btn-save:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text-primary);
  }
  .btn-save:disabled {
    opacity: 0.4;
    cursor: default;
  }

  /* ── Saved section ────────────────────────────────────────────────────── */
  .saved-section {
    margin-top: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .section-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-label);
    margin: 0;
  }
  .muted { font-size: 0.85rem; color: var(--text-muted); margin: 0; }
  .err   { font-size: 0.85rem; color: var(--accent-amber); margin: 0; }

  .saved-card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 7px;
    padding: 0.7rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .saved-card:hover { border-color: var(--border-surface); }
  .saved-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .saved-name {
    font-size: 0.95rem;
    color: var(--text-primary);
    font-weight: 600;
    word-break: break-word;
  }
  .saved-meta {
    font-size: 0.72rem;
    color: var(--text-ghost);
  }
  .saved-actions {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.2rem;
  }
</style>
