import { invoke } from '@tauri-apps/api/core';

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
