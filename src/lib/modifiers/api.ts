import { invoke } from '@tauri-apps/api/core';
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  PushReport,
  SourceKind,
  StatusTemplate,
  NewStatusTemplateInput,
  StatusTemplatePatchInput,
} from '../../types';

export function listCharacterModifiers(
  source: SourceKind,
  sourceId: string,
): Promise<CharacterModifier[]> {
  return invoke<CharacterModifier[]>('list_character_modifiers', { source, sourceId });
}

export function listAllCharacterModifiers(): Promise<CharacterModifier[]> {
  return invoke<CharacterModifier[]>('list_all_character_modifiers');
}

export function addCharacterModifier(input: NewCharacterModifierInput): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('add_character_modifier', { input });
}

export function updateCharacterModifier(
  id: number,
  patch: ModifierPatchInput,
): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('update_character_modifier', { id, patch });
}

export function deleteCharacterModifier(id: number): Promise<void> {
  return invoke<void>('delete_character_modifier', { id });
}

export function setModifierActive(id: number, isActive: boolean): Promise<void> {
  return invoke<void>('set_modifier_active', { id, isActive });
}

export function setModifierHidden(id: number, isHidden: boolean): Promise<void> {
  return invoke<void>('set_modifier_hidden', { id, isHidden });
}

export function materializeAdvantageModifier(args: {
  source: SourceKind;
  sourceId: string;
  itemId: string;
  name: string;
  description: string;
}): Promise<CharacterModifier> {
  return invoke<CharacterModifier>('materialize_advantage_modifier', args);
}

/**
 * Push the modifier's pool effects to its bound merit's `system.bonuses[]`
 * on the live Foundry actor via the bridge. Idempotent — re-pressing replaces
 * our prior bonuses for this modifier without touching player-added ones.
 * Difficulty/note effects are skipped and surfaced in the returned PushReport.
 */
export function pushToFoundry(modifierId: number): Promise<PushReport> {
  return invoke<PushReport>('gm_screen_push_to_foundry', { modifierId });
}

export function listStatusTemplates(): Promise<StatusTemplate[]> {
  return invoke<StatusTemplate[]>('list_status_templates');
}

export function addStatusTemplate(input: NewStatusTemplateInput): Promise<StatusTemplate> {
  return invoke<StatusTemplate>('add_status_template', { input });
}

export function updateStatusTemplate(
  id: number,
  patch: StatusTemplatePatchInput,
): Promise<StatusTemplate> {
  return invoke<StatusTemplate>('update_status_template', { id, patch });
}

export function deleteStatusTemplate(id: number): Promise<void> {
  return invoke<void>('delete_status_template', { id });
}
