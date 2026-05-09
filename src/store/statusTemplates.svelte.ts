// GM Screen status templates runes store. Mirrors src/store/modifiers.svelte.ts
// shape: initialized flag, ensureLoaded / refresh, CRUD methods that merge
// the response row into the local list.

import {
  listStatusTemplates,
  addStatusTemplate,
  updateStatusTemplate,
  deleteStatusTemplate,
} from '$lib/modifiers/api';
import type {
  StatusTemplate,
  NewStatusTemplateInput,
  StatusTemplatePatchInput,
} from '../types';

let _list = $state<StatusTemplate[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);
let _initialized = false;

async function refresh(): Promise<void> {
  _loading = true;
  _error = null;
  try {
    _list = await listStatusTemplates();
  } catch (e) {
    _error = String(e);
    console.error('[statusTemplates] refresh failed:', e);
  } finally {
    _loading = false;
  }
}

function mergeRow(updated: StatusTemplate): void {
  const i = _list.findIndex(t => t.id === updated.id);
  if (i >= 0) _list[i] = updated; else _list.push(updated);
}

function dropRow(id: number): void {
  _list = _list.filter(t => t.id !== id);
}

export const statusTemplates = {
  get list() { return _list; },
  get loading() { return _loading; },
  get error() { return _error; },
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
  },
  async refresh(): Promise<void> { await refresh(); },
  async add(input: NewStatusTemplateInput): Promise<StatusTemplate> {
    const row = await addStatusTemplate(input);
    mergeRow(row);
    return row;
  },
  async update(id: number, patch: StatusTemplatePatchInput): Promise<StatusTemplate> {
    const row = await updateStatusTemplate(id, patch);
    mergeRow(row);
    return row;
  },
  async delete(id: number): Promise<void> {
    await deleteStatusTemplate(id);
    dropRow(id);
  },
  byId(id: number): StatusTemplate | undefined {
    return _list.find(t => t.id === id);
  },
};
