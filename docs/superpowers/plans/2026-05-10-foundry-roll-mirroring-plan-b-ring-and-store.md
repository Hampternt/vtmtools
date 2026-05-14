# Foundry Roll Mirroring — Plan B — BridgeState ring buffer + IPC + frontend store

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded `roll_history` ring buffer to `BridgeState` (capacity 200, dedup by `source_id`), expose it via a `bridge_get_rolls` Tauri command, and create a Svelte runes store (`rolls.svelte.ts`) that primes from IPC and listens for live `bridge://roll-received` events.

**Architecture:** One commit. The ring lives in `BridgeState` next to the existing characters cache. Plan A.2 already emits `bridge://roll-received` from the accept-loop; this plan threads the same emit point through `state.push_roll(roll)` first so the ring stays in sync with the wire event. New IPC command `bridge_get_rolls` snapshots the ring; the frontend store calls it once on mount, then subscribes to the live event for incremental updates. UI consumer follows in Plan C.

**Tech Stack:** Rust (`std::sync::Mutex<VecDeque<...>>` matching existing BridgeState convention), Tauri command + capability, Svelte 5 runes, `@tauri-apps/api/event`.

---

## Required Reading

- `docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md` §6, §7, §8, §10. Cite in commit.
- `docs/superpowers/plans/2026-05-10-foundry-roll-mirroring-plan-a-trait-and-decode.md` — Plan A must be merged. The bridge currently emits `bridge://roll-received` from `bridge/mod.rs`'s accept-loop arm; this plan moves the emit next to the ring-push.
- `ARCHITECTURE.md` §3 (storage strategy — ephemeral state in `BridgeState`), §4 (Tauri events table to update), §5 (typed wrapper requirement), §10 (testing posture).
- `src-tauri/src/bridge/mod.rs` — current `BridgeState` shape, mutex/lock conventions used by existing fields, `accept_loop` event-dispatch.
- `src-tauri/src/bridge/commands.rs` — existing command shape (`bridge_get_characters`, `bridge_get_status`, etc.) to mirror.
- `src-tauri/src/lib.rs` — `invoke_handler` registration site.
- `src/store/bridge.svelte.ts` — runes-mode store pattern to mirror.
- `src/lib/bridge/api.ts` (or wherever `bridge_get_characters` is wrapped) — typed wrapper convention.

## File Structure

```
src-tauri/src/bridge/
├── mod.rs            (MODIFY — add roll_history field, push_roll/get_rolls methods,
│                       move ring-push into accept_loop arm)
└── commands.rs       (MODIFY — add bridge_get_rolls command)

src-tauri/src/
└── lib.rs            (MODIFY — register bridge_get_rolls in invoke_handler)

src/lib/bridge/
└── api.ts            (MODIFY — add bridgeGetRolls typed wrapper)
                      (if api.ts doesn't exist at this path, locate the existing
                       bridge wrapper module — likely src/lib/bridge/api.ts or
                       src/store/bridge.svelte.ts itself.)

src/store/
└── rolls.svelte.ts   (CREATE — runes-mode rolls store)
```

One commit, verified by `./scripts/verify.sh`.

---

## Task 1 — BridgeState ring + IPC command + frontend store

**Files:** as listed in File Structure.

- [ ] **Step 1 (mutex-flavor verification):** Open `src-tauri/src/bridge/mod.rs`. Inspect existing `BridgeState` fields. Note which mutex flavor wraps the in-memory caches (`HashMap<String, CanonicalCharacter>`, `HashMap<SourceKind, ConnectionInfo>`, etc.) — likely `std::sync::Mutex` since the spec §6 noted the cache is read/written from sync command paths. Match the same flavor for `roll_history`. If existing fields use `tokio::sync::Mutex`, use that and adjust the `.lock()` calls below to `.lock().await`.

  Document the verified flavor in a one-line code comment above the new field:

  ```rust
  // roll_history uses std::sync::Mutex to match the existing characters cache.
  ```

- [ ] **Step 2:** Open `src-tauri/src/bridge/mod.rs`. Near the top of the file (after `use` declarations), add the capacity constant:

```rust
/// Capacity of the in-memory roll-history ring. ~80 rolls per typical 4-hour
/// session × 2.5 → 200 covers a session-and-a-half comfortably. Per
/// docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §15
/// open question 4: revisit only if user feedback shows entries dropping
/// mid-session.
const ROLL_HISTORY_CAPACITY: usize = 200;
```

  Add the `VecDeque` import at the top if not already present:

```rust
use std::collections::VecDeque;
```

- [ ] **Step 3:** Find the `BridgeState` struct definition. Add the new field next to the existing in-memory caches:

```rust
pub struct BridgeState {
    // ... existing fields preserved verbatim ...
    pub roll_history: std::sync::Mutex<VecDeque<CanonicalRoll>>,
}
```

  Add `use crate::bridge::types::CanonicalRoll;` if needed.

- [ ] **Step 4:** Find the `BridgeState` constructor (`impl BridgeState { pub fn new(...) -> Self { ... } }`). Initialize the new field:

```rust
roll_history: std::sync::Mutex::new(VecDeque::with_capacity(ROLL_HISTORY_CAPACITY)),
```

  If the constructor uses `Default`, instead implement (or extend) the field's default to the same expression.

- [ ] **Step 5:** Add `push_roll` and `get_rolls` methods on `BridgeState`. Pick a coherent location (likely after the existing characters-cache methods). Add:

```rust
impl BridgeState {
    /// Push a roll into the bounded ring. Newest-first ordering. Dedup by
    /// `source_id` — Foundry occasionally re-fires createChatMessage for the
    /// same message across sockets; pre-removing any existing entry collapses
    /// dupes without losing chronology.
    pub fn push_roll(&self, roll: CanonicalRoll) {
        let mut ring = self.roll_history.lock().expect("roll_history mutex poisoned");
        ring.retain(|r| r.source_id != roll.source_id);
        ring.push_front(roll);
        while ring.len() > ROLL_HISTORY_CAPACITY {
            ring.pop_back();
        }
    }

    /// Snapshot of the ring, newest-first. Cheap clone — capacity 200 of small
    /// structs (Vec<u8> dice arrays + an opaque JSON blob).
    pub fn get_rolls(&self) -> Vec<CanonicalRoll> {
        self.roll_history.lock().expect("roll_history mutex poisoned").iter().cloned().collect()
    }
}
```

  If the existing `BridgeState` impl is wrapped in `impl BridgeState { ... }` (single block), append to it instead of opening a new one — match the file's organization.

- [ ] **Step 6:** Find the `accept_loop` `RollReceived` arm — Plan A.2 set it to:

```rust
InboundEvent::RollReceived(roll) => {
    app_handle.emit("bridge://roll-received", &roll).ok();
}
```

  Update it to push into the ring before emitting:

```rust
InboundEvent::RollReceived(roll) => {
    state.push_roll(roll.clone());
    app_handle.emit("bridge://roll-received", &roll).ok();
}
```

  The `state` reference is already in scope per Plan A.1's accept-loop dispatch shape — verify by reading the function's parameter list. If `state` is named differently (e.g. `bridge_state`), use that.

- [ ] **Step 7:** Open `src-tauri/src/bridge/commands.rs`. Find the existing commands (e.g. `bridge_get_characters`). Add the new command alongside them:

```rust
#[tauri::command]
pub async fn bridge_get_rolls(
    state: tauri::State<'_, std::sync::Arc<crate::bridge::BridgeState>>,
) -> Result<Vec<crate::bridge::types::CanonicalRoll>, String> {
    Ok(state.get_rolls())
}
```

  The exact `tauri::State<'_, ...>` generic type must match the type used by the existing `bridge_get_characters` command — read the file and copy its pattern. If `bridge_get_characters` uses a re-exported alias like `BridgeStateHandle`, reuse it.

- [ ] **Step 8:** Open `src-tauri/src/lib.rs`. Find the `invoke_handler(tauri::generate_handler![...])` macro call. Add `bridge_get_rolls` to the list, alongside the existing `bridge_get_characters`:

```rust
.invoke_handler(tauri::generate_handler![
    // ... existing commands ...
    bridge::commands::bridge_get_characters,
    bridge::commands::bridge_get_rolls,
    // ...
])
```

  Use the same module-prefix pattern as the existing entries (`bridge::commands::bridge_get_characters` if that's how it's referenced; or just `bridge_get_rolls` if the file `use`s the module).

- [ ] **Step 9:** Locate the typed bridge wrapper module. Likely paths:
  - `src/lib/bridge/api.ts`
  - `src/store/bridge.svelte.ts`
  - `src/lib/bridge/commands.ts`

  Open it and find the existing `bridgeGetCharacters` (or similar). Below it add:

```ts
export async function bridgeGetRolls(): Promise<CanonicalRoll[]> {
  return invoke<CanonicalRoll[]>('bridge_get_rolls');
}
```

  Add the type import at the top if not already present:

```ts
import type { CanonicalRoll } from '../../types';
// (or wherever the existing CanonicalCharacter import comes from)
```

- [ ] **Step 10:** Create `src/store/rolls.svelte.ts` with this content:

```ts
// Rolls store — primes from bridge ring on mount, listens for live
// bridge://roll-received events.
//
// Bounded at RING_MAX entries on the frontend too, mirroring the Rust ring.
// Dedup-by-source_id matches Rust's BridgeState::push_roll; if Foundry
// re-emits the same chat-message ID, the existing entry is replaced and
// re-fronted.
//
// See docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §8.

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { bridgeGetRolls } from '$lib/bridge/api'; // adjust import path to match Step 9 location
import type { CanonicalRoll } from '../types';

const RING_MAX = 200;

class RollsStore {
  list = $state<CanonicalRoll[]>([]);
  #unlisten: UnlistenFn | null = null;
  #loaded = false;

  async ensureLoaded() {
    if (this.#loaded) return;
    this.#loaded = true;

    // Prime from the Rust ring snapshot.
    try {
      this.list = await bridgeGetRolls();
    } catch (err) {
      console.error('[rolls] bridge_get_rolls failed:', err);
      this.list = [];
    }

    // Subscribe to live emits.
    try {
      this.#unlisten = await listen<CanonicalRoll>('bridge://roll-received', e => {
        const incoming = e.payload;
        // Dedup by source_id; place newest first.
        this.list = [
          incoming,
          ...this.list.filter(r => r.source_id !== incoming.source_id),
        ].slice(0, RING_MAX);
      });
    } catch (err) {
      console.error('[rolls] listen(bridge://roll-received) failed:', err);
    }
  }

  /** Test/dev hook — clear the in-memory list. Does not affect the Rust ring. */
  clear() { this.list = []; }
}

export const rolls = new RollsStore();
```

  **Adjust the `bridgeGetRolls` import path** to whatever Step 9 chose. If the wrapper lives in `src/store/bridge.svelte.ts` (co-located with the existing bridge store), the import is `import { bridgeGetRolls } from './bridge.svelte'` — match the file's actual location.

- [ ] **Step 11:** Run `cargo test --manifest-path src-tauri/Cargo.toml`. Expected: all existing tests + Plan A's tests still pass; no new tests in this plan but the type-checks confirm `BridgeState`'s shape is internally consistent.

- [ ] **Step 12:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 13 (manual smoke):**
  1. `npm run tauri dev`. Connect Foundry browser.
  2. Open Tauri DevTools console. Verify the rolls store can be primed manually:
     ```js
     const { rolls } = await import('/src/store/rolls.svelte.ts');
     await rolls.ensureLoaded();
     console.log('initial rolls:', $state.snapshot(rolls.list));
     ```
     If `$state.snapshot` isn't available in this devtools context, just log `rolls.list` directly. Initial list is `[]` (no rolls received yet, ring empty).
  3. Roll something in Foundry. Confirm:
     - DevTools logs the `bridge://roll-received` event (from Plan A.2 instrumentation).
     - `rolls.list` length increments by 1 (re-print after a moment).
     - Calling `await bridgeGetRolls()` returns the same single-entry array.
  4. Roll the same dice button twice in quick succession. Foundry generates two distinct chat-message IDs → both appear in the ring (no dedup unless Foundry re-emits with the *same* `_id`, which is rare).
  5. Programmatically force a dedup by calling the bridge twice with a known `source_id` (manual edit only if needed; otherwise trust the unit test in §13 of the spec — but since we have no Rust ring-test in this plan, optionally add one):

  **Optional (recommended) inline ring test.** If time permits, add to the bottom of `src-tauri/src/bridge/mod.rs`:

  ```rust
  #[cfg(test)]
  mod ring_tests {
      use super::*;
      use crate::bridge::types::{CanonicalRoll, RollSplat, SourceKind};
      use serde_json::json;

      fn make_roll(id: &str) -> CanonicalRoll {
          CanonicalRoll {
              source: SourceKind::Foundry,
              source_id: id.into(),
              actor_id: None, actor_name: None, timestamp: None,
              splat: RollSplat::Mortal,
              flavor: String::new(), formula: String::new(),
              basic_results: vec![], advanced_results: vec![],
              total: 0, difficulty: None, criticals: 0,
              messy: false, bestial: false, brutal: false,
              raw: json!({}),
          }
      }

      #[test]
      fn ring_dedups_by_source_id() {
          let state = BridgeState::new(/* match real constructor args */);
          state.push_roll(make_roll("a"));
          state.push_roll(make_roll("b"));
          state.push_roll(make_roll("a"));  // dup of first
          let rolls = state.get_rolls();
          assert_eq!(rolls.len(), 2);
          assert_eq!(rolls[0].source_id, "a", "newest-first; re-pushed 'a' is newest");
          assert_eq!(rolls[1].source_id, "b");
      }

      #[test]
      fn ring_caps_at_capacity() {
          let state = BridgeState::new(/* match real constructor args */);
          for i in 0..(ROLL_HISTORY_CAPACITY + 50) {
              state.push_roll(make_roll(&format!("id_{i}")));
          }
          assert_eq!(state.get_rolls().len(), ROLL_HISTORY_CAPACITY);
      }
  }
  ```

  Adjust `BridgeState::new(...)` to match its actual constructor signature. If the constructor takes parameters that aren't trivially mockable in tests (e.g. an `AppHandle` or a connection sender map), skip these tests — the manual smoke from Step 13.3 verifies the same behavior end-to-end and the unit-test cost isn't justified.

- [ ] **Step 14:** Update the Tauri events table in `ARCHITECTURE.md` §4. Find the existing events table:

  ```
  | Event | Payload | Emitted when |
  |---|---|---|
  | `bridge://roll20/connected` | none | Roll20 extension opens WS connection |
  | ...
  | `bridge://characters-updated` | `Vec<CanonicalCharacter>` | ... |
  ```

  Add a new row after `bridge://characters-updated`:

  ```
  | `bridge://roll-received` | `CanonicalRoll` | Foundry source decoded a `roll_result` chat message into a canonical roll; pushed into bridge state ring (capacity 200, dedup by `source_id`) and emitted in one accept-loop arm |
  ```

  Update the IPC command inventory in §4 too — find the `src-tauri/src/bridge/commands.rs` line that says `(4):` and bump it to `(5):`, adding `bridge_get_rolls` to the comma-list.

- [ ] **Step 15:** Run `./scripts/verify.sh` once more after the doc edit. Expected: green (doc edits don't fail any check, but the gate is the gate).

- [ ] **Step 16:** Commit.

```bash
git add src-tauri/src/bridge/mod.rs src-tauri/src/bridge/commands.rs src-tauri/src/lib.rs src/lib/bridge/api.ts src/store/rolls.svelte.ts ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
feat(bridge): roll_history ring + bridge_get_rolls IPC + rolls store

BridgeState gains a bounded VecDeque<CanonicalRoll> (capacity 200,
dedup-by-source_id, newest-first). The accept_loop's RollReceived arm
pushes into the ring before emitting bridge://roll-received. New
bridge_get_rolls Tauri command snapshots the ring; src/store/rolls.svelte.ts
primes from it on mount and subscribes to live event for incremental updates.

Foundry's chat log remains the durable record per spec §1; persistence
deferred to a future spec without protocol break.

ARCHITECTURE.md §4 events table + commands inventory updated.

Per docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §6, §7, §8.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Checklist (run after Task 1 commit)

- [ ] **Spec coverage:** §6 ring + dedup + capacity 200 ✓. §7 `bridge_get_rolls` IPC + typed wrapper ✓. §8 frontend store with prime + listen + dedup + RING_MAX cap ✓. §10 events-table doc updated ✓.
- [ ] **Anti-scope respected:** No `BridgeSource` trait shape change (frozen by Plan A.1). No `CanonicalRoll` shape change (frozen by Plan A.1). No Foundry hook JS change (frozen by Plan A.2). No new tools or UI files (Plan C).
- [ ] **Mutex flavor consistent:** `roll_history` uses the same flavor as the rest of `BridgeState` (verified Step 1). Lock acquisitions use the matching API (`.lock()` for std, `.lock().await` for tokio). The `expect("...")` panic message is reasonable per ARCH §7 ("panics in command paths are bugs, not error flow").
- [ ] **Type consistency:** TS `CanonicalRoll` (Plan A) is the type returned by `bridgeGetRolls`. Field access in `rolls.svelte.ts` (`r.source_id`) matches the TS interface.
- [ ] **No frontend tests added** — ARCH §10 invariant. The Rust ring tests are optional (Step 13 caveat) and skipped if `BridgeState::new` is hard to mock.
- [ ] **`./scripts/verify.sh`** green for the commit.

## Open questions (deferred from spec)

- **`bridge://roll-history` reconnect-replay event** — reserved by name in spec §10; not wired in v1. A future spec adds it.
- **Persistence layer (SQLite)** — explicitly out of v1 scope (project-owner decision). The wire shape and store API are forward-compatible; a future spec can layer SQLite without protocol break.
