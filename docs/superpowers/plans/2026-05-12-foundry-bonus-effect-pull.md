# Foundry-Bonus Auto-Display & Per-Item Override Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire Foundry `system.bonuses[]` round-trip into the GM Screen so the card auto-displays the player's live merit bonuses, lets the GM save a per-item override that supersedes the live data, and pushes the override back surgically without trampling parallel player-added bonuses.

**Architecture:** Read-through render in the existing frontend `CharacterRow.svelte` (no new IPC for display); per-item override stored as a `CharacterModifier` with `binding: Advantage { item_id }` + a new `foundry_captured_labels` field that triggers a surgical-replace branch in the existing `do_push_to_foundry`. One schema migration adds the field; everything else extends existing code paths.

**Tech Stack:** Rust (sqlx + tokio + Tauri 2), Svelte 5 (runes), SvelteKit static SPA, SQLite. Inline `#[cfg(test)] mod tests` per Rust module per ARCHITECTURE.md §10.

**Spec:** `docs/superpowers/specs/2026-05-12-foundry-bonus-effect-pull-design.md`

---

## File structure

| File | Action | Responsibility |
|---|---|---|
| `src-tauri/migrations/0006_modifier_captured_labels.sql` | Create | Add `foundry_captured_labels_json` column to `character_modifiers`. |
| `src-tauri/src/shared/modifier.rs` | Modify | Add `foundry_captured_labels: Vec<String>` to `CharacterModifier` and `NewCharacterModifier`; update inline tests. |
| `src-tauri/src/db/modifier.rs` | Modify | Read/write the new column in `row_to_modifier`, `db_add`, `db_get`, `db_list`, `db_list_all`. Existing tests adjusted to compile against the new field. |
| `src-tauri/src/tools/gm_screen.rs` | Modify | Extend `do_push_to_foundry` with the surgical-replace branch keyed on `foundry_captured_labels` being non-empty. New helper `is_captured`. New tests for the surgical path. |
| `src/types.ts` | Modify | Mirror Rust changes: `foundryCapturedLabels: string[]` on `CharacterModifier` and `NewCharacterModifierInput`. |
| `src/lib/components/gm-screen/CharacterRow.svelte` | Modify | Filter `bonusesFor` to `activeWhen.check === 'always'`; compute conditional-skips; add `saveAsOverride` handler; compute mismatch-asterisk boolean; pass new props down. |
| `src/lib/components/gm-screen/ModifierCard.svelte` | Modify | Add `conditionalsSkipped` prop + badge UI; add `showMismatch` prop + asterisk UI; add `onSaveAsOverride` prop + button UI (rendered on virtual cards only). |

---

## Task 1: Schema migration + Rust types

**Files:**
- Create: `src-tauri/migrations/0006_modifier_captured_labels.sql`
- Modify: `src-tauri/src/shared/modifier.rs`

**Tests:** required (round-trip serialization of the new field).

- [ ] **Step 1: Create the migration**

Create file `src-tauri/migrations/0006_modifier_captured_labels.sql` with content:

```sql
-- Adds the foundry_captured_labels JSON column to character_modifiers.
-- Default '[]' = empty array, meaning "hand-rolled modifier, additive push".
-- Non-empty = "saved override from a Foundry bonus, surgical push".
ALTER TABLE character_modifiers
    ADD COLUMN foundry_captured_labels_json TEXT NOT NULL DEFAULT '[]';
```

- [ ] **Step 2: Add the field to `CharacterModifier` and `NewCharacterModifier`**

In `src-tauri/src/shared/modifier.rs`, modify the two structs:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterModifier {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub is_active: bool,
    pub is_hidden: bool,
    pub origin_template_id: Option<i64>,
    /// Source labels (`bonus.source` strings) that this modifier "captured"
    /// when it was created via "Save as local override". Non-empty marks
    /// this row as a Foundry override → push becomes surgical-replace
    /// (drops bonuses whose source ∈ this list before appending ours).
    /// Empty = hand-rolled modifier with additive push semantics.
    #[serde(default)]
    pub foundry_captured_labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewCharacterModifier {
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub origin_template_id: Option<i64>,
    #[serde(default)]
    pub foundry_captured_labels: Vec<String>,
}
```

- [ ] **Step 3: Add a round-trip test for the new field**

Append to the existing `#[cfg(test)] mod tests` block at the bottom of `src-tauri/src/shared/modifier.rs`:

```rust
    #[test]
    fn character_modifier_captured_labels_round_trip_json() {
        let json = serde_json::json!({
            "id": 7,
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Resilience",
            "description": "",
            "effects": [],
            "binding": { "kind": "advantage", "itemId": "merit-1" },
            "tags": [],
            "isActive": true,
            "isHidden": false,
            "originTemplateId": null,
            "foundryCapturedLabels": ["Buff Modifier", "Fortify"],
            "createdAt": "2026-05-12 00:00:00",
            "updatedAt": "2026-05-12 00:00:00",
        });
        let m: CharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert_eq!(m.foundry_captured_labels, vec!["Buff Modifier", "Fortify"]);
        let round_trip = serde_json::to_value(&m).expect("serialize");
        assert_eq!(round_trip["foundryCapturedLabels"], serde_json::json!(["Buff Modifier", "Fortify"]));
    }

    #[test]
    fn character_modifier_missing_captured_labels_defaults_to_empty() {
        // Legacy rows from before the migration / IPC payloads without the field.
        let json = serde_json::json!({
            "id": 1,
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Legacy",
            "description": "",
            "effects": [],
            "binding": { "kind": "free" },
            "tags": [],
            "isActive": false,
            "isHidden": false,
            "originTemplateId": null,
            "createdAt": "2026-05-12 00:00:00",
            "updatedAt": "2026-05-12 00:00:00",
        });
        let m: CharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert!(m.foundry_captured_labels.is_empty());
    }

    #[test]
    fn new_character_modifier_captured_labels_round_trip_json() {
        let json = serde_json::json!({
            "source": "foundry",
            "sourceId": "actor-x",
            "name": "Resilience",
            "description": "",
            "effects": [],
            "binding": { "kind": "advantage", "itemId": "merit-1" },
            "tags": [],
            "originTemplateId": null,
            "foundryCapturedLabels": ["Buff Modifier"],
        });
        let n: NewCharacterModifier = serde_json::from_value(json).expect("deserialize");
        assert_eq!(n.foundry_captured_labels, vec!["Buff Modifier"]);
    }
```

- [ ] **Step 4: Run cargo test to verify the new tests pass and migration compiles**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --no-run`
Expected: builds successfully.

Run: `cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests::character_modifier_captured_labels_round_trip_json modifier::tests::character_modifier_missing_captured_labels_defaults_to_empty modifier::tests::new_character_modifier_captured_labels_round_trip_json`
Expected: 3 passed.

- [ ] **Step 5: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: all three gates pass.

> Note: existing tests in `src-tauri/src/db/modifier.rs` and `src-tauri/src/tools/gm_screen.rs` construct `CharacterModifier` directly with struct-literal syntax. They will NOT compile after this task because the new field is non-Default; they get fixed in Task 2 and Task 3. The `--no-run` and per-test invocations above sidestep this — Task 1 commits only after the SQL migration, struct, and new round-trip tests build/pass in isolation via `cargo check`. If `./scripts/verify.sh` fails because of those downstream compile errors, this is the expected blocker — proceed to Task 2 in the same session and re-run verify.sh after Task 2.

Actually — to keep each task verify-clean, **also update the downstream test fixtures in this same task** rather than splitting them. The next sub-steps cover that.

- [ ] **Step 6: Update existing fixtures in `src-tauri/src/db/modifier.rs` to include the new field**

The existing tests construct `CharacterModifier` with struct-literal syntax at multiple sites; add `foundry_captured_labels: vec![]` to each. Locations (line numbers approximate — search for `origin_template_id: None,` and add the new field on the following line):

Around `src-tauri/src/db/modifier.rs:457` and `src-tauri/src/db/modifier.rs:666` — each has a `CharacterModifier { ... origin_template_id: None, created_at: ..., updated_at: ... }` literal. Insert `foundry_captured_labels: vec![],` immediately after `origin_template_id: None,`.

Also in the same file, the existing `row_to_modifier` function near line 22 will need a column read for the new column — DO NOT add it yet, that's Task 2. For now the migration + struct field exists; `row_to_modifier` reading the column comes in Task 2.

Wait — the `row_to_modifier` call sites must compile too. Since `row_to_modifier` constructs a `CharacterModifier` via struct-literal AND the field is non-Default, `row_to_modifier` MUST be updated in the same commit. Do this minimal fix here (just to compile):

In `src-tauri/src/db/modifier.rs`, update `row_to_modifier` to add the field with a placeholder:

```rust
fn row_to_modifier(r: &sqlx::sqlite::SqliteRow) -> Result<CharacterModifier, String> {
    let source_str: String = r.get("source");
    let source = str_to_source(&source_str)
        .ok_or_else(|| format!("db/modifier.list: unknown source '{source_str}'"))?;
    let effects_json: String = r.get("effects_json");
    let effects: Vec<ModifierEffect> = serde_json::from_str(&effects_json)
        .map_err(|e| format!("db/modifier.list: effects deserialize: {e}"))?;
    let binding_json: String = r.get("binding_json");
    let binding: ModifierBinding = serde_json::from_str(&binding_json)
        .map_err(|e| format!("db/modifier.list: binding deserialize: {e}"))?;
    let tags_json: String = r.get("tags_json");
    let tags: Vec<String> = serde_json::from_str(&tags_json)
        .map_err(|e| format!("db/modifier.list: tags deserialize: {e}"))?;
    let captured_json: String = r.try_get("foundry_captured_labels_json").unwrap_or_else(|_| "[]".to_string());
    let foundry_captured_labels: Vec<String> = serde_json::from_str(&captured_json)
        .map_err(|e| format!("db/modifier.list: captured labels deserialize: {e}"))?;
    Ok(CharacterModifier {
        id: r.get("id"),
        source,
        source_id: r.get("source_id"),
        name: r.get("name"),
        description: r.get("description"),
        effects,
        binding,
        tags,
        is_active: r.get::<bool, _>("is_active"),
        is_hidden: r.get::<bool, _>("is_hidden"),
        origin_template_id: r.get("origin_template_id"),
        foundry_captured_labels,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}
```

The `try_get` with fallback to `"[]"` makes the read robust to the migration race during dev (in-memory test DBs that run the migration cleanly will have the column; defensive fallback covers any edge case). After the migration, `r.get("foundry_captured_labels_json")` would work too — `try_get` is used here only as belt-and-suspenders.

- [ ] **Step 7: Update existing fixtures in `src-tauri/src/tools/gm_screen.rs`**

Find the `CanonicalCharacter { ... }` literal in `cached_actor_with_item` (around line ~432 of the test module). It doesn't include `CharacterModifier`, so no change there. But verify there's no `CharacterModifier { ... }` literal anywhere in `gm_screen.rs` — search for `CharacterModifier {`. None exist directly; the test module uses `NewCharacterModifier { ... }` which DOES need updating.

In `src-tauri/src/tools/gm_screen.rs` at the `NewCharacterModifier { ... }` literal inside `push_errors_when_foundry_disconnected_even_with_stale_cache`:

```rust
let new = NewCharacterModifier {
    source: SourceKind::Foundry,
    source_id: "actor-x".to_string(),
    name: "Stale Buff".to_string(),
    description: String::new(),
    effects: vec![pool_effect(2, vec!["attributes.strength"])],
    binding: ModifierBinding::Advantage {
        item_id: "merit-1".to_string(),
    },
    tags: vec![],
    origin_template_id: None,
    foundry_captured_labels: vec![],   // NEW LINE
};
```

- [ ] **Step 8: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/migrations/0006_modifier_captured_labels.sql src-tauri/src/shared/modifier.rs src-tauri/src/db/modifier.rs src-tauri/src/tools/gm_screen.rs
git commit -m "$(cat <<'EOF'
feat(modifier): add foundry_captured_labels field for override identity

Adds the foundry_captured_labels Vec<String> column + struct field that
distinguishes a Foundry-bonus override (non-empty captured labels →
surgical push) from a hand-rolled modifier (empty → additive push).

Refs the foundry-bonus-effect-pull spec; subsequent tasks implement the
DB write path and the surgical-push branch.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: DB write path for captured labels

**Files:**
- Modify: `src-tauri/src/db/modifier.rs`

**Tests:** none new — existing tests round-trip through INSERT/SELECT and cover the path.

- [ ] **Step 1: Update `db_add` to persist the new column**

In `src-tauri/src/db/modifier.rs`, replace the `db_add` function body. The INSERT must include the new column:

```rust
pub(crate) async fn db_add(
    pool: &SqlitePool,
    input: NewCharacterModifier,
) -> Result<CharacterModifier, String> {
    if input.name.trim().is_empty() {
        return Err("db/modifier.add: empty name".to_string());
    }
    let effects_json = serde_json::to_string(&input.effects)
        .map_err(|e| format!("db/modifier.add: serialize effects: {e}"))?;
    let binding_json = serde_json::to_string(&input.binding)
        .map_err(|e| format!("db/modifier.add: serialize binding: {e}"))?;
    let tags_json = serde_json::to_string(&input.tags)
        .map_err(|e| format!("db/modifier.add: serialize tags: {e}"))?;
    let captured_labels_json = serde_json::to_string(&input.foundry_captured_labels)
        .map_err(|e| format!("db/modifier.add: serialize captured labels: {e}"))?;

    let result = sqlx::query(
        "INSERT INTO character_modifiers
         (source, source_id, name, description, effects_json, binding_json, tags_json,
          origin_template_id, foundry_captured_labels_json)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(source_to_str(&input.source))
    .bind(&input.source_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&effects_json)
    .bind(&binding_json)
    .bind(&tags_json)
    .bind(input.origin_template_id)
    .bind(&captured_labels_json)
    .execute(pool)
    .await
    .map_err(|e| format!("db/modifier.add: {e}"))?;
    let id = result.last_insert_rowid();
    db_get(pool, id).await
}
```

- [ ] **Step 2: Update SELECTs in `db_list`, `db_list_all`, `db_get` to include the new column**

For each of the three functions in `src-tauri/src/db/modifier.rs`, update the SELECT column list to include `foundry_captured_labels_json`. The select strings appear at approximately lines 60, 77, and 154 — each has a SELECT that lists columns. Replace each occurrence to add the new column at the end of the field list:

Find:
```rust
"SELECT id, source, source_id, name, description, effects_json,
        binding_json, tags_json, is_active, is_hidden,
        origin_template_id, created_at, updated_at
 FROM character_modifiers
```

Replace with:
```rust
"SELECT id, source, source_id, name, description, effects_json,
        binding_json, tags_json, is_active, is_hidden,
        origin_template_id, foundry_captured_labels_json, created_at, updated_at
 FROM character_modifiers
```

Apply this replacement to all three SELECTs (`db_list`, `db_list_all`, `db_get`).

- [ ] **Step 3: Update `db_materialize_advantage`'s INSERT to include the new column with default `'[]'`**

Find the `db_materialize_advantage` function in `src-tauri/src/db/modifier.rs` (around line 306). The function does an INSERT for a freshly-materialized advantage row. Since materialize is the "hand-rolled empty" path (not an override), `foundry_captured_labels` stays empty.

Locate the INSERT statement inside `db_materialize_advantage` (around line 336). It looks like:

```rust
let result = sqlx::query(
    "INSERT INTO character_modifiers
     (source, source_id, name, description, effects_json, binding_json, tags_json,
      origin_template_id)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
)
```

Replace with the same shape but also including `foundry_captured_labels_json`:

```rust
let result = sqlx::query(
    "INSERT INTO character_modifiers
     (source, source_id, name, description, effects_json, binding_json, tags_json,
      origin_template_id, foundry_captured_labels_json)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, '[]')"
)
```

The literal `'[]'` is the empty JSON array — materialize creates hand-rolled rows with no captured labels.

There is a second INSERT in the same function (around line 386) — the test-only no-conflict path. Apply the same change there.

- [ ] **Step 4: Update the seed/raw INSERTs in test fixtures of `src-tauri/src/db/modifier.rs`**

The test fixtures use raw INSERTs without the new column (around lines 418, 430, 432, 434). The DEFAULT `'[]'` clause in the migration means these still work — SQLite will fill the column with `'[]'` for these inserts. Verify by reading the migration: yes, `DEFAULT '[]'` is set. No change needed to these test-only fixtures.

- [ ] **Step 5: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: green. All existing tests in `db/modifier.rs` pass — the round-trip through INSERT (with empty captured labels via Task 1's struct default) and SELECT (including the new column) preserves correctness.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db/modifier.rs
git commit -m "$(cat <<'EOF'
feat(db): persist foundry_captured_labels in character_modifiers

Wires the new column into row_to_modifier reads (already in place from
Task 1), db_add INSERTs, and the three SELECTs. Hand-rolled rows persist
as empty `[]`; override rows from "Save as local override" will populate
with the captured source labels (next tasks).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Surgical-push branch in `do_push_to_foundry`

**Files:**
- Modify: `src-tauri/src/tools/gm_screen.rs`

**Tests:** required — surgical merge has multiple branches (own-tag, captured-label, always-only, conditional preservation).

- [ ] **Step 1: Add `is_captured` helper near `is_ours`**

In `src-tauri/src/tools/gm_screen.rs`, insert immediately after the `is_ours` function:

```rust
/// True iff the bonus's `source` matches one of the captured labels AND
/// the bonus is `activeWhen.check == "always"`. Used by the surgical-push
/// branch to identify which non-own bonuses to remove on push.
///
/// Conditional bonuses are preserved (untouched) — even if their label
/// happens to be in `captured_labels`, they survive because the GM never
/// captured a non-always bonus (the read-through walker filters those out).
/// Belt-and-suspenders: the activeWhen check here defends against the
/// edge case where a label was reused for both an always and a conditional
/// bonus on the same item.
fn is_captured(bonus: &Value, captured_labels: &[String]) -> bool {
    if captured_labels.is_empty() {
        return false;
    }
    let Some(source) = bonus.get("source").and_then(|v| v.as_str()) else {
        return false;
    };
    if !captured_labels.iter().any(|label| label == source) {
        return false;
    }
    let check = bonus
        .get("activeWhen")
        .and_then(|aw| aw.get("check"))
        .and_then(|c| c.as_str())
        .unwrap_or("always"); // missing activeWhen ⇒ treat as always
    check == "always"
}
```

- [ ] **Step 2: Extend `merge_bonuses` to take captured labels**

Replace the existing `merge_bonuses` function in `src-tauri/src/tools/gm_screen.rs` with:

```rust
/// Filter existing bonuses, then append `ours`.
///
/// Always drops bonuses tagged as ours (`is_ours`). If `captured_labels` is
/// non-empty (saved override path), ALSO drops bonuses where `is_captured`
/// matches — i.e. the always-active player-added bonuses the override
/// captured at save time. Player-added bonuses whose label is NOT in the
/// captured set (e.g. added after the override was saved) are preserved.
fn merge_bonuses(
    existing: &[Value],
    modifier_id: i64,
    captured_labels: &[String],
    ours: Vec<Value>,
) -> Vec<Value> {
    let mut out: Vec<Value> = existing
        .iter()
        .filter(|b| !is_ours(b, modifier_id) && !is_captured(b, captured_labels))
        .cloned()
        .collect();
    out.extend(ours);
    out
}
```

- [ ] **Step 3: Update the call site in `do_push_to_foundry`**

In `src-tauri/src/tools/gm_screen.rs`, find the line in `do_push_to_foundry` that calls `merge_bonuses(&existing, m.id, new_bonuses.clone())` (currently around `let merged = merge_bonuses(...)`). Replace with:

```rust
    let merged = merge_bonuses(&existing, m.id, &m.foundry_captured_labels, new_bonuses.clone());
```

- [ ] **Step 4: Add surgical-merge tests**

Append the following tests to the existing `#[cfg(test)] mod tests` block in `src-tauri/src/tools/gm_screen.rs`:

```rust
    #[test]
    fn is_captured_matches_label_only_when_always_active() {
        let labels = vec!["Buff Modifier".to_string()];

        let always_match = json!({
            "source": "Buff Modifier",
            "value": 2,
            "paths": ["attributes.strength"],
            "activeWhen": { "check": "always", "path": "", "value": "" }
        });
        assert!(is_captured(&always_match, &labels), "always + matching label captured");

        let conditional_match = json!({
            "source": "Buff Modifier",
            "value": 2,
            "paths": ["attributes.strength"],
            "activeWhen": { "check": "isEqual", "path": "hunger.value", "value": "5" }
        });
        assert!(!is_captured(&conditional_match, &labels), "conditional bonus not captured even if label matches");

        let always_other_label = json!({
            "source": "Frenzy Bonus",
            "value": 1,
            "paths": ["attributes.composure"],
            "activeWhen": { "check": "always", "path": "", "value": "" }
        });
        assert!(!is_captured(&always_other_label, &labels), "non-matching label never captured");

        let missing_active_when_treated_as_always = json!({
            "source": "Buff Modifier",
            "value": 2,
            "paths": ["attributes.strength"]
        });
        assert!(is_captured(&missing_active_when_treated_as_always, &labels),
                "missing activeWhen defaults to always (Foundry-side fallback)");
    }

    #[test]
    fn is_captured_empty_labels_short_circuits() {
        let bonus = json!({
            "source": "Anything",
            "value": 1,
            "paths": [],
            "activeWhen": { "check": "always", "path": "", "value": "" }
        });
        assert!(!is_captured(&bonus, &[]));
    }

    #[test]
    fn merge_surgical_removes_captured_labels() {
        let existing = vec![
            json!({                                                 // captured player bonus — removed
                "source": "Buff Modifier", "value": 2, "paths": ["attributes.strength"],
                "activeWhen": { "check": "always", "path": "", "value": "" }
            }),
            json!({                                                 // new player bonus (not captured) — preserved
                "source": "Frenzy Bonus", "value": 1, "paths": ["attributes.composure"],
                "activeWhen": { "check": "always", "path": "", "value": "" }
            }),
            json!({                                                 // our prior push — removed
                "source": "GM Screen #7: Override", "value": 3, "paths": ["attributes.strength"],
                "activeWhen": { "check": "always", "path": "", "value": "" }
            }),
            json!({                                                 // captured-label conditional — preserved (activeWhen != always)
                "source": "Buff Modifier", "value": 5, "paths": ["attributes.strength"],
                "activeWhen": { "check": "isEqual", "path": "hunger.value", "value": "5" }
            }),
        ];
        let labels = vec!["Buff Modifier".to_string()];
        let ours = vec![json!({
            "source": "GM Screen #7: Override", "value": 3, "paths": ["attributes.strength"],
            "activeWhen": { "check": "always", "path": "", "value": "" }
        })];
        let merged = merge_bonuses(&existing, 7, &labels, ours);
        assert_eq!(merged.len(), 3, "kept new-player + conditional + new ours");
        assert!(merged.iter().any(|b| b["source"] == "Frenzy Bonus"));
        assert!(merged.iter().any(|b| b["source"] == "Buff Modifier" && b["activeWhen"]["check"] == "isEqual"));
        assert!(merged.iter().any(|b| b["source"] == "GM Screen #7: Override"));
        assert!(!merged.iter().any(|b| b["source"] == "Buff Modifier" && b["activeWhen"]["check"] == "always"),
                "captured always-active label was removed");
    }

    #[test]
    fn merge_additive_when_no_captured_labels() {
        // Empty captured_labels ⇒ behaves exactly like the legacy additive merge:
        // only own-tagged bonuses removed, all player bonuses preserved.
        let existing = vec![
            json!({"source": "Player Buff", "value": 1, "paths": ["x"],
                   "activeWhen": { "check": "always", "path": "", "value": "" }}),
            json!({"source": "GM Screen #5: A", "value": 2, "paths": ["y"],
                   "activeWhen": { "check": "always", "path": "", "value": "" }}),
        ];
        let ours = vec![json!({"source": "GM Screen #5: A", "value": 99, "paths": ["new"],
                               "activeWhen": { "check": "always", "path": "", "value": "" }})];
        let merged = merge_bonuses(&existing, 5, &[], ours);
        assert_eq!(merged.len(), 2);
        assert!(merged.iter().any(|b| b["source"] == "Player Buff"));
        let ours_new = merged
            .iter()
            .find(|b| b["source"] == "GM Screen #5: A")
            .unwrap();
        assert_eq!(ours_new["value"], 99);
    }
```

- [ ] **Step 5: Fix the EXISTING `merge_filters_only_our_modifier_id`, `merge_with_no_existing_bonuses_just_appends`, and `merge_idempotent_under_repeated_push` tests for the new signature**

The pre-existing tests call `merge_bonuses(&existing, 5, ours)` with three arguments. The new signature is `merge_bonuses(&existing, modifier_id, captured_labels, ours)`. Add `&[]` (empty captured labels) as the third arg for backward-compat:

Find each existing test's `merge_bonuses(` invocation and add `&[]` as the captured-labels argument:

```rust
    #[test]
    fn merge_filters_only_our_modifier_id() {
        // existing setup unchanged...
        let merged = merge_bonuses(&existing, 5, &[], ours);
        // existing assertions unchanged...
    }

    #[test]
    fn merge_with_no_existing_bonuses_just_appends() {
        let merged = merge_bonuses(&[], 1, &[], vec![json!({"source": "GM Screen #1: X"})]);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn merge_idempotent_under_repeated_push() {
        let initial: Vec<Value> = vec![];
        let ours_v1 = vec![json!({"source": "GM Screen #2: X", "value": 1, "paths": ["a"]})];
        let after_first = merge_bonuses(&initial, 2, &[], ours_v1);
        let ours_v2 = vec![json!({"source": "GM Screen #2: X", "value": 1, "paths": ["a"]})];
        let after_second = merge_bonuses(&after_first, 2, &[], ours_v2);
        assert_eq!(after_first, after_second, "re-push yields the same array");
    }
```

- [ ] **Step 6: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/tools/gm_screen.rs
git commit -m "$(cat <<'EOF'
feat(gm-screen): surgical push branch using foundry_captured_labels

When a modifier carries non-empty foundry_captured_labels, push drops
bonuses whose source matches a captured label AND is activeWhen=always,
in addition to our prior own-tagged pushes. Player-added bonuses with
non-captured labels (added after the override was saved) survive. Hand-
rolled modifiers (empty captured labels) keep the additive behavior.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: TS type mirror

**Files:**
- Modify: `src/types.ts`

**Tests:** none new — `npm run check` is the gate.

- [ ] **Step 1: Add `foundryCapturedLabels` to `CharacterModifier`**

In `src/types.ts`, find the `CharacterModifier` interface and add the new field immediately after `originTemplateId`:

```ts
export interface CharacterModifier {
  id: number;
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  isActive: boolean;
  isHidden: boolean;
  originTemplateId: number | null;
  /**
   * Source labels captured from the Foundry item's `system.bonuses[]` at
   * "Save as local override" time. Non-empty marks this modifier as a
   * Foundry override → push uses surgical replace (removes captured-label
   * bonuses + our own prior pushes before appending). Empty = hand-rolled
   * modifier with additive push.
   */
  foundryCapturedLabels: string[];
  createdAt: string;
  updatedAt: string;
}
```

- [ ] **Step 2: Add `foundryCapturedLabels` to `NewCharacterModifierInput`**

In `src/types.ts`, find `NewCharacterModifierInput`:

```ts
export interface NewCharacterModifierInput {
  source: SourceKind;
  sourceId: string;
  name: string;
  description: string;
  effects: ModifierEffect[];
  binding: ModifierBinding;
  tags: string[];
  originTemplateId: number | null;
  foundryCapturedLabels: string[];   // NEW LINE — empty array = hand-rolled
}
```

- [ ] **Step 3: Update existing callers that construct `NewCharacterModifierInput` to include the new field**

Find all sites in `src/` that build a `NewCharacterModifierInput` literal:

Run: `grep -rn "addCharacterModifier\|modifiers.add({" --include="*.ts" --include="*.svelte" src/`

Each call site that constructs the input object needs `foundryCapturedLabels: []` added. Known sites (based on the existing `addFreeModifier` in `CharacterRow.svelte:225`):

In `src/lib/components/gm-screen/CharacterRow.svelte`, find `addFreeModifier`:

```ts
  async function addFreeModifier(): Promise<void> {
    await modifiers.add({
      source: character.source,
      sourceId: character.source_id,
      name: 'New modifier',
      description: '',
      effects: [],
      binding: { kind: 'free' },
      tags: [],
      originTemplateId: null,
      foundryCapturedLabels: [],   // NEW LINE
    });
  }
```

Search for and update any other call sites the grep finds. Likely sites: `src/lib/components/gm-screen/StatusPaletteDock.svelte` (applies templates), `src/lib/components/gm-screen/ModifierEffectEditor.svelte` (saveEditor flow — but that calls `update`, not `add`, so likely unaffected). Confirm by inspection.

- [ ] **Step 4: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: green. `npm run check` validates that all `NewCharacterModifierInput` constructions are satisfied.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src/lib/components/gm-screen/CharacterRow.svelte $(grep -rl "addCharacterModifier\|modifiers.add({" --include="*.ts" --include="*.svelte" src/ | tr '\n' ' ')
git commit -m "$(cat <<'EOF'
feat(types): mirror foundryCapturedLabels in TS CharacterModifier shape

Adds foundryCapturedLabels: string[] to both CharacterModifier and
NewCharacterModifierInput, and updates existing call sites to pass [].
Frontend behavior unchanged in this commit — the override-creation
button arrives in the next task.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Frontend — filter `activeWhen: always` + conditional badge

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte`

**Tests:** none new — type-check + manual smoke in `npm run tauri dev`.

- [ ] **Step 1: Update `bonusesFor` in CharacterRow to filter to `activeWhen: always`**

In `src/lib/components/gm-screen/CharacterRow.svelte`, locate the `bonusesFor` function. Currently it filters out own-tagged bonuses; we need to also keep only always-active ones AND produce a parallel list of skipped conditionals.

Replace the existing `bonusesFor` with TWO functions — `bonusesFor` (returns always-active only) and `conditionalsFor` (returns the dropped conditionals):

```ts
  /** Read sheet-attached bonuses (system.bonuses[]) off a Foundry feature
   *  item by its _id, keeping only `activeWhen.check === 'always'` (and
   *  bonuses with missing/null activeWhen — treated as always by Foundry).
   *  Filters out own-pushed bonuses. Returns [] when the item is gone or
   *  has no qualifying bonuses. */
  function bonusesFor(itemId: string): FoundryItemBonus[] {
    const item = advantageItems.find(it => it._id === itemId);
    if (!item) return [];
    const raw = (item.system as Record<string, unknown>)?.bonuses;
    if (!Array.isArray(raw)) return [];
    return (raw as FoundryItemBonus[]).filter(b => {
      // Drop own-pushed bonuses (would loop on re-pull).
      const src = b.source ?? '';
      for (const prefix of liveTagPrefixes) {
        if (src === prefix || src.startsWith(prefix + ':')) return false;
      }
      // Keep only always-active. Missing activeWhen treated as always per
      // Foundry behavior (defensive — WoD5e always writes activeWhen but
      // legacy data or other sources may omit it).
      const check = b.activeWhen?.check ?? 'always';
      return check === 'always';
    });
  }

  /** Returns the conditional bonuses (activeWhen.check != 'always') for an
   *  item, after own-push filtering. Used to render the "(N conditionals)"
   *  badge. */
  function conditionalsFor(itemId: string): FoundryItemBonus[] {
    const item = advantageItems.find(it => it._id === itemId);
    if (!item) return [];
    const raw = (item.system as Record<string, unknown>)?.bonuses;
    if (!Array.isArray(raw)) return [];
    return (raw as FoundryItemBonus[]).filter(b => {
      const src = b.source ?? '';
      for (const prefix of liveTagPrefixes) {
        if (src === prefix || src.startsWith(prefix + ':')) return false;
      }
      const check = b.activeWhen?.check ?? 'always';
      return check !== 'always';
    });
  }
```

- [ ] **Step 2: Pass `conditionalsSkipped` to ModifierCard**

Still in `src/lib/components/gm-screen/CharacterRow.svelte`, find the `{#each visibleCards as entry, i ...}` block that renders `<ModifierCard ...>`. Locate the `bonuses=` prop. Immediately after the `bonuses` prop, add a `conditionalsSkipped` prop:

```svelte
        bonuses={entry.kind === 'virtual'
          ? bonusesFor(entry.virt.item._id)
          : entry.mod.binding.kind === 'advantage'
            ? bonusesFor(entry.mod.binding.item_id)
            : []}
        conditionalsSkipped={entry.kind === 'virtual'
          ? conditionalsFor(entry.virt.item._id)
          : entry.mod.binding.kind === 'advantage'
            ? conditionalsFor(entry.mod.binding.item_id)
            : []}
        onToggleActive={() => handleToggleActive(entry)}
```

- [ ] **Step 3: Add the `conditionalsSkipped` prop in ModifierCard**

In `src/lib/components/gm-screen/ModifierCard.svelte`, extend the `Props` interface and the destructure:

```ts
  interface Props {
    modifier: CharacterModifier;
    isVirtual?: boolean;
    isStale?: boolean;
    bonuses?: FoundryItemBonus[];
    /**
     * Conditional bonuses (activeWhen.check != 'always') skipped from the
     * read-through. Rendered as a small "(N conditionals)" badge with a
     * tooltip listing the labels + their `activeWhen.check` reasons.
     */
    conditionalsSkipped?: FoundryItemBonus[];
    canPush?: boolean;
    onPush?: () => void;
    canReset?: boolean;
    onReset?: () => void;
    onToggleActive: () => void;
    onOpenEditor: (anchor: HTMLElement) => void;
    onHide: () => void;
    originTemplateName?: string | null;
  }

  let {
    modifier, isVirtual = false, isStale = false, bonuses = [],
    conditionalsSkipped = [],
    canPush = false, onPush,
    canReset = false, onReset,
    onToggleActive, onOpenEditor, onHide,
    originTemplateName = null,
  }: Props = $props();
```

- [ ] **Step 4: Render the conditionals badge in ModifierCard**

In the `<div class="modifier-card">` template, immediately AFTER the existing `{#if bonuses.length > 0}` block (around line ~80 in the template) and BEFORE the `<div class="effects">` block, insert:

```svelte
  {#if conditionalsSkipped.length > 0}
    <p
      class="conditionals-badge"
      title={conditionalsSkipped
        .map(b => `${b.source ?? '(unnamed)'} — ${b.activeWhen?.check ?? '?'}`)
        .join('\n')}
    >
      ({conditionalsSkipped.length} conditional{conditionalsSkipped.length === 1 ? '' : 's'})
    </p>
  {/if}
```

- [ ] **Step 5: Add the badge CSS to ModifierCard's `<style>` block**

In the `<style>` block of `src/lib/components/gm-screen/ModifierCard.svelte`, add the new class. Insert near the existing `.bonus` / `.bonus-source` rules:

```css
  .conditionals-badge {
    margin: 0;
    font-size: 0.6rem;
    color: var(--text-muted);
    font-style: italic;
    cursor: help;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
```

- [ ] **Step 6: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 7: Smoke-test in dev**

Run: `npm run tauri dev`

Manual checks (connect Foundry first):
- A merit with one always-active bonus shows the bonus line; no conditionals badge.
- A merit with one conditional bonus (e.g. activeWhen.check = 'isEqual') shows the badge, no bonus line.
- A merit with both shows the bonus line + a "(1 conditional)" badge with tooltip describing it.

End the dev server. If smoke tests reveal regressions, fix inline and re-run verify.sh before commit.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): filter read-through bonuses to activeWhen=always

bonusesFor now drops bonuses with activeWhen.check != 'always'; a sibling
conditionalsFor surfaces the dropped set as a "(N conditionals)" badge
on the ModifierCard with a tooltip listing each skipped label and reason.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Save-as-override button + mismatch asterisk

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte`
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte`

**Tests:** none new — type-check + manual smoke.

- [ ] **Step 1: Add `saveAsOverride` handler in CharacterRow**

In `src/lib/components/gm-screen/CharacterRow.svelte`, immediately AFTER the `materialize` function, insert:

```ts
  /**
   * Save-as-local-override action. Distinct from `materialize`:
   *   - materialize: creates an empty modifier (no effects, no captured
   *     labels) on first user engagement; subsequent edits build up local
   *     effects from scratch.
   *   - saveAsOverride: snapshots the current always-active bonuses on the
   *     item into a CharacterModifier whose effects mirror the bonuses
   *     AND whose foundryCapturedLabels record the source-label set.
   *     Push then becomes surgical.
   */
  async function saveAsOverride(virt: VirtualCard): Promise<CharacterModifier> {
    const sourceBonuses = bonusesFor(virt.item._id);
    const effects: ModifierEffect[] = sourceBonuses.map(b => ({
      kind: 'pool',
      scope: null,
      delta: b.value,
      note: null,
      paths: b.paths,
    }));
    const capturedLabels = sourceBonuses.map(b => b.source ?? '');
    const created = await modifiers.add({
      source: character.source,
      sourceId: character.source_id,
      name: virt.name,
      description: virt.description,
      effects,
      binding: { kind: 'advantage', item_id: virt.item._id },
      tags: [],
      originTemplateId: null,
      foundryCapturedLabels: capturedLabels,
    });
    // Flip is_active=true so the override is immediately applied in renders
    // that consume active modifiers (active-effects summary, deltas, etc.).
    await modifiers.setActive(created.id, true);
    return created;
  }
```

- [ ] **Step 2: Compute the mismatch flag and pass it down**

Still in `src/lib/components/gm-screen/CharacterRow.svelte`, add a helper function that computes whether a materialized override is out of sync with its live bonuses. Insert immediately AFTER `conditionalsFor`:

```ts
  /** Stable string key for a (value, paths) tuple, used for unordered set
   *  equality. `paths` ordering within one tuple IS significant (spec §3.3).
   */
  function effectKey(value: number, paths: string[]): string {
    return JSON.stringify([value, paths]);
  }

  /** True when this materialized modifier is a Foundry override
   *  (foundryCapturedLabels non-empty) AND its Pool effects don't match
   *  the item's current always-active live bonuses (own-pushes excluded).
   *  See spec §3.3.
   */
  function isOverrideOutOfSync(mod: CharacterModifier): boolean {
    if (mod.foundryCapturedLabels.length === 0) return false;
    if (mod.binding.kind !== 'advantage') return false;
    const live = bonusesFor(mod.binding.item_id);
    const liveSet = new Set(live.map(b => effectKey(b.value, b.paths)));
    const saved = mod.effects.filter(e => e.kind === 'pool');
    const savedSet = new Set(saved.map(e => effectKey(e.delta ?? 0, e.paths ?? [])));
    if (liveSet.size !== savedSet.size) return true;
    for (const k of liveSet) if (!savedSet.has(k)) return true;
    return false;
  }
```

- [ ] **Step 3: Pass mismatch flag + save-as-override handler to ModifierCard**

In the `{#each visibleCards as entry, i ...}` block in `CharacterRow.svelte`, extend the `<ModifierCard>` props. Find the existing props block and add two new props at the end (right before the closing `/>`):

```svelte
      <ModifierCard
        modifier={entry.kind === 'virtual' ? { ... } : entry.mod}
        isVirtual={entry.kind === 'virtual'}
        isStale={entry.kind === 'materialized' && entry.isStale}
        bonuses={...}
        conditionalsSkipped={...}
        onToggleActive={() => handleToggleActive(entry)}
        onHide={() => handleHide(entry)}
        onOpenEditor={(anchor) => openEditor(entry, anchor)}
        canPush={canPushFor(entry)}
        onPush={() => handlePush(entry)}
        canReset={canResetFor(entry)}
        onReset={() => handleReset(entry)}
        originTemplateName={...}
        showMismatch={entry.kind === 'materialized' ? isOverrideOutOfSync(entry.mod) : false}
        onSaveAsOverride={entry.kind === 'virtual'
          ? () => saveAsOverride(entry.virt).catch(err => console.error('[gm-screen] save-as-override failed:', err))
          : undefined}
      />
```

The exact insertion is: `showMismatch={...}` and `onSaveAsOverride={...}` go after `originTemplateName={...}` and before the self-closing `/>`. Leave the existing props (modifier, isVirtual, isStale, bonuses, conditionalsSkipped, onToggleActive, onHide, onOpenEditor, canPush, onPush, canReset, onReset, originTemplateName) unchanged.

- [ ] **Step 4: Add `showMismatch` and `onSaveAsOverride` props to ModifierCard**

In `src/lib/components/gm-screen/ModifierCard.svelte`, extend `Props` and the destructure to include the two new props:

```ts
  interface Props {
    modifier: CharacterModifier;
    isVirtual?: boolean;
    isStale?: boolean;
    bonuses?: FoundryItemBonus[];
    conditionalsSkipped?: FoundryItemBonus[];
    canPush?: boolean;
    onPush?: () => void;
    canReset?: boolean;
    onReset?: () => void;
    onToggleActive: () => void;
    onOpenEditor: (anchor: HTMLElement) => void;
    onHide: () => void;
    originTemplateName?: string | null;
    /**
     * True when this materialized modifier is a Foundry override
     * (foundryCapturedLabels non-empty) whose effects don't match the
     * item's current always-active live bonuses. Drives the yellow
     * mismatch asterisk.
     */
    showMismatch?: boolean;
    /**
     * "Save as local override" handler. When set (i.e. on virtual cards),
     * renders the save-as-override button. Clicking creates a
     * CharacterModifier with effects mirroring the current always-active
     * bonuses + captures their source labels.
     */
    onSaveAsOverride?: () => void;
  }

  let {
    modifier, isVirtual = false, isStale = false, bonuses = [],
    conditionalsSkipped = [],
    canPush = false, onPush,
    canReset = false, onReset,
    onToggleActive, onOpenEditor, onHide,
    originTemplateName = null,
    showMismatch = false,
    onSaveAsOverride,
  }: Props = $props();
```

- [ ] **Step 5: Render the mismatch asterisk + save-as-override button in ModifierCard**

In the `<div class="head">` block, find the `<span class="name">` and the existing `{#if isVirtual}` virtual-mark span. Add a NEW mismatch-mark span IMMEDIATELY AFTER the virtual-mark conditional and BEFORE the `{#if isStale}` stale span:

```svelte
  <div class="head">
    <span class="name" title={modifier.name}>
      {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}{#if showMismatch}<span class="mismatch-mark" title="Saved override differs from current Foundry bonus">*</span>{/if}
      {#if isStale}<span class="stale" title="Source merit removed">stale</span>{/if}
    </span>
    <button
      bind:this={cogEl}
      class="cog"
      title="Edit effects"
      onclick={() => cogEl && onOpenEditor(cogEl)}
    >⚙</button>
  </div>
```

Then in the `<div class="foot">` block, add a "Save as override" button between the toggle and push buttons. The button only renders if `onSaveAsOverride` is set:

```svelte
  <div class="foot">
    <button
      class="toggle"
      class:on={modifier.isActive}
      onclick={onToggleActive}
    >{modifier.isActive ? 'ON' : 'OFF'}</button>
    {#if onSaveAsOverride}
      <button
        class="save-override"
        title="Snapshot the live Foundry bonuses into a saved local override"
        aria-label="Save as local override"
        onclick={onSaveAsOverride}
      >💾</button>
    {/if}
    {#if canPush}
      <button
        class="push"
        title="Push these effects to the merit on Foundry"
        aria-label="Push effects to Foundry"
        onclick={onPush}
      >↑</button>
    {/if}
    {#if canReset}
      <button
        class="reset"
        title="Reset card — drops local effects/paths/tags. Foundry bonuses unaffected."
        aria-label="Reset card"
        onclick={onReset}
      >↺</button>
    {/if}
    <button
      class="hide"
      title={modifier.isHidden ? 'Show card again' : 'Hide card'}
      aria-label={modifier.isHidden ? 'Show card again' : 'Hide card'}
      onclick={onHide}
    >{modifier.isHidden ? '+' : '×'}</button>
  </div>
```

- [ ] **Step 6: Add CSS for the mismatch-mark and save-override button**

In the `<style>` block of `src/lib/components/gm-screen/ModifierCard.svelte`, add the new classes. Near `.virtual-mark`:

```css
  .virtual-mark { color: var(--accent-amber); margin-left: 0.15rem; }
  .mismatch-mark {
    color: var(--accent-amber);
    margin-left: 0.15rem;
    font-weight: 700;
    cursor: help;
  }
```

Near `.push`:

```css
  .save-override {
    background: var(--bg-input);
    color: var(--text-secondary);
    border: 1px solid var(--border-faint);
    border-radius: 0.3rem;
    padding: 0.15rem 0.45rem;
    font-size: 0.7rem;
    cursor: pointer;
    opacity: 0;
    transition: opacity 120ms ease, background 120ms ease, color 120ms ease;
  }
  .modifier-card:hover .save-override,
  .save-override:focus { opacity: 1; }
  .save-override:hover { background: var(--accent); color: var(--text-primary); border-color: var(--accent-bright); }
```

- [ ] **Step 7: Run verify.sh**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 8: Smoke-test in dev**

Run: `npm run tauri dev`

Manual checks (Foundry connected, a player actor with at least one merit with an always-active bonus):
1. **Virtual card visible** — merit with bonus shows virtual card with `*` mark, bonus line, "Save as override" button on hover.
2. **Save as override** — click the save button. Card flips to materialized, name appears without virtual-mark, effects section now shows a Pool effect mirroring the bonus, no mismatch asterisk (`live == saved`).
3. **Edit override** — open editor (cog), change the effect's `delta` from N to N+1, save. Asterisk appears (yellow `*` after the name).
4. **Push** — click push (↑). Confirm in Foundry's `system.bonuses[]` that the player's original bonus is gone, replaced by the GM-tagged bonus with the new value. Player bonuses with different labels on other items are unaffected.
5. **Reset** — click reset (↺). Card returns to virtual; the original player bonus is NOT restored in Foundry (documented limitation of surgical push — the saved override remembers the captured labels but does not roll back what it overwrote).
6. **Conditional bonus** — add a conditional bonus to the same merit in Foundry. After bridge refresh, the override card shows a "(1 conditional)" badge with tooltip listing it.

End the dev server. If regressions appear, fix inline and re-run verify.sh before commit.

- [ ] **Step 9: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): save-as-override action + mismatch asterisk

Adds a 💾 "Save as local override" button on virtual cards. Clicking
snapshots the item's current always-active bonuses into a new
CharacterModifier whose effects mirror the bonuses AND whose
foundryCapturedLabels record the captured source-label set (so push
becomes surgical, per Task 3). Adds a yellow asterisk on materialized
override cards whose effects don't match the current live bonuses.

Closes the foundry-bonus pull spec for v1.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review

**Spec coverage check** (against `docs/superpowers/specs/2026-05-12-foundry-bonus-effect-pull-design.md`):

| Spec section | Task that implements it |
|---|---|
| §3.1 Display path (read-through, filter own-tag + always) | Task 5 (bonusesFor filter) |
| §3.2 Saved-override path (override renders instead of read-through) | Existing flow + Task 6 saveAsOverride creates the row |
| §3.3 Yellow asterisk (live_set != saved_set, ordered paths) | Task 6 isOverrideOutOfSync + mismatch-mark span |
| §3.4 Conditional bonuses badge | Task 5 conditionalsFor + ModifierCard badge |
| §3.5 "Save as local override" button | Task 6 saveAsOverride + save-override button |
| §3.6 Push semantics (surgical when captured labels non-empty) | Task 3 is_captured + merge_bonuses extension |
| §3.7 Documented lingering asterisk | Behavior emerges naturally from Task 3 + Task 6 |
| §3.8 Disconnect behavior | Existing bridge code handles it; nothing to add |
| §4 Schema change | Task 1 migration + struct field |
| §5.1 Frontend pure JS | Task 5 + Task 6 (no new IPC) |
| §5.2 Backend changes scoped to listed files | Task 1-3 only touch those files |
| §5.3 No new commands | Confirmed — existing add_character_modifier reused |
| §6 UX surface (synthesized card, mismatch asterisk, conditionals badge, no revert button) | Tasks 5-6 |
| §7 Edge cases | Covered: empty paths (Task 3 test), captured-label conditional skipped (Task 3 test), missing activeWhen treated as always (Task 3 + Task 5), virtual vs materialized mismatch (Task 6 isOverrideOutOfSync guard) |
| §8 Out of scope | Nothing in plan implements out-of-scope items |
| §9 Testing strategy | Task 1 round-trip; Task 3 merge tests; frontend covered by smoke + npm run check |

**Placeholder scan:** No "TBD", "TODO", "implement later". All code blocks contain complete implementations.

**Type consistency:**
- `foundry_captured_labels: Vec<String>` (Rust) ↔ `foundryCapturedLabels: string[]` (TS) — used consistently across all tasks.
- `is_captured(bonus, labels)` signature consistent between Task 3 definition and Task 3 call site in `merge_bonuses`.
- `bonusesFor` / `conditionalsFor` signatures (`(itemId: string) → FoundryItemBonus[]`) match between definition (Task 5) and CharacterRow call sites + ModifierCard prop types.
- `effectKey(value, paths)` and `isOverrideOutOfSync(mod)` consistent in Task 6.
- `onSaveAsOverride?: () => void` prop matches between CharacterRow pass-down and ModifierCard receipt.

**Verify gate present in every commit task:** Each of Tasks 1, 2, 3, 4, 5, 6 has a `./scripts/verify.sh` step immediately before the commit step. ✓

**Commit footer choice:** Per the brainstorming/spec/plan context, this feature is the round-trip half of the GM Screen ↔ Foundry pipeline. There is no specific open GitHub issue identified for this feature in the plan-arguments. If an issue exists, the executing agent should append a `Closes #N` or `Refs #N` line to the commit messages above per user authorization. Otherwise, leave the footers as drafted.
