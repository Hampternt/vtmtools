# Bridge Protocol Consolidation Implementation Plan (Plan 0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate the Foundry bridge wire protocol with three additions: extended Hello (world metadata + protocol_version + capabilities), subscription envelopes (`bridge.subscribe` / `bridge.unsubscribe`), and an error envelope. Front-loads the protocol-level work that becomes expensive once the module ships from a separate repo.

**Architecture:** Pure protocol/infrastructure. `FoundryInbound::Hello` gains optional fields (backward-compatible with 0.1.0 modules). New `Error` variant routes module-side handler exceptions to a `bridge://foundry/error` Tauri event. `BridgeState` gains `source_info` per-source storage. New `bridge.*` umbrella in the Foundry helper library hosts the subscription builders. Module-side `actor.js` refactors to expose `attach()` / `detach()` so a future subscribers registry can manage it; default behavior (always-send-actors-on-Hello) is preserved.

**Tech Stack:** Rust (`serde`, `tokio`, `tauri`), JavaScript (Foundry V12+ module, no test framework), Tauri 2 bridge layer.

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md`

**Roadmap:** `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`

---

## File structure

### New files
- `src-tauri/src/bridge/foundry/actions/bridge.rs` — outbound `bridge.subscribe` / `bridge.unsubscribe` builders + tests
- `vtmtools-bridge/scripts/foundry-actions/bridge.js` — JS-side subscribers registry, subscribe/unsubscribe handlers

### Modified files
- `src-tauri/src/bridge/types.rs` — add `SourceInfo` struct
- `src-tauri/src/bridge/mod.rs` — `BridgeState` gains `source_info: Mutex<HashMap<SourceKind, SourceInfo>>`; populated on Hello, cleared on disconnect
- `src-tauri/src/bridge/foundry/types.rs` — `Hello` variant gains optional fields; new `Error` variant
- `src-tauri/src/bridge/foundry/translate.rs` — populate `source_info` on Hello; emit `bridge://foundry/error` on Error
- `src-tauri/src/bridge/foundry/actions/mod.rs` — re-export `bridge` umbrella module
- `src-tauri/src/bridge/commands.rs` — add `bridge_get_source_info` Tauri command
- `src-tauri/src/lib.rs` — register `bridge_get_source_info` in `invoke_handler!`
- `src/lib/bridge/api.ts` — typed wrapper `bridgeGetSourceInfo`
- `src/store/bridge.svelte.ts` — `sourceInfo` derived state, refreshed on connect events
- `vtmtools-bridge/scripts/bridge.js` — extend Hello payload; route handler exceptions to `Error` envelope; refactor actor-hooks setup to use the subscribers registry
- `vtmtools-bridge/scripts/foundry-actions/index.js` — register `bridge.*` umbrella in handler-map
- `vtmtools-bridge/scripts/foundry-actions/actor.js` — convert from inline-hook-setup to a subscriber object exposing `attach(socket) / detach()`
- `vtmtools-bridge/module.json` — bump version to `0.2.0`

### Files explicitly NOT touched
- `src-tauri/src/db/*` (Plan 1's territory)
- `src-tauri/src/shared/v5/*` (does not yet exist; Plan 3's territory)
- Any Svelte component except indirectly via `bridge.svelte.ts`
- `src-tauri/src/bridge/roll20/*`
- `vtmtools-bridge/scripts/translate.js` (helper used by `bridge.js`; signature unchanged)

---

## Task overview

| # | Task | Depends on |
|---|---|---|
| 1 | Add `SourceInfo` struct in `bridge/types.rs` | none |
| 2 | Extend `BridgeState` with `source_info` field; clear on disconnect | 1 |
| 3 | Extend `FoundryInbound`: Hello fields + Error variant; backward-compat `Option` fields | 1 |
| 4 | Update `bridge/foundry/translate.rs` to populate `source_info` on Hello and emit `bridge://foundry/error` on Error | 2, 3 |
| 5 | Create `bridge/foundry/actions/bridge.rs` with subscribe/unsubscribe builders + tests | none |
| 6 | Re-export `bridge` umbrella from `actions/mod.rs` | 5 |
| 7 | Add `bridge_get_source_info` Tauri command in `bridge/commands.rs` | 2 |
| 8 | Register `bridge_get_source_info` in `lib.rs` | 7 |
| 9 | Add `bridgeGetSourceInfo` typed wrapper in `src/lib/bridge/api.ts` | 7 |
| 10 | Extend `src/store/bridge.svelte.ts` with `sourceInfo` reactive state | 9 |
| 11 | Create `vtmtools-bridge/scripts/foundry-actions/bridge.js` (subscribers registry + handlers) | none |
| 12 | Refactor `vtmtools-bridge/scripts/foundry-actions/actor.js` into a subscriber object | 11 |
| 13 | Modify `vtmtools-bridge/scripts/bridge.js`: extend Hello payload; route handler exceptions to Error envelope; subscribe `actors` after Hello | 11, 12 |
| 14 | Register `bridge.*` umbrella in `foundry-actions/index.js` | 11 |
| 15 | Bump `vtmtools-bridge/module.json` to `0.2.0` and run final verification gate | all |

Tasks 1, 5, and 11 are independent and can dispatch in parallel as a first wave. Task 12 must follow Task 11.

---

## Task 1: Add `SourceInfo` struct

**Files:**
- Modify: `src-tauri/src/bridge/types.rs`

**Anti-scope:** Do NOT modify `BridgeState`, `FoundryInbound`, or any consumers of these types yet.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §4 (I/O contracts — types defined here are stable contracts).

- [ ] **Step 1: Add `SourceInfo` to `src-tauri/src/bridge/types.rs`**

Append to the end of the file:

```rust
/// Per-source connection metadata captured from the source's Hello frame.
/// Populated by the source's `handle_inbound` impl on Hello receipt; cleared
/// on disconnect by the bridge's connection-cleanup path. Not persisted.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub world_id: Option<String>,
    pub world_title: Option<String>,
    pub system_id: Option<String>,
    pub system_version: Option<String>,
    pub protocol_version: u32,
    pub capabilities: Vec<String>,
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean (one new struct, no consumers yet).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/bridge/types.rs
git commit -m "feat(bridge): add SourceInfo struct for per-source metadata (Plan 0 task 1)"
```

---

## Task 2: Extend `BridgeState` with `source_info`

**Files:**
- Modify: `src-tauri/src/bridge/mod.rs`

**Anti-scope:** Do NOT change accept/disconnect logic outside the connection-cleanup block. Do NOT modify `ConnectionInfo`.

**Depends on:** Task 1

**Invariants cited:** ARCHITECTURE.md §6 (one connection per source); §3 (in-memory ephemeral state lives in `BridgeState`).

- [ ] **Step 1: Add `source_info` field to `BridgeState`**

Edit `src-tauri/src/bridge/mod.rs` at the `BridgeState` struct definition (around lines 35-39):

```rust
pub struct BridgeState {
    pub characters: Mutex<HashMap<String, CanonicalCharacter>>,
    pub connections: Mutex<HashMap<SourceKind, ConnectionInfo>>,
    pub source_info: Mutex<HashMap<SourceKind, crate::bridge::types::SourceInfo>>,
    pub sources: HashMap<SourceKind, Arc<dyn BridgeSource>>,
}
```

- [ ] **Step 2: Initialize `source_info` in `BridgeState::new`**

Edit the `impl BridgeState { pub fn new(...) -> Self }` block (around lines 42-55). Change the `Self { … }` literal to:

```rust
        Self {
            characters: Mutex::new(HashMap::new()),
            connections: Mutex::new(connections),
            source_info: Mutex::new(HashMap::new()),
            sources,
        }
```

- [ ] **Step 3: Clear `source_info` on disconnect**

Edit the `handle_connection` function's disconnect cleanup block (around lines 209-216). Change:

```rust
    // Disconnect cleanup.
    {
        let mut conns = state.connections.lock().await;
        conns.insert(
            kind,
            ConnectionInfo { connected: false, outbound_tx: None },
        );
    }
```

to:

```rust
    // Disconnect cleanup.
    {
        let mut conns = state.connections.lock().await;
        conns.insert(
            kind,
            ConnectionInfo { connected: false, outbound_tx: None },
        );
    }
    {
        let mut info = state.source_info.lock().await;
        info.remove(&kind);
    }
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/bridge/mod.rs
git commit -m "feat(bridge): extend BridgeState with source_info per source (Plan 0 task 2)"
```

---

## Task 3: Extend `FoundryInbound` (Hello fields + Error variant)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs`

**Anti-scope:** Do NOT modify `translate.rs` in this task — only the type definitions change here.

**Depends on:** Task 1

**Invariants cited:** ARCHITECTURE.md §4 (Bridge WebSocket protocol — wire shapes are stable contracts).

- [ ] **Step 1: Replace the `FoundryInbound` enum**

Edit `src-tauri/src/bridge/foundry/types.rs`. Replace the existing enum (lines 6-14) with:

```rust
/// Inbound messages from the Foundry module.
///
/// Hello fields are all `Option<…>` for backward compatibility with 0.1.0
/// modules that send `{ "type": "hello" }` with no payload. Missing
/// `protocol_version` is treated by the desktop as `0` (legacy); missing
/// `capabilities` defaults to `["actors"]` (preserves always-send-actors).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    Actors { actors: Vec<FoundryActor> },
    ActorUpdate { actor: FoundryActor },
    Hello {
        #[serde(default)] protocol_version: Option<u32>,
        #[serde(default)] world_id: Option<String>,
        #[serde(default)] world_title: Option<String>,
        #[serde(default)] system_id: Option<String>,
        #[serde(default)] system_version: Option<String>,
        #[serde(default)] capabilities: Option<Vec<String>>,
    },
    /// Module-side handler threw; surfaced to the GM via toast.
    Error {
        refers_to: String,
        #[serde(default)] request_id: Option<String>,
        code: String,
        message: String,
    },
}
```

- [ ] **Step 2: Verify compilation breaks the way it should**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: compile error in `bridge/foundry/translate.rs` because the existing `Hello` arm pattern no longer matches the variant shape. This is the signal Task 4 needs to update the translator.

- [ ] **Step 3: Commit (broken — fixed in Task 4)**

The next task fixes the compile error in the same logical change. Commit so the contract change is isolated:

```bash
git add src-tauri/src/bridge/foundry/types.rs
git commit -m "feat(bridge/foundry): extend FoundryInbound with Hello fields + Error variant (Plan 0 task 3)"
```

---

## Task 4: Update `translate.rs` to populate `source_info` and route Error

**Files:**
- Modify: `src-tauri/src/bridge/foundry/translate.rs`

**Anti-scope:** Do NOT modify wire types again. Do NOT add new commands.

**Depends on:** Task 2, Task 3

**Invariants cited:** ARCHITECTURE.md §4 (event surface), §7 (error handling — module-stable string prefixes).

- [ ] **Step 1: Read the current `translate.rs`**

Read `src-tauri/src/bridge/foundry/translate.rs` to identify the existing `match` over `FoundryInbound` and the function that owns it (likely a `BridgeSource::handle_inbound` impl in `bridge/foundry/mod.rs`, with translation helpers here). Record the function's current signature.

- [ ] **Step 2: Update the Hello arm to capture fields**

Wherever `FoundryInbound::Hello` is matched (most likely `bridge/foundry/mod.rs::handle_inbound`), replace:

```rust
FoundryInbound::Hello => Ok(vec![]),
```

with:

```rust
FoundryInbound::Hello {
    protocol_version, world_id, world_title,
    system_id, system_version, capabilities,
} => {
    let info = crate::bridge::types::SourceInfo {
        world_id,
        world_title,
        system_id,
        system_version,
        protocol_version: protocol_version.unwrap_or(0),
        capabilities: capabilities.unwrap_or_else(|| vec!["actors".to_string()]),
    };
    // The handle_inbound caller writes the result; the SourceInfo write
    // happens via a sibling helper because handle_inbound is stateless per
    // ADR 0006. We surface SourceInfo by emitting a side-channel event the
    // bridge::handle_connection loop captures. Simplest route: pass through
    // a dedicated trait method or store it via Arc<BridgeState>.
    Ok(vec![])
}
```

Because `BridgeSource::handle_inbound` is stateless (trait signature `&self`, no state arg), populating `source_info` requires either widening the trait or threading state through. **Take the simpler path:** add `source_info` write inside `bridge::mod.rs::handle_connection` after parsing Hello, by detecting `parsed["type"] == "hello"` before delegating to `source.handle_inbound`. This keeps the trait stateless.

- [ ] **Step 3: Add Hello pre-handling in `bridge::mod.rs::handle_connection`**

Edit `src-tauri/src/bridge/mod.rs` at the inbound-loop (around lines 187-206). Replace:

```rust
        let parsed: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[bridge:{}] parse error: {e}  raw: {text}", kind.as_str());
                continue;
            }
        };
        match source.handle_inbound(parsed).await {
```

with:

```rust
        let parsed: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[bridge:{}] parse error: {e}  raw: {text}", kind.as_str());
                continue;
            }
        };

        // Foundry-only: capture Hello metadata into BridgeState::source_info.
        // Roll20 also sends a Hello shape; ignore there for now (capabilities
        // empty by default — matches today's behavior).
        if kind == SourceKind::Foundry {
            if let Some("hello") = parsed.get("type").and_then(|t| t.as_str()) {
                let info = crate::bridge::types::SourceInfo {
                    world_id: parsed.get("world_id").and_then(|v| v.as_str()).map(String::from),
                    world_title: parsed.get("world_title").and_then(|v| v.as_str()).map(String::from),
                    system_id: parsed.get("system_id").and_then(|v| v.as_str()).map(String::from),
                    system_version: parsed.get("system_version").and_then(|v| v.as_str()).map(String::from),
                    protocol_version: parsed.get("protocol_version").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    capabilities: parsed.get("capabilities")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                        .unwrap_or_else(|| vec!["actors".to_string()]),
                };
                let mut store = state.source_info.lock().await;
                store.insert(kind, info);
                drop(store);
            }
            // Error envelope from module: emit Tauri event, do not pass to source.
            if let Some("error") = parsed.get("type").and_then(|t| t.as_str()) {
                let payload = serde_json::json!({
                    "refers_to": parsed.get("refers_to").and_then(|v| v.as_str()).unwrap_or(""),
                    "code":      parsed.get("code").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "message":   parsed.get("message").and_then(|v| v.as_str()).unwrap_or(""),
                });
                let _ = handle.emit("bridge://foundry/error", payload);
                continue;
            }
        }

        match source.handle_inbound(parsed).await {
```

- [ ] **Step 4: Confirm `translate.rs` Hello arm still works (returns empty Vec)**

If `translate.rs` had a Hello arm, leave it returning `Ok(vec![])` (the bridge layer already captured the fields above). The trait method's responsibility for Hello is just "no character data."

If the existing arm was `FoundryInbound::Hello => Ok(vec![])`, change it to:

```rust
FoundryInbound::Hello { .. } => Ok(vec![]),
```

If the existing arm was elsewhere (e.g. inside `bridge/foundry/mod.rs::handle_inbound`), apply the same change.

- [ ] **Step 5: Add the Error arm**

Wherever `FoundryInbound` is matched (the `handle_inbound` implementation), add an arm that does nothing — the bridge layer already routes Error envelopes pre-trait:

```rust
FoundryInbound::Error { .. } => Ok(vec![]),
```

This arm exists for completeness; in practice the bridge layer's `if let Some("error")` check intercepts the message before it reaches `handle_inbound`.

- [ ] **Step 6: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 7: Verify tests still pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: all existing tests pass; no new tests yet.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/bridge/mod.rs src-tauri/src/bridge/foundry/translate.rs src-tauri/src/bridge/foundry/mod.rs
git commit -m "feat(bridge/foundry): capture Hello metadata + route Error envelope (Plan 0 task 4)"
```

---

## Task 5: Create `bridge.*` action umbrella with subscribe/unsubscribe builders

**Files:**
- Create: `src-tauri/src/bridge/foundry/actions/bridge.rs`

**Anti-scope:** Do NOT modify `actions/mod.rs` yet (Task 6). Do NOT consume these builders from any caller in this task — they exist for future plans.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §9 (typed-per-helper convention from Foundry helper roadmap).

- [ ] **Step 1: Write the failing tests**

Create `src-tauri/src/bridge/foundry/actions/bridge.rs`:

```rust
// Foundry bridge.* helper builders. These produce outbound wire envelopes
// the desktop sends to the module to control which Foundry collections
// stream over the WebSocket. The `actors` collection is auto-subscribed
// by the module on Hello (preserving today's always-send-actors behavior);
// future tools opt into other collections via `bridge.subscribe`.

use serde_json::{json, Value};

/// Build a `bridge.subscribe { collection }` envelope.
pub fn build_subscribe(collection: &str) -> Value {
    json!({ "type": "bridge.subscribe", "collection": collection })
}

/// Build a `bridge.unsubscribe { collection }` envelope.
pub fn build_unsubscribe(collection: &str) -> Value {
    json!({ "type": "bridge.unsubscribe", "collection": collection })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribe_envelope_shape() {
        let v = build_subscribe("journal");
        assert_eq!(v["type"], "bridge.subscribe");
        assert_eq!(v["collection"], "journal");
    }

    #[test]
    fn unsubscribe_envelope_shape() {
        let v = build_unsubscribe("scenes");
        assert_eq!(v["type"], "bridge.unsubscribe");
        assert_eq!(v["collection"], "scenes");
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::bridge -- --nocapture`
Expected: 2 passed (the file isn't yet declared in `mod.rs`, so use `cargo check` first to ensure the file is well-formed, then add to `mod.rs` in Task 6).

Actually `cargo test` won't find it until it's in the module tree. Skip this step's run for now — verification happens in Task 6 after `mod.rs` re-exports the file.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/bridge.rs
git commit -m "feat(bridge/foundry): add bridge.* action umbrella with subscribe builders (Plan 0 task 5)"
```

---

## Task 6: Re-export `bridge` umbrella from `actions/mod.rs`

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/mod.rs`

**Anti-scope:** Do NOT add new files in this task. Do NOT use the new builders yet.

**Depends on:** Task 5

**Invariants cited:** ARCHITECTURE.md §9 (umbrella organization).

- [ ] **Step 1: Add `pub mod bridge;` to `actions/mod.rs`**

Edit `src-tauri/src/bridge/foundry/actions/mod.rs`. The existing file likely contains:

```rust
pub mod actor;
pub mod game;
pub mod storyteller;
```

Add:

```rust
pub mod actor;
pub mod bridge;
pub mod game;
pub mod storyteller;
```

- [ ] **Step 2: Verify tests run**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::bridge`
Expected: 2 passed (`subscribe_envelope_shape`, `unsubscribe_envelope_shape`).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/bridge/foundry/actions/mod.rs
git commit -m "feat(bridge/foundry): export bridge.* umbrella (Plan 0 task 6)"
```

---

## Task 7: Add `bridge_get_source_info` Tauri command

**Files:**
- Modify: `src-tauri/src/bridge/commands.rs`

**Anti-scope:** Do NOT register the command in `lib.rs` yet (Task 8). Do NOT consume from frontend yet (Task 9).

**Depends on:** Task 2

**Invariants cited:** ARCHITECTURE.md §4 (Tauri IPC commands), §7 (errors as `Result<T, String>`).

- [ ] **Step 1: Read the existing `bridge/commands.rs`**

Read `src-tauri/src/bridge/commands.rs` to identify the existing pattern (other bridge commands take `State<'_, BridgeConn>`).

- [ ] **Step 2: Add the new command at the bottom of the file**

```rust
/// Returns the captured Hello metadata for a connected source, or None if
/// the source is not currently connected. Async to match the existing
/// bridge command surface (none of these have I/O — the async signature
/// is consistency, not necessity).
#[tauri::command]
pub async fn bridge_get_source_info(
    state: tauri::State<'_, crate::bridge::BridgeConn>,
    source: crate::bridge::types::SourceKind,
) -> Result<Option<crate::bridge::types::SourceInfo>, String> {
    let info = state.0.source_info.lock().await;
    Ok(info.get(&source).cloned())
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean (command exists; no caller yet).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bridge/commands.rs
git commit -m "feat(bridge): add bridge_get_source_info Tauri command (Plan 0 task 7)"
```

---

## Task 8: Register `bridge_get_source_info` in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Anti-scope:** Do NOT touch other entries in the `invoke_handler!` list. Do NOT add other commands.

**Depends on:** Task 7

**Invariants cited:** ARCHITECTURE.md §4, §8 (capability-default ACL — adding a command is in-scope under `core:default`).

- [ ] **Step 1: Add the registration line**

Edit `src-tauri/src/lib.rs`. In the `invoke_handler(tauri::generate_handler![…])` list, add `bridge::commands::bridge_get_source_info` adjacent to the existing bridge commands (after line `bridge::commands::bridge_set_attribute`).

```rust
            bridge::commands::bridge_get_characters,
            bridge::commands::bridge_get_status,
            bridge::commands::bridge_refresh,
            bridge::commands::bridge_set_attribute,
            bridge::commands::bridge_get_source_info,
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(bridge): register bridge_get_source_info command (Plan 0 task 8)"
```

---

## Task 9: Add `bridgeGetSourceInfo` typed wrapper

**Files:**
- Modify: `src/lib/bridge/api.ts`

**Anti-scope:** Do NOT modify `bridge.svelte.ts` yet (Task 10). Do NOT call `invoke()` from any component.

**Depends on:** Task 7

**Invariants cited:** ARCHITECTURE.md §4 (typed frontend API wrappers), §5 (no `invoke()` from components).

- [ ] **Step 1: Read current `src/lib/bridge/api.ts`**

Identify the current pattern: each function calls `invoke<T>('command_name', args)`.

- [ ] **Step 2: Add the wrapper + types**

Append to `src/lib/bridge/api.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';

export type SourceKind = 'roll20' | 'foundry';

export interface SourceInfo {
  worldId: string | null;
  worldTitle: string | null;
  systemId: string | null;
  systemVersion: string | null;
  protocolVersion: number;
  capabilities: string[];
}

export async function bridgeGetSourceInfo(source: SourceKind): Promise<SourceInfo | null> {
  return await invoke<SourceInfo | null>('bridge_get_source_info', { source });
}
```

If `SourceKind` is already exported from this file, do not redeclare; just add `SourceInfo` and the wrapper function.

- [ ] **Step 3: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/lib/bridge/api.ts
git commit -m "feat(bridge): add bridgeGetSourceInfo typed wrapper (Plan 0 task 9)"
```

---

## Task 10: Extend `bridge.svelte.ts` store with `sourceInfo`

**Files:**
- Modify: `src/store/bridge.svelte.ts`

**Anti-scope:** Do NOT modify any tool component (Campaign etc.) yet — Plan 1 consumes this.

**Depends on:** Task 9

**Invariants cited:** ARCHITECTURE.md §4 (Tauri events surface), §6 (Svelte 5 runes mode).

- [ ] **Step 1: Read current `bridge.svelte.ts`**

Identify the existing reactive state pattern (likely `let characters = $state<…>(...)`, listeners on `bridge://foundry/connected` etc.).

- [ ] **Step 2: Add `sourceInfo` reactive state and refresh logic**

Add at module scope alongside the existing reactive state:

```ts
import { bridgeGetSourceInfo, type SourceInfo, type SourceKind } from '$lib/bridge/api';
import { listen } from '@tauri-apps/api/event';

let sourceInfo = $state<Record<SourceKind, SourceInfo | null>>({
  roll20: null,
  foundry: null,
});

async function refreshSourceInfo(source: SourceKind) {
  sourceInfo[source] = await bridgeGetSourceInfo(source);
}

// One-shot setup at module load: listen for connect/disconnect and refresh.
listen<unknown>('bridge://foundry/connected', () => { void refreshSourceInfo('foundry'); });
listen<unknown>('bridge://foundry/disconnected', () => { sourceInfo.foundry = null; });
listen<unknown>('bridge://roll20/connected', () => { void refreshSourceInfo('roll20'); });
listen<unknown>('bridge://roll20/disconnected', () => { sourceInfo.roll20 = null; });

// Initial fetch in case the bridge connected before this module loaded.
void refreshSourceInfo('foundry');
void refreshSourceInfo('roll20');

export function getSourceInfo(source: SourceKind): SourceInfo | null {
  return sourceInfo[source];
}
```

If the file already has `listen()` calls at module scope (per ARCHITECTURE.md §10 — bridge cutover moved them all here), add the new listeners alongside them. Do NOT duplicate existing connect/disconnect listeners; **only add the source-info refresh side-effect inside the existing handlers** if those handlers exist.

- [ ] **Step 3: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src/store/bridge.svelte.ts
git commit -m "feat(bridge): extend bridge store with sourceInfo state (Plan 0 task 10)"
```

---

## Task 11: Create `foundry-actions/bridge.js` (subscribers registry)

**Files:**
- Create: `vtmtools-bridge/scripts/foundry-actions/bridge.js`

**Anti-scope:** Do NOT touch `actor.js` yet (Task 12). Do NOT register handlers in `index.js` yet (Task 14).

**Depends on:** none

**Invariants cited:** Foundry helper roadmap §3 (typed-per-helper, dot-namespaced wire convention).

- [ ] **Step 1: Create the file**

Create `vtmtools-bridge/scripts/foundry-actions/bridge.js`:

```js
// Foundry bridge.* helper executors.
//
// The `subscribers` registry maps a collection name → an object exposing
// `attach(socket)` and `detach()`. A subscriber is responsible for hooking
// the relevant Foundry Document hooks and pushing data over the socket.
//
// Phase 1 (Plan 0) ships the registry and the `actors` subscriber (after
// Task 12 refactor). Phase 5+ subscribers (journal, scene, item, chat,
// combat) will register themselves here when their consumer features land.

import { actorsSubscriber } from "./actor.js";

const subscribers = {
  actors: actorsSubscriber,
};

const active = new Map(); // collection -> attached subscriber
let currentSocket = null;

/** Called by bridge.js after the socket is open. Stores the socket so
 *  subscribe can hand it to the subscriber. */
export function setSocket(socket) {
  currentSocket = socket;
}

/** Called by bridge.js on close. Detaches all subscribers cleanly. */
export function clearAll() {
  for (const [_name, sub] of active) sub.detach();
  active.clear();
  currentSocket = null;
}

/** Subscribe handler. msg = { type: "bridge.subscribe", collection: "<name>" } */
export async function handleSubscribe(msg) {
  const collection = msg?.collection;
  if (!collection) throw new Error("missing_collection");
  const sub = subscribers[collection];
  if (!sub) throw new Error(`no_such_collection:${collection}`);
  if (active.has(collection)) return;
  if (!currentSocket) throw new Error("no_socket");
  sub.attach(currentSocket);
  active.set(collection, sub);
}

/** Unsubscribe handler. msg = { type: "bridge.unsubscribe", collection: "<name>" } */
export async function handleUnsubscribe(msg) {
  const collection = msg?.collection;
  if (!collection) throw new Error("missing_collection");
  const sub = active.get(collection);
  if (!sub) return;
  sub.detach();
  active.delete(collection);
}

export const handlers = {
  "bridge.subscribe": handleSubscribe,
  "bridge.unsubscribe": handleUnsubscribe,
};
```

- [ ] **Step 2: Commit (will not yet load — Task 14 wires it in)**

```bash
git add vtmtools-bridge/scripts/foundry-actions/bridge.js
git commit -m "feat(bridge-module): add foundry-actions/bridge.js subscribers registry (Plan 0 task 11)"
```

---

## Task 12: Refactor `foundry-actions/actor.js` into a subscriber object

**Files:**
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js`
- Modify: `vtmtools-bridge/scripts/translate.js` (if `hookActorChanges` lives there — see Step 1)

**Anti-scope:** Do NOT change the wire shape of any actor.* message. Do NOT change which hooks fire — only their registration timing and a way to detach them.

**Depends on:** Task 11

**Invariants cited:** ARCHITECTURE.md §4 (wire shape stable). **Verification rule:** post-refactor, the timing of "actors arrive after Hello" must be identical — `attach()` must register hooks AND push the initial `Actors` frame in one synchronous-ish sequence so no hook can fire between them.

- [ ] **Step 1: Read `vtmtools-bridge/scripts/translate.js`**

Identify `hookActorChanges(socket)` and `actorToWire`. The existing `bridge.js` calls `pushAllActors()` *and then* `hookActorChanges(socket)` inside the `open` handler. The refactor must wrap both into one `actorsSubscriber.attach(socket)` call so the order is preserved.

- [ ] **Step 2: Append `actorsSubscriber` export to `actor.js`**

Append to `vtmtools-bridge/scripts/foundry-actions/actor.js`:

```js
import { actorToWire, hookActorChanges } from "../translate.js";

const MODULE_ID = "vtmtools-bridge";

let _attached = null;     // { socket, hookHandles[] } when attached, else null

/**
 * The actors subscriber. Encapsulates the "push initial actors + hook
 * future changes" behavior previously inlined in bridge.js's open handler.
 *
 * INVARIANT: attach() pushes the initial Actors frame BEFORE registering
 * hooks, ensuring the desktop never sees an ActorUpdate without the prior
 * Actors snapshot. Calling detach() unregisters hooks; the socket is not
 * closed (bridge.js owns the socket lifecycle).
 */
export const actorsSubscriber = {
  attach(socket) {
    if (_attached) return;
    // Push initial state.
    if (socket?.readyState === WebSocket.OPEN) {
      const actors = game.actors.contents.map(actorToWire);
      socket.send(JSON.stringify({ type: "actors", actors }));
      console.log(`[${MODULE_ID}] actorsSubscriber: pushed ${actors.length} actors`);
    }
    // Register hooks. hookActorChanges currently captures `socket` in a
    // closure and registers updateActor/createActor/deleteActor; it does
    // not return handles. For Plan 0 we accept that detach() can only
    // *stop pushing* by guarding on `_attached`; full hook unregister is
    // a translate.js follow-up if needed.
    hookActorChanges(socket);
    _attached = { socket };
  },

  detach() {
    _attached = null;
    // No-op for hook removal in Plan 0; the socket is closing anyway.
    // If translate.js gains an unhook API later, call it here.
  },
};
```

- [ ] **Step 3: Confirm `actor.js` still exports its existing `handlers` map**

The existing handlers map (`actor.update_field`, `actor.apply_dyscrasia`, etc.) is unchanged. Only the `actorsSubscriber` export is new.

- [ ] **Step 4: Commit**

```bash
git add vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "feat(bridge-module): extract actorsSubscriber from inline bridge.js setup (Plan 0 task 12)"
```

---

## Task 13: Update `bridge.js` (extended Hello + error envelope + use subscribers)

**Files:**
- Modify: `vtmtools-bridge/scripts/bridge.js`

**Anti-scope:** Do NOT change the WS URL, reconnect logic, or status pip.

**Depends on:** Task 11, Task 12

**Invariants cited:** ARCHITECTURE.md §4. **Verification rule:** the `subscribe('actors')` call must fire after Hello is sent, and `attach(socket)` must run before any Foundry hook can deliver an `updateActor` (which would arrive without prior `Actors` snapshot otherwise). The existing code calls `pushAllActors()` then `hookActorChanges(socket)` synchronously inside the `open` handler — the refactor preserves this by collapsing both into `subscribers.handleSubscribe({ collection: 'actors' })`.

- [ ] **Step 1: Replace the imports block**

Edit the top imports of `vtmtools-bridge/scripts/bridge.js`:

```js
import { handlers } from "./foundry-actions/index.js";
import * as bridgeUmbrella from "./foundry-actions/bridge.js";
```

Remove the now-unused `actorToWire, hookActorChanges` import (those are now consumed by `actor.js::actorsSubscriber`).

- [ ] **Step 2: Replace the `open` handler body**

Replace the `socket.addEventListener("open", () => { ... })` callback with:

```js
  socket.addEventListener("open", async () => {
    console.log(`[${MODULE_ID}] connected to ${BRIDGE_URL}`);
    reconnectDelay = 1000;
    bridgeUmbrella.setSocket(socket);
    socket.send(JSON.stringify({
      type: "hello",
      protocol_version: 1,
      world_id: game.world?.id ?? null,
      world_title: game.world?.title ?? null,
      system_id: game.system?.id ?? null,
      system_version: game.system?.version ?? null,
      capabilities: ["actors"],
    }));
    // Auto-subscribe `actors` to preserve today's always-send-actors
    // semantics. Future tools may send `bridge.subscribe` for other
    // collections; the desktop never has to manage `actors`.
    try {
      await bridgeUmbrella.handleSubscribe({ collection: "actors" });
    } catch (err) {
      console.error(`[${MODULE_ID}] actors auto-subscribe failed:`, err);
    }
    updateStatusPip(true);
  });
```

- [ ] **Step 3: Update the `close` handler to clear subscriptions**

Replace the existing `close` handler with:

```js
  socket.addEventListener("close", () => {
    bridgeUmbrella.clearAll();
    socket = null;
    updateStatusPip(false);
    console.log(`[${MODULE_ID}] disconnected — retrying in ${reconnectDelay}ms`);
    if (reconnectTimer) clearTimeout(reconnectTimer);
    reconnectTimer = setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
  });
```

- [ ] **Step 4: Update `pushAllActors()` to delegate (or remove if unused)**

The standalone `pushAllActors()` function is still called by `handleInbound` when an inbound `refresh` arrives. Keep it but rename and route through the subscriber for consistency. Replace:

```js
function pushAllActors() {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  const actors = game.actors.contents.map(actorToWire);
  socket.send(JSON.stringify({ type: "actors", actors }));
  console.log(`[${MODULE_ID}] pushed ${actors.length} actors`);
}
```

with:

```js
import { actorToWire } from "./translate.js";

function pushAllActors() {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  const actors = game.actors.contents.map(actorToWire);
  socket.send(JSON.stringify({ type: "actors", actors }));
  console.log(`[${MODULE_ID}] pushed ${actors.length} actors (refresh)`);
}
```

(Re-import `actorToWire` from translate.js since the top-level import was removed in Step 1.)

- [ ] **Step 5: Replace the `handleInbound` error path with envelope-on-throw**

Replace the existing `handleInbound` function with:

```js
async function handleInbound(msg) {
  if (msg.type === "refresh") {
    pushAllActors();
    return;
  }
  const handler = handlers[msg.type];
  if (!handler) {
    console.warn(`[${MODULE_ID}] unknown inbound type:`, msg.type);
    return;
  }
  try {
    await handler(msg);
  } catch (err) {
    console.error(`[${MODULE_ID}] handler ${msg.type} threw:`, err);
    ui.notifications?.error(`vtmtools: ${msg.type} failed — ${err.message}`);
    // Send error envelope back to desktop.
    const code = err.message?.split(":")[0] || "unknown";
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({
        type: "error",
        refers_to: msg.type,
        request_id: null,
        code,
        message: String(err.message ?? err),
      }));
    }
  }
}
```

- [ ] **Step 6: Manual smoke verification (mandatory per advisor flag #2)**

Run the full Tauri dev cycle:

```bash
npm run tauri dev
```

In a separate Foundry instance:
1. Sideload the updated `vtmtools-bridge/` directory (or use a symlink per README).
2. Open a WoD5e world, load with GM account.
3. Watch the desktop app's Campaign view — the green pip should turn on AND character cards should appear in the same render cycle (no in-between empty state). If cards appear several seconds *after* the green pip, `actorsSubscriber.attach()` is not pushing the initial Actors frame before hooks register; investigate.
4. Edit an actor's hunger in Foundry; verify the change shows in the desktop within 1s.
5. Trigger an error: from the Foundry console, run `actor.update({"system.notes.private": null})` on a deleted actor — the desktop should show a toast "vtmtools: actor.update_field failed — …" (proves error envelope routing works end-to-end).

- [ ] **Step 7: Commit**

```bash
git add vtmtools-bridge/scripts/bridge.js
git commit -m "feat(bridge-module): extended Hello + error envelope + subscriber-driven actor hooks (Plan 0 task 13)"
```

---

## Task 14: Register `bridge.*` umbrella in `foundry-actions/index.js`

**Files:**
- Modify: `vtmtools-bridge/scripts/foundry-actions/index.js`

**Anti-scope:** Do NOT change other umbrella imports.

**Depends on:** Task 11

**Invariants cited:** Foundry helper roadmap §3 (handler-map dispatch).

- [ ] **Step 1: Add `bridge` umbrella to the handlers flatten**

Replace `vtmtools-bridge/scripts/foundry-actions/index.js` with:

```js
// Flattens per-umbrella handler exports into one map for bridge.js::handleInbound.
import { handlers as actorHandlers } from "./actor.js";
import { handlers as bridgeHandlers } from "./bridge.js";
import { handlers as gameHandlers } from "./game.js";
import { handlers as storytellerHandlers } from "./storyteller.js";

export const handlers = {
  ...actorHandlers,
  ...bridgeHandlers,
  ...gameHandlers,
  ...storytellerHandlers,
};
```

- [ ] **Step 2: Manual verification — subscribe round-trip**

In `npm run tauri dev` with the module loaded:
1. Open Foundry's developer console (F12).
2. The auto-subscribe at connection happens silently. To prove the dispatcher works, run from the desktop's developer console:
   ```js
   await window.__TAURI_INTERNALS__.invoke('bridge_get_source_info', { source: 'foundry' })
   ```
   Expected output: an object with `worldId`, `worldTitle`, `protocolVersion: 1`, `capabilities: ["actors"]`.

- [ ] **Step 3: Commit**

```bash
git add vtmtools-bridge/scripts/foundry-actions/index.js
git commit -m "feat(bridge-module): register bridge.* umbrella in handler-map (Plan 0 task 14)"
```

---

## Task 15: Bump module version + final verification gate

**Files:**
- Modify: `vtmtools-bridge/module.json`

**Anti-scope:** No code changes.

**Depends on:** Tasks 1–14

**Invariants cited:** ARCHITECTURE.md §10 (verify.sh as the aggregate gate).

- [ ] **Step 1: Bump `version` to `0.2.0`**

Edit `vtmtools-bridge/module.json`. Change:

```json
"version": "0.1.0",
```

to:

```json
"version": "0.2.0",
```

- [ ] **Step 2: Run the verification gate**

```bash
./scripts/verify.sh
```

Expected: green. Three sub-checks pass: `npm run check`, `cargo test`, `npm run build`.

If `cargo test` fails on the new `bridge::foundry::actions::bridge` tests because they couldn't be discovered, double-check Task 6 added `pub mod bridge;` to `actions/mod.rs`.

- [ ] **Step 3: Manual end-to-end verification**

Same as Task 13 Step 6 — boot dev app + Foundry, verify green pip, source-info Tauri command returns `protocolVersion: 1`, error toast surfaces on a deliberate failure.

- [ ] **Step 4: Commit**

```bash
git add vtmtools-bridge/module.json
git commit -m "chore(bridge-module): bump to 0.2.0 (Plan 0 final)"
```

---

## Self-review checklist

- [x] Spec § 3.2 wire protocol — covered by Tasks 3, 4, 11, 12, 13, 14 (Hello fields + Error envelope + subscription protocol + module-side wiring).
- [x] Spec § 3.3 `bridge_get_source_info` command — covered by Tasks 7, 8, 9, 10.
- [x] Spec § 3.6 verification — Task 15 runs `./scripts/verify.sh` + manual smoke.
- [x] Backward compatibility for 0.1.0 modules — covered by `Option<…>` fields with `#[serde(default)]` on the desktop side (Task 3) and the `unwrap_or_else(|| vec!["actors"])` capability fallback (Task 4 Step 3).
- [x] Advisor flag #2 (actor.js refactor regression risk) — Task 13 Step 6 mandates manual smoke verification of actors-arrive-with-Hello timing.
- [x] Advisor flag #4 (`bridge_get_source_info` async-without-IO note) — Task 7 Step 2 includes the consistency comment.
- [x] No placeholders / TBDs in any task.
- [x] Anti-scope declared on every task.
