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

export interface FoundryRaw {
  id: string;
  name: string;
  owner: string | null;
  system: Record<string, unknown>;
  items: FoundryItem[];
  effects: FoundryActiveEffect[];
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
