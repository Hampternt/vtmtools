<script lang="ts">
  interface Props {
    value: number;
    onChange: (v: number) => void;
    min?: number;
    max?: number;
    ariaLabel?: string;
  }

  let { value, onChange, min = 0, max = 100, ariaLabel }: Props = $props();
  let numpadOpen = $state(false);
  let numpadDraft = $state('');
  let inputEl: HTMLInputElement;
  let sliderEl: HTMLDivElement;

  function setValue(v: number) {
    const clamped = Math.max(min, Math.min(max, Math.round(v)));
    onChange(clamped);
  }

  function onTyped(e: Event) {
    const raw = (e.target as HTMLInputElement).value;
    const n = Number(raw);
    if (!Number.isNaN(n)) setValue(n);
  }

  function onWheel(e: WheelEvent) {
    if (document.activeElement !== inputEl) return;
    e.preventDefault();
    const delta = e.deltaY < 0 ? 1 : -1;
    setValue(value + delta);
  }

  function onArrow(e: KeyboardEvent) {
    if (e.key === 'ArrowUp')   { e.preventDefault(); setValue(value + 1); }
    if (e.key === 'ArrowDown') { e.preventDefault(); setValue(value - 1); }
  }

  function onSliderKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
      setValue(value - 1);
      e.preventDefault();
    } else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
      setValue(value + 1);
      e.preventDefault();
    } else if (e.key === 'PageDown') {
      setValue(value - 10);
      e.preventDefault();
    } else if (e.key === 'PageUp') {
      setValue(value + 10);
      e.preventDefault();
    } else if (e.key === 'Home') {
      setValue(min);
      e.preventDefault();
    } else if (e.key === 'End') {
      setValue(max);
      e.preventDefault();
    } else if (/^[0-9]$/.test(e.key)) {
      const n = Number(e.key);
      if (n >= min && n <= max) {
        setValue(n);
        e.preventDefault();
      }
    }
  }

  function onSliderWheel(e: WheelEvent) {
    if (document.activeElement !== e.currentTarget) return;
    if (e.deltaY < 0) setValue(value + 1);
    else setValue(value - 1);
    e.preventDefault();
  }

  function commitNumpad() {
    const n = Number(numpadDraft);
    if (!Number.isNaN(n)) setValue(n);
    numpadOpen = false;
    numpadDraft = '';
  }

  function numpadPress(ch: string) {
    if (ch === '⌫') numpadDraft = numpadDraft.slice(0, -1);
    else            numpadDraft = (numpadDraft + ch).slice(0, 3);
  }
</script>

<div class="value-selector">
  <input
    bind:this={inputEl}
    class="value-input"
    type="text"
    value={String(value)}
    oninput={onTyped}
    onwheel={onWheel}
    onkeydown={onArrow}
  />
  <div
    bind:this={sliderEl}
    class="dot-row"
    role="slider"
    aria-valuemin={min}
    aria-valuemax={max}
    aria-valuenow={value}
    aria-label={ariaLabel ?? 'Value'}
    tabindex="0"
    onkeydown={onSliderKeydown}
    onwheel={onSliderWheel}
  >
    {#each [1,2,3,4,5,6,7,8,9,10] as n}
      <button
        type="button"
        class="blood-dot dot-{n}"
        class:filled={value >= n}
        onclick={() => setValue(n)}
        aria-label="Set value to {n}"
        tabindex="-1"
      ></button>
    {/each}
    <button
      type="button"
      class="custom-trigger"
      onclick={() => { numpadOpen = !numpadOpen; numpadDraft = String(value); }}
      aria-label="Open custom value numpad"
      tabindex="-1"
    >…</button>
  </div>

  {#if numpadOpen}
    <div class="numpad">
      <div class="numpad-display">
        <span>{numpadDraft || 0}</span>
      </div>
      <div class="numpad-grid">
        {#each ['1','2','3','4','5','6','7','8','9','0','⌫'] as ch}
          <button type="button" onclick={() => numpadPress(ch)}>{ch}</button>
        {/each}
        <button type="button" class="enter" onclick={commitNumpad}>Use {numpadDraft || 0}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .value-selector { display: flex; flex-direction: column; gap: 0.5rem; position: relative; }
  .value-input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.4rem 0.6rem;
    font-size: 1.05rem;
    color: var(--text-primary);
    font-variant-numeric: tabular-nums;
    width: 3.2rem; text-align: center;
    font-weight: 600;
    box-sizing: border-box;
    outline: none;
  }
  .value-input:focus { border-color: var(--accent); }
  .dot-row {
    display: flex; gap: 0.3rem; align-items: center; flex-wrap: wrap;
    padding: 0.15rem 0.25rem;
    border-radius: 0.3rem;
    outline: none;
  }
  .dot-row:focus-visible { box-shadow: 0 0 0 2px var(--accent); }
  .blood-dot {
    width: 1.55rem; height: 1.55rem; border-radius: 50%;
    border: 1px solid var(--border-surface);
    background: transparent;
    cursor: pointer; padding: 0;
    transition: transform 0.12s, border-color 0.12s, box-shadow 0.12s;
  }
  .blood-dot:hover { transform: scale(1.1); border-color: var(--accent); }
  .blood-dot.dot-1.filled  { background: var(--blood-1); border-color: var(--blood-2); }
  .blood-dot.dot-2.filled  { background: var(--blood-2); border-color: var(--blood-3); }
  .blood-dot.dot-3.filled  { background: var(--blood-3); border-color: var(--blood-4); }
  .blood-dot.dot-4.filled  { background: var(--blood-4); border-color: var(--blood-5); }
  .blood-dot.dot-5.filled  { background: var(--blood-5); border-color: var(--blood-6); }
  .blood-dot.dot-6.filled  { background: var(--blood-6); border-color: var(--blood-7); box-shadow: 0 0 3px rgba(132, 48, 36, 0.4); }
  .blood-dot.dot-7.filled  { background: var(--blood-7); border-color: var(--blood-8); box-shadow: 0 0 4px rgba(156, 56, 40, 0.55); }
  .blood-dot.dot-8.filled  { background: var(--blood-8); border-color: var(--blood-9); box-shadow: 0 0 6px rgba(176, 66, 48, 0.7); }
  .blood-dot.dot-9.filled  { background: var(--blood-9); border-color: var(--blood-10); box-shadow: 0 0 8px rgba(200, 76, 52, 0.85); }
  .blood-dot.dot-10.filled { background: var(--blood-10); border-color: #f47050; box-shadow: 0 0 12px rgba(224, 86, 58, 1), 0 0 24px rgba(224, 86, 58, 0.4); }
  .custom-trigger {
    background: var(--bg-input); border: 1px dashed var(--border-surface);
    color: var(--text-muted);
    width: 1.85rem; height: 1.55rem; padding: 0;
    border-radius: 0.7rem;
    font-size: 0.7rem; cursor: pointer; font-style: italic;
  }
  .custom-trigger:hover { color: var(--accent-amber); border-color: var(--accent-amber); }
  .numpad {
    position: absolute; top: 100%; left: 0; z-index: 10;
    margin-top: 0.4rem;
    background: var(--bg-raised);
    border: 1px solid var(--accent-amber);
    border-radius: 0.4rem;
    padding: 0.55rem;
    box-shadow: 0 0.5rem 1.5rem -0.2rem rgba(0,0,0,0.55);
    display: inline-flex; flex-direction: column; gap: 0.35rem;
  }
  .numpad-display {
    padding: 0.25rem 0.4rem;
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 0.25rem;
    min-width: 6rem; text-align: right;
    color: var(--text-primary);
    font-size: 1.1rem; font-variant-numeric: tabular-nums; font-weight: 600;
  }
  .numpad-grid {
    display: grid;
    grid-template-columns: repeat(3, 1.95rem);
    gap: 0.3rem;
  }
  .numpad-grid button {
    background: var(--bg-card); border: 1px solid var(--border-faint);
    color: var(--text-secondary); border-radius: 0.25rem;
    width: 1.95rem; height: 1.95rem; font-size: 0.85rem; cursor: pointer;
    padding: 0; font-variant-numeric: tabular-nums; font-weight: 600;
  }
  .numpad-grid button:hover { border-color: var(--accent); color: var(--accent-bright); }
  .numpad-grid .enter {
    grid-column: 1 / span 3;
    background: var(--accent); color: #fff; border-color: var(--accent-bright);
    font-size: 0.72rem; height: 1.7rem;
  }
</style>
