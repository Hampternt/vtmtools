// Typed wrappers around the bridge_* Tauri commands. Per CLAUDE.md,
// components must NOT call invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type { BridgeCharacter, SourceKind } from '../../types';

export const getStatus = (): Promise<Record<SourceKind, boolean>> =>
  invoke<Record<SourceKind, boolean>>('bridge_get_status');

export const getCharacters = (): Promise<BridgeCharacter[]> =>
  invoke<BridgeCharacter[]>('bridge_get_characters');

export const refresh = (source?: SourceKind): Promise<void> =>
  invoke<void>('bridge_refresh', { source });

export const setAttribute = (
  source: SourceKind,
  sourceId: string,
  name: string,
  value: string,
): Promise<void> =>
  invoke<void>('bridge_set_attribute', { source, sourceId, name, value });
