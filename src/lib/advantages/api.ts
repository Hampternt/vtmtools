import { invoke } from '@tauri-apps/api/core';
import type { Advantage, Field } from '../../types';

export type AdvantageInput = {
  name: string;
  description: string;
  tags: string[];
  properties: Field[];
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
