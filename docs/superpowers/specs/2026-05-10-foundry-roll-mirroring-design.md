# Foundry Roll Mirroring — Inbound roll-feed with VTT-pluggable seam

> **Status:** designed; ready for plan-writing.
> **Roadmap fit:** Phase 3 from `2026-04-30-character-tooling-roadmap.md` §5 (roll mirroring). Sibling track of `2026-05-10-card-modifier-coverage-finish-design.md` (independent files, parallelizable). Complement to the existing outbound roll dispatcher (`rollV5Pool` in vtmtools-bridge, GM-screen `RollDispatcherPopover`).
> **Audience:** anyone implementing the bridge-trait migration, Foundry-side hook, Rust translator, or new "Rolls" tool.
> **Source reference:** `docs/reference/foundry-vtm5e-rolls.md` §"Wire integration sketch" (the inbound side). `docs/reference/foundry-vtm5e-roll-sample.json` (live-captured shape).

---

## §1 What this is

When a roll resolves in the Foundry chat (sheet button, GM dialog, vtmtools-dispatched), Foundry's `createChatMessage` hook fires a `ChatMessage` carrying the dice array. This spec subscribes to that hook in the `vtmtools-bridge` Foundry module, pipes the roll across the existing `wss://localhost:7424` bridge, decodes it Rust-side into a source-agnostic `CanonicalRoll`, and surfaces it in a new "Rolls" tool as a reverse-chronological feed.

The load-bearing decision is the **`BridgeSource` trait migration** from `Vec<CanonicalCharacter>` to `Vec<InboundEvent>`. This is a real breaking change (Roll20Source is touched even though Roll20 doesn't gain a feature) but is the structural seam that lets future VTTs (Hunter games, Werewolf chronicles, future Roll20 chat-scraping) emit roll events with no protocol redesign. Per user's standing extensibility preference + ARCH §9 "Add a VTT bridge source" seam.

History persistence is **ephemeral** per project-owner decision: a bounded ring buffer (capacity 200) lives in `BridgeState`, lost on app restart. Foundry's chat log is the durable record. A future spec can layer SQLite persistence onto the same wire shape with no protocol break.

## §2 Composition — what this builds on

| Piece | What it provides | How this spec uses it |
|---|---|---|
| `src-tauri/src/bridge/source.rs` | `BridgeSource` trait, currently 3 methods | `handle_inbound` return type changes from `Vec<CanonicalCharacter>` to `Vec<InboundEvent>`. |
| `src-tauri/src/bridge/mod.rs` | `accept_loop`, `BridgeState`, `bridge://characters-updated` event emission | New `roll_history` ring on state; new `bridge://roll-received` and `bridge://roll-history` events; accept-loop dispatches per-variant. |
| `src-tauri/src/bridge/foundry/translate.rs` | `to_canonical(actor)` decoder pattern (small, clean) | Pattern mirrored in new `translate_roll.rs`. |
| `src-tauri/src/bridge/foundry/types.rs` | `FoundryInbound` enum (`Actors`, `ActorUpdate`, `Hello`) | Gains a `RollResult { message: FoundryRollMessage }` variant. |
| `vtmtools-bridge/scripts/bridge.js` | Init via `Hooks.once("ready")`; `socket.send(JSON.stringify(...))` wire pattern | New `foundry-hooks/rolls.js` `init(getSocket)` is wired here. |
| `docs/reference/foundry-vtm5e-rolls.md` | Pre-sketched wire shape; splat-detection regex; live ChatMessage sample reference | Authoritative input — this spec ratifies the sketch and decides field names. |
| `docs/reference/foundry-vtm5e-roll-sample.json` | Live-captured ChatMessage with the `rolls[0]` shape | Plan must read this to nail field shapes (e.g. is `total` integer? is `formula` always present? what does `dice` look like?). |
| `src/store/bridge.svelte.ts` | `listen()` + runes-mode list pattern | New `src/store/rolls.svelte.ts` mirrors this exactly. |
| `src/tools.ts` | Tool registry — add-a-tool seam (ARCH §9) | New entry registers the "Rolls" tool. |

## §3 BridgeSource trait migration (Phase 0)

### §3.1 Trait change

Replace `bridge/source.rs`:

```rust
use crate::bridge::types::{CanonicalCharacter, CanonicalRoll};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum InboundEvent {
    /// Source pushed an updated set of characters. Replaces, doesn't merge.
    /// Empty Vec means "no character data in this message" — not a clear signal.
    CharactersUpdated(Vec<CanonicalCharacter>),
    /// Source pushed a roll result (Foundry: createChatMessage hook with rolls[]).
    RollReceived(CanonicalRoll),
}

#[async_trait]
pub trait BridgeSource: Send + Sync {
    /// Parse one inbound JSON frame. Sources may emit zero, one, or many
    /// events per frame (e.g. an `actors` snapshot is one CharactersUpdated;
    /// a `hello` frame yields nothing).
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String>;

    fn build_set_attribute(&self, source_id: &str, name: &str, value: &str) -> Result<Value, String>;
    fn build_refresh(&self) -> Value;
}
```

### §3.2 Roll20Source migration

Mechanical rewrite. Every existing `Ok(chars)` in `Roll20Source::handle_inbound` becomes `Ok(vec![InboundEvent::CharactersUpdated(chars)])`. Return-type signature flips. No behavior change.

### §3.3 FoundrySource migration (Phase 0 part)

Same mechanical rewrite as Roll20 — Phase 0 lands without the roll-decoding arm. The `RollResult` enum variant in `FoundryInbound` and the corresponding match arm in `handle_inbound` land in §5 (Phase 2 of this spec's plan).

### §3.4 accept_loop dispatch

In `bridge/mod.rs::accept_loop`, the `for event in source.handle_inbound(msg).await?` loop becomes:

```rust
match event {
    InboundEvent::CharactersUpdated(chars) => {
        // existing merge + emit bridge://characters-updated
    }
    InboundEvent::RollReceived(roll) => {
        state.push_roll(roll.clone());
        app_handle.emit("bridge://roll-received", &roll).ok();
    }
}
```

`state.push_roll` is the bounded-ring-pusher in §6.

### §3.5 Atomic commit

Per `feedback_atomic_cluster_commits`: the trait change + Roll20 migration + Foundry migration + `accept_loop` dispatch are committed in **one** commit. Intermediate states break the build. Verification: `./scripts/verify.sh` green; manual smoke test: connect Foundry GM browser → actors arrive in Campaign view (CharactersUpdated path still works); refresh works; `set_attribute` works.

## §4 CanonicalRoll shape

In `bridge/types.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RollSplat {
    Mortal,
    Vampire,
    Werewolf,
    Hunter,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalRoll {
    pub source: SourceKind,
    /// Stable per-roll ID — Foundry: chat-message `_id`. Used for ring dedup
    /// (Foundry occasionally re-emits createChatMessage for the same message
    /// across sockets; this collapses dupes).
    pub source_id: String,
    /// Foundry actor ID (or Roll20 char ID, when implemented). Optional —
    /// some chat rolls (GM no-actor rolls) carry no actor.
    pub actor_id: Option<String>,
    /// Display name resolved at receive-time. Optional — fallback to "GM" or
    /// "Anonymous" downstream.
    pub actor_name: Option<String>,
    /// ISO-8601 timestamp from Foundry's `message.timestamp`. The bridge
    /// does not invent timestamps — if absent, `None` and the UI uses
    /// receive-time.
    pub timestamp: Option<String>,
    pub splat: RollSplat,
    /// Foundry: `message.flavor`. Roll20: TBD if/when added.
    pub flavor: String,
    /// Raw formula string e.g. "12dv cs>5 + 0dg cs>5". Used by the UI for
    /// debug/devmode display and by future tooling that wants to re-execute.
    pub formula: String,
    pub basic_results: Vec<u8>,
    pub advanced_results: Vec<u8>,
    /// Total successes after V5 critical bonus (every two 10s = +2).
    pub total: u32,
    pub difficulty: Option<u32>,
    /// Count of natural 10s on basic dice + 10s on hunger dice (messy crits
    /// counted here AND in messy below). Display: "X criticals".
    pub criticals: u32,
    /// Vampire: ≥1 natural 10 on hunger dice → messy critical visual treatment.
    pub messy: bool,
    /// Vampire: ≥1 natural 1 on hunger dice → bestial failure visual treatment.
    pub bestial: bool,
    /// Werewolf: ≥1 brutal/critfail on rage dice → brutal visual treatment.
    pub brutal: bool,
    /// Full Foundry ChatMessage blob — opaque. Per
    /// `feedback_dont_filter_bridge_payload`, ship the entire payload so
    /// downstream consumers can select fields without protocol changes.
    pub raw: serde_json::Value,
}
```

Field rationale:
- `source_id` reuses the chat-message ID so the ring's natural dedup-by-key handles Foundry's occasional message re-emission.
- `messy` / `bestial` / `brutal` / `criticals` are **computed by the Rust translator** from `basic_results` + `advanced_results` — `rollMessageData` is NOT persisted on the message (per the rolls reference doc §Chat message shape, line 127).
- `timestamp` is `Option` so the bridge never invents data. If a future Foundry version drops the field, the UI falls back to receive-time.
- `raw` carries the full `ChatMessage` JSON blob so a future feature (e.g., re-render Foundry's HTML, replay the dice animation) needs no protocol bump.

## §5 Foundry-side hook

### §5.1 New file `vtmtools-bridge/scripts/foundry-hooks/rolls.js`

```js
const SPLAT_REGEX = {
  vampire: /\bd[vg]\b/,
  werewolf: /\bd[wr]\b/,
  hunter: /\bd[hs]\b/,
  mortal: /\bdm\b/,
};

function detectSplat(formula) {
  if (!formula) return 'unknown';
  if (SPLAT_REGEX.vampire.test(formula)) return 'vampire';
  if (SPLAT_REGEX.werewolf.test(formula)) return 'werewolf';
  if (SPLAT_REGEX.hunter.test(formula)) return 'hunter';
  if (SPLAT_REGEX.mortal.test(formula)) return 'mortal';
  return 'unknown';
}

function messageToWire(message) {
  const roll = message.rolls?.[0];
  if (!roll) return null;
  return {
    message_id: message._id ?? message.id,
    actor_id: message.speaker?.actor ?? null,
    actor_name: message.speaker?.alias ?? null,
    timestamp: typeof message.timestamp === 'number'
      ? new Date(message.timestamp).toISOString()
      : null,
    flavor: message.flavor ?? '',
    formula: roll.formula ?? '',
    splat: detectSplat(roll.formula),
    basic_results: extractDiceResults(roll, 'basic'),
    advanced_results: extractDiceResults(roll, 'advanced'),
    total: roll.total ?? 0,
    difficulty: roll.options?.difficulty ?? null,
    raw: message.toJSON ? message.toJSON() : message,
  };
}

function extractDiceResults(roll, kind) {
  // Walk roll.terms[] for Die instances; partition by die.denomination
  // (basic: m/v/w/h, advanced: g/r/s). Returns array of result.result values.
  const out = [];
  const basicDenoms = new Set(['m', 'v', 'w', 'h']);
  const advDenoms = new Set(['g', 'r', 's']);
  const want = kind === 'basic' ? basicDenoms : advDenoms;
  for (const term of roll.terms ?? []) {
    if (term.constructor?.name !== 'Die' && term.class !== 'Die') continue;
    if (!want.has(term.denomination)) continue;
    for (const r of term.results ?? []) {
      if (typeof r.result === 'number') out.push(r.result);
    }
  }
  return out;
}

export function init(getSocket) {
  Hooks.on('createChatMessage', (message, _options, _userId) => {
    if (!message.rolls?.length) return;
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    const wire = messageToWire(message);
    if (!wire) return;
    socket.send(JSON.stringify({ type: 'roll_result', message: wire }));
  });
}
```

### §5.2 Wiring from `bridge.js`

In `Hooks.once("ready", ...)`, after the existing actor-subscribe block:

```js
import { init as initRollsHook } from "./foundry-hooks/rolls.js";
// …
initRollsHook(() => socket);
```

The closure passes the live socket reference. Hook stays registered for the session — no teardown needed (Foundry tears it down on world reload).

### §5.3 Splat detection rationale

Per rolls reference doc §Splat detection (line 130), `roll.system` is `undefined` post-rehydration. The formula regex is the robust signal. JS-side detects + ships the splat string; Rust-side defensively re-detects from `formula` to verify the wire field — if mismatch, log + trust the Rust version (defensive against a JS-side bug).

## §6 Bridge state — bounded ring

### §6.1 State change

In `BridgeState` (`bridge/mod.rs`):

```rust
pub struct BridgeState {
    // existing fields
    pub roll_history: Mutex<VecDeque<CanonicalRoll>>,
}
```

Capacity constant: `const ROLL_HISTORY_CAPACITY: usize = 200;` Documented at module top.

**Mutex choice:** match what existing `BridgeState` fields use (verify during plan-writing — likely `std::sync::Mutex` since the cache is read/written from sync command paths and short-held in async accept-loop). `push_roll` and `get_rolls` never `.await` while holding the lock, so either flavor works; pick consistent with the rest of the struct.

### §6.2 push_roll method

```rust
impl BridgeState {
    pub fn push_roll(&self, roll: CanonicalRoll) {
        let mut ring = self.roll_history.lock().unwrap();
        // Dedup by source_id — Foundry occasionally re-fires createChatMessage.
        ring.retain(|r| r.source_id != roll.source_id);
        ring.push_front(roll);
        while ring.len() > ROLL_HISTORY_CAPACITY {
            ring.pop_back();
        }
    }

    pub fn get_rolls(&self) -> Vec<CanonicalRoll> {
        self.roll_history.lock().unwrap().iter().cloned().collect()
    }
}
```

Front-pushing keeps newest-first ordering; the UI reads in arrival order.

## §7 Tauri commands

In `bridge/commands.rs`:

```rust
#[tauri::command]
pub async fn bridge_get_rolls(state: State<'_, Arc<BridgeState>>) -> Result<Vec<CanonicalRoll>, String> {
    Ok(state.get_rolls())
}
```

Registered in `lib.rs::invoke_handler(tauri::generate_handler![...])` next to `bridge_get_characters`.

Typed wrapper in `src/lib/bridge/api.ts` (or wherever `bridge_get_characters` is wrapped — locate during plan-writing):

```ts
export async function bridgeGetRolls(): Promise<CanonicalRoll[]> {
  return invoke<CanonicalRoll[]>('bridge_get_rolls');
}
```

## §8 Frontend store

`src/store/rolls.svelte.ts` mirrors `bridge.svelte.ts`:

```ts
import { listen } from '@tauri-apps/api/event';
import { bridgeGetRolls } from '$lib/bridge/api';
import type { CanonicalRoll } from '../types';

const RING_MAX = 200;

class RollsStore {
  list = $state<CanonicalRoll[]>([]);

  async ensureLoaded() {
    if (this.list.length > 0) return;
    this.list = await bridgeGetRolls();
    listen<CanonicalRoll>('bridge://roll-received', e => {
      const incoming = e.payload;
      // Dedup-by-source_id mirrors backend's push_roll.
      this.list = [incoming, ...this.list.filter(r => r.source_id !== incoming.source_id)]
        .slice(0, RING_MAX);
    });
  }

  clear() { this.list = []; }
}

export const rolls = new RollsStore();
```

`bridge://roll-history` (full-ring snapshot on reconnect) is **not** wired in v1 — `ensureLoaded` already primes via `bridgeGetRolls`. A future spec adding live reconnect-replay can wire it.

## §9 New "Rolls" tool

### §9.1 Tools registry entry

`src/tools.ts`:

```ts
{
  id: 'rolls',
  label: 'Rolls',
  icon: '🎲',
  component: () => import('./tools/RollFeed.svelte'),
}
```

### §9.2 RollFeed.svelte

New `src/tools/RollFeed.svelte`. Layout sketch:

```
┌── toolbar ─────────────────────────────────────────────────┐
│ Rolls                          [splat ▼] [actor ▼] [clear] │
├── feed (reverse-chronological, scrolling) ─────────────────┤
│ ┌─ entry ────────────────────────────────────────────────┐ │
│ │ ⛧  Strength + Brawl                              [3] ✓ │ │
│ │    Doe, John (vampire) · 12s ago                       │ │
│ │    [⚀⚁⚂⚃⚄⚅] [⚄⚅] · diff 4 · 1 crit · messy            │ │
│ └────────────────────────────────────────────────────────┘ │
│ ⋯                                                          │
└────────────────────────────────────────────────────────────┘
```

Per entry:
- Splat-coded gutter dot (vampire red, werewolf umber, hunter gold, mortal slate).
- Header line: flavor (left), success-count badge (right; styled per outcome — fail/success/messy/bestial gradient).
- Sub-header: actor name + splat label + relative time.
- Dice grid: basic dice + advanced dice colored per result class (success / critical / failure / bestial / messy / brutal). Reference per-die-class table in `docs/reference/foundry-vtm5e-rolls.md:147-153`.
- Optional difficulty + criticals + outcome flags strip.

Filter bar: per-actor dropdown (derived from `rolls.list`), per-splat dropdown, "clear filter". The dropdowns operate on the live list — no backend filter call.

Empty state: `"No rolls yet — when a roll resolves in Foundry, it appears here."`

Density: dossier aesthetic optional. v1 uses existing site palette tokens (per ARCH §6) — slate-blue accents, red criticals, the established surface tokens. No new tokens.

### §9.3 RollEntry.svelte (extracted component)

If RollFeed grows past ~250 lines, split per-row into `src/lib/components/RollEntry.svelte` consuming `roll: CanonicalRoll` as a prop. Plan-writer's call.

## §10 Bridge events

| Event | Payload | Emitted when |
|---|---|---|
| `bridge://roll-received` (NEW) | `CanonicalRoll` | Foundry-source decoded a `roll_result` message. One emit per roll. |
| `bridge://characters-updated` (existing) | `Vec<CanonicalCharacter>` | unchanged |
| `bridge://foundry/connected` (existing) | none | unchanged |
| `bridge://foundry/disconnected` (existing) | none | unchanged |

`bridge://roll-history` is reserved by name for future live-reconnect-replay; not wired in v1.

## §11 Plan packaging — three plans

### Plan A — Trait migration + Foundry roll decode (two commits, one plan)

**Plan A.1 — Trait migration + CanonicalRoll struct (atomic protocol change).**

Files: `bridge/source.rs`, `bridge/mod.rs` (accept_loop dispatch), `bridge/types.rs` (CanonicalRoll), `bridge/roll20/mod.rs` (wrap returns), `bridge/foundry/mod.rs` (wrap returns; no roll arm yet), `src/types.ts` (CanonicalRoll mirror).

**Atomic commit per `feedback_atomic_cluster_commits`** — intermediate states break the bridge runtime (return-type mismatch breaks compile). Single combined commit. Verification: `./scripts/verify.sh` green; manual: Foundry actors still arrive in Campaign view; Roll20 still works; no roll-receiving yet (expected — A.2 wires it).

**Plan A.2 — Foundry roll decode (additive).**

Files: `bridge/foundry/types.rs` (FoundryInbound::RollResult variant + FoundryRollMessage shape), `bridge/foundry/mod.rs` (handle_inbound match arm calls translator), NEW `bridge/foundry/translate_roll.rs` (with `#[cfg(test)] mod tests`), NEW `vtmtools-bridge/scripts/foundry-hooks/rolls.js`, `vtmtools-bridge/scripts/bridge.js` (init the new hook).

Verification: `./scripts/verify.sh` green (cargo tests cover the Rust translator); manual: connect Foundry browser; roll a vampire 7-die check via the sheet button; check Tauri stdout for `bridge://roll-received` emit; confirm `CanonicalRoll` payload has correct `total`, splat, flavor.

### Plan B — Bounded ring + IPC commands + frontend store + bridge events

**Files modified:** `bridge/mod.rs` (BridgeState ring), `bridge/commands.rs` (`bridge_get_rolls`), `lib.rs` (register command), `src/lib/bridge/api.ts` (typed wrapper), NEW `src/store/rolls.svelte.ts`, `src/types.ts` (already has CanonicalRoll from Plan A).

**Single commit.** Verification: with Plan A merged, run a Foundry roll → check `rolls.list` populates in devtools (instantiate the store from a temporary debug button if needed; cleanup before commit). `./scripts/verify.sh` green.

### Plan C — RollFeed.svelte UI tool

**Files modified:** NEW `src/tools/RollFeed.svelte`, possibly `src/lib/components/RollEntry.svelte`, `src/tools.ts`, possibly `src/routes/+layout.svelte` if any new tokens are needed (default: none — reuse existing).

**Single commit.** Verification: launch the app, navigate to Rolls tool, roll 3 different splats in Foundry, confirm each renders with correct outcome class. `./scripts/verify.sh` green.

### Anti-scope per plan

| Plan | MUST NOT touch |
|---|---|
| A (A.1+A.2) | `BridgeState.roll_history` (Plan B), Tauri commands (Plan B), `src/store/rolls.svelte.ts` (Plan B), any frontend tool files (Plan C) |
| B | `BridgeSource` trait shape (frozen by A.1), `CanonicalRoll` shape (frozen by A.1), Foundry hook JS (frozen by A.2), Rust translator (frozen by A.2), UI files (Plan C) |
| C | All Rust files (frozen by A+B), bridge JS (frozen by A), `rolls.svelte.ts` API surface (frozen by B) |

### Invariants cited

- All plans: ARCH §3 (storage strategy — ephemeral state in `BridgeState`, no SQLite), §4 (typed wrappers, no `invoke()` in components), §5 (only `bridge/*` binds the WS port; only `bridge/foundry/*` decodes Foundry shapes), §6 (color tokens, dark-only, `box-sizing: border-box`).
- Plan A: ARCH §10 (`#[cfg(test)] mod tests` for the new translator — see §13). ADR 0006 for the new SourceKind / source-pluggable convention.

### Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

## §12 Error handling

Per ARCH §7:

| Failure | Surfaces as |
|---|---|
| Foundry hook fires but socket is closed | JS-side: silently drops (rolls accumulate as Foundry's chat log; bridge picks up on reconnect). No retry, no buffering. |
| Foundry sends malformed `roll_result` | Rust-side: `to_canonical_roll` returns `Err`; `accept_loop` logs the error and skips the roll. The other source's traffic is unaffected. No frontend toast — frontend just doesn't get this event. |
| Rust-side splat-regex mismatch with JS-side | Logged as warning; trust Rust version. |
| `bridge_get_rolls` IPC failure | Frontend `rolls.svelte.ts` catches in `ensureLoaded`; logs to console; renders empty feed (no toast — empty is the same as "no rolls yet"). |
| `roll_history` mutex poisoned | Use `lock().unwrap()` per existing convention (panics on poison are bugs, not error flow — ARCH §7 and the bridge-state pattern in `mod.rs`). |

## §13 Testing

Per ARCH §10:

- **Rust unit tests**: `bridge/foundry/translate_roll.rs` gets `#[cfg(test)] mod tests` covering:
  - vampire 7-die roll, 0 hunger, 3 successes, 1 crit (no messy/bestial)
  - vampire 5-die roll + 3 hunger with 1 natural 10 on hunger → messy = true
  - vampire 5-die roll + 3 hunger with 1 natural 1 on hunger → bestial = true
  - werewolf 6-die roll + 2 rage with 1 brutal → brutal = true
  - V5 critical bonus: two 10s on basic → +2 successes added
  - splat detection: each formula pattern (`dv`, `dw`, `dh`, `dm`) yields correct `RollSplat`
  - empty `basic_results` + non-empty `advanced_results` (rare but legal) → no panic
- **Rust serde round-trip**: `bridge/types.rs` gets a `#[cfg(test)]` test serializing/deserializing `CanonicalRoll` to ensure the wire shape is stable.
- **No Svelte component tests** (per ARCH §10 — no frontend test framework).
- **Manual UI verification**: every Plan C task ends with a Foundry-side roll smoke-test verifying RollFeed renders correctly across splats.
- `./scripts/verify.sh` green for every plan's final commit.

## §14 Out of scope / future seams

| Future feature | How this spec accommodates it |
|---|---|
| **Persistent SQLite roll history** | The wire shape is forward-compatible. A future `db/rolls.rs` module + migration + `bridge_save_roll` command can layer onto the existing event flow with no protocol change. The frontend store keeps the same API; only its `ensureLoaded` source flips from `bridgeGetRolls` to a paginated DB query. |
| **Roll20 chat scraping** | Roll20Source's `handle_inbound` can emit `RollReceived` events too — no protocol or UI change needed. The DOM extension would need to detect chat messages and forward them, which is per-VTT scope and not on the v1 roadmap. |
| **Outbound roll trigger from RollFeed** | Each entry could carry a "re-roll" button calling the existing `rollV5Pool` outbound. Adds a new IPC + button; UI hook is trivial. |
| **Roll analytics / session statistics** | The bounded ring is browseable; a future "Session" tool could aggregate. With persistent SQLite, cross-session analytics. |
| **Live reconnect-replay** | `bridge://roll-history` event reserved by name; future spec wires it. |
| **Click roll → highlight character on GM Screen** | Reuses the `navigate-to-character` event introduced in `2026-05-10-card-modifier-coverage-finish-design.md` §8. The roll entry's actor section becomes a clickable dispatcher. |
| **Hunter / Werewolf splat-specific UI treatments** | `RollSplat` enum already covers them; per-splat visual differences are a CSS pass on RollEntry, not a new architecture. |

## §15 Open questions

1. **`roll.terms[]` shape post-rehydration.** The JS `extractDiceResults` walks `roll.terms` filtering by `Die` denomination. If Foundry's serializer flattens differently (e.g., a `PoolTerm` wrapping multiple `Die` terms), the walker may miss some dice. **Plan A Step 1** verifies against the live sample (`docs/reference/foundry-vtm5e-roll-sample.json`) before any code lands. If the shape is non-trivial, the walker grows to handle it.
2. **Difficulty source.** The wire sketch in the rolls reference doc shows `difficulty` as a top-level field; the live sample might carry it on `roll.options.difficulty`, on the message itself, or buried in `flags.wod5e`. Plan A Step 1 verifies. If absent, `difficulty: None` and the UI renders as "no difficulty".
3. **Speaker actor mismatch.** `message.speaker.actor` may not always populate — chat narration messages, `/r 1d20` console rolls, etc. The translator handles `Option`; the UI shows "GM" or "Anonymous" as a fallback. Confirm the fallback wording during Plan C.
4. **Capacity tuning.** 200 entries is a guess. With ~5 rolls/hour per actor over a 4-hour session = 80 rolls/session — 200 covers a session-and-a-half comfortably. Revisit only if user feedback shows entries dropping mid-session.

## §16 Phase placement

Phase 3 from `2026-04-30-character-tooling-roadmap.md` §5. Goes on the GitHub Project board as **one feature-level parent issue**: "Foundry roll mirroring (inbound)". Plans A, B, C render as task-list checkboxes in the parent body, not separate board entries (per `feedback_issue_granularity`).
