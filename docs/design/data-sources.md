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
- Roll20 Writeback (pushed to character sheet).
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

## Roll20 Live Feed

**Direction:** Input (Roll20 browser → app)
**What it is:** Live character data streaming in from a Roll20 game session. The browser extension reads character models from Roll20's internal Backbone data layer and sends them to the desktop app over a local WebSocket connection.

**Data carried per character:**
- Character ID (Roll20's internal identifier).
- Character name.
- Controlled-by field (which player owns the character).
- All character sheet attributes — name/value pairs covering stats like hunger, health, willpower, humanity, blood potency, and everything else on the sheet.

**How data arrives:**
Roll20 uses Firebase under the hood. When any attribute changes (a player takes damage, gains hunger, etc.), Firebase fires individual per-attribute events. The extension uses a debounce pattern — it waits 200ms after the last attribute change for a character, then sends one batched update containing the full character snapshot. This means downstream consumers receive **whole-character updates**, not individual field changes.

**Data flow path:**
1. Roll20 page (Firebase) → Backbone model events
2. Extension content script → debounced character snapshot
3. WebSocket message → Tauri backend
4. Backend stores in in-memory HashMap → emits Tauri event
5. Frontend receives `roll20://characters-updated` event

**Currently consumed by:** Campaign tool (live stat display), Resonance Roller (character selector for writeback).

---

## Roll20 Writeback

**Direction:** Output (app → Roll20 browser)
**What it is:** Data pushed from the desktop app back into a Roll20 character sheet. Goes through the same WebSocket connection as the Live Feed, but in the opposite direction.

**Operations supported:**
- **Set Attribute** — write a single attribute value on a specific character (identified by character ID + attribute name). Creates the attribute if it doesn't exist, updates it if it does.
- **Send Chat** — inject a message into the Roll20 chat as if the user typed it.
- **Refresh** — ask the extension to re-read all characters and send fresh snapshots.

**Currently used by:** Resonance Roller (writes resonance result back to a selected character's sheet).

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
**What it is:** The event channel that carries real-time updates from the Rust backend to the Svelte frontend. Uses Tauri's built-in event system with `roll20://` prefixed event names.

**Events currently emitted:**
- `roll20://connected` — the browser extension has connected.
- `roll20://disconnected` — the browser extension has disconnected.
- `roll20://characters-updated` — new or updated character data is available (carries the full character list).

**Currently consumed by:** Campaign tool, Resonance Roller — both listen for connection status and character updates.

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
│                              Roll20 Writeback ←─────────────┘
│                                   │                         │
└───────────────────────────────────│─────────────────────────┘
                                    │
                        ┌───────────▼──────────────┐
                        │   WebSocket (port 7423)   │
                        └───────────┬──────────────┘
                                    │
                        ┌───────────▼──────────────┐
                        │   Browser Extension       │
                        │   (Roll20 Live Feed)      │
                        └───────────┬──────────────┘
                                    │
                        ┌───────────▼──────────────┐
                        │   Roll20 Game Session     │
                        └──────────────────────────┘


┌──────────────────────────────────────────┐
│           Dyscrasia Store (SQLite)        │
│  ← seeded on startup                     │
│  ← custom entries via Dyscrasia Manager  │
│  → queried by Resonance Roller           │
│  → browsed by Dyscrasia Manager          │
└──────────────────────────────────────────┘


Backend ──── Tauri Event Bridge ────→ Frontend
              (roll20:// events)
```

### Quick Reference Table

| Name                    | Direction       | Storage    | Layer            |
|-------------------------|-----------------|------------|------------------|
| GM Roll Config          | Input           | In-memory  | Frontend         |
| Dice Engine             | Internal        | None       | Backend          |
| Resonance Roll Result   | Output/Internal | In-memory  | Backend→Frontend |
| Dyscrasia Store         | Both            | SQLite     | Backend          |
| Chronicle Store         | Both            | SQLite     | Backend          |
| Roll20 Live Feed        | Input           | In-memory  | Extension→Backend|
| Roll20 Writeback        | Output          | None       | Backend→Extension|
| Markdown Export         | Output          | Filesystem | Backend          |
| Tool Event Bus          | Internal        | In-memory  | Frontend         |
| Tauri Event Bridge      | Internal        | None       | Backend→Frontend |
