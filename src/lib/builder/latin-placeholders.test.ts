import { describe, it, expect } from 'vitest';
import { LATIN_PLACEHOLDERS, pickPlaceholder } from './latin-placeholders';

describe('LATIN_PLACEHOLDERS', () => {
  it('has the 12 stub entries from the spec', () => {
    expect(LATIN_PLACEHOLDERS).toHaveLength(12);
    expect(LATIN_PLACEHOLDERS).toContain('Vinculum Sanguinis');
    expect(LATIN_PLACEHOLDERS).toContain('Praesidium Noctis');
    expect(LATIN_PLACEHOLDERS).toContain('Silentium Tumuli');
  });
});

describe('pickPlaceholder', () => {
  it('returns deterministic value for the same input count', () => {
    expect(pickPlaceholder(0)).toBe(pickPlaceholder(0));
    expect(pickPlaceholder(5)).toBe(pickPlaceholder(5));
  });

  it('cycles through the pool as count grows', () => {
    expect(pickPlaceholder(0)).not.toBe(pickPlaceholder(1));
  });

  it('wraps around past pool length', () => {
    expect(pickPlaceholder(0)).toBe(pickPlaceholder(LATIN_PLACEHOLDERS.length));
  });

  it('returns a non-empty string for negative or large counts', () => {
    expect(pickPlaceholder(-1)).toMatch(/.+/);
    expect(pickPlaceholder(99999)).toMatch(/.+/);
  });
});
