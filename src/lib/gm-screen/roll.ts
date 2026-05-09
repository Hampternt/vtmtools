// src/lib/gm-screen/roll.ts
//
// Pure helpers consumed by RollDispatcherPopover.svelte:
//   - summarizeModifiers(mods): partition active non-hidden modifier effects
//     by kind, sum pool/difficulty deltas, collect note text for display.
//   - dispatchRoll(args): validate popover state, build value_paths from
//     PathProvider selections, call triggerFoundryRoll with the composed
//     payload (clamps difficulty to >= 0; uses customFlavor or auto-label).

import { triggerFoundryRoll } from '../foundry-chat/api';
import type { BridgeCharacter, CharacterModifier } from '../../types';
import type { PathProvider } from './path-providers';

/** Aggregated view of one character's active modifier deck. */
export interface ModifierSums {
  /** Sum of every active non-hidden effect's pool delta. */
  pool: number;
  /** Sum of every active non-hidden effect's difficulty delta. */
  difficulty: number;
  /** Notes from active non-hidden 'note'-kind effects (display-only). */
  notes: string[];
}

/**
 * Partition + sum effects from a character's modifier list. Filters to
 * isActive && !isHidden (matches the popover's "what's currently on?" view
 * — the user explicitly framed this as 'sum total of all on toggled cards').
 */
export function summarizeModifiers(mods: CharacterModifier[]): ModifierSums {
  const active = mods.filter((m) => m.isActive && !m.isHidden);
  const allEffects = active.flatMap((m) => m.effects);

  return {
    pool: allEffects
      .filter((e) => e.kind === 'pool')
      .reduce((sum, e) => sum + (e.delta ?? 0), 0),
    difficulty: allEffects
      .filter((e) => e.kind === 'difficulty')
      .reduce((sum, e) => sum + (e.delta ?? 0), 0),
    notes: allEffects
      .filter((e) => e.kind === 'note' && e.note != null)
      .map((e) => e.note!) as string[],
  };
}

/** Input record for dispatchRoll — matches the popover's local state shape. */
export interface DispatchRollArgs {
  char: BridgeCharacter;
  providers: readonly PathProvider[];
  /** Map of provider.id -> chosen option.key. e.g. { attribute: 'strength', skill: 'brawl' }. */
  selections: Record<string, string>;
  /** GM-typed base difficulty (before modifier difficulty sum). */
  baseDifficulty: number;
  /** Pre-computed sums (caller passes summarizeModifiers result). */
  modifierSums: ModifierSums;
  /** Empty string = derive label from selected option labels ("Strength + Brawl"). */
  customFlavor: string;
  rollMode: 'roll' | 'gmroll' | 'blindroll' | 'selfroll';
}

/**
 * Validate args, build the wire payload, fire-and-forget the IPC.
 * Throws synchronously on invariant violations (callee uses these errors
 * to drive an error-toast display).
 */
export async function dispatchRoll(args: DispatchRollArgs): Promise<void> {
  if (args.char.source !== 'foundry') {
    throw new Error(
      'gm-screen/roll: dispatchRoll requires a Foundry character',
    );
  }

  const valuePaths: string[] = [];
  const labelParts: string[] = [];

  for (const provider of args.providers) {
    const optionKey = args.selections[provider.id];
    if (!optionKey) {
      if (provider.required) {
        throw new Error(
          `gm-screen/roll: required provider '${provider.id}' has no selection`,
        );
      }
      continue;
    }
    const opt = provider
      .getOptions(args.char)
      .find((o) => o.key === optionKey);
    if (!opt) {
      throw new Error(
        `gm-screen/roll: provider '${provider.id}' option '${optionKey}' not found`,
      );
    }
    valuePaths.push(opt.path);
    labelParts.push(opt.label);
  }

  const flavor =
    args.customFlavor.trim() || labelParts.join(' + ') || 'Roll';
  const finalDifficulty = Math.max(
    0,
    args.baseDifficulty + args.modifierSums.difficulty,
  );

  await triggerFoundryRoll({
    actorId: args.char.source_id,
    valuePaths,
    difficulty: finalDifficulty,
    flavor,
    rollMode: args.rollMode,
    poolModifier: args.modifierSums.pool,
    advancedDice: null, // null = WoD5e auto-derive (hunger / rage / 0)
    selectors: [], // empty — JS executor's pool_modifier branch bypasses selector-based bonuses
  });
}
