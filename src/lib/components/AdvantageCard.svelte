<script lang="ts">
  import type { Advantage, Field, AdvantageKind, ModifierEffect } from '../../types';
  import { bridge } from '../../store/bridge.svelte';

  /**
   * Discriminated input: lets the same card render both saved library
   * rows and unsaved builder previews. Saved rows carry a DB id plus the
   * provenance / wip flags; previews omit id + provenance (the builder
   * hasn't decided either yet) and pass `wip` alongside the payload.
   */
  type SavedInput   = { kind: 'saved';   data: Advantage };
  type PreviewInput = {
    kind: 'preview';
    data: Omit<Advantage, 'id' | 'isCustom' | 'sourceAttribution'>;
    wip?: boolean;
  };

  interface Props {
    input: SavedInput | PreviewInput;
    /** Preview-mode click-to-edit. Ignored in saved mode. */
    onClick?: (target: HTMLElement) => void;
    /** Saved-mode edit callback. Card has no IPC knowledge.
     *  Wired by Task 3's hover-reveal ✎ affordance. */
    onedit?: () => void;
    /** Saved-mode delete callback. Card has no IPC knowledge.
     *  Wired by Task 3's hover-reveal 🗑 affordance. */
    ondelete?: () => void;
    /** Saved-mode push-to-Foundry callback. Card has no IPC knowledge.
     *  Wired by Task 3's hover-reveal ↑ affordance; only rendered when
     *  the Foundry bridge is connected. */
    onPush?: () => void;
  }

  let { input, onClick, onedit, ondelete, onPush }: Props = $props();

  const data = $derived(input.data);
  const name = $derived(data.name);
  const isPreview = $derived(input.kind === 'preview');
  const isWip = $derived(input.kind === 'saved' ? input.data.wip : (input.wip ?? false));

  function onDeleteClick(e: Event) {
    e.stopPropagation();
    const ok = confirm(`Delete ${name}?`);
    if (ok) ondelete?.();
  }

  function findField(name: string): Field | undefined {
    return data.properties.find(p => p.name === name);
  }

  function numberValue(f: Field | undefined): number | null {
    if (!f) return null;
    if (f.type !== 'number') return null;
    return Array.isArray(f.value) ? f.value[0] ?? null : f.value;
  }

  // Spec footer: LEVEL renders 5 tier dots. Null level (no dot-cost
  // mechanic on this advantage) → all empty. Reads dot_cost_per_level
  // first (the new-writer convention from AdvantageBody / routes),
  // falls back to 'level' (seed + library_push + Foundry-pull writes).
  // `?? 0` keeps the loop comparator non-nullable.
  const level = $derived(numberValue(findField('dot_cost_per_level') ?? findField('level')) ?? 0);

  function kindLabel(k: AdvantageKind): string {
    return ({ merit: 'Merit', flaw: 'Flaw', background: 'Background', boon: 'Boon' } as const)[k] ?? 'Merit';
  }

  /**
   * Render the first ModifierEffect as a footer bonus chip. Format:
   * `{+delta} {scope}` when delta is present, else just `{scope}`.
   * Scope falls back to the literal `'all'` when `paths` is empty
   * (the modifier applies to every pool / stat by virtue of being
   * unscoped). Returns `'—'` when there's no effect at all.
   */
  function bonusLabel(e?: ModifierEffect): string {
    if (!e) return '—';
    const scope = e.scope && e.scope.trim().length > 0
      ? e.scope
      : (e.paths && e.paths.length > 0 ? e.paths[0] : 'all');
    if (e.delta !== null && e.delta !== undefined) {
      const sign = e.delta >= 0 ? '+' : '';
      return `${sign}${e.delta} ${scope}`;
    }
    return scope;
  }

  const firstEffect = $derived<ModifierEffect | undefined>(data.effects?.[0]);
</script>

<article
  class="vamp-card"
  class:preview={isPreview}
  class:wip={isWip}
  data-kind={data.kind}
  onclick={isPreview && onClick ? (e) => onClick(e.target as HTMLElement) : undefined}
>
  {#if input.kind === 'saved'}
    <!-- v1.x backlog: surface sourceAttribution (FVTT · worldTitle) when present; spec §"Card visual design" doesn't define placement -->
    <button
      type="button"
      class="affordance edit-icon"
      aria-label={`Edit ${name}`}
      onclick={(e) => { e.stopPropagation(); onedit?.(); }}
    >✎</button>

    <button
      type="button"
      class="affordance delete-icon"
      aria-label={`Delete ${name}`}
      onclick={onDeleteClick}
    >🗑</button>

    {#if bridge.connections.foundry && onPush}
      <button
        type="button"
        class="affordance push-icon"
        aria-label={`Push ${name} to Foundry`}
        onclick={(e) => { e.stopPropagation(); onPush?.(); }}
        title="Push to Foundry"
      >↑</button>
    {/if}
  {/if}

  <div class="frame">
    <header class="zone-title">
      <h3 data-stage="name">{data.name}</h3>
    </header>
    <section class="zone-content">
      {#if data.description}
        <p class="description" data-stage="description">{data.description}</p>
      {/if}
      {#if data.tags.length > 0}
        <div class="tag-row" data-stage="tags">
          {#each data.tags as t}
            <span class="tag">{t}</span>
          {/each}
        </div>
      {/if}
      <div class="hairline" aria-hidden="true"></div>
    </section>
    <footer class="zone-footer">
      <div class="detail" data-field="type" data-stage="type">
        <h6>TYPE</h6>
        <span class="value">{kindLabel(data.kind)}</span>
      </div>
      <div class="detail" data-field="level" data-stage="level">
        <h6>LEVEL</h6>
        <span class="value tier-dots">
          {#each Array(5) as _, i}
            <span class="dot" class:filled={i < level} aria-hidden="true"></span>
          {/each}
          <span class="sr-only">{level} of 5</span>
        </span>
      </div>
      <div class="detail" data-field="bonus">
        <h6>BONUS</h6>
        <span class="value bonus" class:empty={!firstEffect}>{bonusLabel(firstEffect)}</span>
      </div>
    </footer>
  </div>
</article>

<style>
  .vamp-card {
    position: relative;
    width: 100%;
    max-width: 22rem;
    min-width: 14rem;
    font-family: 'Iowan Old Style', Garamond, serif;
    color: var(--text-primary);
    box-shadow: var(--shadow-medium);
  }

  .frame {
    border: 1px solid var(--outline-color);
    position: relative;
    background: var(--bg-card, #1d1c1d);
  }
  .frame::before {
    content: "";
    position: absolute;
    inset: 4px;
    border: 1px solid var(--outline-color);
    pointer-events: none;
    opacity: 0.35;
  }

  .zone-title {
    padding: 0.85rem 1rem 0.5rem;
    text-align: center;
  }
  .zone-title h3 {
    margin: 0;
    font-family: 'Spartan', sans-serif;
    font-weight: 200;
    font-size: 1.05rem;
    letter-spacing: 1.6px;
    line-height: 1.25;
    text-transform: uppercase;
  }

  .zone-content {
    padding: 0 1rem 0.55rem;
  }
  .zone-content .description {
    margin: 0 0 0.5rem;
    font-family: 'Oswald', sans-serif;
    font-weight: 200;
    font-size: 0.82rem;
    letter-spacing: 0.4px;
    line-height: 1.55;
    text-align: left;
  }
  .zone-content .tag-row {
    display: flex;
    justify-content: center;
    gap: 0.25rem;
    font-family: 'Spartan', sans-serif;
    font-size: 0.6rem;
    letter-spacing: 0.4px;
    color: var(--text-muted);
    text-transform: lowercase;
  }
  .zone-content .tag-row .tag::before { content: "· "; opacity: 0.5; }
  .zone-content .tag-row .tag:first-child::before { content: ""; }
  .zone-content .hairline {
    height: 1px;
    margin: 0.45rem 1rem;
    background: linear-gradient(to right, transparent, var(--hairline), transparent);
  }

  .zone-footer {
    display: flex;
    gap: 0.35rem;
    margin: 0 0.55rem 0.6rem;
    padding: 0 0.4rem;
  }
  .zone-footer .detail {
    flex: 1;
    background: var(--bg-detail);
    border: 1px solid var(--border-detail);
    padding: 0.45rem 0.4rem 0.5rem;
    text-align: center;
  }
  .zone-footer .detail h6 {
    margin: 0 0 0.25rem;
    font-family: 'Spartan', sans-serif;
    font-weight: 200;
    font-size: 0.58rem;
    letter-spacing: 1.1px;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .zone-footer .detail .value {
    font-family: 'Oswald', sans-serif;
    font-weight: 200;
    font-size: 0.85rem;
    letter-spacing: 0.7px;
  }
  .zone-footer .detail[data-field="bonus"] .value {
    color: var(--blood-10);
    text-shadow: 0 0 4px rgba(224, 86, 58, 0.4);
    font-weight: 600;
    font-size: 0.95rem;
  }
  .zone-footer .detail[data-field="bonus"] .value:empty,
  .zone-footer .detail[data-field="bonus"] .value.empty {
    color: var(--text-muted);
    opacity: 0.45;
    font-size: 0.85rem;
    text-shadow: none;
    font-weight: 200;
  }

  .tier-dots {
    display: inline-flex;
    gap: 0.18rem;
    align-items: center;
    justify-content: center;
  }
  .tier-dots .dot {
    width: 0.48rem;
    height: 0.48rem;
    border-radius: 50%;
    background: transparent;
    border: 1px solid var(--accent-amber);
    display: inline-block;
  }
  .tier-dots .dot.filled {
    background: var(--accent-amber);
    box-shadow: 0 0 3px rgba(217, 165, 96, 0.55);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }

  /* Preview-mode highlight: amber border + warm glow makes builder
     previews visually distinct from saved library rows. Carried over
     from the prior implementation; applies to the inner frame so the
     outlined-frame structure is preserved. */
  .vamp-card.preview .frame {
    border-color: var(--accent-amber);
    box-shadow: 0 0 16px rgba(217, 165, 96, 0.22);
  }
  /* WIP draft indicator — subtle dimming. */
  .vamp-card.wip {
    opacity: 0.92;
  }

  /* Hover-reveal affordances — three distinct corners satisfy the
     Fitts's Law mis-click defense the spec calls out for ✎/🗑 and
     naturally extend to ↑ (push). Cards have no IPC knowledge; the
     callbacks are wired by the caller. */
  .affordance {
    position: absolute;
    background: transparent;
    border: 0;
    padding: 0;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 0.85rem;
    line-height: 1;
    opacity: 0;
    transition: opacity 120ms ease-out;
    z-index: 2;
  }
  .vamp-card:hover .edit-icon   { opacity: 0.7; }
  .vamp-card:hover .delete-icon { opacity: 0.6; }
  .vamp-card:hover .push-icon   { opacity: 0.7; }
  .affordance:focus-visible {
    opacity: 1;
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }

  .edit-icon   { top: 0.5rem;    right: 0.5rem; }
  .delete-icon { bottom: 0.5rem; right: 0.5rem; }
  .push-icon   { top: 0.5rem;    left: 0.5rem;  }

  .delete-icon:hover { color: var(--accent-bright); }
  .push-icon:hover   { color: var(--accent-amber); }
</style>
