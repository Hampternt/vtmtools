# Resonance → Roll20 Write-back Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow the GM to select a Roll20 character in the Resonance Roller, roll resonance, and push the result to that character's sheet with one click.

**Architecture:** A new `SetAttribute` outbound message is added to the Rust WebSocket layer and surfaced as a Tauri command. The browser extension gains a handler for that message that writes to Roll20's Backbone/Firebase attribute store. `Resonance.svelte` is restructured to show a character selector at the top, move the result card above the config, and add an Apply button.

**Tech Stack:** Tauri 2, Rust (serde, tokio-tungstenite), SvelteKit + Svelte 5 runes, TypeScript, browser extension (MV3, plain JS, Roll20 Backbone API).

**Correctness gates — no test suite exists:**
- Rust: `cargo check --manifest-path src-tauri/Cargo.toml`
- Frontend: `npm run check` (svelte-check + tsc)

---

### Task 1: Add `SetAttribute` variant to `OutboundMsg`

**Files:**
- Modify: `src-tauri/src/roll20/types.rs`

- [ ] **Step 1: Add the variant**

Open `src-tauri/src/roll20/types.rs`. The `OutboundMsg` enum currently ends after `SendChat`. Add the new variant:

```rust
/// Outbound messages sent to the browser extension.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMsg {
    Refresh,
    SendChat { message: String },
    SetAttribute {
        character_id: String,
        name: String,
        value: String,
    },
}
```

This serialises to `{ "type": "set_attribute", "character_id": "...", "name": "resonance", "value": "Choleric" }`.

- [ ] **Step 2: Verify it compiles**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors or warnings about the new variant.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/roll20/types.rs
git commit -m "feat(roll20): add SetAttribute variant to OutboundMsg"
```

---

### Task 2: Add `set_roll20_attribute` Tauri command

**Files:**
- Modify: `src-tauri/src/roll20/commands.rs`

- [ ] **Step 1: Add the command function**

Open `src-tauri/src/roll20/commands.rs`. Add after `send_roll20_chat`:

```rust
/// Writes a single attribute on a Roll20 character sheet via the extension.
/// No-op if no extension is connected.
#[tauri::command]
pub async fn set_roll20_attribute(
    character_id: String,
    name: String,
    value: String,
    conn: State<'_, Roll20Conn>,
) -> Result<(), String> {
    let tx = conn.0.outbound_tx.lock().await.clone();
    if let Some(tx) = tx {
        let msg = serde_json::to_string(&OutboundMsg::SetAttribute {
            character_id,
            name,
            value,
        })
        .map_err(|e| e.to_string())?;
        tx.send(msg).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/roll20/commands.rs
git commit -m "feat(roll20): add set_roll20_attribute command"
```

---

### Task 3: Register the new command in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add to the invoke handler list**

Open `src-tauri/src/lib.rs`. In the `.invoke_handler(tauri::generate_handler![...])` block, add the new command after `send_roll20_chat`:

```rust
        .invoke_handler(tauri::generate_handler![
            tools::resonance::roll_resonance,
            db::dyscrasia::list_dyscrasias,
            db::dyscrasia::add_dyscrasia,
            db::dyscrasia::update_dyscrasia,
            db::dyscrasia::delete_dyscrasia,
            db::dyscrasia::roll_random_dyscrasia,
            tools::export::export_result_to_md,
            roll20::commands::get_roll20_characters,
            roll20::commands::get_roll20_status,
            roll20::commands::refresh_roll20_data,
            roll20::commands::send_roll20_chat,
            roll20::commands::set_roll20_attribute,
        ])
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(roll20): register set_roll20_attribute in invoke handler"
```

---

### Task 4: Handle `set_attribute` in the browser extension

**Files:**
- Modify: `extension/content.js`

- [ ] **Step 1: Add the message handler branch**

Open `extension/content.js`. Inside the `ws.addEventListener('message', ...)` callback, the current branches handle `'refresh'` and `'send_chat'`. Add a third branch:

```js
  ws.addEventListener('message', (event) => {
    try {
      const msg = JSON.parse(event.data);
      if (msg.type === 'refresh') {
        readAllCharacters();
      } else if (msg.type === 'send_chat' && msg.message) {
        sendChat(msg.message);
      } else if (msg.type === 'set_attribute' && msg.character_id && msg.name) {
        setCharacterAttribute(msg.character_id, msg.name, msg.value ?? '');
      }
    } catch (e) {
      console.warn('[vtmtools] Failed to parse message from app:', e);
    }
  });
```

- [ ] **Step 2: Add the `setCharacterAttribute` function**

Add this function after `sendChat` (around line 112):

```js
function setCharacterAttribute(characterId, attrName, value) {
  const model = window.Campaign.characters.get(characterId);
  if (!model) {
    console.warn('[vtmtools] setCharacterAttribute: character not found:', characterId);
    return;
  }
  const existing = model.attribs.find(a => a.get('name') === attrName);
  if (existing) {
    existing.save({ current: value });
  } else {
    model.attribs.create({ name: attrName, current: value, max: '' });
  }
  console.log(`[vtmtools] Set ${attrName}="${value}" on character ${characterId}`);
}
```

- [ ] **Step 3: Commit**

```bash
git add extension/content.js
git commit -m "feat(extension): handle set_attribute message for Roll20 write-back"
```

---

### Task 5: Restructure `Resonance.svelte`

**Files:**
- Modify: `src/tools/Resonance.svelte`

This task replaces the entire file. The changes are: new Roll20 state + listeners, character selector UI (card strip for medium/wide, dropdown button for narrow), result card moved above config steps, Apply button below result, roll button moved into the target section.

- [ ] **Step 1: Replace the file with the new implementation**

Replace `src/tools/Resonance.svelte` entirely with:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import ResonanceSlider from '$lib/components/ResonanceSlider.svelte';
  import TemperamentConfigComponent from '$lib/components/TemperamentConfig.svelte';
  import ResultCard from '$lib/components/ResultCard.svelte';
  import RollHistory from '$lib/components/RollHistory.svelte';
  import { publishEvent } from '../store/toolEvents';
  import type { RollConfig, ResonanceRollResult, HistoryEntry, Roll20Character, Roll20Attribute } from '../types';

  // ── Roll config ──────────────────────────────────────────────────────────────
  let config: RollConfig = $state({
    temperament: {
      diceCount: 1,
      takeHighest: true,
      negligibleMax: 5,
      fleetingMax: 8,
    },
    weights: {
      phlegmatic: 'neutral',
      melancholy: 'neutral',
      choleric: 'neutral',
      sanguine: 'neutral',
    }
  });

  let result: ResonanceRollResult | null = $state(null);
  let rolling = $state(false);
  let rollHistory: HistoryEntry[] = $state([]);
  let nextId = 0;

  // ── Roll20 state ─────────────────────────────────────────────────────────────
  let connected      = $state(false);
  let characters     = $state<Roll20Character[]>([]);
  let selectedCharId = $state<string | null>(null);
  let selectorOpen   = $state(false);
  let applyState     = $state<'idle' | 'applying' | 'applied'>('idle');

  const selectedChar = $derived(characters.find(c => c.id === selectedCharId) ?? null);

  $effect(() => {
    invoke<boolean>('get_roll20_status').then(s => { connected = s; });
    invoke<Roll20Character[]>('get_roll20_characters').then(c => { characters = c; });

    const unlisteners = [
      listen<void>('roll20://connected',    () => { connected = true; }),
      listen<void>('roll20://disconnected', () => { connected = false; selectedCharId = null; }),
      listen<Roll20Character[]>('roll20://characters-updated', (e) => { characters = e.payload; }),
    ];
    return () => { unlisteners.forEach(p => p.then(u => u())); };
  });

  // ── Helpers ──────────────────────────────────────────────────────────────────
  function attrText(attributes: Roll20Attribute[], name: string): string {
    return attributes.find(a => a.name === name)?.current ?? '';
  }

  // ── Resonance probability math ───────────────────────────────────────────────
  const WEIGHT_MULT: Record<string, number> = {
    impossible:        0,
    extremelyUnlikely: 0.1,
    unlikely:          0.5,
    neutral:           1.0,
    likely:            2.0,
    extremelyLikely:   4.0,
    guaranteed:        Infinity,
  };

  const RES_TYPES = [
    { key: 'phlegmatic', label: 'Phlegmatic', base: 0.25 },
    { key: 'melancholy',  label: 'Melancholy',  base: 0.25 },
    { key: 'choleric',    label: 'Choleric',    base: 0.25 },
    { key: 'sanguine',    label: 'Sanguine',    base: 0.25 },
  ] as const;

  function calcResProbs(w: typeof config.weights): number[] {
    const mults = RES_TYPES.map(t => WEIGHT_MULT[w[t.key]] ?? 1.0);
    const gIdx = mults.findIndex(m => !isFinite(m));
    if (gIdx >= 0) return RES_TYPES.map((_, i) => (i === gIdx ? 100 : 0));
    const weighted = RES_TYPES.map((t, i) => t.base * mults[i]);
    const total = weighted.reduce((a, b) => a + b, 0);
    if (total === 0) return [25, 25, 25, 25];
    const raw = weighted.map(v => (v / total) * 100);
    const rounded = raw.map(v => Math.round(v));
    const diff = 100 - rounded.reduce((a, b) => a + b, 0);
    const maxIdx = rounded.indexOf(Math.max(...rounded));
    rounded[maxIdx] += diff;
    return rounded;
  }

  const resProbs = $derived(calcResProbs(config.weights));

  // ── Actions ──────────────────────────────────────────────────────────────────
  async function roll() {
    rolling = true;
    result = null;
    try {
      result = await invoke<ResonanceRollResult>('roll_resonance', { config });
      if (result) {
        rollHistory = [{ id: nextId++, timestamp: new Date(), result }, ...rollHistory].slice(0, 100);
        publishEvent({
          type: 'resonance_result',
          payload: {
            temperament: result.temperament,
            resonanceType: result.resonanceType,
            isAcute: result.isAcute,
            dyscrasiaName: result.dyscrasia?.name ?? null,
          }
        });
      }
    } finally {
      rolling = false;
    }
  }

  async function applyToCharacter() {
    if (!result?.resonanceType || !selectedCharId) return;
    applyState = 'applying';
    try {
      await invoke('set_roll20_attribute', {
        characterId: selectedCharId,
        name: 'resonance',
        value: result.resonanceType,
      });
      applyState = 'applied';
      setTimeout(() => { applyState = 'idle'; }, 1800);
    } catch {
      applyState = 'idle';
    }
  }

  function selectChar(id: string) {
    selectedCharId = id;
    selectorOpen = false;
  }
</script>

<div class="page">
  <h1 class="title">Resonance Roller</h1>
  <p class="subtitle">Configure the feeding conditions, then roll.</p>

  <div class="main-layout">
    <div class="steps-panel">

      <!-- ── Target character ── -->
      <section class="step target-step">
        <h3>Target character</h3>

        {#if !connected}
          <div class="r20-status r20-disconnected" style="opacity: 0.45">
            <span class="r20-dot"></span>
            <span>Not connected to Roll20</span>
          </div>
        {:else if characters.length === 0}
          <div class="r20-status r20-empty">
            No characters loaded —
            <button class="link-btn" onclick={() => invoke('refresh_roll20_data')}>refresh</button>
          </div>
        {:else}
          <!-- Medium / wide: horizontal wrapping card strip -->
          <div class="char-strip">
            {#each characters as char (char.id)}
              {@const clan = attrText(char.attributes, 'clan')}
              {@const res  = attrText(char.attributes, 'resonance')}
              <button
                class="char-card"
                class:char-card--selected={char.id === selectedCharId}
                data-res={res || null}
                onclick={() => selectChar(char.id)}
              >
                <span class="char-name">{char.name}</span>
                {#if clan}<span class="char-clan">{clan}</span>{/if}
                {#if res}<span class="char-res">{res}</span>{/if}
              </button>
            {/each}
          </div>

          <!-- Narrow: compact selector button + dropdown -->
          <div class="char-selector-narrow">
            <button
              class="selector-btn"
              class:selector-btn--active={!!selectedChar}
              onclick={() => { selectorOpen = !selectorOpen; }}
            >
              {#if selectedChar}
                <span class="sel-dot"></span>
                <span class="sel-name">{selectedChar.name}</span>
                <span class="sel-clan">{attrText(selectedChar.attributes, 'clan')}</span>
              {:else}
                <span class="sel-placeholder">Choose character…</span>
              {/if}
              <span class="sel-chevron" class:open={selectorOpen}>⌄</span>
            </button>

            {#if selectorOpen}
              <button class="selector-backdrop" onclick={() => { selectorOpen = false; }} aria-label="Close picker"></button>
              <div class="selector-dropdown">
                <div class="dropdown-header">Select character</div>
                {#each characters as char (char.id)}
                  {@const clan = attrText(char.attributes, 'clan')}
                  {@const res  = attrText(char.attributes, 'resonance')}
                  <button
                    class="drop-item"
                    class:drop-item--selected={char.id === selectedCharId}
                    onclick={() => selectChar(char.id)}
                  >
                    <div class="drop-item-body">
                      <span class="drop-name">{char.name}</span>
                      {#if clan}<span class="drop-clan">{clan}</span>{/if}
                      {#if res}<span class="drop-res">{res}</span>{/if}
                    </div>
                    {#if char.id === selectedCharId}<span class="drop-check">✓</span>{/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <div class="roll-area">
          <button class="roll-btn" onclick={roll} disabled={rolling}>
            {rolling ? 'Rolling…' : '⚀ Roll'}
          </button>
        </div>
      </section>

      <!-- ── Result + Apply (above config) ── -->
      {#if result}
        <ResultCard {result} />
        {#if selectedCharId && result.resonanceType}
          <div class="apply-row">
            <button
              class="apply-btn"
              class:applied={applyState === 'applied'}
              onclick={applyToCharacter}
              disabled={applyState !== 'idle'}
            >
              {applyState === 'applying' ? 'Applying…'
               : applyState === 'applied' ? '✓ Applied'
               : `✓ Apply to ${selectedChar?.name ?? 'character'}`}
            </button>
          </div>
        {/if}
      {/if}

      <!-- ── Config steps ── -->
      <section class="step">
        <h3>1. Temperament dice</h3>
        <TemperamentConfigComponent
          diceCount={config.temperament.diceCount}
          takeHighest={config.temperament.takeHighest}
          negligibleMax={config.temperament.negligibleMax}
          fleetingMax={config.temperament.fleetingMax}
          onDiceCountChange={(n) => (config.temperament.diceCount = n)}
          onTakeHighestChange={(b) => (config.temperament.takeHighest = b)}
          onNegligibleMaxChange={(n) => (config.temperament.negligibleMax = n)}
          onFleetingMaxChange={(n) => (config.temperament.fleetingMax = n)}
        />
      </section>

      <section class="step">
        <h3>2. Resonance type odds</h3>
        <ResonanceSlider
          label="Phlegmatic"
          value={config.weights.phlegmatic}
          onChange={(v) => (config.weights.phlegmatic = v)}
        />
        <ResonanceSlider
          label="Melancholy"
          value={config.weights.melancholy}
          onChange={(v) => (config.weights.melancholy = v)}
        />
        <ResonanceSlider
          label="Choleric"
          value={config.weights.choleric}
          onChange={(v) => (config.weights.choleric = v)}
        />
        <ResonanceSlider
          label="Sanguine"
          value={config.weights.sanguine}
          onChange={(v) => (config.weights.sanguine = v)}
        />

        <div class="res-probs">
          <div class="res-bar">
            {#each RES_TYPES as type, i}
              {#if resProbs[i] > 0}
                <div
                  class="res-seg res-seg-{type.key}"
                  style="width:{resProbs[i]}%"
                  title="{type.label}: {resProbs[i]}%"
                ></div>
              {/if}
            {/each}
          </div>
          <div class="res-legend">
            {#each RES_TYPES as type, i}
              <div class="leg-item {resProbs[i] === 0 ? 'leg-zero' : ''}">
                <span class="leg-dot leg-dot-{type.key}"></span>
                <span class="leg-name">{type.label}</span>
                <span class="leg-pct">{resProbs[i]}%</span>
              </div>
            {/each}
          </div>
        </div>
      </section>

    </div>

    <!-- ── History (unchanged) ── -->
    <div class="history-panel">
      <RollHistory entries={rollHistory} />
    </div>
  </div>
</div>

<style>
  .page {
    width: 100%;
    container-type: inline-size;
    container-name: resonance-page;
  }
  .title    { color: var(--accent); font-size: 1.8rem; margin-bottom: 0.25rem; }
  .subtitle { color: var(--text-secondary); font-size: 0.9rem; margin-bottom: 1.5rem; }

  /* ── Stacked layout (default / narrow) ── */
  .main-layout { display: flex; flex-direction: column; gap: 1.5rem; align-items: stretch; }
  .steps-panel { display: flex; flex-direction: column; gap: 1.5rem; }
  .step {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 6px;
    padding: 1rem 1.25rem;
  }
  h3 {
    color: var(--text-label); font-size: 0.9rem; text-transform: uppercase;
    letter-spacing: 0.08em; margin: 0 0 0.75rem;
  }

  /* ── Roll20 connection states ── */
  .r20-status {
    font-size: 0.8rem; display: flex; align-items: center;
    gap: 0.4rem; padding: 0.35rem 0; margin-bottom: 0.75rem;
  }
  .r20-disconnected { color: var(--text-muted); }
  .r20-empty        { color: var(--text-secondary); }
  .r20-dot {
    width: 0.45rem; height: 0.45rem; border-radius: 50%;
    background: var(--text-muted); flex-shrink: 0;
  }
  .link-btn {
    background: none; border: none; color: var(--accent); cursor: pointer;
    font-family: inherit; font-size: inherit; padding: 0; text-decoration: underline;
  }

  /* ── Character card strip (shown at ≥30rem) ── */
  .char-strip {
    display: none;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-bottom: 0.75rem;
  }
  .char-card {
    background: var(--bg-raised);
    border: 1px solid var(--border-surface);
    border-left: 3px solid var(--border-surface);
    border-radius: 5px;
    padding: 0.4rem 0.65rem;
    display: flex; flex-direction: column; gap: 0.1rem;
    cursor: pointer; text-align: left;
    transition: border-color 0.15s, background 0.15s, box-shadow 0.2s;
    font-family: inherit;
    box-sizing: border-box;
  }
  .char-card:hover { border-color: var(--border-active); }
  .char-card--selected {
    border-color: var(--accent);
    background: var(--bg-active);
    box-shadow: 0 0 10px #cc222225, inset 0 0 12px #cc222210;
  }
  /* Resonance-colored left accent bar */
  .char-card[data-res="Phlegmatic"] { border-left-color: #3d6b88; }
  .char-card[data-res="Melancholy"] { border-left-color: #6a3d80; }
  .char-card[data-res="Choleric"]   { border-left-color: var(--accent-amber); }
  .char-card[data-res="Sanguine"]   { border-left-color: var(--accent); }
  .char-name { font-size: 0.8rem; color: var(--text-primary); font-weight: bold; }
  .char-clan { font-size: 0.7rem; color: var(--text-secondary); }
  .char-res  { font-size: 0.65rem; color: var(--accent); }

  /* ── Narrow compact selector (hidden at ≥30rem) ── */
  .char-selector-narrow { display: block; position: relative; margin-bottom: 0.75rem; }
  .selector-btn {
    width: 100%; background: var(--bg-raised); border: 1px solid var(--border-surface);
    border-radius: 5px; padding: 0.45rem 0.65rem;
    display: flex; align-items: center; gap: 0.45rem;
    cursor: pointer; font-family: inherit; text-align: left;
    box-sizing: border-box; transition: border-color 0.15s;
  }
  .selector-btn--active { border-color: var(--accent); background: var(--bg-active); }
  .sel-dot {
    width: 0.45rem; height: 0.45rem; border-radius: 50%;
    background: var(--accent); flex-shrink: 0;
  }
  .sel-name        { font-size: 0.8rem; color: var(--text-primary); font-weight: bold; flex: 1; }
  .sel-clan        { font-size: 0.7rem; color: var(--text-secondary); }
  .sel-placeholder { font-size: 0.78rem; color: var(--text-muted); flex: 1; }
  .sel-chevron     { color: var(--text-label); font-size: 0.75rem; flex-shrink: 0; transition: transform 0.2s; }
  .sel-chevron.open { transform: rotate(180deg); }

  .selector-backdrop {
    position: fixed; inset: 0; z-index: 10;
    background: rgba(0, 0, 0, 0.4); border: none; cursor: default;
  }
  .selector-dropdown {
    position: absolute; top: calc(100% + 0.3rem); left: 0; right: 0;
    background: var(--bg-input); border: 1px solid var(--border-active);
    border-radius: 6px; padding: 0.4rem; z-index: 20;
    box-shadow: 0 8px 24px rgba(0,0,0,0.7), 0 0 0 1px #cc222233;
    display: flex; flex-direction: column; gap: 0.25rem;
  }
  .dropdown-header {
    font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.08em;
    color: var(--text-label); padding: 0.2rem 0.3rem 0.35rem;
    border-bottom: 1px solid var(--border-faint); margin-bottom: 0.1rem;
  }
  .drop-item {
    background: var(--bg-card); border: 1px solid var(--border-card);
    border-radius: 4px; padding: 0.4rem 0.55rem;
    display: flex; align-items: center; gap: 0.5rem;
    cursor: pointer; text-align: left; font-family: inherit;
    width: 100%; box-sizing: border-box;
    transition: border-color 0.12s, background 0.12s;
  }
  .drop-item:hover { border-color: var(--border-surface); background: var(--bg-raised); }
  .drop-item--selected { border-color: var(--accent); background: var(--bg-active); }
  .drop-item-body { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 0.05rem; }
  .drop-name  { font-size: 0.78rem; color: var(--text-primary); font-weight: bold; }
  .drop-clan  { font-size: 0.65rem; color: var(--text-secondary); }
  .drop-res   { font-size: 0.62rem; color: var(--accent); }
  .drop-check { font-size: 0.75rem; color: var(--accent); flex-shrink: 0; }

  /* ── Roll button ── */
  .roll-area { display: flex; justify-content: center; margin-top: 0.5rem; }
  .roll-btn {
    padding: 0.75rem 2.5rem;
    background: var(--bg-active); border: 2px solid var(--border-active);
    color: var(--accent); font-size: 1.1rem; font-family: 'Georgia', serif;
    cursor: pointer; border-radius: 4px;
    transition: background 0.2s, box-shadow 0.2s; letter-spacing: 0.05em;
  }
  .roll-btn:hover:not(:disabled) { background: #5a0808; box-shadow: 0 0 16px #cc222244; }
  .roll-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* ── Apply button ── */
  .apply-row { display: flex; justify-content: flex-end; }
  .apply-btn {
    padding: 0.45rem 1.2rem;
    background: var(--bg-sunken);
    border: 1.5px solid var(--border-surface);
    color: var(--accent-amber);
    font-size: 0.85rem; font-family: 'Georgia', serif;
    cursor: pointer; border-radius: 4px;
    transition: background 0.15s, border-color 0.15s, color 0.3s;
  }
  .apply-btn:hover:not(:disabled) {
    border-color: var(--accent-amber);
    background: var(--bg-raised);
  }
  .apply-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .apply-btn.applied {
    border-color: #4caf50;
    color: #4caf50;
    background: #0f2a0f;
  }

  /* ── History panel ── */
  .history-panel {
    background: var(--bg-card); border: 1px solid var(--border-card);
    border-radius: 6px; padding: 0.85rem 0.9rem;
    display: flex; flex-direction: column; max-height: 20rem;
  }

  /* ── Resonance probability visualization (unchanged) ── */
  .res-probs {
    margin-top: 0.75rem; padding-top: 0.65rem;
    border-top: 1px solid var(--border-faint);
    display: flex; flex-direction: column; gap: 0.5rem;
  }
  .res-bar {
    display: flex; height: 0.5rem; border-radius: 3px;
    overflow: hidden; gap: 1px; background: var(--bg-sunken);
  }
  .res-seg { height: 100%; transition: width 0.3s ease; border-radius: 1px; }
  .res-seg-phlegmatic { background: #3d6b88; }
  .res-seg-melancholy  { background: #6a3d80; }
  .res-seg-choleric    { background: var(--accent-amber); }
  .res-seg-sanguine    { background: var(--accent); }
  .res-legend { display: flex; gap: 0.75rem; flex-wrap: wrap; }
  .leg-item { display: flex; align-items: center; gap: 0.3rem; font-size: 0.7rem; transition: opacity 0.2s; }
  .leg-item.leg-zero { opacity: 0.3; }
  .leg-dot { width: 0.5rem; height: 0.5rem; border-radius: 50%; flex-shrink: 0; }
  .leg-dot-phlegmatic { background: #3d6b88; }
  .leg-dot-melancholy  { background: #6a3d80; }
  .leg-dot-choleric    { background: var(--accent-amber); }
  .leg-dot-sanguine    { background: var(--accent); }
  .leg-name { color: var(--text-label); }
  .leg-pct  { color: var(--text-primary); font-weight: 700; min-width: 2.2rem; }

  /* ── Responsive breakpoints ── */

  /* ≥30rem: show card strip, hide narrow selector */
  @container resonance-page (min-width: 30rem) {
    .char-strip           { display: flex; }
    .char-selector-narrow { display: none; }
  }

  /* ≥42rem: side-by-side layout with history panel */
  @container resonance-page (min-width: 42rem) {
    .main-layout   { flex-direction: row; gap: 2rem; align-items: flex-start; }
    .steps-panel   { flex: 1; min-width: 0; }
    .history-panel {
      width: 15rem; flex-shrink: 0;
      position: sticky; top: 1rem;
      max-height: calc(100vh - 3rem);
    }
  }
</style>
```

- [ ] **Step 2: Run type-check**

```bash
npm run check
```

Expected: no errors. If svelte-check warns about `Roll20Attribute` being imported but only used as a type parameter in `attrText`, that is fine — it is used as a type annotation.

- [ ] **Step 3: Commit**

```bash
git add src/tools/Resonance.svelte
git commit -m "feat(resonance): add Roll20 character selector and resonance write-back UI"
```

---

## Self-Review

**Spec coverage:**
- ✅ `SetAttribute` in `OutboundMsg` → Task 1
- ✅ `set_roll20_attribute` Tauri command → Task 2
- ✅ Register in `lib.rs` → Task 3
- ✅ Extension `set_attribute` handler + `setCharacterAttribute` function → Task 4
- ✅ Roll20 state + listeners in `Resonance.svelte` → Task 5
- ✅ Character selector (card strip + narrow dropdown) → Task 5
- ✅ Disconnected state (dimmed, "Not connected") → Task 5
- ✅ Empty state ("No characters loaded — refresh") → Task 5
- ✅ Result card moved above config → Task 5
- ✅ Apply button (only when `selectedCharId && result.resonanceType`) → Task 5
- ✅ Responsive: narrow (<30rem) dropdown, medium/wide (≥30rem) strip, side-by-side (≥42rem) → Task 5
- ✅ `selectedCharId` cleared on disconnect → Task 5

**Placeholder scan:** None found.

**Type consistency:**
- `set_roll20_attribute` — consistent across Rust command, Tauri `invoke_handler`, and frontend `invoke('set_roll20_attribute', ...)`.
- `OutboundMsg::SetAttribute { character_id, name, value }` — field names are snake_case in Rust, serialise to `character_id`/`name`/`value` in JSON, matched by `msg.character_id`/`msg.name`/`msg.value` in `content.js`.
- `Roll20Character`, `Roll20Attribute` — imported from `../types` which already defines both interfaces.
- `attrText` — defined in Task 5 script, called only in Task 5 template. No cross-task dependency issues.
- `applyState` — tri-state `'idle' | 'applying' | 'applied'` used consistently in script, template button text, `class:applied` binding, and `disabled={applyState !== 'idle'}`.

**Design refinements applied:**
- Resonance-colored left accent bar on char cards via `data-res` attribute + CSS attribute selectors. Colors reuse the res-seg palette (`#3d6b88`, `#6a3d80`, `--accent-amber`, `--accent`).
- Selected card glow (`box-shadow: 0 0 10px ... inset 0 0 12px ...`) matches ResultCard's acute text-shadow intensity.
- Apply button uses `--accent-amber` (matching Export button palette), with green `#4caf50` flash only for the brief `applied` confirmation state.
- Dropdown backdrop dims page (`rgba(0,0,0,0.4)`) matching approved brainstorming mockup.
- Disconnected status dimmed to `opacity: 0.45` matching Campaign's disabled btn style.
