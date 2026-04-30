// Shared bridge state for Resonance + Campaign tools. Listens for the
// three bridge://* events and exposes per-source connection state plus
// the merged character list.

import { listen } from '@tauri-apps/api/event';
import { getStatus, getCharacters, bridgeGetSourceInfo, type SourceInfo } from '$lib/bridge/api';
import type { BridgeCharacter, SourceKind } from '../types';

interface BridgeStore {
  connections: Record<SourceKind, boolean>;
  sourceInfo: Record<SourceKind, SourceInfo | null>;
  characters: BridgeCharacter[];
  lastSync: Date | null;
}

export const bridge = $state<BridgeStore>({
  connections: { roll20: false, foundry: false },
  sourceInfo: { roll20: null, foundry: null },
  characters: [],
  lastSync: null,
});

let initialized = false;

async function refreshSourceInfo(source: SourceKind): Promise<void> {
  try {
    bridge.sourceInfo[source] = await bridgeGetSourceInfo(source);
  } catch (e) {
    console.warn(`[bridge] bridgeGetSourceInfo(${source}) failed:`, e);
  }
}

export async function initBridge(): Promise<void> {
  if (initialized) return;
  initialized = true;

  try {
    bridge.connections = await getStatus();
  } catch (e) {
    console.warn('[bridge] getStatus failed:', e);
  }

  try {
    bridge.characters = await getCharacters();
  } catch (e) {
    console.warn('[bridge] getCharacters failed:', e);
  }

  // Initial fetch — covers the case where the bridge connected before
  // initBridge ran, so the connected listener never fired.
  void refreshSourceInfo('roll20');
  void refreshSourceInfo('foundry');

  await Promise.all([
    listen<void>('bridge://roll20/connected', () => {
      bridge.connections.roll20 = true;
      void refreshSourceInfo('roll20');
    }),
    listen<void>('bridge://roll20/disconnected', () => {
      bridge.connections.roll20 = false;
      bridge.sourceInfo.roll20 = null;
    }),
    listen<void>('bridge://foundry/connected', () => {
      bridge.connections.foundry = true;
      void refreshSourceInfo('foundry');
    }),
    listen<void>('bridge://foundry/disconnected', () => {
      bridge.connections.foundry = false;
      bridge.sourceInfo.foundry = null;
    }),
    listen<BridgeCharacter[]>('bridge://characters-updated', (e) => {
      bridge.characters = e.payload;
      bridge.lastSync = new Date();
    }),
  ]);
}

export function anyConnected(): boolean {
  return bridge.connections.roll20 || bridge.connections.foundry;
}

export function sourceLabel(s: SourceKind): string {
  return s === 'roll20' ? 'Roll20' : 'Foundry';
}
