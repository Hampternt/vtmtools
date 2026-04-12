<script lang="ts">
  const {
    diceCount = 1,
    takeHighest = true,
    negligibleMax = 5,
    fleetingMax = 8,
    onDiceCountChange,
    onTakeHighestChange,
    onNegligibleMaxChange,
    onFleetingMaxChange,
  }: {
    diceCount?: number;
    takeHighest?: boolean;
    negligibleMax?: number;
    fleetingMax?: number;
    onDiceCountChange: (n: number) => void;
    onTakeHighestChange: (b: boolean) => void;
    onNegligibleMaxChange: (n: number) => void;
    onFleetingMaxChange: (n: number) => void;
  } = $props();

  const diceCounts = [1, 2, 3, 4, 5];

  let localNegMax = $state(negligibleMax);
  let localFleMax = $state(fleetingMax);
</script>

<div class="temp-config">
  <div class="row">
    <label>Dice pool</label>
    <div class="dice-buttons">
      {#each diceCounts as n}
        <button
          class="die-btn {diceCount === n ? 'active' : ''}"
          onclick={() => onDiceCountChange(n)}
        >{n}d10</button>
      {/each}
    </div>
  </div>

  {#if diceCount > 1}
    <div class="row">
      <label>Take</label>
      <div class="take-buttons">
        <button class="take-btn {takeHighest ? 'active' : ''}" onclick={() => onTakeHighestChange(true)}>
          Highest (→ Intense)
        </button>
        <button class="take-btn {!takeHighest ? 'active' : ''}" onclick={() => onTakeHighestChange(false)}>
          Lowest (→ Negligible)
        </button>
      </div>
    </div>
  {/if}

  <div class="row">
    <label>Thresholds</label>
    <span class="threshold-display">
      Neg 1–{localNegMax} / Flee {localNegMax + 1}–{localFleMax} / Int {localFleMax + 1}–10
    </span>
  </div>

  <div class="row">
    <label>Negligible max</label>
    <input
      type="range" min="0" max="9"
      bind:value={localNegMax}
      oninput={() => {
        if (localNegMax >= localFleMax) localFleMax = localNegMax + 1;
        onNegligibleMaxChange(localNegMax);
        onFleetingMaxChange(localFleMax);
      }}
    />
    <span>{localNegMax}</span>
  </div>

  <div class="row">
    <label>Fleeting max</label>
    <input
      type="range"
      min={localNegMax + 1}
      max="10"
      bind:value={localFleMax}
      oninput={() => onFleetingMaxChange(localFleMax)}
    />
    <span>{localFleMax}</span>
  </div>
</div>

<style>
  .temp-config { display: flex; flex-direction: column; gap: 0.6rem; }
  .row { display: flex; align-items: center; gap: 0.75rem; }
  label { width: 110px; font-size: 0.85rem; color: #a09070; flex-shrink: 0; }
  .die-btn, .take-btn {
    padding: 0.25rem 0.6rem;
    background: #1a0d0d;
    border: 1px solid #3a1a1a;
    color: #8a8a8a;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.8rem;
    transition: background 0.15s, color 0.15s;
  }
  .die-btn.active, .take-btn.active {
    background: #3a0808;
    color: #cc2222;
    border-color: #cc2222;
  }
  .dice-buttons, .take-buttons { display: flex; gap: 4px; flex-wrap: wrap; }
  .threshold-display { font-size: 0.8rem; color: #cc2222; }
  input[type=range] { accent-color: #cc2222; width: 120px; }
  span { font-size: 0.85rem; color: #d4c5a9; min-width: 20px; }
</style>
