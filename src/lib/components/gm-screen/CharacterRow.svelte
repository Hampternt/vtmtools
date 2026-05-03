<script lang="ts">
  import { modifiers } from '../../../store/modifiers.svelte';
  import type {
    BridgeCharacter, CharacterModifier, ModifierEffect, FoundryItem, FoundryItemBonus,
  } from '../../../types';
  import ModifierCard from './ModifierCard.svelte';
  import ModifierEffectEditor from './ModifierEffectEditor.svelte';

  interface Props {
    character: BridgeCharacter;
    activeFilterTags: Set<string>;
    showHidden: boolean;
  }
  let { character, activeFilterTags, showHidden }: Props = $props();

  // Editor popover state — anchored to the cog via getBoundingClientRect()
  // (spec §7.3 "anchored to the cog itself, not a modal"). popoverPos uses
  // viewport coords so the wrap is position:fixed.
  type EditorTarget = { kind: 'materialized', mod: CharacterModifier }
                    | { kind: 'virtual', virt: VirtualCard };
  let editorOpen = $state(false);
  let editorTarget = $state<EditorTarget | null>(null);
  let popoverPos = $state<{ left: number; top: number } | null>(null);

  /**
   * Virtual cards are advantage-derived rows the GM hasn't engaged yet — no
   * DB id. Materialize on first engagement (toggle / hide / save edits).
   */
  interface VirtualCard {
    item: FoundryItem;
    name: string;
    description: string;
  }

  // Stage 1: only Foundry advantages auto-render (spec §3 / Roll20 deferred to Phase 2.5).
  let foundryItems = $derived(
    character.source === 'foundry' && character.raw && typeof character.raw === 'object' && 'items' in (character.raw as Record<string, unknown>)
      ? ((character.raw as { items: FoundryItem[] }).items ?? [])
      : []
  );

  let advantageItems = $derived(
    foundryItems.filter(it => {
      if (it.type !== 'feature') return false;
      const ft = (it.system as Record<string, unknown>)?.featuretype as string | undefined;
      return ft === 'merit' || ft === 'flaw' || ft === 'background' || ft === 'boon';
    })
  );

  let charMods = $derived(modifiers.forCharacter(character.source, character.source_id));

  /** Read sheet-attached bonuses (system.bonuses[]) off a Foundry feature
   *  item by its _id. Returns [] when the item is gone or has no bonuses. */
  function bonusesFor(itemId: string): FoundryItemBonus[] {
    const item = advantageItems.find(it => it._id === itemId);
    if (!item) return [];
    const raw = (item.system as Record<string, unknown>)?.bonuses;
    return Array.isArray(raw) ? (raw as FoundryItemBonus[]) : [];
  }

  // Build the card list per spec §8.1.
  type CardEntry =
    | { kind: 'materialized'; mod: CharacterModifier; isStale: boolean }
    | { kind: 'virtual'; virt: VirtualCard };

  let cardEntries = $derived.by((): CardEntry[] => {
    const entries: CardEntry[] = [];

    // (2) Walk advantage items, merging with materialized rows.
    for (const item of advantageItems) {
      const matched = charMods.find(m => m.binding.kind === 'advantage' && m.binding.item_id === item._id);
      if (matched) {
        entries.push({ kind: 'materialized', mod: matched, isStale: false });
      } else {
        entries.push({ kind: 'virtual', virt: {
          item,
          name: item.name,
          description: ((item.system as Record<string, unknown>)?.description as string | undefined) ?? '',
        }});
      }
    }

    // (3) Append free-floating modifiers (and any 'advantage' mods whose item was deleted — these become stale).
    const knownAdvantageItemIds = new Set(advantageItems.map(it => it._id));
    for (const m of charMods) {
      if (m.binding.kind === 'free') {
        entries.push({ kind: 'materialized', mod: m, isStale: false });
      } else if (m.binding.kind === 'advantage' && !knownAdvantageItemIds.has(m.binding.item_id)) {
        entries.push({ kind: 'materialized', mod: m, isStale: true });
      }
    }
    return entries;
  });

  // (4) Apply filter — active cards always pinned past the filter (spec §7.5).
  function passesTagFilter(e: CardEntry): boolean {
    if (activeFilterTags.size === 0) return true;
    if (e.kind === 'virtual') return false; // virtual has no tags yet
    if (e.mod.isActive) return true;        // active pin rule
    return e.mod.tags.some(t => activeFilterTags.has(t));
  }

  function passesHiddenFilter(e: CardEntry): boolean {
    if (e.kind === 'virtual') return true;
    if (e.mod.isHidden) return showHidden;
    return true;
  }

  // (5) Sort: active DESC, then created_at ASC for materialized; virtuals sort by item name.
  function sortKey(e: CardEntry): [number, string] {
    if (e.kind === 'virtual') return [1, e.virt.name];
    return [e.mod.isActive ? 0 : 1, e.mod.createdAt];
  }

  let visibleCards = $derived(
    cardEntries
      .filter(e => passesTagFilter(e) && passesHiddenFilter(e))
      .sort((a, b) => {
        const [ak, an] = sortKey(a);
        const [bk, bn] = sortKey(b);
        if (ak !== bk) return ak - bk;
        return an < bn ? -1 : an > bn ? 1 : 0;
      })
  );

  /** Materialize a virtual card before applying any change. */
  async function materialize(virt: VirtualCard): Promise<CharacterModifier> {
    return await modifiers.materializeAdvantage({
      source: character.source,
      sourceId: character.source_id,
      itemId: virt.item._id,
      name: virt.name,
      description: virt.description,
    });
  }

  async function handleToggleActive(e: CardEntry): Promise<void> {
    if (e.kind === 'virtual') {
      const m = await materialize(e.virt);
      await modifiers.setActive(m.id, true);
    } else {
      await modifiers.setActive(e.mod.id, !e.mod.isActive);
    }
  }

  async function handleHide(e: CardEntry): Promise<void> {
    if (e.kind === 'virtual') {
      const m = await materialize(e.virt);
      await modifiers.setHidden(m.id, true);
    } else {
      await modifiers.setHidden(e.mod.id, true);
    }
  }

  function openEditor(e: CardEntry, anchor: HTMLElement): void {
    editorTarget = e.kind === 'materialized'
      ? { kind: 'materialized', mod: e.mod }
      : { kind: 'virtual', virt: e.virt };
    // Anchor the popover just to the right of the cog and slightly below.
    // Viewport coords pair with position:fixed below.
    const rect = anchor.getBoundingClientRect();
    popoverPos = { left: rect.right + 8, top: rect.bottom + 4 };
    editorOpen = true;
  }

  function closeEditor(): void {
    editorOpen = false;
    editorTarget = null;
    popoverPos = null;
  }

  async function saveEditor(effects: ModifierEffect[], tags: string[]): Promise<void> {
    if (!editorTarget) return;
    let id: number;
    if (editorTarget.kind === 'virtual') {
      const m = await materialize(editorTarget.virt);
      id = m.id;
    } else {
      id = editorTarget.mod.id;
    }
    await modifiers.update(id, { effects, tags });
    closeEditor();
  }

  async function addFreeModifier(): Promise<void> {
    await modifiers.add({
      source: character.source,
      sourceId: character.source_id,
      name: 'New modifier',
      description: '',
      effects: [],
      binding: { kind: 'free' },
      tags: [],
      originTemplateId: null,
    });
  }

  function damageSummary(): string {
    if (!character.health) return '—';
    const { superficial, aggravated } = character.health;
    if (superficial === 0 && aggravated === 0) return 'Dmg —';
    return `Dmg ${superficial}s/${aggravated}a`;
  }
</script>

<section class="row" data-source={character.source}>
  <header>
    <h2>{character.name}</h2>
    <span class="source">{character.source}</span>
    {#if character.hunger != null}<span class="stat">Hunger {character.hunger}</span>{/if}
    {#if character.willpower}
      <span class="stat">WP {character.willpower.max - character.willpower.superficial - character.willpower.aggravated}/{character.willpower.max}</span>
    {/if}
    <span class="stat">{damageSummary()}</span>
  </header>

  <div
    class="modifier-row"
    style="--cards: {visibleCards.length};"
  >
    {#each visibleCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
      <ModifierCard
        modifier={entry.kind === 'virtual'
          ? {
              id: 0,
              source: character.source,
              sourceId: character.source_id,
              name: entry.virt.name,
              description: entry.virt.description,
              effects: [],
              binding: { kind: 'advantage', item_id: entry.virt.item._id },
              tags: [],
              isActive: false,
              isHidden: false,
              originTemplateId: null,
              createdAt: '',
              updatedAt: '',
            }
          : entry.mod}
        isVirtual={entry.kind === 'virtual'}
        isStale={entry.kind === 'materialized' && entry.isStale}
        bonuses={entry.kind === 'virtual'
          ? bonusesFor(entry.virt.item._id)
          : entry.mod.binding.kind === 'advantage'
            ? bonusesFor(entry.mod.binding.item_id)
            : []}
        onToggleActive={() => handleToggleActive(entry)}
        onHide={() => handleHide(entry)}
        onOpenEditor={(anchor) => openEditor(entry, anchor)}
      />
    {/each}
    <button class="add-modifier" onclick={addFreeModifier}>+ Add modifier</button>
  </div>

  {#if editorOpen && editorTarget && popoverPos}
    <div class="popover-wrap" style="left: {popoverPos.left}px; top: {popoverPos.top}px;">
      <ModifierEffectEditor
        initialEffects={editorTarget.kind === 'materialized' ? editorTarget.mod.effects : []}
        initialTags={editorTarget.kind === 'materialized' ? editorTarget.mod.tags : []}
        onSave={saveEditor}
        onCancel={closeEditor}
      />
    </div>
  {/if}
</section>

<style>
  .row {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.5rem;
    padding: 0.75rem;
    margin-bottom: 0.6rem;
    box-sizing: border-box;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 0.65rem;
    margin-bottom: 0.6rem;
  }
  header h2 { margin: 0; font-size: 0.95rem; color: var(--text-primary); }
  .source {
    font-size: 0.65rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .stat { font-size: 0.75rem; color: var(--text-secondary); }

  .modifier-row {
    --card-trans-duration: 600ms;
    --card-trans-easing: linear(
      0, 0.01 0.8%, 0.038 1.6%, 0.154 3.4%, 0.781 9.7%, 1.01 12.5%,
      1.089 13.8%, 1.153 15.2%, 1.195 16.6%, 1.219 18%, 1.224 19.7%,
      1.208 21.6%, 1.172 23.6%, 1.057 28.6%, 1.007 31.2%, 0.969 34.1%,
      0.951 37.1%, 0.953 40.9%, 0.998 50.4%, 1.011 56%, 0.998 74.7%, 1
    );
    --card-width: 9rem;
    --card-overlap: 0.55;
    --card-shift-delta: 0.5rem;
    /* --cards is set inline via the style prop above to drive the z-stack centering math. */
    position: relative;
    height: 8rem;
    perspective: 800px;
  }

  .add-modifier {
    position: absolute;
    /* placed past the last card; uses the same overlap math */
    left: calc(var(--cards) * var(--card-width) * (1 - var(--card-overlap)));
    top: 0;
    height: 100%;
    width: 9rem;
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px dashed var(--border-faint);
    border-radius: 0.625rem;
    cursor: pointer;
    box-sizing: border-box;
  }
  .add-modifier:hover { color: var(--text-primary); border-color: var(--border-surface); }

  .popover-wrap {
    /* Anchored to the cog via getBoundingClientRect() — viewport coords. */
    position: fixed;
    z-index: 1000;
  }

  @media (prefers-reduced-motion: reduce) {
    .modifier-row {
      --card-overlap: 0;
      --card-shift-delta: 0;
    }
  }
</style>
