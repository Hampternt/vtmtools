// Typed wrappers around the saved_character_* Tauri commands. Per CLAUDE.md,
// components must NOT call invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type { BridgeCharacter, SourceKind } from '../../types';

export interface SavedCharacter {
  id: number;
  source: SourceKind;
  sourceId: string;
  foundryWorld: string | null;
  name: string;
  canonical: BridgeCharacter;
  savedAt: string;
  lastUpdatedAt: string;
  /** ISO-8601 timestamp set by the bridge when the live actor was
   *  observed deleted from its source VTT. null = not known to be deleted.
   *  Owned by the bridge; saving / updating does not touch this. */
  deletedInVttAt: string | null;
}

export const saveCharacter = (
  canonical: BridgeCharacter,
  foundryWorld: string | null,
): Promise<number> =>
  invoke<number>('save_character', { canonical, foundryWorld });

export const listSavedCharacters = (): Promise<SavedCharacter[]> =>
  invoke<SavedCharacter[]>('list_saved_characters');

export const updateSavedCharacter = (
  id: number,
  canonical: BridgeCharacter,
): Promise<void> =>
  invoke<void>('update_saved_character', { id, canonical });

export const deleteSavedCharacter = (id: number): Promise<void> =>
  invoke<void>('delete_saved_character', { id });
