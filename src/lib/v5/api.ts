import { invoke } from '@tauri-apps/api/core';

export interface PoolPart {
  name: string;
  level: number;
}

export type DieKind = 'regular' | 'hunger';

export interface Die {
  kind: DieKind;
  value: number;
}

export interface PoolSpec {
  parts: PoolPart[];
  regularCount: number;
  hungerCount: number;
}

export interface RollResult {
  parts: PoolPart[];
  dice: Die[];
}

export interface Tally {
  successes: number;
  critPairs: number;
  isCritical: boolean;
  isMessyCritical: boolean;
  hasHungerOne: boolean;
}

export interface OutcomeFlags {
  critical: boolean;
  messy: boolean;
  bestialFailure: boolean;
  totalFailure: boolean;
}

export interface Outcome {
  successes: number;
  difficulty: number;
  margin: number;
  passed: boolean;
  flags: OutcomeFlags;
}

export interface SkillCheckInput {
  characterName: string | null;
  attribute: PoolPart;
  skill: PoolPart;
  hunger: number;        // 0..=5
  specialty: string | null;
  difficulty: number;
}

export interface SkillCheckResult {
  spec: PoolSpec;
  roll: RollResult;
  tally: Tally;
  outcome: Outcome;
  message: string;
}

export async function rollSkillCheck(input: SkillCheckInput): Promise<SkillCheckResult> {
  return await invoke<SkillCheckResult>('roll_skill_check', { input });
}
