<script lang="ts">
  import { bridge } from '../store/bridge.svelte';
  import {
    triggerFoundryRoll,
    postFoundryChat,
    type RollV5PoolInput,
    type PostChatAsActorInput,
  } from '$lib/foundry-chat/api';
  import type { BridgeCharacter } from '../types';

  // ── Last-action feedback ─────────────────────────────────────────────
  type ActionState =
    | { kind: 'idle' }
    | { kind: 'pending'; label: string }
    | { kind: 'ok'; label: string; at: string }
    | { kind: 'err'; label: string; message: string };

  let action: ActionState = $state({ kind: 'idle' });

  // ── Character picker ─────────────────────────────────────────────────
  // Restrict to Foundry — game.* helpers are Foundry-only.
  const foundryCharacters = $derived(
    bridge.characters.filter((c: BridgeCharacter) => c.source === 'foundry'),
  );
  let selectedActorId: string = $state('');

  $effect(() => {
    if (
      selectedActorId &&
      !foundryCharacters.some((c) => c.source_id === selectedActorId)
    ) {
      selectedActorId = '';
    }
  });

  // ── Roll form ────────────────────────────────────────────────────────
  let rollPathsText: string = $state(
    'attributes.strength.value, skills.brawl.value',
  );
  let rollDifficulty: number = $state(3);
  let rollFlavor: string = $state('Foundry-test roll');
  let rollAdvancedDice: string = $state(''); // empty = auto
  let rollSelectorsText: string = $state('');

  async function submitRoll() {
    if (!selectedActorId) {
      action = {
        kind: 'err',
        label: 'trigger_foundry_roll',
        message: 'Pick a Foundry actor first.',
      };
      return;
    }
    const valuePaths = rollPathsText
      .split(/[,\s]+/)
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
    const advancedDice =
      rollAdvancedDice.trim() === ''
        ? null
        : Number.parseInt(rollAdvancedDice.trim(), 10);
    const selectors =
      rollSelectorsText.trim() === ''
        ? null
        : rollSelectorsText
            .split(/[,\s]+/)
            .map((s) => s.trim())
            .filter((s) => s.length > 0);
    const input: RollV5PoolInput = {
      actorId: selectedActorId,
      valuePaths,
      difficulty: Number.isFinite(rollDifficulty) ? rollDifficulty : 0,
      flavor: rollFlavor.trim() === '' ? null : rollFlavor,
      advancedDice: Number.isFinite(advancedDice as number)
        ? (advancedDice as number)
        : null,
      selectors,
    };
    action = { kind: 'pending', label: 'trigger_foundry_roll' };
    try {
      await triggerFoundryRoll(input);
      action = {
        kind: 'ok',
        label: 'trigger_foundry_roll',
        at: new Date().toLocaleTimeString(),
      };
    } catch (e) {
      action = {
        kind: 'err',
        label: 'trigger_foundry_roll',
        message: String(e),
      };
    }
  }

  function fillRouseDefaults() {
    rollPathsText = '';
    rollDifficulty = 0;
    rollFlavor = 'Rouse check';
    rollAdvancedDice = '1';
    rollSelectorsText = '';
  }

  // ── Chat form ────────────────────────────────────────────────────────
  let chatContent: string = $state(
    '<p>Test message from vtmtools.</p>',
  );
  let chatFlavor: string = $state('Test');
  let chatRollMode: 'roll' | 'gmroll' | 'blindroll' | 'selfroll' =
    $state('roll');

  async function submitChat() {
    if (!selectedActorId) {
      action = {
        kind: 'err',
        label: 'post_foundry_chat',
        message: 'Pick a Foundry actor first.',
      };
      return;
    }
    const input: PostChatAsActorInput = {
      actorId: selectedActorId,
      content: chatContent,
      flavor: chatFlavor.trim() === '' ? null : chatFlavor,
      rollMode: chatRollMode,
    };
    action = { kind: 'pending', label: 'post_foundry_chat' };
    try {
      await postFoundryChat(input);
      action = {
        kind: 'ok',
        label: 'post_foundry_chat',
        at: new Date().toLocaleTimeString(),
      };
    } catch (e) {
      action = {
        kind: 'err',
        label: 'post_foundry_chat',
        message: String(e),
      };
    }
  }

  async function submitInvalidActor() {
    action = { kind: 'pending', label: 'trigger_foundry_roll (error path)' };
    try {
      await triggerFoundryRoll({
        actorId: 'no-such-actor',
        valuePaths: ['attributes.strength.value'],
        difficulty: 0,
      });
      action = {
        kind: 'ok',
        label: 'trigger_foundry_roll (unexpected success)',
        at: new Date().toLocaleTimeString(),
      };
    } catch (e) {
      action = {
        kind: 'err',
        label: 'trigger_foundry_roll (error path — expected)',
        message: String(e),
      };
    }
  }
</script>

<section class="root">
  <header>
    <h1>Foundry Test Tool</h1>
    <p class="hint">
      Exercises the FHL Phase 2 game.* helpers (issue #17). Foundry must be
      connected and the bridge module must be ≥ 0.3.0.
    </p>
  </header>

  <div class="grid">
    <div class="picker card">
      <h2>Actor</h2>
      {#if foundryCharacters.length === 0}
        <p class="empty">
          No Foundry characters connected. Open the Foundry world and ensure
          the bridge module is loaded.
        </p>
      {:else}
        <label>
          <span class="label">Foundry actor</span>
          <select bind:value={selectedActorId}>
            <option value="">— pick an actor —</option>
            {#each foundryCharacters as char (char.source_id)}
              <option value={char.source_id}>{char.name}</option>
            {/each}
          </select>
        </label>
        {#if selectedActorId}
          <p class="meta">id: <code>{selectedActorId}</code></p>
        {/if}
      {/if}
    </div>

    <div class="card">
      <h2>game.roll_v5_pool</h2>
      <label>
        <span class="label">value_paths (comma- or space-separated)</span>
        <input type="text" bind:value={rollPathsText} placeholder="leave empty for rouse" />
      </label>
      <label>
        <span class="label">difficulty</span>
        <input type="number" min="0" max="10" bind:value={rollDifficulty} />
      </label>
      <label>
        <span class="label">flavor (optional)</span>
        <input type="text" bind:value={rollFlavor} />
      </label>
      <label>
        <span class="label">advanced_dice (blank = auto-derive)</span>
        <input type="text" inputmode="numeric" bind:value={rollAdvancedDice} placeholder="auto" />
      </label>
      <label>
        <span class="label">selectors (comma- or space-separated, optional)</span>
        <input type="text" bind:value={rollSelectorsText} placeholder="attributes.strength, skills.brawl" />
      </label>
      <div class="actions">
        <button type="button" onclick={submitRoll} disabled={action.kind === 'pending'}>
          Send roll
        </button>
        <button type="button" class="ghost" onclick={fillRouseDefaults}>
          Fill rouse defaults
        </button>
      </div>
    </div>

    <div class="card">
      <h2>game.post_chat_as_actor</h2>
      <label>
        <span class="label">content (HTML)</span>
        <textarea rows="3" bind:value={chatContent}></textarea>
      </label>
      <label>
        <span class="label">flavor (optional)</span>
        <input type="text" bind:value={chatFlavor} />
      </label>
      <label>
        <span class="label">roll mode</span>
        <select bind:value={chatRollMode}>
          <option value="roll">roll (public)</option>
          <option value="gmroll">gmroll</option>
          <option value="blindroll">blindroll</option>
          <option value="selfroll">selfroll</option>
        </select>
      </label>
      <div class="actions">
        <button type="button" onclick={submitChat} disabled={action.kind === 'pending'}>
          Post chat
        </button>
      </div>
    </div>

    <div class="card">
      <h2>Error path</h2>
      <p class="hint">
        Triggers a roll against a non-existent actor — expected to surface
        an error from the bridge.
      </p>
      <div class="actions">
        <button type="button" class="ghost" onclick={submitInvalidActor} disabled={action.kind === 'pending'}>
          Trigger error path
        </button>
      </div>
    </div>

    <div class="card status">
      <h2>Last action</h2>
      {#if action.kind === 'idle'}
        <p class="empty">No action yet.</p>
      {:else if action.kind === 'pending'}
        <p class="pending">⏳ {action.label}…</p>
      {:else if action.kind === 'ok'}
        <p class="ok">✓ {action.label} succeeded at {action.at}</p>
      {:else}
        <p class="err">✗ {action.label} failed</p>
        <pre class="err-detail">{action.message}</pre>
      {/if}
    </div>
  </div>
</section>

<style>
  .root {
    padding: 1.5rem;
    color: var(--text-primary);
  }
  header h1 {
    margin: 0 0 0.25rem;
    color: var(--text-primary);
    font-size: 1.25rem;
  }
  .hint {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin: 0 0 1rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 1rem;
  }
  .card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 0.5rem;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .card h2 {
    margin: 0;
    color: var(--text-label);
    font-size: 0.9rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .label {
    color: var(--text-label);
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  input,
  textarea,
  select {
    background: var(--bg-input);
    color: var(--text-primary);
    border: 1px solid var(--border-surface);
    border-radius: 0.25rem;
    padding: 0.4rem 0.5rem;
    font-family: inherit;
    font-size: 0.9rem;
  }
  textarea {
    resize: vertical;
    font-family: ui-monospace, monospace;
  }
  input:focus,
  textarea:focus,
  select:focus {
    outline: 1px solid var(--border-active);
    border-color: var(--border-active);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  button {
    background: var(--accent);
    color: var(--text-primary);
    border: none;
    border-radius: 0.25rem;
    padding: 0.5rem 1rem;
    font-family: inherit;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--accent-bright);
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  button.ghost {
    background: var(--bg-raised);
    color: var(--text-label);
    border: 1px solid var(--border-surface);
  }
  button.ghost:hover:not(:disabled) {
    background: var(--bg-active);
    color: var(--text-primary);
  }
  .status .empty,
  .picker .empty {
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin: 0;
  }
  .status .pending {
    color: var(--accent-amber);
    margin: 0;
  }
  .status .ok {
    color: var(--text-primary);
    margin: 0;
  }
  .status .err {
    color: var(--accent-bright);
    margin: 0 0 0.5rem;
  }
  .err-detail {
    background: var(--bg-sunken);
    color: var(--accent-amber);
    padding: 0.5rem;
    border-radius: 0.25rem;
    border: 1px solid var(--border-faint);
    font-size: 0.8rem;
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
  }
  .meta {
    color: var(--text-muted);
    font-size: 0.75rem;
    margin: 0;
  }
  code {
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
  }
</style>
