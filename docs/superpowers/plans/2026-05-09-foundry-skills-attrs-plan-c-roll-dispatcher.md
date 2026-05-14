# Foundry skills/attrs — Plan C: GM Screen roll dispatcher popover

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plan C tasks are committed, run a SINGLE `code-review:code-review` against the full Plan C branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** GM clicks a 🎲 button on a Foundry character row in the GM Screen → a popover opens with attribute + skill dropdowns, a difficulty input, optional custom flavor text, and a roll-mode selector. The popover surfaces the live sum of all active non-hidden modifier cards' pool and difficulty deltas. Submit dispatches the roll into Foundry chat via `triggerFoundryRoll`. Bypasses Foundry's selectors-based situational-bonus auto-apply by adding a `pool_modifier` field to the wire payload — JS executor switches to direct `WOD5E.api.Roll` when `pool_modifier !== 0`.

**Architecture:** Wire-format extension to `RollV5PoolInput` (adds `roll_mode` + `pool_modifier`, both additive). New TS modules: `path-providers.ts` (pluggable registry — V1 ships `ATTRIBUTE_PROVIDER` + `SKILL_PROVIDER`) and `roll.ts` (`summarizeModifiers` + `dispatchRoll`). New Svelte component `RollDispatcherPopover.svelte` mounted from `CharacterRow.svelte` via a Foundry-only-rendered 🎲 trigger. JS executor in `vtmtools-bridge/scripts/foundry-actions/game.js` branches on `paths.length === 0 || pool_modifier !== 0` to use direct `WOD5E.api.Roll` (bypasses selectors-based double-counting); falls through to the existing `RollFromDataset` path otherwise.

**Tech Stack:** Rust + serde_json (existing), Tauri 2 IPC, Svelte 5 runes mode, TypeScript, vanilla JS (Foundry module).

**Spec:** `docs/superpowers/specs/2026-05-09-foundry-skills-attrs-and-roll-dispatcher-design.md` §6 (Plan C).
**Architecture reference:** `ARCHITECTURE.md` §4 (Tauri IPC + bridge protocol), §6 (CSS / token invariants), §7 (error handling), §10 (testing).

**Spec defaults adopted:**
- Stat-based base pool (attribute + skill dropdowns) — spec §2.4.
- Pluggable PathProvider registry — spec §2.5.
- Privacy controls (custom flavor + roll_mode selector) — spec §2.6.
- Pool modifier as a wire field bypassing selectors — spec §2.7, §6.4.
- Roll20 character rows: 🎲 button NOT rendered — spec §6.6.

**Depends on:** Plan A merged (uses `FOUNDRY_ATTRIBUTE_NAMES` / `FOUNDRY_SKILL_NAMES` from `canonical-names.ts`). Does NOT depend on Plan B (popover doesn't write stats).

**Implementer verification (carried over from spec §13):** `WOD5E.api.RollFromDataset({ dataset: { rollMode } })` propagation is undocumented. If `rollMode` is silently lost on the non-pool-modifier path during Task C3 manual smoke, fall back to **always using direct `WOD5E.api.Roll`** in `rollV5Pool` (simpler invariant) and remove the `RollFromDataset` branch.

---

## File structure

### Files created

| Path | Responsibility |
|---|---|
| `src/lib/gm-screen/path-providers.ts` | `PathProvider` interface + `ATTRIBUTE_PROVIDER` / `SKILL_PROVIDER` / `DEFAULT_PROVIDERS`. Pluggable foundation for future discipline / merit-bonus / renown providers. |
| `src/lib/gm-screen/roll.ts` | `summarizeModifiers(mods) -> { pool, difficulty, notes }`, `dispatchRoll(args) -> Promise<void>`. Pure helpers; the popover composes them. |
| `src/lib/components/gm-screen/RollDispatcherPopover.svelte` | The popover itself. Provider dropdowns + difficulty input + privacy controls + active-modifier sums + Roll/Cancel buttons. Anchored via `position: fixed` with viewport coords from `getBoundingClientRect()` (mirrors existing popover style — see `CharacterRow.svelte`'s editor popover). |

### Files modified

| Path | Change |
|---|---|
| `src-tauri/src/bridge/foundry/types.rs` | Add `roll_mode: Option<String>` and `pool_modifier: Option<i32>` to `RollV5PoolInput`. |
| `src-tauri/src/bridge/foundry/actions/game.rs` | Extend `build_roll_v5_pool`: validate `roll_mode` against `VALID_ROLL_MODES`; pass `roll_mode` and `pool_modifier` (with defaults) into the envelope. Add 5 new tests. |
| `vtmtools-bridge/scripts/foundry-actions/game.js` | Branch on `paths.length === 0 \|\| pool_modifier !== 0` to use direct `WOD5E.api.Roll`; add `computeBasicDice(actor, paths)` helper; pass `rollMode` through both branches. |
| `vtmtools-bridge/module.json` | Bump version 0.4.0 → 0.5.0 (semver minor: additive features). |
| `src/lib/foundry-chat/api.ts` | Add `rollMode?` and `poolModifier?` to `RollV5PoolInput` interface. |
| `src/lib/components/gm-screen/CharacterRow.svelte` | Add 🎲 trigger button (Foundry-only) + popover mount state + popover positioning logic. |

### Files NOT touched in Plan C

- `src/tools/Campaign.svelte` (Plan A territory)
- `src-tauri/src/shared/canonical_fields.rs` (Plan B territory)
- `src-tauri/src/tools/character.rs` (Plan B territory)
- `src/types.ts` (no new types — `BridgeCharacter`, `CharacterModifier`, `ModifierEffect` already cover everything Plan C needs)
- Migrations (no schema change)

### Tauri command surface

Unchanged. `trigger_foundry_roll` already exists; this plan widens its payload.

### Wire-protocol changes

Both fields on `RollV5PoolInput` / `game.roll_v5_pool` envelope are **additive within protocol_version 1**. Module version bump 0.4.0 → 0.5.0 reflects "additive new features." Backward-compat: old desktop ↔ new module = identical behavior; new desktop ↔ old module = `pool_modifier` silently ignored (graceful — `roll_mode` falls through to default `"roll"`).

---

## Task C1: Rust wire-format extension to `RollV5PoolInput` + builder validation

**Goal:** Add `roll_mode: Option<String>` and `pool_modifier: Option<i32>` to the Rust struct + extend `build_roll_v5_pool` to validate `roll_mode` and pass both fields into the envelope. Atomic commit.

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs`
- Modify: `src-tauri/src/bridge/foundry/actions/game.rs`

**Anti-scope:** Do not touch `vtmtools-bridge/scripts/foundry-actions/game.js` (Task C3). Do not touch `src/lib/foundry-chat/api.ts` (Task C2). Do not touch the existing `build_post_chat_as_actor` (its `roll_mode` validation logic is the template — reuse the same `VALID_ROLL_MODES` const if visible at file scope, otherwise duplicate; do not refactor).

**Depends on:** nothing in Plan C (foundation task).

**Invariants cited:** ARCH §7 (error prefix `foundry/game.roll_v5_pool:`). Spec §6.3 (envelope shape). Spec §6.2 (`pool_modifier` is `i32` — negative is valid).

**Tests required:** YES — TDD the builder validation. Logic is small but the additive wire field is the most likely place to have a subtle off-by-one or missing-default bug.

- [x] **Step 1: Write the failing tests**

Open `src-tauri/src/bridge/foundry/actions/game.rs`. Locate the existing `#[cfg(test)] mod tests { ... }` block. Add inside it (after the existing `roll_v5_pool_*` tests, before the `post_chat_as_actor_*` tests):

```rust
    #[test]
    fn roll_v5_pool_envelope_includes_roll_mode_and_pool_modifier() {
        let mut input = sample_roll_input();
        input.roll_mode = Some("gmroll".into());
        input.pool_modifier = Some(2);
        let v = build_roll_v5_pool(&input).expect("happy path");
        assert_eq!(v["roll_mode"], "gmroll");
        assert_eq!(v["pool_modifier"], 2);
    }

    #[test]
    fn roll_v5_pool_default_roll_mode_is_roll() {
        // sample_roll_input has roll_mode: None
        let v = build_roll_v5_pool(&sample_roll_input()).expect("ok");
        assert_eq!(v["roll_mode"], "roll");
    }

    #[test]
    fn roll_v5_pool_default_pool_modifier_is_zero() {
        let v = build_roll_v5_pool(&sample_roll_input()).expect("ok");
        assert_eq!(v["pool_modifier"], 0);
    }

    #[test]
    fn roll_v5_pool_invalid_roll_mode_errors() {
        let mut input = sample_roll_input();
        input.roll_mode = Some("shouting".into());
        let err = build_roll_v5_pool(&input).expect_err("must reject invalid roll_mode");
        assert!(err.contains("invalid roll_mode"), "got: {err}");
        assert!(err.contains("foundry/game.roll_v5_pool"), "got: {err}");
    }

    #[test]
    fn roll_v5_pool_negative_pool_modifier_passes_through() {
        let mut input = sample_roll_input();
        input.pool_modifier = Some(-2);
        let v = build_roll_v5_pool(&input).expect("negative is valid");
        assert_eq!(v["pool_modifier"], -2);
    }
```

- [x] **Step 2: Run the new tests — verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::game::tests::roll_v5_pool -- --nocapture`
Expected: FAIL — `roll_mode` field doesn't exist on `RollV5PoolInput`; `pool_modifier` field doesn't exist; `build_roll_v5_pool` doesn't emit them in the envelope.

- [x] **Step 3: Add the new fields to `RollV5PoolInput`**

Open `src-tauri/src/bridge/foundry/types.rs`. Locate the existing `RollV5PoolInput` struct. Replace it with:

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollV5PoolInput {
    pub actor_id: String,
    pub value_paths: Vec<String>,
    pub difficulty: u8,
    pub flavor: Option<String>,
    pub advanced_dice: Option<u8>,
    pub selectors: Option<Vec<String>>,
    /// One of "roll" / "gmroll" / "blindroll" / "selfroll". None = "roll".
    /// Mirrors the field on PostChatAsActorInput.
    pub roll_mode: Option<String>,
    /// Net pool modifier from the GM Screen popover (sum of active card pool
    /// deltas). i32 because penalties are negative. None = 0. When non-zero,
    /// the JS executor switches to direct WOD5E.api.Roll to bypass selectors-
    /// based situational-bonus auto-apply (avoids double-counting modifier
    /// cards that have been pushed to the sheet via GM Screen Plan C).
    pub pool_modifier: Option<i32>,
}
```

- [x] **Step 4: Extend `build_roll_v5_pool` to validate `roll_mode` and emit both fields**

In `src-tauri/src/bridge/foundry/actions/game.rs`, replace the existing `build_roll_v5_pool(...)` (around lines 10-25) with:

```rust
pub fn build_roll_v5_pool(input: &RollV5PoolInput) -> Result<Value, String> {
    if input.actor_id.is_empty() {
        return Err("foundry/game.roll_v5_pool: actor_id is required".into());
    }
    // value_paths may be empty — empty paths + advanced_dice=1 is a rouse
    // check (basic pool = 0, one hunger die). No emptiness check.
    if let Some(rm) = &input.roll_mode {
        if !VALID_ROLL_MODES.contains(&rm.as_str()) {
            return Err(format!(
                "foundry/game.roll_v5_pool: invalid roll_mode: {rm}"
            ));
        }
    }
    // pool_modifier: no range check — negative is valid (penalty); i32
    // range cannot overflow the JS executor's basic-dice computation in
    // any realistic actor stat sum.
    Ok(json!({
        "type": "game.roll_v5_pool",
        "actor_id": input.actor_id,
        "value_paths": input.value_paths,
        "difficulty": input.difficulty,
        "flavor": input.flavor,
        "advanced_dice": input.advanced_dice,
        "selectors": input.selectors.clone().unwrap_or_default(),
        "roll_mode": input.roll_mode.as_deref().unwrap_or("roll"),
        "pool_modifier": input.pool_modifier.unwrap_or(0),
    }))
}
```

(The existing `VALID_ROLL_MODES` const is already at file scope — used by `build_post_chat_as_actor`. Reuse it; no new const needed.)

- [x] **Step 5: Update the existing `sample_roll_input()` test helper**

In the same `tests` module, locate `sample_roll_input()` (currently around lines 54-63). Replace with:

```rust
    fn sample_roll_input() -> RollV5PoolInput {
        RollV5PoolInput {
            actor_id: "abc".into(),
            value_paths: vec![
                "attributes.strength.value".into(),
                "skills.brawl.value".into(),
            ],
            difficulty: 3,
            flavor: Some("Strength + Brawl".into()),
            advanced_dice: None,
            selectors: None,
            roll_mode: None,
            pool_modifier: None,
        }
    }
```

(Adds defaults for the two new fields. Existing tests against `sample_roll_input()` continue to pass because both new fields are `None` → builder emits defaults.)

- [x] **Step 6: Run the tests — verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::game::tests`
Expected: PASS — all existing `roll_v5_pool_*` tests + the 5 new ones, AND the unchanged `post_chat_as_actor_*` tests.

- [x] **Step 7: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 8: Commit**

```bash
git add src-tauri/src/bridge/foundry/types.rs src-tauri/src/bridge/foundry/actions/game.rs
git commit -m "feat(foundry/game): add roll_mode + pool_modifier to RollV5PoolInput

Wire-format extension for the GM Screen roll dispatcher (Plan C). Both
fields are additive within protocol_version 1.

  roll_mode: Option<String>  -- 'roll' (default) | 'gmroll' | 'blindroll'
                                | 'selfroll'. Validated against the same
                                VALID_ROLL_MODES const that build_post_chat_
                                as_actor uses.
  pool_modifier: Option<i32> -- net pool delta from active modifier cards.
                                None defaults to 0. JS executor (Task C3)
                                will branch on this to switch from
                                RollFromDataset to direct WOD5E.api.Roll
                                when non-zero, bypassing selectors-based
                                double-counting.

Backward-compat: old desktop never sends these fields; new desktop sends
them; old module ignores them. Module version bump lands with the JS
executor change (Task C3)."
```

---

## Task C2: TS wire-format extension to `RollV5PoolInput`

**Goal:** Mirror the new Rust fields on the TS side. After this task, `triggerFoundryRoll({ ..., rollMode, poolModifier })` type-checks at every call site.

**Files:**
- Modify: `src/lib/foundry-chat/api.ts`

**Anti-scope:** Do not modify the function body of `triggerFoundryRoll` — invoke just forwards the input record. Do not touch `PostChatAsActorInput`.

**Depends on:** Task C1 (the Rust struct change is the contract this mirrors).

**Invariants cited:** spec §6.2 (TS interface mirror).

**Tests required:** NO. Wiring; verification is `npm run check`.

- [x] **Step 1: Add the two fields**

Open `src/lib/foundry-chat/api.ts`. Locate the existing `RollV5PoolInput` interface (around lines 3-10). Replace with:

```ts
export interface RollV5PoolInput {
  actorId: string;
  valuePaths: string[];
  difficulty: number;
  flavor?: string | null;
  advancedDice?: number | null;
  selectors?: string[] | null;
  /** One of 'roll' / 'gmroll' / 'blindroll' / 'selfroll'. Default 'roll'. */
  rollMode?: 'roll' | 'gmroll' | 'blindroll' | 'selfroll' | null;
  /** Net pool modifier from active GM Screen modifier cards. Negative = penalty. Default 0. */
  poolModifier?: number | null;
}
```

(Note: the existing `PostChatAsActorInput` already has `rollMode` with the identical four-option literal — same shape, kept consistent.)

- [x] **Step 2: Run `npm run check`**

Run: `npm run check`
Expected: PASS — no existing `triggerFoundryRoll` call sites use the new fields yet, so nothing else needs to change.

- [x] **Step 3: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/lib/foundry-chat/api.ts
git commit -m "feat(foundry-chat): add rollMode + poolModifier to RollV5PoolInput TS

Mirrors the Rust-side field additions from Task C1. New optional fields:
  rollMode?:    'roll' | 'gmroll' | 'blindroll' | 'selfroll' | null
  poolModifier?: number | null

triggerFoundryRoll() body unchanged — invoke forwards the input record
verbatim; the Rust deserializer (camelCase rename) maps to roll_mode /
pool_modifier on the wire."
```

---

## Task C3: JS executor branching + `computeBasicDice` + module version bump

**Goal:** Extend `vtmtools-bridge/scripts/foundry-actions/game.js`'s `rollV5Pool` to branch on `paths.length === 0 || pool_modifier !== 0` for direct `WOD5E.api.Roll`; add `computeBasicDice(actor, paths)` helper; pass `rollMode` through both branches. Bump module.json from 0.4.0 to 0.5.0.

**Files:**
- Modify: `vtmtools-bridge/scripts/foundry-actions/game.js`
- Modify: `vtmtools-bridge/module.json`

**Anti-scope:** Do not touch `bridge.js`. Do not touch `actor.js` (Plan B's territory in spirit, though actually unchanged in this whole umbrella).

**Depends on:** Task C1 (the wire envelope must carry the new fields before the JS executor reads them).

**Invariants cited:** spec §6.4 (JS executor branching contract — call out the double-counting rationale in code comments).

**Tests required:** NO. The project has no JS test infrastructure for `vtmtools-bridge/`; validation is the manual smoke at the end of the plan. The branching logic is short and readable enough that a reviewer can verify it by inspection.

- [x] **Step 1: Replace `rollV5Pool` with the branched version**

Open `vtmtools-bridge/scripts/foundry-actions/game.js`. Locate the existing `async function rollV5Pool(msg) { ... }`. Replace its entire body with:

```js
async function rollV5Pool(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[${MODULE_ID}] game.roll_v5_pool: actor not found: ${msg.actor_id}`);
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  const paths = msg.value_paths ?? [];
  const advancedDice = msg.advanced_dice
    ?? WOD5E.api.getAdvancedDice({ actor });
  const label = msg.flavor ?? deriveFlavorFromPaths(paths);
  const rollMode = msg.roll_mode ?? "roll";
  const poolModifier = msg.pool_modifier ?? 0;

  // Direct-Roll-API path: empty paths (rouse-style) OR caller specified a
  // pool_modifier (popover semantics). Bypassing RollFromDataset here is
  // deliberate — it avoids double-counting any modifier card that has been
  // pushed to the sheet via GM Screen Plan C (those bonuses would also be
  // auto-applied via Foundry's selectors-based situational-bonus pipeline).
  // The popover's poolModifier already encodes the GM's intent.
  if (paths.length === 0 || poolModifier !== 0) {
    const basicDice = computeBasicDice(actor, paths) + poolModifier;
    await WOD5E.api.Roll({
      basicDice,
      advancedDice,
      actor,
      difficulty: msg.difficulty,
      flavor: label,
      quickRoll: true,
      rollMode,
    });
    return;
  }

  // RollFromDataset path: auto-applies sheet bonuses via the WoD5e selectors
  // pipeline. Used for non-popover callers (e.g., a future stat-button click
  // that wants the full sheet-bonus expansion). Selectors stay caller-supplied.
  await WOD5E.api.RollFromDataset({
    dataset: {
      valuePaths: paths.join(" "),
      label,
      difficulty: msg.difficulty,
      selectDialog: false,            // never pop the GM dialog from outside Foundry
      advancedDice,
      selectors: msg.selectors ?? [],
      rollMode,
    },
    actor,
  });
}
```

- [x] **Step 2: Add `computeBasicDice` helper**

In the same file, immediately after `rollV5Pool` (and before `async function postChatAsActor`), add:

```js
/**
 * Walks each path against actor.system and sums the numeric leaf values.
 * Returns 0 for paths that don't resolve to numbers (defensive — actor data
 * shape may have nulls or missing keys). Intentionally does NOT cap at any
 * ceiling; respects whatever value Foundry stores.
 */
function computeBasicDice(actor, paths) {
  let sum = 0;
  for (const path of paths) {
    // path like "attributes.strength.value"; walk against actor.system.
    const v = path.split(".").reduce((obj, key) => obj?.[key], actor.system);
    if (typeof v === "number") sum += v;
  }
  return sum;
}
```

- [x] **Step 3: Bump `module.json` version**

Open `vtmtools-bridge/module.json`. Find:

```json
  "version": "0.4.0",
```

Replace with:

```json
  "version": "0.5.0",
```

- [x] **Step 4: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS — Rust + frontend gates unchanged. (Note: the JS module isn't part of the gate; the Foundry side runs in the user's browser.)

- [x] **Step 5: Commit**

```bash
git add vtmtools-bridge/scripts/foundry-actions/game.js vtmtools-bridge/module.json
git commit -m "feat(bridge-module): rollV5Pool branches on pool_modifier; bump 0.5.0

When the inbound game.roll_v5_pool envelope carries non-zero pool_modifier
OR empty value_paths (rouse-style), rollV5Pool now uses direct WOD5E.api.
Roll instead of RollFromDataset. This bypasses the selectors-based
situational-bonus auto-apply pipeline.

Why: when a GM Screen modifier card has been pushed to the sheet as a
bonus (via Plan C of GM Screen), AND remains active in the modifier
carousel, RollFromDataset would credit it twice: once via Foundry's
selectors auto-apply, once via the popover's poolModifier sum. The
direct-Roll path lets the popover own the pool delta unambiguously.

Adds computeBasicDice(actor, paths) helper that sums numeric leaves at
the dot-paths against actor.system.

rollMode (Task C1's new envelope field) is now passed through both
the direct-Roll and RollFromDataset branches.

Module version 0.4.0 -> 0.5.0 (semver minor: additive features within
protocol_version 1)."
```

---

## Task C4: PathProvider registry

**Goal:** Land the pluggable `PathProvider` foundation. V1 ships `ATTRIBUTE_PROVIDER` + `SKILL_PROVIDER` + `DEFAULT_PROVIDERS`. The popover (Task C6) iterates `DEFAULT_PROVIDERS` to render dropdowns. Future providers (`DISCIPLINE_PROVIDER`, `MERIT_BONUS_PROVIDER`, `RENOWN_PROVIDER`) are appended without touching the popover.

**Files:**
- Create: `src/lib/gm-screen/path-providers.ts`

**Anti-scope:** Do not import from this file yet (`roll.ts` and the popover do that in Tasks C5 and C6). Do not add discipline / merit / renown providers — V1 is attr+skill only.

**Depends on:** Plan A merged (uses `FOUNDRY_ATTRIBUTE_NAMES` / `FOUNDRY_SKILL_NAMES` / literal types from `canonical-names.ts`). Foundry helpers (uses `foundryAttrInt` / `foundrySkillInt` from `src/lib/foundry/raw.ts`).

**Invariants cited:** spec §6.5 (registry shape).

**Tests required:** NO. The two provider records are pure data; their `getOptions` is a one-line `.map`. Validated by `npm run check` + the popover's manual smoke.

- [x] **Step 1: Create the file with full content**

```typescript
// src/lib/gm-screen/path-providers.ts
//
// Pluggable registry for "what can the GM pick to feed value_paths in a roll".
// V1 ships attribute + skill providers; future plans append discipline /
// merit-bonus / renown / werewolf-rage providers WITHOUT touching the
// RollDispatcherPopover component.
//
// Usage: the popover iterates DEFAULT_PROVIDERS and renders one <select>
// per provider whose getOptions() returns a non-empty list. The composer
// (roll.ts::dispatchRoll) walks the same providers to build value_paths
// from the user's selections.

import type { BridgeCharacter } from '../../types';
import { foundryAttrInt, foundrySkillInt } from '../foundry/raw';
import {
  FOUNDRY_ATTRIBUTE_NAMES,
  FOUNDRY_SKILL_NAMES,
} from '../foundry/canonical-names';

/** One row in a provider's dropdown — what the GM picks. */
export interface PathProviderOption {
  /** Stable key used in the popover's selections record (e.g. 'strength'). */
  key: string;
  /** Display label shown in the dropdown (e.g. 'Strength'). */
  label: string;
  /** Current sheet value for the displayed "(3)" hint. */
  value: number;
  /** Foundry dot-path joined into value_paths (e.g. 'attributes.strength.value'). */
  path: string;
}

/** A category of stats the popover can pick from. */
export interface PathProvider {
  /** Stable id used as the key in the popover's selections record. */
  id: string;
  /** Display label rendered above the dropdown (e.g. 'Attribute'). */
  label: string;
  /** When true, the popover blocks submit until this provider has a selection. */
  required: boolean;
  /**
   * Returns the dropdown options for this character. Empty list = the popover
   * skips rendering this provider's <select> (useful for splat-aware future
   * providers like 'discipline' that return [] for non-vampire characters).
   */
  getOptions(char: BridgeCharacter): PathProviderOption[];
}

/** Capitalize the first letter (display-only helper). */
function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** WoD5e attributes (system.attributes.<key>.value). */
export const ATTRIBUTE_PROVIDER: PathProvider = {
  id: 'attribute',
  label: 'Attribute',
  required: true,
  getOptions: (char) =>
    FOUNDRY_ATTRIBUTE_NAMES.map((key) => ({
      key,
      label: capitalize(key),
      value: foundryAttrInt(char, key),
      path: `attributes.${key}.value`,
    })),
};

/** WoD5e skills (system.skills.<key>.value). */
export const SKILL_PROVIDER: PathProvider = {
  id: 'skill',
  label: 'Skill',
  required: true,
  getOptions: (char) =>
    FOUNDRY_SKILL_NAMES.map((key) => ({
      key,
      label: capitalize(key),
      value: foundrySkillInt(char, key),
      path: `skills.${key}.value`,
    })),
};

/**
 * V1 registry. Future providers are appended here without touching
 * RollDispatcherPopover.svelte or roll.ts:
 *
 *   export const DISCIPLINE_PROVIDER: PathProvider = { ... }
 *   export const MERIT_BONUS_PROVIDER: PathProvider = { ... }
 *   export const RENOWN_PROVIDER: PathProvider = { ... }
 *   export const DEFAULT_PROVIDERS = [
 *     ATTRIBUTE_PROVIDER,
 *     SKILL_PROVIDER,
 *     DISCIPLINE_PROVIDER,   // <-- here
 *   ];
 */
export const DEFAULT_PROVIDERS: readonly PathProvider[] = [
  ATTRIBUTE_PROVIDER,
  SKILL_PROVIDER,
];
```

- [x] **Step 2: Run `npm run check`**

Run: `npm run check`
Expected: PASS — type checks against `BridgeCharacter`, `foundryAttrInt`, `foundrySkillInt`, `FOUNDRY_*_NAMES`.

- [x] **Step 3: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/lib/gm-screen/path-providers.ts
git commit -m "feat(gm-screen): add PathProvider registry for roll dispatcher

Pluggable foundation for the popover's stat picker (Task C6). V1 ships
ATTRIBUTE_PROVIDER + SKILL_PROVIDER, registered in DEFAULT_PROVIDERS.
Future plans append DISCIPLINE_PROVIDER / MERIT_BONUS_PROVIDER /
RENOWN_PROVIDER to the array without touching the popover component
or the composer (per the user's extensibility preference; spec §2.5).

Reads from Plan A's canonical-names.ts (FOUNDRY_ATTRIBUTE_NAMES /
FOUNDRY_SKILL_NAMES) and the existing foundryAttrInt/foundrySkillInt
helpers in src/lib/foundry/raw.ts."
```

---

## Task C5: `roll.ts` — `summarizeModifiers` + `dispatchRoll`

**Goal:** Two pure helpers consumed by the popover. `summarizeModifiers` partitions effects by kind and sums deltas. `dispatchRoll` validates the popover's input record and calls `triggerFoundryRoll` with the composed payload (clamps difficulty to non-negative; auto-derives flavor when custom is empty).

**Files:**
- Create: `src/lib/gm-screen/roll.ts`

**Anti-scope:** Do not implement the popover here (Task C6). Do not add capability detection. Do not handle non-Foundry characters silently — throw defensively.

**Depends on:** Task C2 (TS `RollV5PoolInput` has `rollMode` + `poolModifier`), Task C4 (`PathProvider` type).

**Invariants cited:** spec §6.7 (modifier sum), §6.8 (composer), §8 error table.

**Tests required:** NO. Both functions are short; the project has no Vitest infrastructure (`package.json` has no test runner). Validated by `npm run check` for types + manual smoke through the popover (Task C6) and the end-of-plan smoke. If a real bug surfaces during smoke, that's the trigger to add Vitest as a follow-up — pre-installing it for two ~20-line functions doesn't earn its keep.

- [x] **Step 1: Create the file with full content**

```typescript
// src/lib/gm-screen/roll.ts
//
// Pure helpers consumed by RollDispatcherPopover.svelte:
//   - summarizeModifiers(mods): partition active non-hidden modifier effects
//     by kind, sum pool/difficulty deltas, collect note text for display.
//   - dispatchRoll(args): validate popover state, build value_paths from
//     PathProvider selections, call triggerFoundryRoll with the composed
//     payload (clamps difficulty to >= 0; uses customFlavor or auto-label).

import { triggerFoundryRoll } from '../foundry-chat/api';
import type { BridgeCharacter, CharacterModifier } from '../../types';
import type { PathProvider } from './path-providers';

/** Aggregated view of one character's active modifier deck. */
export interface ModifierSums {
  /** Sum of every active non-hidden effect's pool delta. */
  pool: number;
  /** Sum of every active non-hidden effect's difficulty delta. */
  difficulty: number;
  /** Notes from active non-hidden 'note'-kind effects (display-only). */
  notes: string[];
}

/**
 * Partition + sum effects from a character's modifier list. Filters to
 * isActive && !isHidden (matches the popover's "what's currently on?" view
 * — the user explicitly framed this as 'sum total of all on toggled cards').
 */
export function summarizeModifiers(mods: CharacterModifier[]): ModifierSums {
  const active = mods.filter((m) => m.isActive && !m.isHidden);
  const allEffects = active.flatMap((m) => m.effects);

  return {
    pool: allEffects
      .filter((e) => e.kind === 'pool')
      .reduce((sum, e) => sum + (e.delta ?? 0), 0),
    difficulty: allEffects
      .filter((e) => e.kind === 'difficulty')
      .reduce((sum, e) => sum + (e.delta ?? 0), 0),
    notes: allEffects
      .filter((e) => e.kind === 'note' && e.note != null)
      .map((e) => e.note!) as string[],
  };
}

/** Input record for dispatchRoll — matches the popover's local state shape. */
export interface DispatchRollArgs {
  char: BridgeCharacter;
  providers: readonly PathProvider[];
  /** Map of provider.id -> chosen option.key. e.g. { attribute: 'strength', skill: 'brawl' }. */
  selections: Record<string, string>;
  /** GM-typed base difficulty (before modifier difficulty sum). */
  baseDifficulty: number;
  /** Pre-computed sums (caller passes summarizeModifiers result). */
  modifierSums: ModifierSums;
  /** Empty string = derive label from selected option labels ("Strength + Brawl"). */
  customFlavor: string;
  rollMode: 'roll' | 'gmroll' | 'blindroll' | 'selfroll';
}

/**
 * Validate args, build the wire payload, fire-and-forget the IPC.
 * Throws synchronously on invariant violations (callee uses these errors
 * to drive an error-toast display).
 */
export async function dispatchRoll(args: DispatchRollArgs): Promise<void> {
  if (args.char.source !== 'foundry') {
    throw new Error(
      'gm-screen/roll: dispatchRoll requires a Foundry character',
    );
  }

  const valuePaths: string[] = [];
  const labelParts: string[] = [];

  for (const provider of args.providers) {
    const optionKey = args.selections[provider.id];
    if (!optionKey) {
      if (provider.required) {
        throw new Error(
          `gm-screen/roll: required provider '${provider.id}' has no selection`,
        );
      }
      continue;
    }
    const opt = provider
      .getOptions(args.char)
      .find((o) => o.key === optionKey);
    if (!opt) {
      throw new Error(
        `gm-screen/roll: provider '${provider.id}' option '${optionKey}' not found`,
      );
    }
    valuePaths.push(opt.path);
    labelParts.push(opt.label);
  }

  const flavor =
    args.customFlavor.trim() || labelParts.join(' + ') || 'Roll';
  const finalDifficulty = Math.max(
    0,
    args.baseDifficulty + args.modifierSums.difficulty,
  );

  await triggerFoundryRoll({
    actorId: args.char.source_id,
    valuePaths,
    difficulty: finalDifficulty,
    flavor,
    rollMode: args.rollMode,
    poolModifier: args.modifierSums.pool,
    advancedDice: null, // null = WoD5e auto-derive (hunger / rage / 0)
    selectors: [], // empty — JS executor's pool_modifier branch bypasses selector-based bonuses
  });
}
```

- [x] **Step 2: Run `npm run check`**

Run: `npm run check`
Expected: PASS — `triggerFoundryRoll` accepts the new `rollMode` / `poolModifier` fields (Task C2); `BridgeCharacter` / `CharacterModifier` already exist; `PathProvider` is from Task C4.

- [x] **Step 3: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/lib/gm-screen/roll.ts
git commit -m "feat(gm-screen): roll.ts -- summarizeModifiers + dispatchRoll

summarizeModifiers(mods) partitions a character's active non-hidden
modifier effects by kind ('pool' | 'difficulty' | 'note') and sums the
pool / difficulty deltas. Notes are collected as strings for display.
Filter matches the user's 'sum total of all on toggled cards' framing.

dispatchRoll(args) validates popover state, walks the PathProvider
registry to build value_paths from the GM's selections, and calls
triggerFoundryRoll with the composed payload. Final difficulty clamped
to >= 0 (Foundry rejects negative). Custom flavor falls back to
auto-label ('Strength + Brawl') when empty.

No tests yet -- project has no Vitest. Validated via npm run check +
end-of-plan manual smoke."
```

---

## Task C6: `RollDispatcherPopover.svelte` component

**Goal:** The popover itself. Provider dropdowns + difficulty input + privacy controls (custom flavor, roll mode) + active-modifier sums + Roll/Cancel buttons. Anchored via `position: fixed` with viewport coords (mirrors `CharacterRow.svelte`'s editor-popover pattern).

**Files:**
- Create: `src/lib/components/gm-screen/RollDispatcherPopover.svelte`

**Anti-scope:** Do not modify `CharacterRow.svelte` (Task C7). Do not add per-roll modifier override checkboxes (deferred per spec §9). Do not persist selections across opens (spec §13 open-question; current default = reset on close).

**Depends on:** Task C4 (PathProvider), Task C5 (summarizeModifiers + dispatchRoll).

**Invariants cited:** ARCH §6 (use `var(--*)` tokens; no hex literals). spec §6.1 (component shape), §6.10 (privacy-control UI surface).

**Tests required:** NO. Component test infrastructure doesn't exist; validated by manual smoke at the end of the plan.

- [x] **Step 1: Create the file with full content**

```svelte
<script lang="ts">
  import type { BridgeCharacter, CharacterModifier } from '../../../types';
  import {
    DEFAULT_PROVIDERS,
    type PathProvider,
  } from '../../gm-screen/path-providers';
  import {
    summarizeModifiers,
    dispatchRoll,
  } from '../../gm-screen/roll';

  interface Props {
    character: BridgeCharacter;
    modifiers: CharacterModifier[];
    /** Viewport-coord anchor from the trigger button's getBoundingClientRect. */
    anchor: { left: number; top: number };
    onclose: () => void;
  }
  let { character, modifiers, anchor, onclose }: Props = $props();

  // PathProvider state — keyed by provider.id, value is the chosen option.key.
  let selections = $state<Record<string, string>>({});

  // Difficulty + privacy state.
  let baseDifficulty = $state(0);
  let customFlavor = $state('');
  let rollMode = $state<'roll' | 'gmroll' | 'blindroll' | 'selfroll'>('roll');

  // Submission UI state.
  let submitting = $state(false);
  let errorMsg = $state<string | null>(null);

  // Derived: per-provider option lists (for the dropdowns).
  const providerOptions = $derived(
    DEFAULT_PROVIDERS.map((p) => ({
      provider: p,
      options: p.getOptions(character),
    })),
  );

  // Derived: modifier sums (recomputed if upstream modifier toggle changes).
  const sums = $derived(summarizeModifiers(modifiers));

  // Derived: validation — required providers must all have a selection.
  const requiredMissing = $derived(
    DEFAULT_PROVIDERS.filter(
      (p) => p.required && !selections[p.id],
    ).map((p) => p.label),
  );
  const canSubmit = $derived(requiredMissing.length === 0 && !submitting);

  // Derived: preview values shown to the GM before they roll.
  const baseFromStats = $derived.by(() => {
    let sum = 0;
    for (const { provider, options } of providerOptions) {
      const optKey = selections[provider.id];
      const opt = options.find((o) => o.key === optKey);
      if (opt) sum += opt.value;
    }
    return sum;
  });
  const finalPool = $derived(baseFromStats + sums.pool);
  const finalDifficulty = $derived(Math.max(0, baseDifficulty + sums.difficulty));

  async function onSubmit() {
    errorMsg = null;
    submitting = true;
    try {
      await dispatchRoll({
        char: character,
        providers: DEFAULT_PROVIDERS,
        selections,
        baseDifficulty,
        modifierSums: sums,
        customFlavor,
        rollMode,
      });
      onclose();
    } catch (e) {
      errorMsg = e instanceof Error ? e.message : String(e);
    } finally {
      submitting = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />
<!-- Click-outside scrim. The popover content stops propagation. -->
<button class="scrim" onclick={onclose} aria-label="Close roll dispatcher"></button>

<div
  class="popover"
  role="dialog"
  aria-label="Roll for {character.name}"
  style:left="{anchor.left}px"
  style:top="{anchor.top}px"
  onclick={(e) => e.stopPropagation()}
  onkeydown={(e) => e.stopPropagation()}
>
  <div class="header">Roll for <strong>{character.name}</strong></div>

  <div class="body">
    {#each providerOptions as { provider, options } (provider.id)}
      <label class="row">
        <span class="label">{provider.label}{provider.required ? ' *' : ''}</span>
        <select bind:value={selections[provider.id]}>
          <option value="">— pick {provider.label.toLowerCase()} —</option>
          {#each options as opt (opt.key)}
            <option value={opt.key}>{opt.label} ({opt.value})</option>
          {/each}
        </select>
      </label>
    {/each}

    <label class="row">
      <span class="label">Base difficulty</span>
      <input
        type="number"
        min="0"
        max="20"
        bind:value={baseDifficulty}
      />
    </label>

    {#if sums.pool !== 0 || sums.difficulty !== 0 || sums.notes.length > 0}
      <div class="modifier-summary">
        {#if sums.pool !== 0}
          <div class="mod-line">
            <span class="mod-label">Pool modifiers:</span>
            <span class="mod-val" class:positive={sums.pool > 0} class:negative={sums.pool < 0}>
              {sums.pool > 0 ? '+' : ''}{sums.pool}
            </span>
          </div>
        {/if}
        {#if sums.difficulty !== 0}
          <div class="mod-line">
            <span class="mod-label">Difficulty modifiers:</span>
            <span class="mod-val" class:positive={sums.difficulty > 0} class:negative={sums.difficulty < 0}>
              {sums.difficulty > 0 ? '+' : ''}{sums.difficulty}
            </span>
          </div>
        {/if}
        {#each sums.notes as note (note)}
          <div class="mod-note">📝 {note}</div>
        {/each}
      </div>
    {/if}

    <div class="totals">
      <div class="total-line">
        <span class="total-label">Pool:</span>
        <span class="total-val">
          {baseFromStats}{sums.pool !== 0 ? ` ${sums.pool > 0 ? '+' : '−'} ${Math.abs(sums.pool)}` : ''}
          {sums.pool !== 0 ? ` = ${finalPool}` : ''}
        </span>
      </div>
      <div class="total-line">
        <span class="total-label">Difficulty:</span>
        <span class="total-val">
          {baseDifficulty}{sums.difficulty !== 0 ? ` ${sums.difficulty > 0 ? '+' : '−'} ${Math.abs(sums.difficulty)}` : ''}
          {sums.difficulty !== 0 ? ` = ${finalDifficulty}` : ''}
        </span>
      </div>
    </div>

    <details class="privacy">
      <summary>Privacy / flavor</summary>
      <label class="row">
        <span class="label">Custom flavor</span>
        <input
          type="text"
          placeholder="auto: {providerOptions.map(({ provider, options }) => options.find((o) => o.key === selections[provider.id])?.label).filter(Boolean).join(' + ') || 'Roll'}"
          bind:value={customFlavor}
        />
      </label>
      <label class="row">
        <span class="label">Roll mode</span>
        <select bind:value={rollMode}>
          <option value="roll">Public roll</option>
          <option value="gmroll">GM only</option>
          <option value="blindroll">Blind roll</option>
          <option value="selfroll">Self roll</option>
        </select>
      </label>
    </details>

    {#if errorMsg}
      <div class="error">⚠ {errorMsg}</div>
    {/if}
  </div>

  <div class="actions">
    <button class="btn-cancel" onclick={onclose}>Cancel</button>
    <button class="btn-submit" disabled={!canSubmit} onclick={onSubmit}>
      {submitting ? 'Rolling…' : '🎲 Roll in Foundry'}
    </button>
  </div>
  {#if requiredMissing.length > 0}
    <div class="missing-hint">
      Pick: {requiredMissing.join(', ')}
    </div>
  {/if}
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: transparent;
    border: none;
    padding: 0;
    cursor: default;
    z-index: 100;
  }

  .popover {
    position: fixed;
    z-index: 101;
    min-width: 320px;
    max-width: 420px;
    background: var(--bg-raised);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    box-shadow: 0 6px 24px var(--shadow-strong);
    color: var(--text-primary);
  }

  .header {
    font-size: 0.85rem;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: 0.4rem;
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .row {
    display: grid;
    grid-template-columns: 7rem 1fr;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }

  .label {
    color: var(--text-ghost);
    text-transform: uppercase;
    font-size: 0.7rem;
    letter-spacing: 0.05em;
  }

  .row select,
  .row input {
    background: var(--bg-sunken);
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    font-size: 0.85rem;
  }

  .modifier-summary {
    background: var(--bg-sunken);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.8rem;
  }

  .mod-line {
    display: flex;
    justify-content: space-between;
  }

  .mod-label {
    color: var(--text-ghost);
  }

  .mod-val.positive {
    color: var(--accent-positive);
  }

  .mod-val.negative {
    color: var(--accent-negative);
  }

  .mod-note {
    color: var(--text-secondary);
    font-style: italic;
  }

  .totals {
    border-top: 1px dashed var(--border-subtle);
    padding-top: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.9rem;
  }

  .total-line {
    display: flex;
    justify-content: space-between;
  }

  .total-label {
    color: var(--text-ghost);
    font-weight: 600;
  }

  .total-val {
    font-weight: 700;
  }

  .privacy {
    background: var(--bg-sunken);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-size: 0.8rem;
  }

  .privacy summary {
    cursor: pointer;
    color: var(--text-ghost);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.7rem;
    padding: 0.1rem 0;
  }

  .privacy .row {
    margin-top: 0.4rem;
  }

  .error {
    background: var(--bg-error);
    color: var(--text-error);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    font-size: 0.8rem;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    border-top: 1px solid var(--border-subtle);
    padding-top: 0.5rem;
  }

  .btn-cancel,
  .btn-submit {
    padding: 0.35rem 0.8rem;
    border-radius: 4px;
    font-size: 0.85rem;
    cursor: pointer;
  }

  .btn-cancel {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }

  .btn-submit {
    background: var(--accent-primary);
    color: var(--text-on-accent);
    border: 1px solid var(--accent-primary);
    font-weight: 600;
  }

  .btn-submit:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .missing-hint {
    font-size: 0.7rem;
    color: var(--text-warning);
    text-align: right;
    padding-top: 0.2rem;
  }
</style>
```

**CSS token verification:** all colors reference existing `var(--*)` tokens from `src/routes/+layout.svelte`. If `--bg-error`, `--text-error`, `--accent-positive`, `--accent-negative`, `--text-on-accent`, or `--text-warning` don't exist, the implementer must either (a) substitute the closest existing token (check `+layout.svelte`'s `:root` block first) or (b) add the new token to `:root` per ARCH §6 (NEVER hardcode hex).

- [x] **Step 2: Run `npm run check`**

Run: `npm run check`
Expected: PASS — Svelte type check; the component imports compile.

- [x] **Step 3: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/lib/components/gm-screen/RollDispatcherPopover.svelte
git commit -m "feat(gm-screen): RollDispatcherPopover component

Anchored popover (position: fixed, viewport coords) that renders one
dropdown per PathProvider in DEFAULT_PROVIDERS, plus difficulty input,
modifier-sum summary, totals preview, and a collapsible Privacy section
(custom flavor + roll mode).

Wires onSubmit through dispatchRoll(). Validation + error display match
spec §8 error table. Click-outside scrim + Escape key both close the
popover (onclose callback prop).

Container styling uses var(--*) tokens only -- no hex literals per
ARCH §6. New tokens (--bg-error, --text-error, etc.) are referenced;
implementer should add to :root if missing or substitute existing
tokens that look closest."
```

---

## Task C7: Wire popover into `CharacterRow.svelte`

**Goal:** Add the 🎲 trigger button (Foundry-only) and mount the popover. Track `popoverOpen` and `popoverAnchor` state in the row component. Click the button → measure the trigger's bounding rect → open the popover anchored below it.

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`

**Anti-scope:** Do not change the existing modifier carousel, push-to-Foundry button, or status-template apply behavior. Do not change the row's overall layout. Do not modify the popover component (Task C6).

**Depends on:** Task C6 (the component being mounted exists).

**Invariants cited:** spec §6.6 (Foundry-only render), §6.9 (trigger placement).

**Tests required:** NO. Markup wiring only; validated by end-of-plan manual smoke.

- [x] **Step 1: Add the import**

Open `src/lib/components/gm-screen/CharacterRow.svelte`. Locate the existing imports at the top of `<script>`. Add:

```typescript
  import RollDispatcherPopover from './RollDispatcherPopover.svelte';
```

- [x] **Step 2: Add the popover state**

Find the existing `let editorOpen = $state(false);` declaration (around line 22). After the `popoverPos` declaration, add:

```typescript
  // Roll dispatcher popover state — anchored to the 🎲 button.
  let rollPopoverOpen = $state(false);
  let rollPopoverAnchor = $state<{ left: number; top: number } | null>(null);

  function openRollPopover(e: MouseEvent) {
    const btn = e.currentTarget as HTMLElement;
    const rect = btn.getBoundingClientRect();
    // Anchor below the button, left-aligned to the button's left edge.
    rollPopoverAnchor = { left: rect.left, top: rect.bottom + 4 };
    rollPopoverOpen = true;
  }

  function closeRollPopover() {
    rollPopoverOpen = false;
    rollPopoverAnchor = null;
  }
```

- [x] **Step 3: Add the trigger button + popover mount**

Locate the character header in the markup. The existing pattern places the modifier carousel below the character header. Find a structural location after the character header / name display but before the modifier carousel — the file is 429 lines so look for a `.character-header` or similar landmark. Add:

```svelte
{#if character.source === 'foundry'}
  <button
    class="roll-trigger"
    aria-label="Roll for {character.name}"
    onclick={openRollPopover}
    type="button"
  >
    🎲 Roll
  </button>
  {#if rollPopoverOpen && rollPopoverAnchor}
    <RollDispatcherPopover
      {character}
      modifiers={charMods}
      anchor={rollPopoverAnchor}
      onclose={closeRollPopover}
    />
  {/if}
{/if}
```

(`charMods` is the existing per-character modifier list — declared at `CharacterRow.svelte:51` as `let charMods = $derived(modifiers.forCharacter(character.source, character.source_id));`. It holds the full `CharacterModifier[]` for this character; `summarizeModifiers` inside the popover then filters by `isActive && !isHidden`.)

- [x] **Step 4: Add the `.roll-trigger` CSS rule**

In the `<style>` block at the bottom of `CharacterRow.svelte`, add:

```css
  .roll-trigger {
    background: var(--bg-sunken);
    color: var(--text-primary);
    border: 1px solid var(--border-subtle);
    border-radius: 4px;
    padding: 0.2rem 0.55rem;
    font-size: 0.8rem;
    cursor: pointer;
    margin-left: 0.5rem;
  }

  .roll-trigger:hover {
    background: var(--bg-raised);
  }
```

(Adjust `margin-left` if the button needs different spacing for its placement context — the goal is "small, unobtrusive, next to the existing chip strip or character header.")

- [x] **Step 5: Run `npm run check`**

Run: `npm run check`
Expected: PASS — Svelte type check passes.

- [x] **Step 6: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 7: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "feat(gm-screen): mount RollDispatcherPopover from CharacterRow

Adds 🎲 Roll trigger button on Foundry character rows (hidden on Roll20
per spec §6.6). Click measures the button's bounding rect via
getBoundingClientRect() and anchors the popover at viewport coords below
it (mirrors the existing editor-popover anchoring pattern in this file).

Wires onclose to clear popover state. Modifiers passed to popover come
from the existing per-character active-modifier list; the popover then
renders the live sums via summarizeModifiers."
```

---

## Final smoke test (manual, after all 7 tasks committed)

Per CLAUDE.md, verify.sh runs after each commit. The end-of-plan smoke validates the wire path and the popover end-to-end. **Each item below is a discrete verification — do not skip any.**

**Setup:**
- `npm run tauri dev` running.
- Foundry world connected (with module 0.5.0 loaded — re-enable the module after the bump).
- At least one vampire actor with known Strength + Brawl values.
- At least one active GM Screen modifier card on that character with a `pool` effect (e.g., `+2 pool, scope: Frenzy`).
- Optional second active modifier card with a `difficulty` effect (e.g., `+1 difficulty, scope: Bad lighting`).

- [x] **Stat-based roll happy path** — open GM Screen, find the Foundry character, click 🎲 Roll. Pick attribute=Strength, skill=Brawl, baseDifficulty=4. **Expected:** popover shows Pool: `[strSheetVal+brawlSheetVal]+2 = [sum]`, Difficulty: `4+1 = 5` (or whatever modifiers are active). Click "🎲 Roll in Foundry".

- [x] **Roll lands in Foundry chat** — switch to the Foundry browser tab. **Expected:** a chat card titled "Strength + Brawl" with the correct number of dice (matching the popover's "Pool" line) and difficulty 5. The roll outcome is real V5 dice math.

- [x] **Hidden roll via gmroll** — open the popover again. Same selections. Expand the "Privacy / flavor" details. Set Custom flavor = "Hidden roll", Roll mode = "GM only". Submit. **Expected:** the chat card appears only in the GM's view; players see nothing or only a placeholder. The flavor reads "Hidden roll" — NOT "Strength + Brawl".

- [x] **Difficulty clamp** — open the popover again. Set baseDifficulty = -10 (assuming no active difficulty modifiers). **Expected:** popover's "Difficulty" line shows `0` (clamped); on submit, the wire-side `difficulty` is `0` and Foundry doesn't reject.

- [x] **Required-provider validation** — open the popover. Without picking a Skill, click Roll. **Expected:** button is disabled OR the click produces an error toast like `"required provider 'skill' has no selection"`. The popover should display "Pick: Skill" hint.

- [x] **Pool modifier bypasses selectors** — push the active `+2 Frenzy` modifier card to the actor's sheet (Plan C of GM Screen — the "push to Foundry" button on the modifier card). Then open the popover, same Str+Brawl roll. **Expected:** the popover's "Pool" line shows `[sheet-pool]+2`, and the resulting Foundry chat card shows EXACTLY the popover's number — NOT [sheet-pool]+2+2 (which would happen if Foundry's selectors auto-applied the pushed bonus on top of the popover's poolModifier).

- [x] **Roll20 row regression** — find a Roll20 character row in GM Screen. **Expected:** NO 🎲 Roll button on the row.

- [x] **Old module / new desktop** — temporarily downgrade the module: edit `vtmtools-bridge/module.json` back to version 0.4.0, reload the module in Foundry. Trigger a roll from the popover. **Expected:** roll still happens in Foundry chat (graceful degradation per spec §7), but pool_modifier silently doesn't apply (the chat card may show the wrong dice count). Restore version 0.5.0 after this check.

- [x] **`rollMode` propagation in the non-pool-modifier branch (verification of spec §13 implementer note)** — set up a roll where pool_modifier=0 (no active pool modifier) with rollMode=gmroll. Submit. **Expected:** the chat card appears only in the GM view. **If this fails (i.e., the chat card is public despite gmroll being selected):** `RollFromDataset({ dataset: { rollMode } })` doesn't propagate. Apply the spec's fallback: edit `game.js::rollV5Pool` so the direct-`Roll` branch is **always** taken when `paths.length > 0` (remove the `RollFromDataset` else-branch entirely; conditionally set `basicDice = computeBasicDice(actor, paths) + poolModifier` in all cases). Document the decision in a follow-up commit.

---

## Self-review checklist

- [x] Both wire-payload fields (`roll_mode`, `pool_modifier`) appear in: Rust struct (Task C1), Rust builder envelope (C1), TS interface (C2), JS executor reads (C3). All four sides match.
- [x] `pool_modifier === 0` falls through to `RollFromDataset` (preserves existing behavior for non-popover callers); `pool_modifier !== 0` OR `paths.length === 0` switches to direct `Roll` (popover semantics + rouse-style preserved).
- [x] `summarizeModifiers` filters BOTH `isActive` AND `!isHidden` — matches the popover's "currently on" framing.
- [x] `dispatchRoll` clamps `finalDifficulty` to `>= 0` (Foundry rejects negative; clamping prevents a wire-level error from the popover).
- [x] `customFlavor.trim()` falls back to `labelParts.join(' + ')` (auto-derive) which falls back to `'Roll'` (when no providers selected — defensive).
- [x] Popover renders only on Foundry character rows (`{#if character.source === 'foundry'}` guard around BOTH the trigger button and the popover mount).
- [x] Module version bump matches the wire change scope (0.4.0 → 0.5.0; semver minor for additive features within protocol_version 1).

---

## Plan dependencies

- **Depends on:** Plan A merged. Does NOT depend on Plan B.
- **Blocks:** nothing in this umbrella; future plans may extend `DEFAULT_PROVIDERS` with discipline / merit / renown providers without touching anything from this plan.

---

## Execution handoff

Plan C is seven tasks. Recommended:
- **Subagent-driven:** one subagent per task. C1 is TDD; C2-C3 are wire wiring; C4-C5 are pure TS modules; C6 is the component; C7 is markup wiring. Total ~3-4 hr if dispatched serially.
- **Inline:** also fine; the plan reads cleanly top-to-bottom.

After all 7 tasks committed, run a SINGLE `code-review:code-review` against the full Plan C branch diff per the CLAUDE.md lean-flow override. The end-of-plan manual smoke (above) is the gate before that final review.
