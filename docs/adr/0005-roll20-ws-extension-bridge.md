# 0005: Roll20 integration via localhost WebSocket + browser extension

**Status:** accepted
**Date:** 2026-04-19

## Context

Several tools (Campaign viewer, Resonance Roll20 write-back, future
features) depend on live Roll20 character data — health, willpower,
hunger, humanity, blood potency, and attribute write-back for rolls.
We evaluated ingestion paths: Roll20's public API, direct character-sheet
export, a relay server, and a localhost bridge fed by a browser
extension.

## Decision

A Chrome extension (`extension/`) injects a content script into Roll20
pages. The extension reads Roll20's DOM for character state (which the
logged-in user is already authorized to see) and opens a WebSocket
connection to `127.0.0.1:7423`, a listener owned by the Tauri app
(`src-tauri/src/roll20/`). The Tauri app accepts one connection at a
time, maintains the character cache in `Arc<Roll20State>`, and emits
`roll20://connected`, `roll20://disconnected`, and
`roll20://characters-updated` events to the Svelte frontend.

## Consequences

- Data is live ground-truth — whatever the user sees on the sheet is
  what the app sees.
- No Roll20 API key, rate limits, or API dependency.
- No cloud relay; nothing leaves the user's machine.
- Security posture: localhost-only listener; any process running as the
  user can connect. This is equivalent to trusting the user and is the
  stated security posture for a single-user desktop tool (see
  ARCHITECTURE.md §8).
- Roll20 DOM changes can break attribute resolution. Mitigated by
  `docs/reference/roll20-bundle.md`, which documents the current DOM
  paths and Jumpgate sheet attribute names.
- Single-session only — one extension WS connection at a time.
  Sufficient for single-user use; revisit if multi-seat scenarios arise.

## Alternatives considered

- **Roll20 public API.** Rejected — not designed for live sheet state,
  rate-limited, and requires an API key per user.
- **Cloud relay / hosted backend.** Rejected — contradicts the project's
  "no cloud, no server" posture (see ARCHITECTURE.md §12 Non-goals) and
  adds an external dependency.
- **Manual character-sheet export.** Rejected — stale, manual, and
  incompatible with the write-back direction (attribute updates from
  vtmtools to Roll20).
