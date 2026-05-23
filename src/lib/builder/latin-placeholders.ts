export const LATIN_PLACEHOLDERS = [
  'Vinculum Sanguinis',
  'Praesidium Noctis',
  'Fons Aeternitatis',
  'Vigil Tenebris',
  'Memoria Aevi',
  'Custos Animae',
  'Tenebrarum Filia',
  'Sanguis Vetus',
  'Aetas Foederis',
  'Iter Sub Luna',
  'Cor Tenebrosum',
  'Silentium Tumuli',
] as const;

/**
 * Deterministic placeholder rotation. count = number of existing items
 * (so placeholder is stable while the user is typing — only rolls when
 * a new item lands in the library).
 */
export function pickPlaceholder(count: number): string {
  const len = LATIN_PLACEHOLDERS.length;
  const idx = ((count % len) + len) % len;
  return LATIN_PLACEHOLDERS[idx];
}

/**
 * Latin placeholder commit semantics — see spec §"Latin placeholder commit
 * semantics (name stage)". Returns the typed value when non-empty; returns
 * the placeholder when the user hasn't typed anything (placeholder behaves
 * as accepted suggestion). Whitespace-only typed value is treated as empty.
 */
export function commitPlaceholder(typed: string, placeholder: string): string {
  return typed.trim() === '' ? placeholder : typed;
}
