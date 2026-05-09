<script lang="ts">
  import { statusTemplates } from '../../../store/statusTemplates.svelte';
  import type { ModifierEffect, ModifierKind, StatusTemplate } from '../../../types';

  interface Props {
    /** Existing template to edit, or null to author a new one. */
    existing: StatusTemplate | null;
    onClose: () => void;
  }

  let { existing, onClose }: Props = $props();

  let name = $state(existing?.name ?? '');
  let description = $state(existing?.description ?? '');
  let effects = $state<ModifierEffect[]>(
    (existing?.effects ?? []).map(e => ({ ...e }))
  );
  let tags = $state<string[]>([...(existing?.tags ?? [])]);
  let newTag = $state('');
  let saving = $state(false);
  let error = $state<string | null>(null);

  const KINDS: { value: ModifierKind; label: string }[] = [
    { value: 'pool',       label: 'Pool' },
    { value: 'difficulty', label: 'Difficulty' },
    { value: 'note',       label: 'Note' },
  ];

  function addEffect() {
    effects = [...effects, { kind: 'pool', scope: null, delta: 0, note: null, paths: [] }];
  }
  function removeEffect(i: number) { effects = effects.filter((_, idx) => idx !== i); }
  function bumpDelta(i: number, by: number) {
    const cur = effects[i].delta ?? 0;
    effects[i] = { ...effects[i], delta: Math.max(-10, Math.min(10, cur + by)) };
  }
  function setKind(i: number, kind: ModifierKind) {
    effects[i] = kind === 'note'
      ? { ...effects[i], kind, delta: null }
      : { ...effects[i], kind, note: null };
  }
  function commitTag() {
    const t = newTag.trim();
    if (!t || tags.includes(t)) { newTag = ''; return; }
    tags = [...tags, t];
    newTag = '';
  }
  function removeTag(t: string) { tags = tags.filter(x => x !== t); }

  async function save() {
    if (!name.trim()) { error = 'Name required'; return; }
    saving = true;
    error = null;
    try {
      if (existing) {
        await statusTemplates.update(existing.id, { name, description, effects, tags });
      } else {
        await statusTemplates.add({ name, description, effects, tags });
      }
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function del() {
    if (!existing) return;
    if (!confirm(`Delete template "${existing.name}"?`)) return;
    saving = true;
    try {
      await statusTemplates.delete(existing.id);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<aside class="editor" role="dialog" aria-label="Edit status template">
  <header>
    <h3>{existing ? 'Edit template' : 'New template'}</h3>
    <button class="close" onclick={onClose} aria-label="Close">×</button>
  </header>

  <label>
    <span>Name</span>
    <input
      type="text"
      value={name}
      oninput={(e) => name = (e.currentTarget as HTMLInputElement).value}
    />
  </label>
  <label>
    <span>Description</span>
    <textarea
      rows="2"
      value={description}
      oninput={(e) => description = (e.currentTarget as HTMLTextAreaElement).value}
    ></textarea>
  </label>

  <fieldset>
    <legend>Effects</legend>
    {#each effects as effect, i (i)}
      <div class="effect-row">
        <select value={effect.kind} onchange={(e) => setKind(i, (e.currentTarget as HTMLSelectElement).value as ModifierKind)}>
          {#each KINDS as k}<option value={k.value}>{k.label}</option>{/each}
        </select>
        {#if effect.kind === 'note'}
          <input
            type="text"
            placeholder="Note text"
            value={effect.note ?? ''}
            oninput={(e) => effects[i] = { ...effects[i], note: (e.currentTarget as HTMLInputElement).value }}
          />
        {:else}
          <input
            type="text"
            placeholder="Scope (e.g. Social)"
            value={effect.scope ?? ''}
            oninput={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              effects[i] = { ...effects[i], scope: v === '' ? null : v };
            }}
          />
          <div class="stepper">
            <button onclick={() => bumpDelta(i, -1)} aria-label="Decrement">−</button>
            <span>{effect.delta ?? 0}</span>
            <button onclick={() => bumpDelta(i, 1)} aria-label="Increment">+</button>
          </div>
        {/if}
        <button class="remove" onclick={() => removeEffect(i)} aria-label="Remove effect">×</button>
      </div>
    {/each}
    <button class="add" onclick={addEffect}>+ Add effect</button>
  </fieldset>

  <fieldset>
    <legend>Tags</legend>
    <div class="tag-list">
      {#each tags as t}
        <span class="tag-chip">
          {t}
          <button onclick={() => removeTag(t)} aria-label="Remove tag {t}">×</button>
        </span>
      {/each}
      <input
        type="text"
        placeholder="+ tag"
        value={newTag}
        oninput={(e) => newTag = (e.currentTarget as HTMLInputElement).value}
        onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); commitTag(); } }}
        onblur={commitTag}
      />
    </div>
  </fieldset>

  {#if error}<p class="error">{error}</p>{/if}

  <footer>
    {#if existing}<button class="danger" onclick={del} disabled={saving}>Delete</button>{/if}
    <span class="spacer"></span>
    <button class="secondary" onclick={onClose}>Cancel</button>
    <button class="primary" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </footer>
</aside>

<style>
  .editor {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.5rem;
    padding: 1rem;
    width: 24rem;
    box-shadow: 0 0.75rem 2rem -0.25rem rgba(0,0,0,0.6);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-sizing: border-box;
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  header h3 { margin: 0; font-size: 0.95rem; color: var(--text-primary); }
  .close, .remove, .add, button.secondary, button.primary, button.danger, .stepper button {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  button.primary { background: var(--accent); color: var(--text-primary); border-color: var(--accent-bright); }
  button.danger { background: transparent; color: var(--accent-amber); border-color: var(--accent-amber); }
  label { display: flex; flex-direction: column; gap: 0.25rem; }
  label span { font-size: 0.75rem; color: var(--text-label); }
  label input, label textarea {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.3rem 0.5rem;
    font-size: 0.85rem;
    box-sizing: border-box;
    width: 100%;
  }
  fieldset { border: 1px solid var(--border-faint); border-radius: 0.4rem; padding: 0.5rem; margin: 0; }
  legend { font-size: 0.75rem; color: var(--text-label); padding: 0 0.3rem; }
  .effect-row {
    display: grid;
    grid-template-columns: 6rem 1fr auto auto;
    gap: 0.4rem;
    align-items: center;
    margin-bottom: 0.35rem;
  }
  .effect-row select, .effect-row input {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.4rem;
    font-size: 0.75rem;
    box-sizing: border-box;
    width: 100%;
  }
  .stepper { display: inline-flex; gap: 0.25rem; align-items: center; color: var(--text-primary); font-variant-numeric: tabular-nums; }
  .tag-list { display: flex; flex-wrap: wrap; gap: 0.3rem; align-items: center; }
  .tag-chip {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 999px;
    padding: 0.15rem 0.5rem;
    font-size: 0.7rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .tag-chip button { background: transparent; border: none; color: var(--text-muted); cursor: pointer; padding: 0; }
  .tag-list input { width: 7rem; }
  .error { color: var(--accent-amber); font-size: 0.75rem; margin: 0; }
  footer { display: flex; gap: 0.4rem; align-items: center; }
  .spacer { flex: 1; }
</style>
