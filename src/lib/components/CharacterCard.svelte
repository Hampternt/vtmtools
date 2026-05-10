<script lang="ts">
  import type {
    BridgeCharacter,
    FoundryItem,
    FoundryRaw,
    Roll20Raw,
    Roll20RawAttribute,
  } from '../../types';
  import {
    foundryFeatures,
    foundryEffects,
    foundryEffectIsActive,
    foundryAttrInt,
    foundrySkillInt,
    foundryRaw,
  } from '$lib/foundry/raw';
  import { FOUNDRY_SKILL_NAMES } from '$lib/foundry/canonical-names';
  import {
    characterRemoveAdvantage,
    characterAddAdvantage,
    characterSetField,
  } from '$lib/character/api';
  import type { FeatureType, CanonicalFieldName } from '$lib/character/api';
  import { modifiers as modifiersStore } from '../../store/modifiers.svelte';
  import { computeActiveDeltas, activeAdvantageItemIds } from '$lib/character/active-deltas';
  import ModifierEffectEditor from './gm-screen/ModifierEffectEditor.svelte';
  import type { ModifierEffect } from '../../types';
  import { onMount } from 'svelte';

  interface Props {
    character: BridgeCharacter;
    viewIndex?: number;
    onViewChange?: (i: number) => void;
  }
  let { character, viewIndex: viewIndexProp, onViewChange }: Props = $props();

  // Uncontrolled mode default; controlled when caller passes viewIndex.
  let internalView = $state(1);
  const viewIndex = $derived(viewIndexProp ?? internalView);

  function setView(next: number) {
    if (next < 1 || next > 4) return;
    if (onViewChange) onViewChange(next);
    else internalView = next;
  }
  function flip(dir: 1 | -1) {
    setView(((viewIndex - 1 + dir + 4) % 4) + 1);
  }

  // ── Modifier subscription ─────────────────────────────────────────────
  onMount(() => { void modifiersStore.ensureLoaded(); });

  const characterModifiers = $derived(
    modifiersStore.list.filter(
      m => m.source === character.source && m.sourceId === character.source_id,
    ),
  );
  const activeDeltas = $derived(
    computeActiveDeltas(character, modifiersStore.list),
  );
  const activeChipIds = $derived(
    activeAdvantageItemIds(character, modifiersStore.list),
  );
  const hasActiveModifiers = $derived(
    characterModifiers.some(m => m.isActive),
  );

  function deltaTooltip(path: string): string {
    const entry = activeDeltas.get(path);
    if (!entry) return '';
    return entry.sources
      .map(s => `${s.modifierName}${s.scope ? ' — ' + s.scope : ''} (${entry.delta >= 0 ? '+' : ''}${entry.delta})`)
      .join('\n');
  }

  // ── Chip click — toggle activation ────────────────────────────────────
  let chipBusy = $state<string | null>(null);

  async function toggleChip(itemId: string, name: string, description: string) {
    if (character.source !== 'foundry') return; // Roll20 chip toggle deferred to Phase 2.5.
    chipBusy = itemId;
    try {
      const existing = characterModifiers.find(
        m => m.binding.kind === 'advantage' && m.binding.item_id === itemId,
      );
      if (existing) {
        await modifiersStore.setActive(existing.id, !existing.isActive);
      } else {
        const created = await modifiersStore.materializeAdvantage({
          source: character.source,
          sourceId: character.source_id,
          itemId,
          name,
          description,
        });
        await modifiersStore.setActive(created.id, true);
      }
    } catch (e) {
      console.error('[CharacterCard] toggleChip failed:', e);
      window.alert(String(e));
    } finally {
      if (chipBusy === itemId) chipBusy = null;
    }
  }

  // ── Chip right-click — open editor popover ────────────────────────────
  let editorTarget = $state<{ itemId: string; name: string; description: string; effects: ModifierEffect[]; tags: string[] } | null>(null);

  async function openChipEditor(itemId: string, name: string, description: string, ev: Event) {
    ev.preventDefault(); // suppress browser context menu
    if (character.source !== 'foundry') return;
    let modifier = characterModifiers.find(
      m => m.binding.kind === 'advantage' && m.binding.item_id === itemId,
    );
    if (!modifier) {
      modifier = await modifiersStore.materializeAdvantage({
        source: character.source,
        sourceId: character.source_id,
        itemId,
        name,
        description,
      });
    }
    editorTarget = {
      itemId,
      name: modifier.name,
      description: modifier.description,
      effects: modifier.effects.map(e => ({ ...e })),
      tags: [...modifier.tags],
    };
  }

  async function saveChipEditor(effects: ModifierEffect[], tags: string[]) {
    if (!editorTarget) return;
    const modifier = characterModifiers.find(
      m => m.binding.kind === 'advantage' && m.binding.item_id === editorTarget!.itemId,
    );
    if (!modifier) return;
    await modifiersStore.update(modifier.id, { effects, tags });
    editorTarget = null;
  }

  function closeChipEditor() { editorTarget = null; }

  const VIEW_LABELS = ['Basics', 'Stats', 'Disciplines', 'Advantages'] as const;

  // ── Stat editor (ported from Campaign.svelte) ─────────────────────────
  const FIELD_RANGES: Partial<Record<CanonicalFieldName, [number, number]>> = {
    hunger:                [0, 5],
    humanity:              [0, 10],
    humanity_stains:       [0, 10],
    blood_potency:         [0, 10],
    health_superficial:    [0, 20],
    health_aggravated:     [0, 20],
    willpower_superficial: [0, 20],
    willpower_aggravated:  [0, 20],
  };

  function liveEditAllowed(c: BridgeCharacter): boolean {
    return c.source === 'foundry';
  }
  function advantageEditAllowed(c: BridgeCharacter): boolean {
    return c.source === 'foundry';
  }

  let busyKey = $state<string | null>(null);
  function stepperKey(c: BridgeCharacter, field: CanonicalFieldName): string {
    return `${c.source}:${c.source_id}:${field}`;
  }

  async function tweakField(
    c: BridgeCharacter,
    field: CanonicalFieldName,
    delta: number,
    current: number,
  ) {
    const range = FIELD_RANGES[field];
    if (!range) return;
    const next = Math.max(range[0], Math.min(range[1], current + delta));
    if (next === current) return;
    const key = stepperKey(c, field);
    busyKey = key;
    try {
      await characterSetField('live', c.source, c.source_id, field, next);
    } catch (e) {
      console.error('[CharacterCard] characterSetField failed:', e);
      window.alert(String(e));
    } finally {
      if (busyKey === key) busyKey = null;
    }
  }

  // ── Source-aware readers (ported from Campaign.svelte) ────────────────
  function r20Attrs(c: BridgeCharacter): Roll20RawAttribute[] {
    if (c.source !== 'roll20') return [];
    const raw = c.raw as Roll20Raw | null;
    return raw?.attributes ?? [];
  }
  function r20AttrInt(c: BridgeCharacter, name: string): number {
    const a = r20Attrs(c).find(a => a.name === name);
    return a ? (parseInt(a.current, 10) || 0) : 0;
  }
  function r20AttrText(c: BridgeCharacter, name: string): string {
    return r20Attrs(c).find(a => a.name === name)?.current ?? '';
  }
  function attrInt(c: BridgeCharacter, name: string): number {
    if (c.source === 'foundry') return foundryAttrInt(c, name);
    return r20AttrInt(c, name);
  }
  function skillInt(c: BridgeCharacter, name: string): number {
    if (c.source === 'foundry') return foundrySkillInt(c, name);
    return 0;
  }
  function parseDisciplines(c: BridgeCharacter): { type: string; level: number }[] {
    const attrs = r20Attrs(c);
    const prefix = 'repeating_disciplines_';
    const suffix = '_discipline';
    return attrs
      .filter(a => a.name.startsWith(prefix) && a.name.endsWith(suffix) && !a.name.includes('_power_'))
      .map(a => {
        const id = a.name.slice(prefix.length, -suffix.length);
        const nameAttr = attrs.find(x => x.name === `${prefix}${id}_discipline_name`);
        return { type: nameAttr?.current ?? '', level: parseInt(a.current, 10) || 0 };
      })
      .filter(d => d.type && d.level > 0);
  }
  /// Foundry disciplines live under `system.disciplines` as a flat keyed map
  /// of `{ value, powers, ... }`. Convert to the same `{ type, level }[]`
  /// shape `parseDisciplines` returns for Roll20.
  function foundryDisciplines(c: BridgeCharacter): { type: string; level: number }[] {
    const raw = foundryRaw(c);
    if (!raw) return [];
    const map = raw.system?.disciplines as Record<string, { value?: number }> | undefined;
    if (!map) return [];
    return Object.entries(map)
      .map(([k, v]) => ({
        type: k.charAt(0).toUpperCase() + k.slice(1),
        level: v?.value ?? 0,
      }))
      .filter(d => d.level > 0);
  }
  /// Foundry clan: read `system.headers.clan` if present, fallback empty.
  /// (Foundry-VTM5e has the clan stored under headers — not exposed in the
  /// canonical paths reference but observed via `system.clan` on some sheets;
  /// fall through gracefully.)
  function foundryClan(c: BridgeCharacter): string {
    const raw = foundryRaw(c);
    if (!raw) return '';
    const sys = raw.system as Record<string, unknown> | undefined;
    const clan = sys?.clan as string | undefined;
    if (typeof clan === 'string' && clan.trim().length > 0) return clan;
    const headers = sys?.headers as Record<string, unknown> | undefined;
    const headerClan = headers?.clan as string | undefined;
    return typeof headerClan === 'string' ? headerClan : '';
  }
  function foundryGeneration(c: BridgeCharacter): string {
    const raw = foundryRaw(c);
    if (!raw) return '';
    const blood = raw.system?.blood as Record<string, unknown> | undefined;
    const gen = blood?.generation;
    if (typeof gen === 'string') return gen;
    if (typeof gen === 'number') return String(gen);
    return '';
  }
  function isPC(c: BridgeCharacter): boolean {
    return c.controlled_by !== null && c.controlled_by.trim() !== '';
  }
  function fileLabel(c: BridgeCharacter): string {
    const tail = c.source_id.slice(-4).toUpperCase();
    return `Camarilla file 0x${tail}`;
  }
  function dots(filled: number, total: number): boolean[] {
    return Array.from({ length: total }, (_, i) => i < filled);
  }

  // ── Advantage add/remove (ported from Campaign.svelte) ────────────────
  let busyAdvantageKey = $state<string | null>(null);
  function advantageBusyKey(c: BridgeCharacter, itemId: string): string {
    return `${c.source}:${c.source_id}:${itemId}`;
  }
  async function removeAdvantage(c: BridgeCharacter, ft: FeatureType, item: FoundryItem) {
    if (!advantageEditAllowed(c)) return;
    if (!window.confirm(`Remove ${ft} '${item.name}'?`)) return;
    const key = advantageBusyKey(c, item._id);
    busyAdvantageKey = key;
    try {
      await characterRemoveAdvantage('live', c.source, c.source_id, ft, item._id);
    } catch (e) {
      console.error('[CharacterCard] removeAdvantage failed:', e);
      window.alert(String(e));
    } finally {
      if (busyAdvantageKey === key) busyAdvantageKey = null;
    }
  }

  type AddFormState = {
    charKey: string;
    featuretype: FeatureType;
    name: string;
    points: number;
    description: string;
    submitting: boolean;
  };
  let addForm = $state<AddFormState | null>(null);
  function startAdd(c: BridgeCharacter, ft: FeatureType) {
    if (!advantageEditAllowed(c)) return;
    addForm = {
      charKey: `${c.source}:${c.source_id}`, featuretype: ft,
      name: '', points: 0, description: '', submitting: false,
    };
  }
  function cancelAdd() { addForm = null; }
  function isAddActive(c: BridgeCharacter, ft: FeatureType): boolean {
    if (!addForm) return false;
    return addForm.charKey === `${c.source}:${c.source_id}` && addForm.featuretype === ft;
  }
  function addFormValid(f: AddFormState): boolean {
    return f.name.trim().length > 0 && f.points >= 0 && f.points <= 10;
  }
  async function submitAdd(c: BridgeCharacter) {
    if (!addForm) return;
    if (!isAddActive(c, addForm.featuretype)) return;
    if (!addFormValid(addForm)) return;
    addForm.submitting = true;
    try {
      await characterAddAdvantage(
        'live', c.source, c.source_id,
        addForm.featuretype, addForm.name.trim(),
        addForm.description, addForm.points,
      );
      addForm = null;
    } catch (e) {
      console.error('[CharacterCard] characterAddAdvantage failed:', e);
      window.alert(String(e));
      if (addForm) addForm.submitting = false;
    }
  }

  // ── Computed view-data ────────────────────────────────────────────────
  // Note: BridgeCharacter uses snake_case (mirrors Rust serde shape), so
  // field reads are `blood_potency` / `humanity_stains`, not the camelCase
  // the original plan draft showed. See src/types.ts.
  const hunger      = $derived(character.hunger ?? 0);
  const bp          = $derived(character.blood_potency ?? 0);
  const humanity    = $derived(character.humanity ?? 0);
  const stains      = $derived(character.humanity_stains ?? 0);
  const healthMax   = $derived(character.health?.max ?? 0);
  const healthSup   = $derived(character.health?.superficial ?? 0);
  const healthAgg   = $derived(character.health?.aggravated ?? 0);
  const wpMax       = $derived(character.willpower?.max ?? 0);
  const wpSup       = $derived(character.willpower?.superficial ?? 0);
  const wpAgg       = $derived(character.willpower?.aggravated ?? 0);
  const healthOk    = $derived(Math.max(0, healthMax - healthSup - healthAgg));
  const wpOk        = $derived(Math.max(0, wpMax - wpSup - wpAgg));
  const clan        = $derived(
    character.source === 'foundry'
      ? foundryClan(character)
      : r20AttrText(character, 'clan'),
  );
  const generation  = $derived(
    character.source === 'foundry' ? foundryGeneration(character) : '',
  );
  const disciplines = $derived(
    character.source === 'foundry'
      ? foundryDisciplines(character)
      : parseDisciplines(character),
  );
</script>

{#snippet stepper(c: BridgeCharacter, field: CanonicalFieldName, current: number)}
  {@const allowed   = liveEditAllowed(c)}
  {@const key       = stepperKey(c, field)}
  {@const busy      = busyKey === key}
  {@const range     = FIELD_RANGES[field]}
  {#if range}
    {@const atFloor   = current <= range[0]}
    {@const atCeiling = current >= range[1]}
    {@const tooltip   = allowed ? '' : 'Roll20 live editing not supported (Phase 2.5)'}
    <span class="stat-stepper" class:roll20-blocked={!allowed} aria-disabled={!allowed}>
      <button type="button" class="step-btn"
        onclick={() => tweakField(c, field, -1, current)}
        disabled={!allowed || busy || atFloor}
        aria-busy={busy} title={tooltip} aria-label={`Decrease ${field}`}>−</button>
      <button type="button" class="step-btn"
        onclick={() => tweakField(c, field, +1, current)}
        disabled={!allowed || busy || atCeiling}
        aria-busy={busy} title={tooltip} aria-label={`Increase ${field}`}>+</button>
    </span>
  {/if}
{/snippet}

{#snippet chipRemoveBtn(c: BridgeCharacter, ft: FeatureType, item: FoundryItem)}
  {@const allowed = advantageEditAllowed(c)}
  {@const busy    = busyAdvantageKey === advantageBusyKey(c, item._id)}
  {#if allowed}
    <button type="button" class="chip-remove-btn"
      onclick={(ev) => { ev.stopPropagation(); removeAdvantage(c, ft, item); }}
      disabled={busy} aria-busy={busy}
      title={`Remove ${ft}`} aria-label={`Remove ${ft} ${item.name}`}>×</button>
  {/if}
{/snippet}

{#snippet addBtn(c: BridgeCharacter, ft: FeatureType)}
  {#if advantageEditAllowed(c) && !isAddActive(c, ft)}
    <button type="button" class="feat-chip add-chip"
      onclick={() => startAdd(c, ft)}
      title={`Add ${ft}`}>+ Add {ft}</button>
  {/if}
{/snippet}

{#snippet addForm_(c: BridgeCharacter, ft: FeatureType)}
  {#if addForm && isAddActive(c, ft)}
    <form class="add-form" onsubmit={(e) => { e.preventDefault(); void submitAdd(c); }}>
      <div class="form-row">
        <label for={`add-name-${addForm.charKey}-${ft}`}>Name</label>
        <input id={`add-name-${addForm.charKey}-${ft}`} type="text"
          bind:value={addForm.name} maxlength="120" required autofocus />
      </div>
      <div class="form-row">
        <label for={`add-points-${addForm.charKey}-${ft}`}>Points</label>
        <input id={`add-points-${addForm.charKey}-${ft}`} type="number"
          min="0" max="10" bind:value={addForm.points} />
      </div>
      <div class="form-row">
        <label for={`add-desc-${addForm.charKey}-${ft}`}>Description</label>
        <textarea id={`add-desc-${addForm.charKey}-${ft}`}
          bind:value={addForm.description} rows="2"></textarea>
      </div>
      <div class="form-actions">
        <button type="submit" class="btn-save"
          disabled={!addFormValid(addForm) || addForm.submitting}
          aria-busy={addForm.submitting}>Add</button>
        <button type="button" class="btn-save"
          onclick={cancelAdd} disabled={addForm.submitting}>Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}

<div class="dossier" data-pc={isPC(character)}>
  <div class="file-label">{fileLabel(character)}</div>

  <header class="name-row">
    <span class="name">{character.name}</span>
    <span class="badge" class:pc={isPC(character)} class:npc={!isPC(character)}>
      {isPC(character) ? 'PC' : 'NPC'}
    </span>
  </header>

  <div class="clan-line">
    {#if clan}{clan}{/if}{#if clan && generation} · {/if}{#if generation}{generation} generation{/if}
  </div>

  <div class="panel">
    {#if hasActiveModifiers}
      <div class="modifier-banner" title="Active modifiers on this character">
        <span class="banner-label">Active modifiers</span>
        <span class="banner-count">{characterModifiers.filter(m => m.isActive).length}</span>
      </div>
    {/if}
    {#if viewIndex === 1}
      <div class="basics">
        <div class="vital-row">
          <div class="hunger-cluster">
            <div class="hunger-drops">
              {#each dots(hunger, 5) as filled}
                <svg class="blood-drop" class:filled viewBox="0 0 24 32" xmlns="http://www.w3.org/2000/svg">
                  <path d="M12 2C12 2 4 14 4 20a8 8 0 0 0 16 0c0-6-8-18-8-18z" />
                </svg>
              {/each}
            </div>
            {@render stepper(character, 'hunger', hunger)}
          </div>
          <div class="bp-pill">
            <span class="qs-label">BP</span>
            <span class="bp-value">{bp}</span>
            {@render stepper(character, 'blood_potency', bp)}
          </div>
        </div>

        <div class="block">
          <div class="track-label">Conscience</div>
          <div class="conscience-track">
            {#each 'CONSCIENCE'.split('') as letter, i}
              {@const pos = i + 1}
              {@const isFilled = pos <= humanity}
              {@const isStained = pos > 10 - stains}
              <span class="conscience-letter"
                    class:filled={isFilled}
                    class:stained={isStained && !isFilled}>{letter}</span>
            {/each}
          </div>
          <div class="ctrl-grid">
            <span class="ctrl-label">Humanity</span>
            {@render stepper(character, 'humanity', humanity)}
            <span class="ctrl-label">Stains</span>
            {@render stepper(character, 'humanity_stains', stains)}
          </div>
        </div>

        <div class="block">
          <div class="track-label">Health</div>
          <div class="track-boxes">
            {#each Array.from({ length: healthMax }, (_, i) => i) as i}
              <div class="box health"
                   class:superficial={i >= healthOk && i < healthOk + healthSup}
                   class:aggravated={i >= healthOk + healthSup}></div>
            {/each}
          </div>
          <div class="ctrl-grid">
            <span class="ctrl-label">Sup</span>
            {@render stepper(character, 'health_superficial', healthSup)}
            <span class="ctrl-label">Agg</span>
            {@render stepper(character, 'health_aggravated', healthAgg)}
          </div>
        </div>

        <div class="block">
          <div class="track-label">Willpower</div>
          <div class="track-boxes">
            {#each Array.from({ length: wpMax }, (_, i) => i) as i}
              <div class="box willpower"
                   class:filled={i < wpOk}
                   class:superficial={i >= wpOk && i < wpOk + wpSup}
                   class:aggravated={i >= wpOk + wpSup}></div>
            {/each}
          </div>
          <div class="ctrl-grid">
            <span class="ctrl-label">Sup</span>
            {@render stepper(character, 'willpower_superficial', wpSup)}
            <span class="ctrl-label">Agg</span>
            {@render stepper(character, 'willpower_aggravated', wpAgg)}
          </div>
        </div>
      </div>
    {:else if viewIndex === 2}
      {@const ATTR_NAMES = [
        ['strength', 'STR'], ['dexterity', 'DEX'], ['stamina', 'STA'],
        ['charisma', 'CHA'], ['manipulation', 'MAN'], ['composure', 'COM'],
        ['intelligence', 'INT'], ['wits', 'WIT'], ['resolve', 'RES'],
      ] as const}
      {@const skills = character.source === 'foundry'
        ? FOUNDRY_SKILL_NAMES
            .map((name) => ({ name, value: foundrySkillInt(character, name), specialties: [] as string[] }))
            .filter((s) => s.value > 0 || s.specialties.length > 0)
        : []}
      {@const hiddenSkillCount = (character.source === 'foundry' ? FOUNDRY_SKILL_NAMES.length : 0) - skills.length}
      <div class="stats">
        <div class="panel-title">Attributes</div>
        <div class="attr-grid">
          {#each ATTR_NAMES as [n, abbr]}
            {@const path = `attributes.${n}`}
            {@const delta = activeDeltas.get(path)}
            <div class="attr-cell" data-path={path} class:modified={!!delta} title={deltaTooltip(path)}>
              <span class="attr-name">{abbr}</span>
              {#if delta}
                <span class="attr-val">
                  {delta.modified}<span class="delta-badge">{delta.delta > 0 ? '+' : ''}{delta.delta}</span>
                </span>
              {:else}
                <span class="attr-val">{attrInt(character, n)}</span>
              {/if}
            </div>
          {/each}
        </div>
        <hr />
        <div class="panel-title">Skills</div>
        {#if skills.length === 0}
          <div class="skills-empty">No skills with non-zero levels.</div>
        {:else}
          <div class="skills">
            {#each skills as s}
              {@const sPath = `skills.${s.name}`}
              {@const sDelta = activeDeltas.get(sPath)}
              <div class="skill-row" data-path={sPath} class:modified={!!sDelta} title={deltaTooltip(sPath)}>
                <span class="skill-name">{s.name}</span>
                {#if sDelta}
                  <span class="skill-val">
                    {sDelta.modified}<span class="delta-badge">{sDelta.delta > 0 ? '+' : ''}{sDelta.delta}</span>
                  </span>
                {:else}
                  <span class="skill-val">{s.value}</span>
                {/if}
              </div>
            {/each}
          </div>
        {/if}
        {#if hiddenSkillCount > 0}
          <div class="skills-note">Hidden: {hiddenSkillCount} skills at zero</div>
        {/if}
      </div>
    {:else if viewIndex === 3}
      {@const fRaw = foundryRaw(character)}
      {@const powerItems = fRaw && Array.isArray(fRaw.items)
        ? fRaw.items.filter((it) => it?.type === 'power')
        : []}
      {@const grouped = (() => {
        const m: Record<string, { name: string; level: number; powers: string[] }> = {};
        for (const d of disciplines) {
          m[d.type.toLowerCase()] = { name: d.type, level: d.level, powers: [] };
        }
        for (const p of powerItems) {
          const sys = p?.system as Record<string, unknown> | undefined;
          const key = String(sys?.discipline ?? '').toLowerCase();
          if (m[key]) m[key].powers.push(p?.name ?? '');
        }
        return Object.values(m).sort((a, b) => a.name.localeCompare(b.name));
      })()}
      <div class="disc">
        {#if grouped.length === 0}
          <div class="disc-empty">No disciplines on this character.</div>
        {/if}
        {#each grouped as d, idx}
          {#if idx > 0}<hr />{/if}
          <div class="disc-row">
            <div class="disc-name">
              <span>{d.name}</span>
              <span class="dots">{'●'.repeat(Math.min(d.level, 5))}</span>
            </div>
            {#if d.powers.length > 0}
              <div class="powers">
                {#each d.powers as p}
                  <div class="power">{p}</div>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {:else if viewIndex === 4}
      {@const merits      = character.source === 'foundry' ? foundryFeatures(character, 'merit')      : []}
      {@const flaws       = character.source === 'foundry' ? foundryFeatures(character, 'flaw')       : []}
      {@const backgrounds = character.source === 'foundry' ? foundryFeatures(character, 'background') : []}
      {@const boons       = character.source === 'foundry' ? foundryFeatures(character, 'boon')       : []}
      {@const actorFx     = character.source === 'foundry' ? foundryEffects(character)                : []}
      <div class="adv">
        {#if merits.length > 0 || advantageEditAllowed(character)}
          <div class="adv-section">
            <div class="adv-label">Merits</div>
            <div class="chips">
              {#each merits as m}
                {@const points = (m.system?.points as number | undefined) ?? 0}
                <span
                  class="chip merit"
                  data-active={activeChipIds.has(m._id)}
                  data-item-id={m._id}
                  data-busy={chipBusy === m._id}
                  role="button" tabindex="0"
                  onclick={() => toggleChip(m._id, m.name, '')}
                  oncontextmenu={(ev) => openChipEditor(m._id, m.name, '', ev)}
                  onkeydown={(ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); toggleChip(m._id, m.name, ''); } }}
                >
                  <span class="feat-name">{m.name}</span>
                  {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
                  {@render chipRemoveBtn(character, 'merit', m)}
                </span>
              {/each}
              {@render addBtn(character, 'merit')}
            </div>
            {@render addForm_(character, 'merit')}
          </div>
        {/if}
        {#if flaws.length > 0 || advantageEditAllowed(character)}
          <div class="adv-section">
            <div class="adv-label">Flaws</div>
            <div class="chips">
              {#each flaws as f}
                {@const points = (f.system?.points as number | undefined) ?? 0}
                <span
                  class="chip flaw"
                  data-active={activeChipIds.has(f._id)}
                  data-item-id={f._id}
                  data-busy={chipBusy === f._id}
                  role="button" tabindex="0"
                  onclick={() => toggleChip(f._id, f.name, '')}
                  oncontextmenu={(ev) => openChipEditor(f._id, f.name, '', ev)}
                  onkeydown={(ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); toggleChip(f._id, f.name, ''); } }}
                >
                  <span class="feat-name">{f.name}</span>
                  {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
                  {@render chipRemoveBtn(character, 'flaw', f)}
                </span>
              {/each}
              {@render addBtn(character, 'flaw')}
            </div>
            {@render addForm_(character, 'flaw')}
          </div>
        {/if}
        {#if backgrounds.length > 0 || advantageEditAllowed(character)}
          <div class="adv-section">
            <div class="adv-label">Backgrounds</div>
            <div class="chips">
              {#each backgrounds as b}
                {@const points = (b.system?.points as number | undefined) ?? 0}
                <span
                  class="chip bg"
                  data-active={activeChipIds.has(b._id)}
                  data-item-id={b._id}
                  data-busy={chipBusy === b._id}
                  role="button" tabindex="0"
                  onclick={() => toggleChip(b._id, b.name, '')}
                  oncontextmenu={(ev) => openChipEditor(b._id, b.name, '', ev)}
                  onkeydown={(ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); toggleChip(b._id, b.name, ''); } }}
                >
                  <span class="feat-name">{b.name}</span>
                  {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
                  {@render chipRemoveBtn(character, 'background', b)}
                </span>
              {/each}
              {@render addBtn(character, 'background')}
            </div>
            {@render addForm_(character, 'background')}
          </div>
        {/if}
        {#if boons.length > 0 || advantageEditAllowed(character)}
          <div class="adv-section">
            <div class="adv-label">Boons</div>
            <div class="chips">
              {#each boons as bn}
                <span
                  class="chip boon"
                  data-active={activeChipIds.has(bn._id)}
                  data-item-id={bn._id}
                  data-busy={chipBusy === bn._id}
                  role="button" tabindex="0"
                  onclick={() => toggleChip(bn._id, bn.name, '')}
                  oncontextmenu={(ev) => openChipEditor(bn._id, bn.name, '', ev)}
                  onkeydown={(ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); toggleChip(bn._id, bn.name, ''); } }}
                >
                  <span class="feat-name">{bn.name}</span>
                  {@render chipRemoveBtn(character, 'boon', bn)}
                </span>
              {/each}
              {@render addBtn(character, 'boon')}
            </div>
            {@render addForm_(character, 'boon')}
          </div>
        {/if}
        {#if actorFx.length > 0}
          <div class="adv-section">
            <div class="adv-label">Active modifiers (actor)</div>
            <div class="chips">
              {#each actorFx as e}
                {@const active = foundryEffectIsActive(e)}
                <span class="chip effect" class:disabled={!active}
                      title={e.changes?.map(c => `${c.key} mode=${c.mode} value=${c.value}`).join('\n') ?? ''}>
                  <span class="feat-name">{e.name}</span>
                  <span class="fx-badge">{e.changes?.length ?? 0}</span>
                </span>
              {/each}
            </div>
          </div>
        {/if}
        {#if merits.length === 0 && flaws.length === 0 && backgrounds.length === 0 && boons.length === 0 && actorFx.length === 0}
          <div class="adv-empty">No merits, flaws, backgrounds, or modifiers.</div>
        {/if}
      </div>
    {/if}
  </div>

  <footer class="flipper">
    <button class="flip-arrow" type="button" aria-label="Previous view"
            onclick={() => flip(-1)}>‹</button>
    <span class="flip-current">{VIEW_LABELS[viewIndex - 1]}</span>
    <span class="flip-pager">{viewIndex} / 4</span>
    <button class="flip-arrow" type="button" aria-label="Next view"
            onclick={() => flip(+1)}>›</button>
  </footer>
</div>

{#if editorTarget}
  <div class="editor-overlay" onclick={closeChipEditor} role="presentation">
    <div class="editor-anchor" onclick={(ev) => ev.stopPropagation()} role="presentation">
      <ModifierEffectEditor
        initialEffects={editorTarget.effects}
        initialTags={editorTarget.tags}
        onSave={saveChipEditor}
        onCancel={closeChipEditor}
      />
    </div>
  </div>
{/if}

<style>
  /* ── Card frame: 2:3 aspect, scales by --card-scale ─────────────────── */
  .dossier {
    width: calc(280px * var(--card-scale, 1));
    aspect-ratio: 2 / 3;
    background: var(--bg-card-dossier);
    color: var(--text-card-dossier);
    border: 1px solid var(--rule-card-dossier-dashed);
    border-radius: calc(4px * var(--card-scale, 1));
    box-shadow: var(--shadow-card-dossier);
    padding: calc(12px * var(--card-scale, 1)) calc(14px * var(--card-scale, 1));
    display: flex;
    flex-direction: column;
    gap: calc(8px * var(--card-scale, 1));
    position: relative;
    box-sizing: border-box;
    font-family: 'Inter', system-ui, sans-serif;
    overflow: hidden;
  }
  .dossier *, .dossier *::before, .dossier *::after { box-sizing: border-box; }

  /* Inner dashed institutional border (decorative). */
  .dossier::before {
    content: '';
    position: absolute;
    inset: calc(8px * var(--card-scale, 1));
    border: 1px dashed var(--rule-card-dossier-dashed);
    border-radius: calc(2px * var(--card-scale, 1));
    pointer-events: none;
  }

  /* ── Header strip ───────────────────────────────────────────────────── */
  .file-label {
    color: var(--label-card-dossier);
    font-size: calc(8px * var(--card-scale, 1));
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  .name-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: calc(10px * var(--card-scale, 1));
  }
  .name {
    font-size: calc(13px * var(--card-scale, 1));
    font-weight: 600;
    color: var(--text-card-dossier);
  }
  .name::before {
    content: 'SUBJECT  ';
    color: var(--accent-card-dossier);
    font-size: calc(9px * var(--card-scale, 1));
    letter-spacing: 0.2em;
    font-weight: 500;
  }
  .badge {
    background: color-mix(in srgb, var(--accent-card-dossier) 18%, transparent);
    color: var(--accent-card-dossier);
    border: 1px solid var(--accent-card-dossier);
    font-size: calc(9px * var(--card-scale, 1));
    padding: calc(1px * var(--card-scale, 1)) calc(6px * var(--card-scale, 1));
    border-radius: 999px;
    letter-spacing: 0.08em;
  }
  .clan-line {
    color: var(--accent-card-dossier);
    font-size: calc(9px * var(--card-scale, 1));
    letter-spacing: 0.12em;
    text-transform: uppercase;
    min-height: calc(11px * var(--card-scale, 1));
  }

  /* ── Panel host (flexes to fill) ────────────────────────────────────── */
  .panel {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: calc(7px * var(--card-scale, 1));
    padding-top: calc(4px * var(--card-scale, 1));
    overflow: hidden;
  }
  .panel-title {
    color: var(--accent-card-dossier);
    font-size: calc(8px * var(--card-scale, 1));
    letter-spacing: 0.2em;
    text-transform: uppercase;
  }
  .dossier hr {
    border: none;
    border-top: 1px solid var(--rule-card-dossier);
    margin: calc(2px * var(--card-scale, 1)) 0;
  }

  /* ── View 1 — Basics ────────────────────────────────────────────────── */
  .basics {
    display: flex;
    flex-direction: column;
    gap: calc(7px * var(--card-scale, 1));
    overflow: hidden;
  }
  .vital-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: calc(8px * var(--card-scale, 1));
  }
  .hunger-cluster {
    display: flex;
    align-items: center;
    gap: calc(6px * var(--card-scale, 1));
  }
  .hunger-drops {
    display: flex;
    gap: calc(3px * var(--card-scale, 1));
  }
  .blood-drop {
    width: calc(11px * var(--card-scale, 1));
    height: calc(14px * var(--card-scale, 1));
    fill: color-mix(in srgb, var(--alert-card-dossier) 18%, transparent);
    flex: none;
  }
  .blood-drop.filled { fill: var(--alert-card-dossier); }
  .bp-pill {
    display: inline-flex;
    align-items: center;
    gap: calc(4px * var(--card-scale, 1));
    font-size: calc(9px * var(--card-scale, 1));
    letter-spacing: 0.1em;
    padding: calc(2px * var(--card-scale, 1)) calc(8px * var(--card-scale, 1));
    border: 1px solid var(--accent-card-dossier);
    color: var(--accent-card-dossier);
    border-radius: 999px;
  }
  .bp-pill .qs-label { font-weight: 500; }
  .bp-pill .bp-value {
    color: var(--text-card-dossier);
    font-weight: 700;
  }
  .block { display: flex; flex-direction: column; gap: calc(2px * var(--card-scale, 1)); }
  .track-label {
    font-size: calc(8px * var(--card-scale, 1));
    letter-spacing: 0.2em;
    color: color-mix(in srgb, var(--text-card-dossier) 55%, transparent);
    text-transform: uppercase;
    margin-bottom: calc(3px * var(--card-scale, 1));
  }
  .conscience-track {
    display: flex;
    gap: calc(3px * var(--card-scale, 1));
    justify-content: space-between;
    font-weight: 500;
    letter-spacing: 0.16em;
    font-size: calc(11px * var(--card-scale, 1));
  }
  .conscience-letter { color: color-mix(in srgb, var(--text-card-dossier) 20%, transparent); }
  .conscience-letter.filled { color: var(--text-card-dossier); }
  .conscience-letter.stained {
    color: var(--alert-card-dossier);
    text-decoration: line-through;
  }
  .track-boxes {
    display: flex;
    gap: calc(3px * var(--card-scale, 1));
    flex-wrap: wrap;
  }
  .box {
    width: calc(16px * var(--card-scale, 1));
    height: calc(16px * var(--card-scale, 1));
    border: 1px solid var(--accent-card-dossier);
    flex: none;
  }
  .box.health.superficial,
  .box.willpower.superficial {
    background: var(--alert-card-dossier);
    border-color: var(--alert-card-dossier);
  }
  .box.health.aggravated,
  .box.willpower.aggravated {
    background: var(--bg-card-dossier);
    box-shadow: inset 0 0 0 calc(2px * var(--card-scale, 1)) var(--alert-card-dossier);
    border-color: var(--alert-card-dossier);
  }
  .box.willpower.filled {
    background: var(--accent-card-dossier);
    border-color: var(--accent-card-dossier);
  }

  /* Stepper grid for control rows. */
  .ctrl-grid {
    display: grid;
    grid-template-columns: auto auto 1fr auto auto;
    align-items: center;
    gap: calc(4px * var(--card-scale, 1)) calc(6px * var(--card-scale, 1));
    margin-top: calc(2px * var(--card-scale, 1));
  }
  .ctrl-label {
    font-size: calc(8px * var(--card-scale, 1));
    letter-spacing: 0.16em;
    text-transform: uppercase;
    color: color-mix(in srgb, var(--text-card-dossier) 55%, transparent);
  }

  /* ── Steppers (slate-blue circles) ──────────────────────────────────── */
  .stat-stepper {
    display: inline-flex;
    gap: calc(2px * var(--card-scale, 1));
  }
  .step-btn {
    width: calc(14px * var(--card-scale, 1));
    height: calc(14px * var(--card-scale, 1));
    min-width: calc(14px * var(--card-scale, 1));
    border: 1px solid var(--accent-card-dossier);
    background: transparent;
    color: var(--accent-card-dossier);
    border-radius: 999px;
    cursor: pointer;
    font-size: calc(10px * var(--card-scale, 1));
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
  }
  .step-btn:hover:not(:disabled) {
    background: var(--accent-card-dossier);
    color: var(--bg-card-dossier);
  }
  .step-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .stat-stepper.roll20-blocked .step-btn { cursor: not-allowed; }

  /* ── View 2 — Stats ─────────────────────────────────────────────────── */
  .stats {
    display: flex;
    flex-direction: column;
    gap: calc(5px * var(--card-scale, 1));
    overflow: hidden;
    min-height: 0;
  }
  .attr-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: calc(4px * var(--card-scale, 1)) calc(6px * var(--card-scale, 1));
    font-size: calc(10px * var(--card-scale, 1));
    font-family: ui-monospace, monospace;
  }
  .attr-cell {
    display: flex;
    justify-content: space-between;
  }
  .attr-name { color: var(--text-card-dossier); }
  .attr-val {
    color: var(--alert-card-dossier);
    font-weight: 700;
  }
  .skills {
    /* Two-column grid — uses CSS Grid, not multi-column (ARCH §6 forbids
       multi-column due to animate:flip incompatibility). Row-major fill:
       skills go left-to-right alphabetically across each row pair. */
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: calc(2px * var(--card-scale, 1)) calc(8px * var(--card-scale, 1));
    font-size: calc(10px * var(--card-scale, 1));
    font-family: ui-monospace, monospace;
    overflow-y: auto;
    min-height: 0;
  }
  .skill-row {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .skill-name {
    color: var(--text-card-dossier);
    text-transform: capitalize;
  }
  .skill-val {
    color: var(--alert-card-dossier);
    font-weight: 700;
  }
  .skills-empty {
    font-size: calc(9px * var(--card-scale, 1));
    color: color-mix(in srgb, var(--text-card-dossier) 40%, transparent);
    font-style: italic;
  }
  .skills-note {
    font-size: calc(8px * var(--card-scale, 1));
    color: color-mix(in srgb, var(--text-card-dossier) 40%, transparent);
    letter-spacing: 0.08em;
    margin-top: calc(4px * var(--card-scale, 1));
  }

  /* ── View 3 — Disciplines ───────────────────────────────────────────── */
  .disc {
    display: flex;
    flex-direction: column;
    gap: calc(4px * var(--card-scale, 1));
    overflow-y: auto;
    min-height: 0;
  }
  .disc-row {
    font-size: calc(11px * var(--card-scale, 1));
  }
  .disc-name {
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    font-size: calc(10px * var(--card-scale, 1));
    color: var(--text-card-dossier);
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .disc-name .dots {
    color: var(--alert-card-dossier);
    letter-spacing: 0.1em;
  }
  .powers {
    display: flex;
    flex-direction: column;
    gap: calc(1px * var(--card-scale, 1));
    padding-left: calc(8px * var(--card-scale, 1));
    margin-top: calc(2px * var(--card-scale, 1));
  }
  .power {
    font-size: calc(9px * var(--card-scale, 1));
    color: color-mix(in srgb, var(--text-card-dossier) 70%, transparent);
  }
  .power::before {
    content: '› ';
    color: var(--accent-card-dossier);
  }
  .disc-empty {
    font-size: calc(9px * var(--card-scale, 1));
    color: color-mix(in srgb, var(--text-card-dossier) 40%, transparent);
    font-style: italic;
  }

  /* ── View 4 — Advantages ────────────────────────────────────────────── */
  .adv {
    display: flex;
    flex-direction: column;
    gap: calc(6px * var(--card-scale, 1));
    overflow-y: auto;
    min-height: 0;
  }
  .adv-section {
    display: flex;
    flex-direction: column;
    gap: calc(3px * var(--card-scale, 1));
  }
  .adv-label {
    font-size: calc(8px * var(--card-scale, 1));
    letter-spacing: 0.18em;
    color: var(--accent-card-dossier);
    text-transform: uppercase;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: calc(4px * var(--card-scale, 1));
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: calc(4px * var(--card-scale, 1));
    font-size: calc(9px * var(--card-scale, 1));
    padding: calc(2px * var(--card-scale, 1)) calc(7px * var(--card-scale, 1));
    border: 1px solid;
    border-radius: calc(3px * var(--card-scale, 1));
    letter-spacing: 0.04em;
    position: relative;
    transition: background 140ms ease, border-color 140ms ease, color 140ms ease;
  }
  .chip.merit {
    border-color: color-mix(in srgb, var(--accent-card-dossier) 50%, transparent);
    color: var(--text-card-dossier);
  }
  .chip.flaw {
    border-color: color-mix(in srgb, var(--alert-card-dossier) 50%, transparent);
    color: var(--alert-card-dossier);
  }
  .chip.bg {
    border-color: color-mix(in srgb, var(--text-label) 50%, transparent);
    color: var(--text-label);
  }
  .chip.boon {
    border-color: color-mix(in srgb, var(--accent-amber) 50%, transparent);
    color: var(--accent-amber);
  }
  .chip.effect {
    border-color: color-mix(in srgb, var(--accent-card-dossier) 30%, transparent);
    color: color-mix(in srgb, var(--text-card-dossier) 70%, transparent);
  }
  .chip.effect.disabled {
    opacity: 0.45;
    text-decoration: line-through;
  }
  .chip .dots {
    color: var(--alert-card-dossier);
    margin-left: calc(2px * var(--card-scale, 1));
  }

  /* Active-state — ships now so Plan B doesn't re-touch this file. The
     attribute is hardcoded data-active="false" in the markup; Plan B
     flips it to "true" once the modifier subscription is wired. Spec §5.4. */
  .chip[data-active="true"] {
    background: color-mix(in srgb, var(--alert-card-dossier) 20%, transparent);
    border-color: var(--alert-card-dossier);
    color: var(--text-card-dossier);
    box-shadow: 0 0 calc(6px * var(--card-scale, 1))
      color-mix(in srgb, var(--alert-card-dossier) 45%, transparent);
  }
  .chip[data-active="true"]::after {
    content: '◉';
    position: absolute;
    top: calc(-3px * var(--card-scale, 1));
    right: calc(-3px * var(--card-scale, 1));
    font-size: calc(7px * var(--card-scale, 1));
    color: var(--alert-card-dossier);
    line-height: 1;
  }

  .chip-remove-btn {
    background: none;
    border: none;
    color: inherit;
    opacity: 0.6;
    cursor: pointer;
    font-size: calc(11px * var(--card-scale, 1));
    line-height: 1;
    padding: 0 calc(1px * var(--card-scale, 1));
  }
  .chip-remove-btn:hover:not(:disabled) {
    opacity: 1;
    color: var(--alert-card-dossier);
  }
  .chip-remove-btn:disabled { opacity: 0.25; cursor: default; }

  .feat-chip.add-chip {
    background: transparent;
    border: 1px dashed color-mix(in srgb, var(--accent-card-dossier) 40%, transparent);
    color: var(--accent-card-dossier);
    font-size: calc(9px * var(--card-scale, 1));
    padding: calc(2px * var(--card-scale, 1)) calc(7px * var(--card-scale, 1));
    border-radius: calc(3px * var(--card-scale, 1));
    cursor: pointer;
    letter-spacing: 0.04em;
  }
  .feat-chip.add-chip:hover {
    background: color-mix(in srgb, var(--accent-card-dossier) 12%, transparent);
  }
  .fx-badge {
    background: color-mix(in srgb, var(--accent-card-dossier) 25%, transparent);
    color: var(--accent-card-dossier);
    border-radius: 999px;
    padding: 0 calc(4px * var(--card-scale, 1));
    font-size: calc(8px * var(--card-scale, 1));
    font-family: ui-monospace, monospace;
  }
  .adv-empty {
    font-size: calc(9px * var(--card-scale, 1));
    color: color-mix(in srgb, var(--text-card-dossier) 40%, transparent);
    font-style: italic;
  }

  /* ── Inline add-form (advantages) ───────────────────────────────────── */
  .add-form {
    display: flex;
    flex-direction: column;
    gap: calc(3px * var(--card-scale, 1));
    margin-top: calc(3px * var(--card-scale, 1));
    padding: calc(4px * var(--card-scale, 1)) calc(6px * var(--card-scale, 1));
    border: 1px solid var(--rule-card-dossier);
    border-radius: calc(3px * var(--card-scale, 1));
  }
  .form-row {
    display: grid;
    grid-template-columns: calc(50px * var(--card-scale, 1)) 1fr;
    gap: calc(4px * var(--card-scale, 1));
    align-items: center;
    font-size: calc(9px * var(--card-scale, 1));
  }
  .form-row label {
    color: var(--accent-card-dossier);
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-size: calc(8px * var(--card-scale, 1));
  }
  .form-row input,
  .form-row textarea {
    background: var(--bg-input);
    border: 1px solid var(--rule-card-dossier);
    color: var(--text-card-dossier);
    font-size: calc(10px * var(--card-scale, 1));
    padding: calc(2px * var(--card-scale, 1)) calc(4px * var(--card-scale, 1));
    border-radius: calc(2px * var(--card-scale, 1));
    font-family: inherit;
  }
  .form-row input:focus,
  .form-row textarea:focus {
    outline: none;
    border-color: var(--accent-card-dossier);
  }
  .form-actions {
    display: flex;
    gap: calc(4px * var(--card-scale, 1));
    justify-content: flex-end;
  }
  .btn-save {
    background: transparent;
    border: 1px solid var(--accent-card-dossier);
    color: var(--accent-card-dossier);
    font-size: calc(9px * var(--card-scale, 1));
    padding: calc(2px * var(--card-scale, 1)) calc(6px * var(--card-scale, 1));
    border-radius: calc(2px * var(--card-scale, 1));
    cursor: pointer;
    transition: background 140ms ease, color 140ms ease;
  }
  .btn-save:hover:not(:disabled) {
    background: var(--accent-card-dossier);
    color: var(--bg-card-dossier);
  }
  .btn-save:disabled { opacity: 0.4; cursor: default; }

  /* ── Footer flipper (persistent across views) ───────────────────────── */
  .flipper {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(8px * var(--card-scale, 1));
    border-top: 1px solid var(--rule-card-dossier);
    margin: 0 calc(-14px * var(--card-scale, 1)) calc(-12px * var(--card-scale, 1));
    padding: calc(8px * var(--card-scale, 1)) calc(14px * var(--card-scale, 1)) calc(10px * var(--card-scale, 1));
  }
  .flip-arrow {
    width: calc(18px * var(--card-scale, 1));
    height: calc(18px * var(--card-scale, 1));
    display: grid;
    place-items: center;
    color: var(--accent-card-dossier);
    cursor: pointer;
    font-size: calc(14px * var(--card-scale, 1));
    background: transparent;
    border: none;
    padding: 0;
    transition: color 120ms ease;
  }
  .flip-arrow:hover { color: var(--alert-card-dossier); }
  .flip-current {
    font-size: calc(9px * var(--card-scale, 1));
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--alert-card-dossier);
    font-weight: 600;
  }
  .flip-pager {
    font-size: calc(8px * var(--card-scale, 1));
    color: color-mix(in srgb, var(--text-card-dossier) 40%, transparent);
    font-family: ui-monospace, monospace;
    letter-spacing: 0.1em;
  }

  /* ── Reduced-motion fallback ────────────────────────────────────────── */
  @media (prefers-reduced-motion: reduce) {
    .step-btn,
    .chip,
    .btn-save,
    .flip-arrow {
      transition: none;
    }
  }

  /* ── Plan B: modifier integration ───────────────────────────────────── */
  .modifier-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: color-mix(in srgb, var(--alert-card-dossier) 10%, transparent);
    border: 1px solid var(--alert-card-dossier);
    color: var(--alert-card-dossier);
    padding: calc(0.25rem * var(--card-scale, 1)) calc(0.5rem * var(--card-scale, 1));
    font-size: calc(0.65rem * var(--card-scale, 1));
    letter-spacing: 0.12em;
    text-transform: uppercase;
    border-radius: calc(0.2rem * var(--card-scale, 1));
    margin-bottom: calc(0.4rem * var(--card-scale, 1));
  }
  .modifier-banner .banner-count {
    background: var(--alert-card-dossier);
    color: var(--bg-card-dossier);
    font-weight: 700;
    padding: 0 0.5em;
    border-radius: 999px;
  }

  .attr-cell.modified .attr-val,
  .skill-row.modified .skill-val {
    color: var(--alert-card-dossier);
    font-weight: 700;
  }
  /* Compact delta indicator: superscript-style, no background, no border.
     The full delta + source modifier names live in the row's title tooltip. */
  .delta-badge {
    color: var(--alert-card-dossier);
    font-size: 0.65em;
    font-weight: 600;
    margin-left: 0.25em;
    vertical-align: super;
    letter-spacing: 0.02em;
  }

  .chip[role="button"] { cursor: pointer; }
  .chip[data-busy="true"] { opacity: 0.6; pointer-events: none; }

  .editor-overlay {
    position: fixed; inset: 0;
    background: var(--shadow-strong);
    display: grid; place-items: center;
    z-index: 100;
  }
</style>
