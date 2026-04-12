<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ResonanceSlider from '$lib/components/ResonanceSlider.svelte';
  import TemperamentConfigComponent from '$lib/components/TemperamentConfig.svelte';
  import ResultCard from '$lib/components/ResultCard.svelte';
  import { publishEvent } from '../store/toolEvents';
  import type { RollConfig, ResonanceRollResult } from '../types';

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

  const summary = $derived(buildSummary(config));

  function buildSummary(c: RollConfig) {
    const t = c.temperament;
    const diceLabel = t.diceCount === 1
      ? '1 die (standard)'
      : `${t.diceCount} dice — take ${t.takeHighest ? 'highest' : 'lowest'}`;
    return {
      dice: diceLabel,
      thresholds: `Neg 1–${t.negligibleMax} / Flee ${t.negligibleMax + 1}–${t.fleetingMax} / Int ${t.fleetingMax + 1}–10`,
    };
  }

  async function roll() {
    rolling = true;
    result = null;
    try {
      result = await invoke<ResonanceRollResult>('roll_resonance', { config });
      if (result) {
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
    <!-- LEFT: Step wizard -->
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

    <!-- RIGHT: Live summary -->
    <div class="summary-panel">
      <h3>Current Settings</h3>
      <div class="summary-row">
        <span class="sum-label">Temperament</span>
        <span class="sum-value">{summary.dice}</span>
      </div>
      <div class="summary-row">
        <span class="sum-label">Thresholds</span>
        <span class="sum-value">{summary.thresholds}</span>
      </div>
      <div class="summary-row">
        <span class="sum-label">Resonance odds</span>
        <div class="sum-weights">
          {#each Object.entries(config.weights) as [type, level]}
            <span class="weight-pill {level !== 'neutral' ? 'modified' : ''}">
              {type.charAt(0).toUpperCase() + type.slice(1)}: {level}
            </span>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .page { max-width: 900px; }
  .title { color: #cc2222; font-size: 1.8rem; margin-bottom: 0.25rem; }
  .subtitle { color: #6a5a40; font-size: 0.9rem; margin-bottom: 1.5rem; }
  .main-layout { display: flex; gap: 2rem; align-items: flex-start; }
  .steps-panel { flex: 1; display: flex; flex-direction: column; gap: 1.5rem; }
  .step {
    background: #120808;
    border: 1px solid #2a1010;
    border-radius: 6px;
    padding: 1rem 1.25rem;
  }
  h3 { color: #a09070; font-size: 0.9rem; text-transform: uppercase;
       letter-spacing: 0.08em; margin: 0 0 0.75rem; }
  .roll-area { display: flex; justify-content: center; }
  .roll-btn {
    padding: 0.75rem 2.5rem;
    background: #3a0808;
    border: 2px solid #cc2222;
    color: #cc2222;
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
  .summary-panel {
    width: 220px;
    flex-shrink: 0;
    background: #120808;
    border: 1px solid #2a1010;
    border-radius: 6px;
    padding: 1rem;
    position: sticky;
    top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .summary-row { display: flex; flex-direction: column; gap: 0.2rem; }
  .sum-label { font-size: 0.75rem; color: #6a5a40; text-transform: uppercase; letter-spacing: 0.06em; }
  .sum-value { font-size: 0.85rem; color: #d4c5a9; }
  .sum-weights { display: flex; flex-direction: column; gap: 0.2rem; margin-top: 0.2rem; }
  .weight-pill { font-size: 0.8rem; color: #6a6a6a; }
  .weight-pill.modified { color: #cc2222; }
</style>
