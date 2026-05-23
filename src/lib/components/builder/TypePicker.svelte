<script lang="ts">
  export type EnabledType = 'merit' | 'flaw' | 'background' | 'boon';
  export type PreviewType = 'discipline' | 'item';
  type AllType = EnabledType | PreviewType;

  interface Props {
    available?: EnabledType[];
    preview?: PreviewType[];
    onPick: (type: EnabledType) => void;
  }

  let { available = ['merit', 'flaw', 'background', 'boon'], preview = ['discipline', 'item'], onPick }: Props = $props();

  let selected: AllType | null = $state(null);

  const CARDS = {
    merit:      { glyph: 'M', title: 'Merit',      desc: 'A character advantage. Buys dots for an in-game bonus.' },
    flaw:       { glyph: 'F', title: 'Flaw',       desc: 'A character drawback. Awards experience.' },
    background: { glyph: 'B', title: 'Background', desc: 'External resources or connections.' },
    boon:       { glyph: '★', title: 'Boon',       desc: 'Owed favor between Kindred.' },
    discipline: { glyph: 'D', title: 'Discipline', desc: 'Vampiric power with school + levels.' },
    item:       { glyph: 'I', title: 'Item',       desc: 'Weapon, armor, or gear.' },
  } as const;

  function onKeydown(e: KeyboardEvent) {
    const allOptions = available as AllType[];

    if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') {
      const i = allOptions.indexOf(selected ?? allOptions[0]);
      const next = allOptions[(i - 1 + allOptions.length) % allOptions.length];
      selected = next;
      focusCard(next);
      e.preventDefault();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowRight') {
      const i = allOptions.indexOf(selected ?? allOptions[0]);
      const next = allOptions[(i + 1) % allOptions.length];
      selected = next;
      focusCard(next);
      e.preventDefault();
    } else if (e.key === ' ' || e.key === 'Enter') {
      if (selected && available.includes(selected as EnabledType)) {
        onPick(selected as EnabledType);
      }
      e.preventDefault();
    }
  }

  function focusCard(t: AllType) {
    const allOptions = available as AllType[];
    const idx = allOptions.indexOf(t);
    const buttons = document.querySelectorAll('.picker-card');
    (buttons[idx] as HTMLElement)?.focus();
  }

  function handleCardClick(t: EnabledType) {
    selected = t;
    onPick(t);
  }
</script>

<div class="picker-grid" role="radiogroup" aria-label="Type" tabindex="0" onkeydown={onKeydown}>
  {#each available as t, i}
    <button
      type="button"
      class="picker-card"
      role="radio"
      aria-checked={selected === t}
      tabindex={(selected ? selected === t : i === 0) ? 0 : -1}
      onclick={() => handleCardClick(t)}
    >
      <span class="glyph">{CARDS[t].glyph}</span>
      <span class="text">
        <strong>{CARDS[t].title}</strong>
        <span class="desc">{CARDS[t].desc}</span>
      </span>
    </button>
  {/each}
  {#each preview as t}
    <button
      type="button"
      class="picker-card preview-disabled"
      disabled
      title="coming in v1.x — body needs design"
    >
      <span class="glyph">{CARDS[t].glyph}</span>
      <span class="text">
        <strong>{CARDS[t].title}</strong>
        <span class="desc">{CARDS[t].desc}</span>
        <span class="pill">preview · needs design</span>
      </span>
    </button>
  {/each}
</div>

<style>
  .picker-grid {
    display: grid; grid-template-columns: 1fr 1fr; gap: 0.6rem;
    padding: 0.85rem;
  }
  .picker-card {
    background: var(--bg-card);
    border: 1px solid var(--border-faint);
    border-radius: 0.35rem;
    padding: 0.85rem 0.95rem;
    cursor: pointer;
    display: flex; align-items: flex-start; gap: 0.7rem;
    text-align: left;
    font-family: inherit;
    color: var(--text-primary);
  }
  .picker-card:hover:not([disabled]) {
    border-color: var(--accent);
    background: rgba(168, 80, 64, 0.08);
  }
  .picker-card.preview-disabled {
    opacity: 0.45;
    cursor: not-allowed;
    border-style: dashed;
  }
  .glyph {
    width: 2rem; height: 2rem; flex-shrink: 0;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    display: flex; align-items: center; justify-content: center;
    font-size: 1.05rem; font-weight: 600;
    color: var(--accent-amber);
    font-family: ui-serif, "Iowan Old Style", serif;
  }
  .text { display: flex; flex-direction: column; gap: 0.15rem; flex: 1; min-width: 0; }
  .text strong { font-size: 0.82rem; color: var(--text-primary); font-weight: 600; }
  .desc { font-size: 0.68rem; color: var(--text-muted); line-height: 1.4; }
  .pill {
    font-size: 0.55rem; color: var(--text-muted);
    background: rgba(0,0,0,0.25);
    padding: 0.08rem 0.4rem; border-radius: 999px;
    letter-spacing: 0.06em; text-transform: uppercase;
    align-self: flex-start; margin-top: 0.15rem;
  }
</style>
