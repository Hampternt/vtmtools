<script lang="ts">
  // Single roll entry — splat-coded gutter, flavor + outcome badge,
  // actor/splat/criticals/time-ago meta row, and a dice grid colored
  // per result class. Extracted from RollFeed.svelte per plan-c Step 4.
  //
  // Per-die classification matches docs/reference/foundry-vtm5e-rolls.md:147-153.

  import { slide } from 'svelte/transition';
  import type { CanonicalRoll, RollSplat } from '../../types';

  let { roll }: { roll: CanonicalRoll } = $props();

  // ── Render helpers ──────────────────────────────────────────────────────

  function timeAgo(iso: string | null): string {
    if (!iso) return 'just now';
    const t = Date.parse(iso);
    if (Number.isNaN(t)) return 'just now';
    const diff = Math.floor((Date.now() - t) / 1000);
    if (diff < 5) return 'just now';
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return new Date(t).toLocaleString();
  }

  function splatLabel(s: RollSplat): string {
    if (s === 'unknown') return '?';
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  /** Classification of a basic die per docs/reference/foundry-vtm5e-rolls.md:147. */
  function basicClass(d: number): 'critical' | 'success' | 'failure' {
    if (d === 10) return 'critical';
    if (d >= 6) return 'success';
    return 'failure';
  }

  /** Classification of an advanced die — depends on splat. */
  function advancedClass(d: number, splat: RollSplat): string {
    if (splat === 'vampire') {
      if (d === 10) return 'critical messy';
      if (d === 1) return 'bestial';
      if (d >= 6) return 'success';
      return 'failure';
    }
    if (splat === 'werewolf') {
      if (d === 10) return 'critical';
      if (d === 1) return 'brutal';
      if (d >= 6) return 'success';
      return 'failure';
    }
    if (splat === 'hunter') {
      if (d === 10) return 'critical';
      if (d === 1) return 'desperation-fail';
      if (d >= 6) return 'success';
      return 'failure';
    }
    // mortal / unknown → no advanced dice expected, but classify defensively.
    return basicClass(d);
  }

  function outcomeBadge(r: CanonicalRoll): { label: string; cls: string } {
    if (r.bestial) return { label: 'BESTIAL', cls: 'bestial' };
    if (r.brutal) return { label: 'BRUTAL', cls: 'brutal' };
    if (r.messy) return { label: 'MESSY', cls: 'messy' };
    if (r.difficulty != null && r.total < r.difficulty) {
      return { label: `${r.total} / ${r.difficulty}`, cls: 'fail' };
    }
    return { label: `${r.total}${r.difficulty != null ? ` / ${r.difficulty}` : ''}`, cls: 'pass' };
  }

  const out = $derived(outcomeBadge(roll));
</script>

<div class="entry splat-{roll.splat}" in:slide={{ duration: 180, axis: 'y' }}>
  <div class="gutter splat-{roll.splat}"></div>
  <div class="body">
    <div class="row-main">
      <span class="flavor">{roll.flavor || 'Roll'}</span>
      <span class="outcome {out.cls}">{out.label}</span>
    </div>
    <div class="row-meta">
      <span class="actor">{roll.actor_name ?? 'GM'}</span>
      <span class="splat-tag splat-{roll.splat}">{splatLabel(roll.splat)}</span>
      {#if roll.criticals > 0}<span class="meta-pill criticals">{roll.criticals} crit</span>{/if}
      <span class="time">{timeAgo(roll.timestamp)}</span>
    </div>
    <div class="dice-row">
      {#each roll.basic_results as d}
        <span class="die basic {basicClass(d)}" title="basic die">{d}</span>
      {/each}
      {#if roll.advanced_results.length > 0}
        <span class="dice-sep">+</span>
        {#each roll.advanced_results as d}
          <span class="die advanced {advancedClass(d, roll.splat)}" title="advanced die ({roll.splat})">{d}</span>
        {/each}
      {/if}
    </div>
  </div>
</div>

<style>
  .entry {
    display: flex;
    gap: 0.6rem;
    padding: 0.5rem 0.6rem;
    background: var(--bg-sunken);
    border-radius: 3px;
    border: 1px solid var(--border-faint);
  }
  .gutter {
    width: 3px;
    border-radius: 2px;
    flex-shrink: 0;
    align-self: stretch;
  }
  .gutter.splat-vampire   { background: var(--alert-card-dossier); }
  .gutter.splat-werewolf  { background: var(--accent-amber); }
  .gutter.splat-hunter    { background: var(--accent); }
  .gutter.splat-mortal,
  .gutter.splat-unknown   { background: var(--text-muted); }

  .body { display: flex; flex-direction: column; gap: 0.3rem; flex: 1; min-width: 0; }

  .row-main {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    justify-content: space-between;
  }
  .flavor {
    font-size: 0.85rem;
    color: var(--text-primary);
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .outcome {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    padding: 0.1rem 0.5rem;
    border-radius: 2px;
    flex-shrink: 0;
  }
  .outcome.pass    { background: var(--bg-active); color: var(--text-primary); }
  .outcome.fail    { background: transparent; color: var(--text-muted); border: 1px solid var(--border-faint); }
  .outcome.messy   { background: var(--alert-card-dossier); color: var(--text-primary); }
  .outcome.bestial { background: transparent; color: var(--alert-card-dossier); border: 1px solid var(--alert-card-dossier); }
  .outcome.brutal  { background: transparent; color: var(--accent-amber); border: 1px solid var(--accent-amber); }

  .row-meta {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    flex-wrap: wrap;
    font-size: 0.7rem;
  }
  .actor { color: var(--text-secondary); }
  .splat-tag {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    padding: 0 0.3rem;
    border-radius: 2px;
    border: 1px solid var(--border-faint);
  }
  .splat-tag.splat-vampire   { color: var(--alert-card-dossier); border-color: var(--alert-card-dossier); }
  .splat-tag.splat-werewolf  { color: var(--accent-amber);       border-color: var(--accent-amber); }
  .splat-tag.splat-hunter    { color: var(--accent);              border-color: var(--accent); }
  .meta-pill {
    font-size: 0.65rem;
    color: var(--text-muted);
    background: var(--bg-base);
    padding: 0 0.3rem;
    border-radius: 2px;
  }
  .meta-pill.criticals { color: var(--alert-card-dossier); }
  .time { margin-left: auto; color: var(--text-muted); }

  .dice-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
  }
  .dice-sep {
    color: var(--text-muted);
    font-size: 0.85rem;
    margin: 0 0.15rem;
  }
  .die {
    width: 1.4rem;
    height: 1.4rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.7rem;
    font-weight: 700;
    border-radius: 3px;
    border: 1px solid var(--border-faint);
    background: var(--bg-base);
    color: var(--text-primary);
  }
  .die.failure          { color: var(--text-muted); }
  .die.success          { color: var(--text-primary); }
  .die.critical         { color: var(--alert-card-dossier); border-color: var(--alert-card-dossier); }
  .die.critical.messy   { background: var(--alert-card-dossier); color: var(--bg-base); }
  .die.bestial          { color: var(--alert-card-dossier); border: 1px dashed var(--alert-card-dossier); }
  .die.brutal           { color: var(--accent-amber);       border: 1px dashed var(--accent-amber); }
  .die.desperation-fail { color: var(--accent);             border: 1px dashed var(--accent); }
</style>
