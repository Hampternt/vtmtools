// Rolls store — primes from the Rust bridge ring on mount, then listens
// for live `bridge://roll-received` events for incremental updates.
//
// Bounded at RING_MAX entries on the frontend too, mirroring the backend
// VecDeque<CanonicalRoll>. Dedup-by-source_id matches Rust's
// BridgeState::push_roll: if Foundry re-emits the same chat-message ID,
// the existing entry is replaced and re-fronted (newest-first).
//
// See docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §8.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { bridgeGetRolls } from '$lib/bridge/api';
import type { CanonicalRoll } from '../types';

const RING_MAX = 200;

class RollsStore {
  list = $state<CanonicalRoll[]>([]);
  #unlisten: UnlistenFn | null = null;
  #loaded = false;

  async ensureLoaded(): Promise<void> {
    if (this.#loaded) return;
    this.#loaded = true;

    // Prime from the Rust ring snapshot.
    try {
      this.list = await bridgeGetRolls();
    } catch (err) {
      console.error('[rolls] bridge_get_rolls failed:', err);
      this.list = [];
    }

    // Subscribe to live emits.
    try {
      this.#unlisten = await listen<CanonicalRoll>('bridge://roll-received', (e) => {
        const incoming = e.payload;
        // Dedup by source_id; place newest first.
        this.list = [
          incoming,
          ...this.list.filter((r) => r.source_id !== incoming.source_id),
        ].slice(0, RING_MAX);
      });
    } catch (err) {
      console.error('[rolls] listen(bridge://roll-received) failed:', err);
    }
  }

  /** Test/dev hook — clear the in-memory list. Does not affect the Rust ring. */
  clear(): void {
    this.list = [];
  }
}

export const rolls = new RollsStore();
