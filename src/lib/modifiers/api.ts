import { invoke } from '@tauri-apps/api/core';
import type {
  CharacterModifier,
  NewCharacterModifierInput,
  ModifierPatchInput,
  SourceKind,
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
