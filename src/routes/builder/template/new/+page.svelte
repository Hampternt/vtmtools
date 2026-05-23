<script lang="ts">
  import { goto } from '$app/navigation';
  import { addStatusTemplate } from '$lib/modifiers/api';
  import StatusTemplateBody, { type StatusTemplateBodyState } from '$lib/components/builder/bodies/StatusTemplateBody.svelte';
  import type { NewStatusTemplateInput } from '../../../../types';

  let bodyState = $state<StatusTemplateBodyState | undefined>(undefined);
  let valid = $state(false);
  let wipMessage = $state('');
  let busy = $state(false);

  function toStatusTemplateInput(s: StatusTemplateBodyState, isValid: boolean): NewStatusTemplateInput {
    return {
      name: s.name,
      description: s.description,
      effects: s.effects.map(e => ({ ...e, paths: [...e.paths] })),
      tags: [...s.tags],
      wip: !isValid,
      confirmedFields: s.confirmedFields,
    };
  }

  async function save() {
    if (!bodyState || busy) return;
    busy = true;
    try {
      await addStatusTemplate(toStatusTemplateInput(bodyState, valid));
      await goto('/');
    } finally { busy = false; }
  }

  function cancel() { void goto('/'); }
</script>

<svelte:head><title>New Template · vtmtools</title></svelte:head>

<main class="builder-page">
  <header class="title-bar">
    <span class="step-indicator">NEW</span>
    <span class="subtitle">· new template</span>
    <button type="button" class="close" aria-label="Close" onclick={cancel}>×</button>
  </header>
  <StatusTemplateBody bind:bodyState bind:valid bind:wipMessage />
  <footer class="composer-footer">
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
  padding: 0.5rem 0.75rem;
  background: rgba(0, 0, 0, 0.25);
}
.composer-footer .spacer { flex: 1; }
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
</style>
