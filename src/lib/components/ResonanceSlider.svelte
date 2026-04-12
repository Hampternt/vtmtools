<script lang="ts">
  const { label, value = 'neutral', onChange }: {
    label: string;
    value?: string;
    onChange: (v: string) => void;
  } = $props();

  const levels = [
    { id: 'impossible',        display: 'Impossible' },
    { id: 'extremelyUnlikely', display: 'Ext. Unlikely' },
    { id: 'unlikely',          display: 'Unlikely' },
    { id: 'neutral',           display: 'Neutral' },
    { id: 'likely',            display: 'Likely' },
    { id: 'extremelyLikely',   display: 'Ext. Likely' },
    { id: 'guaranteed',        display: 'Guaranteed' },
  ];

  const selectedIndex = $derived(levels.findIndex(l => l.id === value));
</script>

<div class="slider-row">
  <span class="slider-label">{label}</span>
  <div class="steps">
    {#each levels as level, i}
      <button
        class="step {i <= selectedIndex ? 'filled' : ''} {i === selectedIndex ? 'active' : ''}"
        onclick={() => onChange(level.id)}
        title={level.display}
        aria-label="{label}: {level.display}"
      ></button>
    {/each}
  </div>
  <span class="slider-value">{levels[selectedIndex]?.display ?? ''}</span>
</div>

<style>
  .slider-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .slider-label {
    width: 90px;
    font-size: 0.85rem;
    color: #a09070;
  }
  .steps {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .step {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid #3a1a1a;
    background: #1a0d0d;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    padding: 0;
  }
  .step.filled {
    background: #6b1010;
    border-color: #8a1515;
  }
  .step.active {
    background: #cc2222;
    border-color: #ee3333;
    box-shadow: 0 0 6px #cc222266;
  }
  .slider-value {
    font-size: 0.8rem;
    color: #cc2222;
    width: 90px;
  }
</style>
