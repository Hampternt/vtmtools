// Saved characters runes store. Wraps the saved_character_* Tauri commands
// behind a stable surface so components can call .save / .update / .delete
// without re-fetching the list themselves.

import {
  listSavedCharacters,
  saveCharacter,
  updateSavedCharacter,
  deleteSavedCharacter,
  type SavedCharacter,
} from '$lib/saved-characters/api';
import type { BridgeCharacter } from '../types';

let _list = $state<SavedCharacter[]>([]);
let _loading = $state(false);
let _error = $state<string | null>(null);
let _initialized = false;

async function refresh(): Promise<void> {
  _loading = true;
  _error = null;
  try {
    _list = await listSavedCharacters();
  } catch (e) {
    _error = String(e);
    console.error('[savedCharacters] refresh failed:', e);
  } finally {
    _loading = false;
  }
}

export const savedCharacters = {
  get list() { return _list; },
  get loading() { return _loading; },
  get error() { return _error; },
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
  },
  async refresh(): Promise<void> { await refresh(); },
  async save(canonical: BridgeCharacter, foundryWorld: string | null): Promise<void> {
    await saveCharacter(canonical, foundryWorld);
    await refresh();
  },
  async update(id: number, canonical: BridgeCharacter): Promise<void> {
    await updateSavedCharacter(id, canonical);
    await refresh();
  },
  async delete(id: number): Promise<void> {
    await deleteSavedCharacter(id);
    await refresh();
  },
  /** Convenience: find a saved row matching a live char by (source, source_id).
   *  Note: BridgeCharacter (live) is snake_case (`source_id`) since Rust's
   *  CanonicalCharacter has no camelCase serde rename, while SavedCharacter
   *  is camelCase (Rust struct has `#[serde(rename_all = "camelCase")]`). */
  findMatch(live: BridgeCharacter): SavedCharacter | undefined {
    return _list.find(s => s.source === live.source && s.sourceId === live.source_id);
  },
};
