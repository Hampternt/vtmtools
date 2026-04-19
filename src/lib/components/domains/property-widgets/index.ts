import type { Component } from 'svelte';
import type { FieldValue } from '../../../../types';
import StringWidget from './StringWidget.svelte';
import TextWidget from './TextWidget.svelte';
import NumberWidget from './NumberWidget.svelte';
import BoolWidget from './BoolWidget.svelte';

// Widget for a given FieldValue discriminator. Adding a new supported type
// (date / url / email / reference) is a two-line change: import the widget,
// add an entry here.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const WIDGETS: Partial<Record<FieldValue['type'], Component<any>>> = {
  string: StringWidget,
  text:   TextWidget,
  number: NumberWidget,
  bool:   BoolWidget,
};

export const SUPPORTED_TYPES = Object.keys(WIDGETS) as Array<FieldValue['type']>;
