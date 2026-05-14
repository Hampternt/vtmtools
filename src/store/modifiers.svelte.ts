// GM Screen modifiers runes store. Wraps the modifier IPC commands so
// components can call .add / .update / .delete / .setActive / .setHidden /
// .materializeAdvantage without re-fetching the full list themselves.
// Per spec §8.6: no auto-refetch on bridge://characters-updated — modifier
// records are independent of bridge state. Refetch only on mount + on
// successful CRUD response (which carries the updated row).

import {
  listAllCharacterModifiers,
  addCharacterModifier,
  updateCharacterModifier,
  deleteCharacterModifier,
  setModifierActive,
  setModifierHidden,
  setModifierZone,
  materializeAdvantageModifier,
  pushToFoundry as apiPushToFoundry,
} from '$lib/modifiers/api';
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  PushReport,
  SourceKind,
  ModifierZone,
} from '../types';
import { listen } from '@tauri-apps/api/event';

let _list = $state<CharacterModifier[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);
let _initialized = false;

// UI preferences — per spec §8.3 / §7.5; held in-memory only, not persisted.
let _activeFilterTags = $state<Set<string>>(new Set());
let _showHidden = $state(false);
let _showOrphans = $state(false);

async function refresh(): Promise<void> {
  _loading = true;
  _error = null;
  try {
    _list = await listAllCharacterModifiers();
  } catch (e) {
    _error = String(e);
    console.error('[modifiers] refresh failed:', e);
  } finally {
    _loading = false;
  }
}

function mergeRow(updated: CharacterModifier): void {
  const i = _list.findIndex(m => m.id === updated.id);
  if (i >= 0) _list[i] = updated; else _list.push(updated);
}

function dropRow(id: number): void {
  _list = _list.filter(m => m.id !== id);
}

export const modifiers = {
  get list() { return _list; },
  get loading() { return _loading; },
  get error() { return _error; },

  // UI prefs (reactive getters/setters via runes)
  get activeFilterTags() { return _activeFilterTags; },
  setActiveFilterTags(next: Set<string>) { _activeFilterTags = new Set(next); },
  get showHidden() { return _showHidden; },
  set showHidden(v: boolean) { _showHidden = v; },
  get showOrphans() { return _showOrphans; },
  set showOrphans(v: boolean) { _showOrphans = v; },

  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
    // Subscribe to backend-initiated row reaps (live deleteItem from Foundry).
    // The event carries the exact ids to drop — no refetch needed. The store's
    // long-standing "no auto-refetch on bridge state" invariant (see header
    // comment) is preserved: this is an explicit cleanup signal, not a state
    // diff. Spec §6.2 of
    // docs/superpowers/specs/2026-05-13-gm-screen-live-data-priority-design.md.
    void listen<{ ids: number[] }>('modifiers://rows-reaped', (e) => {
      for (const id of e.payload.ids) dropRow(id);
    });
  },
  async refresh(): Promise<void> { await refresh(); },

  /** Lookup helpers — caller filters in-memory for free. */
  forCharacter(source: SourceKind, sourceId: string): CharacterModifier[] {
    return _list.filter(m => m.source === source && m.sourceId === sourceId);
  },

  /** CRUD — each refreshes the row in the local list from the response. */
  async add(input: NewCharacterModifierInput): Promise<CharacterModifier> {
    const row = await addCharacterModifier(input);
    mergeRow(row);
    return row;
  },
  async update(id: number, patch: ModifierPatchInput): Promise<CharacterModifier> {
    const row = await updateCharacterModifier(id, patch);
    mergeRow(row);
    return row;
  },
  async delete(id: number): Promise<void> {
    await deleteCharacterModifier(id);
    dropRow(id);
  },
  async setActive(id: number, isActive: boolean): Promise<void> {
    await setModifierActive(id, isActive);
    const i = _list.findIndex(m => m.id === id);
    if (i >= 0) _list[i] = { ..._list[i], isActive };
  },
  async setHidden(id: number, isHidden: boolean): Promise<void> {
    await setModifierHidden(id, isHidden);
    const i = _list.findIndex(m => m.id === id);
    if (i >= 0) _list[i] = { ..._list[i], isHidden };
  },
  async setZone(id: number, zone: ModifierZone): Promise<void> {
    const row = await setModifierZone(id, zone);
    mergeRow(row);
  },
  async materializeAdvantage(args: {
    source: SourceKind;
    sourceId: string;
    itemId: string;
    name: string;
    description: string;
  }): Promise<CharacterModifier> {
    const row = await materializeAdvantageModifier(args);
    mergeRow(row);
    return row;
  },
  async pushToFoundry(modifierId: number): Promise<PushReport> {
    return await apiPushToFoundry(modifierId);
  },
};
