# Roll20 Integration Design

**Date:** 2026-04-14  
**Status:** Approved  
**Scope:** Browser extension + Tauri WebSocket bridge + Campaign tool tab

---

## Overview

Add a "Campaign" tool to vtmtools that displays live VTM V5 character stats (Hunger, Health, Willpower, Humanity, Blood Potency) pulled from a Roll20 game session. A Chrome browser extension reads Roll20 character data and streams it to the desktop app over a local WebSocket connection. Updates are event-driven via Backbone.js change listeners on the Roll20 page.

---

## Repository Layout

New directories added to the existing monorepo:

```
vtmtools/
├── src/                          # existing Svelte frontend
│   └── tools/
│       └── Campaign.svelte       # new Campaign tool tab
├── src-tauri/
│   └── src/
│       ├── lib.rs                # add: spawn WebSocket server in setup()
│       └── roll20/
│           ├── mod.rs            # WebSocket server + in-memory character state
│           ├── types.rs          # Character, Attribute, WsMessage serde types
│           └── commands.rs       # Tauri commands: get_characters, send_chat, refresh
└── extension/                    # standalone Chrome extension (no bundler)
    ├── manifest.json
    ├── content.js                # content script injected into app.roll20.net
    ├── background.js             # MV3 service worker (minimal)
    └── icons/
        ├── icon16.png
        ├── icon48.png
        └── icon128.png
```

The extension uses no build tooling — plain JS content script. The Tauri app gains a new `roll20` module and one new tool Svelte component.

---

## Browser Extension

### Manifest (MV3, Chrome-first)

- `manifest_version: 3`
- `content_scripts` injected into `https://app.roll20.net/editor/*`
- `host_permissions`: `http://localhost:7423/*` (WebSocket connection to Tauri app)
- `permissions`: none beyond host permissions
- Firefox MV3 compatibility is straightforward — only the native messaging manifest would differ, but we are not using native messaging

### Content Script (`content.js`)

Runs inside the Roll20 editor page. Responsibilities:

1. **On load:** fetch all characters and their attributes, send full payload to app
2. **Backbone listeners:** subscribe to `change` events on all `window.Campaign.characters.models` entries; on any change, re-fetch that character's attributes and send an update
3. **WebSocket client:** maintains a persistent `ws://localhost:7423` connection; auto-reconnects with exponential backoff if the app is not running
4. **Manual refresh:** listens for `{type: "refresh"}` from the app and re-fetches all characters

Character attribute fetch: `GET /character/{id}/attributes` (Roll20 internal endpoint, same origin, no auth needed). Returns `[{name, current, max}]`.

Character list: `window.Campaign.characters.models` — array of Backbone model objects already in page memory. Each model's `model.get('controlledby')` field contains a comma-separated string of player IDs (or `"all"`). If non-empty, the character is treated as a PC; if empty, it is an NPC. This field is read directly from the Backbone model and included in the character payload.

### Background Service Worker (`background.js`)

Minimal. Only needed to respond to browser action clicks (future: open options page). Does not hold the WebSocket connection — that lives in the content script which persists as long as the Roll20 tab is open.

---

## Tauri Backend

### WebSocket Server (`src-tauri/src/roll20/mod.rs`)

- Library: `tokio-tungstenite` (adds to `Cargo.toml`)
- Binds on `127.0.0.1:7423` at app startup, inside `lib.rs` setup
- Accepts one connection at a time (only one Roll20 tab expected); logs and ignores additional connections
- On message received: parse JSON, update `Arc<Mutex<HashMap<String, Character>>>` in app state, emit Tauri event `roll20://characters-updated` to the frontend
- On disconnect: set connection status to disconnected, emit `roll20://disconnected`
- On connect: emit `roll20://connected`

### Types (`src-tauri/src/roll20/types.rs`)

```rust
pub struct Attribute { pub name: String, pub current: String, pub max: String }
pub struct Character  {
    pub id: String,
    pub name: String,
    pub controlled_by: String,  // empty = NPC, non-empty = PC
    pub attributes: Vec<Attribute>,
}

// Inbound from extension
pub enum InboundMsg {
    Characters { characters: Vec<Character> },
    CharacterUpdate { character: Character },
}

// Outbound to extension
pub enum OutboundMsg {
    Refresh,
    SendChat { message: String },
}
```

### Tauri Commands (`src-tauri/src/roll20/commands.rs`)

| Command | Description |
|---|---|
| `get_roll20_characters` | Returns current `Vec<Character>` from in-memory state |
| `get_roll20_status` | Returns `"connected"` or `"disconnected"` |
| `send_roll20_chat(message: String)` | Sends `{type:"send_chat"}` to extension |
| `refresh_roll20_data` | Sends `{type:"refresh"}` to extension |

State is **in-memory only** — no SQLite. Character data is session-scoped and does not need to survive app restarts.

---

## Message Protocol

All messages are JSON over WebSocket. No framing beyond standard WebSocket frames.

**Extension → App:**

```jsonc
// Full read — sent on page load and manual refresh
{ "type": "characters", "characters": [
    { "id": "abc123", "name": "Zara Okafor", "controlled_by": "-OAbc123player",
      "attributes": [{"name": "hunger", "current": "3", "max": "5"}, ...] }
]}

// Single character changed — sent on Backbone change event
{ "type": "character_update",
  "character": { "id": "abc123", "name": "Zara Okafor", "attributes": [...] } }
```

**App → Extension:**

```jsonc
{ "type": "refresh" }
{ "type": "send_chat", "message": "Rolls resonance: Melancholy (Intense)" }
```

---

## Svelte Frontend (`src/tools/Campaign.svelte`)

Registered in `src/tools.ts` as:
```ts
{ id: 'campaign', label: 'Campaign', icon: '🗺️',
  component: () => import('./tools/Campaign.svelte') }
```

### Connected state

- Toolbar: green status dot + "Connected to Roll20", timestamp of last sync, Refresh button
- Character grid: CSS Grid `auto-fill minmax(220px, 1fr)`, `align-items: start` (matches existing DyscrasiaManager pattern)
- One card per character

### Character card

- **Header:** character name + PC/NPC badge (colour-coded)
- **Body:** five stat rows:
  - Hunger — 5 filled/empty dots, crimson (`--accent`)
  - Health — filled/empty/aggravated boxes (aggravated = diagonal stripe)
  - Willpower — filled/empty boxes, indigo
  - Humanity — 10 dots, indigo
  - Blood Potency — single number, amber (`--accent-amber`)
- **Footer:** per-card "raw attrs ▾" toggle that expands a monospace attribute dump panel
- The raw panel shows all attribute name/value pairs from Roll20 as received — used to verify attribute name mapping on first setup

### Disconnected state

- Grey status dot + "Not connected"
- Full-width banner explaining how to connect (open Roll20 in Chrome with extension enabled)
- Refresh button disabled

### Attribute name mapping

Phase 1 builds the raw panel with all attributes visible. After verifying names against a live Roll20 session, Phase 2 wires up the display layer. Assumed standard VTM V5 Roll20 sheet attribute names (to be verified):

| Stat | Expected attribute name |
|---|---|
| Hunger | `hunger` |
| Health (current) | `health` |
| Health (aggravated) | `health_aggravated` |
| Willpower | `willpower` |
| Humanity | `humanity` |
| Blood Potency | `blood_potency` |

If the actual names differ, only the display mapping constants in `Campaign.svelte` need updating — the protocol and backend pass all attributes through unchanged.

---

## Extension Installation (Development)

1. Open `chrome://extensions`
2. Enable "Developer mode"
3. "Load unpacked" → select the `extension/` directory
4. Open a Roll20 game — the extension activates automatically on `app.roll20.net/editor/*`
5. vtmtools must be running for the WebSocket connection to succeed; if not, the extension retries silently

---

## Out of Scope for This Feature

- Firefox support (content script is compatible; native messaging manifest delta is small but deferred)
- Session history / persisting character state between app restarts
- Sorting or filtering the character grid
- Roll20 chat posting UI in the Campaign tab (protocol supports `send_chat`; UI for it is deferred)
- Distribution via Chrome Web Store

---

## Phase Summary

| Phase | Deliverable |
|---|---|
| Phase 1 | End-to-end pipeline working: extension connects, all attributes arrive in app, raw panel shows them. Character cards display stat values as plain text. |
| Phase 2 | Formatted character cards: Hunger dots, Health/Willpower boxes, Humanity dots, Blood Potency number — wired to verified attribute names. |
