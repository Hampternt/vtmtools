<script lang="ts">
  import type { Snippet } from 'svelte';

  type FieldKey = 'name' | 'type' | 'level' | 'category' | 'desc' | 'bonus' | 'tags';
  type SlotState = 'empty' | 'active' | 'confirmed';

  interface Props {
    field: FieldKey;
    state: SlotState;
    keyLabel: string;
    onClick?: () => void;
    children: Snippet;
  }

  let { field, state, keyLabel, onClick, children }: Props = $props();
</script>

<div
  class="slot {field} state-{state}"
  onclick={state === 'confirmed' && onClick ? onClick : undefined}
  role={state === 'confirmed' ? 'button' : undefined}
  tabindex={state === 'confirmed' ? 0 : undefined}
>
  <span class="slot-key">{keyLabel}</span>
  <span class="slot-val">{@render children()}</span>
</div>

<style>
  .slot {
    padding: 0.35rem 0.55rem;
    border-radius: 0.3rem;
    display: flex; align-items: center; gap: 0.4rem;
    font-size: 0.72rem; min-height: 1.8rem;
    border: 1px solid transparent;
    box-sizing: border-box;
    transition: all 0.16s;
  }
  .slot-key {
    font-size: 0.56rem; color: var(--text-muted);
    letter-spacing: 0.08em; text-transform: uppercase;
    flex-shrink: 0;
  }
  .slot-val {
    font-family: "Iowan Old Style", "Garamond", "Palatino Linotype", serif;
    font-weight: 600;
    flex: 1; min-width: 0;
  }

  /* Empty */
  .slot.state-empty {
    background: rgba(0,0,0,0.18);
    border: 1px dashed var(--border-surface);
    color: var(--text-muted);
    font-style: italic;
    opacity: 0.5;
  }
  .slot.state-empty .slot-val { color: var(--text-muted); font-weight: 400; }

  /* Active — pulsing amber */
  .slot.state-active {
    border: 1px solid var(--accent-amber);
    background: rgba(217, 165, 96, 0.06);
    animation: pulse-border 1.4s ease-in-out infinite;
  }
  .slot.state-active .slot-key { color: var(--accent-amber); }
  .slot.state-active .slot-val { color: var(--accent-amber); font-style: italic; font-weight: 500; }
  @keyframes pulse-border {
    0%, 100% { box-shadow: 0 0 0 0 rgba(217, 165, 96, 0.5); }
    50%      { box-shadow: 0 0 0 4px rgba(217, 165, 96, 0); }
  }

  /* Confirmed — per-field colors */
  .slot.state-confirmed { cursor: pointer; }
  .slot.state-confirmed:hover { transform: scale(1.01); }
  .slot.name.state-confirmed     { background: rgba(217, 165, 96, 0.13); border-color: var(--field-name);     color: var(--field-name); }
  .slot.type.state-confirmed     { background: rgba(196, 96, 80, 0.13);  border-color: var(--field-type);     color: var(--field-type); }
  .slot.level.state-confirmed    { background: rgba(192, 136, 80, 0.13); border-color: var(--field-level);    color: var(--field-level); }
  .slot.category.state-confirmed { background: rgba(136, 170, 187, 0.13); border-color: var(--field-category); color: var(--field-category); }
  .slot.desc.state-confirmed     { background: rgba(196, 183, 164, 0.1);  border-color: var(--field-desc);     color: var(--field-desc); }
  .slot.bonus.state-confirmed    { background: rgba(224, 86, 58, 0.12);  border-color: var(--field-bonus);    color: var(--field-bonus); }
  .slot.tags.state-confirmed     { background: rgba(152, 200, 159, 0.13); border-color: var(--field-tags);    color: var(--field-tags); }
</style>
