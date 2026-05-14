<script lang="ts">
  import type { CharacterModifier, ModifierEffect, FoundryItemBonus } from '../../../types';
  import DragSource from '../dnd/DragSource.svelte';
  import type { DragSource as DragSourceType } from '../../dnd/types';
  import CardDragHandle from './CardDragHandle.svelte';
  import CardContextMenu, { type CardAction } from './CardContextMenu.svelte';
  import { dndStore } from '../../dnd/store.svelte';

  interface Props {
    /**
     * Card data. For an advantage-derived virtual card (no DB row yet) the
     * caller passes a synthesized object with id=0 and the displayable
     * name/description from the Foundry feature item.
     */
    modifier: CharacterModifier;
    /** Marks an advantage-derived card not yet materialized — UI shows asterisk */
    isVirtual?: boolean;
    /**
     * Sheet-attached bonuses (system.bonuses[]) from the source Foundry
     * feature item, when the card is advantage-bound. Distinct from
     * modifier.effects (GM Screen annotations) — these come from the actor
     * sheet directly and render even on virtual cards.
     */
    bonuses?: FoundryItemBonus[];
    /**
     * Conditional bonuses (activeWhen.check != 'always') skipped from the
     * read-through. Rendered as a small "(N conditionals)" badge with a
     * tooltip listing the labels + their `activeWhen.check` reasons.
     */
    conditionalsSkipped?: FoundryItemBonus[];
    /** True when this card is push-to-Foundry-eligible: Foundry source,
     *  advantage binding, materialized, with at least one pool effect. */
    canPush?: boolean;
    onPush?: () => void;
    /** True for materialized, advantage-bound cards on Foundry sources. */
    canReset?: boolean;
    onReset?: () => void;
    onToggleActive: () => void;
    onOpenEditor: (anchor: HTMLElement) => void;
    onHide: () => void;
    /**
     * Hard-delete handler for free-bound cards. Distinct from onHide — when
     * present, the card renders a 🗑 trash button next to ×. Caller is
     * responsible for the confirm() dialog before invoking. Undefined for
     * advantage-bound cards (their lifecycle is owned by the live Foundry
     * data, not the GM).
     */
    onDelete?: () => void;
    /**
     * Live-looked-up name of the originating status template, when the card
     * has `originTemplateId` set and the template still exists. Null clears
     * the provenance subtitle. Looked up by parent (CharacterRow) — we don't
     * read the templates store here.
     */
    originTemplateName?: string | null;
    /**
     * True when this materialized modifier is a saved Foundry override
     * (created via "Save as local override"). Drives the yellow origin
     * asterisk that signals "this card's data comes from a saved local
     * copy that supersedes the live Foundry read-through".
     */
    showOverride?: boolean;
    /**
     * "Save as local override" handler. When set (i.e. on virtual cards),
     * renders the save-as-override button. Clicking creates a
     * CharacterModifier with effects mirroring the current always-active
     * bonuses + captures their source labels.
     */
    onSaveAsOverride?: () => void;
  }

  let {
    modifier, isVirtual = false, bonuses = [],
    conditionalsSkipped = [],
    canPush = false, onPush,
    canReset = false, onReset,
    onToggleActive, onOpenEditor, onHide, onDelete,
    originTemplateName = null,
    showOverride = false,
    onSaveAsOverride,
  }: Props = $props();

  let cardEl: HTMLDivElement | undefined = $state();

  let ctxOpen = $state(false);
  let ctxAnchor = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  function handleContextMenu(e: MouseEvent) {
    // Right-click during a held DnD pickup is reserved for cancellation by
    // GmScreen.svelte's global listener. Do nothing here in that case.
    if (dndStore.held !== null) return;
    e.preventDefault();
    ctxAnchor = { x: e.clientX, y: e.clientY };
    ctxOpen = true;
  }

  function closeCtx() { ctxOpen = false; }

  function handleBodyClick(_e: MouseEvent) {
    // The overflow pill (Task 9) uses stop-propagation to avoid toggling on
    // pill-click. This handler runs only for bare body clicks, which always
    // toggle active.
    onToggleActive();
  }

  function handleBodyKey(e: KeyboardEvent) {
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      onToggleActive();
    }
  }

  let cardActions = $derived<CardAction[]>(
    (
      [
        {
          kind: 'item' as const,
          label: 'Open',
          shortcut: 'Enter',
          onActivate: () => { if (cardEl) onOpenEditor(cardEl); },
        },
        {
          kind: 'item' as const,
          label: modifier.isActive ? 'Deactivate' : 'Activate',
          shortcut: 'Click',
          onActivate: onToggleActive,
        },
        { kind: 'divider' as const },
        {
          kind: 'item' as const,
          label: modifier.isHidden ? 'Unhide' : 'Hide',
          onActivate: onHide,
        },
        canPush ? {
          kind: 'item' as const,
          label: 'Push to Foundry',
          onActivate: () => onPush?.(),
        } : null,
        onSaveAsOverride ? {
          kind: 'item' as const,
          label: 'Save as local override',
          onActivate: () => onSaveAsOverride?.(),
        } : null,
        { kind: 'divider' as const },
        canReset ? {
          kind: 'item' as const,
          label: 'Reset card',
          destructive: true as const,
          onActivate: () => onReset?.(),
        } : null,
        onDelete ? {
          kind: 'item' as const,
          label: 'Delete',
          destructive: true as const,
          onActivate: () => onDelete?.(),
        } : null,
      ] as (CardAction | null)[]
    ).filter((a): a is CardAction => a !== null)
  );

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

  // DnD pickup source. Virtual cards (id=0) are pickup-disabled — they have
  // no DB row yet, so emitting a DragSource pointing at id=0 would confuse
  // the matrix. The GM materializes first (click to activate) before reshuffling.
  let dragSource = $derived.by((): DragSourceType => {
    if (modifier.binding.kind === 'advantage') return { kind: 'advantage', mod: modifier };
    return { kind: 'free-mod', mod: modifier };
  });

  let dragDisabled = $derived(isVirtual);

  // Static overflow heuristic (spec §6.2) — count every body element that
  // renders a line, including the conditional ones. Tune the threshold if
  // cards routinely under- or over-pill in practice.
  let bodyLineCount = $derived.by(() => {
    let n = 1; // .card-name
    if (originTemplateName) n += 1;
    n += bonuses.length;
    if (conditionalsSkipped.length > 0) n += 1;
    n += Math.max(modifier.effects.length, 1); // effects (or "(no effect)" placeholder)
    if (modifier.tags.length > 0) n += 1;
    return n;
  });
  const OVERFLOW_THRESHOLD = 5;
  let hasOverflow = $derived(bodyLineCount > OVERFLOW_THRESHOLD);
  let hiddenCount = $derived(Math.max(0, bodyLineCount - OVERFLOW_THRESHOLD));
</script>

<div
  bind:this={cardEl}
  class="modifier-card"
  data-active={modifier.isActive ? 'true' : 'false'}
  data-hidden={modifier.isHidden ? 'true' : 'false'}
  data-zone={modifier.zone}
  oncontextmenu={handleContextMenu}
>
  <DragSource source={dragSource} disabled={dragDisabled}>
    <CardDragHandle isActive={modifier.isActive} zone={modifier.zone} />
  </DragSource>
  <div
    class="card-body"
    role="button"
    tabindex="0"
    onclick={handleBodyClick}
    onkeydown={handleBodyKey}
  >
    {#if modifier.zone === 'situational'}
        <span class="zone-chip" aria-label="Situational modifier">Situational</span>
      {/if}
      <p class="card-name" title={modifier.name}>
        {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}{#if showOverride}<span class="override-mark" title="Saved local override — this card's data comes from your saved copy, which supersedes the live Foundry read-through">*</span>{/if}
      </p>
      {#if originTemplateName}
        <p class="origin">from "{originTemplateName}"</p>
      {/if}
      {#if bonuses.length > 0}
        <div class="bonuses">
          {#each bonuses as b}
            <p class="bonus" title={`${summarizeBonus(b)}${b.source ? ' — ' + b.source : ''}`}>
              <span class="bonus-value">{summarizeBonus(b)}</span>
              {#if b.source}<span class="bonus-source">{b.source}</span>{/if}
            </p>
          {/each}
        </div>
      {/if}
      {#if conditionalsSkipped.length > 0}
        <p
          class="conditionals-badge"
          title={conditionalsSkipped
            .map(b => `${b.source ?? '(unnamed)'} — ${b.activeWhen?.check ?? '?'}`)
            .join('\n')}
        >
          ({conditionalsSkipped.length} conditional{conditionalsSkipped.length === 1 ? '' : 's'})
        </p>
      {/if}
      <div class="effects">
        {#if modifier.effects.length === 0}
          <p class="no-effect">(no effect)</p>
        {:else}
          {#each modifier.effects as e}
            <p class="effect" title={summarize(e)}>{summarize(e)}</p>
          {/each}
        {/if}
      </div>
      {#if modifier.tags.length > 0}
        <div class="tags" title={modifier.tags.map(t => `#${t}`).join(' ')}>
          {#each modifier.tags as t, i}{#if i > 0}{' '}{/if}<span class="tag">#{t}</span>{/each}
        </div>
      {/if}
    </div>
  {#if hasOverflow}
    <button
      type="button"
      class="overflow-pill"
      title="Open full card"
      aria-label="Open full card"
      onclick={(e) => { e.stopPropagation(); if (cardEl) onOpenEditor(cardEl); }}
    >+{hiddenCount} <span class="glyph">⤢</span></button>
  {/if}
  <CardContextMenu open={ctxOpen} anchor={ctxAnchor} actions={cardActions} onClose={closeCtx} />
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
    /* Safety net: clip overflowing content rather than letting it spill
       outside the card boundary. The card's own :hover box-shadow renders
       outside the box and is unaffected. */
    overflow: hidden;
    z-index: calc(100 - var(--distance));

    transform: translateX(calc(var(--base-x) + var(--shift-x)));
    transition: transform var(--card-trans-duration) var(--card-trans-easing),
                box-shadow var(--card-trans-duration) var(--card-trans-easing),
                border-color 200ms ease;
  }

  .modifier-card:hover {
    /* Must exceed the highest possible baseline (=100 for the centre card)
       so a hovered inner card is never covered by a same-z sibling that
       happens to come later in DOM order. */
    z-index: 1000;
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

  /* Suppress the hover lift + box-shadow + neighbour-shift cascade while a
     DnD pickup is held (ancestor toggled by GmScreen.svelte). Cursor sweep
     across the row during a pickup must not trigger the hover animation.
     The `!important` is bounded to .dnd-active so it cannot leak. */
  :global(.dnd-active) .modifier-card:hover {
    transform: translateX(calc(var(--base-x) + var(--shift-x))) !important;
    box-shadow: none !important;
    z-index: calc(100 - var(--distance));
  }
  :global(.dnd-active) .modifier-card:hover + :global(.modifier-card),
  :global(.dnd-active) .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card),
  :global(.dnd-active) .modifier-card:hover + :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card),
  :global(.dnd-active) .modifier-card:has(+ :global(.modifier-card:hover)),
  :global(.dnd-active) .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card:hover)),
  :global(.dnd-active) .modifier-card:has(+ :global(.modifier-card) + :global(.modifier-card) + :global(.modifier-card:hover)) {
    --shift-x: 0rem !important;
  }

  .modifier-card[data-active="true"] {
    border-color: var(--accent-bright);
    background: var(--bg-active);
  }
  .modifier-card[data-hidden="true"] {
    opacity: 0.45;
    filter: saturate(0.6);
  }

  .card-name {
    margin: 0 0 0.3rem;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .virtual-mark { color: var(--accent-amber); margin-left: 0.15rem; }
  .override-mark {
    color: var(--accent-amber);
    margin-left: 0.15rem;
    font-weight: 700;
    cursor: help;
  }
  .origin {
    margin: 0;
    font-size: 0.6rem;
    color: var(--text-muted);
    font-style: italic;
  }
  .bonuses { display: flex; flex-direction: column; gap: 0.1rem; }
  .bonus {
    font-size: 0.65rem;
    margin: 0;
    color: var(--text-secondary);
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
    /* Stay on one line — wrapping pushed total content past the card height
       and spilled the foot below the card. Full text remains in the title. */
    flex-wrap: nowrap;
    min-width: 0;
    overflow: hidden;
  }
  .bonus-value {
    font-weight: 500;
    color: var(--accent-bright);
    flex-shrink: 0;
  }
  .bonus-source {
    font-size: 0.6rem;
    color: var(--text-muted);
    font-style: italic;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .conditionals-badge {
    margin: 0;
    font-size: 0.6rem;
    color: var(--text-muted);
    font-style: italic;
    cursor: help;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .effects { display: flex; flex-direction: column; gap: 0.15rem; }
  .effect, .no-effect {
    font-size: 0.7rem;
    margin: 0;
    color: var(--text-secondary);
    /* Single line per effect — long path lists ellipse. */
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .no-effect { color: var(--text-muted); font-style: italic; }

  /* Single-line truncation: when tags overflow the card width, they ellipse
     mid-string ("#social #physi…") rather than wrapping onto a second row.
     Hover title surfaces the full tag list. */
  .tags {
    display: block;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    font-size: 0.65rem;
    line-height: 1.2;
  }
  .tag { color: var(--text-muted); }

  .overflow-pill {
    position: absolute;
    bottom: 0.3rem;
    right: 0.5rem;
    background: var(--bg-raised);
    border: 1px solid var(--border-faint);
    color: var(--text-primary);
    font-size: 0.6rem;
    padding: 0.05rem 0.45rem;
    border-radius: 999px;
    cursor: pointer;
    z-index: 2;
    transition: background 120ms ease, border-color 120ms ease;
    font-family: inherit;
  }
  .overflow-pill:hover {
    background: var(--accent);
    border-color: var(--accent-bright);
  }
  .overflow-pill .glyph {
    margin-left: 0.2rem;
    opacity: 0.7;
  }

  .card-body {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    flex: 1;
    min-width: 0;
    /* min-height: 0 overrides the default `min-height: auto` on flex children,
       which would otherwise let content (tags, many bonuses) grow past the
       card-body's flex allocation. Combined with overflow: hidden here so any
       excess clips inside card-body rather than displacing the card boundary. */
    min-height: 0;
    overflow: hidden;
    -webkit-mask-image: linear-gradient(180deg, black 80%, transparent);
            mask-image: linear-gradient(180deg, black 80%, transparent);
    cursor: pointer;
  }

  .modifier-card[data-zone="situational"] {
    background: var(--bg-situational-card);
    border-color: var(--border-situational);
  }
  .modifier-card[data-zone="situational"][data-active="true"] {
    border-color: var(--accent-situational-bright);
    background: var(--bg-situational-card);
  }

  .zone-chip {
    align-self: flex-start;
    font-size: 0.55rem;
    padding: 0.05rem 0.4rem;
    border-radius: 3px;
    background: var(--accent-situational);
    color: #d8f0d8;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    line-height: 1.3;
  }

  @media (prefers-reduced-motion: reduce) {
    .modifier-card {
      transition: none;
    }
    .modifier-card:hover {
      transform: translateX(calc(var(--base-x) + var(--shift-x)));
    }
  }
</style>
