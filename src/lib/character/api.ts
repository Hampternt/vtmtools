// Typed wrapper around character_set_field. Per CLAUDE.md, components must
// NOT call invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type { SourceKind, WriteTarget, CanonicalFieldName } from '../../types';

export type { WriteTarget, CanonicalFieldName } from '../../types';

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
