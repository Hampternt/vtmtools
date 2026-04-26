<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ResonanceSlider from '$lib/components/ResonanceSlider.svelte';
  import TemperamentConfigComponent from '$lib/components/TemperamentConfig.svelte';
  import ResultCard from '$lib/components/ResultCard.svelte';
  import RollHistory from '$lib/components/RollHistory.svelte';
  import { publishEvent } from '../store/toolEvents';
  import { bridge, anyConnected, sourceLabel } from '../store/bridge.svelte';
  import { refresh as bridgeRefresh, setAttribute } from '$lib/bridge/api';
  import type { RollConfig, ResonanceRollResult, HistoryEntry, BridgeCharacter, Roll20Raw, DyscrasiaEntry } from '../types';

  // ── Roll config ──────────────────────────────────────────────────────────────
  let config: RollConfig = $state({
    temperament: {
      diceCount: 1,
      takeHighest: true,
      negligibleMax: 5,
      fleetingMax: 8,
    },
    weights: {
      phlegmatic: 'neutral',
      melancholy: 'neutral',
      choleric: 'neutral',
      sanguine: 'neutral',
    }
  });

  let result: ResonanceRollResult | null = $state(null);
  let rolling = $state(false);
  let rollHistory: HistoryEntry[] = $state([]);
  let nextId = 0;

  // ── Bridge state (multi-source) ──────────────────────────────────────────────
  // Selected character keyed by `<source>:<source_id>` since IDs from
  // different sources may technically collide.
  let selectedKey   = $state<string | null>(null);
  let selectorOpen  = $state(false);
  let applyState    = $state<'idle' | 'applying' | 'applied' | 'error'>('idle');
  let confirmedDyscrasia = $state<DyscrasiaEntry | null>(null);
  let dyscrasiaApplyState = $state<'idle' | 'applying' | 'applied' | 'error'>('idle');

  const charKey = (c: BridgeCharacter) => `${c.source}:${c.source_id}`;
  const connected = $derived(anyConnected());
  const characters = $derived(bridge.characters);
  const selectedChar = $derived(characters.find(c => charKey(c) === selectedKey) ?? null);

  $effect(() => {
    if (selectedKey && !characters.some(c => charKey(c) === selectedKey)) {
      selectedKey = null;
    }
  });

  // ── Helpers ──────────────────────────────────────────────────────────────────
  /// Reads a named attribute from a Roll20 character's raw blob. Returns
  /// '' for non-Roll20 characters or attributes that don't exist on the
  /// sheet — Foundry character display omits clan/resonance for now.
  function r20Attr(char: BridgeCharacter, name: string): string {
    if (char.source !== 'roll20') return '';
    const raw = char.raw as Roll20Raw | null;
    return raw?.attributes?.find(a => a.name === name)?.current ?? '';
  }

  // ── Resonance probability math ───────────────────────────────────────────────
  const WEIGHT_MULT: Record<string, number> = {
    impossible:        0,
    extremelyUnlikely: 0.1,
    unlikely:          0.5,
    neutral:           1.0,
    likely:            2.0,
    extremelyLikely:   4.0,
    guaranteed:        Infinity,
  };

  const RES_TYPES = [
    { key: 'phlegmatic', label: 'Phlegmatic', base: 0.25 },
    { key: 'melancholy',  label: 'Melancholy',  base: 0.25 },
    { key: 'choleric',    label: 'Choleric',    base: 0.25 },
    { key: 'sanguine',    label: 'Sanguine',    base: 0.25 },
  ] as const;

  function calcResProbs(w: typeof config.weights): number[] {
    const mults = RES_TYPES.map(t => WEIGHT_MULT[w[t.key]] ?? 1.0);
    const gIdx = mults.findIndex(m => !isFinite(m));
    if (gIdx >= 0) return RES_TYPES.map((_, i) => (i === gIdx ? 100 : 0));
    const weighted = RES_TYPES.map((t, i) => t.base * mults[i]);
    const total = weighted.reduce((a, b) => a + b, 0);
    if (total === 0) return [25, 25, 25, 25];
    const raw = weighted.map(v => (v / total) * 100);
    const rounded = raw.map(v => Math.round(v));
    const diff = 100 - rounded.reduce((a, b) => a + b, 0);
    const maxIdx = rounded.indexOf(Math.max(...rounded));
    rounded[maxIdx] += diff;
    return rounded;
  }

  const resProbs = $derived(calcResProbs(config.weights));

  // ── Actions ──────────────────────────────────────────────────────────────────
  async function roll() {
    rolling = true;
    result = null;
    applyState = 'idle';
    try {
      result = await invoke<ResonanceRollResult>('roll_resonance', { config });
      if (result) {
        rollHistory = [{ id: nextId++, timestamp: new Date(), result }, ...rollHistory].slice(0, 100);
        publishEvent({
          type: 'resonance_result',
          payload: {
            temperament: result.temperament,
            resonanceType: result.resonanceType,
            isAcute: result.isAcute,
            dyscrasiaName: result.dyscrasia?.name ?? null,
          }
        });
      }
    } finally {
      rolling = false;
    }
  }

  async function applyToCharacter() {
    if (!result?.resonanceType || !selectedChar) return;
    applyState = 'applying';
    try {
      await setAttribute(
        selectedChar.source,
        selectedChar.source_id,
        'resonance',
        result.resonanceType,
      );
      applyState = 'applied';
      setTimeout(() => { applyState = 'idle'; }, 1800);
    } catch {
      applyState = 'error';
      setTimeout(() => { applyState = 'idle'; }, 1800);
    }
  }

  async function applyDyscrasia() {
    if (!confirmedDyscrasia || !selectedChar) return;
    if (selectedChar.source !== 'foundry') return; // hard guard; UI also hides the button
    dyscrasiaApplyState = 'applying';
    try {
      const payload = JSON.stringify({
        dyscrasia_name: confirmedDyscrasia.name,
        resonance_type: confirmedDyscrasia.resonanceType,
        description: confirmedDyscrasia.description,
        bonus: confirmedDyscrasia.bonus,
      });
      await setAttribute(
        selectedChar.source,
        selectedChar.source_id,
        'dyscrasia',
        payload,
      );
      dyscrasiaApplyState = 'applied';
      setTimeout(() => { dyscrasiaApplyState = 'idle'; }, 1800);
    } catch {
      dyscrasiaApplyState = 'error';
      setTimeout(() => { dyscrasiaApplyState = 'idle'; }, 1800);
    }
  }

  function selectChar(key: string) {
    selectedKey = key;
    selectorOpen = false;
  }
</script>

<div class="page">
  <h1 class="title">Resonance Roller</h1>
  <p class="subtitle">Configure the feeding conditions, then roll.</p>

  <div class="main-layout">
    <div class="steps-panel">

      <!-- ── Target character ── -->
      <section class="step">
        <h3>Target character</h3>

        {#if !connected}
          <div class="bridge-status bridge-disconnected">
            <span class="bridge-dot"></span>
            <span>Not connected to any VTT bridge</span>
          </div>
        {:else if characters.length === 0}
          <div class="bridge-status bridge-empty">
            No characters loaded —
            <button class="link-btn" onclick={() => bridgeRefresh()}>refresh</button>
          </div>
        {:else}
          <!-- Medium / wide: horizontal wrapping card strip -->
          <div class="char-strip">
            {#each characters as char (charKey(char))}
              {@const key  = charKey(char)}
              {@const clan = r20Attr(char, 'clan')}
              {@const res  = r20Attr(char, 'resonance')}
              <button
                class="char-card"
                class:char-card--selected={key === selectedKey}
                data-res={res || null}
                onclick={() => selectChar(key)}
              >
                <span class="char-name">{char.name}</span>
                <span class="char-source">{sourceLabel(char.source)}</span>
                {#if clan}<span class="char-clan">{clan}</span>{/if}
                {#if res}<span class="char-res">{res}</span>{/if}
              </button>
            {/each}
          </div>

          <!-- Narrow: compact selector button + dropdown -->
          <div class="char-selector-narrow">
            <button
              class="selector-btn"
              class:selector-btn--active={!!selectedChar}
              onclick={() => { selectorOpen = !selectorOpen; }}
            >
              {#if selectedChar}
                <span class="sel-dot"></span>
                <span class="sel-name">{selectedChar.name}</span>
                <span class="sel-clan">{r20Attr(selectedChar, 'clan') || sourceLabel(selectedChar.source)}</span>
              {:else}
                <span class="sel-placeholder">Choose character…</span>
              {/if}
              <span class="sel-chevron" class:open={selectorOpen}>⌄</span>
            </button>

            {#if selectorOpen}
              <button class="selector-backdrop" onclick={() => { selectorOpen = false; }} aria-label="Close picker"></button>
              <div class="selector-dropdown">
                <div class="dropdown-header">Select character</div>
                {#each characters as char (charKey(char))}
                  {@const key  = charKey(char)}
                  {@const clan = r20Attr(char, 'clan')}
                  {@const res  = r20Attr(char, 'resonance')}
                  <button
                    class="drop-item"
                    class:drop-item--selected={key === selectedKey}
                    onclick={() => selectChar(key)}
                  >
                    <div class="drop-item-body">
                      <span class="drop-name">
                        {char.name}
                        <span class="drop-source">[{sourceLabel(char.source)}]</span>
                      </span>
                      {#if clan}<span class="drop-clan">{clan}</span>{/if}
                      {#if res}<span class="drop-res">{res}</span>{/if}
                    </div>
                    {#if key === selectedKey}<span class="drop-check">✓</span>{/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <div class="roll-area">
          <button class="roll-btn" onclick={roll} disabled={rolling}>
            {rolling ? 'Rolling…' : '⚀ Roll'}
          </button>
        </div>
      </section>

      <!-- ── Result + Apply (above config) ── -->
      {#if result}
        <ResultCard
          {result}
          onDyscrasiaConfirmChange={(d) => { confirmedDyscrasia = d; }}
        />
        <div class="apply-row">
          {#if selectedChar && result.resonanceType}
            <button
              class="apply-btn"
              class:applied={applyState === 'applied'}
              class:error={applyState === 'error'}
              onclick={applyToCharacter}
              disabled={applyState !== 'idle'}
            >
              {applyState === 'applying' ? 'Applying…'
               : applyState === 'applied' ? '✓ Applied'
               : applyState === 'error' ? '✗ Failed — retry'
               : `✓ Apply to ${selectedChar.name}`}
            </button>
          {/if}
          {#if selectedChar?.source === 'foundry' && confirmedDyscrasia !== null}
            <button
              class="apply-btn apply-btn--dyscrasia"
              class:applied={dyscrasiaApplyState === 'applied'}
              class:error={dyscrasiaApplyState === 'error'}
              onclick={applyDyscrasia}
              disabled={dyscrasiaApplyState !== 'idle'}
            >
              {dyscrasiaApplyState === 'applying' ? 'Applying Dyscrasia…'
               : dyscrasiaApplyState === 'applied' ? '✓ Dyscrasia Applied'
               : dyscrasiaApplyState === 'error' ? '✗ Failed — retry'
               : `✓ Apply Dyscrasia to ${selectedChar.name}`}
            </button>
          {/if}
        </div>
      {/if}

      <!-- ── Config steps ── -->
      <section class="step">
        <h3>1. Temperament dice</h3>
        <TemperamentConfigComponent
          diceCount={config.temperament.diceCount}
          takeHighest={config.temperament.takeHighest}
          negligibleMax={config.temperament.negligibleMax}
          fleetingMax={config.temperament.fleetingMax}
          onDiceCountChange={(n) => (config.temperament.diceCount = n)}
          onTakeHighestChange={(b) => (config.temperament.takeHighest = b)}
          onNegligibleMaxChange={(n) => (config.temperament.negligibleMax = n)}
          onFleetingMaxChange={(n) => (config.temperament.fleetingMax = n)}
        />
      </section>

      <section class="step">
        <h3>2. Resonance type odds</h3>
        <ResonanceSlider
          label="Phlegmatic"
          value={config.weights.phlegmatic}
          onChange={(v) => (config.weights.phlegmatic = v)}
        />
        <ResonanceSlider
          label="Melancholy"
          value={config.weights.melancholy}
          onChange={(v) => (config.weights.melancholy = v)}
        />
        <ResonanceSlider
          label="Choleric"
          value={config.weights.choleric}
          onChange={(v) => (config.weights.choleric = v)}
        />
        <ResonanceSlider
          label="Sanguine"
          value={config.weights.sanguine}
          onChange={(v) => (config.weights.sanguine = v)}
        />

        <div class="res-probs">
          <div class="res-bar">
            {#each RES_TYPES as type, i}
              {#if resProbs[i] > 0}
                <div
                  class="res-seg res-seg-{type.key}"
                  style="width:{resProbs[i]}%"
                  title="{type.label}: {resProbs[i]}%"
                ></div>
              {/if}
            {/each}
          </div>
          <div class="res-legend">
            {#each RES_TYPES as type, i}
              <div class="leg-item {resProbs[i] === 0 ? 'leg-zero' : ''}">
                <span class="leg-dot leg-dot-{type.key}"></span>
                <span class="leg-name">{type.label}</span>
                <span class="leg-pct">{resProbs[i]}%</span>
              </div>
            {/each}
          </div>
        </div>
      </section>

    </div>

    <!-- ── History (unchanged) ── -->
    <div class="history-panel">
      <RollHistory entries={rollHistory} />
    </div>
  </div>
</div>

<style>
  .page {
    width: 100%;
    container-type: inline-size;
    container-name: resonance-page;
  }
  .title    { color: var(--accent); font-size: 1.8rem; margin-bottom: 0.25rem; }
  .subtitle { color: var(--text-secondary); font-size: 0.9rem; margin-bottom: 1.5rem; }

  /* ── Stacked layout (default / narrow) ── */
  .main-layout { display: flex; flex-direction: column; gap: 1.5rem; align-items: stretch; }
  .steps-panel { display: flex; flex-direction: column; gap: 1.5rem; }
  .step {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 6px;
    padding: 1rem 1.25rem;
  }
  h3 {
    color: var(--text-label); font-size: 0.9rem; text-transform: uppercase;
    letter-spacing: 0.08em; margin: 0 0 0.75rem;
  }

  /* ── Roll20 connection states ── */
  .bridge-status {
    font-size: 0.8rem; display: flex; align-items: center;
    gap: 0.4rem; padding: 0.35rem 0; margin-bottom: 0.75rem;
  }
  .bridge-disconnected{ color: var(--text-muted); opacity: 0.45; }
  .bridge-empty{ color: var(--text-secondary); }
  .bridge-dot {
    width: 0.45rem; height: 0.45rem; border-radius: 50%;
    background: var(--text-muted); flex-shrink: 0;
  }
  .link-btn {
    background: none; border: none; color: var(--accent); cursor: pointer;
    font-family: inherit; font-size: inherit; padding: 0; text-decoration: underline;
  }

  /* ── Character card strip (shown at ≥30rem) ── */
  .char-strip {
    display: none;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .char-card {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-left: 3px solid var(--border-surface);
    border-radius: 5px;
    padding: 0.4rem 0.65rem;
    display: flex; flex-direction: column; gap: 0.1rem;
    cursor: pointer; text-align: left;
    transition: border-color 0.15s, background 0.15s, box-shadow 0.2s;
    font-family: inherit;
    box-sizing: border-box;
  }
  .char-card:hover { border-color: var(--border-active); }
  .char-card--selected {
    border-color: var(--accent);
    background: var(--bg-active);
    box-shadow: 0 0 10px #cc222225, inset 0 0 12px #cc222210;
  }
  /* Resonance-colored left accent bar */
  .char-card[data-res="Phlegmatic"] { border-left-color: #3d6b88; }
  .char-card[data-res="Melancholy"] { border-left-color: #6a3d80; }
  .char-card[data-res="Choleric"]   { border-left-color: var(--accent-amber); }
  .char-card[data-res="Sanguine"]   { border-left-color: var(--accent); }
  .char-name { font-size: 0.8rem; color: var(--text-primary); font-weight: bold; }
  .char-source { font-size: 0.6rem; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.05em; }
  .char-clan { font-size: 0.7rem; color: var(--text-secondary); }
  .char-res  { font-size: 0.65rem; color: var(--accent); }

  /* ── Narrow compact selector (hidden at ≥30rem) ── */
  .char-selector-narrow { display: block; position: relative; margin-bottom: 0.75rem; }
  .selector-btn {
    width: 100%; background: var(--bg-raised); border: 1px solid var(--border-surface);
    border-radius: 5px; padding: 0.45rem 0.65rem;
    display: flex; align-items: center; gap: 0.45rem;
    cursor: pointer; font-family: inherit; text-align: left;
    box-sizing: border-box; transition: border-color 0.15s;
  }
  .selector-btn--active { border-color: var(--accent); background: var(--bg-active); }
  .sel-dot {
    width: 0.45rem; height: 0.45rem; border-radius: 50%;
    background: var(--accent); flex-shrink: 0;
  }
  .sel-name        { font-size: 0.8rem; color: var(--text-primary); font-weight: bold; flex: 1; }
  .sel-clan        { font-size: 0.7rem; color: var(--text-secondary); }
  .sel-placeholder { font-size: 0.78rem; color: var(--text-muted); flex: 1; }
  .sel-chevron     { color: var(--text-label); font-size: 0.75rem; flex-shrink: 0; transition: transform 0.2s; }
  .sel-chevron.open { transform: rotate(180deg); }

  .selector-backdrop {
    position: fixed; inset: 0; z-index: 10;
    background: rgba(0, 0, 0, 0.4); border: none; cursor: default;
  }
  .selector-dropdown {
    position: absolute; top: calc(100% + 0.3rem); left: 0; right: 0;
    background: var(--bg-input); border: 1px solid var(--border-active);
    border-radius: 6px; padding: 0.4rem; z-index: 20;
    box-shadow: 0 8px 24px rgba(0,0,0,0.7), 0 0 0 1px #cc222233;
    display: flex; flex-direction: column; gap: 0.25rem;
  }
  .dropdown-header {
    font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.08em;
    color: var(--text-label); padding: 0.2rem 0.3rem 0.35rem;
    border-bottom: 1px solid var(--border-faint); margin-bottom: 0.1rem;
  }
  .drop-item {
    background: var(--bg-card); border: 1px solid var(--border-card);
    border-radius: 4px; padding: 0.4rem 0.55rem;
    display: flex; align-items: center; gap: 0.5rem;
    cursor: pointer; text-align: left; font-family: inherit;
    width: 100%; box-sizing: border-box;
    transition: border-color 0.12s, background 0.12s;
  }
  .drop-item:hover { border-color: var(--border-surface); background: var(--bg-raised); }
  .drop-item--selected { border-color: var(--accent); background: var(--bg-active); }
  .drop-item-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.05rem; }
  .drop-name  { font-size: 0.78rem; color: var(--text-primary); font-weight: bold; }
  .drop-source { font-size: 0.6rem; color: var(--text-muted); margin-left: 0.3rem; font-weight: normal; }
  .drop-clan  { font-size: 0.65rem; color: var(--text-secondary); }
  .drop-res   { font-size: 0.62rem; color: var(--accent); }
  .drop-check { font-size: 0.75rem; color: var(--accent); flex-shrink: 0; }

  /* ── Roll button ── */
  .roll-area { display: flex; justify-content: center; margin-top: 0.5rem; }
  .roll-btn {
    padding: 0.75rem 2.5rem;
    background: var(--bg-active); border: 2px solid var(--border-active);
    color: var(--accent); font-size: 1.1rem; font-family: 'Georgia', serif;
    cursor: pointer; border-radius: 4px;
    transition: background 0.2s, box-shadow 0.2s; letter-spacing: 0.05em;
  }
  .roll-btn:hover:not(:disabled) { background: #5a0808; box-shadow: 0 0 16px #cc222244; }
  .roll-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ResultCard has its own margin-top: 1.5rem which double-stacks with
     steps-panel gap. Zero it out so the result sits flush in the flow. */
  .steps-panel > :global(.result-card) { margin-top: 0; }

  /* ── Apply button ── */
  .apply-row {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .apply-btn {
    padding: 0.45rem 1.2rem;
    background: var(--bg-sunken);
    border: 1.5px solid var(--border-surface);
    color: var(--accent-amber);
    font-size: 0.85rem; font-family: 'Georgia', serif;
    cursor: pointer; border-radius: 4px;
    transition: background 0.15s, border-color 0.15s, color 0.3s;
  }
  .apply-btn:hover:not(:disabled) {
    border-color: var(--accent-amber);
    background: var(--bg-raised);
  }
  .apply-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .apply-btn.applied {
    border-color: #4caf50;
    color: #4caf50;
    background: #0f2a0f;
  }
  .apply-btn.error {
    border-color: var(--accent);
    color: var(--accent);
  }

  /* ── History panel ── */
  .history-panel {
    background: var(--bg-card); border: 1px solid var(--border-card);
    border-radius: 6px; padding: 0.85rem 0.9rem;
    display: flex; flex-direction: column; max-height: 20rem;
  }

  /* ── Resonance probability visualization (unchanged) ── */
  .res-probs {
    margin-top: 0.75rem; padding-top: 0.65rem;
    border-top: 1px solid var(--border-faint);
    display: flex; flex-direction: column; gap: 0.5rem;
  }
  .res-bar {
    display: flex; height: 0.5rem; border-radius: 3px;
    overflow: hidden; gap: 1px; background: var(--bg-sunken);
  }
  .res-seg { height: 100%; transition: width 0.3s ease; border-radius: 1px; }
  .res-seg-phlegmatic { background: #3d6b88; }
  .res-seg-melancholy  { background: #6a3d80; }
  .res-seg-choleric    { background: var(--accent-amber); }
  .res-seg-sanguine    { background: var(--accent); }
  .res-legend { display: flex; gap: 0.75rem; flex-wrap: wrap; }
  .leg-item { display: flex; align-items: center; gap: 0.3rem; font-size: 0.7rem; transition: opacity 0.2s; }
  .leg-item.leg-zero { opacity: 0.3; }
  .leg-dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; flex-shrink: 0; }
  .leg-dot-phlegmatic { background: #3d6b88; }
  .leg-dot-melancholy  { background: #6a3d80; }
  .leg-dot-choleric    { background: var(--accent-amber); }
  .leg-dot-sanguine    { background: var(--accent); }
  .leg-name { color: var(--text-label); }
  .leg-pct  { color: var(--text-primary); font-weight: 700; min-width: 2.2rem; }

  /* ── Responsive breakpoints ── */

  /* ≥30rem: show card strip, hide narrow selector */
  @container resonance-page (min-width: 30rem) {
    .char-strip           { display: flex; }
    .char-selector-narrow { display: none; }
  }

  /* ≥42rem: side-by-side layout with history panel */
  @container resonance-page (min-width: 42rem) {
    .main-layout   { flex-direction: row; gap: 2rem; align-items: flex-start; }
    .steps-panel   { flex: 1; min-width: 0; }
    .history-panel {
      width: 15rem; flex-shrink: 0;
      position: sticky; top: 1rem;
      max-height: calc(100vh - 3rem);
    }
  }
</style>
