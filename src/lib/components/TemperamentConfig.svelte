<script lang="ts">
  import { untrack } from 'svelte';

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

  // untrack: intentional one-time snapshot of the initial prop value
  let localNegMax = $state(untrack(() => negligibleMax));
  let localFleMax = $state(untrack(() => fleetingMax));

  const scale = $derived(
    Array.from({ length: 10 }, (_, i) => {
      const n = i + 1;
      const zone = n <= localNegMax ? 'neg' : n <= localFleMax ? 'fle' : 'int';
      return { n, zone };
    })
  );

  const probs = $derived(calcProbs(diceCount, takeHighest, localNegMax, localFleMax));

  function calcProbs(count: number, high: boolean, negMax: number, fleMax: number) {
    const cdf = (k: number): number => {
      if (count === 1) return k / 10;
      return high
        ? Math.pow(k / 10, count)
        : 1 - Math.pow((10 - k) / 10, count);
    };
    const neg = Math.round(cdf(negMax) * 100);
    const fle = Math.round((cdf(fleMax) - cdf(negMax)) * 100);
    return { neg, fle, int: 100 - neg - fle };
  }
</script>

<div class="tc">
  <!-- Dice pool -->
  <div class="section">
    <div class="sec-label">Dice pool</div>
    <div class="dice-row">
      {#each [1, 2, 3, 4, 5] as n}
        <button
          class="die {diceCount === n ? 'die-on' : ''}"
          onclick={() => onDiceCountChange(n)}
        >
          <span class="die-num">{n}</span>
          <span class="die-tag">d10</span>
        </button>
      {/each}
    </div>
  </div>

  <!-- Advantage / disadvantage -->
  {#if diceCount > 1}
    <div class="section">
      <div class="sec-label">Which die counts</div>
      <div class="adv-row">
        <button
          class="adv {takeHighest ? 'adv-on' : ''}"
          onclick={() => onTakeHighestChange(true)}
        >
          <span class="adv-arrow">↑</span>
          <span class="adv-name">Best</span>
          <span class="adv-sub">favors Intense</span>
        </button>
        <button
          class="adv {!takeHighest ? 'adv-on' : ''}"
          onclick={() => onTakeHighestChange(false)}
        >
          <span class="adv-arrow">↓</span>
          <span class="adv-name">Worst</span>
          <span class="adv-sub">favors Negligible</span>
        </button>
      </div>
    </div>
  {/if}

  <!-- Zone scale -->
  <div class="section">
    <div class="sec-label">Outcome zones</div>
    <div class="scale">
      {#each scale as { n, zone }}
        <div class="cell cell-{zone}">{n}</div>
      {/each}
    </div>
    <div class="zone-labels">
      <span class="zl zl-neg" style="flex:{localNegMax}">Negligible</span>
      <span class="zl zl-fle" style="flex:{localFleMax - localNegMax}">Fleeting</span>
      <span class="zl zl-int" style="flex:{10 - localFleMax}">Intense</span>
    </div>
    <div class="sliders">
      <div class="sl-row">
        <span class="sl-lbl">Negligible ends at</span>
        <input
          type="range" class="sl sl-neg" min="0" max="9"
          value={localNegMax}
          oninput={(e) => {
            localNegMax = +(e.target as HTMLInputElement).value;
            if (localFleMax <= localNegMax) {
              localFleMax = Math.min(localNegMax + 1, 10);
              onFleetingMaxChange(localFleMax);
            }
            onNegligibleMaxChange(localNegMax);
          }}
        />
        <span class="sl-val sl-val-neg">{localNegMax}</span>
      </div>
      <div class="sl-row">
        <span class="sl-lbl">Fleeting ends at</span>
        <input
          type="range" class="sl sl-fle" min={localNegMax + 1} max="10"
          value={localFleMax}
          oninput={(e) => {
            localFleMax = +(e.target as HTMLInputElement).value;
            onFleetingMaxChange(localFleMax);
          }}
        />
        <span class="sl-val sl-val-fle">{localFleMax}</span>
      </div>
    </div>
  </div>

  <!-- Probability readout -->
  <div class="section">
    <div class="sec-label">
      Outcome chances
      <span class="sec-hint">
        {diceCount === 1
          ? '1d10, straight roll'
          : `${diceCount}d10 — taking ${takeHighest ? 'best' : 'worst'}`}
      </span>
    </div>
    <div class="probs">
      <div class="prob-row">
        <span class="prob-lbl prob-neg">Negligible</span>
        <div class="prob-track">
          <div class="prob-fill prob-fill-neg" style="width:{probs.neg}%"></div>
        </div>
        <span class="prob-pct prob-neg">{probs.neg}%</span>
      </div>
      <div class="prob-row">
        <span class="prob-lbl prob-fle">Fleeting</span>
        <div class="prob-track">
          <div class="prob-fill prob-fill-fle" style="width:{probs.fle}%"></div>
        </div>
        <span class="prob-pct prob-fle">{probs.fle}%</span>
      </div>
      <div class="prob-row">
        <span class="prob-lbl prob-int">Intense</span>
        <div class="prob-track">
          <div class="prob-fill prob-fill-int" style="width:{probs.int}%"></div>
        </div>
        <span class="prob-pct prob-int">{probs.int}%</span>
      </div>
    </div>
  </div>
</div>

<style>
  .tc { display: flex; flex-direction: column; gap: 1.1rem; }

  .section { display: flex; flex-direction: column; gap: 0.45rem; }
  .sec-label {
    font-size: 0.68rem; text-transform: uppercase;
    letter-spacing: 0.1em; color: var(--text-label);
    display: flex; align-items: center; gap: 0.5rem;
  }
  .sec-hint {
    font-size: 0.65rem; color: var(--text-secondary);
    text-transform: none; letter-spacing: 0;
  }

  /* Dice pool */
  .dice-row { display: flex; gap: 5px; }
  .die {
    width: 2.625rem; height: 2.625rem;
    display: flex; flex-direction: column; align-items: center;
    justify-content: center; gap: 1px;
    background: var(--bg-raised); border: 1px solid var(--border-card);
    border-radius: 4px; cursor: pointer;
    transition: background 0.12s, border-color 0.12s, box-shadow 0.12s;
  }
  .die:hover:not(.die-on) { border-color: var(--border-surface); background: #1e0c0c; }
  .die-on { background: var(--bg-active); border-color: var(--border-active); box-shadow: 0 0 10px #cc222228; }
  .die-num { font-size: 1rem; color: var(--text-muted); font-weight: 700; line-height: 1; }
  .die-on .die-num { color: var(--accent); }
  .die-tag { font-size: 0.5rem; color: var(--text-muted); letter-spacing: 0.05em; }
  .die-on .die-tag { color: #7a3a3a; }

  /* Advantage */
  .adv-row { display: flex; gap: 6px; }
  .adv {
    flex: 1; padding: 0.45rem 0.5rem;
    display: flex; flex-direction: column; align-items: center; gap: 1px;
    background: var(--bg-raised); border: 1px solid var(--border-card);
    border-radius: 4px; cursor: pointer;
    transition: background 0.12s, border-color 0.12s;
  }
  .adv:hover:not(.adv-on) { border-color: var(--border-surface); }
  .adv-on { background: var(--bg-active); border-color: var(--border-active); }
  .adv-arrow { font-size: 1rem; color: var(--text-muted); line-height: 1; }
  .adv-on .adv-arrow { color: var(--accent); }
  .adv-name { font-size: 0.75rem; color: var(--text-muted); font-weight: 600; }
  .adv-on .adv-name { color: var(--accent); }
  .adv-sub { font-size: 0.6rem; color: var(--text-muted); }
  .adv-on .adv-sub { color: #7a3a3a; }

  /* Scale */
  .scale { display: flex; gap: 1px; border-radius: 3px; overflow: hidden; }
  .cell {
    flex: 1; text-align: center; padding: 5px 0;
    font-size: 0.72rem; font-weight: 700;
    transition: background 0.18s, color 0.18s;
  }
  .cell-neg { background: #1e1e1e; color: #4a4a4a; }
  .cell-fle { background: #2a1c00; color: var(--accent-amber); }
  .cell-int { background: #2a0000; color: var(--accent); }

  /* Zone labels */
  .zone-labels { display: flex; min-height: 1rem; }
  .zl {
    font-size: 0.58rem; text-align: center;
    text-transform: uppercase; letter-spacing: 0.06em;
    overflow: hidden; white-space: nowrap;
  }
  .zl-neg { color: var(--temp-negligible-dim); }
  .zl-fle { color: var(--temp-fleeting-dim); }
  .zl-int { color: var(--temp-intense-dim); }

  /* Sliders */
  .sliders { display: flex; flex-direction: column; gap: 0.35rem; }
  .sl-row { display: flex; align-items: center; gap: 0.5rem; }
  .sl-lbl { font-size: 0.72rem; color: var(--text-label); width: 7.1875rem; flex-shrink: 0; }
  .sl { flex: 1; cursor: pointer; }
  .sl-neg { accent-color: var(--text-muted); }
  .sl-fle { accent-color: var(--accent-amber); }
  .sl-val { font-size: 0.78rem; min-width: 18px; text-align: right; font-weight: 700; }
  .sl-val-neg { color: var(--text-muted); }
  .sl-val-fle { color: var(--accent-amber); }

  /* Probabilities */
  .probs { display: flex; flex-direction: column; gap: 0.35rem; }
  .prob-row { display: flex; align-items: center; gap: 0.5rem; }
  .prob-lbl { font-size: 0.72rem; width: 4.5rem; flex-shrink: 0; }
  .prob-neg { color: var(--temp-negligible-dim); }
  .prob-fle { color: var(--accent-amber); }
  .prob-int { color: var(--accent); }
  .prob-track {
    flex: 1; height: 5px;
    background: #181010; border-radius: 3px; overflow: hidden;
  }
  .prob-fill { height: 100%; border-radius: 3px; transition: width 0.25s ease; }
  .prob-fill-neg { background: var(--temp-negligible-dim); }
  .prob-fill-fle { background: var(--accent-amber); }
  .prob-fill-int { background: var(--accent); }
  .prob-pct { font-size: 0.72rem; min-width: 32px; text-align: right; }
</style>
