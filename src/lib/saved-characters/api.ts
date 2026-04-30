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
