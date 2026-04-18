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

export interface Roll20Attribute {
  name: string;
  current: string;
  max: string;
}

export interface Roll20Character {
  id: string;
  name: string;
  controlled_by: string;
  attributes: Roll20Attribute[];
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
