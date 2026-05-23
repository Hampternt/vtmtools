<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { listAllCharacterModifiers, updateCharacterModifier, deleteCharacterModifier } from '$lib/modifiers/api';
  import ModifierEffectBody, { type ModifierEffectBodyState } from '$lib/components/builder/bodies/ModifierEffectBody.svelte';
  import type { CharacterModifier, ModifierPatchInput } from '../../../../types';

  const id = $derived(Number(page.params.id));
  let initial = $state<CharacterModifier | undefined>(undefined);
  let bodyState = $state<ModifierEffectBodyState | undefined>(undefined);
  let valid = $state(false);
  let busy = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const all = await listAllCharacterModifiers();
        initial = all.find(m => m.id === id);
        if (!initial) loadError = `Modifier ${id} not found`;
      } catch (e) { loadError = String(e); }
    })();
  });

  function toModifierPatch(s: ModifierEffectBodyState): ModifierPatchInput {
    return {
      effects: s.effects.map(e => ({ ...e, paths: [...e.paths] })),
      tags: [...s.tags],
    };
  }

  async function save() {
    if (!bodyState || !initial || busy) return;
    busy = true;
    try {
      await updateCharacterModifier(id, toModifierPatch(bodyState));
      await goto('/');
    } finally { busy = false; }
  }

  async function onDelete() {
    if (!initial || busy) return;
    if (!confirm('Delete this modifier?')) return;
    busy = true;
    try {
      await deleteCharacterModifier(id);
      await goto('/');
    } finally { busy = false; }
  }

  function cancel() { void goto('/'); }
</script>

<svelte:head><title>Edit Modifier · vtmtools</title></svelte:head>

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
    <ModifierEffectBody
      initialEffects={initial.effects}
      initialTags={initial.tags}
      bind:bodyState
      bind:valid
    />
    <footer class="composer-footer">
      <button type="button" class="delete" onclick={onDelete}>Delete</button>
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
.builder-page.error p { color: var(--validation-error); padding: 1rem; }
.builder-page.loading p { color: var(--text-muted); padding: 1rem; text-align: center; }
</style>
