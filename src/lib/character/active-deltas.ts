// computeActiveDeltas — projects active stat-kind modifiers onto a character.
//
// Pure function. No IPC, no side effects. Mirrors the path-projection
// vocabulary used by src/lib/saved-characters/diff.ts (canonical attribute /
// skill paths like 'attributes.charisma' or 'skills.brawl').
//
// Render-time consumer: src/lib/components/CharacterCard.svelte uses the
// returned map to annotate View 2 entries with strikethrough baseline + red
// modified value + delta badge.
//
// See: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6.3.

import type {
  BridgeCharacter,
  CharacterModifier,
  SourceKind,
} from '../../types';
import {
  advantageFeatures,
  liveBonusesFor,
  ownPushPrefixes,
} from '$lib/foundry/raw';

/** Per-path projection result. */
export interface DeltaEntry {
  /** Canonical path, e.g. 'attributes.charisma'. */
  path: string;
  /** Value read from the character at this path. 0 for non-existent paths. */
  baseline: number;
  /** Sum of all active stat-kind effects targeting this path. Never zero — zero entries are omitted from the returned map. */
  delta: number;
  /** baseline + delta. */
  modified: number;
  /** Modifier names contributing to this delta, for hover-tooltip display.
   * `modifierId` is 0 for virtual contributions (un-materialized Foundry
   * advantage bonuses read live from the actor sheet). */
  sources: { modifierId: number; modifierName: string; scope: string | null }[];
}

/**
 * Read the integer value at a canonical path on a character.
 * Returns 0 for non-existent paths or non-numeric values.
 *
 * Walks `path` (dot-separated) against `raw.system`. The leaf may be:
 *   - a number directly         (e.g. `health.max`     → system.health.max)
 *   - an object with `.value`   (e.g. `attributes.charisma` → system.attributes.charisma.value)
 *
 * Verified 2026-05-10 against foundry-vtm5e-actor-sample.json: disciplines schema is
 * `system.disciplines.<name>` = { value: number, powers: [], visible, ... } — object-with-.value branch.
 *
 * Path coverage matrix (Foundry sources only — Roll20 returns 0):
 *
 *   | Canonical path           | Foundry path                          | Leaf shape       |
 *   |--------------------------|---------------------------------------|------------------|
 *   | attributes.<name>        | system.attributes.<name>              | { value: number }|
 *   | skills.<name>            | system.skills.<name>                  | { value: number }|
 *   | hunger                   | system.hunger.value                   | object → .value  |
 *   | humanity                 | system.humanity.value                 | object → .value  |
 *   | health.max               | system.health.max                     | number directly  |
 *   | health.superficial       | system.health.superficial             | number directly  |
 *   | health.aggravated        | system.health.aggravated              | number directly  |
 *   | willpower.max            | system.willpower.max                  | number directly  |
 *   | willpower.superficial    | system.willpower.superficial          | number directly  |
 *   | willpower.aggravated     | system.willpower.aggravated           | number directly  |
 *   | humanity.stains          | system.humanity.stains                | number directly  |
 *   | blood.potency            | system.blood.potency                  | number directly  |
 *   | disciplines.<name>       | system.disciplines.<name>             | object → .value  |
 *
 * Roll20 sources: returns 0 (Roll20's `raw` is a flat attributes[] array,
 * not the `system.*` tree this resolver walks).
 *
 * See: docs/superpowers/specs/2026-05-10-card-modifier-coverage-finish-design.md §3.
 */
function readPath(char: BridgeCharacter, path: string): number {
  if (char.source !== 'foundry') return 0;
  const raw = char.raw as { system?: unknown } | null;
  if (!raw?.system) return 0;

  let cur: unknown = raw.system;
  for (const seg of path.split('.')) {
    if (cur === null || typeof cur !== 'object') return 0;
    cur = (cur as Record<string, unknown>)[seg];
    if (cur === undefined) return 0;
  }

  // Leaf is a number directly (e.g. health.max).
  if (typeof cur === 'number') return cur;

  // Leaf is an object with .value (e.g. attributes.charisma → { value: 3 }).
  if (cur !== null && typeof cur === 'object') {
    const v = (cur as { value?: unknown }).value;
    if (typeof v === 'number') return v;
  }

  return 0;
}

/**
 * Match a modifier to a character by `(source, source_id)` key.
 * Mirrors §3 of `2026-05-03-gm-screen-design.md` — modifiers anchor to live
 * characters via this composite key, not via FK.
 */
function modifierMatchesChar(
  m: CharacterModifier,
  source: SourceKind,
  sourceId: string,
): boolean {
  return m.source === source && m.sourceId === sourceId;
}

/**
 * Project active path-bound contributions onto a character's canonical paths.
 *
 * Two contribution sources, merged into one path-keyed map:
 *
 *   1. **Local modifiers** — active `CharacterModifier` rows for this
 *      character, projecting their stat/pool effects that have non-empty
 *      `paths`. Inactive modifiers and modifiers belonging to other
 *      characters are filtered out; the caller does NOT need to pre-filter.
 *
 *   2. **Live Foundry bonuses** — for each advantage feature on a Foundry
 *      character that has NO matching materialized `CharacterModifier`
 *      (i.e. virtual card), its always-active `system.bonuses[]` entries
 *      project too. Own-pushed bonuses (tagged `GM Screen #<id>`) are
 *      filtered out to avoid re-pulling our own writes.
 *
 *   The local-takes-priority rule: if ANY materialized modifier exists for
 *   an advantage item (regardless of `isActive`), the live read-through is
 *   suppressed for that item. Toggling the local modifier off means
 *   "I want zero contribution from this item," not "fall back to live."
 *   (Matches the saved-override semantics in
 *   `docs/superpowers/specs/2026-05-12-foundry-bonus-effect-pull-design.md` §3.2.)
 *
 * Entries with `delta === 0` after summation (opposing contributions that
 * cancel) are omitted from the returned map.
 *
 * Returns a Map keyed by path. The View 2 renderer uses `map.has(path)`
 * to decide whether to apply the annotation; the GM screen
 * `ActiveEffectsSummary` iterates the map to render the rollup.
 */
export function computeActiveDeltas(
  char: BridgeCharacter,
  modifiers: CharacterModifier[],
): Map<string, DeltaEntry> {
  const acc = new Map<string, DeltaEntry>();

  function fold(path: string, delta: number, source: { modifierId: number; modifierName: string; scope: string | null }) {
    if (delta === 0) return;
    const existing = acc.get(path);
    if (existing) {
      existing.delta += delta;
      existing.modified = existing.baseline + existing.delta;
      existing.sources.push(source);
    } else {
      const baseline = readPath(char, path);
      acc.set(path, {
        path,
        baseline,
        delta,
        modified: baseline + delta,
        sources: [source],
      });
    }
  }

  // (1) Local active modifiers.
  for (const m of modifiers) {
    if (!m.isActive) continue;
    if (!modifierMatchesChar(m, char.source, char.source_id)) continue;
    for (const e of m.effects) {
      // Project Stat effects (render-time only) AND Pool effects with paths
      // (Pool effects also fold into rolls when the V5 dice helper lands;
      // their card-projection is the common "+1 Charisma"-style merit shape).
      // Pool effects without paths are scope-based bonuses with no specific
      // stat target — they're not visualizable on the card.
      if (e.kind !== 'stat' && e.kind !== 'pool') continue;
      if (e.paths.length === 0) continue;
      const delta = e.delta ?? 0;
      if (delta === 0) continue;
      for (const path of e.paths) {
        fold(path, delta, { modifierId: m.id, modifierName: m.name, scope: e.scope });
      }
    }
  }

  // (2) Live Foundry bonuses, gated on toggle state. The materialized
  //     modifier is the per-card switch:
  //
  //       - No materialized row (virtual) → no contribution (implicit off).
  //       - Materialized + isActive=false → no contribution.
  //       - Materialized + isActive=true + has path-bound local effects →
  //         local contributes (loop 1); live suppressed here.
  //       - Materialized + isActive=true + no path-bound local effects →
  //         live bonuses contribute here.
  //
  //     Roll20 has no `system.bonuses[]` equivalent, so this branch is a
  //     no-op for Roll20 chars.
  if (char.source === 'foundry') {
    const charMods = modifiers.filter((m) => modifierMatchesChar(m, char.source, char.source_id));
    const prefixes = ownPushPrefixes(charMods.map((m) => m.id));
    const itemState = new Map<string, { active: boolean; hasPathEffects: boolean }>();
    for (const m of charMods) {
      if (m.binding.kind !== 'advantage') continue;
      const hasPathEffects = m.effects.some(
        (e) => (e.kind === 'stat' || e.kind === 'pool') && e.paths.length > 0,
      );
      itemState.set(m.binding.item_id, { active: m.isActive, hasPathEffects });
    }
    for (const item of advantageFeatures(char)) {
      const state = itemState.get(item._id);
      if (!state) continue;            // virtual — implicit off.
      if (!state.active) continue;     // toggled off.
      if (state.hasPathEffects) continue; // local supersedes; loop 1 handled it.
      for (const b of liveBonusesFor(item, prefixes)) {
        if (b.value === 0) continue;
        for (const path of b.paths) {
          fold(path, b.value, { modifierId: 0, modifierName: item.name, scope: b.source ?? null });
        }
      }
    }
  }

  // Drop entries where opposing contributions summed to zero.
  for (const [path, entry] of acc) {
    if (entry.delta === 0) acc.delete(path);
  }

  return acc;
}

/**
 * Convenience: collect the set of advantage `item_id`s belonging to active
 * modifiers on this character. View 4 chip-rendering uses this to flip a
 * chip's data-active attribute when its `_id` matches.
 */
export function activeAdvantageItemIds(
  char: BridgeCharacter,
  modifiers: CharacterModifier[],
): Set<string> {
  const out = new Set<string>();
  for (const m of modifiers) {
    if (!m.isActive) continue;
    if (!modifierMatchesChar(m, char.source, char.source_id)) continue;
    if (m.binding.kind === 'advantage') out.add(m.binding.item_id);
  }
  return out;
}
