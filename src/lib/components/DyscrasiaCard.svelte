<script lang="ts">
  import type { DyscrasiaEntry } from '../../types';

  const {
    entry,
    mode,
    cardstate = null,
    onselect,
    onedit,
    ondelete,
  }: {
    entry: DyscrasiaEntry;
    mode: 'manager' | 'acute';
    cardstate?: 'rolled' | 'selected' | null;
    onselect?: () => void;
    onedit?: () => void;
    ondelete?: () => void;
  } = $props();

  let descEl: HTMLElement | undefined = $state(undefined);
  let overflowsDesc = $state(false);
  let isExpanded = $state(false);

  $effect(() => {
    if (!descEl) return;
    const check = () => {
      overflowsDesc = (descEl?.scrollHeight ?? 0) > (descEl?.offsetHeight ?? 0) + 10;
    };
    check();
    const ro = new ResizeObserver(check);
    ro.observe(descEl);
    return () => ro.disconnect();
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="card"
  class:rolled={cardstate === 'rolled'}
  class:selected={cardstate === 'selected'}
  class:clickable={mode === 'acute'}
  onclick={mode === 'acute' ? onselect : undefined}
  onkeydown={mode === 'acute' ? (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect?.(); } } : undefined}
  role={mode === 'acute' ? 'button' : undefined}
  tabindex={mode === 'acute' ? 0 : undefined}
>
  {#if cardstate === 'rolled'}
    <span class="state-badge">rolled</span>
  {:else if cardstate === 'selected'}
    <span class="state-badge selected-badge">selected ✓</span>
  {/if}

  {#if mode === 'manager'}
    <div
      class="type-badge"
      class:type-phlegmatic={entry.resonanceType.toLowerCase() === 'phlegmatic'}
      class:type-melancholy={entry.resonanceType.toLowerCase() === 'melancholy'}
      class:type-choleric={entry.resonanceType.toLowerCase() === 'choleric'}
      class:type-sanguine={entry.resonanceType.toLowerCase() === 'sanguine'}
    >
      {entry.resonanceType}
    </div>
  {/if}

  <div class="name">{entry.name}</div>

  <div
    class="desc"
    class:clipped={overflowsDesc && !isExpanded}
    bind:this={descEl}
  >
    {entry.description}
  </div>

  {#if overflowsDesc && !isExpanded}
    <button
      class="show-more"
      onclick={(e) => { e.stopPropagation(); isExpanded = true; }}
    >
      show more ▾
    </button>
  {/if}

  <div class="footer">
    <span class="bonus">{entry.bonus}</span>
    {#if mode === 'manager'}
      {#if entry.isCustom}
        <div class="actions">
          <button class="action-btn edit" onclick={(e) => { e.stopPropagation(); onedit?.(); }}>✎ Edit</button>
          <button class="action-btn del"  onclick={(e) => { e.stopPropagation(); ondelete?.(); }}>✕</button>
        </div>
      {:else}
        <span class="builtin-badge">built-in</span>
      {/if}
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    padding: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    break-inside: avoid;
    margin-bottom: 0.75rem;
    width: 100%;
    position: relative;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .card.clickable { cursor: pointer; }
  .card.clickable:hover { border-color: var(--border-surface); box-shadow: 0 2px 14px #0009; }
  .card:not(.clickable):hover { border-color: var(--border-surface); }

  .card.rolled {
    border-color: var(--accent);
    background: #1a0808;
    box-shadow: 0 0 12px #cc222233, inset 0 0 18px #cc22220a;
  }
  .card.selected {
    border-color: var(--accent-amber);
    background: #1a1206;
    box-shadow: 0 0 12px #cc992233, inset 0 0 18px #cc99220a;
  }

  .state-badge {
    position: absolute;
    top: 0.5rem;
    right: 0.6rem;
    font-size: 0.52rem;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--accent);
    opacity: 0.8;
  }
  .state-badge.selected-badge { color: var(--accent-amber); opacity: 0.9; }

  .type-badge {
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.09em;
    font-weight: 600;
  }
  .type-badge.type-phlegmatic { color: #7090c0; }
  .type-badge.type-melancholy { color: #9070b0; }
  .type-badge.type-choleric   { color: var(--accent); }
  .type-badge.type-sanguine   { color: var(--accent-amber); }

  .name {
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    line-height: 1.2;
    padding-right: 2rem;
  }

  .desc {
    font-size: 0.72rem;
    color: var(--text-secondary);
    line-height: 1.5;
    flex: 1;
  }
  .desc.clipped {
    max-height: 260px;
    overflow: hidden;
    mask-image: linear-gradient(to bottom, black 70%, transparent 100%);
    -webkit-mask-image: linear-gradient(to bottom, black 70%, transparent 100%);
  }

  .show-more {
    align-self: flex-start;
    font-size: 0.62rem;
    color: var(--text-ghost);
    border: none;
    background: none;
    padding: 0;
    cursor: pointer;
    transition: color 0.15s;
    margin-top: -0.1rem;
  }
  .show-more:hover { color: var(--text-label); }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 0.2rem;
    padding-top: 0.4rem;
    border-top: 1px solid var(--border-faint);
    gap: 0.5rem;
  }
  .bonus { font-size: 0.64rem; color: var(--text-label); flex: 1; }

  .actions { display: flex; gap: 0.3rem; flex-shrink: 0; }
  .action-btn {
    font-size: 0.6rem;
    padding: 0.18rem 0.38rem;
    border-radius: 3px;
    border: 1px solid;
    cursor: pointer;
    background: none;
    transition: background 0.15s, box-shadow 0.15s, transform 0.1s;
  }
  .action-btn:active { transform: scale(0.87); }
  .action-btn.edit { border-color: #4a3a1a; color: var(--accent-amber); }
  .action-btn.edit:hover { background: #1a1206; box-shadow: 0 0 6px #cc992244; }
  .action-btn.del  { border-color: #3a1010; color: var(--accent); }
  .action-btn.del:hover  { background: #1a0505; box-shadow: 0 0 6px #cc222244; }

  .builtin-badge {
    font-size: 0.55rem;
    color: var(--text-ghost);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.1rem 0.3rem;
    flex-shrink: 0;
  }
</style>
