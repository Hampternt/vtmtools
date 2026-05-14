# Foundry skills/attrs — Plan B: `character_set_field` skills/attrs extension (Foundry only)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plan B tasks are committed, run a SINGLE `code-review:code-review` against the full Plan B branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** Extend `character_set_field` to accept namespaced canonical names `attribute.<key>` (9 keys) and `skill.<key>` (27 keys), routing both saved and live writes for Foundry sources. Closes the Foundry portion of issue #28 (Phase 2.5). Logic-only — no UI changes.

**Architecture:** Backend extension to the existing `shared/canonical_fields.rs` namespace module. New runtime arrays `ATTRIBUTE_NAMES` / `SKILL_NAMES` (mirroring Plan A's `FOUNDRY_*_NAMES` TS arrays). New `is_allowed_name(&str) -> bool` replaces the existing `ALLOWED_NAMES.contains(...)` check at the router. Apply layer grows two prefix-dispatch arms (`attribute.*` / `skill.*`) that walk into `CanonicalCharacter.raw` via JSON Pointer using a new `set_raw_u8` helper. Foundry path translation extends with format-string arms; signature changes from `Option<&'static str>` to `Option<String>`. Roll20 stays a stub (the existing fast-fail covers the new names). TS mirror extends `CanonicalFieldName` with template-literal types backed by Plan A's `AttributeName` / `SkillName` literal unions.

**Tech Stack:** Rust + serde_json (existing), Tauri 2 IPC, TypeScript template literal types.

**Spec:** `docs/superpowers/specs/2026-05-09-foundry-skills-attrs-and-roll-dispatcher-design.md` §5 (Plan B).
**Architecture reference:** `ARCHITECTURE.md` §4 (IPC + typed wrappers), §7 (error handling prefixes), §10 (testing convention).

**Spec defaults adopted:**
- Manual-checklist drift policy between TS and Rust name lists (spec §5.6 — matches the existing `BridgeCharacter` mirror convention).
- Range check `0..=5` for both attributes and skills (spec §5.2; matches WoD5e dot-rating ceiling).

**Depends on:** Plan A merged. Plan A creates `src/lib/foundry/canonical-names.ts` which Plan B imports for the literal types.

---

## File structure

### Files modified

| Path | Change |
|---|---|
| `src-tauri/src/shared/canonical_fields.rs` | (a) Add `FLAT_NAMES`, `ATTRIBUTE_NAMES`, `SKILL_NAMES` `pub const` arrays. (b) Add `is_allowed_name(&str) -> bool`. (c) Add `apply_attribute` / `apply_skill` / `set_raw_u8` helpers. (d) Extend `apply_canonical_field` switch with `attribute.*` / `skill.*` prefix arms. (e) Change `canonical_to_foundry_path` return type to `Option<String>` and add new prefix arms. (f) Extend the existing `cargo test` coverage assertions. |
| `src-tauri/src/bridge/foundry/mod.rs` | Update `canonical_to_path` consumer for the new `Option<String>` signature (remove `.to_string()` on the existing path). |
| `src-tauri/src/tools/character.rs` | Replace `ALLOWED_NAMES.contains(&name.as_str())` with `is_allowed_name(&name)`. Add new tests for the namespaced names. |
| `src-tauri/src/db/saved_character.rs` | Add new tests for `patch_saved_field` with `attribute.*` and `skill.*` names. No source changes (`db_patch_field` already calls `apply_canonical_field`). |
| `src/types.ts` | Re-export `AttributeName` / `SkillName` from `src/lib/foundry/canonical-names.ts`. Extend `CanonicalFieldName` with `` `attribute.${AttributeName}` `` and `` `skill.${SkillName}` `` template-literal arms. |

### Files NOT touched in Plan B

- `src/tools/Campaign.svelte` (Plan A territory — already done)
- `src/lib/components/gm-screen/**` (Plan C territory)
- `src-tauri/src/bridge/foundry/actions/**` (Plan C touches `actions/game.rs`; Plan B doesn't)
- `src-tauri/src/bridge/foundry/types.rs` (Plan C territory)
- `src/lib/character/api.ts` (already exposes the typed wrapper; the new template-literal types flow through automatically since `name: CanonicalFieldName`)
- Migrations (no schema change — only the `canonical_json` blob's content shape changes)

### Tauri command surface

Unchanged. `character_set_field` and `patch_saved_field` already exist; this plan only widens the accepted `name` parameter values.

---

## Task B1: `canonical_fields.rs` — name arrays + `is_allowed_name`

**Goal:** Land the three `pub const` arrays and the `is_allowed_name` predicate. The router will switch to using it in Task B4.

**Files:**
- Modify: `src-tauri/src/shared/canonical_fields.rs`

**Anti-scope:** Do not touch `apply_canonical_field` or `canonical_to_foundry_path` in this task. Do not touch `tools/character.rs` (the consumer switch is Task B4). The existing `ALLOWED_NAMES` const stays as-is — `is_allowed_name` is the new check, not a replacement of the const.

**Depends on:** nothing in Plan B.

**Invariants cited:** ARCH §10 (Rust tests inline `#[cfg(test)] mod tests`).

**Tests required:** YES — `is_allowed_name` is real predicate logic and worth pinning before downstream tasks rely on it.

- [x] **Step 1: Write the failing tests**

Open `src-tauri/src/shared/canonical_fields.rs`. Locate the existing `#[cfg(test)] mod tests { ... }` block at the bottom of the file. Add these new tests inside it (after the existing tests):

```rust
    #[test]
    fn is_allowed_name_accepts_legacy_flat_names() {
        for n in FLAT_NAMES {
            assert!(is_allowed_name(n), "should accept legacy flat name '{n}'");
        }
    }

    #[test]
    fn is_allowed_name_accepts_namespaced_attributes() {
        for n in ATTRIBUTE_NAMES {
            let full = format!("attribute.{n}");
            assert!(is_allowed_name(&full), "should accept '{full}'");
        }
    }

    #[test]
    fn is_allowed_name_accepts_namespaced_skills() {
        for n in SKILL_NAMES {
            let full = format!("skill.{n}");
            assert!(is_allowed_name(&full), "should accept '{full}'");
        }
    }

    #[test]
    fn is_allowed_name_rejects_unknown_attribute_key() {
        assert!(!is_allowed_name("attribute.foo"));
        assert!(!is_allowed_name("attribute."));
    }

    #[test]
    fn is_allowed_name_rejects_unknown_skill_key() {
        assert!(!is_allowed_name("skill.bar"));
        assert!(!is_allowed_name("skill."));
    }

    #[test]
    fn is_allowed_name_rejects_unknown_flat_name() {
        assert!(!is_allowed_name("xyzzy"));
        assert!(!is_allowed_name(""));
    }

    #[test]
    fn flat_names_match_legacy_allowed_names() {
        // FLAT_NAMES must equal the existing ALLOWED_NAMES (legacy 8 names) —
        // this test pins the equivalence so future-you doesn't accidentally
        // diverge them.
        assert_eq!(FLAT_NAMES, ALLOWED_NAMES);
    }
```

- [x] **Step 2: Run the new tests — verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields::tests::is_allowed_name -- --nocapture`
Expected: FAIL — `cannot find function 'is_allowed_name'`, `cannot find value 'FLAT_NAMES'`, etc.

- [x] **Step 3: Add the three name arrays**

In `src-tauri/src/shared/canonical_fields.rs`, immediately after the existing `pub const ALLOWED_NAMES: &[&str] = &[ ... ];` declaration (around line 19), add:

```rust
/// Legacy 8 flat canonical names — duplicates ALLOWED_NAMES under a clearer
/// name so the three arrays (FLAT_NAMES + ATTRIBUTE_NAMES + SKILL_NAMES) form
/// the full v2 surface. ALLOWED_NAMES is kept for backward compatibility with
/// any existing callers that iterate it.
pub const FLAT_NAMES: &[&str] = ALLOWED_NAMES;

/// WoD5e v5.3.17 attribute keys (system.attributes.<key>.value).
/// Mirrors src/lib/foundry/canonical-names.ts::FOUNDRY_ATTRIBUTE_NAMES.
/// When changing this list, update the TS array in the same commit.
pub const ATTRIBUTE_NAMES: &[&str] = &[
    "charisma",
    "composure",
    "dexterity",
    "intelligence",
    "manipulation",
    "resolve",
    "stamina",
    "strength",
    "wits",
];

/// WoD5e v5.3.17 skill keys (system.skills.<key>.value).
/// Mirrors src/lib/foundry/canonical-names.ts::FOUNDRY_SKILL_NAMES.
pub const SKILL_NAMES: &[&str] = &[
    "academics",
    "animalken",
    "athletics",
    "awareness",
    "brawl",
    "craft",
    "drive",
    "etiquette",
    "finance",
    "firearms",
    "insight",
    "intimidation",
    "investigation",
    "larceny",
    "leadership",
    "medicine",
    "melee",
    "occult",
    "performance",
    "persuasion",
    "politics",
    "science",
    "stealth",
    "streetwise",
    "subterfuge",
    "survival",
    "technology",
];
```

- [x] **Step 4: Add the `is_allowed_name` function**

Immediately after the three constants (and before `pub fn apply_canonical_field`), add:

```rust
/// Returns true if `name` is in the v2 canonical-name surface:
///   - one of the legacy 8 flat names (FLAT_NAMES), OR
///   - `attribute.<key>` where `<key>` is in ATTRIBUTE_NAMES, OR
///   - `skill.<key>` where `<key>` is in SKILL_NAMES.
///
/// Use this at the router instead of `ALLOWED_NAMES.contains(...)` — the
/// const can't grow inline (no const-fn array concat in stable Rust).
pub fn is_allowed_name(name: &str) -> bool {
    if FLAT_NAMES.contains(&name) {
        return true;
    }
    if let Some(rest) = name.strip_prefix("attribute.") {
        return ATTRIBUTE_NAMES.contains(&rest);
    }
    if let Some(rest) = name.strip_prefix("skill.") {
        return SKILL_NAMES.contains(&rest);
    }
    false
}
```

- [x] **Step 5: Run the new tests — verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields::tests`
Expected: PASS — all existing tests AND the 7 new ones.

- [x] **Step 6: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS — full repo gate.

- [x] **Step 7: Commit**

```bash
git add src-tauri/src/shared/canonical_fields.rs
git commit -m "feat(canonical_fields): add ATTRIBUTE_NAMES / SKILL_NAMES + is_allowed_name

Foundation for Plan B's namespaced canonical names ('attribute.strength',
'skill.brawl', etc.). Three pub const arrays mirror the TS arrays in
src/lib/foundry/canonical-names.ts (manual-checklist drift policy).

Adds is_allowed_name(&str) which probes all three arrays + handles the
'attribute.' / 'skill.' prefixes. Router (tools/character.rs) will switch
from ALLOWED_NAMES.contains() to this in Task B4.

ALLOWED_NAMES preserved as a public alias for backward compat."
```

---

## Task B2: `canonical_fields.rs` — apply layer extension

**Goal:** Add `apply_attribute`, `apply_skill`, `set_raw_u8`, and dispatch from `apply_canonical_field`. After this task, calling `apply_canonical_field(c, "attribute.strength", &json!(3))` on a Foundry character mutates `c.raw` at `system.attributes.strength.value`.

**Files:**
- Modify: `src-tauri/src/shared/canonical_fields.rs`

**Anti-scope:** Do not touch `canonical_to_foundry_path` yet (Task B3). Do not touch the router. Do not change `apply_track_field` or any existing flat-name arm.

**Depends on:** Task B1 (uses `ATTRIBUTE_NAMES` / `SKILL_NAMES`).

**Invariants cited:** ARCH §7 (error prefix `character/set_field:`).

**Tests required:** YES — path-walking with intermediate-object creation is real logic and the most fragile new code in Plan B.

- [x] **Step 1: Write the failing tests**

In `src-tauri/src/shared/canonical_fields.rs::tests`, add after the existing tests:

```rust
    #[test]
    fn apply_attribute_strength_writes_into_raw() {
        let mut c = sample();
        apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!(3))
            .expect("happy path");
        let v = c
            .raw
            .pointer("/system/attributes/strength/value")
            .expect("raw pointer exists");
        assert_eq!(v, &serde_json::json!(3));
    }

    #[test]
    fn apply_skill_brawl_writes_into_raw() {
        let mut c = sample();
        apply_canonical_field(&mut c, "skill.brawl", &serde_json::json!(2))
            .expect("happy path");
        let v = c
            .raw
            .pointer("/system/skills/brawl/value")
            .expect("raw pointer exists");
        assert_eq!(v, &serde_json::json!(2));
    }

    #[test]
    fn apply_attribute_unknown_key_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "attribute.foo", &serde_json::json!(1))
            .unwrap_err();
        assert!(
            err.contains("unknown attribute 'foo'"),
            "got: {err}"
        );
    }

    #[test]
    fn apply_skill_unknown_key_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "skill.bar", &serde_json::json!(1))
            .unwrap_err();
        assert!(err.contains("unknown skill 'bar'"), "got: {err}");
    }

    #[test]
    fn apply_attribute_out_of_range_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!(6))
            .unwrap_err();
        assert!(err.contains("expects integer 0..=5"), "got: {err}");
    }

    #[test]
    fn apply_attribute_wrong_type_errors() {
        let mut c = sample();
        let err = apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!("3"))
            .unwrap_err();
        assert!(err.contains("got string"), "got: {err}");
    }

    #[test]
    fn apply_attribute_overwrites_existing_raw_value() {
        let mut c = sample();
        c.raw = serde_json::json!({
            "system": { "attributes": { "strength": { "value": 1 } } }
        });
        apply_canonical_field(&mut c, "attribute.strength", &serde_json::json!(4))
            .expect("happy path");
        assert_eq!(
            c.raw.pointer("/system/attributes/strength/value"),
            Some(&serde_json::json!(4))
        );
    }

    #[test]
    fn apply_skill_creates_intermediate_objects_when_missing() {
        // sample() raw is `{}` — fully missing system/skills/<key>/value path.
        // set_raw_u8 must create intermediate objects without erroring.
        let mut c = sample();
        apply_canonical_field(&mut c, "skill.occult", &serde_json::json!(5))
            .expect("must create intermediate objects");
        assert_eq!(
            c.raw.pointer("/system/skills/occult/value"),
            Some(&serde_json::json!(5))
        );
    }
```

- [x] **Step 2: Run the new tests — verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields::tests::apply_attribute -- --nocapture`
Expected: FAIL — the prefix arms don't exist yet, so `apply_canonical_field` returns `unknown field 'attribute.strength'`.

- [x] **Step 3: Add the `set_raw_u8` helper**

In `src-tauri/src/shared/canonical_fields.rs`, immediately after the existing `fn type_label(...)` (around line 101, before `pub fn canonical_to_foundry_path`), add:

```rust
/// Walk `raw` by JSON pointer-path segments and overwrite the leaf with `n`.
/// Creates intermediate objects as needed so a saved-side write succeeds even
/// for actors whose raw blob hasn't seen this skill/attribute before.
///
/// `pointer` MUST start with '/' and use '/' as the segment delimiter
/// (RFC 6901 JSON Pointer syntax — same as serde_json's Value::pointer).
///
/// Returns Err if a non-leaf segment exists but is not a JSON object
/// (e.g. trying to walk into a string at /system/attributes when the actor's
/// raw has system.attributes as "broken" — defensive; should never happen
/// with valid Foundry payloads).
fn set_raw_u8(raw: &mut Value, pointer: &str, n: u8) -> Result<(), String> {
    if !pointer.starts_with('/') {
        return Err(format!(
            "character/set_field: invalid pointer '{pointer}' (must start with '/')"
        ));
    }
    let segments: Vec<&str> = pointer[1..].split('/').collect();
    if segments.is_empty() {
        return Err("character/set_field: empty pointer".into());
    }

    // Ensure root is an object.
    if !raw.is_object() {
        *raw = Value::Object(serde_json::Map::new());
    }

    // Walk all but the last segment, creating empty objects as we go.
    let mut cur = raw;
    for seg in &segments[..segments.len() - 1] {
        let obj = cur.as_object_mut().ok_or_else(|| {
            format!("character/set_field: pointer '{pointer}' walks into non-object")
        })?;
        if !obj.contains_key(*seg) {
            obj.insert(seg.to_string(), Value::Object(serde_json::Map::new()));
        }
        // Re-borrow for the next iteration. Unwrap is safe — we just inserted
        // (or it already existed); object_mut may still fail if existing was
        // not an object, which is the defensive Err above on next iteration.
        cur = obj.get_mut(*seg).unwrap();
    }

    // Set the leaf.
    let leaf_obj = cur.as_object_mut().ok_or_else(|| {
        format!("character/set_field: pointer '{pointer}' walks into non-object at leaf parent")
    })?;
    leaf_obj.insert(segments.last().unwrap().to_string(), Value::from(n as u64));
    Ok(())
}
```

- [x] **Step 4: Add `apply_attribute` and `apply_skill` helpers**

Immediately after `set_raw_u8`, add:

```rust
/// Apply `attribute.<key>` write — validates `key` against ATTRIBUTE_NAMES,
/// range-checks the value 0..=5 (WoD5e dot rating), then writes via JSON
/// pointer to /system/attributes/<key>/value.
fn apply_attribute(c: &mut CanonicalCharacter, key: &str, value: &Value) -> Result<(), String> {
    if !ATTRIBUTE_NAMES.contains(&key) {
        return Err(format!("character/set_field: unknown attribute '{key}'"));
    }
    let display_name = format!("attribute.{key}");
    let n = expect_u8_in_range(value, &display_name, 0, 5)?;
    let pointer = format!("/system/attributes/{key}/value");
    set_raw_u8(&mut c.raw, &pointer, n)
}

/// Apply `skill.<key>` write — same shape as apply_attribute but for
/// /system/skills/<key>/value.
fn apply_skill(c: &mut CanonicalCharacter, key: &str, value: &Value) -> Result<(), String> {
    if !SKILL_NAMES.contains(&key) {
        return Err(format!("character/set_field: unknown skill '{key}'"));
    }
    let display_name = format!("skill.{key}");
    let n = expect_u8_in_range(value, &display_name, 0, 5)?;
    let pointer = format!("/system/skills/{key}/value");
    set_raw_u8(&mut c.raw, &pointer, n)
}
```

- [x] **Step 5: Extend `apply_canonical_field` to dispatch the new arms**

Replace the existing `match name { ... }` block in `apply_canonical_field` (lines 28-54) with:

```rust
    // Existing 8 flat-name arms.
    match name {
        "hunger" => {
            let n = expect_u8_in_range(value, name, 0, 5)?;
            c.hunger = Some(n);
            return Ok(());
        }
        "humanity" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity = Some(n);
            return Ok(());
        }
        "humanity_stains" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity_stains = Some(n);
            return Ok(());
        }
        "blood_potency" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.blood_potency = Some(n);
            return Ok(());
        }
        "health_superficial" | "health_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.health, name, n);
            return Ok(());
        }
        "willpower_superficial" | "willpower_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.willpower, name, n);
            return Ok(());
        }
        _ => {}
    }

    // New namespaced arms: attribute.<key>, skill.<key>.
    if let Some(key) = name.strip_prefix("attribute.") {
        return apply_attribute(c, key, value);
    }
    if let Some(key) = name.strip_prefix("skill.") {
        return apply_skill(c, key, value);
    }

    Err(format!("character/set_field: unknown field '{name}'"))
```

(The structural change is replacing the `match` exhaustive arm `other => return Err(...)` with explicit `return Ok(())` per arm + a fall-through to the prefix dispatch + an `unknown field` Err at the bottom. The `Ok(())` at the end of the function body is no longer reached and is removed by this replacement; ensure the function body ends cleanly.)

- [x] **Step 6: Run the tests — verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields::tests`
Expected: PASS — all existing tests AND the 8 new ones from Step 1.

- [x] **Step 7: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 8: Commit**

```bash
git add src-tauri/src/shared/canonical_fields.rs
git commit -m "feat(canonical_fields): apply layer for attribute.* / skill.* names

Adds apply_attribute, apply_skill, and set_raw_u8 (JSON pointer walker
that creates intermediate objects as needed). Extends apply_canonical_field
to dispatch the new prefix arms after the existing flat-name match.

Range-checks 0..=5 for both prefixes (WoD5e dot ceiling). Errors carry
the canonical character/set_field: prefix per ARCH §7."
```

---

## Task B3: `canonical_to_foundry_path` signature change + new arms

**Goal:** Atomic commit changing `canonical_to_foundry_path`'s return type from `Option<&'static str>` to `Option<String>`, adding the `attribute.*` / `skill.*` arms, and updating the single consumer in `bridge/foundry/mod.rs::canonical_to_path`. Coverage assertions extended.

**Files:**
- Modify: `src-tauri/src/shared/canonical_fields.rs`
- Modify: `src-tauri/src/bridge/foundry/mod.rs`

**Anti-scope:** Do not touch the router (Task B4). Do not introduce new helpers — the prefix arms are simple `format!` calls.

**Depends on:** Task B1 (uses `ATTRIBUTE_NAMES` / `SKILL_NAMES`), Task B2 (the `apply_canonical_field` dispatch is the contract that the path mapping must mirror).

**Invariants cited:** spec §5.6 (drift policy — TS/Rust mirror is manual checklist).

**Tests required:** YES — coverage assertions are the structural invariant that prevents future drift between apply / path / TS surfaces.

- [x] **Step 1: Write the new and extended tests**

In `src-tauri/src/shared/canonical_fields.rs::tests`, add (and replace the existing `every_allowed_name_*` tests if they conflict with the new wider versions):

```rust
    #[test]
    fn canonical_to_foundry_path_for_attribute_strength() {
        assert_eq!(
            canonical_to_foundry_path("attribute.strength").as_deref(),
            Some("system.attributes.strength.value")
        );
    }

    #[test]
    fn canonical_to_foundry_path_for_skill_brawl() {
        assert_eq!(
            canonical_to_foundry_path("skill.brawl").as_deref(),
            Some("system.skills.brawl.value")
        );
    }

    #[test]
    fn canonical_to_foundry_path_for_unknown_attribute_returns_none() {
        assert!(canonical_to_foundry_path("attribute.foo").is_none());
    }

    #[test]
    fn canonical_to_foundry_path_for_unknown_skill_returns_none() {
        assert!(canonical_to_foundry_path("skill.bar").is_none());
    }

    #[test]
    fn every_attribute_name_has_foundry_path() {
        for n in ATTRIBUTE_NAMES {
            let full = format!("attribute.{n}");
            assert!(
                canonical_to_foundry_path(&full).is_some(),
                "missing Foundry path for '{full}'"
            );
        }
    }

    #[test]
    fn every_skill_name_has_foundry_path() {
        for n in SKILL_NAMES {
            let full = format!("skill.{n}");
            assert!(
                canonical_to_foundry_path(&full).is_some(),
                "missing Foundry path for '{full}'"
            );
        }
    }

    #[test]
    fn every_attribute_name_applies_via_apply_canonical_field() {
        for n in ATTRIBUTE_NAMES {
            let full = format!("attribute.{n}");
            let mut c = sample();
            let res = apply_canonical_field(&mut c, &full, &serde_json::json!(0));
            assert!(
                res.is_ok(),
                "apply_canonical_field rejected '{full}': {:?}",
                res.err()
            );
        }
    }

    #[test]
    fn every_skill_name_applies_via_apply_canonical_field() {
        for n in SKILL_NAMES {
            let full = format!("skill.{n}");
            let mut c = sample();
            let res = apply_canonical_field(&mut c, &full, &serde_json::json!(0));
            assert!(
                res.is_ok(),
                "apply_canonical_field rejected '{full}': {:?}",
                res.err()
            );
        }
    }
```

The existing `every_allowed_name_has_foundry_path` and `every_allowed_name_applies_via_apply_canonical_field` tests still pass against the legacy 8 names — keep them as-is.

- [x] **Step 2: Run the new tests — verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields::tests::canonical_to_foundry_path_for -- --nocapture`
Expected: FAIL — `canonical_to_foundry_path` returns `Option<&'static str>` (existing signature) and the test calls `.as_deref()` which won't compile against `Option<&str>` directly. Both the signature change and the new arms are needed.

- [x] **Step 3: Change the signature and add the new arms**

Replace the existing `pub fn canonical_to_foundry_path(...)` (lines 105-117) with:

```rust
/// Foundry system-path mapping. Returns the dot-path for any name in the v2
/// canonical-name surface (FLAT_NAMES + ATTRIBUTE_NAMES + SKILL_NAMES).
///
/// Signature: returns `Option<String>` (not `Option<&'static str>`) because
/// the namespaced arms format-construct paths from the key. Legacy 8 arms
/// allocate via `.to_string()` for uniformity — overhead is one allocation
/// per IPC call; not measurable.
pub fn canonical_to_foundry_path(name: &str) -> Option<String> {
    // Legacy 8 flat names.
    let flat = match name {
        "hunger" => Some("system.hunger.value"),
        "humanity" => Some("system.humanity.value"),
        "humanity_stains" => Some("system.humanity.stains"),
        "blood_potency" => Some("system.blood.potency"),
        "health_superficial" => Some("system.health.superficial"),
        "health_aggravated" => Some("system.health.aggravated"),
        "willpower_superficial" => Some("system.willpower.superficial"),
        "willpower_aggravated" => Some("system.willpower.aggravated"),
        _ => None,
    };
    if let Some(s) = flat {
        return Some(s.to_string());
    }

    // Namespaced arms.
    if let Some(key) = name.strip_prefix("attribute.") {
        if !ATTRIBUTE_NAMES.contains(&key) {
            return None;
        }
        return Some(format!("system.attributes.{key}.value"));
    }
    if let Some(key) = name.strip_prefix("skill.") {
        if !SKILL_NAMES.contains(&key) {
            return None;
        }
        return Some(format!("system.skills.{key}.value"));
    }

    None
}
```

- [x] **Step 4: Update the consumer in `bridge/foundry/mod.rs`**

Open `src-tauri/src/bridge/foundry/mod.rs`. Locate `canonical_to_path` (around line 56). Replace:

```rust
fn canonical_to_path(name: &str) -> String {
    if let Some(p) = crate::shared::canonical_fields::canonical_to_foundry_path(name) {
        return p.to_string();
    }
    if name.starts_with("system.") {
        return name.to_string();
    }
    name.to_string()
}
```

with:

```rust
fn canonical_to_path(name: &str) -> String {
    if let Some(p) = crate::shared::canonical_fields::canonical_to_foundry_path(name) {
        return p;
    }
    if name.starts_with("system.") {
        return name.to_string();
    }
    name.to_string()
}
```

(The change is removing `.to_string()` after `return p` — `p` is already an owned `String` now.)

- [x] **Step 5: Run tests — verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::canonical_fields::tests`
Expected: PASS — existing 8 + new attribute/skill coverage assertions all green.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS — no compile errors from the signature change anywhere in the crate.

- [x] **Step 6: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS — full repo gate (cargo + npm check + frontend build).

- [x] **Step 7: Commit**

```bash
git add src-tauri/src/shared/canonical_fields.rs src-tauri/src/bridge/foundry/mod.rs
git commit -m "feat(canonical_fields): canonical_to_foundry_path covers attribute/skill

Changes signature from Option<&'static str> to Option<String> to support
the namespaced format-constructed paths. Adds prefix arms:
  attribute.<key> -> system.attributes.<key>.value
  skill.<key>     -> system.skills.<key>.value
Both arms validate <key> against ATTRIBUTE_NAMES / SKILL_NAMES.

Coverage assertions extended: every name in the v2 surface has a Foundry
path AND can be applied via apply_canonical_field. Drift between the apply
layer and the path layer is now a test failure, not a runtime mystery.

bridge/foundry/mod.rs::canonical_to_path consumer trivially adapted
(removes one .to_string() call now that the return is already owned)."
```

---

## Task B4: Switch router to `is_allowed_name` + add new behavior tests

**Goal:** Replace the `ALLOWED_NAMES.contains(&name.as_str())` check in `tools/character.rs::do_set_field` with `is_allowed_name(&name)` so the router accepts the new namespaced names. Add new tests covering the router's behavior with attribute/skill names.

**Files:**
- Modify: `src-tauri/src/tools/character.rs`

**Anti-scope:** Do not change the existing router behavior for legacy 8 names. Do not change the Roll20 fast-fail logic.

**Depends on:** Task B1 (`is_allowed_name`), Task B3 (the path translation is what the live-side write actually needs).

**Invariants cited:** spec §5.4 (Roll20 fast-fail unchanged), spec §8 error table (router prefixes).

**Tests required:** YES — the router-level behavior for the new names must be pinned (live + Roll20 + Both).

- [x] **Step 1: Write the failing tests**

Open `src-tauri/src/tools/character.rs`. Locate the existing `#[cfg(test)] mod tests { ... }` block. The existing test setup uses these helpers (verified at `character.rs:357-394`):
- `fresh_pool() -> SqlitePool` — in-memory SQLite with migrations applied.
- `make_bridge_state(connected: bool) -> (Arc<BridgeState>, Option<Receiver<String>>)` — stub bridge state with a `StubFoundrySource`. The `Receiver` is the outbound channel; bind it as `_rx` to keep the channel open OR `mut rx` to receive what the router sends.
- The existing `roll20_live_canonical_returns_unsupported_err` test (lines 470-490) is the template for the Roll20 fast-fail tests below.

Add new tests inside the same module (after existing tests):

```rust
    #[tokio::test]
    async fn live_foundry_attribute_strength_routes_outbound() {
        let pool = fresh_pool().await;
        let (state, rx) = make_bridge_state(true);
        let mut rx = rx.expect("connected state must yield a receiver");

        do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "attribute.strength".to_string(),
            serde_json::json!(4),
        )
        .await
        .expect("happy path");

        let sent = rx.recv().await.expect("router must send outbound message");
        assert!(
            sent.contains("system.attributes.strength.value"),
            "outbound payload should contain the translated path; got: {sent}"
        );
        // Value is stringified by do_set_attribute (Rust → JSON), then
        // re-parsed by the JS executor. The stringified literal "4" appears
        // in the wire JSON.
        assert!(sent.contains("\"4\""), "value should be stringified; got: {sent}");
    }

    #[tokio::test]
    async fn live_roll20_attribute_strength_fast_fails() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Roll20,
            "abc".to_string(),
            "attribute.strength".to_string(),
            serde_json::json!(3),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("Roll20 live editing of canonical names not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn live_roll20_skill_brawl_fast_fails() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Roll20,
            "abc".to_string(),
            "skill.brawl".to_string(),
            serde_json::json!(2),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("Roll20 live editing of canonical names not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn unknown_attribute_key_errors_at_router() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);

        let err = do_set_field(
            &pool,
            &state,
            WriteTarget::Live,
            SourceKind::Foundry,
            "abc".to_string(),
            "attribute.foo".to_string(),
            serde_json::json!(0),
        )
        .await
        .unwrap_err();

        assert!(
            err.contains("unknown field 'attribute.foo'"),
            "got: {err}"
        );
    }
```

- [x] **Step 2: Run the new tests — verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tools::character::tests -- --nocapture`
Expected: FAIL — `unknown field 'attribute.strength'` because the router still uses `ALLOWED_NAMES.contains(&name.as_str())` which rejects namespaced names.

- [x] **Step 3: Switch the router to `is_allowed_name`**

In `src-tauri/src/tools/character.rs`, locate the import at line 16:

```rust
use crate::shared::canonical_fields::{canonical_to_roll20_attr, ALLOWED_NAMES};
```

Replace with:

```rust
use crate::shared::canonical_fields::{canonical_to_roll20_attr, is_allowed_name};
```

Then locate `do_set_field` (around line 70) and replace:

```rust
    if !ALLOWED_NAMES.contains(&name.as_str()) {
        return Err(format!("character/set_field: unknown field '{name}'"));
    }
```

with:

```rust
    if !is_allowed_name(&name) {
        return Err(format!("character/set_field: unknown field '{name}'"));
    }
```

- [x] **Step 4: Run tests — verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tools::character::tests`
Expected: PASS — existing tests AND the new namespaced-name tests.

- [x] **Step 5: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/tools/character.rs
git commit -m "feat(character/set_field): accept attribute.* / skill.* names

Switches the router's name-acceptance check from ALLOWED_NAMES.contains()
to is_allowed_name() so 'attribute.strength', 'skill.brawl', etc. flow
through to the apply layer (Task B2) and live-side path translation
(Task B3).

Roll20 + canonical-name fast-fail unchanged: namespaced names hit the
same 'Roll20 live editing of canonical names not yet supported' error
that legacy flat names already do.

Closes the Foundry portion of issue #28."
```

---

## Task B5: `db/saved_character.rs` — round-trip tests for new arms

**Goal:** Pin that `patch_saved_field('attribute.strength', 4)` and `patch_saved_field('skill.brawl', 3)` both round-trip through SQLite — read row → mutate `canonical_json` blob via `apply_canonical_field` → write back → re-read shows the new value at the right pointer.

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs`

**Anti-scope:** No source-code changes — `db_patch_field` already calls `apply_canonical_field` so the new arms wire through automatically. Tests-only task.

**Depends on:** Task B2 (apply layer), Task B4 (router not strictly needed for saved-side tests, but order in plan).

**Invariants cited:** spec §5.7 (db tests for new arms).

**Tests required:** YES — by definition; this task is just tests.

- [x] **Step 1: Locate the existing patch_saved_field tests**

In `src-tauri/src/db/saved_character.rs::tests`, find the existing `patch_field_*` tests (e.g. `patch_field_type_mismatch_errors` around line 500). They use the helpers `fresh_pool()` (in-memory pool with migrations applied), `sample_canonical()` (returns a `CanonicalCharacter` whose `raw` is `{}` or similar), `db_save(pool, &canonical, None)` (returns the new row's id), `db_patch_field(pool, id, name, &value)` (the function under test), and `db_list(pool)` (returns `Vec<SavedCharacter>` for read-back). The new tests mirror this pattern exactly.

- [x] **Step 2: Write the new tests**

Add inside the same `#[cfg(test)] mod tests` block (after the existing `patch_field_*` tests):

```rust
    #[tokio::test]
    async fn patch_field_attribute_strength_round_trip() {
        let pool = fresh_pool().await;
        // Start with a canonical character whose raw blob has the path:
        // system.attributes.strength.value = 1.
        let mut canonical = sample_canonical();
        canonical.raw = serde_json::json!({
            "system": { "attributes": { "strength": { "value": 1 } } }
        });
        let id = db_save(&pool, &canonical, None).await.unwrap();

        db_patch_field(&pool, id, "attribute.strength", &serde_json::json!(4))
            .await
            .expect("happy path");

        // Re-read and assert via db_list.
        let list = db_list(&pool).await.unwrap();
        let raw = &list[0].canonical.raw;
        assert_eq!(
            raw.pointer("/system/attributes/strength/value"),
            Some(&serde_json::json!(4))
        );
    }

    #[tokio::test]
    async fn patch_field_skill_brawl_round_trip_creates_intermediate_objects() {
        let pool = fresh_pool().await;
        // sample_canonical's raw is empty-ish; set_raw_u8 must build the full
        // /system/skills/brawl/value path from scratch.
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        db_patch_field(&pool, id, "skill.brawl", &serde_json::json!(3))
            .await
            .expect("must create intermediate objects and write");

        let list = db_list(&pool).await.unwrap();
        let raw = &list[0].canonical.raw;
        assert_eq!(
            raw.pointer("/system/skills/brawl/value"),
            Some(&serde_json::json!(3))
        );
    }

    #[tokio::test]
    async fn patch_field_unknown_attribute_key_errors() {
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();

        let err = db_patch_field(&pool, id, "attribute.foo", &serde_json::json!(1))
            .await
            .unwrap_err();

        assert!(err.contains("unknown attribute 'foo'"), "got: {err}");
    }
```

(Helpers `fresh_pool`, `sample_canonical`, `db_save`, `db_patch_field`, `db_list` all exist in the same `tests` module already — verified at `saved_character.rs:355-380` and `:500-510`.)

- [x] **Step 3: Run the new tests — verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::saved_character::tests::patch_saved_field`
Expected: PASS — existing flat-name patch tests AND the 3 new namespaced-name tests.

- [x] **Step 4: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "test(db/saved_character): round-trip patch_saved_field for attribute/skill

Confirms db_patch_field correctly mutates the canonical_json blob for
attribute.* and skill.* names by reading row -> apply_canonical_field
-> writing back -> re-reading.

No source change: db_patch_field already calls apply_canonical_field
so the new prefix arms (Task B2) wire through automatically."
```

---

## Task B6: TS mirror — extend `CanonicalFieldName` template literal

**Goal:** Re-export `AttributeName` / `SkillName` from `src/types.ts` (sourced from Plan A's `canonical-names.ts`) and extend `CanonicalFieldName` to include `` `attribute.${AttributeName}` `` and `` `skill.${SkillName}` `` arms. After this task, the existing `characterSetField(...)` typed wrapper in `src/lib/character/api.ts` autocompletes the new names without any other changes.

**Files:**
- Modify: `src/types.ts`

**Anti-scope:** Do not modify `src/lib/character/api.ts` (the wrapper's `name: CanonicalFieldName` parameter automatically widens). Do not modify any consumer Svelte component.

**Depends on:** Plan A merged (uses `src/lib/foundry/canonical-names.ts`), Task B4 (Rust router accepts the new names — keeps TS and Rust in sync).

**Invariants cited:** spec §5.5 (TS mirror), spec §5.6 (manual-checklist drift policy).

**Tests required:** NO. This is wiring; verification is `npm run check` proving the template-literal type compiles cleanly and the existing `characterSetField` calls still type-check.

- [x] **Step 1: Add the imports + re-exports + extended union**

Open `src/types.ts`. Locate the existing `CanonicalFieldName` declaration (around lines 65-77):

```ts
/**
 * Mirrors src-tauri/src/shared/canonical_fields.rs::ALLOWED_NAMES.
 * Adding a name = update both ends in the same commit.
 */
export type CanonicalFieldName =
  | 'hunger'
  | 'humanity'
  | 'humanity_stains'
  | 'blood_potency'
  | 'health_superficial'
  | 'health_aggravated'
  | 'willpower_superficial'
  | 'willpower_aggravated';
```

Replace with:

```ts
import type { AttributeName, SkillName } from './lib/foundry/canonical-names';
export type { AttributeName, SkillName };

/**
 * Mirrors the v2 canonical-name surface in
 * src-tauri/src/shared/canonical_fields.rs::is_allowed_name:
 *   - Legacy 8 flat names (FLAT_NAMES).
 *   - `attribute.<key>` for every key in ATTRIBUTE_NAMES.
 *   - `skill.<key>` for every key in SKILL_NAMES.
 *
 * Adding a name = update BOTH the Rust ATTRIBUTE_NAMES/SKILL_NAMES arrays
 * AND src/lib/foundry/canonical-names.ts in the same commit (manual-checklist
 * convention; matches BridgeCharacter mirror precedent).
 */
export type CanonicalFieldName =
  | 'hunger'
  | 'humanity'
  | 'humanity_stains'
  | 'blood_potency'
  | 'health_superficial'
  | 'health_aggravated'
  | 'willpower_superficial'
  | 'willpower_aggravated'
  | `attribute.${AttributeName}`
  | `skill.${SkillName}`;
```

(The `import type` must come before the existing imports at the top of the file IF the file already has imports. If `src/types.ts` has no other imports, place this at the very top.)

- [x] **Step 2: Run `npm run check`**

Run: `npm run check`
Expected: PASS — the new template-literal type compiles; existing `characterSetField('live', ..., 'hunger', ...)` calls still type-check; `characterSetField('live', ..., 'attribute.strength', ...)` would now compile too.

- [x] **Step 3: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS — full repo gate.

- [x] **Step 4: Commit**

```bash
git add src/types.ts
git commit -m "feat(types): CanonicalFieldName covers attribute.* / skill.* via template literal

Mirrors the Rust-side surface widening from Task B4. Pulls AttributeName
and SkillName literal unions from src/lib/foundry/canonical-names.ts
(Plan A's foundation module) and uses TS template-literal types to
generate the 9 + 27 = 36 new accepted name strings.

Existing characterSetField() typed wrapper in src/lib/character/api.ts
autocompletes the new names with no change to its signature."
```

---

## Final smoke test (manual, after all 6 tasks committed)

Per CLAUDE.md, verify.sh runs after each commit. The end-of-plan smoke verifies the wire path end-to-end against a live Foundry world.

**Setup:** `npm run tauri dev` with a connected Foundry world that has at least one vampire actor whose Strength and Brawl values are known (or set them to known values via the actor sheet first for a clean baseline).

- [x] **Live attribute write happy path** — open the desktop dev-tools console:

```js
await window.__TAURI_INTERNALS__.invoke('character_set_field', {
  target: 'live',
  source: 'foundry',
  sourceId: '<actor-id>',
  name: 'attribute.strength',
  value: 4,
});
```

**Expected:** the actor's Strength attribute updates to 4 in Foundry. Cross-reference by opening the actor sheet in Foundry; the value should reflect immediately (Foundry's reactive updates).

- [x] **Live skill write happy path** — same approach:

```js
await window.__TAURI_INTERNALS__.invoke('character_set_field', {
  target: 'live',
  source: 'foundry',
  sourceId: '<actor-id>',
  name: 'skill.brawl',
  value: 3,
});
```

**Expected:** the actor's Brawl skill updates to 3 in Foundry.

- [x] **Out-of-range validation** — call with `value: 6`. **Expected:** Promise rejects with `"character/set_field: 'attribute.strength' expects integer 0..=5, got 6"`.

- [x] **Unknown key validation** — call with `name: 'attribute.foo'`. **Expected:** Promise rejects with `"character/set_field: unknown field 'attribute.foo'"`.

- [x] **Saved-only write** — disconnect Foundry. Call with `target: 'saved'`. **Expected:** the saved character's `canonical_json` blob updates at `system.skills.brawl.value` (verify by opening the saved character in the campaign manager — the new Skills section from Plan A should show the updated value).

- [x] **Roll20 fast-fail** — find a Roll20 character. Call:

```js
await window.__TAURI_INTERNALS__.invoke('character_set_field', {
  target: 'live',
  source: 'roll20',
  sourceId: '<roll20-char-id>',
  name: 'attribute.strength',
  value: 3,
});
```

**Expected:** Promise rejects with `"character/set_field: Roll20 live editing of canonical names not yet supported"`.

---

## Self-review checklist

- [x] All 36 new names (9 attrs + 27 skills) are pinned by the coverage assertion tests in Task B3 — adding a new key to `ATTRIBUTE_NAMES` or `SKILL_NAMES` without adding a Foundry path arm or apply arm is now a `cargo test` failure.
- [x] The Rust `ATTRIBUTE_NAMES` / `SKILL_NAMES` arrays match the TS `FOUNDRY_ATTRIBUTE_NAMES` / `FOUNDRY_SKILL_NAMES` arrays from Plan A — both use the same 9 + 27 entries in the same alphabetical order.
- [x] `apply_canonical_field`'s 8 legacy flat-name arms are unchanged in behavior — Task B2 reshapes the function structure (`return Ok(())` per arm + fall-through dispatch) but each legacy arm's apply logic is byte-for-byte the same.
- [x] `canonical_to_foundry_path`'s signature change (`Option<&'static str>` → `Option<String>`) is updated at all call sites: `bridge/foundry/mod.rs::canonical_to_path` is the only one (verified via `grep`); legacy 8 arms allocate via `.to_string()` for uniformity (one allocation per IPC call — not measurable).
- [x] Roll20 fast-fail message string is identical to the existing one (legacy + namespaced names get the same error).

---

## Plan dependencies

- **Depends on:** Plan A merged (uses `AttributeName`/`SkillName` from `src/lib/foundry/canonical-names.ts` in Task B6).
- **Blocks:** nothing in Plan C (Plan C does NOT call `character_set_field`; the popover triggers rolls only).
- **Closes:** Foundry portion of issue #28. The Roll20 portion of #28 stays open (out of scope per spec §2.3).

---

## Execution handoff

Plan B is six tasks; B1, B2, B4 are TDD; B3, B5 are tests-only or test-extension; B6 is wiring. Recommended:
- **Subagent-driven:** one subagent per task. Tasks are mostly independent within Plan B (B2 needs B1's arrays; B3 needs B2's apply contract; B4 needs B1's predicate; B5 needs B2's apply layer; B6 needs Plan A merged). Total ~1.5 hr if dispatched serially.
- **Inline:** also fine. The plan is short enough that batched execution within a single session works.
