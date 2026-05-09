// src/lib/gm-screen/path-providers.ts
//
// Pluggable registry for "what can the GM pick to feed value_paths in a roll".
// V1 ships attribute + skill providers; future plans append discipline /
// merit-bonus / renown / werewolf-rage providers WITHOUT touching the
// RollDispatcherPopover component.
//
// Usage: the popover iterates DEFAULT_PROVIDERS and renders one <select>
// per provider whose getOptions() returns a non-empty list. The composer
// (roll.ts::dispatchRoll) walks the same providers to build value_paths
// from the user's selections.

import type { BridgeCharacter } from '../../types';
import { foundryAttrInt, foundrySkillInt } from '../foundry/raw';
import {
  FOUNDRY_ATTRIBUTE_NAMES,
  FOUNDRY_SKILL_NAMES,
} from '../foundry/canonical-names';

/** One row in a provider's dropdown — what the GM picks. */
export interface PathProviderOption {
  /** Stable key used in the popover's selections record (e.g. 'strength'). */
  key: string;
  /** Display label shown in the dropdown (e.g. 'Strength'). */
  label: string;
  /** Current sheet value for the displayed "(3)" hint. */
  value: number;
  /** Foundry dot-path joined into value_paths (e.g. 'attributes.strength.value'). */
  path: string;
}

/** A category of stats the popover can pick from. */
export interface PathProvider {
  /** Stable id used as the key in the popover's selections record. */
  id: string;
  /** Display label rendered above the dropdown (e.g. 'Attribute'). */
  label: string;
  /** When true, the popover blocks submit until this provider has a selection. */
  required: boolean;
  /**
   * Returns the dropdown options for this character. Empty list = the popover
   * skips rendering this provider's <select> (useful for splat-aware future
   * providers like 'discipline' that return [] for non-vampire characters).
   */
  getOptions(char: BridgeCharacter): PathProviderOption[];
}

/** Capitalize the first letter (display-only helper). */
function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** WoD5e attributes (system.attributes.<key>.value). */
export const ATTRIBUTE_PROVIDER: PathProvider = {
  id: 'attribute',
  label: 'Attribute',
  required: true,
  getOptions: (char) =>
    FOUNDRY_ATTRIBUTE_NAMES.map((key) => ({
      key,
      label: capitalize(key),
      value: foundryAttrInt(char, key),
      path: `attributes.${key}.value`,
    })),
};

/** WoD5e skills (system.skills.<key>.value). */
export const SKILL_PROVIDER: PathProvider = {
  id: 'skill',
  label: 'Skill',
  required: true,
  getOptions: (char) =>
    FOUNDRY_SKILL_NAMES.map((key) => ({
      key,
      label: capitalize(key),
      value: foundrySkillInt(char, key),
      path: `skills.${key}.value`,
    })),
};

/**
 * V1 registry. Future providers are appended here without touching
 * RollDispatcherPopover.svelte or roll.ts:
 *
 *   export const DISCIPLINE_PROVIDER: PathProvider = { ... }
 *   export const MERIT_BONUS_PROVIDER: PathProvider = { ... }
 *   export const RENOWN_PROVIDER: PathProvider = { ... }
 *   export const DEFAULT_PROVIDERS = [
 *     ATTRIBUTE_PROVIDER,
 *     SKILL_PROVIDER,
 *     DISCIPLINE_PROVIDER,   // <-- here
 *   ];
 */
export const DEFAULT_PROVIDERS: readonly PathProvider[] = [
  ATTRIBUTE_PROVIDER,
  SKILL_PROVIDER,
];
