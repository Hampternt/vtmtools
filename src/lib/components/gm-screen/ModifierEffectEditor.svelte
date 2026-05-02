<script lang="ts">
  import type { ModifierEffect, ModifierKind } from '../../../types';

  interface Props {
    initialEffects: ModifierEffect[];
    initialTags: string[];
    onSave: (effects: ModifierEffect[], tags: string[]) => Promise<void>;
    onCancel: () => void;
  }

  let { initialEffects, initialTags, onSave, onCancel }: Props = $props();

  let effects = $state<ModifierEffect[]>(initialEffects.map(e => ({ ...e })));
  let tags = $state<string[]>([...initialTags]);
  let newTag = $state('');
  let saving = $state(false);
  let error = $state<string | null>(null);

  const KINDS: { value: ModifierKind; label: string }[] = [
    { value: 'pool',       label: 'Pool' },
    { value: 'difficulty', label: 'Difficulty' },
    { value: 'note',       label: 'Note' },
  ];

  function addEffect() {
    effects = [...effects, { kind: 'pool', scope: null, delta: 0, note: null }];
  }

  function removeEffect(i: number) {
    effects = effects.filter((_, idx) => idx !== i);
  }

  function bumpDelta(i: number, by: number) {
    const cur = effects[i].delta ?? 0;
    const next = Math.max(-10, Math.min(10, cur + by));
    effects[i] = { ...effects[i], delta: next };
  }

  function setKind(i: number, kind: ModifierKind) {
    // When switching to/from 'note', clear the now-irrelevant fields per spec
    // §4 (delta=None for note kind, note=None for pool/difficulty kinds).
    if (kind === 'note') {
      effects[i] = { ...effects[i], kind, delta: null };
    } else {
      effects[i] = { ...effects[i], kind, note: null };
    }
  }

  function commitTag() {
    const t = newTag.trim();
    if (!t || tags.includes(t)) { newTag = ''; return; }
    tags = [...tags, t];
    newTag = '';
  }

  function removeTag(t: string) {
    tags = tags.filter(x => x !== t);
  }

  async function handleSave() {
    saving = true;
    error = null;
    try {
      await onSave(effects, tags);
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="popover" role="dialog" aria-label="Edit modifier effects">
  <header>
    <h3>Effects</h3>
    <button class="close" onclick={onCancel} aria-label="Cancel">×</button>
  </header>

  <div class="effects-list">
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
            class="scope"
            value={effect.scope ?? ''}
            oninput={(e) => {
              const v = (e.currentTarget as HTMLInputElement).value;
              effects[i] = { ...effects[i], scope: v === '' ? null : v };
            }}
          />
          <div class="stepper">
            <button onclick={() => bumpDelta(i, -1)} aria-label="Decrement">−</button>
            <span class="delta">{effect.delta ?? 0}</span>
            <button onclick={() => bumpDelta(i, 1)} aria-label="Increment">+</button>
          </div>
        {/if}

        <button class="remove" onclick={() => removeEffect(i)} aria-label="Remove effect">×</button>
      </div>
    {/each}
    <button class="add" onclick={addEffect}>+ Add effect</button>
  </div>

  <div class="tags-section">
    <h4>Tags</h4>
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
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  <footer>
    <button class="secondary" onclick={onCancel}>Cancel</button>
    <button class="primary" onclick={handleSave} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </footer>
</div>

<style>
  .popover {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.5rem;
    padding: 0.85rem;
    width: 22rem;
    box-shadow: 0 0.75rem 2rem -0.25rem rgba(0,0,0,0.6);
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-sizing: border-box;            /* ARCH §6 */
  }
  header { display: flex; align-items: center; justify-content: space-between; }
  header h3 { margin: 0; font-size: 0.9rem; color: var(--text-primary); }
  .close, .remove, .add, button.secondary, button.primary, .stepper button {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .close { padding: 0.1rem 0.4rem; }
  .effects-list { display: flex; flex-direction: column; gap: 0.4rem; }
  .effect-row {
    display: grid;
    grid-template-columns: 6rem 1fr auto auto;
    gap: 0.4rem;
    align-items: center;
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
  .stepper { display: inline-flex; gap: 0.25rem; align-items: center; }
  .stepper .delta {
    min-width: 1.6rem;
    text-align: center;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
  }
  .tags-section h4 { margin: 0 0 0.4rem 0; font-size: 0.75rem; color: var(--text-label); font-weight: 500; }
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
  .tag-chip button {
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.7rem;
    padding: 0;
  }
  .tag-list input { width: 6rem; }
  .error { color: var(--accent-amber); font-size: 0.75rem; margin: 0; }
  footer { display: flex; justify-content: flex-end; gap: 0.4rem; }
  button.primary { background: var(--accent); color: var(--text-primary); border-color: var(--accent-bright); }
</style>
