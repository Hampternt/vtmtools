import { invoke } from '@tauri-apps/api/core';

export interface RollV5PoolInput {
  actorId: string;
  valuePaths: string[];
  difficulty: number;
  flavor?: string | null;
  advancedDice?: number | null;
  selectors?: string[] | null;
}

export interface PostChatAsActorInput {
  actorId: string;
  content: string;
  flavor?: string | null;
  rollMode?: 'roll' | 'gmroll' | 'blindroll' | 'selfroll' | null;
}

export async function triggerFoundryRoll(input: RollV5PoolInput): Promise<void> {
  await invoke('trigger_foundry_roll', { input });
}

export async function postFoundryChat(input: PostChatAsActorInput): Promise<void> {
  await invoke('post_foundry_chat', { input });
}
