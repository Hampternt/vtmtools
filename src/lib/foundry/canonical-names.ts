// src/lib/foundry/canonical-names.ts
//
// Canonical names for WoD5e attributes and skills. Foundation module for:
//   - Plan A: Campaign.svelte's Skills section iterates FOUNDRY_SKILL_NAMES.
//   - Plan B: src/types.ts's CanonicalFieldName template-literal type
//             extends `attribute.${AttributeName}` and `skill.${SkillName}`.
//   - Plan C: src/lib/gm-screen/path-providers.ts's ATTRIBUTE_PROVIDER /
//             SKILL_PROVIDER iterate the runtime arrays for dropdown options.
//
// Mirrors src-tauri/src/shared/canonical_fields.rs::ATTRIBUTE_NAMES and
// SKILL_NAMES. When changing this list, update the Rust arrays in the same
// commit (manual checklist convention; matches the BridgeCharacter mirror
// pattern in src/types.ts).

/** WoD5e v5.3.17 attribute keys (system.attributes.<key>.value). */
export type AttributeName =
  | 'charisma'
  | 'composure'
  | 'dexterity'
  | 'intelligence'
  | 'manipulation'
  | 'resolve'
  | 'stamina'
  | 'strength'
  | 'wits';

/** WoD5e v5.3.17 skill keys (system.skills.<key>.value). */
export type SkillName =
  | 'academics'
  | 'animalken'
  | 'athletics'
  | 'awareness'
  | 'brawl'
  | 'craft'
  | 'drive'
  | 'etiquette'
  | 'finance'
  | 'firearms'
  | 'insight'
  | 'intimidation'
  | 'investigation'
  | 'larceny'
  | 'leadership'
  | 'medicine'
  | 'melee'
  | 'occult'
  | 'performance'
  | 'persuasion'
  | 'politics'
  | 'science'
  | 'stealth'
  | 'streetwise'
  | 'subterfuge'
  | 'survival'
  | 'technology';

/**
 * Runtime array of attribute keys, sorted alphabetically (matches the WoD5e
 * sheet's display order). Use this to iterate when rendering dropdowns or
 * grids — never hardcode the list elsewhere.
 *
 * Mirrors src-tauri/src/shared/canonical_fields.rs::ATTRIBUTE_NAMES.
 */
export const FOUNDRY_ATTRIBUTE_NAMES: readonly AttributeName[] = [
  'charisma',
  'composure',
  'dexterity',
  'intelligence',
  'manipulation',
  'resolve',
  'stamina',
  'strength',
  'wits',
] as const;

/**
 * Runtime array of skill keys, sorted alphabetically.
 *
 * Mirrors src-tauri/src/shared/canonical_fields.rs::SKILL_NAMES.
 */
export const FOUNDRY_SKILL_NAMES: readonly SkillName[] = [
  'academics',
  'animalken',
  'athletics',
  'awareness',
  'brawl',
  'craft',
  'drive',
  'etiquette',
  'finance',
  'firearms',
  'insight',
  'intimidation',
  'investigation',
  'larceny',
  'leadership',
  'medicine',
  'melee',
  'occult',
  'performance',
  'persuasion',
  'politics',
  'science',
  'stealth',
  'streetwise',
  'subterfuge',
  'survival',
  'technology',
] as const;

/**
 * Canonical path strings for V5 vital tracks. Used by:
 *   - active-deltas.readPath (path-resolver targets)
 *   - ModifierEffectEditor path-input autocomplete
 *
 * Path strings match the dot-path the resolver expects (see active-deltas.ts).
 * `health.superficial` / `health.aggravated` / `willpower.superficial` /
 * `willpower.aggravated` paths resolve correctly but are intentionally OMITTED
 * from autocomplete — uncommon modifier targets; user can type them by hand.
 */
export const FOUNDRY_VITAL_PATHS = [
  'hunger',
  'humanity',
  'humanity.stains',
  'health.max',
  'willpower.max',
  'blood.potency',
] as const;

/**
 * Per-character discipline path autocomplete. Reads the actor's current
 * disciplines map and returns canonical paths like 'disciplines.auspex'.
 * Returns [] for non-Foundry chars or chars with no disciplines field.
 */
export function foundryDisciplineNames(
  char: import('../../types').BridgeCharacter,
): string[] {
  if (char.source !== 'foundry') return [];
  const raw = char.raw as { system?: { disciplines?: Record<string, unknown> } } | null;
  const disciplines = raw?.system?.disciplines;
  if (!disciplines || typeof disciplines !== 'object') return [];
  return Object.keys(disciplines).map(name => `disciplines.${name}`);
}
