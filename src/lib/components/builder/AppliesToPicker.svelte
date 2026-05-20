<script lang="ts">
  import { FOUNDRY_SKILL_NAMES, FOUNDRY_ATTRIBUTE_NAMES } from '$lib/foundry/canonical-names';

  interface Props {
    paths: string[];
    onChange: (paths: string[]) => void;
  }

  let { paths, onChange }: Props = $props();
  let advancedOpen = $state(false);
  let pathDraft = $state('');

  const PHYSICAL = ['athletics', 'brawl', 'craft', 'drive', 'firearms', 'larceny', 'melee', 'stealth', 'survival'];
  const SOCIAL   = ['animal_ken', 'etiquette', 'insight', 'intimidation', 'leadership', 'performance', 'persuasion', 'streetwise', 'subterfuge'];
  const MENTAL   = ['academics', 'awareness', 'finance', 'investigation', 'medicine', 'occult', 'politics', 'science', 'technology'];

  const SOCIAL_PATHS   = SOCIAL.map(s => `skills.${s}`);
  const PHYSICAL_PATHS = PHYSICAL.map(s => `skills.${s}`);
  const MENTAL_PATHS   = MENTAL.map(s => `skills.${s}`);

  const selectedSemantic = $derived.by(() => {
    const set = new Set(paths);
    if (paths.length === 0) return 'all';
    if (paths.length === SOCIAL_PATHS.length   && SOCIAL_PATHS.every(p   => set.has(p))) return 'social';
    if (paths.length === PHYSICAL_PATHS.length && PHYSICAL_PATHS.every(p => set.has(p))) return 'physical';
    if (paths.length === MENTAL_PATHS.length   && MENTAL_PATHS.every(p   => set.has(p))) return 'mental';
    return 'custom';
  });

  function selectCategory(category: 'social' | 'physical' | 'mental' | 'all') {
    if (category === 'social')   onChange([...SOCIAL_PATHS]);
    if (category === 'physical') onChange([...PHYSICAL_PATHS]);
    if (category === 'mental')   onChange([...MENTAL_PATHS]);
    if (category === 'all')      onChange([]);
  }

  function pickSkill(skill: string) {
    onChange([`skills.${skill}`]);
  }

  function pickAttribute(attr: string) {
    onChange([`attributes.${attr}`]);
  }

  function commitPath() {
    const p = pathDraft.trim();
    if (!p || paths.includes(p)) { pathDraft = ''; return; }
    onChange([...paths, p]);
    pathDraft = '';
  }

  function removePath(p: string) {
    onChange(paths.filter(x => x !== p));
  }
</script>

<div class="applies-to">
  <div class="chip-row">
    <button class:selected={selectedSemantic === 'social'}   onclick={() => selectCategory('social')}>Social</button>
    <button class:selected={selectedSemantic === 'physical'} onclick={() => selectCategory('physical')}>Physical</button>
    <button class:selected={selectedSemantic === 'mental'}   onclick={() => selectCategory('mental')}>Mental</button>
    <details class="skill-detail">
      <summary>A skill…</summary>
      <div class="dropdown">
        {#each FOUNDRY_SKILL_NAMES as s}
          <button class="dropdown-item" onclick={() => pickSkill(s)}>{s}</button>
        {/each}
      </div>
    </details>
    <details class="skill-detail">
      <summary>An attribute…</summary>
      <div class="dropdown">
        {#each FOUNDRY_ATTRIBUTE_NAMES as a}
          <button class="dropdown-item" onclick={() => pickAttribute(a)}>{a}</button>
        {/each}
      </div>
    </details>
    <button class:selected={selectedSemantic === 'all'} onclick={() => selectCategory('all')}>All rolls</button>
  </div>

  <button class="advanced-toggle" onclick={() => advancedOpen = !advancedOpen}>
    {advancedOpen ? '▾' : '▸'} Advanced: paths · narrow scope
  </button>

  {#if advancedOpen}
    <div class="advanced">
      <div class="path-chips">
        {#each paths as p}
          <span class="path-chip">
            {p === '' ? '(pathless)' : p}
            <button onclick={() => removePath(p)} aria-label="Remove path {p}">×</button>
          </span>
        {/each}
        <input
          type="text"
          placeholder="+ path (e.g. attributes.strength)"
          bind:value={pathDraft}
          onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); commitPath(); } }}
          onblur={() => { if (pathDraft.trim()) commitPath(); }}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .applies-to { display: flex; flex-direction: column; gap: 0.4rem; }
  .chip-row { display: flex; gap: 0.3rem; flex-wrap: wrap; align-items: flex-start; }
  .chip-row > button, .skill-detail > summary {
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    color: var(--text-secondary);
    border-radius: 0.25rem;
    padding: 0.32rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
    font-family: inherit;
  }
  .chip-row > button:hover, .skill-detail > summary:hover { border-color: var(--accent); color: var(--accent-bright); }
  .chip-row > button.selected {
    border-color: var(--accent);
    background: rgba(168, 80, 64, 0.15);
    color: var(--accent-bright);
    font-weight: 600;
  }
  .skill-detail { position: relative; }
  .skill-detail > summary { list-style: none; cursor: pointer; }
  .dropdown {
    position: absolute; top: 100%; left: 0; z-index: 10;
    margin-top: 0.2rem; max-height: 14rem; overflow-y: auto;
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-radius: 0.3rem;
    padding: 0.3rem;
    box-shadow: 0 0.4rem 1rem -0.2rem rgba(0,0,0,0.5);
    display: flex; flex-direction: column; gap: 0.15rem;
    min-width: 9rem;
  }
  .dropdown-item {
    background: transparent; border: none; text-align: left;
    color: var(--text-secondary); padding: 0.25rem 0.4rem;
    font-size: 0.72rem; cursor: pointer; border-radius: 0.2rem;
    font-family: inherit;
  }
  .dropdown-item:hover { background: var(--bg-card); color: var(--accent-bright); }
  .advanced-toggle {
    background: transparent; border: none; color: var(--text-muted);
    font-size: 0.65rem; cursor: pointer; align-self: flex-start;
    padding: 0.25rem 0; letter-spacing: 0.04em;
  }
  .advanced-toggle:hover { color: var(--accent-bright); }
  .advanced {
    padding: 0.4rem;
    background: rgba(0,0,0,0.18);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
  }
  .path-chips { display: flex; gap: 0.3rem; flex-wrap: wrap; align-items: center; }
  .path-chip {
    background: var(--bg-input); color: var(--text-secondary);
    border: 1px solid var(--border-faint); border-radius: 999px;
    padding: 0.1rem 0.45rem; font-size: 0.65rem;
    font-family: ui-monospace, monospace;
    display: inline-flex; align-items: center; gap: 0.2rem;
  }
  .path-chip button {
    background: transparent; border: none; color: var(--text-muted);
    cursor: pointer; font-size: 0.7rem; padding: 0;
  }
  .path-chips input {
    background: var(--bg-input); border: 1px solid var(--border-faint);
    color: var(--text-primary); border-radius: 0.2rem;
    padding: 0.15rem 0.4rem; font-size: 0.65rem; width: 11rem;
    font-family: ui-monospace, monospace;
    box-sizing: border-box;
  }
</style>
