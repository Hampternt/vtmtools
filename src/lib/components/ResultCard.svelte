<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import AcutePanel from '$lib/components/AcutePanel.svelte';
  import type { ResonanceRollResult, DyscrasiaEntry } from '../../types';

  const { result }: { result: ResonanceRollResult } = $props();

  let confirmedDyscrasia: DyscrasiaEntry | null = $state(null);
  let acuteConfirmed = $state(false);

  // Reset when a new result arrives.
  // Reading result makes it a dependency; confirmedDyscrasia/acuteConfirmed
  // are not read here so no cycle.
  $effect(() => {
    if (result) {
      confirmedDyscrasia = null;
      acuteConfirmed = false;
    }
  });

  async function exportToMd() {
    await invoke('export_result_to_md', { result, dyscrasia: confirmedDyscrasia });
  }
</script>

<div class="result-card">
  <div class="result-row">
    <span class="label">Temperament</span>
    <span class="value {result.isAcute ? 'acute' : result.temperament}">
      {result.isAcute ? 'ACUTE' : result.temperament.toUpperCase()}
      <span class="dice-info">
        (rolled {result.temperamentDie}
        {#if result.temperamentDice.length > 1}
          from [{result.temperamentDice.join(', ')}]
        {/if})
      </span>
    </span>
  </div>

  {#if result.resonanceType}
    <div class="result-row">
      <span class="label">Resonance</span>
      <span class="value">{result.resonanceType}</span>
    </div>
  {/if}

  {#if acuteConfirmed && confirmedDyscrasia}
    <div class="result-row">
      <span class="label">Dyscrasia</span>
      <span class="value" style="color: var(--accent-amber)">{confirmedDyscrasia.name}</span>
    </div>
  {/if}

  <div class="card-footer">
    <button class="export-btn" onclick={exportToMd}>Export to .md</button>
  </div>
</div>

{#if result.isAcute && !acuteConfirmed && result.resonanceType}
  <AcutePanel
    resonanceType={result.resonanceType}
    initialDyscrasia={result.dyscrasia}
    onconfirm={(d) => { confirmedDyscrasia = d; acuteConfirmed = true; }}
  />
{/if}

<style>
  .result-card {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 6px;
    padding: 1.25rem;
    margin-top: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .result-row { display: flex; align-items: baseline; gap: 1rem; }
  .label { width: 6.875rem; color: var(--text-label); font-size: 0.85rem; flex-shrink: 0; }
  .value { font-size: 1rem; color: var(--text-primary); font-weight: 600; }
  .value.negligible { color: var(--temp-negligible); }
  .value.fleeting   { color: var(--accent-amber); }
  .value.intense    { color: var(--accent); }
  .value.acute      { color: var(--accent-bright); text-shadow: 0 0 8px #cc222288; }
  .dice-info { font-size: 0.8rem; color: var(--text-muted); font-weight: 400; margin-left: 0.5rem; }
  .card-footer { display: flex; justify-content: flex-end; border-top: 1px solid var(--border-surface); padding-top: 0.75rem; }
  .export-btn {
    padding: 0.3rem 0.8rem;
    background: #1a1a0d;
    border: 1px solid #6a5a20;
    color: var(--accent-amber);
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.8rem;
  }
</style>
