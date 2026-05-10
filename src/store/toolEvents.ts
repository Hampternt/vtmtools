import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';
import type { SourceKind } from '../types';

export interface ResonanceEvent {
  type: 'resonance_result';
  payload: {
    temperament: string;
    resonanceType: string | null;
    isAcute: boolean;
    dyscrasiaName: string | null;
  };
}

/**
 * Cross-tool navigate request — any tool can dispatch to ask the layout
 * to focus the given character in the GM Screen (switch tool if needed,
 * scroll the matching row into view, flash). Subscribed by +layout.svelte
 * (not GmScreen.svelte) because the GM Screen is dynamically imported and
 * unmounted when inactive — its onMount can't receive an event published
 * from another tool. See docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md §8.
 */
export interface NavigateToCharacterEvent {
  type: 'navigate-to-character';
  source: SourceKind;
  sourceId: string;
}

export type ToolEvent = ResonanceEvent | NavigateToCharacterEvent;

// Tools publish events here. Other tools subscribe as needed.
export const toolEvents: Writable<ToolEvent | null> = writable(null);

export function publishEvent(event: ToolEvent): void {
  toolEvents.set(event);
}
