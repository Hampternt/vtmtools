# Roll20 VTT Bundle Analysis

> **Source file**: `vtt.bundle.0d1d45fcfc0cc967c025.js` (~20 MB, ~55,000 lines, webpack-minified)
>
> **Analysis date**: 2026-04-18
>
> **Confidence note**: This analysis was reverse-engineered from a minified webpack bundle. String literals and API paths are high-confidence. Internal class semantics and field relationships are inferred from variable names, grep proximity, and structural patterns. Nothing here is official Roll20 documentation.
>
> **Staleness warning**: The bundle filename contains a webpack content hash (`0d1d45fcfc0cc967c025`). This is a point-in-time snapshot of Roll20's Jumpgate client. Roll20 deploys frequently — globals, Firestore collection paths, and minified class names can change between builds. Treat internal names (e.g., class `Bg`, module IDs) as ephemeral. The public-facing API surface (`window.Campaign`, `d20.*`) has been stable across builds historically, but verify against the live page before relying on anything documented here.

---

## 1. What the Bundle IS

The VTT bundle is Roll20's entire Jumpgate (next-gen) client application, compiled into a single webpack chunk. It contains every subsystem needed to run a virtual tabletop session in the browser.

### Tech Stack

| Layer | Technology | Evidence |
|---|---|---|
| Rendering engine | **Babylon.js** + WebGL | `VTTEngine.instance`, `tabletop.engine.babylonEngine`, `tabletop.scene`, shader code for grids/tinting/lighting |
| UI framework | **Vue.js 3** | `createApp`, `defineComponent`, `ref()`, `reactive()`, `computed()`, `watch()` (~275 occurrences) |
| Data models | **Backbone.js** | `Backbone.Model.extend`, `Backbone.Collection`, `.get()` / `.set()` / `.extend()` (~25 references) |
| Backend data | **Firebase** (Auth + Firestore + Realtime Database) | `firebase.auth()`, `.collection()` (161x), `onSnapshot`, `query()` (1105x) |
| Real-time sync | **Firebase Realtime Database WebSocket** | Custom frame protocol, `wss://` transport with long-poll fallback |
| Module system | **Webpack** | `__webpack_require__`, `webpackChunk`, chunk lazy-loading |

### Architectural Layers (top to bottom)

```
┌─────────────────────────────────────────────────────────────────┐
│  Vue.js 3 UI        Sidebar, chat, journal, settings panels    │
├─────────────────────────────────────────────────────────────────┤
│  Backbone Models     Characters, Pages, Tokens, Handouts,      │
│  & Collections       Macros, Decks, RollableTables, Players    │
├─────────────────────────────────────────────────────────────────┤
│  Babylon.js Canvas   Map tiles, token meshes, dynamic lighting,│
│  (WebGL)             fog of war, grid shaders, particle FX     │
├─────────────────────────────────────────────────────────────────┤
│  Firebase Layer      Auth, Firestore collections, Realtime DB  │
│                      WebSocket sync, offline persistence       │
├─────────────────────────────────────────────────────────────────┤
│  REST APIs           /editor/*, /audio_library/*, /compendium/*│
│                      /sheetsandbox/*, /campaign/*, S3 uploads  │
└─────────────────────────────────────────────────────────────────┘
```

### Key Subsystem Sizes (by grep hit count)

| Subsystem | Indicator pattern | Hits | Notes |
|---|---|---|---|
| Character/attributes | `character\|attribute` | 460 | Core data model |
| Token/campaign/page | `token\|Campaign\|page` | 574 | Canvas objects |
| Dice/rolling | `dice\|roll\|initiative` | 571 | Rolling engine |
| Effects/aura/tint | `aura\|effect\|particle\|tint\|statusmarker` | 324 | Visual FX |
| Vue.js components | `createApp\|defineComponent\|ref(\|reactive(` | 275 | UI layer |
| Canvas/rendering | `Emitter\|Particle\|weather` | 183 | Particle system |
| Firebase queries | `query()` | 1105 | Data layer backbone |
| Audio/jukebox | `jukebox\|soundclip\|playlist\|audio_library` | 95 | Music system |
| Handouts/tables/macros | `handout\|rollabletable\|macro\|deck` | 53 | Content objects |
| Measurement/ping/ruler | `ping\|pointer\|ruler\|measure\|waypoint` | 559 | Tabletop tools |

---

## 2. Global Objects Exposed to the Browser

These `window`-level globals are the primary surface area that a browser extension (like ours) can reach. This is how the extension in `extension/content.js` already interacts with Roll20.

### `window.Campaign` (Backbone Model)

The campaign is the top-level container. Key collections on it:

| Property | Type | What it holds |
|---|---|---|
| `.characters` | Backbone Collection | All characters in the campaign |
| `.players` | Backbone Collection | Connected players |
| `.pages` | (inferred) | Map/scene pages |
| `.rollabletables` | (inferred) | Random tables |

### `d20.*` Namespace

| Global | Purpose |
|---|---|
| `d20.Campaign` | Same as `window.Campaign` |
| `d20.textchat` | Chat system: `.doChatInput(msg)`, `.incoming()` |
| `d20.engine` | Canvas engine: `.fog`, `.drawshape`, lighting controls |
| `d20.compendium` | Compendium browser: `.compendiumBase`, `.shortName` |
| `d20.journal` | Journal/handout management |
| `d20.canvas_overlay` | Grid overlay: `.activeHexGrid`, `.activeIsometricGrid`, `.gl` (WebGL context) |

### Window Flags

| Variable | Type | Meaning |
|---|---|---|
| `window.campaign_id` | string | Current campaign ID (64 occurrences in bundle) |
| `window.IS_GM` | boolean | Whether current user is GM |
| `window.IS_SPECTATOR` | boolean | Spectator mode |
| `window.IS_ADMIN` | boolean | Admin privileges |
| `window.IS_EMBEDDED` | boolean | Discord Activity embedded mode |
| `window.d20_account_id` | string | Current user's account ID |
| `window.currentPlayer` | object | Current player model |
| `window.currentUserId` | string | Current user ID |
| `window.PRIMARY_CHARSHEET_NAME` | string | Primary character sheet template name |
| `window.SECONDARY_CHARSHEET_NAME` | string | Secondary sheet template |
| `window.FIREBASE_ROOT` | string | Firebase database root path |
| `window.campaign_storage_path` | string | Firebase storage path for this campaign |
| `window.GNTKN` | string | Game/session token |
| `window.MAX_UPLOAD_SIZE_MB` | number | Upload size limit |
| `window.DUNGEON_SCRAWL_URL` | string | Dungeon Scrawl integration URL |

---

## 3. Data Models (Backbone)

### Character Model

Accessed via `window.Campaign.characters.get(id)` or `.models` array.

| Field | Access | Description |
|---|---|---|
| `id` | `.id` | Unique character ID |
| `name` | `.get("name")` | Display name (223 occurrences) |
| `avatar` | `.get("avatar")` | Avatar image URL (44x) |
| `bio` | `.get("bio")` | Biography/description HTML |
| `gmnotes` | `.get("gmnotes")` | GM-only notes (6x) |
| `controlledby` | `.get("controlledby")` | Comma-separated player IDs (86x) |
| `inplayerjournals` | `.get("inplayerjournals")` | Player visibility list (22x) |
| `charactersheetname` | `.get("charactersheetname")` | Sheet template identifier (6x) |
| `defaulttoken` | `.get("defaulttoken")` | Default token JSON (7x) |
| `vault_character_id` | `.get("vault_character_id")` | External vault link |
| `nexus_character_id` | `.get("nexus_character_id")` | Demiplane nexus link |

**Attributes sub-collection** (`character.attribs`):

Each attribute is a Backbone model with:
- `.get("name")` — attribute name (e.g., `"hunger"`, `"hp"`, `"strength"`)
- `.get("current")` — current value
- `.get("max")` — max value (optional)

Firebase storage: `char-attribs/char/{characterId}` collection.

### Token Model

Tokens are graphics on the canvas that can represent characters.

| Field | Access | Description |
|---|---|---|
| `represents` | `.get("represents")` | Character ID this token represents (85x) |
| `controlledby` | `.get("controlledby")` | Player control list |
| `layer` | `.get("layer")` | Layer: `"map"`, `"objects"`, `"gmlayer"`, `"lighting"`, `"foreground"` |
| `left`, `top` | `.get("left")` | Position on canvas |
| `width`, `height` | `.get("width")` | Dimensions (238x, 223x) |
| `bar1`, `bar2`, `bar3` | `.get("bar1_value")` | Resource bar values + `_max` |
| `statusmarkers` | `.get("statusmarkers")` | Status condition markers string (12x) |
| `tint_color` | `.get("tint_color")` | Token tint overlay |
| `bright_light`, `dim_light` | `.get("bright_light")` | Light emission radii |
| `vision` | `.get("vision")` | Character vision settings |

### Page Model

Pages represent maps/scenes.

| Field | Description |
|---|---|
| `height`, `width` | Page dimensions (in grid cells; multiply by 70 for pixels) |
| `grid_type` | `"grid"` (square), `"hex"`, `"hexr"` (rotated hex) |
| `grid_opacity` | Grid line transparency |
| `scale_number` + `scale_units` | Measurement calibration (e.g., 5ft per cell) |
| `fogofwar` | Classic fog enabled |
| `dynamic_lighting_enabled` | Dynamic lighting on/off |
| `daylight_mode_enabled` | Global illumination toggle |
| `thegraphics` | Collection of all graphics/tokens on this page |
| `windows` / `doors` | Door/portal models for dynamic lighting walls |

---

## 4. Firebase Data Architecture

### Firestore Collections (high confidence)

| Collection Path | Contents |
|---|---|
| `characters/{id}` | Character documents (name, avatar, bio, etc.) |
| `char-attribs/char/{characterId}` | Character attributes (name/current/max triples) |
| `char-blobs/{characterId}` | Character blob data (likely bio/gmnotes HTML) |
| `char-abils/char/{characterId}` | Character abilities/macros |

### Firebase Realtime Database

Used for real-time synchronization during play sessions:

| Path | What syncs |
|---|---|
| `{campaign_storage_path}/pages/{pageId}` | Page state |
| `{campaign_storage_path}/graphics` | Token positions/properties |
| `{campaign_storage_path}/texts` | Text objects on canvas |
| `{campaign_storage_path}/paths` | Drawn paths/shapes |
| `{campaign_storage_path}/dynamic_fog/masks/page/{pageId}` | Fog of war state |

### WebSocket Protocol

Firebase Realtime DB uses its own WebSocket frame protocol:

| Frame type | Code | Purpose |
|---|---|---|
| Control | `t: "c"` | Connection management |
| Data | `t: "d"` | Application data sync |
| Handshake | `t: "h"` | Initial connection setup (returns server timestamp, session ID) |
| Ping | `t: "p"` | Keep-alive (every 45 seconds) |
| Reset | `t: "r"` | Reconnect command |
| Shutdown | `t: "s"` | Server-initiated disconnect |
| Error | `t: "e"` | Error notification |

Max frame size: 16,384 bytes (multi-frame reassembly for larger messages).

### Auth Flow

Firebase Authentication with multiple sign-in methods:
- Email/password
- OAuth popup/redirect
- Anonymous auth
- Custom token (`signInWithCustomToken`)
- Multi-factor auth (MFA start/finalize flows)

Token stored as `Authorization: Bearer {idToken}` in REST requests.

---

## 5. REST API Endpoints

### Roll20 Editor APIs

| Endpoint | Method | Purpose |
|---|---|---|
| `/editor/ping` | GET | Server health check |
| `/editor/oauth_token` | GET | OAuth token management |
| `/editor/oauth/client_credentials_token` | GET | Client credentials flow |
| `/editor/analytics_report/` | POST | Analytics |
| `/editor/popout` | GET | Popout window |
| `/editor/popoutchat` | GET | Chat popout |
| `/editor/popoutjukebox` | GET | Jukebox popout |
| `/editor/startping/true\|false` | GET | Start/stop ping monitoring |
| `/editor/audiourl/bb` | GET | Audio URL fetch |

### Campaign & Content APIs

| Endpoint | Purpose |
|---|---|
| `/campaign/` | Campaign data |
| `/campaigns/updatecharsheets/{campaign_id}` | Update character sheets |
| `/characters/` | Character CRUD |
| `/compendium/compendium/getPages` | Compendium page fetch |
| `/compendium/compendium/globalsearch/{system}` | Compendium search |
| `/doroll` | Server-side dice roll |
| `/rollabletables/` | Rollable table management |
| `/decks/` | Card deck management |
| `/cardtrades/` | Card trading |

### Media & Assets

| Endpoint | Purpose |
|---|---|
| `/audio_library/upload` | Upload audio |
| `/audio_library/playlists` | Playlist management |
| `/audio_library/search` | Search audio library |
| `/image_library/reqimage` | Request image from library |
| `/image_library/s3putsign/` | Get S3 signed URL for uploads |
| `/user_assets/pdfs` | PDF uploads |
| `/account/available_quota` | Check upload quota |

### Character Sheet APIs

| Endpoint | Purpose |
|---|---|
| `/sheetsandbox/getsheetdefaults` | Get character sheet defaults |
| `/sheetsandbox/savesheetsettings` | Save sheet settings |
| `/csc/charactersheet-api` | Character sheet API |
| `/csc/compendium` | Compendium integration |

### Third-Party Service URLs

| Service | URLs |
|---|---|
| **Compendium GraphQL** | `https://compendium.csc.roll20teams.net/graphql`, `https://compendium.production.roll20preflight.net/graphql` |
| **Character Sheet API** | `https://api.charactersheet.production.roll20preflight.net` |
| **Advanced Sheets** | `https://advanced-sheets.production.roll20preflight.net` |
| **Sheet HTTP** | `https://sheet-http.production.roll20preflight.net` |
| **Demiplane** | `https://app.demiplane.com`, `/nexus/{ID}/compendium-link` |
| **Dungeon Scrawl** | `window.DUNGEON_SCRAWL_URL` (dynamic) |
| **S3 File Storage** | `https://s3.amazonaws.com/files.d20.io`, `cdn.roll20.net` |
| **Marketplace** | `https://marketplace.roll20.net` |
| **Dice Service** | `https://dice.roll20.net` |
| **Image Server** | `https://imgsrv.roll20.net` |

### Discord Activity Proxy

When running inside Discord (embedded mode), all requests route through a CORS proxy:
```
https://{DISCORD_ACTIVITY_CLIENT_ID}.discordsays.com/.proxy/...
```
Proxied paths include: `audio_library/*`, `editor/*`, `image_library/*`, `user_assets/*`, `compendium/*`, `roll20/imgsrv/`, `amazonaws/s3`, `googleapis/storage`.

---

## 6. Rendering Engine (Babylon.js)

### Architecture

| Component | Access | Purpose |
|---|---|---|
| `VTTEngine.instance` | Singleton | Central engine instance |
| `tabletop.engine.babylonEngine` | Babylon renderer | WebGL rendering context |
| `tabletop.scene` | Babylon Scene | 3D scene graph |
| `tabletop.scene.metadata.gmMode` | boolean | GM privilege flag in renderer |
| `tabletop.engine.lightingViewport` | Viewport | Lighting computation viewport |

### Layer Stack (bottom to top)

1. **Map** — background tiles/images
2. **Objects** — tokens and creature graphics
3. **GM Layer** — GM-only objects invisible to players
4. **Walls** — dynamic lighting walls/doors
5. **Lighting** — fog of war polygons
6. **Foreground** — foreground overlay objects

### Custom Shaders

**Grid shader**: Real-time WebGL fragment shader computing grid lines. Uniforms include `u_GridColor`, `u_GridOpacity`, `u_CellSize`, `u_CurrentZoom`. Uses distance-based anti-aliasing and reduces alpha for off-board areas.

**Tint shader**: `TintMaterialPlugin` applies color overlays to tokens via fragment shader: `mix(gl_FragColor.rgb, u_TintColor, 0.5)`.

### Supporting Managers

| Manager | Responsibility |
|---|---|
| `lightingManager` | Dynamic lighting + fog computation, `redrawLightingNextTick()` |
| `effectsManager` | Particle/visual effects |
| `pingManager` | Player pings/indicators |
| `popupManager` | Context menus/dialogs |
| `fileManager` | Asset loading/deletion, `waitForAssetToLoad()` |
| `textureAtlasManager` | Texture atlas consolidation |
| `keyboardInputManager` | Keyboard shortcuts |
| `notificationsManager` | Toast notifications |
| `uiManager` | UI state (file uploader modal, etc.) |

### Grid Systems

- Square grid: Standard cell-based
- Hex grid: `d20.canvas_overlay.activeHexGrid`
- Isometric grid: `d20.canvas_overlay.activeIsometricGrid`
- Snapping: `snapToIncrement()`, `snapToHexCorner()`, `snapToIsoCenter()`
- Coordinate conversion: `tileAtCoords()`, `GetHexAt()`
- Cell size: 70 pixels per grid cell

---

## 7. Chat & Dice System

### Chat Types

The text chat system (`d20.textchat`) processes these message types (inferred from string patterns):

| Type | Syntax | Purpose |
|---|---|---|
| Standard | (plain text) | Normal chat message |
| Whisper | `/w PlayerName msg` | Private message to player |
| GM whisper | `/w gm msg` | Whisper to GM |
| Emote | `/em action` | Third-person emote |
| Description | `/desc text` | Narration/description |
| OOC | `/ooc text` | Out-of-character |
| Roll | `/roll 2d6+3` | Dice roll |

### Dice Rolling

The bundle includes both client-side and server-side dice:

- **`/doroll`** — server-side endpoint for rolls
- **`https://dice.roll20.net`** — dedicated dice service
- **Quantum Roll** — secure server-side dice (`quantumroll`, `trueDiceRoll`, `pseudorandom` strings found). Pro feature that generates dice on the server to prevent client manipulation.
- **Roll templates** — `rolltemplate` system for formatted roll output (e.g., `rolltemplate==="5e"` renders D&D 5e styled output at line 54971)
- **Inline rolls** — `[[2d6+3]]` syntax within chat messages

### Speaking As

The `speakingas` property controls which character identity sends a chat message — allows GMs to speak as NPCs.

### Roll Templates

Found a concrete template at line 54971:
```javascript
if (d.rolltemplate === "5e") return `<table>...`
```
Roll templates are HTML renderers keyed by sheet system name that format dice results into styled output cards.

---

## 8. Subscription Feature Gates

Several features are gated behind Roll20 Pro/Plus subscriptions. The bundle references these marketing redirect URLs:

| Feature | Gate URL param | What it does |
|---|---|---|
| **Transmogrifier** | `irefevent=Transmog` | Copy characters/pages between campaigns |
| **Custom Effects** | `irefevent=CustomEffects` | Particle effects on tokens |
| **Dynamic Lighting Tool** | `irefevent=DLTool` | Advanced dynamic lighting editor |
| **Pins Customization** | `irefevent=PinsCustomization` | Custom map pin styles |

All redirect to `why-subscribe-to-roll20?iref=VTT&irefevent={feature}`.

These features may not be available to all users — relevant when designing extension features that depend on them.

---

## 9. What Features Are Possible

This section describes what an extension (like ours in `extension/content.js`) can practically build, grounded in the data surfaces exposed by the bundle.

### Currently Exploited (what we already do)

| Feature | Data Source | How |
|---|---|---|
| **Read character attributes** | `Campaign.characters.models[].attribs` | Iterate Backbone collection, read name/current/max |
| **Set character attributes** | `model.attribs.find()` + `.set()` / `.create()` | Backbone model API triggers Firebase sync |
| **Send chat messages** | `d20.textchat.doChatInput(msg)` | Programmatic chat injection |
| **Connection status** | WebSocket open/close | Know when extension is connected |

### Feasible to Add (high confidence)

#### Live Resource Monitoring
**Data**: `token.get("bar1_value")`, `token.get("bar2_value")`, `token.get("bar3_value")` + `_max` variants
**Feature**: Real-time HP / Willpower / Hunger tracking for all tokens on the active page. Backbone models fire `change` events, so you can subscribe: `token.on("change:bar1_value", callback)`.
**VTM use**: Track Hunger (bar), Health (bar), Willpower (bar) at the token level without polling.

#### Status Condition Tracking
**Data**: `token.get("statusmarkers")` — comma-separated string of marker names
**Feature**: Monitor and display active conditions (Staggered, Impaired, etc.) for all characters. Can set markers programmatically via `token.set("statusmarkers", "...")`.
**VTM use**: Automatically apply status markers when Hunger reaches critical levels, or when health tracks fill.

#### Initiative / Turn Order
**Data**: Turn order data (~19 references for `initiative`/`turnorder`/`tracker`)
**Feature**: Read and manipulate the initiative tracker. Could auto-roll initiative using VTM rules, reorder combatants, track rounds.
**VTM use**: V5 doesn't use traditional initiative but some chronicles use a turn tracker for combat structure.

#### GM Notes & Bio Access
**Data**: `character.get("gmnotes")`, `character.get("bio")`
**Feature**: Read character bios and GM-only notes from the desktop app. Useful for quick reference during play without switching to the Roll20 journal.
**Requires**: GM role (`window.IS_GM === true`).

#### Page/Map Awareness
**Data**: `d20.Campaign.activePage()`, page model properties
**Feature**: Know which map is currently active, its grid type, scale, dimensions. Could synchronize map context with the desktop app (e.g., "you're on the Elysium map, scale is 5ft/cell").
**VTM use**: Contextual location tracking for chronicle notes.

#### Handout Reading
**Data**: Handout Backbone models (~53 hits for handout patterns)
**Feature**: Read handout titles and contents from the campaign journal. Could surface relevant handouts in the desktop app based on context.
**VTM use**: Quick access to chronicle lore, Kindred politics notes, domain maps.

#### Jukebox / Ambient Audio
**Data**: Audio library APIs (`/audio_library/playlists`, `/audio_library/search`), jukebox controls (~95 hits)
**Feature**: Control background music from the desktop app. Read current playlist state, trigger tracks.
**VTM use**: Mood-appropriate music triggers (e.g., play combat music when entering Frenzy).

#### Macro / Ability Execution
**Data**: `character.abilities` collection, `istokenaction` flag, macro execution
**Feature**: List and trigger character macros/abilities from the desktop app. Token actions are macros flagged for quick access.
**VTM use**: Quick-fire Rouse checks, Frenzy rolls, Discipline powers from the companion app.

#### Rollable Table Queries
**Data**: `d20.Campaign.rollabletables` collection
**Feature**: Read and roll on campaign random tables from the desktop app.
**VTM use**: Random encounter tables, NPC name generators, hunt scene outcomes.

#### Player Presence
**Data**: `d20.Campaign.players` collection, `window.currentPlayer`
**Feature**: See who's connected, player names, roles. Could show a player roster in the desktop app.

### Possible but Complex (medium confidence)

#### Token Position Tracking
**Data**: `token.get("left")`, `token.get("top")`, `token.get("width")`, `token.get("height")`, `token.get("layer")`
**Feature**: Track where tokens are on the map in real-time. Could compute distances between tokens, detect adjacency.
**VTM use**: Know which characters are near each other for area-effect powers (Presence, Dominate eye contact range).

#### Dynamic Lighting State
**Data**: `lightingManager`, page dynamic lighting properties, door/wall models
**Feature**: Read whether lighting is enabled, which doors are open/closed.
**Pro-gated**: Dynamic lighting is a subscription feature.

#### Fog of War Manipulation
**Data**: `d20.engine.fog`, `d20.engine.finishPolygonReveal()`, `d20.engine.clearPageFog()`
**Feature**: Programmatically reveal/hide fog areas.
**Risk**: GM-only, could disrupt game state. Better as a convenience tool than automation.

#### Particle Effects on Tokens
**Data**: `effectsManager` (183 hits for emitter/particle/weather patterns)
**Feature**: Apply visual effects to tokens — fire auras, shadow emanations.
**Pro-gated**: Custom effects require subscription.

#### Chat Log Interception
**Data**: `d20.textchat.incoming()` — the chat message handler
**Feature**: Read all incoming chat messages in real-time, including roll results. Could parse roll results and feed them into the desktop app's tracking.
**VTM use**: Capture Rouse check results, hunt rolls, and other in-chat dice automatically rather than requiring manual entry.

#### Compendium Search
**Data**: `/compendium/compendium/globalsearch/{system}`, compendium GraphQL API
**Feature**: Search the compendium from the desktop app. Would need auth tokens.
**VTM use**: Quick rule lookups during play.

### Impractical / Not Recommended

| Feature | Why not |
|---|---|
| **Canvas rendering manipulation** | Babylon.js scene graph is deeply internal; changes would conflict with the render loop and break visual state |
| **Direct Firebase writes** | Bypassing the Backbone model layer would desync the client; all writes should go through `.set()` / `.create()` |
| **Auth token extraction** | Security concern; the extension should use the existing session, not extract tokens |
| **Bypassing subscription gates** | Feature-gated code checks server-side; client-only unlocking would fail or violate ToS |

---

## 10. Architecture Diagram

```
┌──────────────────────────────────────────────────────────────┐
│                    Roll20 Browser Tab                        │
│                                                              │
│  ┌──────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │ Vue.js   │  │ Backbone     │  │ Babylon.js             │ │
│  │ UI Layer │  │ Models       │  │ Canvas Engine           │ │
│  │          │  │              │  │                         │ │
│  │ Sidebar  │  │ Characters ──┼──│─► Token meshes         │ │
│  │ Chat     │  │ Pages ───────┼──│─► Map tiles            │ │
│  │ Journal  │  │ Tokens ──────┼──│─► Graphics             │ │
│  │ Settings │  │ Handouts     │  │ Lighting manager       │ │
│  │ Jukebox  │  │ Macros       │  │ Fog of war             │ │
│  │          │  │ Decks        │  │ Grid shaders           │ │
│  │          │  │ Players      │  │ Effects/particles      │ │
│  └──────────┘  └──────┬───────┘  └────────────────────────┘ │
│                       │                                      │
│                       │ .get() / .set() / .on("change")      │
│                       │                                      │
│              ┌────────▼────────────────────┐                 │
│              │    Firebase Sync Layer      │                 │
│              │                             │                 │
│              │  Firestore (collections)    │                 │
│              │  Realtime DB (WebSocket)    │                 │
│              │  Auth (tokens/sessions)     │                 │
│              └────────┬────────────────────┘                 │
│                       │                                      │
├───────────────────────┼──────────────────────────────────────┤
│  content.js (ext)     │                                      │
│  ┌────────────────────▼───┐        ┌──────────────────────┐ │
│  │ Extension reads        │  ws:// │ Tauri Desktop App    │ │
│  │ window.Campaign.*      ├────────► port 7423             │ │
│  │ d20.textchat.*         │◄───────┤ Roll20 commands      │ │
│  │ Backbone model API     │        │ Character tracking   │ │
│  └────────────────────────┘        │ Resonance tools      │ │
│                                    └──────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

---

## 11. Key Takeaways for Extension Development

1. **Backbone is the API.** All reads and writes to game state should go through Backbone `.get()` / `.set()` / `.create()`. This triggers Firebase sync automatically. Never write to Firebase directly.

2. **Content script isolation matters.** Chrome extension content scripts run in an isolated world and cannot access `window.Campaign` or `d20.*` directly. Our extension already handles this by injecting code into the page's main world (see `extension/content.js`). Any new Backbone subscriptions or global access must use the same injection mechanism — subscribing from the content script's isolated scope will see `undefined`.

3. **Backbone events are free.** `model.on("change:fieldName", callback)` gives you real-time updates without polling. This is how the extension should watch for attribute changes rather than using timers.

4. **`d20.textchat.doChatInput()` is the chat API.** It handles all chat types including rolls (`/roll 2d6`), whispers (`/w gm`), emotes (`/em`), and roll templates.

5. **GM-only data requires `window.IS_GM`.** Fields like `gmnotes`, GM layer tokens, and fog controls are only accessible to the GM.

6. **Pro features can't be unlocked client-side.** Transmogrifier, custom effects, dynamic lighting tools, and pin customization are server-validated subscription features.

7. **The rendering engine is off-limits.** Babylon.js scene manipulation would conflict with Roll20's render loop. Stick to the Backbone model layer.

8. **Firebase paths are campaign-scoped.** All data lives under `{campaign_storage_path}/` — switching campaigns means all references change.

9. **Token bars map to character attributes.** `bar1`/`bar2`/`bar3` on tokens are linked to character attributes. Changing the attribute via the Backbone model updates the bar display automatically.
