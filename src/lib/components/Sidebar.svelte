<script lang="ts">
  import { tools } from '../../tools';

  interface Props {
    activeTool: string;
    onSelect: (id: string) => void;
  }

  let { activeTool, onSelect }: Props = $props();
</script>

<nav class="sidebar">
  {#each tools as tool}
    <button
      class="tool-btn {activeTool === tool.id ? 'active' : ''}"
      onclick={() => onSelect(tool.id)}
      aria-label={tool.label}
    >
      <span class="icon">{tool.icon}</span>
      <span class="label">{tool.label}</span>
    </button>
  {/each}
</nav>

<style>
  /* ── Wide: full sidebar with labels ── */
  .sidebar {
    width: 12.5rem;
    min-height: 100vh;
    background: var(--bg-base);
    border-right: 1px solid var(--border-surface);
    display: flex;
    flex-direction: column;
    padding: 1rem 0;
    flex-shrink: 0;
  }
  .tool-btn {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.9rem;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }
  .tool-btn:hover { background: #1a0505; color: var(--accent); }
  .tool-btn.active {
    background: #1a0505;
    color: var(--accent);
    border-left: 3px solid var(--accent);
  }
  .icon { font-size: 1.1rem; }
  .label { font-weight: 500; }

  /* ── Narrow window: icon-only strip ── */
  @media (max-width: 40rem) {
    .sidebar { width: 3rem; padding: 0.5rem 0; }
    .label { display: none; }
    .tool-btn {
      justify-content: center;
      padding: 0.75rem 0;
      border-left-width: 0 !important;
    }
    .tool-btn.active { border-bottom: 3px solid var(--accent); }
  }

  /* ── Very narrow: horizontal top bar ── */
  @media (max-width: 28rem) {
    .sidebar {
      width: 100%;
      min-height: auto;
      flex-direction: row;
      border-right: none;
      border-bottom: 1px solid var(--border-surface);
      padding: 0;
      overflow-x: auto;
    }
    .label { display: inline; }
    .tool-btn {
      flex: 0 0 auto;
      padding: 0.6rem 1rem;
      border-bottom: none !important;
      border-left: none !important;
    }
    .tool-btn.active {
      border-bottom: 3px solid var(--accent) !important;
    }
  }
</style>
