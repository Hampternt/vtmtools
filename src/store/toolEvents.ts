import { writable } from 'svelte/store';
import type { Writable } from 'svelte/store';

export interface ResonanceEvent {
  type: 'resonance_result';
  payload: {
    temperament: string;
    resonanceType: string | null;
    isAcute: boolean;
    dyscrasiaName: string | null;
  };
}

export type ToolEvent = ResonanceEvent;

// Tools publish events here. Other tools subscribe as needed.
export const toolEvents: Writable<ToolEvent | null> = writable(null);

export function publishEvent(event: ToolEvent): void {
  toolEvents.set(event);
}
