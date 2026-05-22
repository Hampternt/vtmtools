import type { AssetCondition, CustomModifier } from '../../types';

/**
 * Condition multiplier table.
 *
 * `custom` is GM-supplied at purchase time and not present here — callers
 * for `custom` must pass the user's number explicitly.
 *
 * Mirrors the constants in `src-tauri/src/shared/wealth.rs`:
 *   CONDITION_MULTIPLIER_NEW       = 1.0
 *   CONDITION_MULTIPLIER_USED      = 0.6
 *   CONDITION_MULTIPLIER_VERY_USED = 0.3
 */
export const CONDITION_MULTIPLIERS: Record<Exclude<AssetCondition, 'custom'>, number> = {
  new: 1.0,
  used: 0.6,
  very_used: 0.3,
};

/**
 * Compute final purchase price.
 *
 * formula: pickInBand × condition × Π(custom_modifiers[i].mult)
 *
 * @param pickInBand    GM's chosen value within the slot's price band.
 * @param condition     The AssetCondition variant (or 'custom').
 * @param customMult    Required when condition === 'custom'. Ignored otherwise.
 * @param customMods    Stackable modifiers; empty array == no adjustment.
 */
export function computeFinalPrice(
  pickInBand: number,
  condition: AssetCondition,
  customMult: number | null,
  customMods: CustomModifier[],
): number {
  const condMult =
    condition === 'custom'
      ? (customMult ?? 1.0)
      : CONDITION_MULTIPLIERS[condition];

  const modProduct = customMods.reduce((acc, m) => acc * m.mult, 1);

  return pickInBand * condMult * modProduct;
}

/**
 * Default `pickInBand` when the GM hasn't moved the slider: midpoint of the
 * slot's [min, max] band.
 */
export function defaultPickInBand(basePriceMin: number, basePriceMax: number): number {
  return (basePriceMin + basePriceMax) / 2;
}
