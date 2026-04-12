<script lang="ts">
  import { slide } from 'svelte/transition';
  import type { HistoryEntry } from '../../types';

  const { entries }: { entries: HistoryEntry[] } = $props();

  function formatTime(d: Date): string {
    const diff = Math.floor((Date.now() - d.getTime()) / 1000);
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  const TEMP_COLORS: Record<string, string> = {
    negligible: '#555',
    fleeting: '#cc9922',
    intense: '#cc2222',
  };
</script>

<div class="history">
  <div class="hist-header">
    <span class="hist-title">Roll Log</span>
    {#if entries.length > 0}
      <span class="hist-count">{entries.length}</span>
    {/if}
  </div>

  {#if entries.length === 0}
    <div class="empty">No rolls recorded yet.</div>
  {:else}
    <div class="entries">
      {#each entries as entry (entry.id)}
        <div class="entry" in:slide={{ duration: 180, axis: 'y' }}>
          <div
            class="indicator"
            style="background:{TEMP_COLORS[entry.result.temperament]}"
          ></div>
          <div class="body">
            <div class="row-main">
              <span
                class="temp"
                style="color:{TEMP_COLORS[entry.result.temperament]}"
              >{entry.result.temperament}</span>
              {#if entry.result.resonanceType}
                <span class="resonance">{entry.result.resonanceType}</span>
              {:else}
                <span class="none">no resonance</span>
              {/if}
              {#if entry.result.isAcute}
                <span class="acute">ACUTE</span>
              {/if}
            </div>
            <div class="row-meta">
              <span class="dice-info">
                rolled {entry.result.temperamentDie}
                {#if entry.result.temperamentDice.length > 1}
                  of [{entry.result.temperamentDice.join(', ')}]
                {/if}
              </span>
              <span class="time">{formatTime(entry.timestamp)}</span>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .history {
    display: flex; flex-direction: column; gap: 0.5rem;
    height: 100%;
  }

  .hist-header {
    display: flex; align-items: center; gap: 0.5rem;
    padding-bottom: 0.4rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .hist-title {
    font-size: 0.68rem; text-transform: uppercase;
    letter-spacing: 0.1em; color: var(--text-label);
    flex: 1;
  }
  .hist-count {
    font-size: 0.65rem; color: var(--text-secondary);
    background: #1e1208; border: 1px solid #3a2810;
    border-radius: 10px; padding: 0 0.4rem;
    line-height: 1.6;
  }

  .empty {
    font-size: 0.78rem; color: var(--text-secondary);
    text-align: center; padding: 2rem 0;
    font-style: italic;
  }

  .entries {
    display: flex; flex-direction: column; gap: 1px;
    overflow-y: auto; flex: 1;
  }
  .entries::-webkit-scrollbar { width: 3px; }
  .entries::-webkit-scrollbar-track { background: transparent; }
  .entries::-webkit-scrollbar-thumb { background: #2a1a1a; border-radius: 2px; }

  .entry {
    display: flex; gap: 0.6rem; align-items: flex-start;
    padding: 0.55rem 0.5rem;
    background: var(--bg-sunken);
    border-radius: 3px;
    transition: background 0.1s;
  }
  .entry:hover { background: #150808; }

  .indicator {
    width: 3px; border-radius: 2px;
    align-self: stretch; flex-shrink: 0;
    min-height: 1.75rem;
  }

  .body { display: flex; flex-direction: column; gap: 0.2rem; min-width: 0; }

  .row-main { display: flex; align-items: baseline; gap: 0.4rem; flex-wrap: wrap; }
  .temp { font-size: 0.78rem; font-weight: 700; text-transform: capitalize; }
  .resonance { font-size: 0.78rem; color: var(--text-primary); }
  .none { font-size: 0.75rem; color: var(--text-ghost); font-style: italic; }
  .acute {
    font-size: 0.6rem; letter-spacing: 0.08em;
    color: var(--accent-bright); background: #3a0000;
    border: 1px solid var(--accent); border-radius: 2px;
    padding: 0 0.3rem; line-height: 1.6;
  }

  .row-meta { display: flex; gap: 0.5rem; align-items: baseline; }
  .dice-info { font-size: 0.68rem; color: var(--text-muted); }
  .time { font-size: 0.65rem; color: var(--text-secondary); margin-left: auto; }
</style>
