<script lang="ts">
  // ActiveEffectsSummary — left-side roll-up of every active effect on a
  // single character. Shown alongside the modifier carousel in the GM screen
  // CharacterRow. Renders nothing when no modifier on the character is active.
  //
  // Two-pass aggregation:
  //   1. Path-bound effects (stat or pool with non-empty paths) — reuses
  //      computeActiveDeltas() so the rollup matches what View 2 of the
  //      character card displays.
  //   2. Pathless pool / difficulty / note effects — iterated independently
  //      because they don't project to a stat path (they're scope-based or
  //      informational).
  //
  // See: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6.

  import type { BridgeCharacter, CharacterModifier } from '../../../types';
  import { computeActiveDeltas } from '$lib/character/active-deltas';

  interface Props {
    character: BridgeCharacter;
    modifiers: CharacterModifier[];
  }
  let { character, modifiers }: Props = $props();

  const activeMods = $derived(modifiers.filter(m => m.isActive));
  const pathDeltas = $derived(computeActiveDeltas(character, modifiers));

  interface NonPathEffect {
    kind: 'pool' | 'difficulty' | 'note';
    scope: string | null;
    delta: number | null;
    note: string | null;
    modifierName: string;
  }

  const nonPathEffects = $derived.by((): NonPathEffect[] => {
    const out: NonPathEffect[] = [];
    for (const m of activeMods) {
      for (const e of m.effects) {
        // Skip effects already represented in pathDeltas (path-bound stat/pool).
        if ((e.kind === 'pool' || e.kind === 'stat') && e.paths.length > 0) continue;
        if (e.kind === 'pool' || e.kind === 'difficulty' || e.kind === 'note') {
          out.push({
            kind: e.kind,
            scope: e.scope,
            delta: e.delta,
            note: e.note,
            modifierName: m.name,
          });
        }
      }
    }
    return out;
  });

  const totalEffectCount = $derived(pathDeltas.size + nonPathEffects.length);

  // 'attributes.charisma' -> 'Charisma'; 'skills.brawl' -> 'Brawl'
  function pathLabel(path: string): string {
    const dot = path.indexOf('.');
    const segment = dot >= 0 ? path.slice(dot + 1) : path;
    return segment.charAt(0).toUpperCase() + segment.slice(1);
  }

  function signed(n: number): string {
    return n > 0 ? `+${n}` : `${n}`;
  }

  function pathTooltip(entry: { sources: { modifierName: string; scope: string | null }[] }): string {
    return entry.sources
      .map(s => `${s.modifierName}${s.scope ? ' — ' + s.scope : ''}`)
      .join('\n');
  }
</script>

{#if activeMods.length > 0}
  <aside class="active-effects-summary" aria-label="Active effects">
    <div class="title">Active effects</div>
    <ul class="effect-list">
      {#each [...pathDeltas.values()] as entry (entry.path)}
        <li class="effect-item path-delta" title={pathTooltip(entry)}>
          <span class="lbl">{pathLabel(entry.path)}</span>
          <span class="val">{signed(entry.delta)}</span>
        </li>
      {/each}
      {#each nonPathEffects as e, i (`${e.kind}-${i}`)}
        {#if e.kind === 'pool'}
          <li class="effect-item pool" title={e.modifierName}>
            <span class="lbl">{e.scope ? `${e.scope} pool` : 'Pool'}</span>
            <span class="val">{signed(e.delta ?? 0)}d</span>
          </li>
        {:else if e.kind === 'difficulty'}
          <li class="effect-item diff" title={e.modifierName}>
            <span class="lbl">Diff{e.scope ? ` ${e.scope}` : ''}</span>
            <span class="val">{signed(e.delta ?? 0)}</span>
          </li>
        {:else if e.kind === 'note'}
          <li class="effect-item note" title={e.modifierName}>
            <span class="lbl note-text">"{e.note ?? ''}"</span>
          </li>
        {/if}
      {/each}
    </ul>
    <div class="footer">
      {activeMods.length} modifier{activeMods.length === 1 ? '' : 's'} · {totalEffectCount} fx
    </div>
  </aside>
{/if}

<style>
  .active-effects-summary {
    width: 12rem;
    flex-shrink: 0;
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 0.4rem;
    padding: 0.45rem 0.55rem;
    box-sizing: border-box;
    height: 8rem;            /* matches .modifier-row height for alignment */
    display: flex;
    flex-direction: column;
  }
  .title {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--text-label);
    padding-bottom: 0.3rem;
    border-bottom: 1px solid var(--border-faint);
    margin-bottom: 0.3rem;
  }
  .effect-list {
    list-style: none;
    padding: 0;
    margin: 0;
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    font-size: 0.72rem;
    line-height: 1.4;
  }
  .effect-item {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.4rem;
    padding: 0.05rem 0;
  }
  .lbl {
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .val {
    color: var(--accent-bright);
    font-weight: 600;
    font-family: ui-monospace, monospace;
    flex-shrink: 0;
  }
  .effect-item.diff .val { color: var(--accent-amber); }
  .effect-item.note { padding: 0.1rem 0; }
  .note-text {
    color: var(--text-secondary);
    font-style: italic;
    font-size: 0.7rem;
  }
  .footer {
    font-size: 0.62rem;
    color: var(--text-muted);
    text-align: right;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.25rem;
    margin-top: 0.25rem;
    letter-spacing: 0.04em;
  }
</style>
