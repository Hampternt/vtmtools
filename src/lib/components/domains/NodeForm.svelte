<script lang="ts">
  import { untrack } from 'svelte';
  import type { ChronicleNode, Field, FieldValue } from '../../../types';
  import * as api from '../../domains/api';
  import { session, cache, refreshNodes, refreshEdges, selectNode } from '../../../store/domains.svelte';
  import PropertyEditor from './PropertyEditor.svelte';
  import { SUPPORTED_TYPES } from './property-widgets';

  const { node = null, parentId = null, oncancel, onsave }: {
    node?: ChronicleNode | null;
    parentId?: number | null;
    oncancel: () => void;
    onsave: (saved: ChronicleNode) => void;
  } = $props();

  let label       = $state(untrack(() => node?.label ?? ''));
  let nodeType    = $state(untrack(() => node?.type ?? 'area'));
  let description = $state(untrack(() => node?.description ?? ''));
  let tagsText    = $state(untrack(() => (node?.tags ?? []).join(', ')));
  let properties  = $state<Field[]>(untrack(() => structuredClone(node?.properties ?? [])));

  let newPropName = $state('');
  let newPropType = $state<FieldValue['type']>('string');

  let saving = $state(false);
  let localError = $state('');

  // Known types suggested from existing cache (for the autocomplete list).
  const knownTypes = $derived(
    Array.from(new Set(cache.nodes.map(n => n.type))).sort()
  );

  function defaultValueFor(type: FieldValue['type']): Field {
    switch (type) {
      case 'string': return { name: newPropName.trim(), type: 'string', value: '' };
      case 'text':   return { name: newPropName.trim(), type: 'text',   value: '' };
      case 'number': return { name: newPropName.trim(), type: 'number', value: 0 };
      case 'bool':   return { name: newPropName.trim(), type: 'bool',   value: false };
      default:       return { name: newPropName.trim(), type: 'string', value: '' };
    }
  }

  function addProperty() {
    const name = newPropName.trim();
    if (!name) { localError = 'Property name is required.'; return; }
    if (properties.some(p => p.name === name)) { localError = 'Property name already used.'; return; }
    properties = [...properties, defaultValueFor(newPropType)];
    newPropName = '';
    localError = '';
  }

  function updateProperty(index: number, updated: Field) {
    properties = properties.map((p, i) => (i === index ? updated : p));
  }

  function removeProperty(index: number) {
    properties = properties.filter((_, i) => i !== index);
  }

  async function save() {
    if (!label.trim()) { localError = 'Label is required.'; return; }
    if (!nodeType.trim()) { localError = 'Type is required.'; return; }
    if (session.chronicleId == null) { localError = 'No chronicle selected.'; return; }

    const tags = tagsText.split(',').map(t => t.trim()).filter(Boolean);

    saving = true;
    localError = '';
    try {
      let saved: ChronicleNode;
      if (node) {
        saved = await api.updateNode(
          node.id, nodeType.trim(), label.trim(), description, tags, properties,
        );
      } else {
        saved = await api.createNode(
          session.chronicleId, nodeType.trim(), label.trim(), description, tags, properties,
        );
        if (parentId != null) {
          try {
            await api.createEdge(
              session.chronicleId, parentId, saved.id, 'contains', '', [],
            );
          } catch (e) {
            localError = `Node created, but linking to parent failed: ${e}`;
          }
        }
      }
      await refreshNodes();
      await refreshEdges();
      selectNode(saved.id);
      onsave(saved);
    } catch (e) {
      localError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form">
  <div class="form-title">{node ? 'Edit node' : (parentId != null ? 'Add child node' : 'Add root node')}</div>

  <div class="field">
    <label for="nf-label">Label</label>
    <input id="nf-label" bind:value={label} placeholder="e.g. Manhattan" />
  </div>

  <div class="field">
    <label for="nf-type">Type</label>
    <input id="nf-type" list="nf-types" bind:value={nodeType} placeholder="area, character, business…" />
    <datalist id="nf-types">
      {#each knownTypes as t (t)}
        <option value={t}></option>
      {/each}
    </datalist>
  </div>

  <div class="field">
    <label for="nf-tags">Tags (comma-separated)</label>
    <input id="nf-tags" bind:value={tagsText} placeholder="nightclub, feeding-ground" />
  </div>

  <div class="field">
    <label for="nf-desc">Description</label>
    <textarea id="nf-desc" bind:value={description} rows={4}></textarea>
  </div>

  <div class="props">
    <div class="props-label">Properties</div>
    {#each properties as p, i (p.name)}
      <PropertyEditor
        field={p}
        readonly={false}
        onchange={(updated) => updateProperty(i, updated)}
        onremove={() => removeProperty(i)}
      />
    {/each}

    <div class="add-prop">
      <input class="small" bind:value={newPropName} placeholder="new property name" />
      <select class="small" bind:value={newPropType}>
        {#each SUPPORTED_TYPES as t (t)}
          <option value={t}>{t}</option>
        {/each}
      </select>
      <button class="btn" onclick={addProperty}>+ Add property</button>
    </div>
  </div>

  {#if localError}
    <p class="error">{localError}</p>
  {/if}

  <div class="actions">
    <button class="btn" onclick={oncancel} disabled={saving}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>

<style>
  .form {
    border: 1px solid var(--border-active);
    background: var(--bg-card);
    border-radius: 6px;
    padding: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .form-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-label);
  }
  .field { display: flex; flex-direction: column; gap: 0.2rem; }
  label {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  input, select, textarea {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    font-family: inherit;
    outline: none;
  }
  input:focus, select:focus, textarea:focus { border-color: var(--accent); }
  textarea { resize: vertical; min-height: 4rem; }
  .small { font-size: 0.7rem; padding: 0.25rem 0.4rem; }
  .props {
    border-top: 1px solid var(--border-faint);
    padding-top: 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .props-label {
    font-size: 0.55rem;
    color: #7c9;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .add-prop {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.35rem;
    align-items: center;
    padding-top: 0.3rem;
  }
  .actions { display: flex; justify-content: flex-end; gap: 0.4rem; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.75rem;
    font-size: 0.74rem;
    cursor: pointer;
  }
  .btn:hover:not(:disabled) { color: var(--text-primary); }
  .btn.primary { color: var(--accent); }
  .btn.primary:hover { box-shadow: 0 0 8px #cc222255; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .error { font-size: 0.7rem; color: var(--accent-bright); }
</style>
