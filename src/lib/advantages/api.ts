import { invoke } from '@tauri-apps/api/core';
import type { Advantage, AdvantageKind, Field, ModifierEffect } from '../../types';

export type AdvantageInput = {
  name: string;
  description: string;
  kind: AdvantageKind;
  tags: string[];
  properties: Field[];
  /**
   * Item Builder Shell, Task 7: per-advantage ModifierEffect list. Required
   * (the Tauri `add_advantage` / `update_advantage` commands expect it on
   * every call); pass `[]` when authoring without bonuses. Forward-compat
   * storage — push_advantage_to_world does NOT yet consume these.
   */
  effects: ModifierEffect[];
  /**
   * Item Builder Shell: omit (or false) for normal saves. Set true to
   * persist a draft that bypasses validation. Task 6 only adds the
   * column — the add/update Tauri commands currently ignore this field;
   * future tasks (18+) will plumb it through.
   */
  wip?: boolean;
  /** Stage keys the user explicitly confirmed in the multi-stage builder. */
  confirmedFields?: string[];
};

export function listAdvantages(): Promise<Advantage[]> {
  return invoke<Advantage[]>('list_advantages');
}

export function addAdvantage(input: AdvantageInput): Promise<Advantage> {
  return invoke<Advantage>('add_advantage', input);
}

export function updateAdvantage(id: number, input: AdvantageInput): Promise<void> {
  return invoke<void>('update_advantage', { id, ...input });
}

export function deleteAdvantage(id: number): Promise<void> {
  return invoke<void>('delete_advantage', { id });
}

export function rollRandomAdvantage(tags: string[]): Promise<Advantage | null> {
  return invoke<Advantage | null>('roll_random_advantage', { tags });
}
