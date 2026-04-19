import type { FieldValue } from '../../types';

export type FieldPreset = {
  name: string;
  type: FieldValue['type'];
  defaultValue: string | number | boolean;
  hint: string;
};

/**
 * Quick-add chips surfaced in AdvantageForm's properties section.
 * Clicking one appends a new Field with this name/type/defaultValue.
 * A preset chip is disabled when a field with the same name already
 * exists on the row (name uniqueness is enforced in the form).
 */
export const FIELD_PRESETS: FieldPreset[] = [
  { name: 'level',     type: 'number', defaultValue: 1,  hint: 'Fixed dot cost' },
  { name: 'min_level', type: 'number', defaultValue: 1,  hint: 'Minimum dots (for ranged merits)' },
  { name: 'max_level', type: 'number', defaultValue: 5,  hint: 'Maximum dots (for ranged merits)' },
  { name: 'source',    type: 'string', defaultValue: '', hint: 'Sourcebook reference' },
  { name: 'prereq',    type: 'text',   defaultValue: '', hint: 'Prerequisite text' },
];
