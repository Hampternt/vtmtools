# 0001: Tauri 2 + SvelteKit + SQLite stack

**Status:** accepted
**Date:** 2026-04-19

## Context

vtmtools is a GM-facing desktop tool for Vampire: The Masquerade 5th
Edition. Requirements: offline-capable, native filesystem access (for
Markdown exports and a local SQLite database), modest binary size, single-
user operation, no hosted backend, strong types shared between frontend
and backend, and a small maintenance surface. We evaluated Electron,
Tauri 1, Tauri 2, and a web-only deployment.

## Decision

- Frontend: SvelteKit with the static adapter, running as a single-page
  app (no SSR, no server routes).
- Backend: Rust, compiled into the Tauri 2 shell.
- Database: SQLite via `sqlx`, stored in the Tauri app data directory.
- IPC: Tauri 2 commands + events, with a capability allowlist.

## Consequences

- Binary size is a fraction of an equivalent Electron app (native webview
  on each OS, not a bundled Chromium).
- Serde types in Rust can be mirrored in TypeScript with minimal drift;
  the IPC boundary is strongly typed in both directions.
- Tauri 2's capabilities system gives a default-deny IPC surface —
  adding a new command requires listing it in a capability JSON.
- SvelteKit's SPA mode avoids SSR complexity that is irrelevant for a
  desktop app.
- Trade: smaller plugin ecosystem than Electron, and occasional platform
  webview differences require targeted workarounds.

## Alternatives considered

- **Electron.** Rejected for binary size, resource overhead, and lack of
  fit with a Rust backend.
- **Tauri 1.** Rejected in favor of Tauri 2 for its capabilities system,
  improved plugin architecture, and active direction of the project.
- **Web-only SPA.** Rejected — no filesystem access for Markdown exports
  or SQLite, and no way to host the localhost WebSocket listener the
  Roll20 integration requires (see ADR 0005).
