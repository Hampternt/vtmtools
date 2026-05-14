<script lang="ts">
  import { modifiers } from '../../../store/modifiers.svelte';
  import { statusTemplates } from '../../../store/statusTemplates.svelte';
  import type {
    BridgeCharacter, CharacterModifier, ModifierEffect, FoundryItem, FoundryItemBonus,
    ModifierZone,
  } from '../../../types';
  import type { SavedCharacter } from '$lib/saved-characters/api';
  import ModifierCard from './ModifierCard.svelte';
  import ModifierEffectEditor from './ModifierEffectEditor.svelte';
  import CardOverlay from './CardOverlay.svelte';
  import RollDispatcherPopover from './RollDispatcherPopover.svelte';
  import ActiveEffectsSummary from './ActiveEffectsSummary.svelte';
  import DropZone from '$lib/components/dnd/DropZone.svelte';
  import type { DropTarget } from '$lib/dnd/types';

  interface Props {
    character: BridgeCharacter;
    activeFilterTags: Set<string>;
    showHidden: boolean;
    saved?: SavedCharacter | null;
  }
  let { character, activeFilterTags, showHidden, saved = null }: Props = $props();

  // DnD drop targets — one per zone, carrying the character so cross-row
  // moves (v2) can compare source.character to target.character. v1 only
  // emits move-zone actions (same-row); cross-row drops still register a
  // target but the matrix rejects them.
  let characterTarget = $derived<DropTarget>({ kind: 'character-zone', character });
  let situationalTarget = $derived<DropTarget>({ kind: 'situational-zone', character });

  // Editor popover state — anchored to the cog via getBoundingClientRect()
  // (spec §7.3 "anchored to the cog itself, not a modal"). popoverPos uses
  // viewport coords so the wrap is position:fixed.
  type EditorTarget = { kind: 'materialized', mod: CharacterModifier }
                    | { kind: 'virtual', virt: VirtualCard };
  let editorOpen = $state(false);
  let editorTarget = $state<EditorTarget | null>(null);

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
   *  item by its _id, keeping only `activeWhen.check === 'always'` (and
   *  bonuses with missing/null activeWhen — treated as always by Foundry).
   *  Filters out own-pushed bonuses. Returns [] when the item is gone or
   *  has no qualifying bonuses. */
  function bonusesFor(itemId: string): FoundryItemBonus[] {
    const item = advantageItems.find(it => it._id === itemId);
    if (!item) return [];
    const raw = (item.system as Record<string, unknown>)?.bonuses;
    if (!Array.isArray(raw)) return [];
    return (raw as FoundryItemBonus[]).filter(b => {
      // Drop own-pushed bonuses (would loop on re-pull).
      const src = b.source ?? '';
      for (const prefix of liveTagPrefixes) {
        if (src === prefix || src.startsWith(prefix + ':')) return false;
      }
      // Keep only always-active. Missing activeWhen treated as always per
      // Foundry behavior (defensive — WoD5e always writes activeWhen but
      // legacy data or other sources may omit it).
      const check = b.activeWhen?.check ?? 'always';
      return check === 'always';
    });
  }

  /** Returns the conditional bonuses (activeWhen.check != 'always') for an
   *  item, after own-push filtering. Used to render the "(N conditionals)"
   *  badge. */
  function conditionalsFor(itemId: string): FoundryItemBonus[] {
    const item = advantageItems.find(it => it._id === itemId);
    if (!item) return [];
    const raw = (item.system as Record<string, unknown>)?.bonuses;
    if (!Array.isArray(raw)) return [];
    return (raw as FoundryItemBonus[]).filter(b => {
      const src = b.source ?? '';
      for (const prefix of liveTagPrefixes) {
        if (src === prefix || src.startsWith(prefix + ':')) return false;
      }
      const check = b.activeWhen?.check ?? 'always';
      return check !== 'always';
    });
  }

  /** True when this materialized modifier is a saved Foundry override
   *  (created via "Save as local override" — `foundryCapturedLabels`
   *  non-empty AND advantage-bound). Drives the origin-marker asterisk
   *  on the card so the GM can tell at a glance that the displayed data
   *  comes from a saved local copy rather than a live read-through.
   *  See spec §3.3.
   */
  function isSavedOverride(mod: CharacterModifier): boolean {
    return mod.foundryCapturedLabels.length > 0 && mod.binding.kind === 'advantage';
  }

  // Build the card list per spec §8.1 (updated 2026-05-13 — advantage-orphan
  // branch removed; orphan reaping happens backend-side on deleteItem).
  type CardEntry =
    | { kind: 'materialized'; mod: CharacterModifier }
    | { kind: 'virtual'; virt: VirtualCard };

  let cardEntries = $derived.by((): CardEntry[] => {
    const entries: CardEntry[] = [];

    // (2) Walk advantage items, merging with materialized rows.
    for (const item of advantageItems) {
      const matched = charMods.find(m => m.binding.kind === 'advantage' && m.binding.item_id === item._id);
      if (matched) {
        entries.push({ kind: 'materialized', mod: matched });
      } else {
        entries.push({ kind: 'virtual', virt: {
          item,
          name: item.name,
          description: ((item.system as Record<string, unknown>)?.description as string | undefined) ?? '',
        }});
      }
    }

    // (3) Append free-floating modifiers. Advantage-bound orphans no longer
    // render here — the deleteItem hook in vtmtools-bridge triggers a DB delete
    // that arrives via `modifiers://rows-reaped`, removing them from the store.
    for (const m of charMods) {
      if (m.binding.kind === 'free') {
        entries.push({ kind: 'materialized', mod: m });
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

  // Shared filter/sort logic for both zones. visibleCards becomes two derivations.
  function filterAndSort(entries: CardEntry[]): CardEntry[] {
    return entries
      .filter(e => passesTagFilter(e) && passesHiddenFilter(e))
      .sort((a, b) => {
        const [ak, an] = sortKey(a);
        const [bk, bn] = sortKey(b);
        if (ak !== bk) return ak - bk;
        return an < bn ? -1 : an > bn ? 1 : 0;
      });
  }

  // Zone of a CardEntry: virtual cards are always character-zone (they derive
  // from a Foundry advantage item which is zone-locked to 'character').
  function entryZone(e: CardEntry): ModifierZone {
    return e.kind === 'virtual' ? 'character' : e.mod.zone;
  }

  let characterCards = $derived(filterAndSort(cardEntries.filter(e => entryZone(e) === 'character')));
  let situationalCards = $derived(filterAndSort(cardEntries.filter(e => entryZone(e) === 'situational')));

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

  /**
   * Save-as-local-override action. Distinct from `materialize`:
   *   - materialize: creates an empty modifier (no effects, no captured
   *     labels) on first user engagement; subsequent edits build up local
   *     effects from scratch.
   *   - saveAsOverride: snapshots the current always-active bonuses on the
   *     item into a CharacterModifier whose effects mirror the bonuses
   *     AND whose foundryCapturedLabels record the source-label set.
   *     Push then becomes surgical.
   */
  async function saveAsOverride(virt: VirtualCard): Promise<CharacterModifier> {
    const sourceBonuses = bonusesFor(virt.item._id);
    const effects: ModifierEffect[] = sourceBonuses.map(b => ({
      kind: 'pool',
      scope: null,
      delta: b.value,
      note: null,
      paths: b.paths,
    }));
    const capturedLabels = sourceBonuses.map(b => b.source ?? '');
    const created = await modifiers.add({
      source: character.source,
      sourceId: character.source_id,
      name: virt.name,
      description: virt.description,
      effects,
      binding: { kind: 'advantage', item_id: virt.item._id },
      tags: [],
      originTemplateId: null,
      foundryCapturedLabels: capturedLabels,
      zone: 'character',
    });
    // Flip is_active=true so the override is immediately applied in renders
    // that consume active modifiers (active-effects summary, deltas, etc.).
    await modifiers.setActive(created.id, true);
    return created;
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

  function openEditor(e: CardEntry, _anchor: HTMLElement): void {
    editorTarget = e.kind === 'materialized'
      ? { kind: 'materialized', mod: e.mod }
      : { kind: 'virtual', virt: e.virt };
    editorOpen = true;
  }

  function closeEditor(): void {
    editorOpen = false;
    editorTarget = null;
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

  async function addFreeModifier(zone: ModifierZone): Promise<void> {
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
      zone,
    });
  }

  async function handleHardDelete(mod: CharacterModifier): Promise<void> {
    const ok = confirm(`Delete "${mod.name}" permanently? This cannot be undone.`);
    if (!ok) return;
    await modifiers.delete(mod.id);
  }

  function damageSummary(): string {
    if (!character.health) return '—';
    const { superficial, aggravated } = character.health;
    if (superficial === 0 && aggravated === 0) return 'Dmg —';
    return `Dmg ${superficial}s/${aggravated}a`;
  }
</script>

{#snippet renderCard(entry: CardEntry)}
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
          zone: 'character',
          createdAt: '',
          updatedAt: '',
        }
      : entry.mod}
    isVirtual={entry.kind === 'virtual'}
    bonuses={entry.kind === 'virtual'
      ? bonusesFor(entry.virt.item._id)
      : entry.mod.binding.kind === 'advantage'
        ? bonusesFor(entry.mod.binding.item_id)
        : []}
    conditionalsSkipped={entry.kind === 'virtual'
      ? conditionalsFor(entry.virt.item._id)
      : entry.mod.binding.kind === 'advantage'
        ? conditionalsFor(entry.mod.binding.item_id)
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
    showOverride={entry.kind === 'materialized' ? isSavedOverride(entry.mod) : false}
    onSaveAsOverride={entry.kind === 'virtual'
      ? () => saveAsOverride(entry.virt).catch(err => console.error('[gm-screen] save-as-override failed:', err))
      : undefined}
    onDelete={entry.kind === 'materialized' && entry.mod.binding.kind === 'free'
      ? () => handleHardDelete(entry.mod)
      : undefined}
  />
{/snippet}

<section
  class="row"
  data-source={character.source}
  data-character-source={character.source}
  data-character-source-id={character.source_id}
>
  <header>
    <h2>{character.name}</h2>
    {#if saved?.deletedInVttAt}
      <span class="vtt-deleted-badge"
        title="Deleted in {character.source === 'foundry' ? 'Foundry' : 'Roll20'}">deleted</span>
    {/if}
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
  <div class="zone-stack">
    <div class="zone-column" data-zone="character">
      <div class="zone-label">Character</div>
      <DropZone target={characterTarget}>
        <div
          class="modifier-row"
          style="--cards: {characterCards.length};"
        >
          {#each characterCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
            {@render renderCard(entry)}
          {/each}
          <button class="add-modifier" onclick={() => addFreeModifier('character')}>+ Add modifier</button>
        </div>
      </DropZone>
    </div>
    <div class="zone-column" data-zone="situational">
      <div class="zone-label">Situational</div>
      <DropZone target={situationalTarget}>
        <div
          class="modifier-row"
          style="--cards: {situationalCards.length};"
        >
          {#each situationalCards as entry, i (entry.kind === 'virtual' ? `v-${entry.virt.item._id}` : `m-${entry.mod.id}`)}
            {@render renderCard(entry)}
          {/each}
          <button class="add-modifier" onclick={() => addFreeModifier('situational')}>+ Add modifier</button>
        </div>
      </DropZone>
    </div>
  </div>
  </div>

  {#if pushNotice}
    <p class="push-notice" class:ok={pushNotice.ok} class:err={!pushNotice.ok}>
      {pushNotice.text}
    </p>
  {/if}

  {#if editorTarget}
    {@const target = editorTarget}
    <CardOverlay
      bind:open={editorOpen}
      title={target.kind === 'materialized' ? target.mod.name : target.virt.name}
      onClose={closeEditor}
    >
      <ModifierEffectEditor
        initialEffects={target.kind === 'materialized' ? target.mod.effects : []}
        initialTags={target.kind === 'materialized' ? target.mod.tags : []}
        onSave={saveEditor}
        onCancel={closeEditor}
        {character}
      />
    </CardOverlay>
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
  .row-body > .zone-stack { flex: 1; min-width: 0; }

  .zone-stack {
    display: flex;
    flex: 1;
    gap: 0.6rem;
    min-width: 0;
  }
  .zone-column {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .zone-label {
    font-size: 0.6rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding-left: 0.25rem;
  }
  .zone-column[data-zone="situational"] .zone-label {
    color: var(--accent-situational-bright);
  }

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

  .vtt-deleted-badge {
    background: color-mix(in srgb, var(--text-muted) 40%, transparent);
    color: var(--text-primary);
    font-size: 0.6rem;
    padding: 0 0.4em;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-weight: 600;
  }
</style>
