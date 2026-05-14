# Foundry Roll Mirroring — Plan A — BridgeSource trait migration + Foundry roll decode

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the `BridgeSource` trait from `Vec<CanonicalCharacter>` to `Vec<InboundEvent>` (atomic protocol change), then add the additive Foundry-side `roll_result` wire message + JS hook + Rust translator that emits `bridge://roll-received` events.

**Architecture:** Two commits. **A.1** is an atomic protocol-shape change touching every existing `BridgeSource` impl (Roll20 + Foundry) and the accept-loop dispatcher in one commit — intermediate states break the build. **A.2** is purely additive: Foundry gains a `RollResult` arm in its inbound enum, a JS hook subscribing to `createChatMessage`, and a Rust translator that decodes `roll.formula` + dice arrays into a source-agnostic `CanonicalRoll`. Bridge state ring (Plan B) and UI (Plan C) follow.

**Tech Stack:** Rust (`async-trait`, `serde`, `serde_json`), Foundry V11+ system module JS, V5 dice formula parsing.

---

## Required Reading

- `docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md` — the spec; cite §3, §4, §5 in commits.
- `docs/reference/foundry-vtm5e-rolls.md` — splat detection, message shape, hook order.
- `docs/reference/foundry-vtm5e-roll-sample.json` — live ChatMessage capture; **read this before Task 2 to lock the wire shape**.
- `ARCHITECTURE.md` §2 (bridge domain), §4 (events), §5 (module boundaries — only `bridge/*` decodes per-source shapes), §10 (testing — `#[cfg(test)] mod tests` inline).
- `src-tauri/src/bridge/source.rs` — current trait (39 lines).
- `src-tauri/src/bridge/mod.rs` — accept_loop, BridgeState, event emission.
- `src-tauri/src/bridge/types.rs` — CanonicalCharacter, SourceKind, HealthTrack.
- `src-tauri/src/bridge/foundry/mod.rs` — current FoundrySource impl.
- `src-tauri/src/bridge/foundry/types.rs` — current FoundryInbound enum.
- `src-tauri/src/bridge/foundry/translate.rs` — actor-translation pattern to mirror.
- `src-tauri/src/bridge/roll20/mod.rs` — Roll20Source impl (will need wrap-and-rethrow migration).
- `vtmtools-bridge/scripts/bridge.js` — Foundry-side init + socket lifecycle.
- `vtmtools-bridge/scripts/translate.js` — actor-to-wire pattern, may inform messageToWire.

## File Structure

```
src-tauri/src/bridge/
├── source.rs            (MODIFY — handle_inbound returns Vec<InboundEvent>; add InboundEvent enum)
├── mod.rs               (MODIFY — accept_loop dispatches per variant; emit bridge://roll-received)
├── types.rs             (MODIFY — add CanonicalRoll struct + RollSplat enum)
├── roll20/
│   └── mod.rs           (MODIFY — wrap returns into CharactersUpdated)
└── foundry/
    ├── mod.rs           (MODIFY — wrap returns; add RollResult match arm in A.2)
    ├── types.rs         (MODIFY — add RollResult variant + FoundryRollMessage in A.2)
    └── translate_roll.rs (CREATE in A.2 — to_canonical_roll + #[cfg(test)] tests)

src/
└── types.ts             (MODIFY — mirror CanonicalRoll + RollSplat)

vtmtools-bridge/scripts/
├── bridge.js            (MODIFY in A.2 — wire foundry-hooks/rolls.js init)
└── foundry-hooks/
    └── rolls.js         (CREATE in A.2)
```

Two commits, each verified by `./scripts/verify.sh`.

---

## Task 1 (commit A.1) — Trait migration + CanonicalRoll struct

**Files:**
- Modify: `src-tauri/src/bridge/source.rs` (full rewrite of trait + new InboundEvent)
- Modify: `src-tauri/src/bridge/types.rs` (add CanonicalRoll + RollSplat)
- Modify: `src-tauri/src/bridge/mod.rs` (accept_loop dispatch over events)
- Modify: `src-tauri/src/bridge/roll20/mod.rs` (wrap returns)
- Modify: `src-tauri/src/bridge/foundry/mod.rs` (wrap returns; no decode arm yet)
- Modify: `src/types.ts` (mirror CanonicalRoll + RollSplat)

- [ ] **Step 1:** Open `src-tauri/src/bridge/types.rs`. After the existing `CanonicalCharacter` struct, append:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RollSplat {
    Mortal,
    Vampire,
    Werewolf,
    Hunter,
    Unknown,
}

/// A source-agnostic roll result. Each source's `BridgeSource` impl
/// decodes its own raw chat-message / roll shape into this canonical form.
///
/// `source_id` reuses the source's stable per-roll ID (Foundry: chat
/// `_id`); the bridge ring dedups by this key.
///
/// `messy` / `bestial` / `brutal` / `criticals` are computed by the
/// translator from `basic_results` + `advanced_results` — Foundry does
/// NOT persist `rollMessageData` on the chat message
/// (see docs/reference/foundry-vtm5e-rolls.md §"Chat message shape").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalRoll {
    pub source: SourceKind,
    pub source_id: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    /// ISO-8601. None if the source's frame didn't carry a timestamp.
    pub timestamp: Option<String>,
    pub splat: RollSplat,
    pub flavor: String,
    pub formula: String,
    pub basic_results: Vec<u8>,
    pub advanced_results: Vec<u8>,
    pub total: u32,
    pub difficulty: Option<u32>,
    pub criticals: u32,
    pub messy: bool,
    pub bestial: bool,
    pub brutal: bool,
    pub raw: serde_json::Value,
}
```

- [ ] **Step 2:** At the bottom of `src-tauri/src/bridge/types.rs`, add a `#[cfg(test)] mod tests` block (or extend an existing one). Add a serde round-trip test for `CanonicalRoll`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_roll_serde_roundtrip() {
        let roll = CanonicalRoll {
            source: SourceKind::Foundry,
            source_id: "msg_abc123".into(),
            actor_id: Some("actor_xyz".into()),
            actor_name: Some("Doe, John".into()),
            timestamp: Some("2026-05-10T12:00:00Z".into()),
            splat: RollSplat::Vampire,
            flavor: "Strength + Brawl".into(),
            formula: "5dv cs>5 + 2dg cs>5".into(),
            basic_results: vec![3, 7, 9, 10, 6],
            advanced_results: vec![2, 8],
            total: 4,
            difficulty: Some(4),
            criticals: 1,
            messy: false,
            bestial: false,
            brutal: false,
            raw: json!({ "anything": "goes" }),
        };
        let s = serde_json::to_string(&roll).unwrap();
        let back: CanonicalRoll = serde_json::from_str(&s).unwrap();
        assert_eq!(back.source_id, roll.source_id);
        assert_eq!(back.splat, RollSplat::Vampire);
        assert_eq!(back.basic_results, vec![3, 7, 9, 10, 6]);
        assert_eq!(back.criticals, 1);
        assert_eq!(back.raw, json!({ "anything": "goes" }));
    }

    #[test]
    fn roll_splat_serde_snake_case() {
        let s = serde_json::to_string(&RollSplat::Vampire).unwrap();
        assert_eq!(s, "\"vampire\"");
        let back: RollSplat = serde_json::from_str("\"werewolf\"").unwrap();
        assert_eq!(back, RollSplat::Werewolf);
    }
}
```

If a `#[cfg(test)] mod tests` already exists in this file, add the new tests below the existing ones rather than creating a second module.

- [ ] **Step 3:** Open `src-tauri/src/bridge/source.rs`. Replace the entire file contents with:

```rust
use crate::bridge::types::{CanonicalCharacter, CanonicalRoll};
use async_trait::async_trait;
use serde_json::Value;

/// One event emitted from a single inbound frame. A frame may yield zero,
/// one, or many events — e.g. `actors` snapshot is one CharactersUpdated;
/// a `hello` frame yields nothing.
#[derive(Debug, Clone)]
pub enum InboundEvent {
    /// Source pushed an updated set of characters. Frontend re-renders
    /// from the merged cache; an empty Vec is a legal "no characters
    /// attached to this message" — not a clear signal.
    CharactersUpdated(Vec<CanonicalCharacter>),
    /// Source pushed a roll result.
    RollReceived(CanonicalRoll),
}

/// Per-source protocol adapter. Sources are stateless transformers;
/// shared state (cache, outbound channel, connected flag, roll history)
/// lives in `BridgeState`.
#[async_trait]
pub trait BridgeSource: Send + Sync {
    /// Parse one inbound JSON frame into zero or more events.
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String>;

    fn build_set_attribute(
        &self,
        source_id: &str,
        name: &str,
        value: &str,
    ) -> Result<Value, String>;

    fn build_refresh(&self) -> Value;
}
```

- [ ] **Step 4:** Open `src-tauri/src/bridge/roll20/mod.rs`. Find `Roll20Source::handle_inbound` (the `#[async_trait]` impl method). Its current return shape is `Result<Vec<CanonicalCharacter>, String>`. Update both the signature and the return-site wrapping:

```rust
async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String> {
    // existing parse logic unchanged — produces a Vec<CanonicalCharacter>
    // (possibly empty) named e.g. `chars`. After it computes `chars`:
    Ok(vec![InboundEvent::CharactersUpdated(chars)])
}
```

You'll need `use crate::bridge::source::InboundEvent;` at the top of the file. If multiple early-return sites exist (e.g. error mappings on parse failure), each `Ok(...)` wraps the same way.

- [ ] **Step 5:** Open `src-tauri/src/bridge/foundry/mod.rs`. Apply the same migration as Roll20: signature flip on `handle_inbound`, wrap `Ok(chars)` → `Ok(vec![InboundEvent::CharactersUpdated(chars)])`. Existing `Hello` arms that produced an empty Vec become `Ok(vec![])`. Add `use crate::bridge::source::InboundEvent;` import. **Do NOT add a RollResult arm yet** — that's Task 2.

- [ ] **Step 6:** Open `src-tauri/src/bridge/mod.rs`. Find `accept_loop` (the function that owns the per-connection select-loop on inbound messages from the source's WS handle). Find where it currently calls `source.handle_inbound(msg).await?` and uses the returned `Vec<CanonicalCharacter>` to merge + emit `bridge://characters-updated`. Restructure to dispatch by event variant.

  The exact current shape varies — read the file before patching. Conceptually, replace:

```rust
// BEFORE
let chars = source.handle_inbound(msg).await?;
state.merge_characters(source_kind, chars);
app_handle.emit("bridge://characters-updated", &state.all_characters()).ok();
```

  with:

```rust
// AFTER
let events = source.handle_inbound(msg).await?;
for event in events {
    match event {
        InboundEvent::CharactersUpdated(chars) => {
            state.merge_characters(source_kind, chars);
            app_handle.emit("bridge://characters-updated", &state.all_characters()).ok();
        }
        InboundEvent::RollReceived(_roll) => {
            // Plan B wires the ring + bridge://roll-received emit. For now,
            // log and drop so Plan A.2's hook can be smoke-tested without
            // Plan B existing.
            log::debug!("RollReceived event arrived; ring + emit pending Plan B");
        }
    }
}
```

  Add `use crate::bridge::source::InboundEvent;` if not present. Use whatever logging facade the bridge module currently uses (`log::debug` is the convention from existing code; verify and match).

- [ ] **Step 7:** Open `src/types.ts`. Find the existing `BridgeCharacter` / `CanonicalCharacter` mirror. After it, add:

```ts
export type RollSplat = 'mortal' | 'vampire' | 'werewolf' | 'hunter' | 'unknown';

/**
 * Source-agnostic roll result. Mirrors src-tauri/src/bridge/types.rs::CanonicalRoll.
 * Drift between Rust and TS is not tolerated — change both in the same commit.
 */
export interface CanonicalRoll {
  source: SourceKind;
  source_id: string;
  actor_id: string | null;
  actor_name: string | null;
  timestamp: string | null;
  splat: RollSplat;
  flavor: string;
  formula: string;
  basic_results: number[];
  advanced_results: number[];
  total: number;
  difficulty: number | null;
  criticals: number;
  messy: boolean;
  bestial: boolean;
  brutal: boolean;
  raw: unknown;
}
```

- [ ] **Step 8:** Run `cargo test --manifest-path src-tauri/Cargo.toml -- types::tests`. Expected: `canonical_roll_serde_roundtrip` and `roll_splat_serde_snake_case` pass.

- [ ] **Step 9:** Run `./scripts/verify.sh`. Expected: green. (`npm run check` validates the TS mirror; `cargo test` runs the new round-trip; `cargo check` validates the trait signature changes.)

- [ ] **Step 10 (manual smoke):** `npm run tauri dev`. Connect Foundry browser; confirm actors load into Campaign view (CharactersUpdated path through the new event-dispatch loop is healthy). Click refresh in the toolbar; confirm re-fetch works. Confirm Roll20 connection (if available) still ingests characters. **No roll mirroring yet — Task 2 wires it.**

- [ ] **Step 11:** Commit.

```bash
git add src-tauri/src/bridge/source.rs src-tauri/src/bridge/types.rs src-tauri/src/bridge/mod.rs src-tauri/src/bridge/roll20/mod.rs src-tauri/src/bridge/foundry/mod.rs src/types.ts
git commit -m "$(cat <<'EOF'
refactor(bridge): handle_inbound returns Vec<InboundEvent>

BridgeSource::handle_inbound's return type flips from Vec<CanonicalCharacter>
to Vec<InboundEvent>, where InboundEvent is { CharactersUpdated(Vec<...>),
RollReceived(CanonicalRoll) }. Roll20 + Foundry sources wrap their existing
character returns; the accept_loop dispatches per-variant. RollReceived
arms log-and-drop until Plan B wires the ring buffer + emit.

CanonicalRoll struct + RollSplat enum land in shared types with serde
round-trip tests. TS mirror added in src/types.ts.

Atomic protocol shape change per feedback_atomic_cluster_commits —
intermediate trait return-type states break compile.

Per docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §3, §4.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2 (commit A.2) — Foundry roll decode (additive)

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs` (add RollResult variant + FoundryRollMessage)
- Modify: `src-tauri/src/bridge/foundry/mod.rs` (handle RollResult arm)
- Create: `src-tauri/src/bridge/foundry/translate_roll.rs` (translator + tests)
- Create: `vtmtools-bridge/scripts/foundry-hooks/rolls.js` (JS hook)
- Modify: `vtmtools-bridge/scripts/bridge.js` (wire the hook init)

- [ ] **Step 0 (verification gate):** Open `docs/reference/foundry-vtm5e-roll-sample.json` and inspect:
  - Is `message.rolls[0].terms[]` a flat array of `Die` instances, or are there wrapping `PoolTerm` / parenthetical-grouping terms?
  - Where does `difficulty` live in the captured sample? (Likely `roll.options.difficulty`, but possibly absent — the sample doc notes "no rollMode", suggesting some fields may be missing.)
  - Does the captured sample have a `timestamp` field on the message? Type: number (ms epoch) or already-formatted string?
  - What's the `_id` field name? (Foundry typically `_id`; some clients expose `.id`.)

  Document findings as a one-line comment at the top of `translate_roll.rs`'s production code (Step 5):

```rust
// Verified 2026-05-10 against foundry-vtm5e-roll-sample.json: terms[] is flat,
// difficulty at roll.options.difficulty (absent → None), timestamp is ms-epoch number,
// message ID at _id field.
```

  If any finding contradicts the spec assumptions (e.g. terms are nested in PoolTerms), pause and report — the JS hook in Step 8 needs to know.

- [ ] **Step 1:** Open `src-tauri/src/bridge/foundry/types.rs`. Add the `FoundryRollMessage` struct and extend `FoundryInbound`. The exact additions:

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FoundryRollMessage {
    pub message_id: String,
    pub actor_id: Option<String>,
    pub actor_name: Option<String>,
    /// ISO-8601 string — JS-side converts ms-epoch to ISO before sending.
    pub timestamp: Option<String>,
    pub flavor: String,
    pub formula: String,
    pub splat: String,
    pub basic_results: Vec<u8>,
    pub advanced_results: Vec<u8>,
    pub total: u32,
    pub difficulty: Option<u32>,
    /// Full Foundry ChatMessage blob — opaque to Rust, forwarded to UI as `raw`.
    pub raw: serde_json::Value,
}

// Extend the FoundryInbound enum — add a new arm.
// (locate the existing #[derive(Deserialize)] enum FoundryInbound and add:)
//   RollResult { message: FoundryRollMessage },
```

  Find the existing `FoundryInbound` enum definition (it has `Actors`, `ActorUpdate`, `Hello` variants per the spec). Add the new arm:

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    Actors { actors: Vec<FoundryActor> },
    ActorUpdate { actor: FoundryActor },
    Hello,
    RollResult { message: FoundryRollMessage },  // NEW
}
```

- [ ] **Step 2:** Create `src-tauri/src/bridge/foundry/translate_roll.rs`. Full file content:

```rust
// Foundry WoD5e roll → CanonicalRoll.
//
// Foundry does NOT persist rollMessageData (totals, criticals, messy, bestial)
// on the ChatMessage — see docs/reference/foundry-vtm5e-rolls.md §"Chat message shape".
// The translator recomputes those classifications from basic_results + advanced_results.

use crate::bridge::foundry::types::FoundryRollMessage;
use crate::bridge::types::{CanonicalRoll, RollSplat, SourceKind};

pub fn to_canonical_roll(raw: &FoundryRollMessage) -> CanonicalRoll {
    let splat = parse_splat(&raw.splat, &raw.formula);
    let criticals = count_criticals(&raw.basic_results, &raw.advanced_results, splat);
    let messy = matches!(splat, RollSplat::Vampire) && raw.advanced_results.iter().any(|&d| d == 10);
    let bestial = matches!(splat, RollSplat::Vampire) && raw.advanced_results.iter().any(|&d| d == 1);
    let brutal = matches!(splat, RollSplat::Werewolf) && raw.advanced_results.iter().any(|&d| d == 1);

    CanonicalRoll {
        source: SourceKind::Foundry,
        source_id: raw.message_id.clone(),
        actor_id: raw.actor_id.clone(),
        actor_name: raw.actor_name.clone(),
        timestamp: raw.timestamp.clone(),
        splat,
        flavor: raw.flavor.clone(),
        formula: raw.formula.clone(),
        basic_results: raw.basic_results.clone(),
        advanced_results: raw.advanced_results.clone(),
        total: raw.total,
        difficulty: raw.difficulty,
        criticals,
        messy,
        bestial,
        brutal,
        raw: raw.raw.clone(),
    }
}

/// Defensive: trust the JS-side splat string when it's a known value, else
/// re-detect from formula via regex per docs/reference/foundry-vtm5e-rolls.md
/// §"Splat detection". `roll.system` is unreliable post-rehydration so the
/// formula is the robust signal.
fn parse_splat(js_splat: &str, formula: &str) -> RollSplat {
    match js_splat {
        "mortal" => RollSplat::Mortal,
        "vampire" => RollSplat::Vampire,
        "werewolf" => RollSplat::Werewolf,
        "hunter" => RollSplat::Hunter,
        _ => detect_splat_from_formula(formula),
    }
}

fn detect_splat_from_formula(formula: &str) -> RollSplat {
    // Order matters: vampire and werewolf both have advanced dice (g/r),
    // so check the more specific basic+advanced pairings first.
    if has_die(formula, 'v') || has_die(formula, 'g') {
        RollSplat::Vampire
    } else if has_die(formula, 'w') || has_die(formula, 'r') {
        RollSplat::Werewolf
    } else if has_die(formula, 'h') || has_die(formula, 's') {
        RollSplat::Hunter
    } else if has_die(formula, 'm') {
        RollSplat::Mortal
    } else {
        RollSplat::Unknown
    }
}

/// Detects `Ndx` patterns where x is the die-class letter. Lightweight
/// substring match — formulas are simple, no regex crate needed.
fn has_die(formula: &str, denom: char) -> bool {
    // Walk the formula looking for "d<denom>" not preceded/followed by another letter
    // (keeps "dh" from matching inside "Dhampir" or similar — paranoid since formulas
    // are simple strings, but free defensiveness).
    let needle = format!("d{denom}");
    let bytes = formula.as_bytes();
    let nb = needle.as_bytes();
    if bytes.len() < nb.len() { return false; }
    for i in 0..=bytes.len() - nb.len() {
        if &bytes[i..i + nb.len()] == nb {
            // require boundary after the denomination
            let after = bytes.get(i + nb.len());
            if after.map_or(true, |b| !b.is_ascii_alphabetic()) {
                return true;
            }
        }
    }
    false
}

/// Count critical successes — natural 10s on basic dice plus natural 10s on
/// hunger dice (latter are messy crits, counted in both `criticals` and `messy`).
/// Werewolf rage 10s and hunter desperation 10s are counted as criticals too.
fn count_criticals(basic: &[u8], advanced: &[u8], splat: RollSplat) -> u32 {
    let basic_tens = basic.iter().filter(|&&d| d == 10).count() as u32;
    let advanced_tens = match splat {
        RollSplat::Vampire | RollSplat::Werewolf | RollSplat::Hunter => {
            advanced.iter().filter(|&&d| d == 10).count() as u32
        }
        _ => 0,
    };
    basic_tens + advanced_tens
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_msg(splat: &str, formula: &str, basic: Vec<u8>, advanced: Vec<u8>) -> FoundryRollMessage {
        FoundryRollMessage {
            message_id: "msg_test".into(),
            actor_id: Some("actor_test".into()),
            actor_name: Some("Tester".into()),
            timestamp: Some("2026-05-10T00:00:00Z".into()),
            flavor: "Test".into(),
            formula: formula.into(),
            splat: splat.into(),
            basic_results: basic,
            advanced_results: advanced,
            total: 0,
            difficulty: None,
            raw: json!({}),
        }
    }

    #[test]
    fn vampire_clean_no_messy_no_bestial() {
        let m = make_msg("vampire", "5dv cs>5 + 0dg cs>5", vec![3, 7, 9, 10, 6], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Vampire);
        assert_eq!(c.criticals, 1);
        assert!(!c.messy);
        assert!(!c.bestial);
        assert!(!c.brutal);
    }

    #[test]
    fn vampire_messy_when_hunger_ten() {
        let m = make_msg("vampire", "5dv cs>5 + 3dg cs>5", vec![3, 7, 9, 6, 6], vec![2, 8, 10]);
        let c = to_canonical_roll(&m);
        assert!(c.messy);
        assert_eq!(c.criticals, 1, "the 10 on hunger counts as a crit");
    }

    #[test]
    fn vampire_bestial_when_hunger_one() {
        let m = make_msg("vampire", "5dv cs>5 + 3dg cs>5", vec![3, 7, 9, 6, 6], vec![1, 8, 4]);
        let c = to_canonical_roll(&m);
        assert!(c.bestial);
        assert!(!c.messy);
    }

    #[test]
    fn werewolf_brutal_when_rage_one() {
        let m = make_msg("werewolf", "5dw cs>5 + 2dr cs>5", vec![3, 7, 9], vec![1, 8]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Werewolf);
        assert!(c.brutal);
    }

    #[test]
    fn double_basic_tens_count_two_criticals() {
        let m = make_msg("vampire", "4dv cs>5", vec![10, 10, 5, 7], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.criticals, 2);
    }

    #[test]
    fn splat_detection_from_formula_when_js_splat_unknown() {
        let m = make_msg("unknown", "5dv cs>5 + 2dg cs>5", vec![6, 7], vec![3]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Vampire);
    }

    #[test]
    fn empty_advanced_no_panic() {
        let m = make_msg("mortal", "3dm cs>5", vec![6, 7, 8], vec![]);
        let c = to_canonical_roll(&m);
        assert_eq!(c.splat, RollSplat::Mortal);
        assert!(!c.messy);
        assert!(!c.bestial);
        assert!(!c.brutal);
    }

    #[test]
    fn has_die_word_boundary() {
        // 'dh' inside hypothetical word should not falsely detect hunter.
        // Real formulas don't have alphabetic chars after the die denom, but
        // the test pins the boundary check.
        assert!(has_die("3dh cs>5", 'h'));
        assert!(!has_die("3dho cs>5", 'h'), "'dh' followed by letter should not match");
    }
}
```

- [ ] **Step 3:** Open `src-tauri/src/bridge/foundry/mod.rs`. Find the existing `mod` declarations (probably `mod actions;`, `mod translate;`, `mod types;`). Add:

```rust
mod translate_roll;
```

- [ ] **Step 4:** In the same file, find `FoundrySource::handle_inbound`. It currently has `match`-arms for `Actors`, `ActorUpdate`, `Hello`. Add the `RollResult` arm:

```rust
FoundryInbound::RollResult { message } => {
    let canonical = translate_roll::to_canonical_roll(&message);
    Ok(vec![InboundEvent::RollReceived(canonical)])
}
```

  Place this arm alongside the existing arms in the match expression. The arm returns the new event variant directly (no character data, no merge).

- [ ] **Step 5:** Run `cargo test --manifest-path src-tauri/Cargo.toml -- bridge::foundry::translate_roll::tests`. Expected: all 8 tests pass.

- [ ] **Step 6:** Open `src-tauri/src/bridge/mod.rs`. Update the `RollReceived` placeholder arm to actually emit (Plan B will replace this with ring-pushing too, but we want the bridge event for smoke-testing now):

```rust
InboundEvent::RollReceived(roll) => {
    // Plan B will additionally push to BridgeState.roll_history.
    app_handle.emit("bridge://roll-received", &roll).ok();
}
```

  This unblocks Plan A.2's manual verification (Step 11) without depending on Plan B — the JS hook can be tested end-to-end by listening to the event in DevTools.

- [ ] **Step 7:** Create `vtmtools-bridge/scripts/foundry-hooks/rolls.js`. Full file content:

```js
// vtmtools — Foundry roll-result hook.
// Subscribes to createChatMessage; serializes the roll into the wire
// shape vtmtools' Rust translator expects, and ships it on the open socket.
//
// Splat detection: regex on roll.formula per docs/reference/foundry-vtm5e-rolls.md
// §"Splat detection" — roll.system is unreliable post-rehydration.
//
// Per-die-class extraction: walk roll.terms[] for Die instances, partition by
// denomination (basic: m/v/w/h, advanced: g/r/s).

const SPLAT_PATTERNS = {
  vampire: /\bd[vg]\b/,
  werewolf: /\bd[wr]\b/,
  hunter: /\bd[hs]\b/,
  mortal: /\bdm\b/,
};

function detectSplat(formula) {
  if (!formula) return 'unknown';
  if (SPLAT_PATTERNS.vampire.test(formula)) return 'vampire';
  if (SPLAT_PATTERNS.werewolf.test(formula)) return 'werewolf';
  if (SPLAT_PATTERNS.hunter.test(formula)) return 'hunter';
  if (SPLAT_PATTERNS.mortal.test(formula)) return 'mortal';
  return 'unknown';
}

const BASIC_DENOMS = new Set(['m', 'v', 'w', 'h']);
const ADV_DENOMS = new Set(['g', 'r', 's']);

function extractDiceResults(roll, kind) {
  const out = [];
  const want = kind === 'basic' ? BASIC_DENOMS : ADV_DENOMS;
  const terms = Array.isArray(roll?.terms) ? roll.terms : [];
  for (const term of terms) {
    // Term may be a Die instance, a NumericTerm, or an OperatorTerm. Filter to dice.
    const isDie = term?.constructor?.name === 'Die' || term?.class === 'Die';
    if (!isDie) continue;
    const denom = String(term.denomination ?? '').toLowerCase();
    if (!want.has(denom)) continue;
    for (const r of term.results ?? []) {
      if (typeof r?.result === 'number') out.push(r.result);
    }
  }
  return out;
}

function messageToWire(message) {
  const roll = message.rolls?.[0];
  if (!roll) return null;

  const formula = roll.formula ?? '';
  const splat = detectSplat(formula);

  const ts = typeof message.timestamp === 'number'
    ? new Date(message.timestamp).toISOString()
    : (typeof message.timestamp === 'string' ? message.timestamp : null);

  let raw;
  try { raw = message.toJSON ? message.toJSON() : message; }
  catch { raw = {}; }

  return {
    message_id: message._id ?? message.id ?? '',
    actor_id: message.speaker?.actor ?? null,
    actor_name: message.speaker?.alias ?? null,
    timestamp: ts,
    flavor: typeof message.flavor === 'string' ? message.flavor : '',
    formula,
    splat,
    basic_results: extractDiceResults(roll, 'basic'),
    advanced_results: extractDiceResults(roll, 'advanced'),
    total: typeof roll.total === 'number' ? roll.total : 0,
    difficulty: typeof roll.options?.difficulty === 'number' ? roll.options.difficulty : null,
    raw,
  };
}

export function init(getSocket) {
  Hooks.on('createChatMessage', (message, _options, _userId) => {
    if (!message.rolls?.length) return;
    const socket = getSocket();
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    const wire = messageToWire(message);
    if (!wire) return;
    try {
      socket.send(JSON.stringify({ type: 'roll_result', message: wire }));
    } catch (err) {
      console.error('[vtmtools-bridge] roll-result send failed:', err);
    }
  });
}
```

- [ ] **Step 8:** Open `vtmtools-bridge/scripts/bridge.js`. Add the import alongside the existing `bridgeUmbrella` import (top of file):

```js
import { init as initRollsHook } from "./foundry-hooks/rolls.js";
```

  Find the `Hooks.once("ready", ...)` block. After the existing actor-subscribe try/catch (around line 50 in the file), add the rolls-hook init:

```js
// vtmtools roll mirror: subscribe to createChatMessage. The hook stays
// registered for the world-session lifetime; teardown happens on world reload.
initRollsHook(() => socket);
```

  The closure passes the live socket reference each call, since `socket` is the module-scope `let` that gets reassigned on reconnect.

- [ ] **Step 9:** Run `./scripts/verify.sh`. Expected: green. (`cargo test` runs the new translator tests; `npm run build` validates everything else.)

- [ ] **Step 10 (manual smoke — Foundry-side):**
  1. `npm run tauri dev`. Connect Foundry browser to `wss://localhost:7424`.
  2. In the Tauri DevTools console, execute:
     ```js
     window.__TAURI__?.event.listen('bridge://roll-received', e => console.log('ROLL:', e.payload));
     ```
     If `__TAURI__.event` isn't directly accessible, use `import('@tauri-apps/api/event').then(({listen}) => listen('bridge://roll-received', e => console.log('ROLL:', e.payload)))`.
  3. In Foundry, open a vampire actor sheet. Click any skill+attribute dice button → roll dialog → Roll. Confirm DevTools logs the event with:
     - Correct `splat: 'vampire'`
     - `basic_results` matching the dice shown in chat
     - `advanced_results` matching hunger dice
     - `criticals`, `messy`, `bestial` correct (verify against the dice you saw)
     - Non-empty `formula`, non-empty `flavor`
  4. Repeat for a mortal actor. Confirm `splat: 'mortal'`, `advanced_results: []`, `messy: false`, `bestial: false`.
  5. If a werewolf actor is available, repeat. Confirm `splat: 'werewolf'`, `brutal` flag toggles correctly on a rage 1.
  6. Confirm the existing actor-load path is unaffected — characters still appear in Campaign view.

- [ ] **Step 11 (manual smoke — Roll20):** With Roll20 extension connected, edit an attribute (e.g., bump WP). Confirm `bridge://characters-updated` still fires (the InboundEvent dispatch loop is healthy on Roll20 too). No roll events expected from Roll20.

- [ ] **Step 12:** Commit.

```bash
git add src-tauri/src/bridge/foundry/types.rs src-tauri/src/bridge/foundry/mod.rs src-tauri/src/bridge/foundry/translate_roll.rs src-tauri/src/bridge/mod.rs vtmtools-bridge/scripts/foundry-hooks/rolls.js vtmtools-bridge/scripts/bridge.js
git commit -m "$(cat <<'EOF'
feat(bridge): inbound Foundry roll mirror via createChatMessage hook

Foundry-side hook subscribes to createChatMessage, serializes the roll
into a wire shape with formula-derived splat, basic/advanced dice partitioned
by denomination, and ships on the existing wss bridge. Rust translator
recomputes criticals/messy/bestial/brutal from the dice arrays
(rollMessageData isn't persisted by Foundry — see foundry-vtm5e-rolls.md).

bridge://roll-received emits per inbound roll. Bounded ring + IPC + UI
follow in Plans B and C.

Per docs/superpowers/specs/2026-05-10-foundry-roll-mirroring-design.md §3, §4, §5.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Checklist (run after Task 2 commit)

- [ ] **Spec coverage:** §3 trait migration done (commit A.1). §4 CanonicalRoll struct done (A.1). §5 Foundry hook + Rust translator + splat detection done (A.2). §10 `bridge://roll-received` event emitted (A.2 via the temporary emit-from-mod.rs; Plan B will move it next to the ring push). §15 open question 1 (terms shape) verified in Step 0.
- [ ] **Anti-scope respected:** No `BridgeState.roll_history` field (Plan B). No Tauri commands (Plan B). No `src/store/rolls.svelte.ts` (Plan B). No frontend tools (Plan C).
- [ ] **No placeholders / dead branches:** RollReceived arm in `bridge/mod.rs` emits the event. JS hook pulls full ChatMessage `raw`. Rust tests cover the eight enumerated cases.
- [ ] **Type consistency:** Rust `CanonicalRoll` field names match TS mirror exactly (`source_id`, `actor_id`, `basic_results`, `advanced_results`, etc.). Rust `RollSplat` snake-case serialization matches TS string literals.
- [ ] **Verify gate:** `./scripts/verify.sh` green for both A.1 and A.2.
- [ ] **Defensive splat detection:** JS-side ships a splat string; Rust verifies via formula regex. Trust hierarchy: known JS splat string → use; else regex.

## Open questions (deferred from spec §15)

- **Difficulty source location** — Step 0 verified against the live sample. If `roll.options.difficulty` is absent, JS sends `null` and Rust deserializes to `None`. Plan C UI handles `None`.
- **Speaker actor missing** — JS-side already handles `??` fallback to `null`. Rust deserializes to `None`. UI fallback ("GM" / "Anonymous") is Plan C's call.
- **Capacity tuning (200)** — Plan B's concern; this plan doesn't bound anything (events emit without buffering until B).
