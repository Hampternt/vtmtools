import { describe, it, expect } from 'vitest';
import { deriveBonusName, isAutoDerived } from './bonus-name';

describe('deriveBonusName', () => {
  it('returns "[name] bonus" for a non-empty name', () => {
    expect(deriveBonusName('Sanguine Tenacity')).toBe('Sanguine Tenacity bonus');
  });

  it('returns empty string for empty name', () => {
    expect(deriveBonusName('')).toBe('');
  });

  it('trims whitespace before deriving', () => {
    expect(deriveBonusName('  Iron Will  ')).toBe('Iron Will bonus');
  });
});

describe('isAutoDerived', () => {
  it('matches when current is exactly "[merit name] bonus"', () => {
    expect(isAutoDerived('Sanguine Tenacity bonus', 'Sanguine Tenacity')).toBe(true);
  });

  it('does not match when user has edited', () => {
    expect(isAutoDerived('Custom Name', 'Sanguine Tenacity')).toBe(false);
  });

  it('matches empty bonus name + empty merit name', () => {
    expect(isAutoDerived('', '')).toBe(true);
  });
});
