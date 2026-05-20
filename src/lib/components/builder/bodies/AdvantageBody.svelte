<script lang="ts">
  import { createStageMachine } from '$lib/builder/stage-machine';
  import { pickPlaceholder } from '$lib/builder/latin-placeholders';
  import CardComposer from '../CardComposer.svelte';
  import CardSlot from '../CardSlot.svelte';
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
    state?: AdvantageBodyState;
  }

  function emptyState(init: Advantage | undefined, count: number): AdvantageBodyState {
    const initialName = init?.name ?? pickPlaceholder(count);
    return {
      name: initialName,
      kind: init?.kind ?? 'merit',
      level: Number((init?.properties.find(p => p.name === 'dot_cost_per_level' && p.type === 'number')?.value) ?? 1),
      tags: init ? [...init.tags] : [],
      description: init?.description ?? '',
      bonus: { kind: 'pool', bonusName: '', value: 1, paths: [] },
      confirmedFields: [],
    };
  }

  let {
    initial,
    existingCount = 0,
    valid = $bindable(false),
    state = $bindable<AdvantageBodyState>(),
  }: Props = $props();

  // Initialize state if not provided by parent
  if (!state) {
    state = emptyState(initial, existingCount);
  }

  const machine = createStageMachine({
    tabOrder: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
    required: ['name', 'bonus'],
    initialConfirmed: initial?.confirmedFields ?? undefined,
  });

  $effect(() => {
    valid = machine.isComplete();
  });

  $effect(() => {
    if (state) state.confirmedFields = Array.from(machine.confirmed);
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
  {#if state}
    <div class="card-row two-col">
      <CardSlot field="name" state={slotStateFor('name')} keyLabel="Name" onClick={() => machine.setStage('name')}>
        {state.name}
      </CardSlot>
      <CardSlot field="type" state={slotStateFor('type')} keyLabel="Type" onClick={() => machine.setStage('type')}>
        {kindLabel(state.kind)}
      </CardSlot>
    </div>
    <div class="card-row two-col">
      <CardSlot field="level" state={slotStateFor('level')} keyLabel="Level" onClick={() => machine.setStage('level')}>
        <span class="tier-dots">
          {#each [1,2,3,4,5] as n}
            <span class="tier-pip" class:f={state.level >= n}></span>
          {/each}
        </span>
      </CardSlot>
      <CardSlot field="category" state="empty" keyLabel="Category">n/a (merits)</CardSlot>
    </div>
  {/if}
{/snippet}

{#snippet spotlight()}
  {#if state && machine.stage === 'name'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Name this merit</h3>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="big-input"
      type="text"
      bind:value={state.name}
      onkeydown={(e) => { if (e.key === 'Enter' || e.key === 'Tab') { e.preventDefault(); machine.advance(); } }}
      autofocus
    />
    <p class="hint">Tab or Enter accepts placeholder · Shift+Tab goes back</p>
  {:else if state && machine.stage === 'type'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">What kind?</h3>
    <div class="kind-row">
      {#each ['merit', 'flaw', 'background', 'boon'] as k}
        <button
          class:selected={state.kind === k}
          onclick={() => { if (state) state.kind = k as AdvantageKind; machine.advance(); }}
        >{kindLabel(k as AdvantageKind)}</button>
      {/each}
    </div>
  {:else if state && machine.stage === 'level'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Tier level</h3>
    <div class="tier-row">
      {#each [1,2,3,4,5] as n}
        <button
          class="tier-pip-button"
          class:f={state.level >= n}
          onclick={() => { if (state) state.level = n; machine.advance(); }}
        ></button>
      {/each}
    </div>
  {:else}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <p class="hint">Stage "{machine.stage}" — UI lands in Tasks 16, 17.</p>
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
</style>
