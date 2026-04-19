<script lang="ts">
  import * as api from '../../domains/api';
  import { session, cache, setChronicle, refreshChronicles } from '../../../store/domains.svelte';

  let creating = $state(false);
  let renaming = $state(false);
  let draftName = $state('');
  let draftDesc = $state('');
  let localError = $state('');

  const current = $derived(cache.chronicles.find(c => c.id === session.chronicleId) ?? null);

  function startCreate() {
    creating = true;
    renaming = false;
    draftName = '';
    draftDesc = '';
    localError = '';
  }

  function startRename() {
    if (!current) return;
    renaming = true;
    creating = false;
    draftName = current.name;
    draftDesc = current.description;
    localError = '';
  }

  function cancel() {
    creating = false;
    renaming = false;
    localError = '';
  }

  async function saveCreate() {
    const name = draftName.trim();
    if (!name) { localError = 'Name is required.'; return; }
    try {
      const c = await api.createChronicle(name, draftDesc.trim());
      await refreshChronicles();
      await setChronicle(c.id);
      creating = false;
    } catch (e) {
      localError = String(e);
    }
  }

  async function saveRename() {
    if (!current) return;
    const name = draftName.trim();
    if (!name) { localError = 'Name is required.'; return; }
    try {
      await api.updateChronicle(current.id, name, draftDesc.trim());
      await refreshChronicles();
      renaming = false;
    } catch (e) {
      localError = String(e);
    }
  }

  async function deleteCurrent() {
    if (!current) return;
    if (!confirm(`Delete chronicle "${current.name}"? All nodes and edges will be removed. This cannot be undone.`)) return;
    try {
      await api.deleteChronicle(current.id);
      await refreshChronicles();
      const next = cache.chronicles[0]?.id ?? null;
      await setChronicle(next);
    } catch (e) {
      localError = String(e);
    }
  }

  async function onSelect(e: Event) {
    const id = Number((e.target as HTMLSelectElement).value);
    await setChronicle(id);
  }
</script>

<header class="header">
  <div class="title">🏰 Domains Manager</div>

  <div class="spacer"></div>

  {#if !creating && !renaming}
    <label class="field">
      <span class="label">Chronicle</span>
      <select value={session.chronicleId ?? ''} onchange={onSelect} disabled={cache.chronicles.length === 0}>
        {#if cache.chronicles.length === 0}
          <option value="">(none)</option>
        {/if}
        {#each cache.chronicles as c (c.id)}
          <option value={c.id}>{c.name}</option>
        {/each}
      </select>
    </label>

    <button class="btn" onclick={startCreate}>+ New</button>
    {#if current}
      <button class="btn" onclick={startRename}>✎ Rename</button>
      <button class="btn danger" onclick={deleteCurrent}>✕ Delete</button>
    {/if}
    <button class="btn" disabled title="Coming soon">⇩ Kumu</button>
  {:else}
    <div class="inline-form">
      <input class="input" placeholder="Chronicle name" bind:value={draftName} />
      <input class="input" placeholder="Description (optional)" bind:value={draftDesc} />
      <button class="btn primary" onclick={creating ? saveCreate : saveRename}>Save</button>
      <button class="btn" onclick={cancel}>Cancel</button>
    </div>
  {/if}
</header>

{#if localError}
  <div class="error">{localError}</div>
{/if}

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border-surface);
  }
  .title { font-weight: 600; color: var(--text-primary); font-size: 0.92rem; }
  .spacer { flex: 1; }
  .field { display: inline-flex; align-items: center; gap: 0.4rem; }
  .label {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  select, .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
    font-family: inherit;
    outline: none;
  }
  select:focus, .input:focus { border-color: var(--accent); }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.65rem;
    font-size: 0.74rem;
    cursor: pointer;
    transition: box-shadow 0.15s, transform 0.1s;
  }
  .btn:hover:not(:disabled) { color: var(--text-primary); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn.primary { color: var(--accent); }
  .btn.primary:hover { box-shadow: 0 0 8px #cc222255; }
  .btn.danger { color: var(--accent); }
  .btn.danger:hover { box-shadow: 0 0 8px #cc222255; }
  .inline-form { display: flex; gap: 0.4rem; align-items: center; }
  .error {
    color: var(--accent-bright);
    font-size: 0.72rem;
    padding: 0.3rem 0.75rem;
    background: var(--bg-sunken);
    border-bottom: 1px solid var(--border-surface);
  }
</style>
