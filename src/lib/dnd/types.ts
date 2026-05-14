/**
 * Discriminated unions for the DnD primitive's source, target, and action
 * contracts. Pinned in v1 so v2 (cross-row drag) and v3 (Status Template
 * palette as drag source) can extend without breaking — adding new variants
 * is additive, never structural.
 *
 * See spec docs/superpowers/specs/2026-05-14-gm-screen-modifier-zones-and-dnd-design.md
 * §"Source / Target contracts" and §"Permission matrix".
 */
import type {
  BridgeCharacter,
  CharacterModifier,
  ModifierZone,
  StatusTemplate,
} from '../../types';

/**
 * What is being dragged. Each variant carries the full source object so
 * action handlers don't have to refetch.
 *
 * v1 issues only `free-mod` from pointerdown handlers in DragSource.
 * Advantage-bound modifiers ARE allowed to enter the held state (the
 * matrix function rejects every drop target, so they snap back) — this
 * keeps the matrix as the single source of truth.
 */
export type DragSource =
  | { kind: 'free-mod'; mod: CharacterModifier }
  | { kind: 'advantage'; mod: CharacterModifier }   // v1: every target returns []
  | { kind: 'template'; template: StatusTemplate }; // v3: not used in v1

/**
 * Where a drop can land. The character is carried so cross-row v2 logic
 * (and the v1 same-row constraint) can compare source.character to target.character.
 */
export type DropTarget =
  | { kind: 'character-zone'; character: BridgeCharacter }
  | { kind: 'situational-zone'; character: BridgeCharacter };

/**
 * One action the user can execute by completing the drop. Returned from
 * `getActionsFor(source, target)`. Empty array = invalid drop = auto-cancel.
 * Single-element array = execute immediately. ≥2 elements = open DropMenu.
 *
 * v1 emits only `move-zone`. The other variants are pinned for v2/v3.
 */
export type Action =
  | { id: 'move-zone';      label: string; newZone: ModifierZone }   // v1
  | { id: 'move-character'; label: string; newSourceId: string }     // v2
  | { id: 'copy-character'; label: string; newSourceId: string }     // v2
  | { id: 'apply-template'; label: string; zone: ModifierZone };     // v3
