# Foundry skills/attrs surface + GM-Screen roll dispatcher — design spec

> **Status:** designed; ready for plan-writing.
> **Issues:** [#28](https://github.com/Hampternt/vtmtools/issues/28) (Foundry portion of Phase 2.5 — closed by Plan B).
> **Source authority for Plan B:** `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md` §2.5 explicitly defers skills/attributes path-into-raw patching as the "Phase 2.5 follow-up." This spec ships that follow-up (Foundry only; Roll20 stays a stub per §2.8 of that spec).
> **Audience:** anyone implementing Plan A, B, or C below.

---

## §1 What this is

One umbrella spec covering three plans. Each plan is independently committable; Plans B and C are independently parallelizable (no file overlap — see §10).

| Plan | One-liner | Surface |
|---|---|---|
| **A** | Surface Foundry skills (alongside the already-shipped attributes) in the Campaign manager card layout | Frontend-only |
| **B** | Extend `character_set_field` to namespaced `attribute.*` / `skill.*` canonical names; logic-only, no UI | Backend (Rust + TS mirror) |
| **C** | GM Screen → Foundry roll dispatcher popover; sums active modifier deltas, attribute+skill picker, privacy controls | New Svelte component + small additive wire-payload extension to `RollV5PoolInput` |

The three plans are tied together because they share the same skill/attribute vocabulary and the same actor-sheet provenance, but each ships a coherent slice on its own.

---

## §2 Settled decisions (recap from brainstorm)

These are settled. Open questions live in §13.

### §2.1 One umbrella spec, three plans

Mirrors the GM Screen Plan A/B/C packaging the project just used. Each plan ends in `./scripts/verify.sh` + commit per CLAUDE.md.

### §2.2 Namespaced canonical names: `attribute.<key>`, `skill.<key>`

The canonical-name surface grows from 8 → 44 effective accepted names. Existing 8 flat names (`hunger`, `humanity`, etc.) are unchanged for backward compatibility. New names are dot-prefixed.

Note: the existing `ALLOWED_NAMES` const in Rust stays as the 8 legacy flat names — extending it inline would require unstable const-fn concat. The new acceptance check is `is_allowed_name(&str) -> bool` which probes three arrays (`FLAT_NAMES`, `ATTRIBUTE_NAMES`, `SKILL_NAMES`). See §5.1 for the implementation.

Rationale for the dot-prefix shape: avoids any future collision risk between an attribute name and a top-level field, makes the kind of name immediately obvious at the call site, and gives TS template-literal types something concrete to constrain against.

### §2.3 Foundry-only for Plan B

Roll20 mapping for skills/attrs is deferred (matches the existing posture from §2.8 of `2026-05-02-character-set-field-router-design.md`). `canonical_to_roll20_attr` returns `None` for the new names; the existing fast-fail error message at `target=Live + source=Roll20 + canonical name` covers them.

### §2.4 Plan C: stat-based (attribute + skill) base pool, modifiers add on top

Two required dropdowns (attribute, skill) populated from the actor's sheet. Difficulty number input. Active non-hidden modifier cards' pool deltas + difficulty deltas auto-summed and surfaced; user does NOT tick per-roll which modifiers apply (active state on the card IS the on/off — matches the user's "sum total of all on toggled cards" framing).

### §2.5 Plan C value-path picker is pluggable

Even at V1 (which only renders attribute + skill dropdowns), the popover iterates a `PathProvider[]` registry. Future providers (`DISCIPLINE_PROVIDER`, `MERIT_BONUS_PROVIDER`, `RENOWN_PROVIDER`, etc.) drop in without touching the popover component. Aligns with the project's standing extensibility preference.

### §2.6 Plan C: privacy controls baked in regardless

Custom flavor text override + `roll_mode` selector (`roll` / `gmroll` / `blindroll` / `selfroll`). Required because the GM frequently makes rolls that should not reveal what's being rolled.

### §2.7 Plan C: pool_modifier on the wire, not via Foundry's `selectors`

Foundry's `RollFromDataset` computes the basic pool from `value_paths` summed against actor stats — it does NOT accept a "+N basic dice" override. Two options were considered:
- (a) Push our modifiers as Foundry sheet bonuses first, then call the regular roll (lets Foundry's selectors-based auto-apply do the work).
- (b) Add `pool_modifier: Option<i32>` to `RollV5PoolInput`; JS executor switches to direct `WOD5E.api.Roll({ basicDice, ... })` and computes `basicDice` itself.

(a) is invasive (write-then-roll) and risks **double-counting** — if a modifier card has previously been pushed to the sheet AND remains active in the popover, both Foundry's selectors-based auto-apply AND our pool_modifier would credit the same card. (b) is honest, additive, and bypasses selectors-based auto-apply by design (since the popover's modifier sum already reflects the GM's intent).

**Decided:** (b). See §6.4 for the JS executor behavior.

---

## §3 Architecture (high level)

```
GM clicks 🎲 Roll button on a CharacterRow (Foundry chars only — see §6.6)
  ▼
RollDispatcherPopover (new component, anchored under the row)
  │ reads:
  │   - bridgeStore (live actor for value paths, attribute/skill values)
  │   - modifiersStore (active non-hidden modifiers for this character)
  │
  │ user fills: attribute, skill, baseDifficulty, optional customFlavor, rollMode
  │
  │ on submit, dispatchRoll() composes:
  ▼
triggerFoundryRoll({
  actorId,
  valuePaths: [<attrPath>, <skillPath>],
  difficulty: baseDifficulty + diffModifierSum,
  flavor: customFlavor ?? autoLabel,
  rollMode,
  poolModifier: poolModifierSum,                  // NEW field (§6.2)
  advancedDice: null,                             // null = WoD5e auto-derive (hunger for vampires)
  selectors: [],                                  // empty — see §6.4 on bypassing selector-based auto-apply
})
  ▼
IPC trigger_foundry_roll → game.roll_v5_pool wire envelope
  ▼
foundry-actions/game.js: branch on poolModifier presence (§6.4)
  ▼
Foundry chat shows the roll (visibility per rollMode)
```

---

## §4 Plan A — Skills surface in Campaign manager (and fix Foundry attr display)

### §4.0 Bug fix bundled in Plan A

**The current "Core attributes" display in `Campaign.svelte` is Roll20-only.** Lines `569-578` read via `r20AttrInt(char, 'strength')` etc., which calls `r20Attrs(char)` → returns `[]` for non-Roll20 sources → all 9 attributes display as `0` for any Foundry character today.

Plan A fixes this as part of the same component touch. A new source-aware `attrInt(char, name)` helper dispatches to `r20AttrInt` (Roll20) or `foundryAttrInt` (Foundry) by `char.source`. Same pattern for the new `skillInt(char, name)` introduced for the Skills section.

This bug fix is in scope here because (a) the user's original "attrs aren't tracked properly" framing covers it, (b) Plan A is already touching the attribute markup, and (c) it would be misleading to ship a Skills section that works for Foundry characters while the adjacent attribute section silently shows zeros for the same characters.

### §4.1 What changes

`src/tools/Campaign.svelte` gets:
1. A source-aware `attrInt(char, name)` helper that replaces the 9 inline `r20AttrInt(char, ...)` calls in the attribute section.
2. A new collapsible section "Skills" beneath the existing "Core attributes" collapsible. Reads use a parallel `skillInt(char, skillName)` helper that dispatches to `foundrySkillInt` (Foundry) or returns `0` (Roll20 — see §4.2).

### §4.2 Display rules

- Show **all 27 skills**, sorted alphabetically. Empty/zero values render as `0` (matches the attribute layout's behavior for unset values).
- Layout: 3-column grid (3 cols × 9 rows fits 27 cleanly).
- New `expandedSkills: Set<charKey>` toggle state in `Campaign.svelte`, mirrors the existing `expandedAttrs` pattern.
- New `skills` footer toggle button next to the existing `attrs` / `info` / `feats` buttons.
- Roll20 characters: section toggle button is **not rendered** at all (`{#if char.source === 'foundry'}` guard around the button). Attempting to read Roll20 skills via this codepath returns 0; rendering a section of 27 zeros would be misleading. The attribute section continues to render for Roll20 (it works correctly via `r20AttrInt`).

### §4.3 Skill name list

Canonical 27 from WoD5e v5.3.17 (verified against `docs/reference/foundry-vtm5e-actor-sample.json`):

```
academics, animalken, athletics, awareness, brawl, craft, drive,
etiquette, finance, firearms, insight, intimidation, investigation,
larceny, leadership, medicine, melee, occult, performance, persuasion,
politics, science, stealth, streetwise, subterfuge, survival, technology
```

Lives as a single TS const `FOUNDRY_SKILL_NAMES: readonly SkillName[]` in a new `src/lib/foundry/canonical-names.ts` module that Plan A creates. **This module is the foundation for Plan B's TS literal types and Plan C's PathProvider lookups** — both import from it. See §5.5 and §6.5.

```ts
// src/lib/foundry/canonical-names.ts (NEW — Plan A creates)
export type AttributeName =
  | 'charisma' | 'composure' | 'dexterity' | 'intelligence'
  | 'manipulation' | 'resolve' | 'stamina' | 'strength' | 'wits';

export type SkillName =
  | 'academics' | 'animalken' | 'athletics' | 'awareness' | 'brawl'
  | 'craft' | 'drive' | 'etiquette' | 'finance' | 'firearms' | 'insight'
  | 'intimidation' | 'investigation' | 'larceny' | 'leadership' | 'medicine'
  | 'melee' | 'occult' | 'performance' | 'persuasion' | 'politics' | 'science'
  | 'stealth' | 'streetwise' | 'subterfuge' | 'survival' | 'technology';

// Mirrors src-tauri/src/shared/canonical_fields.rs::ATTRIBUTE_NAMES.
// When changing this list, update the Rust array in the same commit.
export const FOUNDRY_ATTRIBUTE_NAMES: readonly AttributeName[] = [
  'charisma', 'composure', 'dexterity', 'intelligence',
  'manipulation', 'resolve', 'stamina', 'strength', 'wits',
] as const;

// Mirrors src-tauri/src/shared/canonical_fields.rs::SKILL_NAMES.
export const FOUNDRY_SKILL_NAMES: readonly SkillName[] = [
  'academics', 'animalken', 'athletics', 'awareness', 'brawl', 'craft',
  'drive', 'etiquette', 'finance', 'firearms', 'insight', 'intimidation',
  'investigation', 'larceny', 'leadership', 'medicine', 'melee', 'occult',
  'performance', 'persuasion', 'politics', 'science', 'stealth',
  'streetwise', 'subterfuge', 'survival', 'technology',
] as const;
```

### §4.4 Files (Plan A)

| Action | File | Reason |
|---|---|---|
| Create | `src/lib/foundry/canonical-names.ts` | Canonical name lists + literal types for skills + attributes (foundation for Plan B's literal types and Plan C's PathProvider lookups) |
| Modify | `src/tools/Campaign.svelte` | (1) Add source-aware `attrInt` / `skillInt` helpers; (2) replace 9 inline `r20AttrInt(char, ...)` calls in the attribute section with `attrInt`; (3) add Skills collapsible section + toggle state |

No backend changes. No new IPC commands.

---

## §5 Plan B — `character_set_field` skills/attrs extension (Foundry only)

### §5.1 Vocabulary extension

`src-tauri/src/shared/canonical_fields.rs` grows three `pub const` arrays:

```rust
pub const ATTRIBUTE_NAMES: &[&str] = &[
    "charisma", "composure", "dexterity", "intelligence",
    "manipulation", "resolve", "stamina", "strength", "wits",
];

pub const SKILL_NAMES: &[&str] = &[
    "academics", "animalken", "athletics", "awareness", "brawl",
    "craft", "drive", "etiquette", "finance", "firearms", "insight",
    "intimidation", "investigation", "larceny", "leadership", "medicine",
    "melee", "occult", "performance", "persuasion", "politics", "science",
    "stealth", "streetwise", "subterfuge", "survival", "technology",
];

pub const FLAT_NAMES: &[&str] = &[ /* existing 8 */ ];
```

`ALLOWED_NAMES` is computed at runtime via a `OnceLock<Vec<&'static str>>` (or simply checked via `is_allowed_name(&str) -> bool` that probes all three arrays — cheaper, avoids the lazy-init):

```rust
pub fn is_allowed_name(name: &str) -> bool {
    if FLAT_NAMES.contains(&name) { return true; }
    if let Some(rest) = name.strip_prefix("attribute.") {
        return ATTRIBUTE_NAMES.contains(&rest);
    }
    if let Some(rest) = name.strip_prefix("skill.") {
        return SKILL_NAMES.contains(&rest);
    }
    false
}
```

Existing `ALLOWED_NAMES.contains(&name.as_str())` checks in `tools/character.rs` are replaced with `is_allowed_name(&name)`. **`ALLOWED_NAMES` itself is preserved** as a public const for callers/tests that want to iterate the flat list (it stays the 8 legacy names — extending it inline would require unstable const-fn concat).

### §5.2 Apply layer (path-walking patcher)

`apply_canonical_field` extends with two prefix arms:

```rust
pub fn apply_canonical_field(
    c: &mut CanonicalCharacter,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    // Existing 8 flat arms unchanged...

    if let Some(key) = name.strip_prefix("attribute.") {
        return apply_attribute(c, key, value);
    }
    if let Some(key) = name.strip_prefix("skill.") {
        return apply_skill(c, key, value);
    }

    Err(format!("character/set_field: unknown field '{name}'"))
}

fn apply_attribute(c: &mut CanonicalCharacter, key: &str, value: &Value) -> Result<(), String> {
    if !ATTRIBUTE_NAMES.contains(&key) {
        return Err(format!("character/set_field: unknown attribute '{key}'"));
    }
    let n = expect_u8_in_range(value, &format!("attribute.{key}"), 0, 5)?;
    let ptr = format!("/system/attributes/{key}/value");
    set_raw_u8(&mut c.raw, &ptr, n)?;
    Ok(())
}

fn apply_skill(c: &mut CanonicalCharacter, key: &str, value: &Value) -> Result<(), String> {
    if !SKILL_NAMES.contains(&key) {
        return Err(format!("character/set_field: unknown skill '{key}'"));
    }
    let n = expect_u8_in_range(value, &format!("skill.{key}"), 0, 5)?;
    let ptr = format!("/system/skills/{key}/value");
    set_raw_u8(&mut c.raw, &ptr, n)?;
    Ok(())
}

/// Walks `c.raw` by JSON Pointer and overwrites the value with `n`.
/// Creates intermediate objects as needed so a saved-side write can succeed
/// even for actors whose raw blob hasn't seen this skill before.
fn set_raw_u8(raw: &mut Value, pointer: &str, n: u8) -> Result<(), String> {
    // Walk the pointer, creating empty objects for missing intermediate keys.
    // At the leaf, set `Value::from(n as u64)`.
    // Implementation: iterate path segments split by '/'; for each non-leaf
    // segment, get_mut or insert empty object; at leaf, assign.
    // Returns Err if a non-leaf segment exists but is not an object.
    /* ~20 lines */
    Ok(())
}
```

### §5.3 Foundry path translation

`canonical_to_foundry_path` signature changes from `Option<&'static str>` to `Option<String>` (because the formatted paths are dynamic):

```rust
pub fn canonical_to_foundry_path(name: &str) -> Option<String> {
    // Existing 8 flat arms return Some(static_str.to_string())
    if let Some(key) = name.strip_prefix("attribute.") {
        if !ATTRIBUTE_NAMES.contains(&key) { return None; }
        return Some(format!("system.attributes.{key}.value"));
    }
    if let Some(key) = name.strip_prefix("skill.") {
        if !SKILL_NAMES.contains(&key) { return None; }
        return Some(format!("system.skills.{key}.value"));
    }
    None
}
```

Callers (`bridge/foundry/mod.rs::canonical_to_path`) that consumed `&'static str` now consume `String` — single-location update.

### §5.4 Roll20 path translation

`canonical_to_roll20_attr` continues to return `None` for the new names. The existing fast-fail error message in `tools/character.rs` at `target=Live + Roll20 + canonical name` covers them — no change needed.

### §5.5 TS mirror

`AttributeName` and `SkillName` literal types are defined in Plan A's `src/lib/foundry/canonical-names.ts` (see §4.3). Plan B re-exports the literal types from `src/types.ts` and uses them to extend `CanonicalFieldName`:

```ts
// src/types.ts (extend)
import type { AttributeName, SkillName } from './lib/foundry/canonical-names';
export type { AttributeName, SkillName };

export type CanonicalFieldName =
  | 'hunger' | 'humanity' | 'humanity_stains'   // existing 8
  | 'blood_potency' | 'health_superficial' | 'health_aggravated'
  | 'willpower_superficial' | 'willpower_aggravated'
  | `attribute.${AttributeName}`
  | `skill.${SkillName}`;
```

Template literal types give autocomplete + collision checking at the call site.

**Plan B depends on Plan A** for the literal-type definitions. Order: Plan A first, then Plan B and Plan C in any order (or in parallel).

### §5.6 Drift policy: manual checklist (matches existing convention)

The Rust `ATTRIBUTE_NAMES` / `SKILL_NAMES` arrays and the TS `AttributeName` / `SkillName` literal unions are **parallel sources of truth**. Drift between them produces silent type holes (a name that exists in TS but not Rust, or vice versa, won't fail the type-checker — TS compiles, Rust fast-fails at runtime with `"unknown attribute"`).

Three options were considered:

- (a) Code-generate the TS list from the Rust list at build time (heavy; new build step).
- (b) Add a `#[test]` that reads the TS file and asserts the union members match the Rust arrays (medium; brittle string parsing).
- (c) Manual checklist: when adding/removing a name, update both sides in the same commit. Matches the **existing precedent** for `BridgeCharacter` (whose Rust↔TS mirror is hand-maintained, see comment in `src-tauri/src/bridge/types.rs` and the `// Mirrors src-tauri/src/bridge/types.rs` comment in `src/types.ts`).

**Decided:** (c). Lightweight, matches the project's standing convention, sufficient for N=36 names that almost never change. Add an inline comment to both files calling out the mirror.

### §5.7 Tests (Plan B)

Per `superpowers:test-driven-development` policy from CLAUDE.md, this plan does involve genuine logic (path-walking patcher), so tests are required.

**`shared/canonical_fields.rs`:**
- Coverage assertion (extended from existing): every flat name AND every `attribute.*` AND every `skill.*` permutation has a Foundry path AND can be applied via `apply_canonical_field`.
- `apply_attribute` happy path (one parameterized test over `ATTRIBUTE_NAMES`).
- `apply_skill` happy path (one parameterized test over `SKILL_NAMES`).
- `apply_attribute` rejects unknown key under known prefix (`attribute.foo`).
- `apply_skill` rejects unknown key (`skill.bar`).
- `apply_attribute` rejects out-of-range integer (6).
- `apply_attribute` rejects wrong type (string for `attribute.strength`).
- `set_raw_u8` creates intermediate objects when missing (saved-side scenario where the actor's raw blob hasn't seen the skill yet).

**`db/saved_character.rs` (extend existing tests):**
- `patch_saved_field('attribute.strength', 4)` happy path: read → mutate raw blob → write → re-read shows new value at `system.attributes.strength.value`.
- `patch_saved_field('skill.brawl', 3)` happy path.

**`tools/character.rs` (extend existing tests):**
- `target=Live` Foundry source `attribute.strength` → outbound payload routed (mock `BridgeConn`).
- `target=Live` Roll20 source `skill.brawl` → fast-fail error (matches existing flat-name error format).
- `target=Both` happy path for `skill.brawl` — saved-first ordering preserved.

### §5.8 Files (Plan B)

| Action | File | Reason |
|---|---|---|
| Modify | `src-tauri/src/shared/canonical_fields.rs` | Add `ATTRIBUTE_NAMES`/`SKILL_NAMES`, `is_allowed_name`, `apply_attribute`/`apply_skill`, `set_raw_u8`; change `canonical_to_foundry_path` return type to `Option<String>` |
| Modify | `src-tauri/src/tools/character.rs` | Replace `ALLOWED_NAMES.contains(...)` with `is_allowed_name(...)` |
| Modify | `src-tauri/src/bridge/foundry/mod.rs` | Update `canonical_to_path` for `Option<String>` signature |
| Modify | `src-tauri/src/db/saved_character.rs` | New tests; no source changes (`db_patch_field` already calls `apply_canonical_field` so the new arms wire through automatically) |
| Modify | `src/types.ts` | Add `AttributeName`, `SkillName` literal unions; extend `CanonicalFieldName` |
| Modify | `src/lib/character/api.ts` | Re-export new types if needed |

Total: 6 modifications, 0 new files. No new SQL migrations; no new Tauri commands; no new wire variants.

---

## §6 Plan C — GM Screen roll dispatcher popover

### §6.1 Component

`src/lib/components/gm-screen/RollDispatcherPopover.svelte` — new. Anchored under the character header (CSS `position: absolute` over the `CharacterRow`). Click outside or Escape dismisses.

Local component state (Svelte 5 runes):
- `selectedAttr: AttributeName | null` — defaults to first attribute alphabetically (charisma) or remembers last selection per character via a small in-memory map (no persistence needed).
- `selectedSkill: SkillName | null` — same.
- `baseDifficulty: number` — defaults to `0`.
- `customFlavor: string` — defaults to empty (auto-derived label is used).
- `rollMode: 'roll' | 'gmroll' | 'blindroll' | 'selfroll'` — defaults to `'roll'`.

No store. The popover is per-character ephemeral. Re-opening clears state (or restores last selection — see §13 open question).

### §6.2 Wire-format additive change to `RollV5PoolInput`

**Both fields are NEW.** The existing `RollV5PoolInput` has neither.

```rust
// src-tauri/src/bridge/foundry/types.rs
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollV5PoolInput {
    pub actor_id: String,
    pub value_paths: Vec<String>,
    pub difficulty: u8,
    pub flavor: Option<String>,
    pub advanced_dice: Option<u8>,
    pub selectors: Option<Vec<String>>,
    pub roll_mode: Option<String>,                  // NEW
    pub pool_modifier: Option<i32>,                 // NEW (i32 — modifier can be negative)
}
```

```ts
// src/lib/foundry-chat/api.ts (modify existing)
export interface RollV5PoolInput {
  actorId: string;
  valuePaths: string[];
  difficulty: number;
  flavor?: string | null;
  advancedDice?: number | null;
  selectors?: string[] | null;
  rollMode?: 'roll' | 'gmroll' | 'blindroll' | 'selfroll' | null;     // NEW
  poolModifier?: number | null;                                       // NEW
}
```

### §6.3 Rust builder validation extension

`build_roll_v5_pool` in `src-tauri/src/bridge/foundry/actions/game.rs` extends:

```rust
const VALID_ROLL_MODES: &[&str] = &["roll", "gmroll", "blindroll", "selfroll"];

pub fn build_roll_v5_pool(input: &RollV5PoolInput) -> Result<Value, String> {
    if input.actor_id.is_empty() {
        return Err("foundry/game.roll_v5_pool: actor_id is required".into());
    }
    if let Some(rm) = &input.roll_mode {
        if !VALID_ROLL_MODES.contains(&rm.as_str()) {
            return Err(format!(
                "foundry/game.roll_v5_pool: invalid roll_mode: {rm}"
            ));
        }
    }
    // No range check on pool_modifier — negative is valid (penalty), and
    // overflow into actor stat sums is impossible at i32 precision.

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

### §6.4 JS executor — branch on pool_modifier presence

`vtmtools-bridge/scripts/foundry-actions/game.js::rollV5Pool` extends:

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
  // pool_modifier (popover semantics — bypass selectors-based auto-apply
  // to avoid double-counting modifier cards that have already been pushed
  // to the sheet as bonuses).
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

  // RollFromDataset path (auto-applies sheet bonuses via selectors). Used
  // for non-popover callers that want Foundry's full sheet-bonus pipeline.
  await WOD5E.api.RollFromDataset({
    dataset: {
      valuePaths: paths.join(" "),
      label,
      difficulty: msg.difficulty,
      selectDialog: false,
      advancedDice,
      selectors: msg.selectors ?? [],
      rollMode,
    },
    actor,
  });
}

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

**Critical behavior note** (call out in spec, repeat in implementer's plan task):
- When `poolModifier` is non-zero, the executor uses `WOD5E.api.Roll` directly. This **bypasses Foundry's selectors-based situational-bonus auto-apply**. The popover's modifier sum already encodes the GM's intent, and Foundry's selectors would double-count any modifier card that has previously been pushed to the sheet via the GM Screen Plan C "push to Foundry" feature.
- When `poolModifier` is zero (and `paths.length > 0`), the executor uses `RollFromDataset` (preserves existing behavior for non-popover callers).
- When `paths.length === 0` (rouse-style), the executor uses `WOD5E.api.Roll` directly with `basicDice = poolModifier` (which may be 0).

### §6.5 Pluggable PathProvider registry

```ts
// src/lib/gm-screen/path-providers.ts (new)
import type { BridgeCharacter } from '../../types';
import { foundryAttrInt, foundrySkillInt } from '../foundry/raw';
import { FOUNDRY_ATTRIBUTE_NAMES, FOUNDRY_SKILL_NAMES } from '../foundry/canonical-names';

export interface PathProviderOption {
  key: string;            // e.g. 'strength' or 'brawl'
  label: string;          // e.g. 'Strength' or 'Brawl'
  value: number;          // current sheet value (for the displayed "(3)" hint)
  path: string;           // Foundry dot-path: 'attributes.strength.value'
}

export interface PathProvider {
  id: string;             // 'attribute' | 'skill' | (future) 'discipline' | ...
  label: string;          // 'Attribute' | 'Skill' | ...
  required: boolean;      // true for V1 attr+skill; future providers may be optional
  getOptions(char: BridgeCharacter): PathProviderOption[];
}

export const ATTRIBUTE_PROVIDER: PathProvider = {
  id: 'attribute',
  label: 'Attribute',
  required: true,
  getOptions: (char) => FOUNDRY_ATTRIBUTE_NAMES.map((key) => ({
    key,
    label: capitalize(key),
    value: foundryAttrInt(char, key),
    path: `attributes.${key}.value`,
  })),
};

export const SKILL_PROVIDER: PathProvider = {
  id: 'skill',
  label: 'Skill',
  required: true,
  getOptions: (char) => FOUNDRY_SKILL_NAMES.map((key) => ({
    key,
    label: capitalize(key),
    value: foundrySkillInt(char, key),
    path: `skills.${key}.value`,
  })),
};

// V1 registry. Future providers (DISCIPLINE_PROVIDER, MERIT_BONUS_PROVIDER,
// RENOWN_PROVIDER) are appended here without touching the popover component.
export const DEFAULT_PROVIDERS: readonly PathProvider[] = [
  ATTRIBUTE_PROVIDER,
  SKILL_PROVIDER,
];

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
```

The popover renders one `<select>` per provider in `DEFAULT_PROVIDERS`. Adding a third provider = one entry in the array.

### §6.6 Roll20 character behavior

The 🎲 Roll button **is not rendered** on Roll20 character rows. Same pattern as Plan A's skills section: `{#if character.source === 'foundry'}` guard around the button.

Rationale: `triggerFoundryRoll` is Foundry-only by definition; rendering a disabled button with a "Foundry only" tooltip would be visual noise on every Roll20 row. When/if Roll20 grows roll-dispatch capability, this guard becomes a per-source dispatch.

### §6.7 Modifier sum

```ts
// src/lib/gm-screen/roll.ts (new) — pure helper
import type { CharacterModifier } from '../../types';

export interface ModifierSums {
  pool: number;
  difficulty: number;
  notes: string[];        // display-only for the popover's "active notes" line
}

export function summarizeModifiers(mods: CharacterModifier[]): ModifierSums {
  const active = mods.filter(m => m.isActive && !m.isHidden);
  const allEffects = active.flatMap(m => m.effects);

  return {
    pool: allEffects
      .filter(e => e.kind === 'pool')
      .reduce((sum, e) => sum + (e.delta ?? 0), 0),
    difficulty: allEffects
      .filter(e => e.kind === 'difficulty')
      .reduce((sum, e) => sum + (e.delta ?? 0), 0),
    notes: allEffects
      .filter(e => e.kind === 'note' && e.note != null)
      .map(e => e.note!),
  };
}
```

### §6.8 Composer

```ts
// src/lib/gm-screen/roll.ts (continued)
import { triggerFoundryRoll } from '../foundry-chat/api';
import type { BridgeCharacter } from '../../types';
import type { PathProvider } from './path-providers';

export interface DispatchRollArgs {
  char: BridgeCharacter;
  providers: readonly PathProvider[];
  selections: Record<string /* providerId */, string /* optionKey */>;
  baseDifficulty: number;
  modifierSums: ModifierSums;
  customFlavor: string;            // empty string = use auto-derived label
  rollMode: 'roll' | 'gmroll' | 'blindroll' | 'selfroll';
}

export async function dispatchRoll(args: DispatchRollArgs): Promise<void> {
  if (args.char.source !== 'foundry') {
    throw new Error('gm-screen/roll: dispatchRoll requires a Foundry character');
  }

  // Build value_paths from selections via the providers registry.
  const valuePaths: string[] = [];
  const labelParts: string[] = [];
  for (const provider of args.providers) {
    const optionKey = args.selections[provider.id];
    if (!optionKey) {
      if (provider.required) {
        throw new Error(`gm-screen/roll: required provider '${provider.id}' has no selection`);
      }
      continue;
    }
    const opt = provider.getOptions(args.char).find(o => o.key === optionKey);
    if (!opt) {
      throw new Error(`gm-screen/roll: provider '${provider.id}' option '${optionKey}' not found`);
    }
    valuePaths.push(opt.path);
    labelParts.push(opt.label);
  }

  const flavor = args.customFlavor.trim() || labelParts.join(' + ');
  const finalDifficulty = Math.max(0, args.baseDifficulty + args.modifierSums.difficulty);

  await triggerFoundryRoll({
    actorId: args.char.source_id,
    valuePaths,
    difficulty: finalDifficulty,
    flavor,
    rollMode: args.rollMode,
    poolModifier: args.modifierSums.pool,
    advancedDice: null,    // null = WoD5e auto-derive (hunger/rage/0 by splat)
    selectors: [],         // empty — JS executor's pool_modifier branch bypasses selector-based bonuses
  });
}
```

### §6.9 CharacterRow wiring

`src/lib/components/gm-screen/CharacterRow.svelte` adds a small button next to the existing chip strip (between the character header and the modifier carousel) — only rendered for Foundry characters. Clicking toggles the popover open/closed; clicking outside or pressing Escape dismisses.

```svelte
{#if character.source === 'foundry'}
  <button class="roll-trigger" aria-label="Roll for {character.name}"
          onclick={() => popoverOpen = !popoverOpen}>
    🎲 Roll
  </button>
  {#if popoverOpen}
    <RollDispatcherPopover
      {character}
      modifiers={charModifiers}
      onclose={() => popoverOpen = false}
    />
  {/if}
{/if}
```

### §6.10 Privacy controls (UI surface)

Inside the popover, beneath the difficulty input and above the action buttons:

- **Custom flavor text** input (single-line). Placeholder shows the auto-derived label (e.g. "Strength + Brawl"). Leaving empty uses the auto-label.
- **Roll mode** select: `Public roll` / `GM only` / `Blind roll` / `Self roll`. Default `Public roll`.

Both controls map directly to `customFlavor` and `rollMode` state.

### §6.11 Tests (Plan C)

The project has no TS/JS test infrastructure (no Vitest, no Jest — verified against `package.json`). Adding it for Plan C's two small helpers would be infrastructure overkill. Plan C testing therefore matches the project's existing convention: **Rust unit tests for the wire-format builder**, **`npm run check` for type correctness of the new TS modules**, **manual smoke for the helpers and the popover end-to-end**.

**Rust unit tests (`actions/game.rs`):**
- `roll_v5_pool_envelope_includes_roll_mode_and_pool_modifier` — happy path verifying both new fields land in the envelope.
- `roll_v5_pool_default_roll_mode_is_roll` — omitted `roll_mode` defaults to `"roll"` in the envelope.
- `roll_v5_pool_default_pool_modifier_is_zero` — omitted `pool_modifier` defaults to `0` in the envelope.
- `roll_v5_pool_invalid_roll_mode_errors` — bad mode rejected with the canonical error format.
- `roll_v5_pool_negative_pool_modifier_passes_through` — `-2` survives.

**TS validation:**
- `npm run check` — proves `path-providers.ts`, `roll.ts`, and `RollDispatcherPopover.svelte` all type-check against the new wire-format types.
- `summarizeModifiers` and `dispatchRoll` are intentionally simple (one filter+reduce per kind, plus argument validation throws). Manual smoke (§11) covers all branches by exercising the popover with characters that have varied modifier states.

**JS-side validation:**
- `computeBasicDice` and the `rollV5Pool` executor branching are validated via manual smoke per §11. The `pool_modifier !== 0` branch and the empty-paths branch must both be exercised.

If Plan C reveals a real bug-shaped logic problem in `summarizeModifiers` or `dispatchRoll` during manual smoke, that's the trigger to add Vitest as a separate follow-up — but pre-installing it for two ~20-line functions doesn't earn its keep.

### §6.12 Files (Plan C)

| Action | File | Reason |
|---|---|---|
| Modify | `src-tauri/src/bridge/foundry/types.rs` | Add `roll_mode`, `pool_modifier` to `RollV5PoolInput` |
| Modify | `src-tauri/src/bridge/foundry/actions/game.rs` | Validation + envelope fields + new tests |
| Modify | `vtmtools-bridge/scripts/foundry-actions/game.js` | Branch on `pool_modifier` / `paths` for direct-Roll vs. RollFromDataset path; add `computeBasicDice` |
| Modify | `vtmtools-bridge/module.json` | Bump 0.3.0 → 0.4.0 (additive, protocol_version unchanged) |
| Modify | `src/lib/foundry-chat/api.ts` | Add `rollMode` + `poolModifier` to `RollV5PoolInput` |
| Create | `src/lib/gm-screen/path-providers.ts` | PathProvider type + V1 registry |
| Create | `src/lib/gm-screen/roll.ts` | `summarizeModifiers` + `dispatchRoll` |
| Create | `src/lib/components/gm-screen/RollDispatcherPopover.svelte` | The popover itself |
| Modify | `src/lib/components/gm-screen/CharacterRow.svelte` | Add 🎲 trigger + popover mount (Foundry-only) |

Total: **3 new TS modules, 1 new Svelte component, 6 modifications.** No new SQL migrations, no new Tauri commands.

---

## §7 Wire-protocol change summary

Only `RollV5PoolInput` / the `game.roll_v5_pool` envelope changes. Both changes are additive within protocol_version 1:

```json
{
  "type": "game.roll_v5_pool",
  "actor_id": "ObCGftjZjCvpPBdN",
  "value_paths": ["attributes.strength.value", "skills.brawl.value"],
  "difficulty": 4,
  "flavor": "Strength + Brawl",
  "advanced_dice": null,
  "selectors": [],
  "roll_mode": "gmroll",          // NEW (default "roll")
  "pool_modifier": 2              // NEW (default 0)
}
```

**Backward compatibility:**
- Old desktop ↔ new module (0.4.0): old desktop never sends the new fields; new module's executor sees `roll_mode === undefined` (defaults to "roll") and `pool_modifier === undefined` (defaults to 0, falls into RollFromDataset branch) — identical behavior to pre-this-spec.
- New desktop ↔ old module (0.3.0): old module's `rollV5Pool` ignores the unknown fields and uses the existing RollFromDataset path. The roll still happens; the GM's pool_modifier silently doesn't apply, and `roll_mode` falls through to default. **Graceful degradation.** A capability check on `Hello.capabilities` could harden this in a future iteration but is not required (matches the existing "no capability gating" posture from `2026-05-01-foundry-game-roll-helpers-design.md` §9).

Module version bump: 0.3.0 → 0.4.0 (semver minor: additive features).

---

## §8 Error handling

Follows ARCHITECTURE.md §7: Rust commands return `Result<T, String>` with module-stable prefixes; frontend catches in API wrappers and surfaces via toast.

| Scenario | Behavior | Module prefix |
|---|---|---|
| Plan B: name in `attribute.*` / `skill.*` with unknown key | `apply_canonical_field` returns Err | `character/set_field: unknown attribute '<key>'` / `unknown skill '<key>'` |
| Plan B: out-of-range value (>5) | `apply_canonical_field` returns Err | `character/set_field: 'attribute.strength' expects integer 0..=5, got 7` |
| Plan B: `target=Live + Roll20 + attribute.*/skill.*` | Existing fast-fail | `character/set_field: Roll20 live editing of canonical names not yet supported` |
| Plan C: invalid `roll_mode` | `build_roll_v5_pool` returns Err | `foundry/game.roll_v5_pool: invalid roll_mode: <value>` |
| Plan C: required provider has no selection | `dispatchRoll` throws | `gm-screen/roll: required provider '<id>' has no selection` |
| Plan C: option key not found in provider | `dispatchRoll` throws | `gm-screen/roll: provider '<id>' option '<key>' not found` |
| Plan C: non-Foundry character passed to `dispatchRoll` | Defensive throw | `gm-screen/roll: dispatchRoll requires a Foundry character` |
| Plan C: Foundry disconnected | `bridge::commands::send_to_source` returns Err | `foundry/game.roll_v5_pool: bridge not connected` (existing path) |
| Plan C: actor not found in Foundry | JS executor throws | `bridge://foundry/error` event → toast `actor not found: <id>` |

---

## §9 Anti-scope (what this spec MUST NOT touch)

- **Hunger / remorse / contested rolls** (issue #11). The popover does NOT bake these in. Default `advancedDice = null` lets WoD5e auto-derive hunger for vampires; that IS the entire interaction. Custom hunger/rouse/contested rolls are a separate future affordance.
- **Roll mirroring back into vtmtools** (issue #10 / `chat.*` umbrella). Rolls are fire-and-forget at the wire level; results live in Foundry chat only.
- **Roll log UI** (issue #12).
- **Roll20 live skills/attrs editing.** Returns the existing fast-fail error.
- **Roll20 popover** (Plan C only renders for Foundry characters).
- **Per-roll modifier override** (e.g., per-roll checkboxes to exclude an active card from this specific roll). The card's `isActive` IS the on/off; if the GM wants to exclude a card, they deactivate it. Future seam noted in §12.
- **`ModifierEffect.scope` semantics.** Stays a free-form display label.
- **Sheet edit UI for skills/attrs.** Plan B is logic-only per the user's explicit ask. A future stat-editor extension can wire +/- buttons, but it's out of scope here.
- **Editing skills via `health.max` / `willpower.max`** (still deferred per `2026-05-02-character-set-field-router-design.md` §2.5).

---

## §10 Plan packaging order

| Plan | Depends on | Parallelizable with |
|---|---|---|
| A | nothing | (must run first) |
| B | Plan A (imports `AttributeName` / `SkillName` literal types from `canonical-names.ts`) | C (after A lands) |
| C | Plan A (imports `FOUNDRY_*_NAMES` from `canonical-names.ts`) | B (after A lands) |

**File overlap check** (verified against §4.4, §5.8, §6.12):
- Only `src/types.ts` is touched by both Plan B and a hypothetical other plan, but Plan C does NOT touch it. No two plans modify the same file.
- The single shared dependency is the new `src/lib/foundry/canonical-names.ts` module created by Plan A. Plans B and C only **read** from it.

**Recommended sequence if executed serially** (one implementer agent, single session):
1. Plan A (smallest, validates the skills surface AND lays the foundation module; ~30 min).
2. Plan B (closes issue #28 Foundry portion; pure backend; ~1.5 hr).
3. Plan C (the meaty one; ~3-4 hr).

**Recommended sequence if executed in parallel** (per `superpowers:using-git-worktrees`): land Plan A first (sequential gate), then Plans B and C in two worktrees. Final-merge order between B and C doesn't matter (no file conflicts). Each plan ends in `./scripts/verify.sh` per CLAUDE.md.

---

## §11 Manual smoke tests (run once at the end)

Each plan ships `./scripts/verify.sh` per the CLAUDE.md hard rule. Manual smokes are done once at the end of all three plans, not per-task.

**Plan A:**
1. Open Campaign tool with a connected Foundry world.
2. Expand a Foundry character's "Core attributes" collapsible — verify all 9 attributes display the correct sheet values (NOT zero — this is the bug-fix verification).
3. Expand the same character's "Skills" collapsible.
4. Verify all 27 skills appear with correct values (cross-reference one or two against the actor's sheet in Foundry).
5. Verify a Roll20 character has no Skills collapsible toggle button in the footer.
6. Verify a Roll20 character's "Core attributes" still display the correct Roll20 values (regression check on the source-aware helper).

**Plan B:**
1. From a Foundry-connected dev session, open the dev console.
2. Call `await window.__TAURI_INTERNALS__.invoke('character_set_field', { target: 'live', source: 'foundry', sourceId: '<id>', name: 'attribute.strength', value: 4 })`.
3. Verify the actor's Strength attribute updates in Foundry.
4. Repeat for `'skill.brawl'`.
5. Call with `value: 6` → expect Err matching `'attribute.strength' expects integer 0..=5, got 6`.
6. Call with `name: 'attribute.foo'` → expect Err matching `unknown attribute 'foo'`.

**Plan C:**
1. Open GM Screen with a connected Foundry world; ensure a character has at least one active modifier card with a `pool` effect (e.g. `+2 pool, scope: Frenzy`).
2. Click the 🎲 Roll button on the character.
3. Pick attribute=Strength, skill=Brawl, difficulty=4. Verify the popover shows "Pool: 5+2=7, Diff: 4+0=4" (or actual values for the actor).
4. Submit. Verify a Strength + Brawl roll appears in Foundry chat with the +2 pool dice rolled and difficulty 4.
5. Open the popover again. Set roll_mode = "GM only", custom flavor = "Hidden roll". Submit. Verify the chat card appears only to the GM and shows "Hidden roll" instead of "Strength + Brawl".
6. Set baseDifficulty = -10 (with no diff modifiers). Submit. Verify the wire-side `difficulty` is clamped to `0` (not negative — Foundry would reject).
7. Verify the 🎲 button does NOT appear on Roll20 character rows.

---

## §12 Future seams

| Future feature | How this spec accommodates it |
|---|---|
| **Discipline picker in popover** | Add `DISCIPLINE_PROVIDER` to `DEFAULT_PROVIDERS`. No change to `RollDispatcherPopover.svelte` or `dispatchRoll`. |
| **Merit-as-bonus-dice picker** | Add `MERIT_BONUS_PROVIDER` (filters items by `system.featuretype === 'merit'`, surfaces those with a `bonuses[].value > 0`). Same registry pattern. |
| **Werewolf renown / rage in popover** | Add `RENOWN_PROVIDER` / `RAGE_PROVIDER`. Splat-aware rendering: provider's `getOptions` returns empty for non-werewolf, and the popover skips empty providers. |
| **Roll mirroring (issue #10)** | Independent — adds a `chat.*` inbound umbrella. The popover doesn't change; results land in a future "roll log" UI. |
| **Per-roll modifier override** | Add `selectedModifierIds: Set<number>` state to the popover; default = all-active; render checkboxes next to the modifier-sum display. `summarizeModifiers` extends to take an optional id-filter. |
| **Roll20 popover** | When Roll20 grows roll-dispatch capability, the `{#if character.source === 'foundry'}` guard becomes a per-source dispatch. `dispatchRoll` extends to route by source; `RollV5PoolInput` stays Foundry-only. |
| **Stat editor UI for skills/attrs** | Plan B's IPC is already in place; UI plan is purely additive (e.g., +/- buttons on the new Skills section in Campaign manager). |

---

## §13 Open questions

### Resolved during this brainstorm

- ✅ Spec packaging — one umbrella, three plans (§2.1).
- ✅ Canonical-name shape — namespaced (`attribute.*` / `skill.*`) (§2.2).
- ✅ Roll20 parity for Plan B — Foundry-only (§2.3).
- ✅ Popover base mode — stat-based (attr + skill), modifiers add on top (§2.4).
- ✅ Path-provider extensibility — registry pattern, V1 ships attr+skill (§2.5).
- ✅ Privacy controls — custom flavor + roll_mode, baked in (§2.6).
- ✅ pool_modifier mechanism — additive wire field, JS executor switches to direct-Roll API (§2.7, §6.4).
- ✅ Roll20 character rendering in popover — button hidden (§6.6).
- ✅ TS/Rust drift policy — manual checklist matching `BridgeCharacter` mirror convention (§5.6).

### Outstanding (deferred / non-blocking)

- **Popover state persistence.** Should reopening the popover for the same character restore the last attribute/skill selection, or always reset? Current spec: reset on close. Non-blocking; can be revised post-implementation if it feels clunky.
- **Capability gating for `pool_modifier`.** A capability-aware desktop could refuse to send `pool_modifier` to a 0.3.0 module. Current spec: rely on graceful degradation (§7). Non-blocking; future hardening.
- **Auto-pick "active discipline" as a future provider.** WoD5e exposes `system.selectedDiscipline`. When `DISCIPLINE_PROVIDER` is added later, it could pre-select the actor's currently-selected discipline. Out of scope here.

### Implementer verification (not blocking spec sign-off)

- **`RollFromDataset({ dataset: { rollMode } })` propagation.** `WOD5E.api.Roll` is documented to accept `rollMode` (`docs/reference/foundry-vtm5e-rolls.md:38`). `RollFromDataset` calls `Roll` internally but its dataset-shape contract for `rollMode` is not documented. The Plan C implementer must verify during the Plan C task that adds the JS executor branching: if `dataset.rollMode` is silently ignored on the non-pool-modifier path, fall back to either (a) always using direct `Roll` (preferred — simpler invariant: "popover always uses direct Roll, never RollFromDataset"), or (b) constructing a manual `ChatMessage.create({ rollMode, ... })` wrapper around the Roll output.

---

## §14 Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

| Plan | What `verify.sh` proves |
|---|---|
| A | TS type check; no broken imports of `FOUNDRY_SKILL_NAMES`/`FOUNDRY_ATTRIBUTE_NAMES`; frontend build succeeds. |
| B | Coverage assertions (every name has Foundry path + applies); per-target router branches; TS template-literal type compiles. |
| C | Rust tests for new `RollV5PoolInput` fields + envelope shape; `npm run check` for `path-providers.ts` / `roll.ts` / `RollDispatcherPopover.svelte`; frontend build with new component. (TS helpers are validated via manual smoke per §11 — no Vitest infra in project.) |

Manual smokes (§11) are done once at the end, not per-task.

---

## §15 Pointers

- `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md` §2.5 (deferred Phase 2.5 — Plan B's source of authority), §2.7 (Roll20 read-only policy), §2.8 (Roll20 canonical-name fast-fail).
- `docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md` (RollV5PoolInput baseline; §9 backward-compat posture).
- `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md` (game.* umbrella; wire-protocol convention).
- `docs/superpowers/specs/2026-05-03-gm-screen-design.md` (existing modifier model; Plan C of GM Screen for the push-to-Foundry counterpart).
- `docs/reference/foundry-vtm5e-paths.md` §"Shape findings beyond the canonical fields" (canonical 9 attrs + 27 skills list, verified against WoD5e v5.3.17).
- `docs/reference/foundry-vtm5e-actor-sample.json` (ground-truth wire shape for `system.attributes.*` and `system.skills.*`).
- `docs/reference/foundry-vtm5e-rolls.md` (WoD5e roll API: `RollFromDataset`, `Roll`, `getAdvancedDice`).
- `ARCHITECTURE.md` §4 (Tauri IPC + bridge protocol), §7 (error handling), §10 (testing convention), §11 (plan conventions).
