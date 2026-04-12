<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ResonanceSlider from '$lib/components/ResonanceSlider.svelte';
  import TemperamentConfigComponent from '$lib/components/TemperamentConfig.svelte';
  import ResultCard from '$lib/components/ResultCard.svelte';
  import RollHistory from '$lib/components/RollHistory.svelte';
  import { publishEvent } from '../store/toolEvents';
  import type { RollConfig, ResonanceRollResult, HistoryEntry } from '../types';

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

  // ── Resonance probability math (mirrors weighted_resonance_pick in dice.rs) ──
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

  async function roll() {
    rolling = true;
    result = null;
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
</script>

<div class="page">
  <h1 class="title">Resonance Roller</h1>
  <p class="subtitle">Configure the feeding conditions, then roll.</p>

  <div class="main-layout">
    <!-- LEFT: Configuration + result -->
    <div class="steps-panel">
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

        <!-- Live probability breakdown -->
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

      <div class="roll-area">
        <button class="roll-btn" onclick={roll} disabled={rolling}>
          {rolling ? 'Rolling…' : '⚀ Roll'}
        </button>
      </div>

      {#if result}
        <ResultCard {result} />
      {/if}
    </div>

    <!-- RIGHT: Roll history -->
    <div class="history-panel">
      <RollHistory entries={rollHistory} />
    </div>
  </div>
</div>

<style>
  .page {
    width: 100%;
    /* container queries let children respond to this element's width,
       not the viewport — handles any sidebar width or window shape */
    container-type: inline-size;
    container-name: resonance-page;
  }
  .title { color: var(--accent); font-size: 1.8rem; margin-bottom: 0.25rem; }
  .subtitle { color: var(--text-secondary); font-size: 0.9rem; margin-bottom: 1.5rem; }

  /* ── Stacked layout (default / narrow) ── */
  .main-layout {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    align-items: stretch;
  }
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
  .roll-area { display: flex; justify-content: center; }
  .roll-btn {
    padding: 0.75rem 2.5rem;
    background: var(--bg-active);
    border: 2px solid var(--border-active);
    color: var(--accent);
    font-size: 1.1rem;
    font-family: 'Georgia', serif;
    cursor: pointer;
    border-radius: 4px;
    transition: background 0.2s, box-shadow 0.2s;
    letter-spacing: 0.05em;
  }
  .roll-btn:hover:not(:disabled) {
    background: #5a0808;
    box-shadow: 0 0 16px #cc222244;
  }
  .roll-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .history-panel {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 6px;
    padding: 0.85rem 0.9rem;
    display: flex;
    flex-direction: column;
    /* Stacked: limit height so it doesn't swamp the page */
    max-height: 20rem;
  }

  /* ── Resonance probability visualization ── */
  .res-probs {
    margin-top: 0.75rem;
    padding-top: 0.65rem;
    border-top: 1px solid var(--border-faint);
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  /* Stacked proportional bar */
  .res-bar {
    display: flex;
    height: 0.5rem;
    border-radius: 3px;
    overflow: hidden;
    gap: 1px;
    background: var(--bg-sunken); /* shows as gap colour */
  }
  .res-seg {
    height: 100%;
    transition: width 0.3s ease;
    border-radius: 1px;
  }
  .res-seg-phlegmatic { background: #3d6b88; }
  .res-seg-melancholy  { background: #6a3d80; }
  .res-seg-choleric    { background: var(--accent-amber); }
  .res-seg-sanguine    { background: var(--accent); }

  /* Legend row */
  .res-legend {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .leg-item {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.7rem;
    transition: opacity 0.2s;
  }
  .leg-item.leg-zero { opacity: 0.3; }
  .leg-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .leg-dot-phlegmatic { background: #3d6b88; }
  .leg-dot-melancholy  { background: #6a3d80; }
  .leg-dot-choleric    { background: var(--accent-amber); }
  .leg-dot-sanguine    { background: var(--accent); }
  .leg-name { color: var(--text-label); }
  .leg-pct  { color: var(--text-primary); font-weight: 700; min-width: 2.2rem; }

  /* ── Wide layout: side-by-side when container >= 42rem ── */
  @container resonance-page (min-width: 42rem) {
    .main-layout {
      flex-direction: row;
      gap: 2rem;
      align-items: flex-start;
    }
    .steps-panel { flex: 1; min-width: 0; }
    .history-panel {
      width: 15rem;
      flex-shrink: 0;
      position: sticky;
      top: 1rem;
      max-height: calc(100vh - 3rem);
    }
  }
</style>
