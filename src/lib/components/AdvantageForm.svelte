<script lang="ts">
  import type { Advantage, AdvantageKind, Field, FieldValue } from '../../types';
  import { addAdvantage, updateAdvantage, type AdvantageInput } from '$lib/advantages/api';

  const KIND_OPTIONS: { value: AdvantageKind; label: string }[] = [
    { value: 'merit',      label: 'Merit'      },
    { value: 'flaw',       label: 'Flaw'       },
    { value: 'background', label: 'Background' },
    { value: 'boon',       label: 'Boon'       },
  ];
  import { FIELD_PRESETS, type FieldPreset } from '$lib/advantages/fieldPresets';
  import PropertyEditor from '$lib/components/properties/PropertyEditor.svelte';
  import { SUPPORTED_TYPES } from '$lib/components/properties/property-widgets';

  const { entry, oncancel, onsave }: {
    entry?: Advantage;
    oncancel?: () => void;
    onsave?: () => void;
  } = $props();

  let name        = $state(entry?.name ?? '');
  let description = $state(entry?.description ?? '');
  let kind: AdvantageKind     = $state(entry?.kind ?? 'merit');
  let tags: string[]          = $state(entry ? [...entry.tags]       : []);
  let properties: Field[]     = $state(entry ? [...entry.properties] : []);
  let tagDraft    = $state('');
  let saveError   = $state('');
  let saving      = $state(false);

  const trimmedTags         = $derived(tags.map(t => t.trim()).filter(t => t.length > 0));
  const tagsUnique          = $derived(new Set(trimmedTags).size === trimmedTags.length);
  const propertyNames       = $derived(properties.map(p => p.name.trim()));
  const propertiesUnique    = $derived(new Set(propertyNames).size === propertyNames.length);
  const propertiesNonEmpty  = $derived(propertyNames.every(n => n.length > 0));

  const valid = $derived(
    name.trim().length > 0 && tagsUnique && propertiesUnique && propertiesNonEmpty
  );

  function addTag() {
    const v = tagDraft.trim();
    if (!v) return;
    if (tags.includes(v)) { tagDraft = ''; return; }
    tags = [...tags, v];
    tagDraft = '';
  }

  function removeTag(i: number) {
    tags = tags.filter((_, idx) => idx !== i);
  }

  function onTagKey(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); addTag(); }
  }

  function applyPreset(preset: FieldPreset) {
    if (properties.some(p => p.name === preset.name)) return;
    const newField = buildPresetField(preset);
    properties = [...properties, newField];
  }

  function buildPresetField(preset: FieldPreset): Field {
    switch (preset.type) {
      case 'number':
        return { name: preset.name, type: 'number', value: Number(preset.defaultValue) };
      case 'bool':
        return { name: preset.name, type: 'bool',   value: Boolean(preset.defaultValue) };
      case 'text':
        return { name: preset.name, type: 'text',   value: String(preset.defaultValue) };
      case 'string':
      default:
        return { name: preset.name, type: 'string', value: String(preset.defaultValue) };
    }
  }

  function addCustomProperty() {
    properties = [...properties, { name: '', type: 'string', value: '' } as Field];
  }

  function renameProperty(i: number, newName: string) {
    properties = properties.map((p, idx) => idx === i ? { ...p, name: newName } : p);
  }

  function retypeProperty(i: number, newType: FieldValue['type']) {
    const prev = properties[i];
    const blank = buildPresetField({ name: prev.name, type: newType, defaultValue: '', hint: '' });
    properties = properties.map((p, idx) => idx === i ? blank : p);
  }

  function updateProperty(i: number, updated: Field) {
    properties = properties.map((p, idx) => idx === i ? updated : p);
  }

  function removeProperty(i: number) {
    properties = properties.filter((_, idx) => idx !== i);
  }

  async function handleSave() {
    if (!valid) return;
    saving = true;
    saveError = '';
    const input: AdvantageInput = {
      name: name.trim(),
      description,
      kind,
      tags: trimmedTags,
      properties: properties.map(p => ({ ...p, name: p.name.trim() })),
    };
    try {
      if (entry) {
        await updateAdvantage(entry.id, input);
      } else {
        await addAdvantage(input);
      }
      onsave?.();
    } catch (e) {
      saveError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<form class="form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
  <section class="section">
    <label class="label" for="adv-name">Name</label>
    <input id="adv-name" class="input" bind:value={name} />

    <label class="label" for="adv-desc">Description</label>
    <textarea id="adv-desc" class="textarea" bind:value={description}></textarea>

    <label class="label" for="adv-kind">Kind</label>
    <select id="adv-kind" class="input" bind:value={kind}>
      {#each KIND_OPTIONS as opt}
        <option value={opt.value}>{opt.label}</option>
      {/each}
    </select>
  </section>

  <section class="section">
    <div class="label">Tags</div>
    <div class="tag-row">
      {#each tags as t, i}
        <span class="tag">
          {t}
          <button type="button" class="tag-x" onclick={() => removeTag(i)} aria-label="Remove tag">×</button>
        </span>
      {/each}
      <input
        class="tag-input"
        bind:value={tagDraft}
        onkeydown={onTagKey}
        placeholder="Add tag (Enter)"
      />
    </div>
    {#if !tagsUnique}
      <p class="validation">Tags must be unique.</p>
    {/if}
  </section>

  <section class="section">
    <div class="label">Properties</div>
    <div class="preset-row">
      {#each FIELD_PRESETS as preset}
        {@const disabled = properties.some(p => p.name === preset.name)}
        <button
          type="button"
          class="preset"
          {disabled}
          title={preset.hint}
          onclick={() => applyPreset(preset)}
        >+ {preset.name}</button>
      {/each}
      <button type="button" class="preset" onclick={addCustomProperty}>+ Custom…</button>
    </div>

    <ul class="prop-list">
      {#each properties as prop, i (i)}
        <li class="prop-row">
          <input
            class="prop-name"
            value={prop.name}
            placeholder="field name"
            oninput={(e) => renameProperty(i, (e.target as HTMLInputElement).value)}
          />
          <select
            class="prop-type"
            value={prop.type}
            onchange={(e) => retypeProperty(i, (e.target as HTMLSelectElement).value as FieldValue['type'])}
          >
            {#each SUPPORTED_TYPES as t}
              <option value={t}>{t}</option>
            {/each}
          </select>
          <div class="prop-widget">
            <PropertyEditor
              field={prop}
              readonly={false}
              onchange={(u) => updateProperty(i, u)}
              onremove={() => removeProperty(i)}
            />
          </div>
        </li>
      {/each}
    </ul>

    {#if !propertiesUnique}
      <p class="validation">Property names must be unique.</p>
    {/if}
    {#if !propertiesNonEmpty}
      <p class="validation">Every property needs a name.</p>
    {/if}
  </section>

  {#if saveError}
    <p class="error">{saveError}</p>
  {/if}

  <div class="footer">
    <button type="button" class="btn" onclick={oncancel}>Cancel</button>
    <button type="submit"  class="btn primary" disabled={!valid || saving}>
      {saving ? 'Saving…' : (entry ? 'Save' : 'Add')}
    </button>
  </div>
</form>

<style>
  .form {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 6px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-sizing: border-box;
  }
  .section  { display: flex; flex-direction: column; gap: 0.35rem; }
  .label    { color: var(--text-label); font-size: 0.7rem; }
  .input, .textarea, .tag-input, .prop-name, .prop-type {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.32rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
    box-sizing: border-box;
  }
  .textarea { min-height: 4rem; resize: vertical; }
  .tag-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.35rem;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem;
    box-sizing: border-box;
  }
  .tag-row .tag-input {
    flex: 1;
    min-width: 6rem;
    border: none;
    padding: 0.2rem;
    background: transparent;
  }
  .tag {
    background: var(--bg-sunken);
    color: var(--text-secondary);
    border-radius: 10px;
    padding: 0.1rem 0.4rem 0.1rem 0.5rem;
    font-size: 0.66rem;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
  }
  .tag-x {
    background: none;
    border: none;
    color: var(--text-ghost);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0;
  }
  .tag-x:hover { color: var(--accent); }
  .preset-row { display: flex; flex-wrap: wrap; gap: 0.3rem; }
  .preset {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.2rem 0.55rem;
    font-size: 0.68rem;
    cursor: pointer;
  }
  .preset:hover:not([disabled]) { color: var(--accent); border-color: var(--accent); }
  .preset[disabled] { opacity: 0.45; cursor: not-allowed; }
  .prop-list { list-style: none; padding: 0; margin: 0.3rem 0 0 0; display: flex; flex-direction: column; gap: 0.35rem; }
  .prop-row {
    display: grid;
    grid-template-columns: 7rem 5.5rem 1fr;
    gap: 0.35rem;
    align-items: start;
  }
  .prop-widget { min-width: 0; }
  .validation { color: var(--accent); font-size: 0.68rem; margin: 0.2rem 0 0 0; }
  .error      { color: var(--accent); font-size: 0.72rem; margin: 0; }
  .footer     { display: flex; justify-content: flex-end; gap: 0.4rem; }
  .btn {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.7rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .btn.primary { background: var(--bg-active); border-color: var(--border-active); color: var(--accent); }
  .btn[disabled] { opacity: 0.45; cursor: not-allowed; }
</style>
