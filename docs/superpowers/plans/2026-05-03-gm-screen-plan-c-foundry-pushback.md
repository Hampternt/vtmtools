# GM Screen — Plan C: Foundry Push-Back Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-card "Push to Foundry" button on advantage-bound modifier cards that mirrors the card's effects to the underlying merit's `system.bonuses[]` on the live Foundry actor — opt-in per button press, never automatic.

**Architecture:** Extend `ModifierEffect` with a structured `paths: Vec<String>` field so the GM-Screen effect shape mirrors the only Foundry bonus fields that mechanically matter (`value` + `paths`). Conditional fields (`activeWhen`, `displayWhenInactive`, `unless`) are always pushed with safe defaults. A new dedicated IPC command `gm_screen_push_to_foundry(modifier_id)` reads the cached Foundry actor, finds the bound merit item, splices our previously-pushed bonuses out (identified by a `"GM Screen #<id>"` prefix in the bonus's `source` field), appends new ones translated from the modifier's `pool` effects, and writes the merged array via `actor.update_item_field`. Returns a `PushReport` so the UI can toast pushed/skipped counts. `difficulty` and `note` effects are skipped (no Foundry equivalent in `system.bonuses[]`); the report surfaces this so the GM understands the asymmetry.

**Tech Stack:** Tauri 2 + sqlx (existing IPC pattern from `tools/character.rs`); Foundry bridge via `bridge::foundry::actions::actor::build_update_item_field`; SvelteKit + Svelte 5 runes for the UI; `system.bonuses[]` shape per `src/types.ts::FoundryItemBonus`.

**Spec context:** This is the **first explicit GM Screen → Foundry write surface**. The spec at `docs/superpowers/specs/2026-05-03-gm-screen-design.md` §1 currently asserts: *"the tool does not auto-apply effects to the sheet."* Manual, opt-in, per-button-press push is consistent with the broader posture ("GM decides") but the wording must be updated. Task P7 lands the spec edit alongside the code so a reader can trace the shift.

---

## Translation rule (canonical reference for P5)

For each `ModifierEffect e` on a modifier `m`:

| `e.kind`     | Pushed?  | Translation                                                                                                       |
|--------------|----------|-------------------------------------------------------------------------------------------------------------------|
| `pool`       | Yes      | `{ source: "GM Screen #<m.id>: <m.name>", value: e.delta ?? 0, paths: e.paths.is_empty() ? vec![""] : e.paths.clone(), activeWhen: { check: "always", path: "", value: "" }, displayWhenInactive: true, unless: "" }` |
| `difficulty` | Skipped  | Foundry's `system.bonuses[]` is additive against stat dot-paths; no native difficulty subtraction mechanism. Skip with reason `"difficulty: no Foundry bonus equivalent"`. |
| `note`       | Skipped  | No mechanical value. Skip with reason `"note: descriptive only"`.                                                 |

**Idempotency:** before appending, the push command filters the existing `system.bonuses[]` array, removing any element whose `source` starts with the literal prefix `"GM Screen #<m.id>"` (followed by `:` or end-of-string). This means re-pushing replaces our prior contribution without touching player-added bonuses or bonuses pushed for other modifiers on the same item.

**Empty-paths handling:** Per the user-supplied Foundry sample, `paths: [""]` is a valid pathless bonus shape. We honor that as the default when the GM hasn't picked any paths.

---

## File structure

**Modify (Rust):**
- `src-tauri/src/shared/modifier.rs` — add `paths: Vec<String>` field to `ModifierEffect` with `#[serde(default)]` so existing `effects_json` rows deserialize cleanly.
- `src-tauri/src/db/modifier.rs` — extend an existing round-trip test to verify the new field; no command changes (effects_json is already a TEXT blob).
- `src-tauri/src/lib.rs` — register the new `gm_screen_push_to_foundry` command and the new `tools::gm_screen` module.
- `src-tauri/src/tools/mod.rs` — declare `pub mod gm_screen;`

**Create (Rust):**
- `src-tauri/src/tools/gm_screen.rs` — new module owning the push command. Houses (1) the `effect_to_bonus` translation function, (2) the `merge_bonuses` filter+append helper, (3) the `do_push_to_foundry` async inner, (4) the `gm_screen_push_to_foundry` `#[tauri::command]` wrapper, (5) the `PushReport` and `SkippedEffect` types, (6) inline tests covering the translation table, the merge logic (idempotency, no-clobber of player bonuses, no-clobber of other modifiers' bonuses), and the source/binding/cache-lookup error branches.

**Modify (TypeScript):**
- `src/types.ts` — add `paths: string[]` to `ModifierEffect`; add `PushReport` and `SkippedEffect` mirrors.
- `src/lib/modifiers/api.ts` — add `pushToFoundry(modifierId: number): Promise<PushReport>` typed wrapper.
- `src/store/modifiers.svelte.ts` — add `pushToFoundry` method that calls the wrapper and returns the report (no store-state mutation; the bonuses display refreshes via the existing bridge cache when Foundry echoes the updated actor back).
- `src/lib/components/gm-screen/ModifierEffectEditor.svelte` — add a chip-style paths input below the scope/delta row (hidden when `kind === 'note'`). Empty paths array is allowed.
- `src/lib/components/gm-screen/ModifierCard.svelte` — show paths inline alongside the existing effect summary; render a "↑ Push to Foundry" button in the foot when the new `canPush` prop is true.
- `src/lib/components/gm-screen/CharacterRow.svelte` — compute `canPush` per card (foundry source, advantage binding, materialized, has at least one pool effect), wire `onPush` to the store, surface `PushReport` via a transient inline notice on the card row.

**Modify (docs):**
- `docs/superpowers/specs/2026-05-03-gm-screen-design.md` — reword §1 (one sentence) to clarify the auto-apply prohibition, and add §11A "Phase 2.5 — explicit Foundry write-back" with the translation rule table and the manual-only posture statement.

**No DB migration required** — `effects_json` is a TEXT blob containing serialized JSON; the new `paths` field is added via `#[serde(default)]` on the Rust struct so old rows load with `paths = vec![]`.

---

## Task P1: Extend ModifierEffect with paths (Rust + TS)

**Files:**
- Modify: `src-tauri/src/shared/modifier.rs`
- Modify: `src-tauri/src/db/modifier.rs` (extend an existing round-trip test)
- Modify: `src/types.ts`

**Tests required:** Yes — schema/serde change. One round-trip test verifying old payloads (no `paths` key) deserialize with empty paths, and new payloads round-trip cleanly.

- [ ] **Step 1: Add `paths` to the Rust struct**

In `src-tauri/src/shared/modifier.rs`, locate the `ModifierEffect` struct and add the field. The struct currently uses `#[serde(rename_all = "camelCase")]`; the new field needs `#[serde(default)]` so historical `effects_json` rows that lack the key deserialize with an empty vec.

```rust
// src-tauri/src/shared/modifier.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModifierEffect {
    pub kind: ModifierKind,
    pub scope: Option<String>,
    pub delta: Option<i32>,
    pub note: Option<String>,
    /// Foundry-bonus dot-paths (e.g. ["attributes.strength", "skills.subterfuge"]).
    /// Mirrors `FoundryItemBonus.paths`. Empty vec = pathless. Only used by the
    /// push-to-Foundry command on `pool`-kind effects; ignored for other kinds.
    #[serde(default)]
    pub paths: Vec<String>,
}
```

- [ ] **Step 2: Extend a round-trip test**

In `src-tauri/src/db/modifier.rs`, find an existing test that round-trips `ModifierEffect` JSON (search for `effects_json` or `ModifierEffect`). Add coverage for both the legacy-shape and new-shape cases.

```rust
#[test]
fn modifier_effect_serde_back_compat_and_roundtrip() {
    // Legacy effects_json (no `paths` field) deserializes with empty paths.
    let legacy = r#"{"kind":"pool","scope":"Strength","delta":2,"note":null}"#;
    let parsed: crate::shared::modifier::ModifierEffect =
        serde_json::from_str(legacy).expect("legacy shape parses");
    assert_eq!(parsed.paths, Vec::<String>::new());
    assert_eq!(parsed.delta, Some(2));

    // New shape round-trips cleanly.
    let new_shape = crate::shared::modifier::ModifierEffect {
        kind: crate::shared::modifier::ModifierKind::Pool,
        scope: Some("Strength rolls".into()),
        delta: Some(3),
        note: None,
        paths: vec!["attributes.strength".into(), "skills.brawl".into()],
    };
    let json = serde_json::to_string(&new_shape).unwrap();
    let back: crate::shared::modifier::ModifierEffect =
        serde_json::from_str(&json).unwrap();
    assert_eq!(back, new_shape);
}
```

- [ ] **Step 3: Run the test to verify both cases pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml modifier_effect_serde_back_compat_and_roundtrip -- --nocapture`
Expected: PASS.

- [ ] **Step 4: Mirror in TypeScript**

In `src/types.ts`, find `ModifierEffect` and add `paths: string[]`. Note: TS does not support a "default missing on parse" — but the Rust serializer always emits the field (it'll be an empty array for legacy data after one save), and if any historical TS-side parse hits a missing `paths` field it's read as `undefined`, which `paths.length` etc. handle defensively. Editor and card code below treats `paths ?? []` to be safe.

```ts
// src/types.ts
export interface ModifierEffect {
  kind: ModifierKind;
  scope: string | null;
  delta: number | null;
  note: string | null;
  /**
   * Foundry-bonus dot-paths (e.g. ["attributes.strength"]). Empty array = pathless.
   * Only used by the push-to-Foundry button on `pool`-kind effects.
   */
  paths: string[];
}
```

- [ ] **Step 5: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

Expected: green.

```bash
git add src-tauri/src/shared/modifier.rs src-tauri/src/db/modifier.rs src/types.ts
git commit -m "$(cat <<'EOF'
feat(gm-screen): add paths[] to ModifierEffect for Foundry push

Prep for Plan C — the new field mirrors FoundryItemBonus.paths so
the push-to-Foundry command can map effects mechanically without a
freeform-text translation step. Defaults to empty for serde back-compat.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P2: Editor UI — chip-style paths input

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierEffectEditor.svelte`

**Tests required:** No — UI authoring change; covered by manual smoke at the end of the plan.

- [ ] **Step 1: Add a paths chip-input row inside the effect-row, hidden for `kind === 'note'`**

Read the current file first. The `effect-row` grid currently has 4 columns: `select | scope-input | stepper | remove-button`. We need to keep that layout for the first row, then add a second row (paths chips) that spans full-width *under* the kind/scope/stepper line for non-note kinds.

Add one local helper and modify the `{#each effects}` block. The grid template stays the same; the paths line is a sibling div under each effect-row.

```svelte
<script lang="ts">
  // ...existing imports/state unchanged...

  function addPath(i: number, raw: string) {
    const p = raw.trim();
    if (!p) return;
    const cur = effects[i].paths ?? [];
    if (cur.includes(p)) return;
    effects[i] = { ...effects[i], paths: [...cur, p] };
  }

  function removePath(i: number, p: string) {
    const cur = effects[i].paths ?? [];
    effects[i] = { ...effects[i], paths: cur.filter(x => x !== p) };
  }
</script>

<!-- Inside the {#each effects as effect, i (i)} block, AFTER the existing
     .effect-row div, before its closing wrapper. Wrap the existing
     .effect-row + new .effect-paths in a .effect-block container. -->

{#each effects as effect, i (i)}
  <div class="effect-block">
    <div class="effect-row">
      <!-- existing select / scope-or-note / stepper / remove markup unchanged -->
    </div>
    {#if effect.kind !== 'note'}
      <div class="effect-paths">
        <span class="paths-label">paths</span>
        {#each effect.paths ?? [] as p}
          <span class="path-chip">
            {p === '' ? '(pathless)' : p}
            <button onclick={() => removePath(i, p)} aria-label="Remove path {p}">×</button>
          </span>
        {/each}
        <input
          type="text"
          class="path-input"
          placeholder="+ path (e.g. attributes.strength)"
          onkeydown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              const el = e.currentTarget as HTMLInputElement;
              addPath(i, el.value);
              el.value = '';
            }
          }}
          onblur={(e) => {
            const el = e.currentTarget as HTMLInputElement;
            if (el.value.trim()) { addPath(i, el.value); el.value = ''; }
          }}
        />
      </div>
    {/if}
  </div>
{/each}
```

Update `addEffect()` to include `paths: []`:

```ts
function addEffect() {
  effects = [...effects, { kind: 'pool', scope: null, delta: 0, note: null, paths: [] }];
}
```

Add to the `<style>` block (use existing token vars only — no hex literals, per ARCH §6):

```css
.effect-block { display: flex; flex-direction: column; gap: 0.3rem; }
.effect-paths {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.3rem;
  padding-left: 6.4rem;          /* aligns with end of select column above */
}
.paths-label {
  font-size: 0.65rem;
  color: var(--text-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.path-chip {
  background: var(--bg-input);
  color: var(--text-secondary);
  border: 1px solid var(--border-faint);
  border-radius: 999px;
  padding: 0.1rem 0.45rem;
  font-size: 0.7rem;
  display: inline-flex;
  align-items: center;
  gap: 0.2rem;
  font-family: ui-monospace, monospace;
}
.path-chip button {
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 0.7rem;
  padding: 0;
}
.path-input {
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-faint);
  border-radius: 0.3rem;
  padding: 0.15rem 0.4rem;
  font-size: 0.7rem;
  flex: 0 0 14rem;
  min-width: 8rem;
  box-sizing: border-box;
}
```

- [ ] **Step 2: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

```bash
git add src/lib/components/gm-screen/ModifierEffectEditor.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): chip-style paths picker on effect editor

Hidden for note-kind effects. Pathless effects are allowed (Foundry
accepts paths:[""]). Used by the upcoming push-to-Foundry button.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P3: Card display — show paths under each effect

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte`

**Tests required:** No — display change.

- [ ] **Step 1: Update `summarize()` to include paths inline**

In `src/lib/components/gm-screen/ModifierCard.svelte`, find the `summarize(e: ModifierEffect)` function (around line 31) and append a paths suffix when present.

```ts
function summarize(e: ModifierEffect): string {
  if (e.kind === 'note') return e.note ?? 'note';
  const sign = (e.delta ?? 0) >= 0 ? '+' : '';
  const scope = e.scope ? `${e.scope} ` : '';
  const label = e.kind === 'pool' ? 'dice' : 'difficulty';
  const paths = (e.paths ?? []).filter(p => p !== '');
  const pathSuffix = paths.length > 0 ? ` → ${paths.join(', ')}` : '';
  return `${scope}${sign}${e.delta ?? 0} ${label}${pathSuffix}`;
}
```

- [ ] **Step 2: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): show effect paths inline on the card

When a pool/difficulty effect has paths, render them after the delta
to make the mechanical mapping visible without opening the editor.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P4: Push button on the card (UI only — handler stub)

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte`

**Tests required:** No — wiring only.

- [ ] **Step 1: Add `canPush` and `onPush` props**

Extend the `Props` interface and the `$props()` destructure:

```ts
interface Props {
  // ...existing props unchanged...
  /** True when this card is push-to-Foundry-eligible: Foundry source,
   *  advantage binding, materialized, with at least one pool effect. */
  canPush?: boolean;
  onPush?: () => void;
}

let {
  modifier, isVirtual = false, isStale = false, bonuses = [],
  canPush = false, onPush,
  onToggleActive, onOpenEditor, onHide,
}: Props = $props();
```

- [ ] **Step 2: Render the button in the `.foot` block, between toggle and hide**

```svelte
<div class="foot">
  <button
    class="toggle"
    class:on={modifier.isActive}
    onclick={onToggleActive}
  >{modifier.isActive ? 'ON' : 'OFF'}</button>
  {#if canPush}
    <button
      class="push"
      title="Push these effects to the merit on Foundry"
      onclick={onPush}
    >↑ Push</button>
  {/if}
  {#if !modifier.isHidden}
    <button class="hide" title="Hide card" onclick={onHide}>×</button>
  {/if}
</div>
```

Add to the `<style>` block:

```css
.push {
  background: var(--bg-input);
  color: var(--text-secondary);
  border: 1px solid var(--border-faint);
  border-radius: 0.3rem;
  padding: 0.15rem 0.5rem;
  font-size: 0.65rem;
  cursor: pointer;
  opacity: 0;
  transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
}
.modifier-card:hover .push,
.push:focus { opacity: 1; }
.push:hover { background: var(--accent); color: var(--text-primary); border-color: var(--accent-bright); }
```

- [ ] **Step 3: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): add push-to-Foundry button on eligible cards

Visible only when canPush=true (parent computes: Foundry source,
materialized advantage binding, has pool effects). Wired in P6.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P5: New IPC command `gm_screen_push_to_foundry` + translation logic + tests

**Files:**
- Create: `src-tauri/src/tools/gm_screen.rs`
- Modify: `src-tauri/src/tools/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/types.ts` (add `PushReport` mirror)
- Modify: `src/lib/modifiers/api.ts` (add typed wrapper)

**Tests required:** Yes — translation table + merge idempotency + error branches + the new `get_by_id` helper. This is a character-data transform (per CLAUDE.md "Reserve TDD for genuine logic: ... character data transforms").

> **Pre-flight verifications already done by the plan-writer:**
> - `crate::db::modifier` has `list_character_modifiers`, `list_all_character_modifiers`, `add_character_modifier`, `update_character_modifier`, `delete_character_modifier`, `set_modifier_active`, `set_modifier_hidden`, `materialize_advantage_modifier` — but **no single-id loader**. Step 0 below adds one.
> - `BridgeState.characters` is `Mutex<HashMap<String, CanonicalCharacter>>` (in `src-tauri/src/bridge/mod.rs`). The String key is built by `CanonicalCharacter::key()` as `format!("{}:{}", source.as_str(), source_id)` (in `src-tauri/src/bridge/types.rs`). Foundry lookup key is therefore `"foundry:<source_id>"`.
> - `CanonicalCharacter.raw` is `serde_json::Value` (NOT `Option<Value>`). Access `.get("items")` directly — no Option unwrap.

- [ ] **Step 0: Add `get_modifier_by_id` to `db/modifier.rs` (TDD)**

In `src-tauri/src/db/modifier.rs`, add a new helper that selects one row by id and reconstructs a `CharacterModifier` from the stored columns + JSON blobs. Use the same row-mapping pattern as the existing `list_character_modifiers` (which already knows how to assemble a `CharacterModifier` from `effects_json`, `binding_json`, `tags_json`).

Write the test first:

```rust
// in the existing #[cfg(test)] mod tests { ... } block
#[tokio::test]
async fn get_by_id_returns_inserted_row() {
    let pool = test_pool().await;     // reuse whatever helper the existing tests use
    let new = NewCharacterModifier {
        source: SourceKind::Foundry,
        source_id: "actor-x".into(),
        name: "Test Mod".into(),
        description: String::new(),
        effects: vec![ModifierEffect {
            kind: ModifierKind::Pool, scope: None, delta: Some(2),
            note: None, paths: vec!["attributes.strength".into()],
        }],
        binding: ModifierBinding::Advantage { item_id: "item-y".into() },
        tags: vec!["combat".into()],
        origin_template_id: None,
    };
    let added = add_character_modifier(&pool, new).await.unwrap();
    let loaded = get_modifier_by_id(&pool, added.id).await.unwrap();
    assert_eq!(loaded.id, added.id);
    assert_eq!(loaded.name, "Test Mod");
    assert_eq!(loaded.effects.len(), 1);
    assert_eq!(loaded.effects[0].paths, vec!["attributes.strength".to_string()]);
    assert!(matches!(loaded.binding, ModifierBinding::Advantage { ref item_id } if item_id == "item-y"));
}

#[tokio::test]
async fn get_by_id_unknown_returns_err() {
    let pool = test_pool().await;
    let err = get_modifier_by_id(&pool, 99999).await.expect_err("unknown id must err");
    assert!(err.contains("99999") || err.to_lowercase().contains("not found"));
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml get_by_id`
Expected: FAIL with "unresolved import" / "function not defined".

Now implement (model after how `list_character_modifiers` builds rows; the SELECT shape and JSON-decoding logic should be identical):

```rust
pub async fn get_modifier_by_id(
    pool: &SqlitePool,
    id: i64,
) -> Result<CharacterModifier, String> {
    let row = sqlx::query!(
        r#"SELECT id, source, source_id, name, description,
                  effects_json, binding_json, tags_json,
                  is_active, is_hidden, origin_template_id,
                  created_at, updated_at
           FROM character_modifiers WHERE id = ?"#,
        id
    )
    .fetch_optional(pool).await
    .map_err(|e| format!("db/modifier/get_by_id: {e}"))?
    .ok_or_else(|| format!("db/modifier/get_by_id: id {id} not found"))?;
    // Reuse the same field-decoding path used by list_character_modifiers.
    // If list_character_modifiers inlines the JSON parsing, extract a private
    // `row_to_modifier(row)` helper and have both call sites use it.
    Ok(/* ...same construction as list_character_modifiers's per-row block... */)
}
```

> **Implementation note:** if `list_character_modifiers` currently inlines the `serde_json::from_str` calls and field assembly, lift that into a private `fn row_to_modifier(...) -> Result<CharacterModifier, String>` and use it from both `list_*` and `get_modifier_by_id`. This keeps row decoding DRY and avoids drift.

Run the tests again — expected: PASS.

- [ ] **Step 1: Declare the new module**

In `src-tauri/src/tools/mod.rs`, append:

```rust
pub mod gm_screen;
```

- [ ] **Step 2: Write the failing translation tests first**

Create `src-tauri/src/tools/gm_screen.rs` with the translation function and merge helper as the test target. Start with the test module so the tests fail compile-first; then add the implementation.

```rust
// src-tauri/src/tools/gm_screen.rs
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tauri::State;

use crate::bridge::types::SourceKind;
use crate::bridge::BridgeState;
use crate::shared::modifier::{ModifierBinding, ModifierEffect, ModifierKind};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkippedEffect {
    pub effect_index: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PushReport {
    pub pushed: usize,
    pub skipped: Vec<SkippedEffect>,
}

/// Build the `source` field that tags one of our pushed bonuses.
/// Format: `"GM Screen #<id>: <name>"`. The `"GM Screen #<id>"` prefix
/// (followed by `:` or end-of-string) is what we filter on for re-push.
fn source_tag(modifier_id: i64, modifier_name: &str) -> String {
    format!("GM Screen #{modifier_id}: {modifier_name}")
}

/// True iff the bonus's `source` was pushed by THIS modifier.
/// Matches `"GM Screen #<id>"` followed by exactly `:` or end-of-string,
/// so id 5 doesn't match id 50.
fn is_ours(bonus: &Value, modifier_id: i64) -> bool {
    let Some(source) = bonus.get("source").and_then(|v| v.as_str()) else { return false; };
    let prefix = format!("GM Screen #{modifier_id}");
    if !source.starts_with(&prefix) { return false; }
    let rest = &source[prefix.len()..];
    rest.is_empty() || rest.starts_with(':')
}

/// Translate one ModifierEffect to a Foundry bonus value. Returns None for
/// non-pool kinds (those are reported as skipped in the PushReport).
fn effect_to_bonus(
    effect: &ModifierEffect,
    modifier_id: i64,
    modifier_name: &str,
) -> Option<Value> {
    if effect.kind != ModifierKind::Pool { return None; }
    let value = effect.delta.unwrap_or(0);
    let paths: Vec<String> = if effect.paths.is_empty() {
        vec!["".to_string()]
    } else {
        effect.paths.clone()
    };
    Some(json!({
        "source": source_tag(modifier_id, modifier_name),
        "value": value,
        "paths": paths,
        "activeWhen": { "check": "always", "path": "", "value": "" },
        "displayWhenInactive": true,
        "unless": "",
    }))
}

/// Filter existing bonuses (drop ones tagged as ours), then append `ours`.
/// Player-added bonuses and bonuses from other modifiers are preserved.
fn merge_bonuses(existing: &[Value], modifier_id: i64, ours: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = existing
        .iter()
        .filter(|b| !is_ours(b, modifier_id))
        .cloned()
        .collect();
    out.extend(ours);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pool_effect(delta: i32, paths: Vec<&str>) -> ModifierEffect {
        ModifierEffect {
            kind: ModifierKind::Pool,
            scope: None,
            delta: Some(delta),
            note: None,
            paths: paths.into_iter().map(String::from).collect(),
        }
    }

    #[test]
    fn translation_pool_with_paths_emits_bonus() {
        let e = pool_effect(2, vec!["attributes.strength", "skills.brawl"]);
        let b = effect_to_bonus(&e, 7, "Brawl Buff").expect("pool kind translates");
        assert_eq!(b["value"], 2);
        assert_eq!(b["paths"], json!(["attributes.strength", "skills.brawl"]));
        assert_eq!(b["source"], "GM Screen #7: Brawl Buff");
        assert_eq!(b["activeWhen"]["check"], "always");
        assert_eq!(b["displayWhenInactive"], true);
        assert_eq!(b["unless"], "");
    }

    #[test]
    fn translation_pool_with_no_paths_emits_pathless_bonus() {
        let e = pool_effect(3, vec![]);
        let b = effect_to_bonus(&e, 1, "X").expect("pool kind translates");
        assert_eq!(b["paths"], json!([""]), "empty paths becomes [\"\"] per Foundry sample");
    }

    #[test]
    fn translation_difficulty_returns_none() {
        let e = ModifierEffect {
            kind: ModifierKind::Difficulty, scope: None, delta: Some(-1),
            note: None, paths: vec!["attributes.strength".into()],
        };
        assert!(effect_to_bonus(&e, 1, "X").is_none(), "difficulty must skip");
    }

    #[test]
    fn translation_note_returns_none() {
        let e = ModifierEffect {
            kind: ModifierKind::Note, scope: None, delta: None,
            note: Some("careful".into()), paths: vec![],
        };
        assert!(effect_to_bonus(&e, 1, "X").is_none(), "note must skip");
    }

    #[test]
    fn merge_filters_only_our_modifier_id() {
        let existing = vec![
            json!({"source": "Player Buff", "value": 1, "paths": ["x"]}),
            json!({"source": "GM Screen #5: A", "value": 2, "paths": ["y"]}),     // ours
            json!({"source": "GM Screen #50: B", "value": 3, "paths": ["z"]}),    // NOT ours (id 50)
            json!({"source": "GM Screen #6: C", "value": 4, "paths": ["w"]}),     // NOT ours (id 6)
            json!({"source": "GM Screen #5", "value": 9, "paths": []}),           // ours (no name suffix)
        ];
        let ours = vec![json!({"source": "GM Screen #5: A", "value": 99, "paths": ["new"]})];
        let merged = merge_bonuses(&existing, 5, ours);
        assert_eq!(merged.len(), 4, "kept 3 non-ours + 1 new");
        assert!(merged.iter().any(|b| b["source"] == "Player Buff"));
        assert!(merged.iter().any(|b| b["source"] == "GM Screen #50: B"));
        assert!(merged.iter().any(|b| b["source"] == "GM Screen #6: C"));
        let ours_new = merged.iter().find(|b| b["source"] == "GM Screen #5: A").unwrap();
        assert_eq!(ours_new["value"], 99);
    }

    #[test]
    fn merge_with_no_existing_bonuses_just_appends() {
        let merged = merge_bonuses(&[], 1, vec![json!({"source": "GM Screen #1: X"})]);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn merge_idempotent_under_repeated_push() {
        let initial: Vec<Value> = vec![];
        let ours_v1 = vec![json!({"source": "GM Screen #2: X", "value": 1, "paths": ["a"]})];
        let after_first = merge_bonuses(&initial, 2, ours_v1);
        let ours_v2 = vec![json!({"source": "GM Screen #2: X", "value": 1, "paths": ["a"]})];
        let after_second = merge_bonuses(&after_first, 2, ours_v2);
        assert_eq!(after_first, after_second, "re-push yields the same array");
    }
}
```

- [ ] **Step 3: Run the tests; expect them to compile and pass (translation + merge are pure functions)**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --package vtmtools tools::gm_screen::tests
```

Expected: PASS for all 7 tests.

- [ ] **Step 4: Add the IPC command (the do_* + tauri::command pair)**

Append to `src-tauri/src/tools/gm_screen.rs`:

```rust
/// Inner logic: load modifier, validate binding, read cached actor + item,
/// merge, send actor.update_item_field via the bridge.
pub(crate) async fn do_push_to_foundry(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    modifier_id: i64,
) -> Result<PushReport, String> {
    // 1. Load the modifier (helper added in Step 0).
    let m = crate::db::modifier::get_modifier_by_id(pool, modifier_id).await
        .map_err(|e| format!("gm_screen/push: load modifier {modifier_id}: {e}"))?;

    // 2. Validate source + binding.
    if m.source != SourceKind::Foundry {
        return Err(format!(
            "gm_screen/push: modifier {} is not a Foundry-source modifier (source={:?})",
            modifier_id, m.source
        ));
    }
    let item_id = match &m.binding {
        ModifierBinding::Advantage { item_id } => item_id.clone(),
        ModifierBinding::Free => {
            return Err(format!(
                "gm_screen/push: modifier {modifier_id} has free binding; only advantage-bound modifiers can push"
            ));
        }
    };

    // 3. Build the new bonuses and the skipped report.
    let mut new_bonuses = Vec::new();
    let mut skipped = Vec::new();
    for (i, effect) in m.effects.iter().enumerate() {
        match effect_to_bonus(effect, m.id, &m.name) {
            Some(b) => new_bonuses.push(b),
            None => {
                let reason = match effect.kind {
                    ModifierKind::Difficulty => "difficulty: no Foundry bonus equivalent",
                    ModifierKind::Note => "note: descriptive only",
                    ModifierKind::Pool => unreachable!("Pool always translates"),
                };
                skipped.push(SkippedEffect { effect_index: i, reason: reason.to_string() });
            }
        }
    }

    // 4. Read cached actor + locate the item, then read existing bonuses.
    //    We use the BridgeState's character cache (already populated by the
    //    Foundry bridge on Hello/refresh). The cache is a HashMap<String, _>
    //    keyed by `CanonicalCharacter::key()` = `format!("{source}:{source_id}")`.
    //    TOCTOU note: if the player edits bonuses in Foundry between our read
    //    and our write, edits to OUR-tagged bonuses can be lost. Player-added
    //    bonuses are safe (filtered out of `is_ours`). Acceptable for v1.
    let key = format!("{}:{}", SourceKind::Foundry.as_str(), m.source_id);
    let chars = bridge_state.characters.lock().await;
    let actor = chars.get(&key).cloned().ok_or_else(|| format!(
        "gm_screen/push: actor {} not in bridge cache (is Foundry connected?)",
        m.source_id
    ))?;
    drop(chars);

    // CanonicalCharacter.raw is a serde_json::Value (NOT Option<Value>).
    let items = actor.raw.get("items").and_then(|v| v.as_array()).ok_or_else(|| format!(
        "gm_screen/push: actor {} raw has no items[] array", m.source_id
    ))?;
    let item = items.iter().find(|it| it.get("_id").and_then(|v| v.as_str()) == Some(item_id.as_str()))
        .ok_or_else(|| format!(
            "gm_screen/push: item {} not found on actor {} (was the merit deleted?)",
            item_id, m.source_id
        ))?;
    let existing: Vec<Value> = item.get("system")
        .and_then(|s| s.get("bonuses"))
        .and_then(|b| b.as_array())
        .cloned()
        .unwrap_or_default();

    // 5. Merge and send.
    let merged = merge_bonuses(&existing, m.id, new_bonuses.clone());
    let payload = crate::bridge::foundry::actions::actor::build_update_item_field(
        &m.source_id,
        &item_id,
        "system.bonuses",
        Value::Array(merged),
    );
    let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    crate::bridge::commands::send_to_source_inner(bridge_state, SourceKind::Foundry, text).await
        .map_err(|e| format!("gm_screen/push: bridge send failed: {e}"))?;

    Ok(PushReport { pushed: new_bonuses.len(), skipped })
}

#[tauri::command]
pub async fn gm_screen_push_to_foundry(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    modifier_id: i64,
) -> Result<PushReport, String> {
    do_push_to_foundry(&db.0, &bridge.0, modifier_id).await
}
```

> **Implementation note for the agent:** Verify the exact name of the modifier-by-id lookup function in `src-tauri/src/db/modifier.rs` before writing this code. The plan assumes `get_by_id(pool, id) -> Result<CharacterModifier, ...>`. If the existing API differs (e.g., `find_by_id`, `load_one`, returning `Option<CharacterModifier>`), use whatever the file actually exports. Add a helper if absolutely necessary, but prefer the existing surface — the rest of the codebase already loads modifiers by id somewhere (e.g., during the `update`/`set_active`/`set_hidden` paths), so the helper exists in some form.

- [ ] **Step 5: Register the command in `src-tauri/src/lib.rs`**

Find the `invoke_handler!` macro call (in `tauri::Builder::default()....invoke_handler(...)`). Append the new command to the existing list of `tools::*` entries:

```rust
.invoke_handler(tauri::generate_handler![
    // ...existing handlers...
    crate::tools::gm_screen::gm_screen_push_to_foundry,
])
```

- [ ] **Step 6: Mirror PushReport / SkippedEffect in TypeScript**

In `src/types.ts`, add to the GM Screen section (near `CharacterModifier`):

```ts
export interface SkippedEffect {
  effectIndex: number;
  reason: string;
}

export interface PushReport {
  pushed: number;
  skipped: SkippedEffect[];
}
```

- [ ] **Step 7: Add the typed wrapper**

In `src/lib/modifiers/api.ts`, append:

```ts
import type { PushReport } from '../../types';
// ...existing imports/wrappers unchanged...

export async function pushToFoundry(modifierId: number): Promise<PushReport> {
  return await invoke<PushReport>('gm_screen_push_to_foundry', { modifierId });
}
```

- [ ] **Step 8: Run all verify gates**

```bash
./scripts/verify.sh
```

Expected: green. The Rust tests added in Step 2 must still pass; the new command must compile; `npm run check` must pass for the new TS types/wrappers.

- [ ] **Step 9: Commit**

```bash
git add \
  src-tauri/src/tools/gm_screen.rs \
  src-tauri/src/tools/mod.rs \
  src-tauri/src/lib.rs \
  src/types.ts \
  src/lib/modifiers/api.ts
git commit -m "$(cat <<'EOF'
feat(gm-screen): gm_screen_push_to_foundry command

Translates pool effects to FoundryItemBonus rows and writes them to the
bound merit's system.bonuses[] via actor.update_item_field. Tagged with
"GM Screen #<id>" source prefix so re-push replaces our prior bonuses
without touching player-added ones. Difficulty and note effects are
skipped and surfaced in the PushReport.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P6: Wire push through store + CharacterRow + toast

**Files:**
- Modify: `src/store/modifiers.svelte.ts`
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`

**Tests required:** No — wiring only.

- [ ] **Step 1: Add `pushToFoundry` to the store**

In `src/store/modifiers.svelte.ts`, near the other CRUD methods, add:

```ts
import { pushToFoundry as apiPushToFoundry } from '../lib/modifiers/api';
import type { PushReport } from '../types';
// ...existing imports unchanged...

// inside the modifiers store object, alongside setActive/setHidden/update:
async pushToFoundry(modifierId: number): Promise<PushReport> {
  return await apiPushToFoundry(modifierId);
}
```

- [ ] **Step 2: Wire `canPush`, `onPush`, and an inline notice in `CharacterRow.svelte`**

Add a transient notice state and a handler:

```ts
let pushNotice = $state<{ cardKey: string; text: string; ok: boolean } | null>(null);

function canPushFor(e: CardEntry): boolean {
  if (character.source !== 'foundry') return false;
  if (e.kind !== 'materialized') return false;            // virtual = no DB row yet
  if (e.mod.binding.kind !== 'advantage') return false;
  if (e.isStale) return false;
  return e.mod.effects.some(eff => eff.kind === 'pool');
}

async function handlePush(e: CardEntry) {
  if (e.kind !== 'materialized') return;
  const cardKey = `m-${e.mod.id}`;
  try {
    const report = await modifiers.pushToFoundry(e.mod.id);
    const skippedSummary = report.skipped.length > 0
      ? ` (skipped ${report.skipped.length}: ${report.skipped.map(s => s.reason).join('; ')})`
      : '';
    pushNotice = {
      cardKey,
      text: `Pushed ${report.pushed} bonus${report.pushed === 1 ? '' : 'es'} to Foundry${skippedSummary}`,
      ok: true,
    };
  } catch (err) {
    pushNotice = { cardKey, text: `Push failed: ${err}`, ok: false };
  }
  // auto-clear after 5s
  setTimeout(() => { if (pushNotice?.cardKey === cardKey) pushNotice = null; }, 5000);
}
```

In the `<ModifierCard>` invocation, add the two new props:

```svelte
<ModifierCard
  {/* ...existing props unchanged... */}
  canPush={canPushFor(entry)}
  onPush={() => handlePush(entry)}
/>
```

Render the notice under the modifier-row (single notice per row, scoped by cardKey for clarity):

```svelte
{#if pushNotice}
  <p class="push-notice" class:ok={pushNotice.ok} class:err={!pushNotice.ok}>
    {pushNotice.text}
  </p>
{/if}
```

Add styles:

```css
.push-notice {
  font-size: 0.7rem;
  margin: 0.4rem 0 0 0;
  padding: 0.25rem 0.5rem;
  border-radius: 0.25rem;
  background: var(--bg-input);
  color: var(--text-secondary);
}
.push-notice.ok  { color: var(--text-primary); border-left: 2px solid var(--accent-bright); }
.push-notice.err { color: var(--accent-amber);  border-left: 2px solid var(--accent-amber); }
```

- [ ] **Step 3: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

```bash
git add src/store/modifiers.svelte.ts src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): wire push button through store and character row

CharacterRow computes canPush per card (foundry source, advantage
binding, materialized, has pool effects) and handles the push, surfacing
the PushReport via a transient inline notice with auto-clear.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P7: Spec update — reword §1 and add §11A

**Files:**
- Modify: `docs/superpowers/specs/2026-05-03-gm-screen-design.md`

**Tests required:** No — doc only. (Per CLAUDE.md memory `feedback_plans_must_include_verify`, still run verify.sh before committing.)

- [ ] **Step 1: Reword §1**

Locate the §1 sentence:

> *"the tool does not auto-fold modifiers into V5 dice pools or auto-apply effects to the sheet."*

Replace with:

> *"the tool does not auto-fold modifiers into V5 dice pools, and never automatically applies effects to the sheet. The GM may explicitly mirror a card's effects to the bound merit's `system.bonuses[]` on Foundry via a per-card push button (Phase 2.5, §11A) — but this is always a manual, opt-in, single-button-press action, never automatic."*

- [ ] **Step 2: Add §11A**

Add a new subsection under §11 (or §10 if §11 doesn't exist; check the current ToC). Title it `## 11A. Phase 2.5 — Explicit Foundry write-back (push to merit bonuses)`. Body:

```markdown
## 11A. Phase 2.5 — Explicit Foundry write-back (push to merit bonuses)

A per-card "↑ Push" button on advantage-bound cards (Foundry sources only)
that mirrors the card's effects to the bound merit's `system.bonuses[]`.

**Visibility:** the button is rendered only when ALL of the following hold:

- character source is `foundry`
- card is materialized (has DB row, not a virtual advantage card)
- binding is `advantage` (not `free`)
- card is not stale (the source merit still exists on the actor)
- card has at least one `pool` effect

**Translation rule.** Each `ModifierEffect` translates as follows:

| `kind`       | Behavior                                                                          |
|--------------|-----------------------------------------------------------------------------------|
| `pool`       | Emit one bonus: `value = delta`, `paths = e.paths` (`[""]` if empty per Foundry sample). All conditional fields default: `activeWhen = { check: "always", path: "", value: "" }`, `displayWhenInactive = true`, `unless = ""`. |
| `difficulty` | Skipped — Foundry's `system.bonuses[]` has no difficulty mechanism. Surfaced in `PushReport.skipped` so the GM understands the asymmetry. |
| `note`       | Skipped — descriptive only.                                                       |

**Idempotency.** Each pushed bonus is tagged `source: "GM Screen #<modifier_id>: <name>"`.
Re-pushing first removes any bonus whose `source` starts with `"GM Screen #<id>"`
(matching exactly, so id 5 doesn't match id 50), then appends the freshly translated
ones. Player-added bonuses and bonuses pushed for other modifiers on the same item
are preserved.

**TOCTOU caveat.** The push reads `system.bonuses[]` from the cached actor in
`BridgeState`, then writes the merged array. If the player edits the same item's
bonuses in Foundry between the read and the write, edits to bonuses tagged as ours
can be lost (player-added bonuses are not at risk — they're filtered through `is_ours`
and pass through unchanged). Acceptable for v1; documented in
`src-tauri/src/tools/gm_screen.rs::do_push_to_foundry`.

**No Roll20 equivalent.** Roll20 sheets don't expose a `system.bonuses[]`
analogue; the push button is hidden for Roll20 characters by the visibility
predicate above.

### Reset semantics

A per-card "↺ Reset" button on materialized advantage cards (Foundry sources only)
deletes the local DB row and reverts the card to its virtual baseline (just the
item name + whatever bonuses Foundry currently has on the merit).

**Local-only delete.** Reset does NOT touch Foundry's `system.bonuses[]`. The
two data stores are separated by design: local effects are *intent*, Foundry
bonuses are *durable state*. Reset drops the intent; durable state is left to
the GM (or the player) to manage on the Foundry side.

**Orphan implication.** If the card was previously pushed, the
`GM Screen #<old_id>: <name>` tagged bonuses persist on the merit until
manually removed in Foundry. They remain visibly labeled, so the GM can
spot them in the merit's bonus list. Over many reset→re-push cycles
orphan bonuses with stale ids can accumulate; a future "clean up GM Screen
orphans" action (out of scope for this phase) would batch-remove them.

**Confirmation.** Reset is destructive (deletes all local effects, paths,
tags, isActive, isHidden in one step). The button triggers a `confirm()`
dialog before delete.

**Free modifiers.** Not eligible — there's no live baseline to revert to.
Reset is hidden on free-binding cards.
```

- [ ] **Step 3: Run verify.sh (doc-only commit; verify still gates per CLAUDE.md hard rule)**

```bash
./scripts/verify.sh
```

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-05-03-gm-screen-design.md
git commit -m "$(cat <<'EOF'
docs(gm-screen): document Phase 2.5 explicit Foundry write-back

Rewords §1 to make the manual-only push exception explicit, and adds
§11A documenting the translation rule, idempotency strategy, and TOCTOU
caveat for the push-to-Foundry button shipped in Plan C.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P8: Filter our pushed bonuses out of the sheet-bonuses display

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`

**Tests required:** No — display filter only.

**Why:** without this filter, after a push the same effect appears twice on the card — once in the local "GM Screen effects" section, once mirrored back in the "Sheet-attached bonuses" section. Filtering bonuses tagged `"GM Screen #<id>"` for any *live* modifier id on this character keeps the sheet-bonuses section showing only player-added bonuses. Orphan bonuses (tagged with ids no longer in the store) intentionally pass the filter so the GM can still see them.

- [ ] **Step 1: Build a per-character set of live tag prefixes and apply it in `bonusesFor()`**

In `src/lib/components/gm-screen/CharacterRow.svelte`, modify `bonusesFor()` so that bonuses whose `source` matches any live modifier's tag prefix are filtered out.

```ts
// Add a derived set of live tag prefixes for this character.
let liveTagPrefixes = $derived(new Set(
  charMods.map(m => `GM Screen #${m.id}`)
));

function bonusesFor(itemId: string): FoundryItemBonus[] {
  const item = advantageItems.find(it => it._id === itemId);
  if (!item) return [];
  const raw = (item.system as Record<string, unknown>)?.bonuses;
  if (!Array.isArray(raw)) return [];
  return (raw as FoundryItemBonus[]).filter(b => {
    const src = b.source ?? '';
    // Filter out anything tagged with a LIVE modifier id (those are shown
    // in the local effects section). Orphans (stale ids) intentionally
    // pass through so the GM can spot them.
    for (const prefix of liveTagPrefixes) {
      if (src === prefix || src.startsWith(prefix + ':')) return false;
    }
    return true;
  });
}
```

- [ ] **Step 2: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): hide our pushed bonuses from sheet-bonuses display

Bonuses tagged "GM Screen #<id>" for any live modifier on the character
are filtered out of the sheet-bonuses section, since the same data is
already shown in the local effects section. Orphan bonuses (stale ids
with no matching live modifier) pass through so the GM can spot them.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task P9: Reset button on materialized advantage cards

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte`
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`

**Tests required:** No — UI + wiring to existing `delete_character_modifier` IPC. No new IPC.

- [ ] **Step 1: Add `canReset` and `onReset` props to ModifierCard**

In `src/lib/components/gm-screen/ModifierCard.svelte`, extend the Props interface and the destructure:

```ts
interface Props {
  // ...existing props unchanged (modifier, isVirtual, isStale, bonuses,
  //  canPush, onPush, onToggleActive, onOpenEditor, onHide)...
  /** True for materialized, advantage-bound cards on Foundry sources. */
  canReset?: boolean;
  onReset?: () => void;
}

let {
  modifier, isVirtual = false, isStale = false, bonuses = [],
  canPush = false, onPush,
  canReset = false, onReset,
  onToggleActive, onOpenEditor, onHide,
}: Props = $props();
```

Render the button in `.foot`, between Push and Hide:

```svelte
<div class="foot">
  <button class="toggle" class:on={modifier.isActive} onclick={onToggleActive}>
    {modifier.isActive ? 'ON' : 'OFF'}
  </button>
  {#if canPush}
    <button class="push" title="Push these effects to the merit on Foundry" onclick={onPush}>↑ Push</button>
  {/if}
  {#if canReset}
    <button class="reset" title="Reset card — drops local effects/paths/tags. Foundry bonuses unaffected." onclick={onReset}>↺ Reset</button>
  {/if}
  {#if !modifier.isHidden}
    <button class="hide" title="Hide card" onclick={onHide}>×</button>
  {/if}
</div>
```

Add styles (amber outline distinguishes reset from push; uses existing token):

```css
.reset {
  background: var(--bg-input);
  color: var(--text-secondary);
  border: 1px solid var(--accent-amber);
  border-radius: 0.3rem;
  padding: 0.15rem 0.5rem;
  font-size: 0.65rem;
  cursor: pointer;
  opacity: 0;
  transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
}
.modifier-card:hover .reset,
.reset:focus { opacity: 1; }
.reset:hover { background: var(--accent-amber); color: var(--bg-card); }
```

- [ ] **Step 2: Wire `canReset` and `handleReset` in CharacterRow**

In `src/lib/components/gm-screen/CharacterRow.svelte`, alongside `canPushFor`:

```ts
function canResetFor(e: CardEntry): boolean {
  if (character.source !== 'foundry') return false;
  if (e.kind !== 'materialized') return false;
  if (e.mod.binding.kind !== 'advantage') return false;
  return true;
}

async function handleReset(e: CardEntry) {
  if (e.kind !== 'materialized') return;
  const ok = confirm(
    `Reset "${e.mod.name}"?\n\n` +
    `This deletes the local effects, paths, and tags for this card.\n` +
    `Any bonuses previously pushed to Foundry will REMAIN on the merit ` +
    `(visible as "GM Screen #${e.mod.id}: ...") and must be removed in ` +
    `Foundry manually if no longer wanted.`
  );
  if (!ok) return;
  await modifiers.delete(e.mod.id);
}
```

In the `<ModifierCard>` invocation, add the two new props alongside `canPush`/`onPush`:

```svelte
<ModifierCard
  {/* ...existing props unchanged including canPush/onPush... */}
  canReset={canResetFor(entry)}
  onReset={() => handleReset(entry)}
/>
```

- [ ] **Step 3: Run verify.sh and commit**

```bash
./scripts/verify.sh
```

```bash
git add \
  src/lib/components/gm-screen/ModifierCard.svelte \
  src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): reset button on materialized advantage cards

Drops the local DB row (effects, paths, tags, isActive, isHidden) and
reverts the card to its virtual baseline. Foundry-side bonuses are not
touched — the orphan implication is documented in the confirm dialog
and in spec §11A. Free-binding and Roll20 cards do not show the button.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Final smoke test (manual, after all tasks committed)

Auto-run by the implementer is not possible — these steps need a live Foundry connection.

1. Start Foundry, load a VTM5e character with a merit (e.g. "D Dracule Flow" or any feature with `featuretype === 'merit'`).
2. `npm run tauri dev`. Open the GM Screen tool.
3. Find the merit card on the relevant character row. Click the cog. Add a `pool` effect: `delta = 2`, paths chip = `attributes.strength`. Save.
4. Verify: card displays `+2 dice → attributes.strength` in the effect list.
5. Click `↑ Push` on the card. Verify the inline notice reads `Pushed 1 bonus to Foundry`.
6. In Foundry, open the merit. Verify a bonus appears under "Bonuses" with source `GM Screen #N: <merit name>`, value 2, path `attributes.strength`. Verify the character's Strength roll pool reflects +2.
7. **Idempotency check:** add a second pool effect with `delta = 1`, path `skills.brawl`. Save. Click `↑ Push`. Verify in Foundry: now 2 bonuses with `GM Screen #N` source (one for each effect). The first bonus from step 6 is NOT duplicated.
8. **Skip-report check:** add a `note` effect and a `difficulty` effect. Save. Click `↑ Push`. Verify the inline notice reads `Pushed 2 bonuses to Foundry (skipped 2: difficulty: no Foundry bonus equivalent; note: descriptive only)`.
9. **Player-bonus preservation:** in Foundry, manually add a bonus with source `Test Player Bonus`. Click `↑ Push` in the GM Screen. Verify the player bonus is still present in Foundry (not clobbered).
10. **Visibility check:** verify the `↑ Push` button does NOT appear on (a) virtual cards, (b) free modifiers, (c) cards on Roll20 characters, (d) materialized cards with only note/difficulty effects.
11. **Dedup check (P8):** after step 5's push, verify the merit's bonus appears ONLY in the local effects section on the card, NOT also in the sheet-attached bonuses section. The player-bonus from step 9 is still visible in the sheet-attached section.
12. **Reset check (P9):** click `↺ Reset` on a card that's been pushed. Confirm the dialog. Verify the card reverts to virtual (just the merit name). In Foundry, verify the `GM Screen #N: ...` bonus is STILL on the merit (orphaned, by design). Reload the page; verify the orphan now appears in the sheet-attached bonuses section on the virtual card (no live id matches the tag, so the dedup filter passes it through). Verify the `↺ Reset` button does NOT appear on (a) virtual cards, (b) free modifiers, (c) cards on Roll20 characters.

---

## Self-review (reviewed against the user's clarifying message and spec §1)

**1. Spec coverage:**
- User wanted "dedicated button per card, manual push, mirror effects to merit bonuses, Foundry only" — covered by P4 (button) + P5 (translation) + P6 (wiring).
- User wanted the effect editor to "have similar scope... of being able to set paths" — covered by P1 (schema) + P2 (chip-style picker).
- User wanted "default values for activeWhen / displayWhenInactive / unless" — covered by `effect_to_bonus` in P5 (always defaults).
- User wanted "convenient bidirectional mapping" — the new effect shape (paths + delta) matches the pushed bonus shape (paths + value), and the existing P0 bonuses-display feature already reads `system.bonuses[]` for display.

**2. Placeholder scan:** None — every step has the actual code.

**3. Type consistency:**
- `ModifierEffect.paths: Vec<String>` (Rust) ↔ `paths: string[]` (TS) — consistent.
- `PushReport.pushed: usize` (Rust) ↔ `pushed: number` (TS); `skipped: Vec<SkippedEffect>` ↔ `skipped: SkippedEffect[]` — consistent.
- `SkippedEffect.effectIndex` is camelCase TS-side because `shared/modifier.rs` uses `#[serde(rename_all = "camelCase")]` and we follow that convention; in Rust the field is `effect_index`. Mirrored.
- `gm_screen_push_to_foundry` IPC name (snake_case) ↔ `pushToFoundry` wrapper (camelCase) — matches the established `update_character_modifier` ↔ `update` pattern in `src/lib/modifiers/api.ts`.

**4. Spec §1 conflict surfaced:** P7 lands the wording change in the same branch. Reviewer/user can see the posture shift in the diff.

**5. CLAUDE.md compliance check:**
- Every commit step is preceded by `./scripts/verify.sh` ✓
- No new `std::fs` (only bridge writes) ✓
- No hex literals in any new CSS (uses `--bg-input`, `--accent`, `--text-*` etc.) ✓
- New IPC surfaces via typed wrapper in `src/lib/modifiers/api.ts`, not direct `invoke()` from components ✓
- `box-sizing: border-box` on width:100% inputs in P2 ✓
- TDD only on the genuine logic task (P5: translation + merge) ✓; wiring/UI tasks have no test step

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-03-gm-screen-plan-c-foundry-pushback.md`.

**Per project CLAUDE.md "lean execution override":**

- Sub-skill: `superpowers:subagent-driven-development`, but with the project-specific lean variant: ONE implementer subagent per task, no per-task spec-compliance reviewer, no per-task code-quality reviewer. Run `./scripts/verify.sh` between tasks. After ALL 9 tasks committed, run a SINGLE `code-review:code-review` against the full Plan C diff.
- TDD-on-demand: only P1 (one round-trip test) and P5 (full translation/merge test suite + new `get_modifier_by_id` helper) require failing-test-first. The other 7 tasks (P2–P4, P6–P9) are wiring/UI/docs — `verify.sh` is the gate.

Ready to dispatch P1?
