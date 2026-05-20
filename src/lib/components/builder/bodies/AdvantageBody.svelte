<script lang="ts">
  import { createStageMachine } from '$lib/builder/stage-machine';
  import { pickPlaceholder } from '$lib/builder/latin-placeholders';
  import { deriveBonusName } from '$lib/builder/bonus-name';
  import CardComposer from '../CardComposer.svelte';
  import CardSlot from '../CardSlot.svelte';
  import ValueSelector from '../ValueSelector.svelte';
  import AppliesToPicker from '../AppliesToPicker.svelte';
  import type { Advantage, AdvantageKind } from '../../../../types';

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
    confirmedFields: string[];
  }

  interface Props {
    initial?: Advantage;
    existingCount?: number;
    valid?: boolean;
    bodyState?: AdvantageBodyState;
  }

  function emptyState(init: Advantage | undefined, count: number): AdvantageBodyState {
    const initialName = init?.name ?? pickPlaceholder(count);
    return {
      name: initialName,
      kind: init?.kind ?? 'merit',
      level: Number((init?.properties.find(p => p.name === 'dot_cost_per_level' && p.type === 'number')?.value) ?? 1),
      tags: init ? [...init.tags] : [],
      description: init?.description ?? '',
      bonus: {
        kind: 'pool',
        bonusName: deriveBonusName(initialName),
        value: 1,
        paths: [],
      },
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
    required: ['name', 'bonus'],
    initialConfirmed: initial?.confirmedFields ?? undefined,
  });

  let bonusSubStage = $state<'type' | 'name' | 'value' | 'applies'>('type');
  let bonusNameSticky = $state(false);

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
        const PHYSICAL = ['athletics', 'brawl', 'craft', 'drive', 'firearms', 'larceny', 'melee', 'stealth', 'survival'];
        const SOCIAL = ['animal_ken', 'etiquette', 'insight', 'intimidation', 'leadership', 'performance', 'persuasion', 'streetwise', 'subterfuge'];
        const MENTAL = ['academics', 'awareness', 'finance', 'investigation', 'medicine', 'occult', 'politics', 'science', 'technology'];
        const set = new Set(skills);
        if (PHYSICAL.every(s => set.has(s))) return 'Physical';
        if (SOCIAL.every(s => set.has(s))) return 'Social';
        if (MENTAL.every(s => set.has(s))) return 'Mental';
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
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); machine.advance(); } }}
      autofocus
    />
    <p class="hint">Tab or Enter accepts placeholder · Shift+Tab goes back</p>
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
  {:else}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <p class="hint">Stage "{machine.stage}" — UI lands in Task 17.</p>
  {/if}
{/snippet}

<CardComposer
  {cardRows}
  {spotlight}
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
</style>
