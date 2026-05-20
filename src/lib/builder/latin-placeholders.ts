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
