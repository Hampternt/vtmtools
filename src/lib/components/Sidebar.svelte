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
  .sidebar {
    width: 200px;
    min-height: 100vh;
    background: #0d0d0d;
    border-right: 1px solid #3a0a0a;
    display: flex;
    flex-direction: column;
    padding: 1rem 0;
  }
  .tool-btn {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    color: #8a8a8a;
    cursor: pointer;
    font-size: 0.9rem;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }
  .tool-btn:hover {
    background: #1a0505;
    color: #cc2222;
  }
  .tool-btn.active {
    background: #1a0505;
    color: #cc2222;
    border-left: 3px solid #cc2222;
  }
  .icon { font-size: 1.1rem; }
  .label { font-weight: 500; }
</style>
