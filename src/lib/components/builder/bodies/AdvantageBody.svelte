<script lang="ts">
  import { createStageMachine } from '$lib/builder/stage-machine.svelte';
  import { pickPlaceholder, commitPlaceholder } from '$lib/builder/latin-placeholders';
  import { deriveBonusName } from '$lib/builder/bonus-name';
  import CardComposer from '../CardComposer.svelte';
  import CardSlot from '../CardSlot.svelte';
  import ValueSelector from '../ValueSelector.svelte';
  import AppliesToPicker from '../AppliesToPicker.svelte';
  import TagChipInput from '../TagChipInput.svelte';
  import AdvantageCard from '../../AdvantageCard.svelte';
  import { V5_SKILL_CATEGORIES } from '$lib/foundry/canonical-names';
  import type { Advantage, AdvantageKind, ModifierEffect } from '../../../../types';

  export interface AdvantageBodyBonus {
    kind: 'pool' | 'difficulty' | 'auto_success';
    bonusName: string;
    value: number;
    paths: string[];
  }

  export interface AdvantageBodyState {
    name: string;
    kind: AdvantageKind;
    level: number;
    tags: string[];
    description: string;
    bonus: AdvantageBodyBonus;
    // Effects beyond the first slot — preserved on edit so the save handler
    // can re-emit them. AdvantageBody only models a single bonus, so anything
    // imported from Foundry / authored elsewhere with multiple effects would
    // otherwise be silently dropped on round-trip.
    extraEffects: ModifierEffect[];
    confirmedFields: string[];
  }

  interface Props {
    initial?: Advantage;
    existingCount?: number;
    valid?: boolean;
    bodyState?: AdvantageBodyState;
  }

  function emptyState(init: Advantage | undefined, count: number): AdvantageBodyState {
    // For a new advantage, leave name empty so the Latin placeholder displays
    // as HTML ghost text (spec §"Latin placeholder commit semantics"). The
    // displayed placeholder comes from the `placeholder` top-level const
    // computed from `existingCount`. Editing an existing advantage uses its
    // stored name as before. `count` is intentionally unused here but kept in
    // the signature to avoid churning callers.
    void count;
    const initialName = init?.name ?? '';
    const firstEffect = init?.effects?.[0];
    const restEffects = init?.effects?.slice(1) ?? [];

    return {
      name: initialName,
      kind: init?.kind ?? 'merit',
      level: Number((init?.properties.find(p => p.name === 'dot_cost_per_level' && p.type === 'number')?.value) ?? 0),
      tags: init ? [...init.tags] : [],
      description: init?.description ?? '',
      bonus: firstEffect
        ? {
            // Only Pool/Difficulty/AutoSuccess are bonus-stage kinds.
            // Foreign kinds (note, stat) fall through to 'pool' so the user
            // can correct manually — better than silent data loss.
            kind: ['pool', 'difficulty', 'auto_success'].includes(firstEffect.kind)
              ? (firstEffect.kind as AdvantageBodyBonus['kind'])
              : 'pool',
            bonusName: firstEffect.bonusName ?? deriveBonusName(initialName),
            value: firstEffect.delta ?? 1,
            paths: [...firstEffect.paths],
          }
        : {
            kind: 'pool',
            bonusName: deriveBonusName(initialName),
            value: 1,
            paths: [],
          },
      extraEffects: restEffects.map(e => ({ ...e })),
      confirmedFields: [],
    };
  }

  let {
    initial,
    existingCount = 0,
    valid = $bindable(false),
    bodyState = $bindable<AdvantageBodyState>(),
  }: Props = $props();

  // Initialize bodyState if not provided by parent
  if (!bodyState) {
    bodyState = emptyState(initial, existingCount);
  }

  const machine = createStageMachine({
    tabOrder: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
    required: ['name'],
    initialConfirmed: initial?.confirmedFields ?? undefined,
  });

  // Latin placeholder driving the name-input ghost text. Stable for the
  // builder session; rotation only advances when the library grows.
  const placeholder = pickPlaceholder(existingCount);

  function onNameKeydown(e: KeyboardEvent) {
    if (!bodyState) return;
    if (e.key === 'Tab' || e.key === 'Enter') {
      if (bodyState.name.trim() === '') {
        bodyState.name = commitPlaceholder(bodyState.name, placeholder);
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        machine.advance();
      }
      // Tab: let default focus shift happen
    } else if (e.key === 'Escape') {
      bodyState.name = '';
      e.preventDefault();
    }
  }

  let bonusSubStage = $state<'type' | 'name' | 'value' | 'applies'>('type');
  // Sticky-name semantics: when editing an existing advantage whose stored
  // bonusName diverges from the derived default, treat the loaded name as
  // user-authored so the name-syncing `$effect` below doesn't immediately
  // overwrite it. Without this guard, opening "Iron Gullet" + a custom
  // "Brawl Buff" bonus name re-derives to "Iron Gullet bonus" on first
  // reactive tick.
  let bonusNameSticky = $state(
    initial?.effects?.[0]?.bonusName != null &&
    initial.effects[0].bonusName !== deriveBonusName(initial.name)
  );

  $effect(() => {
    if (bodyState && !bonusNameSticky) {
      bodyState.bonus.bonusName = deriveBonusName(bodyState.name);
    }
  });

  function onBonusNameInput(v: string) {
    bonusNameSticky = true;
    if (bodyState) bodyState.bonus.bonusName = v;
  }

  function bonusKindLabel(k: AdvantageBodyBonus['kind']): string {
    return { pool: 'dice', difficulty: 'difficulty', auto_success: 'auto successes' }[k] ?? k;
  }

  function appliesToLabel(paths: string[]): string {
    if (paths.length === 0) return 'all';
    if (paths.length === 1) {
      const last = paths[0].split('.').pop() ?? paths[0];
      return last.charAt(0).toUpperCase() + last.slice(1).replace(/_/g, ' ');
    }
    if (paths.length === 9) {
      const all = paths.every(p => p.startsWith('skills.'));
      if (all) {
        const skills = paths.map(p => p.substring(7));
        const set = new Set(skills);
        if (V5_SKILL_CATEGORIES.physical.every(s => set.has(s))) return 'Physical';
        if (V5_SKILL_CATEGORIES.social.every(s => set.has(s))) return 'Social';
        if (V5_SKILL_CATEGORIES.mental.every(s => set.has(s))) return 'Mental';
      }
    }
    return `${paths.length} paths`;
  }

  $effect(() => {
    valid = machine.isComplete();
  });

  $effect(() => {
    if (bodyState) bodyState.confirmedFields = Array.from(machine.confirmed);
  });

  function slotStateFor(field: string): 'empty' | 'active' | 'confirmed' {
    if (machine.stage === field) return 'active';
    if (machine.confirmed.has(field)) return 'confirmed';
    return 'empty';
  }

  function kindLabel(k: AdvantageKind): string {
    const labels: Record<string, string> = { merit: 'Merit', flaw: 'Flaw', background: 'Background', boon: 'Boon' };
    return labels[k] ?? k;
  }

  function queueRemaining(): string[] {
    return ['bonus', 'description', 'tags']
      .filter(s => s !== machine.stage && !machine.confirmed.has(s));
  }

  function toAdvantageInput(s: AdvantageBodyState) {
    // Returns a partial Advantage shape suitable for the AdvantageCard preview
    // branch. AdvantageCard's PreviewInput type is
    // `Omit<Advantage, 'id' | 'isCustom' | 'sourceAttribution'>`.
    return {
      name: s.name,
      description: s.description,
      kind: s.kind,
      tags: s.tags,
      properties: [
        { name: 'dot_cost_per_level', type: 'number' as const, value: s.level },
      ],
      effects: [{
        kind: s.bonus.kind,
        scope: null,
        delta: s.bonus.value,
        note: null,
        paths: s.bonus.paths,
        bonusName: s.bonus.bonusName,
      }],
      wip: !machine.isComplete(),
      confirmedFields: machine.isComplete() ? undefined : Array.from(machine.confirmed),
    };
  }

  function onPreviewClick(target: HTMLElement) {
    const slot = target.closest('[data-stage]') as HTMLElement | null;
    if (slot && slot.dataset.stage) {
      machine.setStage(slot.dataset.stage);
    }
  }
</script>

{#snippet cardRows()}
  {#if bodyState}
    <div class="card-row two-col">
      <CardSlot field="name" state={slotStateFor('name')} keyLabel="Name" onClick={() => machine.setStage('name')}>
        {bodyState.name}
      </CardSlot>
      <CardSlot field="type" state={slotStateFor('type')} keyLabel="Type" onClick={() => machine.setStage('type')}>
        {kindLabel(bodyState.kind)}
      </CardSlot>
    </div>
    <div class="card-row two-col">
      <CardSlot field="level" state={slotStateFor('level')} keyLabel="Level" onClick={() => machine.setStage('level')}>
        <span class="tier-dots">
          {#each [1,2,3,4,5] as n}
            <span class="tier-pip" class:f={bodyState.level >= n}></span>
          {/each}
        </span>
      </CardSlot>
      <CardSlot field="category" state="empty" keyLabel="Category">n/a (merits)</CardSlot>
    </div>
    <div class="card-row one-col">
      <CardSlot field="desc" state={slotStateFor('description')} keyLabel="Description" onClick={() => machine.setStage('description')}>
        {bodyState.description || '—'}
      </CardSlot>
    </div>
    <div class="card-row one-col">
      <CardSlot field="bonus" state={slotStateFor('bonus')} keyLabel="Bonus" onClick={() => { machine.setStage('bonus'); bonusSubStage = 'type'; }}>
        {#if machine.confirmed.has('bonus')}
          <span class="bonus-line">
            <strong>+{bodyState.bonus.value}</strong> {bonusKindLabel(bodyState.bonus.kind)} when making
            <em>{appliesToLabel(bodyState.bonus.paths)}</em> rolls
          </span>
        {:else}
          —
        {/if}
      </CardSlot>
    </div>
    <div class="card-row one-col">
      <CardSlot field="tags" state={slotStateFor('tags')} keyLabel="Tags" onClick={() => machine.setStage('tags')}>
        {#if bodyState.tags.length > 0}
          {#each bodyState.tags as t}<span class="mini-tag">{t}</span>{/each}
        {:else}—{/if}
      </CardSlot>
    </div>
  {/if}
{/snippet}

{#snippet spotlight()}
  {#if bodyState && machine.stage === 'name'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Name this merit</h3>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="big-input"
      type="text"
      bind:value={bodyState.name}
      placeholder={placeholder}
      onkeydown={onNameKeydown}
      aria-label="Advantage name"
      autofocus
    />
    <p class="hint">Tab or Enter accepts placeholder · Shift+Tab goes back</p>
    <button class="next-btn" onclick={() => machine.advance()}>Next →</button>
  {:else if bodyState && machine.stage === 'type'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">What kind?</h3>
    <div class="kind-row">
      {#each ['merit', 'flaw', 'background', 'boon'] as k}
        <button
          class:selected={bodyState.kind === k}
          onclick={() => { if (bodyState) bodyState.kind = k as AdvantageKind; machine.advance(); }}
        >{kindLabel(k as AdvantageKind)}</button>
      {/each}
    </div>
  {:else if bodyState && machine.stage === 'level'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Tier level</h3>
    <div class="tier-row">
      {#each [1,2,3,4,5] as n}
        <button
          class="tier-pip-button"
          class:f={bodyState.level >= n}
          onclick={() => { if (bodyState) bodyState.level = n; machine.advance(); }}
        ></button>
      {/each}
    </div>
  {:else if bodyState && machine.stage === 'bonus'}
    {#if bonusSubStage === 'type'}
      <div class="prompt-label">— SPOTLIGHT —</div>
      <h3 class="prompt">What kind of bonus?</h3>
      <div class="bonus-type-grid">
        <button onclick={() => { if (bodyState) bodyState.bonus.kind = 'pool'; bonusSubStage = 'name'; }}>⚂ Dice Pool</button>
        <button onclick={() => { if (bodyState) bodyState.bonus.kind = 'difficulty'; bonusSubStage = 'name'; }}>↧ Difficulty</button>
        <button onclick={() => { if (bodyState) bodyState.bonus.kind = 'auto_success'; bonusSubStage = 'name'; }}>✓ Auto Successes</button>
      </div>
    {:else if bonusSubStage === 'name'}
      <div class="prompt-label">— SPOTLIGHT —</div>
      <h3 class="prompt">Bonus name</h3>
      <!-- svelte-ignore a11y_autofocus -->
      <input class="big-input" type="text" value={bodyState.bonus.bonusName} oninput={(e) => onBonusNameInput((e.currentTarget as HTMLInputElement).value)} onkeydown={(e) => { if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); bonusSubStage = 'value'; } }} autofocus />
      <p class="hint">Default: "{deriveBonusName(bodyState.name)}" · Tab accepts</p>
      <button class="next-btn" onclick={() => bonusSubStage = 'value'}>Next →</button>
    {:else if bonusSubStage === 'value'}
      <div class="prompt-label">— SPOTLIGHT —</div>
      <h3 class="prompt">How many {bodyState.bonus.kind === 'difficulty' ? 'difficulty steps' : bodyState.bonus.kind === 'auto_success' ? 'auto successes' : 'dice'}?</h3>
      <ValueSelector value={bodyState.bonus.value} onChange={(n) => { if (bodyState) bodyState.bonus.value = n; }} />
      <button class="next-btn" onclick={() => bonusSubStage = 'applies'}>Next: applies to →</button>
    {:else if bonusSubStage === 'applies'}
      <div class="prompt-label">— SPOTLIGHT —</div>
      <h3 class="prompt">Applies to which rolls?</h3>
      <AppliesToPicker paths={bodyState.bonus.paths} onChange={(p) => { if (bodyState) bodyState.bonus.paths = p; }} />
      <button class="next-btn" onclick={() => machine.advance()}>Confirm bonus →</button>
    {/if}
  {:else if bodyState && machine.stage === 'description'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Describe {bodyState.name}</h3>
    <!-- svelte-ignore a11y_autofocus -->
    <textarea
      class="big-textarea"
      bind:value={bodyState.description}
      placeholder="Optional. A line or two of flavor."
      onkeydown={(e) => { if (e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); machine.advance(); } }}
      autofocus
    ></textarea>
    <p class="hint">Tab to confirm (empty is fine)</p>
    <button class="next-btn" onclick={() => machine.advance()}>Next →</button>
  {:else if bodyState && machine.stage === 'tags'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Tag this merit</h3>
    <TagChipInput tags={bodyState.tags} onChange={(t) => { if (bodyState) bodyState.tags = t; }} autofocus />
    <p class="hint">Tab confirms · empty is fine · type Enter to commit</p>
    <div style="display: flex; justify-content: center; margin-top: 0.6rem;">
      <button class="next-btn" onclick={() => machine.advance()}>Done →</button>
    </div>
  {:else if bodyState && machine.stage === 'review'}
    <div class="prompt-label">— REVIEW —</div>
    <h3 class="prompt review-prompt">{bodyState.name} is ready.</h3>
    <p class="hint">Click any slot above to re-edit · Ctrl+Enter to save</p>
  {/if}
{/snippet}

{#snippet preview()}
  {#if bodyState}
    <AdvantageCard input={{ kind: 'preview', data: toAdvantageInput(bodyState) }} onClick={onPreviewClick} />
  {/if}
{/snippet}

<CardComposer
  {cardRows}
  {spotlight}
  {preview}
  isComplete={machine.isComplete()}
  queueValues={queueRemaining()}
/>

<style>
  .card-row { display: grid; gap: 0.45rem; }
  .card-row.two-col { grid-template-columns: 1fr 1fr; }
  .tier-dots { display: inline-flex; gap: 0.18rem; }
  .tier-pip {
    width: 0.55rem; height: 0.55rem; border-radius: 50%;
    border: 1px solid var(--border-surface);
  }
  .tier-pip.f { background: var(--field-level); border-color: var(--field-level); box-shadow: 0 0 3px rgba(192, 136, 80, 0.5); }
  .prompt-label { font-size: 0.6rem; color: var(--accent-amber); letter-spacing: 0.1em; text-transform: uppercase; text-align: center; }
  .prompt { margin: 0; font-family: "Iowan Old Style", "Garamond", serif; font-size: 1.1rem; color: var(--text-primary); text-align: center; font-weight: 600; }
  .big-input {
    background: var(--bg-input);
    border: 1px solid var(--accent-amber);
    border-radius: 0.3rem;
    padding: 0.55rem 0.75rem;
    font-size: 1.05rem;
    color: var(--text-primary);
    font-family: "Iowan Old Style", "Garamond", serif;
    box-sizing: border-box;
    width: 100%;
    outline: none;
  }
  .hint { font-size: 0.65rem; color: var(--text-muted); text-align: center; font-style: italic; margin: 0; }
  .kind-row, .tier-row { display: flex; gap: 0.45rem; justify-content: center; }
  .kind-row button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.3rem;
    padding: 0.45rem 0.85rem; font-size: 0.85rem; cursor: pointer;
    font-family: inherit;
  }
  .kind-row button.selected { background: var(--field-type); color: #fff; border-color: var(--field-type); }
  .tier-pip-button {
    width: 1.8rem; height: 1.8rem; border-radius: 50%;
    border: 1px solid var(--border-surface);
    background: transparent; cursor: pointer; padding: 0;
    transition: transform 0.12s;
  }
  .tier-pip-button:hover { transform: scale(1.1); border-color: var(--accent); }
  .tier-pip-button.f { background: var(--field-level); border-color: var(--field-level); box-shadow: 0 0 4px rgba(192, 136, 80, 0.6); }
  .bonus-type-grid { display: flex; flex-direction: column; gap: 0.5rem; max-width: 18rem; margin: 0 auto; }
  .bonus-type-grid button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.3rem;
    padding: 0.6rem 0.85rem; font-size: 0.85rem; cursor: pointer;
    font-family: inherit; text-align: left;
  }
  .bonus-type-grid button:hover { border-color: var(--accent); color: var(--accent-bright); }
  .next-btn {
    background: var(--accent); color: #fff; border: 1px solid var(--accent-bright);
    border-radius: 0.3rem; padding: 0.35rem 0.85rem; font-size: 0.78rem;
    cursor: pointer; font-family: inherit; align-self: center;
  }
  .bonus-line strong { color: var(--field-bonus); font-weight: 700; }
  .bonus-line em { color: var(--accent-blue); font-style: normal; font-weight: 600; }
  .card-row.one-col { grid-template-columns: 1fr; }
  .big-textarea {
    background: var(--bg-input);
    border: 1px solid var(--accent-amber);
    border-radius: 0.3rem;
    padding: 0.55rem 0.75rem;
    font-size: 0.85rem;
    color: var(--text-primary);
    font-family: inherit;
    box-sizing: border-box;
    width: 100%;
    min-height: 5rem;
    resize: vertical;
    outline: none;
  }
  .mini-tag {
    display: inline-block;
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-secondary);
    border-radius: 999px;
    padding: 0.05rem 0.4rem;
    font-size: 0.62rem;
    font-family: inherit;
    margin-right: 0.2rem;
    font-style: italic;
  }
  .review-prompt {
    color: var(--accent-bright);
  }
</style>
