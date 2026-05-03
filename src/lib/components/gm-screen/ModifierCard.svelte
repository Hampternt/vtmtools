<script lang="ts">
  import type { CharacterModifier, ModifierEffect, FoundryItemBonus } from '../../../types';

  interface Props {
    /**
     * Card data. For an advantage-derived virtual card (no DB row yet) the
     * caller passes a synthesized object with id=0 and the displayable
     * name/description from the Foundry feature item.
     */
    modifier: CharacterModifier;
    /** Marks an advantage-derived card not yet materialized — UI shows asterisk */
    isVirtual?: boolean;
    /** Marks a stale card whose source merit was deleted — UI shows badge */
    isStale?: boolean;
    /**
     * Sheet-attached bonuses (system.bonuses[]) from the source Foundry
     * feature item, when the card is advantage-bound. Distinct from
     * modifier.effects (GM Screen annotations) — these come from the actor
     * sheet directly and render even on virtual cards.
     */
    bonuses?: FoundryItemBonus[];
    onToggleActive: () => void;
    onOpenEditor: (anchor: HTMLElement) => void;
    onHide: () => void;
  }

  let { modifier, isVirtual = false, isStale = false, bonuses = [], onToggleActive, onOpenEditor, onHide }: Props = $props();

  let cogEl: HTMLButtonElement | undefined = $state();

  function summarize(e: ModifierEffect): string {
    if (e.kind === 'note') return e.note ?? 'note';
    const sign = (e.delta ?? 0) >= 0 ? '+' : '';
    const scope = e.scope ? `${e.scope} ` : '';
    const label = e.kind === 'pool' ? 'dice' : 'difficulty';
    const paths = (e.paths ?? []).filter(p => p !== '');
    const pathSuffix = paths.length > 0 ? ` → ${paths.join(', ')}` : '';
    return `${scope}${sign}${e.delta ?? 0} ${label}${pathSuffix}`;
  }

  /** "attributes.strength" → "Strength". Last dot-segment, capitalized. */
  function prettyPath(p: string): string {
    const last = p.split('.').pop() ?? p;
    return last.charAt(0).toUpperCase() + last.slice(1);
  }

  function summarizeBonus(b: FoundryItemBonus): string {
    const sign = b.value >= 0 ? '+' : '';
    const stats = b.paths.map(prettyPath).join(', ');
    return stats ? `${sign}${b.value} ${stats}` : `${sign}${b.value}`;
  }
</script>

<div
  class="modifier-card"
  data-active={modifier.isActive ? 'true' : 'false'}
  data-hidden={modifier.isHidden ? 'true' : 'false'}
>
  <div class="head">
    <span class="name">
      {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}
      {#if isStale}<span class="stale" title="Source merit removed">stale</span>{/if}
    </span>
    <button
      bind:this={cogEl}
      class="cog"
      title="Edit effects"
      onclick={() => cogEl && onOpenEditor(cogEl)}
    >⚙</button>
  </div>
  {#if bonuses.length > 0}
    <div class="bonuses" title="Sheet-attached bonuses">
      {#each bonuses as b}
        <p class="bonus">
          <span class="bonus-value">{summarizeBonus(b)}</span>
          {#if b.source}<span class="bonus-source">{b.source}</span>{/if}
        </p>
      {/each}
    </div>
  {/if}
  <div class="effects">
    {#if modifier.effects.length === 0}
      <p class="no-effect">(no effect)</p>
    {:else}
      {#each modifier.effects as e}
        <p class="effect">{summarize(e)}</p>
      {/each}
    {/if}
  </div>
  {#if modifier.tags.length > 0}
    <div class="tags">
      {#each modifier.tags as t}<span class="tag">#{t}</span>{/each}
    </div>
  {/if}
  <div class="foot">
    <button
      class="toggle"
      class:on={modifier.isActive}
      onclick={onToggleActive}
    >{modifier.isActive ? 'ON' : 'OFF'}</button>
    {#if !modifier.isHidden}
      <button class="hide" title="Hide card" onclick={onHide}>×</button>
    {/if}
  </div>
</div>

<style>
  /* Per-card positioning variables — the parent .modifier-row provides
     --card-width / --card-overlap / --card-shift-delta / --cards (spec §7.2). */
  .modifier-card {
    --card-i: sibling-index();
    --base-x: calc((var(--card-i) - 1) * var(--card-width) * (1 - var(--card-overlap)));
    --shift-x: 0rem;
    --centre: calc((var(--cards) + 1) / 2);
    --distance: max(calc(var(--card-i) - var(--centre)), calc(var(--centre) - var(--card-i)));

    position: absolute;
    left: 0;
    top: 0;
    width: var(--card-width);
    height: 100%;
    padding: 0.6rem 0.75rem;
    box-sizing: border-box;            /* ARCH §6: no global reset */
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.625rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    z-index: calc(100 - var(--distance));

    transform: translateX(calc(var(--base-x) + var(--shift-x)));
    transition: transform var(--card-trans-duration) var(--card-trans-easing),
                box-shadow var(--card-trans-duration) var(--card-trans-easing),
                border-color 200ms ease;
  }

  .modifier-card:hover {
    z-index: 100;
    transform: translateX(calc(var(--base-x) + var(--shift-x))) translateY(-0.75rem) translateZ(20px);
    box-shadow: 0 1.25rem 2rem -0.5rem var(--accent);
  }

  /* neighbour-shift cascade — cards AFTER hovered slide right */
  .modifier-card:hover + :global(.modifier-card)                                              { --shift-x: calc(var(--card-shift-delta) * 3); }
  .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card)                    { --shift-x: calc(var(--card-shift-delta) * 2); }
  .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card) {
    --shift-x: calc(var(--card-shift-delta) * 1);
  }
  /* neighbour-shift cascade — cards BEFORE hovered slide left, via :has() */
  .modifier-card:has(+ :global(.modifier-card:hover))                                                                       { --shift-x: calc(var(--card-shift-delta) * -3); }
  .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card:hover))                                             { --shift-x: calc(var(--card-shift-delta) * -2); }
  .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card:hover))                   { --shift-x: calc(var(--card-shift-delta) * -1); }

  .modifier-card[data-active="true"] {
    border-color: var(--accent-bright);
    background: var(--bg-active);
  }
  .modifier-card[data-hidden="true"] {
    opacity: 0.45;
    filter: saturate(0.6);
  }

  .head { display: flex; align-items: center; justify-content: space-between; gap: 0.4rem; }
  .name { font-size: 0.85rem; color: var(--text-primary); font-weight: 500; }
  .virtual-mark { color: var(--accent-amber); margin-left: 0.15rem; }
  .stale { font-size: 0.65rem; color: var(--accent-amber); margin-left: 0.4rem; }
  .cog {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .modifier-card:hover .cog,
  .cog:focus { opacity: 1; }

  .bonuses { display: flex; flex-direction: column; gap: 0.1rem; }
  .bonus { font-size: 0.65rem; margin: 0; color: var(--text-secondary); display: flex; gap: 0.4rem; align-items: baseline; flex-wrap: wrap; }
  .bonus-value { font-weight: 500; color: var(--accent-bright); }
  .bonus-source { font-size: 0.6rem; color: var(--text-muted); font-style: italic; }

  .effects { display: flex; flex-direction: column; gap: 0.15rem; }
  .effect, .no-effect { font-size: 0.7rem; margin: 0; color: var(--text-secondary); }
  .no-effect { color: var(--text-muted); font-style: italic; }

  .tags { display: flex; flex-wrap: wrap; gap: 0.2rem; }
  .tag { font-size: 0.65rem; color: var(--text-muted); }

  .foot { display: flex; justify-content: space-between; align-items: center; margin-top: auto; }
  .toggle {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 999px;
    padding: 0.15rem 0.55rem;
    font-size: 0.65rem;
    cursor: pointer;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .toggle.on {
    background: var(--accent);
    color: var(--text-primary);
    border-color: var(--accent-bright);
  }
  .hide {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 0.85rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .modifier-card:hover .hide,
  .hide:focus { opacity: 1; }

  @media (prefers-reduced-motion: reduce) {
    .modifier-card {
      transition: none;
    }
    .modifier-card:hover {
      transform: translateX(calc(var(--base-x) + var(--shift-x)));
    }
  }
</style>
