<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { untrack } from 'svelte';
  import type { DyscrasiaEntry } from '../../types';

  const {
    entry = null,
    oncancel,
    onsave,
  }: {
    entry?: DyscrasiaEntry | null;
    oncancel: () => void;
    onsave: (saved: DyscrasiaEntry) => void;
  } = $props();

  const TYPES = ['phlegmatic', 'melancholy', 'choleric', 'sanguine'] as const;

  let resonanceType = $state(untrack(() => entry?.resonanceType?.toLowerCase() ?? 'phlegmatic'));
  let name         = $state(untrack(() => entry?.name ?? ''));
  let description  = $state(untrack(() => entry?.description ?? ''));
  let bonus        = $state(untrack(() => entry?.bonus ?? ''));
  let saving = $state(false);
  let error  = $state('');

  async function save() {
    if (!name.trim() || !description.trim() || !bonus.trim()) {
      error = 'All fields are required.';
      return;
    }
    saving = true;
    error = '';
    try {
      let saved: DyscrasiaEntry;
      if (entry) {
        await invoke<void>('update_dyscrasia', {
          id: entry.id,
          name: name.trim(),
          description: description.trim(),
          bonus: bonus.trim(),
        });
        saved = { ...entry, name: name.trim(), description: description.trim(), bonus: bonus.trim() };
      } else {
        saved = await invoke<DyscrasiaEntry>('add_dyscrasia', {
          resonanceType,
          name: name.trim(),
          description: description.trim(),
          bonus: bonus.trim(),
        });
      }
      onsave(saved);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form-card">
  <div class="form-title">{entry ? 'Edit Dyscrasia' : 'Add Custom Dyscrasia'}</div>

  <div class="field">
    <label for="rtype">Resonance Type</label>
    <select id="rtype" bind:value={resonanceType} disabled={!!entry}>
      {#each TYPES as t}
        <option value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="dname">Name</label>
    <input id="dname" type="text" bind:value={name} placeholder="e.g. Unshakeable Calm" />
  </div>

  <div class="field">
    <label for="ddesc">Description</label>
    <textarea id="ddesc" bind:value={description} rows={3} placeholder="Flavour text…"></textarea>
  </div>

  <div class="field">
    <label for="dbonus">Bonus</label>
    <input id="dbonus" type="text" bind:value={bonus} placeholder="e.g. +2 dice to Fortitude rolls" />
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <div class="form-actions">
    <button class="cancel-btn" onclick={oncancel} disabled={saving}>Cancel</button>
    <button class="save-btn"   onclick={save}     disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>

<style>
  .form-card {
    background: var(--bg-card);
    border: 1px solid var(--border-active);
    border-radius: 8px;
    padding: 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    break-inside: avoid;
    margin-bottom: 0.75rem;
  }
  .form-title {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
  }
  .field { display: flex; flex-direction: column; gap: 0.2rem; }
  label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }
  input, select, textarea {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.35rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
    font-family: inherit;
    outline: none;
    transition: border-color 0.15s;
  }
  input:focus, select:focus, textarea:focus { border-color: var(--accent); }
  textarea { resize: vertical; min-height: 4rem; }
  select:disabled { opacity: 0.6; cursor: not-allowed; }
  .error { font-size: 0.68rem; color: var(--accent-bright); }
  .form-actions { display: flex; gap: 0.4rem; justify-content: flex-end; }
  .save-btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent);
    border-radius: 4px;
    padding: 0.3rem 0.8rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: box-shadow 0.15s, transform 0.1s;
  }
  .save-btn:hover:not(:disabled) { box-shadow: 0 0 8px #cc222255; }
  .save-btn:active { transform: scale(0.93); }
  .save-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .cancel-btn {
    background: none;
    border: 1px solid var(--border-surface);
    color: var(--text-muted);
    border-radius: 4px;
    padding: 0.3rem 0.8rem;
    font-size: 0.72rem;
    cursor: pointer;
    transition: border-color 0.15s, transform 0.1s;
  }
  .cancel-btn:hover:not(:disabled) { border-color: var(--border-active); color: var(--text-label); }
  .cancel-btn:active { transform: scale(0.93); }
  .cancel-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
