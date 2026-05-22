import { describe, it, expect } from 'vitest';
import { createStageMachine, type StageMachine } from './stage-machine.svelte';

const tabOrder = ['name', 'type', 'level', 'bonus', 'description', 'tags'] as const;
const required = ['name', 'bonus'] as const;

function fresh(): StageMachine {
  return createStageMachine({ tabOrder: [...tabOrder], required: [...required] });
}

describe('stage machine', () => {
  it('starts at the first stage with no confirmed fields', () => {
    const m = fresh();
    expect(m.stage).toBe('name');
    expect(m.confirmed.size).toBe(0);
  });

  it('advance() marks current confirmed and moves to next', () => {
    const m = fresh();
    m.advance();
    expect(m.confirmed.has('name')).toBe(true);
    expect(m.stage).toBe('type');
  });

  it('retreat() does NOT unconfirm previous; just moves the cursor', () => {
    const m = fresh();
    m.advance();
    m.advance();
    m.retreat();
    expect(m.stage).toBe('type');
    expect(m.confirmed.has('name')).toBe(true);
    expect(m.confirmed.has('type')).toBe(true);
  });

  it('setStage(key) jumps to that stage and removes it from confirmed', () => {
    const m = fresh();
    m.advance(); m.advance(); m.advance();
    m.setStage('name');
    expect(m.stage).toBe('name');
    expect(m.confirmed.has('name')).toBe(false);
    expect(m.confirmed.has('type')).toBe(true);
    expect(m.confirmed.has('level')).toBe(true);
  });

  it('isComplete() returns true only when all required fields are confirmed', () => {
    const m = fresh();
    expect(m.isComplete()).toBe(false);
    m.advance();
    expect(m.isComplete()).toBe(false);
    m.advance();
    m.advance();
    m.advance();
    expect(m.isComplete()).toBe(true);
  });

  it('seedFromConfirmed() places stage at first non-confirmed in tabOrder', () => {
    const m = createStageMachine({
      tabOrder: [...tabOrder],
      required: [...required],
      initialConfirmed: ['name', 'type'],
    });
    expect(m.stage).toBe('level');
    expect(m.confirmed.has('name')).toBe(true);
    expect(m.confirmed.has('type')).toBe(true);
  });

  it('seedFromConfirmed all required → review stage', () => {
    const m = createStageMachine({
      tabOrder: [...tabOrder],
      required: [...required],
      initialConfirmed: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
    });
    expect(m.stage).toBe('review');
    expect(m.isComplete()).toBe(true);
  });

  it('advance past last tabOrder entry → review stage', () => {
    const m = fresh();
    for (let i = 0; i < tabOrder.length; i++) m.advance();
    expect(m.stage).toBe('review');
  });

  it('retreat from first stage stays at first', () => {
    const m = fresh();
    m.retreat();
    expect(m.stage).toBe('name');
  });
});
