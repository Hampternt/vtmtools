import type { BridgeCharacter, FoundryRaw, FoundryItem, FoundryItemBonus, FoundryActiveEffect } from '../../types';

/// Returns the FoundryRaw blob for a Foundry-sourced character, or null
/// for non-Foundry chars or when the raw blob is missing.
export function foundryRaw(char: BridgeCharacter): FoundryRaw | null {
  if (char.source !== 'foundry') return null;
  return (char.raw ?? null) as FoundryRaw | null;
}

/// All embedded items on a Foundry actor. Empty array for non-Foundry chars
/// or when items is absent (legacy module payload).
export function foundryItems(char: BridgeCharacter): FoundryItem[] {
  const raw = foundryRaw(char);
  if (!raw) return [];
  return Array.isArray(raw.items) ? raw.items : [];
}

/// Items filtered by Foundry document type. WoD5e item types include
/// "feature" (merits/flaws/backgrounds/boons), "weapon", "discipline",
/// "resonance", "speciality", "ritual", "ceremony", "formula".
export function foundryItemsByType(char: BridgeCharacter, type: string): FoundryItem[] {
  return foundryItems(char).filter((i) => i.type === type);
}

/// WoD5e features (merits/flaws/backgrounds/boons). Foundry stores all four
/// as `type: "feature"` distinguished by `system.featuretype`. Pass the
/// featuretype string to filter to one subset; pass undefined to get all.
export function foundryFeatures(
  char: BridgeCharacter,
  featuretype?: 'merit' | 'flaw' | 'background' | 'boon',
): FoundryItem[] {
  const features = foundryItemsByType(char, 'feature');
  if (!featuretype) return features;
  return features.filter((i) => i.system?.featuretype === featuretype);
}

/// Actor-level ActiveEffects (modifiers attached directly to the actor,
/// not via an embedded item). For item-attached effects, use
/// foundryItemEffects(item).
export function foundryEffects(char: BridgeCharacter): FoundryActiveEffect[] {
  const raw = foundryRaw(char);
  if (!raw) return [];
  return Array.isArray(raw.effects) ? raw.effects : [];
}

/// ActiveEffects embedded inside one item document. Each effect carries
/// `transfer: true` if it propagates to the parent actor.
export function foundryItemEffects(item: FoundryItem): FoundryActiveEffect[] {
  return Array.isArray(item.effects) ? item.effects : [];
}

/// Foundry attribute value (e.g. system.attributes.strength.value). Returns
/// 0 for non-Foundry chars or when the path is absent.
export function foundryAttrInt(char: BridgeCharacter, attrName: string): number {
  const raw = foundryRaw(char);
  if (!raw) return 0;
  const attrs = raw.system?.attributes as Record<string, { value?: number }> | undefined;
  return attrs?.[attrName]?.value ?? 0;
}

/// Foundry skill value (e.g. system.skills.brawl.value). Returns 0 for
/// non-Foundry chars or when the path is absent.
export function foundrySkillInt(char: BridgeCharacter, skillName: string): number {
  const raw = foundryRaw(char);
  if (!raw) return 0;
  const skills = raw.system?.skills as Record<string, { value?: number }> | undefined;
  return skills?.[skillName]?.value ?? 0;
}

/// Helper for "is this effect currently active?" — disabled effects exist
/// on the document but don't apply mechanically.
export function foundryEffectIsActive(effect: FoundryActiveEffect): boolean {
  return !effect.disabled;
}

/// All four advantage subtypes (merit, flaw, background, boon) collapsed
/// into one list. The GM Screen card surface and the active-effects
/// aggregator both walk this same set; defining it here keeps the filter
/// in one place.
export function advantageFeatures(char: BridgeCharacter): FoundryItem[] {
  return foundryItemsByType(char, 'feature').filter((it) => {
    const ft = (it.system as Record<string, unknown>)?.featuretype as string | undefined;
    return ft === 'merit' || ft === 'flaw' || ft === 'background' || ft === 'boon';
  });
}

/// Always-active, non-own-pushed bonuses on a Foundry feature item.
/// `ownPushPrefixes` are the per-modifier `GM Screen #<id>` tags that
/// would loop on re-pull if not filtered out. Missing `activeWhen` is
/// treated as always-active per Foundry behavior.
export function liveBonusesFor(item: FoundryItem, ownPushPrefixes: Iterable<string>): FoundryItemBonus[] {
  const raw = (item.system as Record<string, unknown>)?.bonuses;
  if (!Array.isArray(raw)) return [];
  const prefixes = [...ownPushPrefixes];
  return (raw as FoundryItemBonus[]).filter((b) => {
    const src = b.source ?? '';
    for (const prefix of prefixes) {
      if (src === prefix || src.startsWith(prefix + ':')) return false;
    }
    const check = b.activeWhen?.check ?? 'always';
    return check === 'always';
  });
}

/// The own-push tag prefix set for one character's modifiers — the
/// `GM Screen #<id>` strings used by `do_push_to_foundry` to tag bonuses
/// it wrote. Paired with `liveBonusesFor` to filter out bonuses we
/// pushed (avoids round-trip echo when we re-read them).
export function ownPushPrefixes(modifierIds: Iterable<number>): Set<string> {
  const out = new Set<string>();
  for (const id of modifierIds) out.add(`GM Screen #${id}`);
  return out;
}
