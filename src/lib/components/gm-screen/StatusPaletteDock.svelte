<script lang="ts">
  import { onMount } from 'svelte';
  import { statusTemplates } from '../../../store/statusTemplates.svelte';
  import { modifiers } from '../../../store/modifiers.svelte';
  import StatusTemplateEditor from './StatusTemplateEditor.svelte';
  import StatusTemplateCard from './StatusTemplateCard.svelte';
  import type { StatusTemplate, BridgeCharacter } from '../../../types';

  interface Props {
    /**
     * Currently focused character (last-clicked row), or null if none.
     * Click-to-apply is gated on this — clicking a template with no focus
     * surfaces a hint instead of silently applying to the topmost character.
     */
    focusedCharacter: BridgeCharacter | null;
  }

  let { focusedCharacter }: Props = $props();

  let editorOpen = $state(false);
  let editorExisting = $state<StatusTemplate | null>(null);
  // Tagged hint so the error tone can persist (no auto-clear) while
  // success/info auto-fade. Errors stay until the user clicks again.
  let applyHint = $state<{ text: string; tone: 'info' | 'success' | 'error' } | null>(null);

  onMount(() => { void statusTemplates.ensureLoaded(); });

  async function applyTemplate(t: StatusTemplate): Promise<void> {
    if (!focusedCharacter) {
      applyHint = { text: 'Click a character row first, then a template.', tone: 'info' };
      setTimeout(() => { applyHint = null; }, 8000);
      return;
    }
    try {
      // $state.snapshot strips the Svelte 5 runes proxy off the template's
      // effects (structuredClone throws DataCloneError on proxies). Spec §8.4
      // independent copy is also preserved by the IPC round-trip (Rust stores
      // effects as JSON), but the snapshot keeps the intent visible.
      await modifiers.add({
        source: focusedCharacter.source,
        sourceId: focusedCharacter.source_id,
        name: t.name,
        description: t.description,
        effects: $state.snapshot(t.effects),
        binding: { kind: 'free' },
        tags: [...t.tags],
        originTemplateId: t.id,
        foundryCapturedLabels: [],
        zone: 'situational',
      });
      applyHint = { text: `Applied "${t.name}" to ${focusedCharacter.name}.`, tone: 'success' };
      setTimeout(() => { applyHint = null; }, 2500);
    } catch (err) {
      console.error('[gm-screen] apply template failed:', err);
      applyHint = { text: `Failed to apply: ${err}`, tone: 'error' };
    }
  }

  function openNewEditor() {
    editorExisting = null;
    editorOpen = true;
  }

  function openEditEditor(t: StatusTemplate) {
    editorExisting = t;
    editorOpen = true;
  }

  function summarize(t: StatusTemplate): string {
    if (t.effects.length === 0) return '(no effects)';
    return t.effects.map(e => {
      if (e.kind === 'note') return e.note ?? 'note';
      const sign = (e.delta ?? 0) >= 0 ? '+' : '';
      const what = e.kind === 'pool' ? 'dice' : 'diff';
      return `${e.scope ? e.scope + ' ' : ''}${sign}${e.delta ?? 0} ${what}`;
    }).join(' · ');
  }
</script>

<aside class="palette">
  <header>
    <h2>Status palette</h2>
    <button class="new" onclick={openNewEditor}>+ New template</button>
  </header>

  {#if applyHint}<p class="hint" class:error={applyHint.tone === 'error'}>{applyHint.text}</p>{/if}
  {#if !focusedCharacter}<p class="hint muted">Click a character row to enable template apply.</p>{/if}

  {#if statusTemplates.list.length === 0}
    <p class="empty">No templates yet. Click <strong>+ New template</strong>.</p>
  {:else}
    <div class="grid">
      {#each statusTemplates.list as t (t.id)}
        <StatusTemplateCard
          input={{ kind: 'saved', data: t }}
          summary={summarize(t)}
          applyDisabled={!focusedCharacter}
          applyTitle={focusedCharacter ? `Apply to ${focusedCharacter.name}` : 'Pick a character first'}
          onapply={() => applyTemplate(t)}
          onedit={() => openEditEditor(t)}
        />
      {/each}
    </div>
  {/if}

  {#if editorOpen}
    <div class="editor-overlay">
      <StatusTemplateEditor existing={editorExisting} onClose={() => { editorOpen = false; editorExisting = null; }} />
    </div>
  {/if}
</aside>

<style>
  .palette {
    background: var(--bg-card);
    border-left: 1px solid var(--border-faint);
    padding: 0.85rem;
    width: 18rem;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    overflow-y: auto;
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  header h2 { margin: 0; font-size: 0.85rem; color: var(--text-label); }
  .new {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.2rem 0.5rem;
    font-size: 0.7rem;
    cursor: pointer;
  }
  .hint { font-size: 0.7rem; color: var(--accent-amber); margin: 0; }
  .hint.muted { color: var(--text-muted); }
  .hint.error {
    color: var(--accent-amber);
    font-weight: 500;
    padding: 0.25rem 0.4rem;
    border-left: 2px solid var(--accent-amber);
    background: var(--bg-input);
    border-radius: 0.25rem;
    word-break: break-word;
  }
  .empty { color: var(--text-muted); font-size: 0.75rem; font-style: italic; }
  .grid { display: flex; flex-direction: column; gap: 0.4rem; }

  .editor-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }
</style>
