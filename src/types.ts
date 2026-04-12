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
