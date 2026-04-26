# Data Sources & I/O Reference

This document names and describes every source of data in VTM Tools where data comes from, what it carries, which direction it flows, and what currently uses it. These names are the shared vocabulary for all other design documents.

---

## GM Roll Config

**Direction:** Input (user → app)
**What it is:** The settings the Storyteller configures before making a resonance roll. This is entered directly in the Resonance Roller UI each session.

**Data carried:**
- Temperament settings: how many dice to roll (1–5), whether to take the highest or lowest, and the threshold numbers that define what counts as Negligible, Fleeting, or Intense.
- Resonance weights: per-type likelihood sliders (Phlegmatic, Melancholy, Choleric, Sanguine) ranging from Impossible through Neutral to Guaranteed.

**Currently used by:** Resonance Roller. Passed to the Dice Engine as a complete config bundle on each roll.

**Notes:** This config is ephemeral: it lives in component state and resets when the app restarts. It is not persisted to the database.

---

## Dice Engine

**Direction:** Internal (produces output from config input)
**What it is:** The random number generation backend. Takes a GM Roll Config and produces raw roll results. All randomness in the app flows through here.

**Data produced:**
- Temperament roll: the full pool of d10 results plus which die was selected (highest or lowest) and the resulting temperament tier (Negligible / Fleeting / Intense).
- Resonance type pick: a weighted random selection across the four resonance types, controlled by the GM Roll Config weights. A display die is also rolled for flavor but does not determine the outcome.
- Acute check: a single d10 roll; 9–10 means Acute, which triggers a dyscrasia lookup.

**Currently used by:** Resonance Roller (via `roll_resonance` command). All dice logic runs on the backend.

---

## Resonance Roll Result

**Direction:** Output (app → user, and internal → other systems)
**What it is:** The complete outcome of a single resonance roll. This is the main "product" of the Resonance Roller and the data that flows outward to exports, Roll20 writeback, and the Tool Event Bus.

**Data carried:**
- All temperament dice rolled and which one was selected.
- The temperament tier (Negligible / Fleeting / Intense).
- The resonance type (if Fleeting or Intense).
- The acute check die and result (if Intense).
- An optional attached dyscrasia (added after the GM picks or rolls one).

**Currently consumed by:**
- The Resonance Roller UI (displays the result card).
- Roll History (stored in-memory for the current session).
- Markdown Export (written to disk).
- Bridge Writeback (pushed to a Roll20 sheet or a Foundry actor).
- Tool Event Bus (broadcast to other tools).

---

## Dyscrasia Store

**Direction:** Both (app reads and writes)
**What it is:** The SQLite database table that holds all dyscrasia entries — both the built-in ones from the V5 sourcebook and any custom ones the user has created.

**Data carried per entry:**
- Resonance type (Phlegmatic / Melancholy / Choleric / Sanguine).
- Name (e.g., "Despondent").
- Description — the full effect text.
- Bonus — a short mechanical tag (e.g., "+1 die to Auspex").
- Whether it is built-in or custom.

**Behavior:**
- Built-in entries are re-seeded from code on every app startup. This means updates to the sourcebook data are applied automatically and cannot be lost.
- Custom entries are user-created, user-editable, and persist across restarts.
- Random dyscrasia rolls select from all entries (built-in + custom) matching the requested resonance type.

**Currently used by:** Dyscrasia Manager (full CRUD + random roll), Resonance Roller (random roll on Acute result).

---

## Chronicle Store

**Direction:** Both (app reads and writes)
**What it is:** The SQLite tables holding all user-authored chronicle data — geographic/organizational areas, characters, businesses, merits, influence holdings, and typed relationships between them. Split across three tables: `chronicles`, `nodes`, `edges`.

**Data carried per chronicle:** name, description, timestamps.

**Data carried per node:** type (freeform user-chosen string), label, markdown description, cross-cutting tags, and a typed property bag (array of named fields with declared types: string, text, number, date, url, email, bool, reference).

**Data carried per edge:** type (freeform user-chosen string), from-node, to-node, markdown description, typed property bag.

**Behavior:**
- Nothing is seeded; all data is user-created.
- The `"contains"` edge type is treated by the UI as the navigation relationship (breadcrumbs and drilldown walk it), but the DB does not privilege it.
- Deleting a chronicle cascades to all its nodes and edges. Deleting a node cascades to all its edges.
- A node has at most one incoming `contains` edge (enforced by a partial unique index) — the `contains` sub-graph is a strict tree.

**Currently used by:** Domains Manager (full CRUD + derived tree queries).

---

## Bridge Live Feed

**Direction:** Input (VTT browser → app)
**What it is:** Live character data streaming in from one or more VTT sessions. Each VTT has its own data path on its own port; both feed a single canonical character cache the frontend consumes uniformly. See [ADR 0006](../adr/0006-bridge-source-generalization.md).

### Roll20 source

**Data path:** A Chrome extension reads character models from Roll20's internal Backbone data layer and sends them to the desktop app over a plain WebSocket on `ws://127.0.0.1:7423`.

**Data carried per character (raw):** id, name, controlled-by, attributes (name/current/max triples for everything on the sheet).

**How data arrives:** Roll20 uses Firebase under the hood. When any attribute changes (a player takes damage, gains hunger, etc.), Firebase fires individual per-attribute events. The extension uses a debounce pattern — it waits 200ms after the last attribute change for a character, then sends one batched update containing the full character snapshot. So downstream consumers receive **whole-character updates**, not field diffs.

### Foundry source

**Data path:** The `vtmtools-bridge/` Foundry module (installed in the GM's world) runs in the GM's browser only — gated on `game.user.isGM`. It dials `wss://127.0.0.1:7424` (the desktop app generates a self-signed `localhost` cert on first launch; the GM accepts it once per browser by visiting `https://localhost:7424`). On `ready` it pushes the full actor list, then forwards individual actor changes from `updateActor` / `createActor` / `deleteActor` hooks.

**Data carried per actor (raw):** id, name, owner (the first non-GM `OWNER`-permission user, or null), and the full `actor.system` blob — which carries vampire/werewolf/hunter splat fields side-by-side because WoD5e attaches all three schemas to every actor regardless of type. See [`foundry-vtm5e-paths.md`](./foundry-vtm5e-paths.md) and [`foundry-vtm5e-actor-sample.json`](./foundry-vtm5e-actor-sample.json).

### Canonical shape (what consumers see)

Both sources are translated at the bridge layer into a `CanonicalCharacter` with shared fields (hunger, health, willpower, humanity, humanity_stains, blood_potency) plus a `raw` blob for source-specific extras (Roll20 attributes / Foundry `actor.system`). Tools that need source-specific data cast `char.raw` to a per-source type. The merged cache is keyed by `<source>:<source_id>`.

**Data flow path:**
1. VTT page (Roll20 Firebase / Foundry actor data) → source-specific scrape (extension content script / Foundry module hooks)
2. WebSocket message (`ws://:7423` or `wss://:7424`) → Tauri backend
3. `BridgeSource::handle_inbound` translates raw → canonical
4. Backend stores in `HashMap<String, CanonicalCharacter>` → emits Tauri event
5. Frontend receives `bridge://characters-updated` event

**Currently consumed by:** Campaign tool (live stat display), Resonance Roller (character selector for writeback).

---

## Bridge Writeback

**Direction:** Output (app → VTT browser)
**What it is:** Data pushed from the desktop app back into a VTT character. Goes through the same per-source WebSocket connection as the Live Feed, but in the opposite direction. The frontend calls one generic `bridge_set_attribute(source, source_id, name, value)` Tauri command; each source impl translates it into source-specific operations.

**Operations supported:**
- **Set Attribute** — write a single attribute value on a specific character. The `name` is opaque to the frontend; per-source semantics:
  - **Roll20:** `name` is a sheet-attribute name (e.g. `resonance`). Creates the attribute if missing, updates if present.
  - **Foundry:** Most names map to `actor.update({ "system.<path>.value": <value> })` (e.g. `hunger`, `humanity`, `health_superficial`). The exception is `resonance` — WoD5e stores it as an Item document, so the source builds a `create_item` message and the Foundry module calls `actor.createEmbeddedDocuments("Item", [...])` after deleting any existing resonance items. See [`foundry-vtm5e-paths.md`](./foundry-vtm5e-paths.md).
- **Refresh** — ask the source(s) to re-read all characters and send fresh snapshots. With no source specified, refreshes everyone.

**Note:** The Roll20-only `Send Chat` operation from the pre-bridge era was dropped during the cutover (no frontend consumer). The wire protocol still allows it; the typed wrapper does not expose it.

**Currently used by:** Resonance Roller (writes resonance result back to a selected character's sheet — works against either source).

---

## Markdown Export

**Direction:** Output (app → filesystem)
**What it is:** Resonance roll results saved as Markdown files to `~/Documents/vtmtools/`. Each export creates one timestamped `.md` file containing the full roll result plus any attached dyscrasia.

**Data carried:**
- All fields from the Resonance Roll Result.
- Attached dyscrasia details (if any).
- Export timestamp.

**Currently used by:** Resonance Roller (export button on the result card).

---

## Tool Event Bus

**Direction:** Internal (tool → tool)
**What it is:** A Svelte store that lets tools broadcast events to each other without direct coupling. One tool publishes an event; any other tool can subscribe and react.

**Current event types:**
- `resonance_result` — fired when a resonance roll completes. Carries the temperament, resonance type, acute status, and dyscrasia name.

**Design intent:** This exists so that future tools (e.g., a Conflict Tracker or Character Builder) can react to events from other tools without the tools needing to know about each other. The bus is the decoupling layer.

**Currently used by:** Resonance Roller (publishes). No tool currently subscribes — the bus is infrastructure for future cross-tool features.

---

## Tauri Event Bridge

**Direction:** Internal (backend → frontend)
**What it is:** The event channel that carries real-time updates from the Rust backend to the Svelte frontend. Uses Tauri's built-in event system with the `bridge://` prefix.

**Events currently emitted:**
- `bridge://roll20/connected` — Roll20 extension opened a WS connection.
- `bridge://roll20/disconnected` — Roll20 extension closed.
- `bridge://foundry/connected` — Foundry module opened a wss connection.
- `bridge://foundry/disconnected` — Foundry module closed.
- `bridge://characters-updated` — any source pushed new or updated character data; carries the merged `Vec<CanonicalCharacter>` across all sources.

**Currently consumed by:** Campaign tool, Resonance Roller — both subscribe via `src/store/bridge.svelte.ts` (the typed bridge store), not by listening directly. Per CLAUDE.md, components never call `invoke` or `listen` directly.

**Notes:** This is distinct from the Tool Event Bus. The Tauri Event Bridge carries backend-to-frontend signals (connection state, data updates). The Tool Event Bus carries frontend-to-frontend signals (cross-tool reactions). They operate on different layers and should not be conflated.

---

## Summary: How Data Flows

```
┌─────────────────────────────────────────────────────────────┐
│                        USER (Storyteller)                   │
│                                                             │
│   GM Roll Config ──→ Dice Engine ──→ Resonance Roll Result  │
│                                        │    │    │          │
│                                        │    │    └──→ Tool Event Bus
│                                        │    └──→ Markdown Export
│                                        │                    │
│                          Bridge Writeback ←─────────────────┘
│                                   │                         │
└───────────────────────────────────│─────────────────────────┘
                                    │
                  ┌─────────────────┴────────────────┐
                  │                                  │
       ┌──────────▼──────────┐         ┌─────────────▼─────────────┐
       │  ws://:7423          │         │  wss://:7424              │
       └──────────┬──────────┘         └─────────────┬─────────────┘
                  │                                  │
       ┌──────────▼──────────┐         ┌─────────────▼─────────────┐
       │  Roll20 extension   │         │  vtmtools-bridge module    │
       │  (Chrome content)   │         │  (Foundry GM browser only) │
       └──────────┬──────────┘         └─────────────┬─────────────┘
                  │                                  │
       ┌──────────▼──────────┐         ┌─────────────▼─────────────┐
       │  Roll20 session     │         │  Foundry world (any host)  │
       └─────────────────────┘         └───────────────────────────┘


┌──────────────────────────────────────────┐
│           Dyscrasia Store (SQLite)        │
│  ← seeded on startup                     │
│  ← custom entries via Dyscrasia Manager  │
│  → queried by Resonance Roller           │
│  → browsed by Dyscrasia Manager          │
└──────────────────────────────────────────┘


Backend ──── Tauri Event Bridge ────→ Frontend
              (bridge:// events; merged characters cache)
```

### Quick Reference Table

| Name                    | Direction       | Storage    | Layer            |
|-------------------------|-----------------|------------|------------------|
| GM Roll Config          | Input           | In-memory  | Frontend         |
| Dice Engine             | Internal        | None       | Backend          |
| Resonance Roll Result   | Output/Internal | In-memory  | Backend→Frontend |
| Dyscrasia Store         | Both            | SQLite     | Backend          |
| Chronicle Store         | Both            | SQLite     | Backend          |
| Bridge Live Feed        | Input           | In-memory  | VTT→Backend      |
| Bridge Writeback        | Output          | None       | Backend→VTT      |
| Markdown Export         | Output          | Filesystem | Backend          |
| Tool Event Bus          | Internal        | In-memory  | Frontend         |
| Tauri Event Bridge      | Internal        | None       | Backend→Frontend |
