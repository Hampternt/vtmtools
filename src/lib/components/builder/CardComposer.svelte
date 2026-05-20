<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    spotlight: Snippet;
    cardRows: Snippet;
    queueLabel?: string;
    queueValues?: string[];
  }

  let { spotlight, cardRows, queueLabel = 'up next', queueValues = [] }: Props = $props();
</script>

<div class="composer">
  <div class="card-preview">
    <div class="card-preview-label">— card preview —</div>
    {@render cardRows()}
  </div>

  <div class="spotlight">
    {@render spotlight()}
  </div>

  {#if queueValues.length > 0}
    <div class="queue">
      <span class="queue-label">{queueLabel}</span>
      <span class="queue-list">
        {#each queueValues as v, i}
          {#if i > 0}<span class="queue-arrow">→</span>{/if}
          <span class="queue-item">{v}</span>
        {/each}
      </span>
    </div>
  {/if}
</div>

<style>
  .composer { display: flex; flex-direction: column; }
  .card-preview {
    padding: 0.85rem 0.9rem;
    background: linear-gradient(180deg, rgba(217,165,96,0.04) 0%, rgba(217,165,96,0.01) 100%);
    border-bottom: 1px solid var(--border-faint);
    display: flex; flex-direction: column; gap: 0.5rem;
  }
  .card-preview-label {
    font-size: 0.55rem; color: var(--text-muted);
    letter-spacing: 0.1em; text-transform: uppercase;
    text-align: center;
  }
  .spotlight {
    padding: 1.2rem 1rem;
    background: radial-gradient(ellipse at center, rgba(217, 165, 96, 0.08) 0%, transparent 65%);
    border-bottom: 1px solid var(--border-faint);
    display: flex; flex-direction: column; gap: 0.7rem;
    position: relative;
    min-height: 8rem;
  }
  .spotlight::before {
    content: ''; position: absolute; inset: 0.4rem;
    border: 1px solid rgba(217, 165, 96, 0.18);
    border-radius: 0.3rem; pointer-events: none;
  }
  .queue {
    padding: 0.4rem 0.8rem;
    display: flex; align-items: center; justify-content: space-between;
    background: rgba(0,0,0,0.2);
    font-size: 0.62rem;
  }
  .queue-label { color: var(--text-muted); letter-spacing: 0.08em; text-transform: uppercase; }
  .queue-list { display: flex; gap: 0.4rem; align-items: center; color: var(--text-secondary); }
  .queue-arrow { color: var(--text-muted); }
  .queue-item { color: var(--text-label); }
</style>
