import { invoke } from '@tauri-apps/api/core';
import type { ImportOutcome } from '../../types';

/**
 * Push a local advantage row → active Foundry world as a world-level
 * feature Item doc. No-op if Foundry isn't connected (silent success
 * at the IPC layer; the UI gates the button on bridge connectivity).
 *
 * Resolves on success; rejects with a `"tools/library_push: ..."`
 * string if the advantage id is unknown or the wire envelope fails to
 * serialize. Bridge-disconnected case is NOT an error.
 */
export function pushAdvantageToWorld(id: number): Promise<void> {
  return invoke<void>('push_advantage_to_world', { id });
}

/**
 * Read the bridge's `world_items` snapshot for Foundry, filter to
 * feature-type items, and write rows into the local advantages
 * library. Returns the per-row outcome list for the post-import
 * toast (see `summarizeImport` in `./importer.ts`).
 *
 * Caller is responsible for triggering `subscribeToWorldItems()`
 * first so the bridge cache is populated.
 */
export function importAdvantagesFromWorld(): Promise<ImportOutcome[]> {
  return invoke<ImportOutcome[]>('import_advantages_from_world');
}

/**
 * Ask the Foundry module to start streaming world-level Item docs
 * via `bridge.subscribe { collection: "item" }`. The initial snapshot
 * arrives as a `bridge://foundry/items-updated` event (see
 * `src/store/bridge.svelte.ts`); subsequent createItem / updateItem /
 * deleteItem hook events stream after.
 */
export function subscribeToWorldItems(): Promise<void> {
  return invoke<void>('bridge_subscribe',
    { source: 'foundry', collection: 'item' });
}

/**
 * Stop streaming world-level Item docs. Optional — leaving the
 * subscription active across sessions is low-cost (the bridge cache
 * stays warm). Mostly useful for tests / forced re-hydration.
 */
export function unsubscribeFromWorldItems(): Promise<void> {
  return invoke<void>('bridge_unsubscribe',
    { source: 'foundry', collection: 'item' });
}
