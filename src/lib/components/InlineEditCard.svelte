<script lang="ts">
  import AdvantageCard from './AdvantageCard.svelte';
  import TagChipInput from './builder/TagChipInput.svelte';
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import { updateAdvantage, type AdvantageInput } from '$lib/advantages/api';
  import type { Advantage, AdvantageKind } from '../../types';

  interface Props {
    advantage: Advantage;
    editing?: boolean;
    onSaved?: (saved: Advantage) => void;
    onCancelled?: () => void;
    onDelete?: (id: number) => void;
    onPush?: () => void;
  }

  let { advantage, editing = $bindable(false), onSaved, onCancelled, onDelete, onPush }: Props = $props();

  // Local edit-mode draft state — only populated when entering edit mode.
  let draftName = $state('');
  let draftDescription = $state('');
  let draftTags = $state<string[]>([]);
  let draftKind = $state<AdvantageKind>('merit');
  let draftLevel = $state(0);
  let validationWarn = $state<string | null>(null);
  let busy = $state(false);

  function levelFromAdvantage(a: Advantage): number {
    // Reads dot_cost_per_level first (matches new writer convention),
    // falls back to 'level' (seed / library_push / Foundry-pull). Mirrors
    // AdvantageCard's reader, so display and edit start from the same value.
    const f = a.properties.find(p => (p.name === 'dot_cost_per_level' || p.name === 'level') && p.type === 'number');
    if (!f) return 0;
    const v = Array.isArray(f.value) ? f.value[0] ?? 0 : f.value;
    return Number(v) || 0;
  }

  function enterEdit() {
    draftName = advantage.name;
    draftDescription = advantage.description ?? '';
    draftTags = [...advantage.tags];
    draftKind = advantage.kind;
    draftLevel = levelFromAdvantage(advantage);
    validationWarn = null;
    editing = true;
  }

  function buildInput(asWip: boolean): AdvantageInput {
    return {
      name: draftName,
      kind: draftKind,
      description: draftDescription,
      tags: [...draftTags],
      properties: [
        ...advantage.properties.filter(p => p.name !== 'dot_cost_per_level' && p.name !== 'level'),
        { name: 'dot_cost_per_level', type: 'number', value: draftLevel },
      ],
      effects: [...advantage.effects.map(e => ({ ...e, paths: [...e.paths] }))],
      wip: asWip,
      confirmedFields: advantage.confirmedFields,
    };
  }

  async function save() {
    if (busy) return;
    busy = true;
    try {
      const wip = draftName.trim() === '';
      const input = buildInput(wip);
      await updateAdvantage(advantage.id, input);
      onSaved?.({
        ...advantage,
        name: draftName,
        description: draftDescription,
        tags: [...draftTags],
        kind: draftKind,
        properties: input.properties,
        wip: input.wip ?? false,
        effects: input.effects,
      });
      editing = false;
    } finally {
      busy = false;
    }
  }

  function cancel() {
    editing = false;
    onCancelled?.();
  }

  async function navigateToBonus() {
    // Spec §"Hand-off to Form #2" — Gap 1 resolution (c):
    // auto-save WIP, then route to Form #2's bonus stage.
    if (busy) return;
    busy = true;
    try {
      const wip = draftName.trim() === '';
      await updateAdvantage(advantage.id, buildInput(wip));
      editing = false;
      await goto(`/builder/advantage/${advantage.id}?stage=bonus`);
    } finally {
      busy = false;
    }
  }

  function onNameBlur() {
    if (draftName.trim() === '') {
      validationWarn = "⚠ name is empty — won't save as complete";
    } else if (validationWarn?.includes('name')) {
      validationWarn = null;
    }
  }

  function onLevelKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
      draftLevel = Math.max(0, draftLevel - 1);
      e.preventDefault();
    } else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
      draftLevel = Math.min(5, draftLevel + 1);
      e.preventDefault();
    } else if (e.key === 'Home') {
      draftLevel = 0;
      e.preventDefault();
    } else if (e.key === 'End') {
      draftLevel = 5;
      e.preventDefault();
    } else if (/^[0-5]$/.test(e.key)) {
      draftLevel = Number(e.key);
      e.preventDefault();
    }
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (!editing) return;
    if (e.key === 'Escape') {
      cancel();
      e.preventDefault();
    } else if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
      void save();
      e.preventDefault();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onWindowKeydown);
    return () => window.removeEventListener('keydown', onWindowKeydown);
  });
</script>

{#if !editing}
  <AdvantageCard
    input={{ kind: 'saved', data: advantage }}
    onedit={enterEdit}
    ondelete={() => onDelete?.(advantage.id)}
    {onPush}
  />
{:else}
  <article class="vamp-card editing">
    <div class="frame">
      <div class="hint-strip">
        click any field · <span class="kbd">Tab</span> walks · <span class="kbd">Esc</span> cancels · <span class="kbd">Ctrl+S</span> saves
      </div>
      <header class="zone-title">
        <input
          type="text"
          bind:value={draftName}
          onblur={onNameBlur}
          class="name-input"
          aria-label="Advantage name"
        />
      </header>
      <section class="zone-content">
        <textarea
          bind:value={draftDescription}
          class="description-input"
          aria-label="Description"
          rows="3"
        ></textarea>
        <TagChipInput tags={draftTags} onChange={(t) => (draftTags = t)} />
        {#if validationWarn}
          <p class="validation-warn" role="status">{validationWarn}</p>
        {/if}
        <div class="hairline" aria-hidden="true"></div>
      </section>
      <footer class="zone-footer">
        <div class="detail" data-field="type">
          <h6>TYPE</h6>
          <select bind:value={draftKind} class="kind-select" aria-label="Kind">
            <option value="merit">Merit</option>
            <option value="flaw">Flaw</option>
            <option value="background">Background</option>
            <option value="boon">Boon</option>
          </select>
        </div>
        <div class="detail" data-field="level">
          <h6>LEVEL</h6>
          <span
            class="value tier-dots"
            role="slider"
            aria-label="Level"
            aria-valuemin="0"
            aria-valuemax="5"
            aria-valuenow={draftLevel}
            tabindex="0"
            onkeydown={onLevelKeydown}
          >
            {#each Array(5) as _, i}
              <button
                type="button"
                tabindex="-1"
                class="dot"
                class:filled={i < draftLevel}
                aria-label={`Set to ${i + 1}`}
                onclick={() => (draftLevel = i + 1)}
              ></button>
            {/each}
          </span>
        </div>
        <div class="detail" data-field="bonus">
          <h6>BONUS</h6>
          <button type="button" class="bonus-edit" onclick={navigateToBonus} disabled={busy}>
            edit →
          </button>
        </div>
      </footer>
    </div>
  </article>
  <div class="edit-footer">
    <span class="kbd-hint">
      <span class="kbd">Enter</span> saves · <span class="kbd">Esc</span> cancels
    </span>
    <div class="actions">
      <button type="button" onclick={cancel} disabled={busy}>Cancel</button>
      <button type="button" class="save" onclick={save} disabled={busy}>Save</button>
    </div>
  </div>
{/if}

<style>
  /* Edit-mode wraps the Witcher frame in dashed amber per spec, and
     copies the relevant frame structure since Svelte scopes <style>
     per file (AdvantageCard's styles don't leak into this file). */
  .vamp-card.editing {
    position: relative;
    width: 100%;
    max-width: 22rem;
    min-width: 14rem;
    font-family: 'Iowan Old Style', Garamond, serif;
    color: var(--text-primary);
    box-shadow: var(--shadow-medium);
  }
  .vamp-card.editing .frame {
    border: 1px dashed var(--accent-amber);
    position: relative;
    background: var(--bg-card, #1d1c1d);
  }

  .hint-strip {
    font-size: 0.55rem;
    letter-spacing: 0.08em;
    color: var(--text-muted);
    text-align: center;
    padding: 0.3rem 0.5rem 0;
    font-family: 'Spartan', sans-serif;
    text-transform: uppercase;
  }

  .zone-title { padding: 0.85rem 1rem 0.5rem; text-align: center; }
  .zone-content { padding: 0 1rem 0.55rem; }
  .zone-content .hairline {
    height: 1px;
    margin: 0.45rem 1rem;
    background: linear-gradient(to right, transparent, var(--hairline), transparent);
  }

  .name-input {
    width: 100%;
    background: transparent;
    border: 1px solid var(--outline-color);
    font-family: 'Spartan', sans-serif;
    font-weight: 200;
    font-size: 1.05rem;
    letter-spacing: 1.6px;
    text-transform: uppercase;
    text-align: center;
    color: inherit;
    padding: 0.2rem 0.5rem;
  }
  .name-input:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }
  .description-input {
    width: 100%;
    min-height: 3.5rem;
    background: transparent;
    border: 1px solid var(--outline-color);
    font-family: 'Oswald', sans-serif;
    font-weight: 200;
    font-size: 0.82rem;
    letter-spacing: 0.4px;
    color: inherit;
    resize: vertical;
  }
  .description-input:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }
  .validation-warn {
    color: var(--validation-warn);
    font-size: 0.7rem;
    margin: 0.25rem 0 0;
  }

  .zone-footer {
    display: flex;
    gap: 0.35rem;
    margin: 0 0.55rem 0.6rem;
    padding: 0 0.4rem;
  }
  .detail {
    flex: 1;
    background: var(--bg-detail);
    border: 1px solid var(--border-detail);
    padding: 0.45rem 0.4rem 0.5rem;
    text-align: center;
  }
  .detail h6 {
    margin: 0 0 0.25rem;
    font-family: 'Spartan', sans-serif;
    font-weight: 200;
    font-size: 0.58rem;
    letter-spacing: 1.1px;
    text-transform: uppercase;
    color: var(--text-muted);
  }

  .kind-select {
    width: 100%;
    background: transparent;
    border: 1px solid var(--outline-color);
    color: inherit;
    font-family: 'Oswald', sans-serif;
    font-size: 0.8rem;
  }
  .kind-select:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }

  .bonus-edit {
    background: transparent;
    border: 1px solid var(--outline-color);
    color: var(--accent-amber);
    font-family: 'Oswald', sans-serif;
    font-size: 0.78rem;
    cursor: pointer;
    padding: 0.15rem 0.4rem;
  }
  .bonus-edit:hover { background: rgba(217, 165, 96, 0.08); }
  .bonus-edit:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }

  .tier-dots {
    display: inline-flex;
    gap: 0.18rem;
    align-items: center;
    justify-content: center;
  }
  .tier-dots .dot {
    background: transparent;
    border: 1px solid var(--accent-amber);
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    padding: 0;
    cursor: pointer;
  }
  .tier-dots .dot.filled {
    background: var(--accent-amber);
    box-shadow: 0 0 3px rgba(217, 165, 96, 0.55);
  }
  .tier-dots:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }

  .edit-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.4rem 0.5rem;
    background: rgba(0, 0, 0, 0.2);
    margin-top: 0.3rem;
  }
  .kbd-hint {
    font-size: 0.6rem;
    color: var(--text-muted);
    font-family: 'Oswald', sans-serif;
  }
  .kbd {
    display: inline-block;
    background: rgba(0, 0, 0, 0.4);
    border: 1px solid var(--outline-color);
    border-radius: 2px;
    padding: 0 0.25rem;
    font-family: ui-monospace, monospace;
    font-size: 0.65rem;
  }
  .actions { display: flex; gap: 0.3rem; }
  .actions button {
    font-family: 'Spartan', sans-serif;
    font-weight: 400;
    font-size: 0.7rem;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    padding: 0.3rem 0.7rem;
    cursor: pointer;
  }
  .actions button:focus-visible {
    outline: 1px solid var(--accent-amber);
    outline-offset: 2px;
  }
  .actions .save {
    background: var(--accent-red);
    color: var(--text-primary);
    border: 1px solid var(--accent-red);
  }
</style>
