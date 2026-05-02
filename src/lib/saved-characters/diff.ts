// Pure-TS diff between a saved character snapshot and the live bridge view.
// Path-based projection over canonical fields + Foundry skill/attribute paths,
// plus a list-based comparator for specialty Items. Result is a flat
// DiffEntry[] suitable for table rendering.

import type { BridgeCharacter } from '$lib/bridge/api';

/** A single difference between saved and live. Identity by `key`. */
export interface DiffEntry {
  key: string;
  label: string;
  before: string;
  after: string;
}

/** A diffable path: how to read it, what to call it, stable identity. */
interface DiffablePath {
  key: string;
  label: string;
  read: (c: BridgeCharacter) => string | number | null;
}

function cap(s: string): string {
  return s.replace(/_/g, ' ').replace(/\b\w/g, ch => ch.toUpperCase());
}

/** Canonical fields — apply to all sources (Roll20 + Foundry).
 *  BridgeCharacter is snake_case (Rust CanonicalCharacter has no
 *  rename_all = "camelCase"), so the field accesses below mirror that. */
const CANONICAL_PATHS: DiffablePath[] = [
  { key: 'name',                  label: 'Name',                    read: c => c.name },
  { key: 'hunger',                label: 'Hunger',                  read: c => c.hunger ?? null },
  { key: 'humanity',              label: 'Humanity',                read: c => c.humanity ?? null },
  { key: 'humanity.stains',       label: 'Stains',                  read: c => c.humanity_stains ?? null },
  { key: 'health.max',            label: 'Health (max)',            read: c => c.health?.max ?? null },
  { key: 'health.superficial',    label: 'Health (superficial)',    read: c => c.health?.superficial ?? null },
  { key: 'health.aggravated',     label: 'Health (aggravated)',     read: c => c.health?.aggravated ?? null },
  { key: 'willpower.max',         label: 'Willpower (max)',         read: c => c.willpower?.max ?? null },
  { key: 'willpower.superficial', label: 'Willpower (superficial)', read: c => c.willpower?.superficial ?? null },
  { key: 'bloodPotency',          label: 'Blood Potency',           read: c => c.blood_potency ?? null },
];

// Foundry WoD5e skill keys — extracted from the live actor sample at
// docs/reference/foundry-vtm5e-actor-sample.json. 27 entries; note
// `animalken` is one word (no underscore) in the actual schema.
const FOUNDRY_SKILL_KEYS = [
  'academics', 'animalken', 'athletics', 'awareness', 'brawl', 'craft',
  'drive', 'etiquette', 'finance', 'firearms', 'insight', 'intimidation',
  'investigation', 'larceny', 'leadership', 'medicine', 'melee', 'occult',
  'performance', 'persuasion', 'politics', 'science', 'stealth',
  'streetwise', 'subterfuge', 'survival', 'technology',
];

// Foundry WoD5e attribute keys — full names, 9 entries.
const FOUNDRY_ATTR_KEYS = [
  'strength', 'dexterity', 'stamina',
  'charisma', 'manipulation', 'composure',
  'intelligence', 'wits', 'resolve',
];

const FOUNDRY_PATHS: DiffablePath[] = [
  ...FOUNDRY_SKILL_KEYS.map(k => ({
    key:   `skills.${k}`,
    label: `${cap(k)} (skill)`,
    read:  (c: BridgeCharacter) =>
      c.source === 'foundry'
        ? ((c.raw as any)?.system?.skills?.[k]?.value ?? null)
        : null,
  })),
  ...FOUNDRY_ATTR_KEYS.map(k => ({
    key:   `attrs.${k}`,
    label: `${cap(k)} (attribute)`,
    read:  (c: BridgeCharacter) =>
      c.source === 'foundry'
        ? ((c.raw as any)?.system?.attributes?.[k]?.value ?? null)
        : null,
  })),
];

/** The consolidated path list used by diffCharacter. */
export const DIFFABLE_PATHS: DiffablePath[] = [...CANONICAL_PATHS, ...FOUNDRY_PATHS];

/** Build a map from skill key → list of specialty names on that skill. */
function collectSpecialties(raw: unknown): Record<string, string[]> {
  const out: Record<string, string[]> = {};
  const items = (raw as any)?.items;
  if (!Array.isArray(items)) return out;
  for (const item of items) {
    if (item?.type !== 'speciality') continue;
    const skill = item?.system?.skill;
    if (typeof skill !== 'string' || !skill) continue;
    if (!out[skill]) out[skill] = [];
    out[skill].push(String(item?.name ?? ''));
  }
  return out;
}

/**
 * List comparator for specialty Items. Roll20 saves skip this entirely.
 * Returns one DiffEntry per skill where the set of specialty names changed,
 * with comma-joined sorted names so order doesn't produce false positives.
 */
export function diffSpecialties(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  if (saved.source !== 'foundry') return [];
  const savedMap = collectSpecialties(saved.raw);
  const liveMap  = collectSpecialties(live.raw);
  const skills = new Set([...Object.keys(savedMap), ...Object.keys(liveMap)]);
  const entries: DiffEntry[] = [];
  for (const skill of skills) {
    const before = (savedMap[skill] ?? []).slice().sort().join(', ') || '—';
    const after  = (liveMap[skill]  ?? []).slice().sort().join(', ') || '—';
    if (before !== after) {
      entries.push({
        key:   `specialty.${skill}`,
        label: `Specialty: ${cap(skill)}`,
        before,
        after,
      });
    }
  }
  return entries;
}

/** Build a map of (featuretype → Map<name, points>) from raw.items[] feature documents. */
function collectAdvantages(raw: unknown): Record<string, Map<string, number>> {
  const out: Record<string, Map<string, number>> = {
    merit: new Map(), flaw: new Map(), background: new Map(), boon: new Map(),
  };
  const items = (raw as { items?: unknown[] } | null)?.items ?? [];
  for (const item of items as Array<Record<string, unknown>>) {
    if (item.type !== 'feature') continue;
    const sys = item.system as Record<string, unknown> | undefined;
    const ft = sys?.featuretype as string | undefined;
    const name = item.name as string | undefined;
    if (!ft || !(ft in out) || !name) continue;
    const points = typeof sys?.points === 'number' ? sys.points : 0;
    out[ft].set(name, points);
  }
  return out;
}

/**
 * List comparator for advantage Items (merits/flaws/backgrounds/boons).
 * Roll20 saves skip entirely (advantages live in repeating-section attrs,
 * not feature documents). Matching key is `name` within `featuretype`,
 * matching diffSpecialties' keying.
 */
export function diffAdvantages(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  if (saved.source !== 'foundry') return [];
  const savedMap = collectAdvantages(saved.raw);
  const liveMap  = collectAdvantages(live.raw);
  const entries: DiffEntry[] = [];
  for (const ft of ['merit', 'flaw', 'background', 'boon'] as const) {
    const sv = savedMap[ft];
    const lv = liveMap[ft];
    const allNames = new Set([...sv.keys(), ...lv.keys()]);
    for (const name of allNames) {
      const before = sv.get(name);
      const after  = lv.get(name);
      const label  = `${cap(ft)}: ${name}`;
      const key    = `${ft}.${name}`;
      if (before === undefined && after !== undefined) {
        entries.push({ key, label, before: '—', after: after > 0 ? `+ (${after})` : 'added' });
      } else if (after === undefined && before !== undefined) {
        entries.push({ key, label, before: before > 0 ? `(${before})` : 'present', after: '—' });
      } else if (before !== after) {
        entries.push({ key, label, before: String(before), after: String(after) });
      }
    }
  }
  return entries;
}

/**
 * Diff a saved character against a live one. Returns the changed entries
 * across canonical fields, Foundry skills/attributes, and specialties.
 *
 * Pure function. Caller is responsible for ensuring the two inputs refer
 * to the same character (typically by matching (source, source_id)).
 */
export function diffCharacter(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  const pathDiffs: DiffEntry[] = DIFFABLE_PATHS
    .map(p => ({ key: p.key, label: p.label, before: p.read(saved), after: p.read(live) }))
    .filter(({ before, after }) => before !== after)
    .map(({ key, label, before, after }) => ({
      key,
      label,
      before: before == null ? '—' : String(before),
      after:  after  == null ? '—' : String(after),
    }));
  return [...pathDiffs, ...diffSpecialties(saved, live), ...diffAdvantages(saved, live)];
}
