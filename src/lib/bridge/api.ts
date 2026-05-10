// Typed wrappers around the bridge_* Tauri commands. Per CLAUDE.md,
// components must NOT call invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type { BridgeCharacter, CanonicalRoll, SourceKind } from '../../types';

export type { BridgeCharacter, CanonicalRoll, SourceKind };

export interface SourceInfo {
  worldId: string | null;
  worldTitle: string | null;
  systemId: string | null;
  systemVersion: string | null;
  protocolVersion: number;
  capabilities: string[];
}

export const getStatus = (): Promise<Record<SourceKind, boolean>> =>
  invoke<Record<SourceKind, boolean>>('bridge_get_status');

export const getCharacters = (): Promise<BridgeCharacter[]> =>
  invoke<BridgeCharacter[]>('bridge_get_characters');

export const bridgeGetRolls = (): Promise<CanonicalRoll[]> =>
  invoke<CanonicalRoll[]>('bridge_get_rolls');

export const refresh = (source?: SourceKind): Promise<void> =>
  invoke<void>('bridge_refresh', { source });

export const setAttribute = (
  source: SourceKind,
  sourceId: string,
  name: string,
  value: string,
): Promise<void> =>
  invoke<void>('bridge_set_attribute', { source, sourceId, name, value });

export const bridgeGetSourceInfo = (source: SourceKind): Promise<SourceInfo | null> =>
  invoke<SourceInfo | null>('bridge_get_source_info', { source });
