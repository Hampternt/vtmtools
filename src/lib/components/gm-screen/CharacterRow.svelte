<script lang="ts">
  import { modifiers } from '../../../store/modifiers.svelte';
  import { statusTemplates } from '../../../store/statusTemplates.svelte';
  import type {
    BridgeCharacter, CharacterModifier, ModifierEffect, FoundryItem, FoundryItemBonus,
  } from '../../../types';
  import ModifierCard from './ModifierCard.svelte';
  import ModifierEffectEditor from './ModifierEffectEditor.svelte';
  import RollDispatcherPopover from './RollDispatcherPopover.svelte';
  import ActiveEffectsSummary from './ActiveEffectsSummary.svelte';

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

  // Roll dispatcher popover state — anchored to the 🎲 button.
  let rollPopoverOpen = $state(false);
  let rollPopoverAnchor = $state<{ left: number; top: number } | null>(null);

  function openRollPopover(e: MouseEvent) {
    const btn = e.currentTarget as HTMLElement;
    const rect = btn.getBoundingClientRect();
    // Anchor below the button, left-aligned to the button's left edge.
    rollPopoverAnchor = { left: rect.left, top: rect.bottom + 4 };
    rollPopoverOpen = true;
  }

  function closeRollPopover() {
    rollPopoverOpen = false;
    rollPopoverAnchor = null;
  }

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

  // Add a derived set of live tag prefixes for this character.
  let liveTagPrefixes = $derived(new Set(
    charMods.map(m => `GM Screen #${m.id}`)
  ));

  /** Read sheet-attached bonuses (system.bonuses[]) off a Foundry feature
   *  item by its _id. Returns [] when the item is gone or has no bonuses. */
  function bonusesFor(itemId: string): FoundryItemBonus[] {
    const item = advantageItems.find(it => it._id === itemId);
    if (!item) return [];
    const raw = (item.system as Record<string, unknown>)?.bonuses;
    if (!Array.isArray(raw)) return [];
    return (raw as FoundryItemBonus[]).filter(b => {
      const src = b.source ?? '';
      // Filter out anything tagged with a LIVE modifier id (those are shown
      // in the local effects section). Orphans (stale ids) intentionally
      // pass through so the GM can spot them.
      for (const prefix of liveTagPrefixes) {
        if (src === prefix || src.startsWith(prefix + ':')) return false;
      }
      return true;
    });
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
      // Virtual cards are by definition not yet hidden — first click hides.
      const m = await materialize(e.virt);
      await modifiers.setHidden(m.id, true);
    } else {
      // Materialized: toggle so the same button serves as hide / unhide.
      await modifiers.setHidden(e.mod.id, !e.mod.isHidden);
    }
  }

  // Per-row transient notice surfacing the PushReport (or error). Single
  // notice per row, scoped by cardKey. Auto-clears after 5s.
  let pushNotice = $state<{ cardKey: string; text: string; ok: boolean } | null>(null);

  function canPushFor(e: CardEntry): boolean {
    if (character.source !== 'foundry') return false;
    if (e.kind !== 'materialized') return false;            // virtual = no DB row yet
    if (e.mod.binding.kind !== 'advantage') return false;
    if (e.isStale) return false;
    return e.mod.effects.some(eff => eff.kind === 'pool');
  }

  async function handlePush(e: CardEntry): Promise<void> {
    if (e.kind !== 'materialized') return;
    const cardKey = `m-${e.mod.id}`;
    try {
      const report = await modifiers.pushToFoundry(e.mod.id);
      const skippedSummary = report.skipped.length > 0
        ? ` (skipped ${report.skipped.length}: ${report.skipped.map(s => s.reason).join('; ')})`
        : '';
      pushNotice = {
        cardKey,
        text: `Pushed ${report.pushed} bonus${report.pushed === 1 ? '' : 'es'} to Foundry${skippedSummary}`,
        ok: true,
      };
    } catch (err) {
      pushNotice = { cardKey, text: `Push failed: ${err}`, ok: false };
    }
    setTimeout(() => { if (pushNotice?.cardKey === cardKey) pushNotice = null; }, 5000);
  }

  function canResetFor(e: CardEntry): boolean {
    if (character.source !== 'foundry') return false;
    if (e.kind !== 'materialized') return false;
    if (e.mod.binding.kind !== 'advantage') return false;
    return true;
  }

  async function handleReset(e: CardEntry): Promise<void> {
    if (e.kind !== 'materialized') return;
    const ok = confirm(
      `Reset "${e.mod.name}"?\n\n` +
      `This deletes the local effects, paths, and tags for this card.\n` +
      `Any bonuses previously pushed to Foundry will REMAIN on the merit ` +
      `(visible as "GM Screen #${e.mod.id}: ...") and must be removed in ` +
      `Foundry manually if no longer wanted.`
    );
    if (!ok) return;
    await modifiers.delete(e.mod.id);
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
      foundryCapturedLabels: [],
    });
  }

  function damageSummary(): string {
    if (!character.health) return '—';
    const { superficial, aggravated } = character.health;
    if (superficial === 0 && aggravated === 0) return 'Dmg —';
    return `Dmg ${superficial}s/${aggravated}a`;
  }
</script>

<section
  class="row"
  data-source={character.source}
  data-character-source={character.source}
  data-character-source-id={character.source_id}
>
  <header>
    <h2>{character.name}</h2>
    <span class="source">{character.source}</span>
    {#if character.hunger != null}<span class="stat">Hunger {character.hunger}</span>{/if}
    {#if character.willpower}
      <span class="stat">WP {character.willpower.max - character.willpower.superficial - character.willpower.aggravated}/{character.willpower.max}</span>
    {/if}
    <span class="stat">{damageSummary()}</span>
    {#if character.source === 'foundry'}
      <button
        class="roll-trigger"
        aria-label="Roll for {character.name}"
        onclick={openRollPopover}
        type="button"
      >
        🎲 Roll
      </button>
      {#if rollPopoverOpen && rollPopoverAnchor}
        <RollDispatcherPopover
          {character}
          modifiers={charMods}
          anchor={rollPopoverAnchor}
          onclose={closeRollPopover}
        />
      {/if}
    {/if}
  </header>

  <div class="row-body">
  <ActiveEffectsSummary {character} modifiers={charMods} />
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
              foundryCapturedLabels: [],
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
        canPush={canPushFor(entry)}
        onPush={() => handlePush(entry)}
        canReset={canResetFor(entry)}
        onReset={() => handleReset(entry)}
        originTemplateName={entry.kind === 'materialized' && entry.mod.originTemplateId != null
          ? (statusTemplates.byId(entry.mod.originTemplateId)?.name ?? null)
          : null}
      />
    {/each}
    <button class="add-modifier" onclick={addFreeModifier}>+ Add modifier</button>
  </div>
  </div>

  {#if pushNotice}
    <p class="push-notice" class:ok={pushNotice.ok} class:err={!pushNotice.ok}>
      {pushNotice.text}
    </p>
  {/if}

  {#if editorOpen && editorTarget && popoverPos}
    <div class="popover-wrap" style="left: {popoverPos.left}px; top: {popoverPos.top}px;">
      <ModifierEffectEditor
        initialEffects={editorTarget.kind === 'materialized' ? editorTarget.mod.effects : []}
        initialTags={editorTarget.kind === 'materialized' ? editorTarget.mod.tags : []}
        onSave={saveEditor}
        onCancel={closeEditor}
        {character}
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

  .roll-trigger {
    background: var(--bg-sunken);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 4px;
    padding: 0.2rem 0.55rem;
    font-size: 0.8rem;
    cursor: pointer;
    margin-left: 0.5rem;
  }

  .roll-trigger:hover {
    background: var(--bg-raised);
  }

  /* Flex wrapper that places ActiveEffectsSummary on the left of the
     modifier carousel. Both children align top so a tall summary doesn't
     stretch the carousel. */
  .row-body {
    display: flex;
    align-items: flex-start;
    gap: 0.6rem;
  }
  .row-body > .modifier-row { flex: 1; min-width: 0; }

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

  .push-notice {
    font-size: 0.7rem;
    margin: 0.4rem 0 0 0;
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    background: var(--bg-input);
    color: var(--text-secondary);
  }
  .push-notice.ok  { color: var(--text-primary); border-left: 2px solid var(--accent-bright); }
  .push-notice.err { color: var(--accent-amber);  border-left: 2px solid var(--accent-amber); }

  @media (prefers-reduced-motion: reduce) {
    .modifier-row {
      --card-overlap: 0;
      --card-shift-delta: 0;
    }
  }

  /* Banner-click navigate target — `flash-target` is added imperatively
     by +layout.svelte's toolEvents subscriber, so the rule must be
     :global() to escape Svelte's component-scoped class hashing.
     rgba uses --alert-card-dossier's RGB (#d24545) as established
     inline-alpha precedent (no token exists for this opacity). */
  :global(.row.flash-target) {
    animation: flash-pulse 1.5s ease-out;
  }
  :global {
    @keyframes flash-pulse {
      0%   { box-shadow: 0 0 0 0   rgba(210, 69, 69, 0.6); }
      50%  { box-shadow: 0 0 0 6px rgba(210, 69, 69, 0); }
      100% { box-shadow: 0 0 0 0   rgba(210, 69, 69, 0); }
    }
  }
</style>
