<script lang="ts">
  import { goto } from '$app/navigation';
  import { addAdvantage, listAdvantages, type AdvantageInput } from '$lib/advantages/api';
  import AdvantageBody, { type AdvantageBodyState } from '$lib/components/builder/bodies/AdvantageBody.svelte';

  let bodyState = $state<AdvantageBodyState | undefined>(undefined);
  let valid = $state(false);
  let existingCount = $state(0);
  let busy = $state(false);

  $effect(() => {
    void (async () => {
      try { existingCount = (await listAdvantages()).length; } catch { existingCount = 0; }
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
    if (!bodyState || busy) return;
    busy = true;
    try {
      await addAdvantage(toAdvantageInput(bodyState, valid));
      await goto('/');
    } finally { busy = false; }
  }

  function cancel() { void goto('/'); }
</script>

<svelte:head><title>New Advantage · vtmtools</title></svelte:head>

<main class="builder-page">
  <header class="title-bar">
    <span class="step-indicator">NEW</span>
    <span class="subtitle">· new merit</span>
    <button type="button" class="close" aria-label="Close" onclick={cancel}>×</button>
  </header>
  <AdvantageBody bind:bodyState bind:valid {existingCount} />
  <footer class="composer-footer">
    <button type="button" onclick={cancel}>Cancel</button>
    <button type="button" class="save" onclick={save} disabled={busy}>
      {valid ? 'Save' : 'Save WIP'}
    </button>
  </footer>
</main>

<style>
.builder-page {
  width: min(36rem, 100% - 2rem);
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 1rem 0;
}
.title-bar {
  display: flex;
  align-items: center;
  padding: 0.45rem 0.75rem;
  background: rgba(0, 0, 0, 0.3);
  border-bottom: 1px solid var(--hairline);
}
.step-indicator {
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 0.56rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--accent-amber);
}
.subtitle {
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 0.56rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-left: 0.25rem;
}
.close {
  margin-left: auto;
  background: transparent;
  border: 0;
  color: var(--text-muted);
  font-size: 1.2rem;
  cursor: pointer;
  padding: 0 0.25rem;
}
.composer-footer {
  display: flex;
  gap: 0.4rem;
  align-items: center;
  justify-content: flex-end;
  padding: 0.5rem 0.75rem;
  background: rgba(0, 0, 0, 0.25);
}
.composer-footer .save { background: var(--accent-red); color: var(--text-primary); border: 1px solid var(--accent-red); padding: 0.3rem 0.7rem; cursor: pointer; }
.composer-footer button { font-family: 'Spartan', sans-serif; font-weight: 400; font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; }
.composer-footer button[disabled] { opacity: 0.6; cursor: not-allowed; }
</style>
