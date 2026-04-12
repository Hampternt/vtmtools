<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { ResonanceRollResult, DyscrasiaEntry } from '../../types';

  const { result }: { result: ResonanceRollResult } = $props();

  let dyscrasias: DyscrasiaEntry[] = $state([]);
  let selectedDyscrasia: DyscrasiaEntry | null = $state(null);
  let loadingDyscrasias = $state(false);

  $effect(() => {
    if (result.isAcute && result.resonanceType) {
      loadDyscrasias(result.resonanceType);
    }
  });

  async function loadDyscrasias(rtype: string) {
    loadingDyscrasias = true;
    dyscrasias = await invoke<DyscrasiaEntry[]>('list_dyscrasias', { resonanceType: rtype });
    loadingDyscrasias = false;
  }

  async function rollRandomDyscrasia() {
    selectedDyscrasia = await invoke<DyscrasiaEntry | null>('roll_random_dyscrasia', {
      resonanceType: result.resonanceType
    });
  }

  async function exportToMd() {
    await invoke('export_result_to_md', { result, dyscrasia: selectedDyscrasia });
  }
</script>

<div class="result-card">
  <div class="result-row">
    <span class="label">Temperament</span>
    <span class="value {result.temperament}">
      {result.temperament.toUpperCase()}
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

  {#if result.acuteDie !== null && result.acuteDie !== undefined}
    <div class="result-row">
      <span class="label">Acute check</span>
      <span class="value {result.isAcute ? 'acute' : ''}">
        {result.isAcute ? 'ACUTE' : 'Not Acute'} (rolled {result.acuteDie})
      </span>
    </div>
  {/if}

  {#if result.isAcute}
    <div class="dyscrasia-section">
      <span class="label">Dyscrasia</span>
      <div class="dyscrasia-actions">
        <button class="action-btn" onclick={rollRandomDyscrasia} disabled={loadingDyscrasias}>
          Roll randomly
        </button>
        <select
          class="pick-select"
          onchange={(e) => {
            const id = parseInt((e.target as HTMLSelectElement).value);
            selectedDyscrasia = dyscrasias.find(d => d.id === id) ?? null;
          }}
        >
          <option value="">— Pick manually —</option>
          {#each dyscrasias as d}
            <option value={d.id}>{d.name}</option>
          {/each}
        </select>
      </div>

      {#if selectedDyscrasia}
        <div class="dyscrasia-detail">
          <strong>{selectedDyscrasia.name}</strong>
          <p>{selectedDyscrasia.description}</p>
          <span class="bonus">{selectedDyscrasia.bonus}</span>
        </div>
      {/if}
    </div>
  {/if}

  <div class="card-footer">
    <button class="export-btn" onclick={exportToMd}>Export to .md</button>
  </div>
</div>

<style>
  .result-card {
    background: #1a0d0d;
    border: 1px solid #3a1a1a;
    border-radius: 6px;
    padding: 1.25rem;
    margin-top: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .result-row { display: flex; align-items: baseline; gap: 1rem; }
  .label { width: 110px; color: #a09070; font-size: 0.85rem; flex-shrink: 0; }
  .value { font-size: 1rem; color: #d4c5a9; font-weight: 600; }
  .value.negligible { color: #6a6a6a; }
  .value.fleeting { color: #cc9922; }
  .value.intense { color: #cc2222; }
  .value.acute { color: #ff4444; text-shadow: 0 0 8px #cc222288; }
  .dice-info { font-size: 0.8rem; color: #666; font-weight: 400; margin-left: 0.5rem; }
  .dyscrasia-section { display: flex; flex-direction: column; gap: 0.5rem; border-top: 1px solid #3a1a1a; padding-top: 0.75rem; }
  .dyscrasia-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
  .action-btn {
    padding: 0.3rem 0.8rem;
    background: #3a0808;
    border: 1px solid #cc2222;
    color: #cc2222;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.85rem;
  }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .pick-select {
    background: #1a0d0d;
    border: 1px solid #3a1a1a;
    color: #d4c5a9;
    padding: 0.25rem;
    border-radius: 3px;
  }
  .dyscrasia-detail {
    background: #0d0505;
    border: 1px solid #2a0a0a;
    border-radius: 4px;
    padding: 0.75rem;
  }
  .dyscrasia-detail strong { color: #cc2222; }
  .dyscrasia-detail p { color: #a09070; font-size: 0.85rem; margin: 0.4rem 0; }
  .bonus { color: #cc9922; font-size: 0.85rem; }
  .card-footer { display: flex; justify-content: flex-end; border-top: 1px solid #3a1a1a; padding-top: 0.75rem; }
  .export-btn {
    padding: 0.3rem 0.8rem;
    background: #1a1a0d;
    border: 1px solid #6a5a20;
    color: #cc9922;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.8rem;
  }
</style>
