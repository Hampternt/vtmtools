# Advantage editor (#8) — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship issue [#8](https://github.com/Hampternt/vtmtools/issues/8) — add and remove merits / flaws / backgrounds / boons on **live Foundry** character cards via two new Tauri commands. Roll20 live editing fast-fails (Phase 2.5 follow-up). Saved-side editing works for any source. Diff layer extends with `diffAdvantages`.

**Architecture:** Two new Tauri commands (`character_add_advantage`, `character_remove_advantage`) in `src-tauri/src/tools/character.rs` compose Foundry's already-shipped `actor.create_feature` / `actor.delete_item_by_id` builders on the live side and two new JSON-walking helpers (`db_add_advantage` / `db_remove_advantage`) in `src-tauri/src/db/saved_character.rs` on the saved side. Both commands accept the `WriteTarget` enum from #6 and follow saved-first / partial-success-error semantics. A `FeatureType` Rust enum + TS literal mirrors the four valid values (`merit | flaw | background | boon`). Diff layer extends with `diffAdvantages` mirroring the existing `diffSpecialties` pattern. Frontend gets chip-X remove buttons + a per-category inline Add form on the existing feats section of live Foundry cards. Saved-only-card editing is anti-scope (Phase 2.5 follow-up).

**Tech Stack:** Rust 2021 (Tauri 2, sqlx, tokio, serde_json, uuid), TypeScript (Svelte 5 runes mode), SQLite.

**Spec:** `docs/superpowers/specs/2026-05-02-phase-2-character-editing-design.md` (covers #7 and #8; this plan is the #8 half)

**Hard rules** (from `CLAUDE.md`):
- Every task ending in a commit MUST run `./scripts/verify.sh` first and produce a green result.
- Frontend components NEVER call `invoke(...)` directly — go through `src/lib/character/api.ts`.
- Never run `git status -uall` (memory issues on large trees).
- Never hardcode hex colors — use tokens from `:root` in `src/routes/+layout.svelte`.

---

## Fresh-session bootstrap

If you are picking this plan up in a new session, here's everything you need:

**Read first (in order):**
1. `CLAUDE.md` — auto-loaded; defines verify gate + frontend-wrapper rule + lean-execution / TDD-on-demand overrides.
2. **This plan** — has full code for every task. Spec is referenced but not strictly required.
3. (Optional) `docs/superpowers/specs/2026-05-02-phase-2-character-editing-design.md` §2.2, §2.3, §2.5–§2.8, §4.1–§4.5 — adds rationale (saved-id strategy, Both atomicity, Roll20 deferral).

**Recommended dispatch shape (subagent-driven):**

```
Task 0 (pre-flight)              ← single subagent, blocking
  │
  ▼
Task 1 (db helpers + uuid)
  │
  ▼
Task 2 (router commands + FeatureType)
  │
  ▼
Task 3 (lib.rs registration)
  │
  ├──► Task 4 (TS types + api.ts)         ┐
  │                                         ├── parallel-safe (disjoint files)
  └──► Task 5 (diff.ts extension)         ─┘
                                  │
                                  ▼
                          Task 6 (chip-X UI)
                                  │
                                  ▼
                          Task 7 (Add form UI)
                                  │
                                  ▼
                          Task 8 (Roll20 disabled-state polish)
                                  │
                                  ▼
                          Task 9 (final verify + smoke)
```

Tasks 4 and 5 touch disjoint files and can run in parallel. The Svelte tasks (6, 7, 8) all modify `Campaign.svelte` and must be sequential.

**Suggested first message in the new session:**

> "Execute the plan at `docs/superpowers/plans/2026-05-02-advantage-editor.md` using `superpowers:subagent-driven-development`. Run Task 0; then Task 1; then Task 2; then Task 3; then dispatch Tasks 4+5 in parallel; then 6, 7, 8, 9 sequentially. Final commit footer: `Closes #8`."

---

## File map

| Action | Path | Purpose | Task |
|---|---|---|---|
| Modify | `src-tauri/Cargo.toml` | Add `uuid = { version = "1", features = ["v4"] }` | 1 |
| Modify | `src-tauri/src/db/saved_character.rs` | `db_add_advantage` + `db_remove_advantage` + tests | 1 |
| Modify | `src-tauri/src/tools/character.rs` | `FeatureType` enum, `character_add_advantage` + `character_remove_advantage` + inner helpers + tests | 2 |
| Modify | `src-tauri/src/lib.rs` | Register both new commands in `invoke_handler!` | 3 |
| Modify | `src/types.ts` | Add `FeatureType` literal type | 4 |
| Modify | `src/lib/character/api.ts` | `characterAddAdvantage` + `characterRemoveAdvantage` typed wrappers | 4 |
| Modify | `src/lib/saved-characters/diff.ts` | `diffAdvantages` list comparator + compose into `diffCharacter` | 5 |
| Modify | `src/tools/Campaign.svelte` | chip-X remove buttons on feature chips | 6 |
| Modify | `src/tools/Campaign.svelte` | per-category Add form + submit handler | 7 |
| Modify | `src/tools/Campaign.svelte` | Roll20 disabled-state polish | 8 |

Total: 8 modifications. No new files. No SQL migrations. No wire variants. No `vtmtools-bridge` JS changes. Tauri command surface grows from 39 → 41 (`character_add_advantage`, `character_remove_advantage`).

---

### Task 0: Pre-flight green build

Verifies the workspace is green before starting; surfaces any unrelated issues before they get attributed to this work.

**Files:** none

- [ ] **Step 1: Run the aggregate gate.**

```bash
./scripts/verify.sh
```

Expected: green. If it fails, stop and resolve before starting Task 1.

---

### Task 1: `db/saved_character.rs` — JSON-walking helpers + uuid dep

The genuinely new persistence primitives: append a feature-typed item to `canonical.raw.items[]` (with a synthesized `local-<uuid>` id), or remove one by `_id`+`featuretype`.

**Tests: required.** Per CLAUDE.md TDD-on-demand override, character data transforms get TDD. The existing `db_patch_field` tests in this file are the pattern to mirror.

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/db/saved_character.rs`

- [ ] **Step 1: Add the `uuid` dependency.**

In `src-tauri/Cargo.toml`, find the `[dependencies]` section. The current end of the section reads:

```toml
rcgen = "0.13"
tokio-rustls = "0.26"
rustls-pemfile = "2"
```

Add one line directly after `rustls-pemfile = "2"`:

```toml
uuid = { version = "1", features = ["v4"] }
```

So the section ends:

```toml
rcgen = "0.13"
tokio-rustls = "0.26"
rustls-pemfile = "2"
uuid = { version = "1", features = ["v4"] }
```

Verify the dep resolves:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS (no usage yet, just the dep is fetched and compiled).

- [ ] **Step 2: Add the failing tests first (TDD).**

In `src-tauri/src/db/saved_character.rs`, find the existing `#[cfg(test)] mod tests { … }` block (starts around line 215). Inside, after the existing `patch_field_*` tests (the most recent additions from #6), add the following tests. Place them after the last existing `#[tokio::test]` and before the closing `}` of the `mod tests` block:

```rust
    // ── #8 advantage editor tests ────────────────────────────────────────

    fn sample_canonical_with_items(items: serde_json::Value) -> CanonicalCharacter {
        let mut c = sample_canonical();
        c.raw = serde_json::json!({ "items": items });
        c
    }

    async fn seed_with_canonical(pool: &SqlitePool, c: &CanonicalCharacter) -> i64 {
        db_save(pool, c, None).await.unwrap()
    }

    #[tokio::test]
    async fn add_advantage_happy_path_appends_item_with_local_uuid() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical();
        let id = db_save(&pool, &canonical, None).await.unwrap();

        db_add_advantage(&pool, id, "merit", "Iron Will", "Strong-minded.", 2)
            .await
            .unwrap();

        let list = db_list(&pool).await.unwrap();
        let items = list[0].canonical.raw.get("items")
            .and_then(|v| v.as_array())
            .expect("items array");
        assert_eq!(items.len(), 1);
        let item = &items[0];
        let item_id = item.get("_id").and_then(|v| v.as_str()).unwrap();
        assert!(item_id.starts_with("local-"), "got id: {item_id}");
        assert_eq!(item.get("type").and_then(|v| v.as_str()), Some("feature"));
        assert_eq!(item.get("name").and_then(|v| v.as_str()), Some("Iron Will"));
        let sys = item.get("system").unwrap();
        assert_eq!(sys.get("featuretype").and_then(|v| v.as_str()), Some("merit"));
        assert_eq!(sys.get("description").and_then(|v| v.as_str()), Some("Strong-minded."));
        assert_eq!(sys.get("points").and_then(|v| v.as_i64()), Some(2));
    }

    #[tokio::test]
    async fn add_advantage_invalid_featuretype_errors() {
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();
        let err = db_add_advantage(&pool, id, "discipline", "X", "y", 1)
            .await
            .unwrap_err();
        assert!(err.contains("invalid featuretype"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_missing_id_errors() {
        let pool = fresh_pool().await;
        let err = db_add_advantage(&pool, 9999, "merit", "X", "y", 1)
            .await
            .unwrap_err();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_materializes_items_array_if_absent() {
        // sample_canonical() has raw = json!({}), no items key. Must work.
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();
        db_add_advantage(&pool, id, "boon", "Owed Favor", "From Camarilla.", 3)
            .await
            .unwrap();
        let list = db_list(&pool).await.unwrap();
        let items = list[0].canonical.raw.get("items")
            .and_then(|v| v.as_array())
            .expect("items array");
        assert_eq!(items.len(), 1);
    }

    #[tokio::test]
    async fn remove_advantage_happy_path() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical_with_items(serde_json::json!([
            { "_id": "item-keep",   "type": "feature", "name": "Keep",
              "system": { "featuretype": "merit", "description": "k", "points": 1 },
              "effects": [] },
            { "_id": "item-remove", "type": "feature", "name": "Remove",
              "system": { "featuretype": "merit", "description": "r", "points": 1 },
              "effects": [] },
        ]));
        let id = seed_with_canonical(&pool, &canonical).await;

        db_remove_advantage(&pool, id, "merit", "item-remove").await.unwrap();

        let list = db_list(&pool).await.unwrap();
        let items = list[0].canonical.raw.get("items")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("_id").and_then(|v| v.as_str()), Some("item-keep"));
        assert!(!list[0].last_updated_at.is_empty());
    }

    #[tokio::test]
    async fn remove_advantage_missing_id_errors() {
        let pool = fresh_pool().await;
        let canonical = sample_canonical_with_items(serde_json::json!([
            { "_id": "item-1", "type": "feature", "name": "X",
              "system": { "featuretype": "merit" }, "effects": [] },
        ]));
        let id = seed_with_canonical(&pool, &canonical).await;
        let err = db_remove_advantage(&pool, id, "merit", "nonexistent").await.unwrap_err();
        assert!(err.contains("no merit with id 'nonexistent'"), "got: {err}");
    }

    #[tokio::test]
    async fn remove_advantage_featuretype_mismatch_errors() {
        // Defense-in-depth: if the UI passes the wrong featuretype, the id-match
        // alone isn't enough — both must agree.
        let pool = fresh_pool().await;
        let canonical = sample_canonical_with_items(serde_json::json!([
            { "_id": "item-1", "type": "feature", "name": "X",
              "system": { "featuretype": "merit" }, "effects": [] },
        ]));
        let id = seed_with_canonical(&pool, &canonical).await;
        let err = db_remove_advantage(&pool, id, "flaw", "item-1").await.unwrap_err();
        assert!(err.contains("no flaw with id 'item-1'"), "got: {err}");
    }

    #[tokio::test]
    async fn remove_advantage_no_items_key_errors() {
        // sample_canonical()'s raw is {} — no items key at all.
        let pool = fresh_pool().await;
        let id = db_save(&pool, &sample_canonical(), None).await.unwrap();
        let err = db_remove_advantage(&pool, id, "merit", "item-1").await.unwrap_err();
        assert!(err.contains("no item with id 'item-1'"), "got: {err}");
    }
```

- [ ] **Step 3: Run tests — verify they fail.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::saved_character
```

Expected: FAIL with "cannot find function `db_add_advantage`" / "cannot find function `db_remove_advantage`".

- [ ] **Step 4: Implement.**

Add the following two functions to `src-tauri/src/db/saved_character.rs`, immediately after the existing `db_patch_field` function (around line 203, before `#[tauri::command] pub async fn patch_saved_field`):

```rust
/// Append a feature item to canonical.raw.items[]. Item shape matches what
/// Foundry's actor.create_feature executor produces (type=feature,
/// system.featuretype/description/points). The synthesized `_id` uses the
/// `local-<uuid>` convention (router spec §2.3) — survives until the next
/// "Update saved" replaces the blob with the live one.
pub(crate) async fn db_add_advantage(
    pool: &SqlitePool,
    id: i64,
    featuretype: &str,
    name: &str,
    description: &str,
    points: i32,
) -> Result<(), String> {
    match featuretype {
        "merit" | "flaw" | "background" | "boon" => {}
        other => return Err(format!(
            "db/saved_character.add_advantage: invalid featuretype: {other}"
        )),
    }

    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.add_advantage: {e}"))?
        .ok_or_else(|| "db/saved_character.add_advantage: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.add_advantage: deserialize failed: {e}"))?;

    let new_item = serde_json::json!({
        "_id": format!("local-{}", uuid::Uuid::new_v4()),
        "type": "feature",
        "name": name,
        "system": {
            "featuretype": featuretype,
            "description": description,
            "points": points,
        },
        "effects": [],
    });

    let raw = canonical.raw.as_object_mut().ok_or_else(||
        "db/saved_character.add_advantage: canonical.raw is not an object".to_string()
    )?;
    let items = raw.entry("items".to_string())
        .or_insert_with(|| serde_json::Value::Array(vec![]));
    let arr = items.as_array_mut().ok_or_else(||
        "db/saved_character.add_advantage: canonical.raw.items is not an array".to_string()
    )?;
    arr.push(new_item);

    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.add_advantage: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.add_advantage: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.add_advantage: not found".to_string());
    }
    Ok(())
}

/// Remove a feature item by `_id` AND `featuretype` (defense-in-depth so a
/// UI bug can't accidentally delete a discipline document via a matching id).
pub(crate) async fn db_remove_advantage(
    pool: &SqlitePool,
    id: i64,
    featuretype: &str,
    item_id: &str,
) -> Result<(), String> {
    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.remove_advantage: {e}"))?
        .ok_or_else(|| "db/saved_character.remove_advantage: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.remove_advantage: deserialize failed: {e}"))?;

    let raw = canonical.raw.as_object_mut().ok_or_else(||
        "db/saved_character.remove_advantage: canonical.raw is not an object".to_string()
    )?;
    let Some(items) = raw.get_mut("items").and_then(|v| v.as_array_mut()) else {
        return Err(format!(
            "db/saved_character.remove_advantage: no item with id '{item_id}'"
        ));
    };
    let original_len = items.len();
    items.retain(|item| {
        let id_match = item.get("_id").and_then(|v| v.as_str()) == Some(item_id);
        let ft_match = item
            .get("system")
            .and_then(|s| s.get("featuretype"))
            .and_then(|v| v.as_str())
            == Some(featuretype);
        !(id_match && ft_match)
    });
    if items.len() == original_len {
        return Err(format!(
            "db/saved_character.remove_advantage: no {featuretype} with id '{item_id}'"
        ));
    }

    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.remove_advantage: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.remove_advantage: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.remove_advantage: not found".to_string());
    }
    Ok(())
}
```

- [ ] **Step 5: Run tests — verify they pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::saved_character
```

Expected: PASS. The 8 new advantage tests + the existing tests in this module all green.

- [ ] **Step 6: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 7: Commit.**

```bash
git add src-tauri/Cargo.toml src-tauri/src/db/saved_character.rs
git commit -m "feat(db/saved_character): add db_add_advantage + db_remove_advantage helpers"
```

---

### Task 2: `tools/character.rs` — router commands + `FeatureType` enum

Two new Tauri commands compose Task 1's saved-side helpers and Foundry's already-shipped `actor.create_feature` / `actor.delete_item_by_id` builders. Saved-first / partial-success-error semantics mirror #6.

**Tests: required.** Per CLAUDE.md TDD-on-demand override — IPC routing logic with branching control flow (target match, Roll20 fast-fail, Both atomicity) gets TDD. The existing `do_set_field` tests in this file are the pattern to mirror.

**Files:**
- Modify: `src-tauri/src/tools/character.rs`

- [ ] **Step 1: Add the failing tests first (TDD).**

In `src-tauri/src/tools/character.rs`, find the existing `#[cfg(test)] mod tests { … }` block. Inside, after the existing `do_set_field` tests, add the following test block. Place after the last existing `#[tokio::test]` and before the closing `}`:

```rust
    // ── #8 advantage editor tests ────────────────────────────────────────

    async fn seed_saved_row_with_item(pool: &SqlitePool, source_id: &str, item_id: &str, ft: &str) {
        let mut c = sample_canonical();
        c.raw = serde_json::json!({
            "items": [
                { "_id": item_id, "type": "feature", "name": "Pre-existing",
                  "system": { "featuretype": ft, "description": "x", "points": 1 },
                  "effects": [] }
            ]
        });
        let canonical_json = serde_json::to_string(&c).unwrap();
        sqlx::query(
            "INSERT INTO saved_characters
             (source, source_id, foundry_world, name, canonical_json)
             VALUES ('foundry', ?, NULL, 'Test', ?)",
        )
        .bind(source_id)
        .bind(&canonical_json)
        .execute(pool)
        .await
        .unwrap();
    }

    // ─── add_advantage ─────────────────────────────────────────────────

    #[tokio::test]
    async fn add_advantage_empty_name_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "   ".to_string(), "desc".to_string(), 2,
        ).await.unwrap_err();
        assert!(err.contains("empty name"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_points_out_of_range_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 11,
        ).await.unwrap_err();
        assert!(err.contains("out of range"), "got: {err}");
    }

    #[tokio::test]
    async fn add_advantage_roll20_live_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Roll20,
            "abc".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 1,
        ).await.unwrap_err();
        assert!(
            err.contains("Roll20 live editing of advantages not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn add_advantage_roll20_saved_succeeds() {
        // Saved-side editing works for any source — Roll20 fast-fail only on Live.
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        // Seed a roll20 saved row.
        let mut c = sample_canonical();
        c.source = SourceKind::Roll20;
        let canonical_json = serde_json::to_string(&c).unwrap();
        sqlx::query(
            "INSERT INTO saved_characters
             (source, source_id, foundry_world, name, canonical_json)
             VALUES ('roll20', 'r20-1', NULL, 'R20', ?)",
        )
        .bind(&canonical_json)
        .execute(&pool)
        .await
        .unwrap();

        do_add_advantage(
            &pool, &state, WriteTarget::Saved, SourceKind::Roll20,
            "r20-1".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 1,
        ).await.unwrap();
    }

    #[tokio::test]
    async fn add_advantage_target_saved_writes_db() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row(&pool, "abc").await;

        do_add_advantage(
            &pool, &state, WriteTarget::Saved, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "Iron Will".to_string(), "Strong-minded.".to_string(), 2,
        ).await.unwrap();

        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        let items = c.raw.get("items").and_then(|v| v.as_array()).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].get("name").and_then(|v| v.as_str()), Some("Iron Will"));
    }

    #[tokio::test]
    async fn add_advantage_target_live_sends_payload() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");

        do_add_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "actor-xyz".to_string(), FeatureType::Flaw,
            "Bad Sight".to_string(), "Squints.".to_string(), 1,
        ).await.unwrap();

        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.create_feature"));
        assert_eq!(payload.get("actor_id").and_then(|v| v.as_str()), Some("actor-xyz"));
        assert_eq!(payload.get("featuretype").and_then(|v| v.as_str()), Some("flaw"));
        assert_eq!(payload.get("name").and_then(|v| v.as_str()), Some("Bad Sight"));
    }

    #[tokio::test]
    async fn add_advantage_target_both_writes_both() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");
        seed_saved_row(&pool, "abc").await;

        do_add_advantage(
            &pool, &state, WriteTarget::Both, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Boon,
            "Owed Favor".to_string(), "From Camarilla.".to_string(), 3,
        ).await.unwrap();

        // Saved write landed.
        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(c.raw.get("items").and_then(|v| v.as_array()).unwrap().len(), 1);

        // Live wire payload sent.
        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.create_feature"));
    }

    #[tokio::test]
    async fn add_advantage_both_partial_success_when_live_fails() {
        // Force tx.send() to fail by dropping the receiver while keeping the
        // sender alive in BridgeState.connections. tokio mpsc::Sender::send
        // returns SendError when the receiver has been dropped, which
        // send_to_source_inner surfaces as Err — triggering the partial-success
        // path (saved-first ordering means the saved write already landed).
        let pool = fresh_pool().await;
        let (state, rx) = make_bridge_state(true);
        drop(rx);
        seed_saved_row(&pool, "abc").await;

        let err = do_add_advantage(
            &pool, &state, WriteTarget::Both, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit,
            "X".to_string(), "y".to_string(), 1,
        ).await.unwrap_err();

        assert!(
            err.starts_with("character/add_advantage: saved updated, live failed:"),
            "got: {err}"
        );

        // Saved row was still written (saved-first ordering).
        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert_eq!(c.raw.get("items").and_then(|v| v.as_array()).unwrap().len(), 1);
    }

    // ─── remove_advantage ─────────────────────────────────────────────

    #[tokio::test]
    async fn remove_advantage_empty_item_id_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_remove_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit, "  ".to_string(),
        ).await.unwrap_err();
        assert!(err.contains("empty item_id"), "got: {err}");
    }

    #[tokio::test]
    async fn remove_advantage_roll20_live_errors() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        let err = do_remove_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Roll20,
            "abc".to_string(), FeatureType::Merit, "item-1".to_string(),
        ).await.unwrap_err();
        assert!(
            err.contains("Roll20 live editing of advantages not yet supported"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn remove_advantage_target_saved_writes_db() {
        let pool = fresh_pool().await;
        let (state, _rx) = make_bridge_state(true);
        seed_saved_row_with_item(&pool, "abc", "item-1", "merit").await;

        do_remove_advantage(
            &pool, &state, WriteTarget::Saved, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit, "item-1".to_string(),
        ).await.unwrap();

        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        let items = c.raw.get("items").and_then(|v| v.as_array()).unwrap();
        assert!(items.is_empty(), "items should be empty after remove");
    }

    #[tokio::test]
    async fn remove_advantage_target_live_sends_payload() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");

        do_remove_advantage(
            &pool, &state, WriteTarget::Live, SourceKind::Foundry,
            "actor-xyz".to_string(), FeatureType::Background, "item-bg".to_string(),
        ).await.unwrap();

        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.delete_item_by_id"));
        assert_eq!(payload.get("actor_id").and_then(|v| v.as_str()), Some("actor-xyz"));
        assert_eq!(payload.get("item_id").and_then(|v| v.as_str()), Some("item-bg"));
    }

    #[tokio::test]
    async fn remove_advantage_target_both_writes_both() {
        let pool = fresh_pool().await;
        let (state, mut rx) = make_bridge_state(true);
        let rx = rx.as_mut().expect("connected");
        seed_saved_row_with_item(&pool, "abc", "item-1", "merit").await;

        do_remove_advantage(
            &pool, &state, WriteTarget::Both, SourceKind::Foundry,
            "abc".to_string(), FeatureType::Merit, "item-1".to_string(),
        ).await.unwrap();

        // Saved row updated.
        let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE source_id = 'abc'")
            .fetch_one(&pool).await.unwrap();
        let json: String = row.get("canonical_json");
        let c: CanonicalCharacter = serde_json::from_str(&json).unwrap();
        assert!(c.raw.get("items").and_then(|v| v.as_array()).unwrap().is_empty());

        // Live payload sent.
        let payload_text = rx.try_recv().expect("payload sent");
        let payload: serde_json::Value = serde_json::from_str(&payload_text).unwrap();
        assert_eq!(payload.get("type").and_then(|v| v.as_str()), Some("actor.delete_item_by_id"));
    }
```

- [ ] **Step 2: Run tests — verify they fail.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml tools::character
```

Expected: FAIL with "cannot find type/function `FeatureType` / `do_add_advantage` / `do_remove_advantage`".

- [ ] **Step 3: Implement.**

In `src-tauri/src/tools/character.rs`, immediately after the existing `WriteTarget` enum declaration, add the `FeatureType` enum:

```rust
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum FeatureType {
    Merit,
    Flaw,
    Background,
    Boon,
}

impl FeatureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureType::Merit      => "merit",
            FeatureType::Flaw       => "flaw",
            FeatureType::Background => "background",
            FeatureType::Boon       => "boon",
        }
    }
}
```

Then, immediately after the existing `forward_live` function (and before the `#[cfg(test)] mod tests` block), add the four new functions — two `#[tauri::command]` wrappers and two `pub(crate)` inner helpers:

```rust
#[tauri::command]
pub async fn character_add_advantage(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    name: String,
    description: String,
    points: i32,
) -> Result<(), String> {
    do_add_advantage(
        &db.0, &bridge.0, target, source, source_id,
        featuretype, name, description, points,
    ).await
}

pub(crate) async fn do_add_advantage(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    name: String,
    description: String,
    points: i32,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("character/add_advantage: empty name".to_string());
    }
    if !(0..=10).contains(&points) {
        return Err(format!(
            "character/add_advantage: points {points} out of range 0..=10"
        ));
    }

    if target != WriteTarget::Saved && source == SourceKind::Roll20 {
        return Err(
            "character/add_advantage: Roll20 live editing of advantages not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    let do_saved = || async {
        crate::db::saved_character::db_add_advantage(
            pool,
            saved_id.unwrap(),
            featuretype.as_str(),
            &name,
            &description,
            points,
        )
        .await
    };

    let do_live = || async {
        let payload = crate::bridge::foundry::actions::actor::build_create_feature(
            &source_id,
            featuretype.as_str(),
            &name,
            &description,
            points,
        )
        .map_err(|e| format!("character/add_advantage: {e}"))?;
        let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        crate::bridge::commands::send_to_source_inner(bridge_state, source, text).await
    };

    match target {
        WriteTarget::Saved => do_saved().await,
        WriteTarget::Live  => do_live().await,
        WriteTarget::Both  => {
            do_saved().await
                .map_err(|e| format!("character/add_advantage: saved write failed: {e}"))?;
            do_live().await
                .map_err(|e| format!(
                    "character/add_advantage: saved updated, live failed: {e}"
                ))
        }
    }
}

#[tauri::command]
pub async fn character_remove_advantage(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    item_id: String,
) -> Result<(), String> {
    do_remove_advantage(
        &db.0, &bridge.0, target, source, source_id, featuretype, item_id,
    ).await
}

pub(crate) async fn do_remove_advantage(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    item_id: String,
) -> Result<(), String> {
    if item_id.trim().is_empty() {
        return Err("character/remove_advantage: empty item_id".to_string());
    }

    if target != WriteTarget::Saved && source == SourceKind::Roll20 {
        return Err(
            "character/remove_advantage: Roll20 live editing of advantages not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    let do_saved = || async {
        crate::db::saved_character::db_remove_advantage(
            pool, saved_id.unwrap(), featuretype.as_str(), &item_id,
        )
        .await
    };

    let do_live = || async {
        let payload = crate::bridge::foundry::actions::actor::build_delete_item_by_id(
            &source_id, &item_id,
        );
        let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        crate::bridge::commands::send_to_source_inner(bridge_state, source, text).await
    };

    match target {
        WriteTarget::Saved => do_saved().await,
        WriteTarget::Live  => do_live().await,
        WriteTarget::Both  => {
            do_saved().await
                .map_err(|e| format!("character/remove_advantage: saved write failed: {e}"))?;
            do_live().await
                .map_err(|e| format!(
                    "character/remove_advantage: saved updated, live failed: {e}"
                ))
        }
    }
}
```

- [ ] **Step 4: Run tests — verify they pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml tools::character
```

Expected: PASS. All existing `do_set_field` tests + the 13 new advantage tests green.

If the `mut rx` pattern in `add_advantage_target_live_sends_payload` fails to compile because `make_bridge_state` returns `Option<Receiver<String>>` and not `&mut`, change the call to bind mutably: `let (state, mut rx) = make_bridge_state(true);` then use `let rx = rx.as_mut().expect("connected");`. The test code above already uses this pattern.

- [ ] **Step 5: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add src-tauri/src/tools/character.rs
git commit -m "feat(tools/character): add character_add_advantage + character_remove_advantage"
```

---

### Task 3: Register Tauri commands in `lib.rs`

Wire the two new commands into `invoke_handler!` so the frontend can call them.

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Read the current `invoke_handler!` block.**

Use the `Read` tool on `src-tauri/src/lib.rs` to locate the `tauri::generate_handler![` block. Find the line:

```rust
            tools::character::character_set_field,
```

This was added by #6.

- [ ] **Step 2: Add the two new commands directly below.**

Replace:

```rust
            tools::character::character_set_field,
```

with:

```rust
            tools::character::character_set_field,
            tools::character::character_add_advantage,
            tools::character::character_remove_advantage,
```

- [ ] **Step 3: Run cargo check.**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(lib): register character_add_advantage + character_remove_advantage"
```

---

### Task 4: Frontend types + typed wrappers

Type-safe surface so `Campaign.svelte` can call the router commands without `invoke()`. Parallel-safe with Task 5 (different files).

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/character/api.ts`

- [ ] **Step 1: Add `FeatureType` to `src/types.ts`.**

Find the existing `CanonicalFieldName` literal-union in `src/types.ts` (around line 69–77, ending with `'willpower_aggravated';`). Insert the following two lines immediately after that closing `;`, before the `HealthTrack` interface:

```ts

/** Mirrors src-tauri/src/tools/character.rs::FeatureType. */
export type FeatureType = 'merit' | 'flaw' | 'background' | 'boon';
```

- [ ] **Step 2: Extend `src/lib/character/api.ts` with the two new wrappers.**

The current file (3 exports) ends at line 24. Replace the entire file body with:

```ts
// Typed wrappers around character_set_field / character_add_advantage /
// character_remove_advantage. Per CLAUDE.md, components must NOT call
// invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type {
  SourceKind,
  WriteTarget,
  CanonicalFieldName,
  FeatureType,
} from '../../types';

export type { WriteTarget, CanonicalFieldName, FeatureType } from '../../types';

export const characterSetField = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('character_set_field', { target, source, sourceId, name, value });

export const patchSavedField = (
  id: number,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('patch_saved_field', { id, name, value });

export const characterAddAdvantage = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  featuretype: FeatureType,
  name: string,
  description: string,
  points: number,
): Promise<void> =>
  invoke<void>('character_add_advantage', {
    target, source, sourceId, featuretype, name, description, points,
  });

export const characterRemoveAdvantage = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  featuretype: FeatureType,
  itemId: string,
): Promise<void> =>
  invoke<void>('character_remove_advantage', {
    target, source, sourceId, featuretype, itemId,
  });
```

- [ ] **Step 3: Run npm check.**

```bash
npm run check
```

Expected: PASS — no TS errors.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add src/types.ts src/lib/character/api.ts
git commit -m "feat(lib/character): add typed wrappers for advantage add/remove"
```

---

### Task 5: Diff layer extension — `diffAdvantages`

Mirrors `diffSpecialties` (already shipped). Filters `raw.items[]` by `type === 'feature'`, partitions by `system.featuretype`, compares by `name` (matches the `diffSpecialties` keying pattern). Composes into `diffCharacter`. Parallel-safe with Task 4.

**Tests: not required** (per CLAUDE.md TDD-on-demand override — pure pattern-mirror over the already-tested `diffSpecialties`; the type system + `npm run check` is the gate).

**Files:**
- Modify: `src/lib/saved-characters/diff.ts`

- [ ] **Step 1: Add `collectAdvantages` and `diffAdvantages`.**

In `src/lib/saved-characters/diff.ts`, immediately after `diffSpecialties` (around line 125, before `diffCharacter`), insert:

```ts
/** Build a map of (featuretype → Map<name, points>) from raw.items[] feature documents. */
function collectAdvantages(raw: unknown): Record<string, Map<string, number>> {
  const out: Record<string, Map<string, number>> = {
    merit: new Map(), flaw: new Map(), background: new Map(), boon: new Map(),
  };
  const items = (raw as { items?: unknown[] } | null)?.items ?? [];
  for (const item of items as Array<Record<string, unknown>>) {
    if (item.type !== 'feature') continue;
    const sys = item.system as Record<string, unknown> | undefined;
    const ft = sys?.featuretype as string | undefined;
    const name = item.name as string | undefined;
    if (!ft || !(ft in out) || !name) continue;
    const points = typeof sys?.points === 'number' ? sys.points : 0;
    out[ft].set(name, points);
  }
  return out;
}

/**
 * List comparator for advantage Items (merits/flaws/backgrounds/boons).
 * Roll20 saves skip entirely (advantages live in repeating-section attrs,
 * not feature documents). Matching key is `name` within `featuretype`,
 * matching diffSpecialties' keying.
 */
export function diffAdvantages(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  if (saved.source !== 'foundry') return [];
  const savedMap = collectAdvantages(saved.raw);
  const liveMap  = collectAdvantages(live.raw);
  const entries: DiffEntry[] = [];
  for (const ft of ['merit', 'flaw', 'background', 'boon'] as const) {
    const sv = savedMap[ft];
    const lv = liveMap[ft];
    const allNames = new Set([...sv.keys(), ...lv.keys()]);
    for (const name of allNames) {
      const before = sv.get(name);
      const after  = lv.get(name);
      const label  = `${cap(ft)}: ${name}`;
      const key    = `${ft}.${name}`;
      if (before === undefined && after !== undefined) {
        entries.push({ key, label, before: '—', after: after > 0 ? `+ (${after})` : 'added' });
      } else if (after === undefined && before !== undefined) {
        entries.push({ key, label, before: before > 0 ? `(${before})` : 'present', after: '—' });
      } else if (before !== after) {
        entries.push({ key, label, before: String(before), after: String(after) });
      }
    }
  }
  return entries;
}
```

- [ ] **Step 2: Compose `diffAdvantages` into `diffCharacter`.**

Find the existing `diffCharacter` function at the bottom of the file (around line 134–148):

```ts
export function diffCharacter(
  saved: BridgeCharacter,
  live: BridgeCharacter,
): DiffEntry[] {
  const pathDiffs: DiffEntry[] = DIFFABLE_PATHS
    .map(p => ({ key: p.key, label: p.label, before: p.read(saved), after: p.read(live) }))
    .filter(({ before, after }) => before !== after)
    .map(({ key, label, before, after }) => ({
      key,
      label,
      before: before == null ? '—' : String(before),
      after:  after  == null ? '—' : String(after),
    }));
  return [...pathDiffs, ...diffSpecialties(saved, live)];
}
```

Replace the final `return` line:

```ts
  return [...pathDiffs, ...diffSpecialties(saved, live)];
```

with:

```ts
  return [...pathDiffs, ...diffSpecialties(saved, live), ...diffAdvantages(saved, live)];
```

- [ ] **Step 3: Run npm check.**

```bash
npm run check
```

Expected: PASS.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 5: Commit.**

```bash
git add src/lib/saved-characters/diff.ts
git commit -m "feat(saved-characters/diff): add diffAdvantages list comparator"
```

---

### Task 6: Chip-X remove buttons on feature chips

Adds an `×` button to each existing feature chip (merit / flaw / background / boon) on **live Foundry** cards. Click → `window.confirm` → `characterRemoveAdvantage('live', ...)`.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Add the import + handler.**

In the script block of `Campaign.svelte`, find the existing import block (around lines 11–16). The line:

```ts
  } from '$lib/foundry/raw';
```

Replace it with:

```ts
  } from '$lib/foundry/raw';
  import type { FoundryItem } from '../types';
  import { characterRemoveAdvantage, characterAddAdvantage } from '$lib/character/api';
  import type { FeatureType } from '$lib/character/api';
```

(`characterAddAdvantage` is imported now even though Task 7 uses it — keeps the import block tidy. If Plan A has already been applied, the file already imports `characterSetField`; leave that line in place.)

After the existing helpers in the script block (after `function refresh()` around line 168, before the `</script>` closing tag), add:

```ts
  // ── Advantage editor (#8) ────────────────────────────────────────────────

  function advantageEditAllowed(char: BridgeCharacter): boolean {
    return char.source === 'foundry';
  }

  let busyAdvantageKey = $state<string | null>(null);

  function advantageBusyKey(char: BridgeCharacter, itemId: string): string {
    return `${char.source}:${char.source_id}:${itemId}`;
  }

  async function removeAdvantage(
    char: BridgeCharacter,
    featuretype: FeatureType,
    item: FoundryItem,
  ) {
    if (!advantageEditAllowed(char)) return;
    if (!window.confirm(`Remove ${featuretype} '${item.name}'?`)) return;
    const key = advantageBusyKey(char, item._id);
    busyAdvantageKey = key;
    try {
      await characterRemoveAdvantage(
        'live', char.source, char.source_id, featuretype, item._id,
      );
    } catch (e) {
      console.error('[Campaign] characterRemoveAdvantage failed:', e);
      window.alert(String(e));
    } finally {
      if (busyAdvantageKey === key) busyAdvantageKey = null;
    }
  }
```

- [ ] **Step 2: Add the chip-X snippet.**

After the closing `</script>` and *before* the opening `<div class="campaign">`, add (or, if Plan A already added a `stepper` snippet there, append below it):

```svelte
{#snippet chipRemoveBtn(char: BridgeCharacter, featuretype: FeatureType, item: FoundryItem)}
  {@const allowed = advantageEditAllowed(char)}
  {@const busy    = busyAdvantageKey === advantageBusyKey(char, item._id)}
  {#if allowed}
    <button
      type="button"
      class="chip-remove-btn"
      onclick={() => removeAdvantage(char, featuretype, item)}
      disabled={busy}
      aria-busy={busy}
      title={`Remove ${featuretype}`}
      aria-label={`Remove ${featuretype} ${item.name}`}
    >×</button>
  {/if}
{/snippet}
```

- [ ] **Step 3: Wire `chipRemoveBtn` into the four feature rows.**

Find the feats section (around lines 506–586). For each of the four feature rows, append the chip-remove button inside the chip span. Specifically:

**Merits row** — find the inner chip block (around lines 513–520):

```svelte
                    {#each merits as m}
                      {@const points = (m.system?.points as number | undefined) ?? 0}
                      {@const itemFx = foundryItemEffects(m).filter(foundryEffectIsActive)}
                      <span class="feat-chip merit" title={itemFx.length > 0 ? `${itemFx.length} active modifier(s)` : ''}>
                        <span class="feat-name">{m.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {#if itemFx.length > 0}<span class="feat-fx-badge">+{itemFx.length}</span>{/if}
                      </span>
                    {/each}
```

Replace with (only the line after `{#if itemFx.length > 0}<span class="feat-fx-badge">...` changes — add the chip-remove call):

```svelte
                    {#each merits as m}
                      {@const points = (m.system?.points as number | undefined) ?? 0}
                      {@const itemFx = foundryItemEffects(m).filter(foundryEffectIsActive)}
                      <span class="feat-chip merit" title={itemFx.length > 0 ? `${itemFx.length} active modifier(s)` : ''}>
                        <span class="feat-name">{m.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {#if itemFx.length > 0}<span class="feat-fx-badge">+{itemFx.length}</span>{/if}
                        {@render chipRemoveBtn(char, 'merit', m)}
                      </span>
                    {/each}
```

**Flaws row** — same pattern. Add `{@render chipRemoveBtn(char, 'flaw', f)}` directly before the chip's closing `</span>`:

```svelte
                    {#each flaws as f}
                      {@const points = (f.system?.points as number | undefined) ?? 0}
                      {@const itemFx = foundryItemEffects(f).filter(foundryEffectIsActive)}
                      <span class="feat-chip flaw" title={itemFx.length > 0 ? `${itemFx.length} active modifier(s)` : ''}>
                        <span class="feat-name">{f.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {#if itemFx.length > 0}<span class="feat-fx-badge">+{itemFx.length}</span>{/if}
                        {@render chipRemoveBtn(char, 'flaw', f)}
                      </span>
                    {/each}
```

**Backgrounds row** — same pattern:

```svelte
                    {#each backgrounds as b}
                      {@const points = (b.system?.points as number | undefined) ?? 0}
                      <span class="feat-chip background">
                        <span class="feat-name">{b.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {@render chipRemoveBtn(char, 'background', b)}
                      </span>
                    {/each}
```

**Boons row** — same pattern:

```svelte
                    {#each boons as bn}
                      <span class="feat-chip boon">
                        <span class="feat-name">{bn.name}</span>
                        {@render chipRemoveBtn(char, 'boon', bn)}
                      </span>
                    {/each}
```

- [ ] **Step 4: Add the chip-remove CSS.**

Inside the `<style>` block, after the existing `.feat-fx-badge { ... }` rule (around line 1302), insert:

```css
  /* ── Chip remove button (#8) ─────────────────────────────────────────── */
  .chip-remove-btn {
    margin-left: 0.2rem;
    width: 1rem;
    height: 1rem;
    padding: 0;
    font-size: 0.85rem;
    font-weight: 700;
    line-height: 1;
    color: var(--text-ghost);
    background: transparent;
    border: 1px solid transparent;
    border-radius: 2px;
    cursor: pointer;
    transition: color 0.1s, border-color 0.1s, background 0.1s, opacity 0.1s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
  }
  .feat-chip:hover .chip-remove-btn {
    opacity: 1;
  }
  .chip-remove-btn:hover {
    color: var(--accent);
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 15%, transparent);
  }
  .chip-remove-btn:disabled,
  .chip-remove-btn[aria-busy="true"] {
    opacity: 0.4;
    cursor: wait;
  }
```

- [ ] **Step 5: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 6: Commit.**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(tools/campaign): chip-X remove buttons on feature chips"
```

---

### Task 7: Per-category Add form

A `+ Add` chip-shaped button at the end of each feature row opens an inline form (name / points / description). Submit calls `characterAddAdvantage('live', ...)`. One form open at a time across all cards.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Add Add-form state + handlers.**

Inside the script block, after the `removeAdvantage` function from Task 6, add:

```ts
  // ── Add advantage form ──────────────────────────────────────────────────

  type AddFormState = {
    charKey: string;
    featuretype: FeatureType;
    name: string;
    points: number;
    description: string;
    submitting: boolean;
  };

  let addForm = $state<AddFormState | null>(null);

  function startAdd(char: BridgeCharacter, featuretype: FeatureType) {
    if (!advantageEditAllowed(char)) return;
    addForm = {
      charKey: `${char.source}:${char.source_id}`,
      featuretype,
      name: '',
      points: 0,
      description: '',
      submitting: false,
    };
  }

  function cancelAdd() { addForm = null; }

  function isAddActive(char: BridgeCharacter, featuretype: FeatureType): boolean {
    if (!addForm) return false;
    return addForm.charKey === `${char.source}:${char.source_id}`
        && addForm.featuretype === featuretype;
  }

  function addFormValid(form: AddFormState): boolean {
    return form.name.trim().length > 0
        && form.points >= 0
        && form.points <= 10;
  }

  async function submitAdd(char: BridgeCharacter) {
    if (!addForm) return;
    if (!isAddActive(char, addForm.featuretype)) return;
    if (!addFormValid(addForm)) return;
    addForm.submitting = true;
    try {
      await characterAddAdvantage(
        'live', char.source, char.source_id,
        addForm.featuretype,
        addForm.name.trim(),
        addForm.description,
        addForm.points,
      );
      addForm = null;
    } catch (e) {
      console.error('[Campaign] characterAddAdvantage failed:', e);
      window.alert(String(e));
      if (addForm) addForm.submitting = false;
    }
  }
```

- [ ] **Step 2: Add the Add-form snippets.**

Append after the existing snippets (after `chipRemoveBtn` from Task 6):

```svelte
{#snippet addBtn(char: BridgeCharacter, featuretype: FeatureType)}
  {#if advantageEditAllowed(char) && !isAddActive(char, featuretype)}
    <button
      type="button"
      class="feat-chip add-chip"
      onclick={() => startAdd(char, featuretype)}
      title={`Add ${featuretype}`}
    >+ Add {featuretype}</button>
  {/if}
{/snippet}

{#snippet addForm_(char: BridgeCharacter, featuretype: FeatureType)}
  {#if addForm && isAddActive(char, featuretype)}
    <form
      class="add-form"
      onsubmit={(e) => { e.preventDefault(); void submitAdd(char); }}
    >
      <div class="form-row">
        <label for={`add-name-${addForm.charKey}-${featuretype}`}>Name</label>
        <input
          id={`add-name-${addForm.charKey}-${featuretype}`}
          type="text"
          bind:value={addForm.name}
          maxlength="120"
          required
          autofocus
        />
      </div>
      <div class="form-row">
        <label for={`add-points-${addForm.charKey}-${featuretype}`}>Points</label>
        <input
          id={`add-points-${addForm.charKey}-${featuretype}`}
          type="number"
          min="0"
          max="10"
          bind:value={addForm.points}
        />
      </div>
      <div class="form-row">
        <label for={`add-desc-${addForm.charKey}-${featuretype}`}>Description</label>
        <textarea
          id={`add-desc-${addForm.charKey}-${featuretype}`}
          bind:value={addForm.description}
          rows="2"
        ></textarea>
      </div>
      <div class="form-actions">
        <button
          type="submit"
          class="btn-save"
          disabled={!addFormValid(addForm) || addForm.submitting}
          aria-busy={addForm.submitting}
        >Add</button>
        <button
          type="button"
          class="btn-save"
          onclick={cancelAdd}
          disabled={addForm.submitting}
        >Cancel</button>
      </div>
    </form>
  {/if}
{/snippet}
```

(The trailing `_` on `addForm_` avoids the name collision with the `addForm` reactive state variable.)

- [ ] **Step 3: Wire the Add chip + Add form into each feature row.**

Find each `<div class="feat-row">` block. For each one — Merits, Flaws, Backgrounds, Boons — add `{@render addBtn(char, '<ft>')}` inside `feat-chips` (after the `{#each}` block) and `{@render addForm_(char, '<ft>')}` directly after the closing `</div>` of `feat-chips`.

Concretely for **Merits** (around lines 509–524). The current block reads:

```svelte
              {#if merits.length > 0}
                <div class="feat-row">
                  <span class="stat-label">Merits</span>
                  <div class="feat-chips">
                    {#each merits as m}
                      ...
                    {/each}
                  </div>
                </div>
              {/if}
```

Replace with (note the wrap removal — the row should always render now so the Add button is always available):

```svelte
              {#if merits.length > 0 || advantageEditAllowed(char)}
                <div class="feat-row">
                  <span class="stat-label">Merits</span>
                  <div class="feat-chips">
                    {#each merits as m}
                      {@const points = (m.system?.points as number | undefined) ?? 0}
                      {@const itemFx = foundryItemEffects(m).filter(foundryEffectIsActive)}
                      <span class="feat-chip merit" title={itemFx.length > 0 ? `${itemFx.length} active modifier(s)` : ''}>
                        <span class="feat-name">{m.name}</span>
                        {#if points > 0}<span class="feat-dots">{'•'.repeat(Math.min(points, 5))}</span>{/if}
                        {#if itemFx.length > 0}<span class="feat-fx-badge">+{itemFx.length}</span>{/if}
                        {@render chipRemoveBtn(char, 'merit', m)}
                      </span>
                    {/each}
                    {@render addBtn(char, 'merit')}
                  </div>
                  {@render addForm_(char, 'merit')}
                </div>
              {/if}
```

Apply the same shape to **Flaws**, **Backgrounds**, and **Boons** — change the `{#if X.length > 0}` guard to `{#if X.length > 0 || advantageEditAllowed(char)}`, add `{@render addBtn(char, '<ft>')}` after the chips loop, and `{@render addForm_(char, '<ft>')}` after the `feat-chips` div.

- [ ] **Step 4: Update the empty-state message.**

The existing empty-state at the bottom of the feats section (around line 582–584):

```svelte
              {#if merits.length === 0 && flaws.length === 0 && backgrounds.length === 0 && boons.length === 0 && actorFx.length === 0}
                <span class="feat-empty">No merits, flaws, backgrounds, or modifiers on this character.</span>
              {/if}
```

When `advantageEditAllowed(char)`, all four feat rows now always render (their guards changed in Step 3) and the empty-state would never trigger for Foundry cards. For Roll20 cards the existing message still fires correctly. The condition is fine as-is — leave it.

- [ ] **Step 5: Add the Add-chip + form CSS.**

Inside the `<style>` block, after the existing `.chip-remove-btn[aria-busy="true"] { ... }` rule (added in Task 6, Step 4), insert:

```css
  /* ── Add-advantage chip + inline form (#8) ───────────────────────────── */
  .feat-chip.add-chip {
    background: transparent;
    border-style: dashed;
    border-color: var(--border-surface);
    color: var(--text-ghost);
    cursor: pointer;
    font-weight: 500;
  }
  .feat-chip.add-chip:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
  .add-form {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    margin-top: 0.4rem;
    padding: 0.5rem;
    background: var(--bg-sunken);
    border: 1px solid var(--border-faint);
    border-radius: 5px;
  }
  .form-row {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .form-row label {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-ghost);
    font-weight: 600;
  }
  .form-row input,
  .form-row textarea {
    background: var(--bg-input);
    border: 1px solid var(--border-faint);
    border-radius: 3px;
    padding: 0.25rem 0.4rem;
    font-size: 0.8rem;
    color: var(--text-primary);
    font-family: inherit;
  }
  .form-row input:focus,
  .form-row textarea:focus {
    outline: none;
    border-color: var(--accent);
  }
  .form-actions {
    display: flex;
    gap: 0.4rem;
    justify-content: flex-end;
  }
```

- [ ] **Step 6: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 7: Commit.**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(tools/campaign): per-category Add advantage form"
```

---

### Task 8: Roll20 disabled-state polish

The chip-X and Add-chip snippets from Tasks 6 + 7 already gate on `advantageEditAllowed(char)` (returns false for Roll20), so they don't render at all on Roll20 live cards. This task confirms the Roll20 path is genuinely silent — no half-built UI — and adds a one-line tooltip on the *empty* feats section so the GM understands why Add is missing.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Add a visible Roll20 hint on the feats section.**

Inside the `card-section` div for the feats panel (around line 507–586, gated by `expandedFeats.has(charKey)`), at the very top of the section before the Merits row, insert this conditional hint:

```svelte
              {#if char.source === 'roll20'}
                <div class="roll20-hint" title="Roll20 advantage editing not yet supported (Phase 2.5)">
                  <span>Roll20 advantage editing — coming in Phase 2.5</span>
                </div>
              {/if}
```

So the section now reads:

```svelte
          {#if expandedFeats.has(charKey)}
            <div class="card-section">
              {#if char.source === 'roll20'}
                <div class="roll20-hint" title="Roll20 advantage editing not yet supported (Phase 2.5)">
                  <span>Roll20 advantage editing — coming in Phase 2.5</span>
                </div>
              {/if}
              {#if merits.length > 0 || advantageEditAllowed(char)}
                ...
```

- [ ] **Step 2: Add the Roll20-hint CSS.**

Inside the `<style>` block, after the `.feat-empty { ... }` rule (around line 1307), insert:

```css
  .roll20-hint {
    font-size: 0.72rem;
    color: var(--text-ghost);
    font-style: italic;
    padding: 0.2rem 0;
    border-bottom: 1px dashed var(--border-faint);
    margin-bottom: 0.2rem;
  }
```

- [ ] **Step 3: Run the verification gate.**

```bash
./scripts/verify.sh
```

Expected: PASS.

- [ ] **Step 4: Commit (with issue-closure footer).**

```bash
git add src/tools/Campaign.svelte
git commit -m "$(cat <<'EOF'
feat(tools/campaign): Roll20 advantage-editing hint on feats section

Closes #8
EOF
)"
```

---

### Task 9: Final verification + manual smoke

Closes the loop with the manual verification path from spec §7.

**Files:** none

- [ ] **Step 1: Final aggregate verification.**

```bash
./scripts/verify.sh
```

Expected: PASS — `cargo test`, `npm run check`, `npm run build` all green.

- [ ] **Step 2: Confirm test counts.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::saved_character 2>&1 | tail -3
cargo test --manifest-path src-tauri/Cargo.toml tools::character 2>&1 | tail -3
```

Expected:
- `db::saved_character`: ≥ 19 tests pass (11 pre-existing + 8 new)
- `tools::character`: ≥ 20 tests pass (7 pre-existing + 13 new)

- [ ] **Step 3: Manual smoke (recommended before announcing done).**

From a Foundry-connected dev session against a character with a saved counterpart:

1. Start the dev app:
   ```bash
   npm run tauri dev
   ```
2. Connect Foundry world (browser, accept cert, GM login, enable module).
3. Open the Campaign tool; pick a character; expand the feats section.
4. **Add merit:** click `+ Add merit` → form appears; fill `Iron Will` / `2` / `Strong-minded.`; click `Add` → chip "Iron Will ••" appears in Merits row within a tick; drift badge appears on the live card.
5. **Add flaw:** repeat with `Bad Sight` / `1` / `Squints.` in the Flaws row.
6. **Add background:** repeat with `Resources` / `3` / `Inheritance.`.
7. **Add boon:** repeat with `Owed Favor` / `0` / `From Camarilla.`.
8. **Cancel:** click `+ Add merit`, then click `Cancel` — form disappears; nothing committed.
9. **Validation:** click `+ Add merit`, leave the name empty — `Add` button is disabled. Set points to 11 — `Add` becomes disabled.
10. **Remove:** hover the new "Iron Will" chip → `×` button fades in; click → confirm dialog → confirm → chip disappears within a tick.
11. **Drift via Compare:** open the Compare modal on a character; verify a "Merit added: Iron Will" / "Merit removed: Iron Will" entry surfaces under the existing diff list.
12. **Roll20:** if a Roll20 character is also connected, expand its feats section. The Roll20 hint banner appears at the top of the section. No `+ Add` chips, no `×` buttons render anywhere.
13. **Disconnect Foundry, re-add a merit:** `+ Add merit` → submit → `window.alert` does NOT fire (per spec §5.6, disconnected Foundry on `target=Live` succeeds as a no-op). The chip does NOT appear (no live update). This is the documented behavior.

Skipping the manual smoke is acceptable if `verify.sh` is green; flag it in the PR description so the user knows.

The `Closes #8` footer is already in Task 8's commit message — merging the resulting branch / PR will auto-close the issue.

---

## Dependency graph

```
Task 0 (pre-flight)
  ▼
Task 1 (db helpers + uuid)
  ▼
Task 2 (router commands + FeatureType)
  ▼
Task 3 (lib.rs registration)
  ▼
[Task 4 (TS types + api.ts), Task 5 (diff.ts)]   ← parallel-safe (disjoint files)
  ▼
Task 6 (chip-X UI)
  ▼
Task 7 (Add form UI)
  ▼
Task 8 (Roll20 hint)
  ▼
Task 9 (final verify + smoke)
```

Tasks 1–3 are sequential (Rust types compile in dependency order). Tasks 4 and 5 can run in parallel — `src/types.ts` + `src/lib/character/api.ts` vs `src/lib/saved-characters/diff.ts`. Tasks 6, 7, 8 all touch `Campaign.svelte` and must be sequential.

---

## Anti-scope (sub-agents must not touch these files)

| Anti-scope file/area | Why |
|---|---|
| Any `vtmtools-bridge/` JS file | No new wire variants; module unchanged. Both verbs use already-shipped `actor.create_feature` / `actor.delete_item_by_id` |
| `src-tauri/migrations/*.sql` | No schema change |
| `src-tauri/src/db/saved_character.rs::update_saved_character` | Whole-blob path is unchanged |
| `src-tauri/src/db/saved_character.rs::db_patch_field` | Field-patch path from #6 is unchanged |
| `src-tauri/src/bridge/foundry/actions/actor.rs` | `build_create_feature` / `build_delete_item_by_id` are pre-shipped FHL Phase 1 helpers — do not touch |
| `src-tauri/src/bridge/roll20/mod.rs` | Roll20 stays passthrough; Roll20 live editing is fast-fail |
| `src/lib/components/CompareModal.svelte` | Diff-rendering is unchanged — just receives more entries from `diffCharacter` |
| Saved cards (`Campaign.svelte:670-686`) | v1 deferral — chip-X / Add on saved-only cards is Phase 2.5 |
| `src/store/savedCharacters.svelte.ts` | Existing store unchanged; no new methods needed (live re-render comes via `bridge://characters-updated`; saved re-render via store reload after our IPC writes) |
| Toast component | None exists — this plan uses `console.error` + `window.alert` |
| `docs/superpowers/specs/**` | Spec is frozen for plan execution |

---

## Verification gate summary

Per CLAUDE.md hard rule: every task ending in a commit runs `./scripts/verify.sh` first.

| Task | `cargo test` adds | `npm run check` impact | `npm run build` impact |
|---|---|---|---|
| 1 | +8 tests in `db::saved_character` | none | none |
| 2 | +13 tests in `tools::character` | none | none |
| 3 | none (registration) | none | command surface grows 39 → 41 |
| 4 | none | mirror types for new commands | new exports compile |
| 5 | none | `diffAdvantages` typed correctly | recompiles |
| 6 | none | new helpers + snippet | recompiles |
| 7 | none | form state + snippet | recompiles |
| 8 | none | conditional hint | recompiles |
| 9 | aggregate green | aggregate green | aggregate green |

Final command-surface count: 39 → 41 (`character_add_advantage`, `character_remove_advantage`).

---

## Pointers

- Spec: `docs/superpowers/specs/2026-05-02-phase-2-character-editing-design.md`
- Sibling plan (Plan A = #7): `docs/superpowers/plans/2026-05-02-stat-editor-ui.md`
- Router (#6, shipped): `docs/superpowers/plans/2026-05-02-character-set-field-router.md`
- FHL Phase 1 (shipped): `src-tauri/src/bridge/foundry/actions/actor.rs::build_create_feature` + `build_delete_item_by_id`
- ARCHITECTURE.md §4 (Tauri IPC + bridge protocol), §6 (color tokens), §7 (errors), §9 (Add a Tauri command)
- `docs/reference/foundry-vtm5e-paths.md` — WoD5e actor schema (item-document shapes)
- `docs/reference/foundry-vtm5e-actor-sample.json` — live actor wire blob; ground truth for `raw.items[]` shape
- Issue: https://github.com/Hampternt/vtmtools/issues/8
