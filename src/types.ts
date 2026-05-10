export interface TemperamentConfig {
  diceCount: number;
  takeHighest: boolean;
  negligibleMax: number;
  fleetingMax: number;
}

export interface ResonanceWeights {
  phlegmatic: string;
  melancholy: string;
  choleric: string;
  sanguine: string;
}

export interface RollConfig {
  temperament: TemperamentConfig;
  weights: ResonanceWeights;
}

export interface DyscrasiaEntry {
  id: number;
  resonanceType: string;
  name: string;
  description: string;
  bonus: string;
  isCustom: boolean;
}

export interface Advantage {
  id: number;
  name: string;
  description: string;
  tags: string[];
  properties: Field[];
  isCustom: boolean;
}

export interface ResonanceRollResult {
  temperamentDice: number[];
  temperamentDie: number;
  temperament: 'negligible' | 'fleeting' | 'intense';
  resonanceType: string | null;
  resonanceDie: number | null;
  acuteDie: number | null;
  isAcute: boolean;
  dyscrasia: DyscrasiaEntry | null;
}

export interface HistoryEntry {
  id: number;
  timestamp: Date;
  result: ResonanceRollResult;
}

// ---------------------------------------------------------------------------
// Bridge layer: source-agnostic character mirror for Roll20 / Foundry / etc.
// Mirrors src-tauri/src/bridge/types.rs.
// ---------------------------------------------------------------------------

export type SourceKind = 'roll20' | 'foundry';

/** Mirrors src-tauri/src/tools/character.rs::WriteTarget. */
export type WriteTarget = 'live' | 'saved' | 'both';

import type { AttributeName, SkillName } from './lib/foundry/canonical-names';
export type { AttributeName, SkillName };

/**
 * Mirrors the v2 canonical-name surface in
 * src-tauri/src/shared/canonical_fields.rs::is_allowed_name:
 *   - Legacy 8 flat names (FLAT_NAMES).
 *   - `attribute.<key>` for every key in ATTRIBUTE_NAMES.
 *   - `skill.<key>` for every key in SKILL_NAMES.
 *
 * Adding a name = update BOTH the Rust ATTRIBUTE_NAMES/SKILL_NAMES arrays
 * AND src/lib/foundry/canonical-names.ts in the same commit (manual-checklist
 * convention; matches BridgeCharacter mirror precedent).
 */
export type CanonicalFieldName =
  | 'hunger'
  | 'humanity'
  | 'humanity_stains'
  | 'blood_potency'
  | 'health_superficial'
  | 'health_aggravated'
  | 'willpower_superficial'
  | 'willpower_aggravated'
  | `attribute.${AttributeName}`
  | `skill.${SkillName}`;

/** Mirrors src-tauri/src/tools/character.rs::FeatureType. */
export type FeatureType = 'merit' | 'flaw' | 'background' | 'boon';

export interface HealthTrack {
  max: number;
  superficial: number;
  aggravated: number;
}

export interface BridgeCharacter {
  source: SourceKind;
  source_id: string;
  name: string;
  controlled_by: string | null;
  hunger: number | null;
  health: HealthTrack | null;
  willpower: HealthTrack | null;
  humanity: number | null;
  humanity_stains: number | null;
  blood_potency: number | null;
  /// Source-specific extras the canonical fields don't capture. For
  /// Roll20 sources this is the original Roll20 character (id, name,
  /// controlled_by, attributes: [{name, current, max}, ...]) so legacy
  /// helpers like parseDisciplines still work against it.
  raw: unknown;
}

export interface Roll20RawAttribute {
  name: string;
  current: string;
  max: string;
}

export interface Roll20Raw {
  id: string;
  name: string;
  controlled_by: string;
  attributes: Roll20RawAttribute[];
}

// ---------------------------------------------------------------------------
// Foundry source-specific raw shapes (when BridgeCharacter.source === 'foundry').
// Mirrors the wire shape produced by vtmtools-bridge/scripts/translate.js
// and FoundryActor in src-tauri/src/bridge/foundry/types.rs.
// ---------------------------------------------------------------------------

export interface FoundryActiveEffectChange {
  key: string;
  mode: number;        // 0=custom, 1=multiply, 2=add, 3=downgrade, 4=upgrade, 5=override
  value: string;
  priority?: number | null;
}

export interface FoundryActiveEffect {
  _id: string;
  name: string;
  disabled: boolean;
  transfer?: boolean;  // when true on an item-attached effect, copies to the parent actor
  origin?: string | null;
  changes: FoundryActiveEffectChange[];
  // Foundry includes more (duration, statuses, img, etc.) — left as
  // index-signature for extension; consumers cast as needed.
  [k: string]: unknown;
}

export interface FoundryItem {
  _id: string;
  type: string;        // "feature" | "weapon" | "discipline" | "resonance" | "speciality" | ...
  name: string;
  system: Record<string, unknown>;     // type-specific schema; for features: { featuretype, points, description, bonuses?, ... }
  effects: FoundryActiveEffect[];
  [k: string]: unknown;
}

/**
 * Sheet-attached bonus on a Foundry feature item (system.bonuses[]). Unlike
 * ModifierEffect (a GM Screen annotation), bonuses come from the actor sheet
 * itself — the WoD5e system writes them when the player ticks "this merit
 * adds +1 to Strength" on a feature.
 */
export interface FoundryItemBonus {
  /** Display label the player typed (e.g. "Buff Modifier"). */
  source: string;
  /** Numeric modifier applied to each path. Negative values are penalties. */
  value: number;
  /** Stat dot-paths the bonus applies to (e.g. "attributes.strength", "skills.subterfuge"). */
  paths: string[];
  /** Active-state predicate. `check: 'always'` means unconditional. */
  activeWhen?: { check: string; path?: string; value?: string };
  /** When false, hide the bonus from the sheet UI when its activeWhen evaluates false. */
  displayWhenInactive?: boolean;
  /** Optional dot-path predicate that disables the bonus when truthy. */
  unless?: string;
}

export interface FoundryRaw {
  id: string;
  name: string;
  owner: string | null;
  system: Record<string, unknown>;
  items: FoundryItem[];
  effects: FoundryActiveEffect[];
}

// ---------------------------------------------------------------------------
// GM Screen — character modifiers (mirrors src-tauri/src/shared/modifier.rs).
// CharacterModifier serializes camelCase via Rust serde rename; the binding
// discriminator is `kind` and uses snake_case variants ('free' / 'advantage').
// ---------------------------------------------------------------------------

export type ModifierKind = 'pool' | 'difficulty' | 'note' | 'stat';

export interface ModifierEffect {
  kind: ModifierKind;
  scope: string | null;
  delta: number | null;
  note: string | null;
  /**
   * Foundry-bonus dot-paths (e.g. ["attributes.strength"]). Empty array = pathless.
   * Only used by the push-to-Foundry button on `pool`-kind effects.
   */
  paths: string[];
}

export type ModifierBinding =
  | { kind: 'free' }
  | { kind: 'advantage'; item_id: string };

export interface CharacterModifier {
  id: number;
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  isActive: boolean;
  isHidden: boolean;
  originTemplateId: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface NewCharacterModifierInput {
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  originTemplateId: number | null;
}

export interface ModifierPatchInput {
  name?: string;
  description?: string;
  effects?: ModifierEffect[];
  tags?: string[];
}

export interface StatusTemplate {
  id: number;
  name: string;
  description: string;
  effects: ModifierEffect[];
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

export interface NewStatusTemplateInput {
  name: string;
  description: string;
  effects: ModifierEffect[];
  tags: string[];
}

export interface StatusTemplatePatchInput {
  name?: string;
  description?: string;
  effects?: ModifierEffect[];
  tags?: string[];
}

/** Per-effect skip record returned by `gm_screen_push_to_foundry`. */
export interface SkippedEffect {
  effectIndex: number;
  reason: string;
}

/** Result of one push-to-Foundry button press. */
export interface PushReport {
  pushed: number;
  skipped: SkippedEffect[];
}

// ---------------------------------------------------------------------------
// Domains Manager / Chronicle graph types
// ---------------------------------------------------------------------------

export interface Chronicle {
  id: number;
  name: string;
  description: string;
  created_at: string;
  updated_at: string;
}

export type FieldValue =
  | { type: 'string';    value: string | string[] }
  | { type: 'text';      value: string }
  | { type: 'number';    value: number | number[] }
  | { type: 'date';      value: string }
  | { type: 'url';       value: string }
  | { type: 'email';     value: string }
  | { type: 'bool';      value: boolean }
  | { type: 'reference'; value: number };

export type Field = { name: string } & FieldValue;

export interface ChronicleNode {
  id: number;
  chronicle_id: number;
  type: string;
  label: string;
  description: string;
  tags: string[];
  properties: Field[];
  created_at: string;
  updated_at: string;
}

export interface ChronicleEdge {
  id: number;
  chronicle_id: number;
  from_node_id: number;
  to_node_id: number;
  edge_type: string;
  description: string;
  properties: Field[];
  created_at: string;
  updated_at: string;
}

export type EdgeDirection = 'in' | 'out' | 'both';
