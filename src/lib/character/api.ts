// Typed wrappers around character_set_field / character_add_advantage /
// character_remove_advantage. Per CLAUDE.md, components must NOT call
// invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type {
  SourceKind,
  WriteTarget,
  CanonicalFieldName,
  FeatureType,
} from '../../types';

export type { WriteTarget, CanonicalFieldName, FeatureType } from '../../types';

export const characterSetField = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('character_set_field', { target, source, sourceId, name, value });

export const patchSavedField = (
  id: number,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('patch_saved_field', { id, name, value });

export const characterAddAdvantage = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  featuretype: FeatureType,
  name: string,
  description: string,
  points: number,
): Promise<void> =>
  invoke<void>('character_add_advantage', {
    target, source, sourceId, featuretype, name, description, points,
  });

export const characterRemoveAdvantage = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  featuretype: FeatureType,
  itemId: string,
): Promise<void> =>
  invoke<void>('character_remove_advantage', {
    target, source, sourceId, featuretype, itemId,
  });
