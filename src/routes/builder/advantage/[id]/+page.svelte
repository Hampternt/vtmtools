<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { listAdvantages, updateAdvantage, deleteAdvantage, type AdvantageInput } from '$lib/advantages/api';
  import AdvantageBody, { type AdvantageBodyState } from '$lib/components/builder/bodies/AdvantageBody.svelte';
  import type { Advantage } from '../../../../types';

  const id = $derived(Number(page.params.id));
  const stageParam = $derived(page.url.searchParams.get('stage') ?? null);

  const initialStage = $derived(
    (['name', 'type', 'level', 'bonus', 'description', 'tags'] as const).includes(stageParam as 'name')
      ? (stageParam as 'name' | 'type' | 'level' | 'bonus' | 'description' | 'tags')
      : undefined
  );

  let initial = $state<Advantage | undefined>(undefined);
  let bodyState = $state<AdvantageBodyState | undefined>(undefined);
  let valid = $state(false);
  let wipMessage = $state('');
  let busy = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const all = await listAdvantages();
        initial = all.find(a => a.id === id);
        if (!initial) loadError = `Advantage ${id} not found`;
      } catch (e) { loadError = String(e); }
    })();
  });

  function toAdvantageInput(s: AdvantageBodyState, isValid: boolean): AdvantageInput {
    return {
      name: s.name,
      kind: s.kind,
      description: s.description,
      tags: [...s.tags],
      properties: [{ name: 'dot_cost_per_level', type: 'number', value: s.level }],
      effects: [
        { kind: s.bonus.kind, bonusName: s.bonus.bonusName, delta: s.bonus.value, paths: [...s.bonus.paths] },
        ...s.extraEffects,
      ],
      wip: !isValid,
      confirmedFields: s.confirmedFields,
    } as AdvantageInput;
  }

  async function save() {
    if (!bodyState || !initial || busy) return;
    busy = true;
    try {
      await updateAdvantage(id, toAdvantageInput(bodyState, valid));
      await goto('/');
    } finally { busy = false; }
  }

  async function onDelete() {
    if (!initial || busy) return;
    if (!confirm(`Delete ${initial.name}?`)) return;
    busy = true;
    try {
      await deleteAdvantage(id);
      await goto('/');
    } finally { busy = false; }
  }

  function cancel() { void goto('/'); }
</script>

<svelte:head><title>Edit Advantage · vtmtools</title></svelte:head>

{#if loadError}
  <main class="builder-page error">
    <p>{loadError}</p>
    <button type="button" onclick={cancel}>Back</button>
  </main>
{:else if initial}
  <main class="builder-page">
    <header class="title-bar">
      <span class="step-indicator">EDIT</span>
      <span class="subtitle">· {initial.name}</span>
      <button type="button" class="close" aria-label="Close" onclick={cancel}>×</button>
    </header>
    <AdvantageBody {initial} bind:bodyState bind:valid bind:wipMessage {initialStage} />
    <footer class="composer-footer">
      <button type="button" class="delete" onclick={onDelete}>Delete</button>
      {#if wipMessage}
        <span class="wip-strip">● {wipMessage}</span>
      {/if}
      <div class="spacer"></div>
      <button type="button" onclick={cancel}>Cancel</button>
      <button type="button" class="save" onclick={save} disabled={busy}>
        {valid ? 'Save' : 'Save WIP'}
      </button>
    </footer>
  </main>
{:else}
  <main class="builder-page loading"><p>Loading…</p></main>
{/if}

<style>
/* Same styles as new/+page.svelte; copy them verbatim — Svelte scopes <style> per file */
.builder-page { width: min(36rem, 100% - 2rem); margin: 0 auto; display: flex; flex-direction: column; gap: 0.4rem; padding: 1rem 0; }
.title-bar { display: flex; align-items: center; padding: 0.45rem 0.75rem; background: rgba(0, 0, 0, 0.3); border-bottom: 1px solid var(--hairline); }
.step-indicator { font-family: 'Spartan', sans-serif; font-weight: 200; font-size: 0.56rem; letter-spacing: 0.1em; text-transform: uppercase; color: var(--accent-amber); }
.subtitle { font-family: 'Spartan', sans-serif; font-weight: 200; font-size: 0.56rem; letter-spacing: 0.08em; text-transform: uppercase; color: var(--text-muted); margin-left: 0.25rem; }
.close { margin-left: auto; background: transparent; border: 0; color: var(--text-muted); font-size: 1.2rem; cursor: pointer; padding: 0 0.25rem; }
.composer-footer { display: flex; gap: 0.4rem; align-items: center; padding: 0.5rem 0.75rem; background: rgba(0, 0, 0, 0.25); }
.composer-footer .spacer { flex: 1; }
.composer-footer .delete { color: var(--text-muted); background: transparent; border: 1px solid var(--text-muted); padding: 0.3rem 0.7rem; cursor: pointer; }
.composer-footer .delete:hover { color: var(--accent-red); border-color: var(--accent-red); }
.composer-footer .save { background: var(--accent-red); color: var(--text-primary); border: 1px solid var(--accent-red); padding: 0.3rem 0.7rem; cursor: pointer; }
.composer-footer button { font-family: 'Spartan', sans-serif; font-weight: 400; font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; }
.composer-footer button[disabled] { opacity: 0.6; cursor: not-allowed; }
.wip-strip {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.2rem 0.6rem;
  background: rgba(201, 160, 100, 0.15);
  border: 1px solid var(--wip);
  border-radius: 99px;
  color: var(--wip);
  font-family: 'Oswald', sans-serif;
  font-weight: 200;
  font-size: 0.65rem;
  letter-spacing: 0.04em;
}
.wip-strip::first-letter { font-size: 0.85em; }
.builder-page.error p { color: var(--validation-error); padding: 1rem; }
.builder-page.loading p { color: var(--text-muted); padding: 1rem; text-align: center; }
</style>
