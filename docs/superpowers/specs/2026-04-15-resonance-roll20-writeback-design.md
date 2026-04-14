# Resonance → Roll20 Write-back

**Date:** 2026-04-15
**Status:** Draft

## Summary

Add character targeting and attribute write-back to the Resonance Roller. After rolling, the GM can select a Roll20 character and push the resulting resonance value directly to their character sheet with one click.

---

## Data Flow

```
Resonance.svelte
  → invoke('set_roll20_attribute', { characterId, name: 'resonance', value })
    → Rust: OutboundMsg::SetAttribute over WebSocket
      → extension content.js: receives { type: 'set_attribute', character_id, name, value }
        → model.attribs.find(name) → existing.save({ current: value })
           OR model.attribs.create({ name, current: value, max: '' })
```

The reverse path (Roll20 → app) is unchanged — character data continues to arrive via existing `characters-updated` events.

---

## Backend Changes

### `src-tauri/src/roll20/types.rs`

Add a new variant to `OutboundMsg`:

```rust
SetAttribute {
    character_id: String,
    name: String,
    value: String,
}
```

Serialises to:
```json
{ "type": "set_attribute", "character_id": "...", "name": "resonance", "value": "Choleric" }
```

### `src-tauri/src/roll20/commands.rs`

Add one new Tauri command:

```rust
#[tauri::command]
pub async fn set_roll20_attribute(
    character_id: String,
    name: String,
    value: String,
    conn: State<'_, Roll20Conn>,
) -> Result<(), String>
```

Behaviour: serialise `OutboundMsg::SetAttribute` and send over the outbound channel. No-op if no extension is connected (same pattern as `send_roll20_chat`).

### `src-tauri/src/lib.rs`

Register `set_roll20_attribute` in the Tauri command handler list.

---

## Extension Changes

### `extension/content.js`

Add a new message handler branch in the `ws.addEventListener('message', ...)` block:

```js
} else if (msg.type === 'set_attribute' && msg.character_id && msg.name) {
    setCharacterAttribute(msg.character_id, msg.name, msg.value ?? '');
}
```

Add the implementation function:

```js
function setCharacterAttribute(characterId, attrName, value) {
    const model = window.Campaign.characters.get(characterId);
    if (!model) return;
    const existing = model.attribs.find(a => a.get('name') === attrName);
    if (existing) {
        existing.save({ current: value });
    } else {
        model.attribs.create({ name: attrName, current: value, max: '' });
    }
}
```

---

## Frontend Changes

### `src/tools/Resonance.svelte`

All changes are confined to this single file.

#### New state

```ts
let connected      = $state(false);
let characters     = $state<Roll20Character[]>([]);
let selectedCharId = $state<string | null>(null);
let selectorOpen   = $state(false);   // narrow mode dropdown
let applying       = $state(false);
```

#### New Roll20 listeners (in `$effect`)

Mirror what `Campaign.svelte` already does:
- `invoke('get_roll20_status')` → seed `connected`
- `invoke('get_roll20_characters')` → seed `characters`
- Listen on `roll20://connected`, `roll20://disconnected`, `roll20://characters-updated`
- When disconnected: clear `selectedCharId`

#### New `applyToCharacter()` function

```ts
async function applyToCharacter() {
    if (!result?.resonanceType || !selectedCharId) return;
    applying = true;
    try {
        await invoke('set_roll20_attribute', {
            characterId: selectedCharId,
            name: 'resonance',
            value: result.resonanceType,
        });
    } finally {
        applying = false;
    }
}
```

#### Layout restructure

The page gains a **target section** at the top rendered from a `$derived` variable:

```ts
const selectedChar = $derived(
    characters.find(c => c.id === selectedCharId) ?? null
);
```

**Target section renders four states:**

1. **Disconnected** — section visible but dimmed; selector button grayed out, label: "Not connected to Roll20".
2. **Connected, no characters yet** — small inline note: "No characters loaded — try refreshing" with a Refresh button (calls `refresh_roll20_data`).
3. **Connected, wide/medium** (`container ≥ 30rem`) — horizontal wrapping card strip. Each card shows: name, clan (`attrText(char.attributes, 'clan')`), current resonance (`attrText(char.attributes, 'resonance')`). Selected card gets accent border + `--bg-active` fill.
4. **Connected, narrow** (`container < 30rem`) — compact selector button (shows selected char name+clan or "Choose character…" placeholder). Clicking opens a dropdown panel listing full character cards. Clicking outside or selecting closes it. Roll button is dimmed until a character is selected.

**Roll button** moves from its current position to sit inside/adjacent to the target section.

**Result card** moves to render directly below the target section (above the config steps), so it's always visible without scrolling after a roll.

**Apply button** appears below the result card when `result !== null && selectedCharId !== null`. Label: `applying ? 'Applying…' : '✓ Apply to [char name]'`. Disabled while `applying`.

#### Responsive breakpoints (container queries)

Uses the existing `container-name: resonance-page` already on `.page`.

| Container width | Character selector behaviour |
|---|---|
| `< 30rem` | Compact dropdown button |
| `≥ 30rem` | Horizontal wrapping card strip |
| `≥ 48rem` | Card strip + roll button + result all in one top card (wide layout) |

These align with the existing `42rem` breakpoint already in the file.

---

## What Does Not Change

- Resonance rolling logic (`roll_resonance` Tauri command, dice math) — untouched.
- Roll history — untouched.
- `Campaign.svelte` — untouched.
- All other tools — untouched.
- The `resonance` field written is always `result.resonanceType` (e.g. `"Choleric"`, `"Melancholy"`) — the exact string the sheet already uses based on the raw attribute data shown in the brief.

---

## Out of Scope

- Writing any attribute other than `resonance`.
- Bulk apply (applying to multiple characters at once).
- Confirmation dialogs or undo — a second roll + apply is the correction mechanism.
- Error surfacing beyond console — if the extension is present but the write silently fails, the next `characters-updated` event will reflect the real sheet state.
