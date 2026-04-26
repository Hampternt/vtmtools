// Shared bridge state for Resonance + Campaign tools. Listens for the
// three bridge://* events and exposes per-source connection state plus
// the merged character list.

import { listen } from '@tauri-apps/api/event';
import { getStatus, getCharacters } from '$lib/bridge/api';
import type { BridgeCharacter, SourceKind } from '../types';

interface BridgeStore {
  connections: Record<SourceKind, boolean>;
  characters: BridgeCharacter[];
  lastSync: Date | null;
}

export const bridge = $state<BridgeStore>({
  connections: { roll20: false, foundry: false },
  characters: [],
  lastSync: null,
});

let initialized = false;

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

  await Promise.all([
    listen<void>('bridge://roll20/connected', () => {
      bridge.connections.roll20 = true;
    }),
    listen<void>('bridge://roll20/disconnected', () => {
      bridge.connections.roll20 = false;
    }),
    listen<void>('bridge://foundry/connected', () => {
      bridge.connections.foundry = true;
    }),
    listen<void>('bridge://foundry/disconnected', () => {
      bridge.connections.foundry = false;
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
