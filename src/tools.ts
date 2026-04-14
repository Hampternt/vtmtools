import type { Component } from 'svelte';

export interface Tool {
  id: string;
  label: string;
  icon: string; // emoji or SVG string
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  component: () => Promise<{ default: Component<any> }>;
}

// Add new tools here — the sidebar renders from this list automatically.
export const tools: Tool[] = [
  {
    id: 'resonance',
    label: 'Resonance Roller',
    icon: '🩸',
    component: () => import('./tools/Resonance.svelte'),
  },
  {
    id: 'dyscrasias',
    label: 'Dyscrasias',
    icon: '📋',
    component: () => import('./tools/DyscrasiaManager.svelte'),
  },
  {
    id: 'campaign',
    label: 'Campaign',
    icon: '🗺️',
    component: () => import('./tools/Campaign.svelte'),
  },
  // Future tools go here, e.g.:
  // { id: 'combat', label: 'Combat Tracker', icon: '⚔️', component: () => import('./tools/Combat.svelte') },
];
