/**
 * The permission matrix. Returns the set of actions available for a given
 * (source, target) pair. Empty = invalid drop (auto-cancel). Single =
 * execute immediately. ≥2 = open DropMenu with the action list.
 *
 * v1 returns one action for two specific cells (free-bound card moving
 * between Character and Situational zones on the same character row).
 * Everything else returns [].
 *
 * v2 will add four cells for cross-row Move/Copy. v3 will add two cells
 * for template-source → situational-zone Apply. Each new phase adds
 * branches here — no contract changes.
 *
 * See spec §"Permission matrix" for the full v1/v2/v3 matrix table.
 */
import type { Action, DragSource, DropTarget } from './types';

export function getActionsFor(source: DragSource, target: DropTarget): Action[] {
  // Advantage-bound source: always invalid (zone-locked to character; no cross-row in v1).
  if (source.kind === 'advantage') return [];

  // Template-source: v3 only. In v1 always invalid.
  if (source.kind === 'template') return [];

  // Free-bound source from here. v1 same-row constraint: target character
  // must match source character.
  const sameChar =
    source.mod.source === target.character.source &&
    source.mod.sourceId === target.character.source_id;
  if (!sameChar) return [];

  // Move from character-zone → situational-zone.
  if (source.mod.zone === 'character' && target.kind === 'situational-zone') {
    return [{ id: 'move-zone', label: 'Move to Situational', newZone: 'situational' }];
  }
  // Move from situational-zone → character-zone.
  if (source.mod.zone === 'situational' && target.kind === 'character-zone') {
    return [{ id: 'move-zone', label: 'Move to Character', newZone: 'character' }];
  }

  // Dropping on the same zone the card already lives in: no-op (invalid → snap back).
  return [];
}
