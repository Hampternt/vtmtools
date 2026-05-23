<script lang="ts">
  import { createStageMachine } from '$lib/builder/stage-machine.svelte';
  import CardComposer from '../CardComposer.svelte';
  import CardSlot from '../CardSlot.svelte';
  import TagChipInput from '../TagChipInput.svelte';
  import ValueSelector from '../ValueSelector.svelte';
  import AppliesToPicker from '../AppliesToPicker.svelte';
  import type { ModifierEffect, ModifierKind, BridgeCharacter } from '../../../../types';

  export interface ModifierEffectBodyState {
    effects: ModifierEffect[];
    tags: string[];
    confirmedFields: string[];
  }

  interface Props {
    initialEffects: ModifierEffect[];
    initialTags: string[];
    character?: BridgeCharacter;
    valid?: boolean;
    bodyState?: ModifierEffectBodyState;
    wipMessage?: string;
  }

  let {
    initialEffects, initialTags, character,
    valid = $bindable(false),
    bodyState = $bindable<ModifierEffectBodyState>(),
    wipMessage = $bindable(''),
  }: Props = $props();

  if (!bodyState) {
    bodyState = {
      effects: initialEffects.map(e => ({ ...e })),
      tags: [...initialTags],
      confirmedFields: [],
    };
  }

  const machine = createStageMachine({
    tabOrder: ['effects', 'tags'],
    required: ['effects'],
  });

  $effect(() => { valid = machine.isComplete(); });

  $effect(() => {
    if (bodyState) bodyState.confirmedFields = Array.from(machine.confirmed);
  });

  $effect(() => {
    if (!bodyState) { wipMessage = ''; return; }
    if (!machine.confirmed.has('effects') || bodyState.effects.length === 0) {
      wipMessage = 'no effects yet · saveable as draft';
    } else {
      wipMessage = '';
    }
  });

  function slotStateFor(field: string): 'empty' | 'active' | 'confirmed' {
    if (machine.stage === field) return 'active';
    if (machine.confirmed.has(field)) return 'confirmed';
    return 'empty';
  }

  function addEffect() {
    if (!bodyState) return;
    bodyState.effects = [...bodyState.effects, {
      kind: 'pool' as ModifierKind,
      scope: null,
      delta: 1,
      note: null,
      paths: [],
      bonusName: null,
    }];
  }

  function removeEffect(i: number) {
    if (!bodyState) return;
    bodyState.effects = bodyState.effects.filter((_, idx) => idx !== i);
  }
</script>

{#snippet cardRows()}
  {#if bodyState}
    <div class="card-row one-col">
      <CardSlot field="bonus" state={slotStateFor('effects')} keyLabel="Effects" onClick={() => machine.setStage('effects')}>
        {bodyState.effects.length} effect{bodyState.effects.length === 1 ? '' : 's'}
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
  {#if bodyState && machine.stage === 'effects'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Edit effects</h3>
    {#each bodyState.effects as effect, i}
      <div class="effect-row">
        <select bind:value={effect.kind}>
          <option value="pool">Pool</option>
          <option value="difficulty">Difficulty</option>
          <option value="auto_success">Auto Success</option>
          <option value="note">Note</option>
          <option value="stat">Stat</option>
        </select>
        {#if effect.kind === 'note'}
          <input type="text" placeholder="Note text" value={effect.note ?? ''}
            oninput={(e) => effect.note = (e.currentTarget as HTMLInputElement).value || null} />
        {:else}
          <input type="text" placeholder="Scope (optional)" value={effect.scope ?? ''}
            oninput={(e) => { const v = (e.currentTarget as HTMLInputElement).value; effect.scope = v === '' ? null : v; }} />
          <ValueSelector value={effect.delta ?? 0} onChange={(n) => effect.delta = n} min={-10} max={10} />
        {/if}
        <button class="x-btn" onclick={() => removeEffect(i)} aria-label="Remove effect">×</button>
      </div>
      {#if effect.kind !== 'note'}
        <div style="padding-left: 1rem;">
          <AppliesToPicker paths={effect.paths} onChange={(p) => effect.paths = p} />
        </div>
      {/if}
    {/each}
    <button class="add-effect-btn" onclick={addEffect}>+ Add effect</button>
    <div style="display: flex; justify-content: center; margin-top: 0.6rem;">
      <button class="next-btn" onclick={() => machine.advance()}>Done →</button>
    </div>
  {:else if bodyState && machine.stage === 'tags'}
    <div class="prompt-label">— SPOTLIGHT —</div>
    <h3 class="prompt">Tags</h3>
    <TagChipInput tags={bodyState.tags} onChange={(t) => { if (bodyState) bodyState.tags = t; }} autofocus />
    <div style="display: flex; justify-content: center; margin-top: 0.6rem;">
      <button class="next-btn" onclick={() => machine.advance()}>Done →</button>
    </div>
  {:else if bodyState && machine.stage === 'review'}
    <div class="prompt-label">— REVIEW —</div>
    <h3 class="prompt review-prompt">Effects ready.</h3>
    <p class="hint">Click any slot above to re-edit · Ctrl+Enter to save</p>
  {/if}
{/snippet}

<CardComposer {cardRows} {spotlight} isComplete={machine.isComplete()} />

<style>
  .card-row { display: grid; gap: 0.45rem; }
  .card-row.one-col { grid-template-columns: 1fr; }
  .prompt-label { font-size: 0.6rem; color: var(--accent-amber); letter-spacing: 0.1em; text-transform: uppercase; text-align: center; }
  .prompt { margin: 0; font-family: "Iowan Old Style", "Garamond", serif; font-size: 1.1rem; color: var(--text-primary); text-align: center; font-weight: 600; }
  .review-prompt { color: var(--accent-bright); }
  .hint { font-size: 0.65rem; color: var(--text-muted); text-align: center; font-style: italic; margin: 0; }
  .effect-row {
    display: grid; grid-template-columns: 7rem 1fr auto auto;
    gap: 0.4rem; align-items: center;
    padding: 0.35rem 0;
    border-bottom: 1px solid var(--border-faint);
  }
  .effect-row select, .effect-row input {
    background: var(--bg-input); border: 1px solid var(--border-faint);
    color: var(--text-primary); border-radius: 0.2rem;
    padding: 0.25rem 0.4rem; font-size: 0.75rem; font-family: inherit;
    box-sizing: border-box; width: 100%;
  }
  .x-btn { background: transparent; border: none; color: var(--text-muted); cursor: pointer; padding: 0 0.3rem; font-size: 1rem; }
  .x-btn:hover { color: var(--accent); }
  .add-effect-btn {
    background: transparent; border: 1px dashed var(--border-surface);
    color: var(--text-label); border-radius: 0.25rem;
    padding: 0.35rem; font-size: 0.7rem; cursor: pointer;
    margin-top: 0.5rem; width: 100%; font-family: inherit;
  }
  .add-effect-btn:hover { color: var(--accent-bright); border-color: var(--accent); }
  .next-btn {
    background: var(--accent); color: #fff; border: 1px solid var(--accent-bright);
    border-radius: 0.3rem; padding: 0.35rem 0.85rem; font-size: 0.78rem;
    cursor: pointer; font-family: inherit;
  }
  .mini-tag {
    display: inline-block; background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 999px;
    padding: 0.05rem 0.4rem; font-size: 0.62rem;
    margin-right: 0.2rem; font-style: italic;
  }
</style>
