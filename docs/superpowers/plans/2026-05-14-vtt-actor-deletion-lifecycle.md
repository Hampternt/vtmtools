# VTT Actor Deletion Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Propagate Foundry actor deletions through the bridge cache, saved-character store, and Campaign/GM Screen UIs so deleted-in-Foundry actors stop appearing as live rows, saved snapshots get a "deleted" badge, and saved rows have a one-click forget button.

**Architecture:** Replace the merge-only `InboundEvent::CharactersUpdated` enum variant with three precise variants (`CharactersSnapshot` / `CharacterUpdated` / `CharacterRemoved`). The cache replaces a source's slice on snapshot, inserts on update, removes on delete. Foundry's `deleteActor` JS hook gets its own wire shape (`actor_deleted`). A new nullable `deleted_in_vtt_at` column on `saved_characters` (world-scoped via `foundry_world`) is owned by bridge reconciliation; frontend renders a badge and adds a `[Forget saved]` button.

**Tech Stack:** Rust (`sqlx`, `tokio`, `serde`, `async_trait`), Svelte 5 runes, TypeScript, Tauri 2 event system, FoundryVTT JS module (vanilla, no bundler).

**Spec:** `docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md` (read this before starting Task 1).

**Project rules:**
- Every commit step runs `./scripts/verify.sh` first (CLAUDE.md hard rule). A red verify is a blocker; fix in the same task.
- Plan and spec files in `docs/superpowers/*` are gitignored — do NOT `git add` either of them.
- Tests required only where a step explicitly says "tests: required". Wiring/refactor/type-mirror tasks rely on `verify.sh` (cargo test + cargo check + npm check + frontend build).

---

## File Map

**Create:**
- `src-tauri/migrations/0008_saved_character_deleted_in_vtt.sql` — schema add for the nullable timestamp column

**Modify (Rust):**
- `src-tauri/src/db/saved_character.rs` — add `deleted_in_vtt_at` field on `SavedCharacter`; update `db_list` SELECT; add three internal helpers (`db_mark_deleted_in_vtt`, `db_clear_deleted_in_vtt`, `db_reconcile_vtt_presence`) plus `ReconcileStats`; add tests for the helpers
- `src-tauri/src/bridge/foundry/types.rs` — add `FoundryInbound::ActorDeleted { actor_id: String }`; add deserialize round-trip test
- `src-tauri/src/bridge/source.rs` — replace `InboundEvent::CharactersUpdated(...)` with `CharactersSnapshot { source, characters }`, `CharacterUpdated(CanonicalCharacter)`, `CharacterRemoved { source, source_id }`
- `src-tauri/src/bridge/foundry/mod.rs` — emit the three new events; add `actor_deleted → CharacterRemoved` arm; add unit test
- `src-tauri/src/bridge/roll20/mod.rs` — emit `CharactersSnapshot` from `Characters`, `CharacterUpdated` from `CharacterUpdate`
- `src-tauri/src/bridge/mod.rs` — replace the `accept_loop` event match arms with handlers for the three new variants; wire DB side-effects (mark / clear / reconcile)

**Modify (Frontend):**
- `src/lib/saved-characters/api.ts` — add `deletedInVttAt: string | null` to the `SavedCharacter` interface
- `src/store/savedCharacters.svelte.ts` — subscribe to `bridge://characters-updated` in `ensureLoaded`; call `refresh()` on event
- `src/lib/components/CharacterCardShell.svelte` — add `[Forget saved]` button, `"deleted"` badge
- `src/lib/components/gm-screen/CharacterRow.svelte` — accept new optional `saved` prop; render badge in header strip
- `src/tools/GmScreen.svelte` — pass `saved` (via `savedCharacters.findMatch` for live rows, `savedRow` for saved-only) into `CharacterRow`

**Modify (Foundry module JS):**
- `vtmtools-bridge/scripts/translate.js` — split `deleteActor` out of `hookActorChanges`; send `{ type: "actor_deleted", actor_id }`

**Modify (Docs):**
- `ARCHITECTURE.md` — §2 Bridge domain (new `InboundEvent` shape), §6 Invariants (cache source-slice-authoritative), §10 Testing (new test modules)

---

## Task 1: Migration + Foundry inbound `ActorDeleted` variant + SavedCharacter field

**Files:**
- Create: `src-tauri/migrations/0008_saved_character_deleted_in_vtt.sql`
- Modify: `src-tauri/src/db/saved_character.rs:7-18` (struct), `:74-105` (db_list SELECT + row mapping)
- Modify: `src-tauri/src/bridge/foundry/types.rs` (add variant + test)

Tests: required for the new `actor_deleted` deserialize (bridge protocol decoding — CLAUDE.md TDD-on-demand applies).

- [ ] **Step 1: Create the migration file**

Create `src-tauri/migrations/0008_saved_character_deleted_in_vtt.sql` with this exact content:

```sql
-- Adds deleted_in_vtt_at column to saved_characters. ISO-8601 timestamp
-- (matches saved_at / last_updated_at format) recorded when the bridge
-- layer observed the live actor disappearing from its source VTT. NULL
-- means "not known to be deleted" — either never deleted, or the
-- deletion happened before this column existed.
--
-- Set by: bridge::accept_loop on CharacterRemoved events AND by
-- snapshot reconciliation when a saved row's source_id is absent from
-- a fresh CharactersSnapshot of the same Foundry world.
-- Cleared by: an explicit CharacterUpdated event for the same key, or
-- by snapshot reconciliation seeing the source_id reappear.
-- Owned entirely by the bridge layer; save_character /
-- update_saved_character do not touch this column.

ALTER TABLE saved_characters
    ADD COLUMN deleted_in_vtt_at TEXT;
```

No backfill — NULL is the correct "not known to be deleted" state for existing rows.

- [ ] **Step 2: Add `deleted_in_vtt_at` field to `SavedCharacter` struct**

Edit `src-tauri/src/db/saved_character.rs:7-18`. The struct currently ends at line 18 with `last_updated_at: String,`. Add the new field right after it (just before the closing brace at line 18):

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCharacter {
    pub id: i64,
    pub source: SourceKind,
    pub source_id: String,
    pub foundry_world: Option<String>,
    pub name: String,
    pub canonical: CanonicalCharacter,
    pub saved_at: String,
    pub last_updated_at: String,
    /// ISO-8601 timestamp set by the bridge layer when the live actor was
    /// observed to have been deleted from its source VTT. NULL = not
    /// known to be deleted. Owned by the bridge reconciliation paths.
    pub deleted_in_vtt_at: Option<String>,
}
```

- [ ] **Step 3: Update `db_list` SELECT + row mapping**

Edit `src-tauri/src/db/saved_character.rs:76-105`. Update the SELECT to include the new column and the row-builder to populate the new field:

```rust
async fn db_list(pool: &SqlitePool) -> Result<Vec<SavedCharacter>, String> {
    let rows = sqlx::query(
        "SELECT id, source, source_id, foundry_world, name, canonical_json,
                saved_at, last_updated_at, deleted_in_vtt_at
         FROM saved_characters
         ORDER BY id ASC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/saved_character.list: {e}"))?;

    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let source_str: String = r.get("source");
        let source = str_to_source(&source_str)
            .ok_or_else(|| format!("db/saved_character.list: unknown source '{source_str}'"))?;
        let canonical_json: String = r.get("canonical_json");
        let canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
            .map_err(|e| format!("db/saved_character.list: deserialize failed: {e}"))?;
        out.push(SavedCharacter {
            id: r.get("id"),
            source,
            source_id: r.get("source_id"),
            foundry_world: r.get("foundry_world"),
            name: r.get("name"),
            canonical,
            saved_at: r.get("saved_at"),
            last_updated_at: r.get("last_updated_at"),
            deleted_in_vtt_at: r.get("deleted_in_vtt_at"),
        });
    }
    Ok(out)
}
```

- [ ] **Step 4: Add `FoundryInbound::ActorDeleted` variant**

Edit `src-tauri/src/bridge/foundry/types.rs`. The `FoundryInbound` enum currently ends with the `ItemDeleted` variant. Add a new variant **immediately before** `ItemDeleted` (keep alphabetical-ish ordering; both are "deleted" types — put `ActorDeleted` first since actor is a parent of item conceptually):

```rust
    /// A Foundry actor was deleted from its world. Triggered by the
    /// `deleteActor` hook in `vtmtools-bridge/scripts/translate.js`.
    /// `actor_id` is the Foundry actor `_id` (canonical `source_id`).
    /// The deleted actor's body is not shipped — by the time the hook
    /// fires the actor is already gone; the deletion IS the message.
    ActorDeleted {
        actor_id: String,
    },
    /// A Foundry item was deleted from an actor. Triggered by the
    /// `deleteItem` hook in `vtmtools-bridge/scripts/translate.js`.
    /// `actor_id` is the Foundry actor `_id` (canonical `source_id`);
    /// `item_id` is the deleted item's `_id`.
    ItemDeleted {
        actor_id: String,
        item_id: String,
    },
```

- [ ] **Step 5: Add failing test for `actor_deleted` deserialize**

Edit `src-tauri/src/bridge/foundry/types.rs`. In the `#[cfg(test)] mod tests` block at the bottom of the file (next to the existing `foundry_inbound_deserializes_item_deleted` test at `:233`), append:

```rust
    #[test]
    fn foundry_inbound_deserializes_actor_deleted() {
        let wire = r#"{"type":"actor_deleted","actor_id":"actor-xyz"}"#;
        let parsed: FoundryInbound = serde_json::from_str(wire).expect("parses");
        match parsed {
            FoundryInbound::ActorDeleted { actor_id } => {
                assert_eq!(actor_id, "actor-xyz");
            }
            _ => panic!("expected ActorDeleted, got {parsed:?}"),
        }
    }
```

- [ ] **Step 6: Run the new test (and the migration) — expect compile failure**

Run:
```bash
cargo test --manifest-path src-tauri/Cargo.toml foundry_inbound_deserializes_actor_deleted
```

The test should compile-fail at this point because:
- The `ActorDeleted` variant is now defined, so the test code references a valid variant. But `db_list`'s row mapping references `deleted_in_vtt_at: r.get(...)` and the migration runs on test setup. If `sqlx::migrate!` correctly applies `0008_*.sql`, the test passes.

Expected: **PASS**. If FAIL, check that the migration is being picked up (`ls src-tauri/migrations/` to confirm `0008_*` is present) and that the column name is spelled identically in the SELECT and the migration.

- [ ] **Step 7: Run `verify.sh` to confirm full project builds**

```bash
./scripts/verify.sh
```

Expected: green. The new field is plumbed through but unused — no behavior change yet. The new variant is unused too — no behavior change.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/migrations/0008_saved_character_deleted_in_vtt.sql \
        src-tauri/src/db/saved_character.rs \
        src-tauri/src/bridge/foundry/types.rs
git commit -m "$(cat <<'EOF'
feat(bridge): add SavedCharacter.deleted_in_vtt_at column + ActorDeleted wire variant

Migration 0008 adds the nullable timestamp column; SavedCharacter mirrors
it. FoundryInbound gains an ActorDeleted variant so the deleteActor JS
hook can signal removal explicitly instead of mis-routing through
actor_update. No behavior wired up yet — Task 3 connects the cache
handler and source impl.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §1, §2.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: `saved_character.rs` DB helpers + tests

**Files:**
- Modify: `src-tauri/src/db/saved_character.rs` — add `ReconcileStats` struct, three `pub async fn` helpers (no `#[tauri::command]` — internal exports), and a `#[cfg(test)]` test module covering them

Tests: required. These are domain helpers with non-trivial SQL (parameterized IN-list, empty-list guard, world-scoping). The world-isolation test is the regression guard for the bug the spec exists to fix.

- [ ] **Step 1: Write the failing tests**

Edit `src-tauri/src/db/saved_character.rs`. Find the existing `#[cfg(test)] mod tests` block (around `:391`). Inside it, append the following tests. They'll fail compile because `db_mark_deleted_in_vtt`, `db_clear_deleted_in_vtt`, `db_reconcile_vtt_presence`, and `ReconcileStats` don't exist yet.

```rust
    // --- deleted_in_vtt_at helpers ---

    async fn make_foundry_saved(
        pool: &SqlitePool,
        world: Option<&str>,
        source_id: &str,
        name: &str,
    ) -> i64 {
        // Build a minimal CanonicalCharacter and save it.
        let canonical = CanonicalCharacter {
            source: SourceKind::Foundry,
            source_id: source_id.into(),
            name: name.into(),
            controlled_by: None,
            hunger: None,
            health: None,
            willpower: None,
            humanity: None,
            humanity_stains: None,
            blood_potency: None,
            raw: serde_json::Value::Null,
        };
        db_save(pool, &canonical, world.map(|s| s.to_string()))
            .await
            .expect("save")
    }

    async fn read_deleted_in_vtt_at(pool: &SqlitePool, id: i64) -> Option<String> {
        let row = sqlx::query("SELECT deleted_in_vtt_at FROM saved_characters WHERE id = ?")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("fetch");
        row.get("deleted_in_vtt_at")
    }

    #[tokio::test]
    async fn mark_deleted_in_vtt_sets_timestamp() {
        let pool = test_pool().await;
        let id = make_foundry_saved(&pool, Some("World A"), "actor-1", "Alice").await;
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_none(), "starts null");

        let updated = db_mark_deleted_in_vtt(&pool, SourceKind::Foundry, "actor-1")
            .await
            .expect("mark");
        assert!(updated, "row was matched");
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_some(), "now set");
    }

    #[tokio::test]
    async fn mark_deleted_in_vtt_is_idempotent() {
        let pool = test_pool().await;
        let id = make_foundry_saved(&pool, Some("World A"), "actor-1", "Alice").await;
        db_mark_deleted_in_vtt(&pool, SourceKind::Foundry, "actor-1").await.unwrap();
        let first = read_deleted_in_vtt_at(&pool, id).await;
        db_mark_deleted_in_vtt(&pool, SourceKind::Foundry, "actor-1").await.unwrap();
        let second = read_deleted_in_vtt_at(&pool, id).await;
        assert!(first.is_some() && second.is_some(), "set both times");
        // Second call refreshes the timestamp; equality is not required.
    }

    #[tokio::test]
    async fn mark_deleted_in_vtt_no_match_returns_false() {
        let pool = test_pool().await;
        let updated = db_mark_deleted_in_vtt(&pool, SourceKind::Foundry, "no-such-actor")
            .await
            .expect("mark");
        assert!(!updated);
    }

    #[tokio::test]
    async fn clear_deleted_in_vtt_unsets_timestamp() {
        let pool = test_pool().await;
        let id = make_foundry_saved(&pool, Some("World A"), "actor-1", "Alice").await;
        db_mark_deleted_in_vtt(&pool, SourceKind::Foundry, "actor-1").await.unwrap();
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_some());

        let cleared = db_clear_deleted_in_vtt(&pool, SourceKind::Foundry, "actor-1")
            .await
            .expect("clear");
        assert!(cleared, "row was matched");
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_none(), "now null");
    }

    #[tokio::test]
    async fn reconcile_stamps_absent_rows_in_matching_world() {
        let pool = test_pool().await;
        let id_a = make_foundry_saved(&pool, Some("World A"), "actor-a", "Alice").await;
        let id_b = make_foundry_saved(&pool, Some("World A"), "actor-b", "Bob").await;

        // Snapshot from World A contains only actor-a; actor-b must be stamped.
        let stats = db_reconcile_vtt_presence(&pool, "World A", &["actor-a".into()])
            .await
            .expect("reconcile");
        assert_eq!(stats.stamped, 1);

        assert!(read_deleted_in_vtt_at(&pool, id_a).await.is_none(), "present row untouched");
        assert!(read_deleted_in_vtt_at(&pool, id_b).await.is_some(), "absent row stamped");
    }

    #[tokio::test]
    async fn reconcile_clears_returning_rows() {
        let pool = test_pool().await;
        let id = make_foundry_saved(&pool, Some("World A"), "actor-a", "Alice").await;
        db_mark_deleted_in_vtt(&pool, SourceKind::Foundry, "actor-a").await.unwrap();
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_some());

        let stats = db_reconcile_vtt_presence(&pool, "World A", &["actor-a".into()])
            .await
            .expect("reconcile");
        assert_eq!(stats.cleared, 1);
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_none(), "present row cleared");
    }

    #[tokio::test]
    async fn reconcile_is_world_scoped() {
        // The regression guard for the cross-world false-positive bug
        // the spec was written to prevent.
        let pool = test_pool().await;
        let id_a = make_foundry_saved(&pool, Some("World A"), "actor-1", "Alice").await;
        let id_b = make_foundry_saved(&pool, Some("World B"), "actor-2", "Bob").await;

        // Snapshot from World B contains only actor-2. World A's row MUST
        // NOT be stamped — it lives in a different world.
        let stats = db_reconcile_vtt_presence(&pool, "World B", &["actor-2".into()])
            .await
            .expect("reconcile");
        assert_eq!(stats.stamped, 0, "World A rows untouched by World B snapshot");
        assert_eq!(stats.cleared, 0);

        assert!(read_deleted_in_vtt_at(&pool, id_a).await.is_none(), "World A actor-1 untouched");
        assert!(read_deleted_in_vtt_at(&pool, id_b).await.is_none(), "World B actor-2 present in snapshot");
    }

    #[tokio::test]
    async fn reconcile_handles_empty_snapshot() {
        // Empty present_source_ids is a valid input (Foundry sent an
        // actors snapshot with zero actors — e.g. a fresh world).
        // SQL `WHERE source_id NOT IN ()` is a syntax error in SQLite,
        // so the impl must branch on empty input.
        let pool = test_pool().await;
        let id = make_foundry_saved(&pool, Some("World A"), "actor-1", "Alice").await;

        let stats = db_reconcile_vtt_presence(&pool, "World A", &[])
            .await
            .expect("reconcile");
        assert_eq!(stats.stamped, 1, "all rows in world stamped on empty snapshot");
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_some());
    }

    #[tokio::test]
    async fn reconcile_skips_rows_with_null_foundry_world() {
        // Legacy rows with NULL foundry_world are exempt — SQL `=` excludes NULL.
        let pool = test_pool().await;
        let id = make_foundry_saved(&pool, None, "actor-1", "Alice").await;

        let stats = db_reconcile_vtt_presence(&pool, "World A", &[])
            .await
            .expect("reconcile");
        assert_eq!(stats.stamped, 0);
        assert!(read_deleted_in_vtt_at(&pool, id).await.is_none(), "NULL-world row untouched");
    }
```

The tests reuse the existing `test_pool()` helper defined in the same test module (search for `async fn test_pool` to confirm — it should already exist; if not, model it after `db/chronicle.rs`'s test pool).

- [ ] **Step 2: Run the failing tests — expect compile failure**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib db::saved_character::tests
```

Expected: **compile error** — `db_mark_deleted_in_vtt`, `db_clear_deleted_in_vtt`, `db_reconcile_vtt_presence`, and `ReconcileStats` don't exist yet.

- [ ] **Step 3: Add `ReconcileStats` + the three helper functions**

Edit `src-tauri/src/db/saved_character.rs`. Add these definitions at the end of the file, **before** the `#[cfg(test)] mod tests` block:

```rust
/// Result counters returned by `db_reconcile_vtt_presence`.
#[derive(Debug, Default)]
pub struct ReconcileStats {
    /// Rows where `deleted_in_vtt_at` was newly set (was NULL → now non-NULL,
    /// or refreshed from a prior timestamp — both count).
    pub stamped: u64,
    /// Rows where `deleted_in_vtt_at` was cleared (was non-NULL → now NULL).
    pub cleared: u64,
}

/// Set `deleted_in_vtt_at = datetime('now')` for the saved record matching
/// `(source, source_id)`. Idempotent — re-stamps to the latest timestamp
/// if already set. Returns `Ok(true)` if a row was updated, `Ok(false)`
/// if no row matched. Called from `bridge::accept_loop` on
/// `CharacterRemoved` events.
pub async fn db_mark_deleted_in_vtt(
    pool: &SqlitePool,
    source: SourceKind,
    source_id: &str,
) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE saved_characters
            SET deleted_in_vtt_at = datetime('now')
          WHERE source = ? AND source_id = ?"
    )
    .bind(source_to_str(&source))
    .bind(source_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.mark_deleted_in_vtt: {e}"))?;
    Ok(result.rows_affected() > 0)
}

/// Set `deleted_in_vtt_at = NULL` for the saved record matching
/// `(source, source_id)`. No-op if already NULL or row absent. Returns
/// `Ok(true)` if a row was updated. Called from `bridge::accept_loop`
/// on `CharacterUpdated` events.
pub async fn db_clear_deleted_in_vtt(
    pool: &SqlitePool,
    source: SourceKind,
    source_id: &str,
) -> Result<bool, String> {
    let result = sqlx::query(
        "UPDATE saved_characters
            SET deleted_in_vtt_at = NULL
          WHERE source = ? AND source_id = ?
            AND deleted_in_vtt_at IS NOT NULL"
    )
    .bind(source_to_str(&source))
    .bind(source_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.clear_deleted_in_vtt: {e}"))?;
    Ok(result.rows_affected() > 0)
}

/// Foundry-only world-scoped reconciliation. For saved rows with
/// `source = 'foundry' AND foundry_world = foundry_world`:
///   - clear `deleted_in_vtt_at` if `source_id` is in `present_source_ids`
///   - set `deleted_in_vtt_at = datetime('now')` otherwise
/// One transaction. Rows with NULL `foundry_world` are skipped (SQL `=`
/// excludes NULL by definition — legacy / world-less saves are exempt).
///
/// MUST be called only when the snapshot's world is known. Passing a
/// non-empty `foundry_world` from a different source's snapshot is a
/// caller bug — there is no defensive guard here, only the value comparison.
///
/// SQLite's `WHERE col NOT IN ()` is a syntax error; the function
/// branches on `present_source_ids.is_empty()` to skip the clear-step
/// and run only the stamp-step in that case.
pub async fn db_reconcile_vtt_presence(
    pool: &SqlitePool,
    foundry_world: &str,
    present_source_ids: &[String],
) -> Result<ReconcileStats, String> {
    let mut tx = pool.begin().await
        .map_err(|e| format!("db/saved_character.reconcile_vtt_presence: begin: {e}"))?;

    let mut stats = ReconcileStats::default();

    if present_source_ids.is_empty() {
        // Stamp every present row (no live ids to spare). The "IS NULL" guard
        // on deleted_in_vtt_at avoids counting re-stamps as fresh stamps.
        let result = sqlx::query(
            "UPDATE saved_characters
                SET deleted_in_vtt_at = datetime('now')
              WHERE source = 'foundry'
                AND foundry_world = ?
                AND deleted_in_vtt_at IS NULL"
        )
        .bind(foundry_world)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("db/saved_character.reconcile_vtt_presence: stamp-all: {e}"))?;
        stats.stamped = result.rows_affected();
    } else {
        // Build the placeholder list dynamically — sqlx doesn't expand Vec into IN().
        let placeholders = vec!["?"; present_source_ids.len()].join(",");

        // Clear rows that ARE in the snapshot.
        let clear_sql = format!(
            "UPDATE saved_characters
                SET deleted_in_vtt_at = NULL
              WHERE source = 'foundry'
                AND foundry_world = ?
                AND source_id IN ({placeholders})
                AND deleted_in_vtt_at IS NOT NULL"
        );
        let mut clear_q = sqlx::query(&clear_sql).bind(foundry_world);
        for id in present_source_ids {
            clear_q = clear_q.bind(id);
        }
        let cleared = clear_q.execute(&mut *tx).await
            .map_err(|e| format!("db/saved_character.reconcile_vtt_presence: clear: {e}"))?;
        stats.cleared = cleared.rows_affected();

        // Stamp rows that are NOT in the snapshot.
        let stamp_sql = format!(
            "UPDATE saved_characters
                SET deleted_in_vtt_at = datetime('now')
              WHERE source = 'foundry'
                AND foundry_world = ?
                AND source_id NOT IN ({placeholders})
                AND deleted_in_vtt_at IS NULL"
        );
        let mut stamp_q = sqlx::query(&stamp_sql).bind(foundry_world);
        for id in present_source_ids {
            stamp_q = stamp_q.bind(id);
        }
        let stamped = stamp_q.execute(&mut *tx).await
            .map_err(|e| format!("db/saved_character.reconcile_vtt_presence: stamp: {e}"))?;
        stats.stamped = stamped.rows_affected();
    }

    tx.commit().await
        .map_err(|e| format!("db/saved_character.reconcile_vtt_presence: commit: {e}"))?;
    Ok(stats)
}
```

- [ ] **Step 4: Run the helper tests — expect pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib db::saved_character::tests
```

Expected: **all tests pass**. If any fail, inspect the failure message; common issues are typo in column name (`deleted_in_vtt_at`) or forgetting to apply the migration in the test setup (the `test_pool()` helper should call `sqlx::migrate!` — verify that).

- [ ] **Step 5: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. New helpers compile and pass tests; existing tests continue to pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/db/saved_character.rs
git commit -m "$(cat <<'EOF'
feat(db): add saved_character vtt-deletion helpers

Adds db_mark_deleted_in_vtt, db_clear_deleted_in_vtt, and the
world-scoped db_reconcile_vtt_presence (plus ReconcileStats).
World-isolation regression test guards against the cross-world
false-positive that motivated the spec. Empty-snapshot path uses
a stamp-only UPDATE because SQLite's WHERE NOT IN () is invalid.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §2.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Bridge event cutover (atomic)

This is the single largest task by design — replacing `InboundEvent::CharactersUpdated` touches the enum in `source.rs`, both source impls (`foundry/mod.rs`, `roll20/mod.rs`), and `bridge/mod.rs::accept_loop` simultaneously. Splitting these into smaller commits fails compile. Per `feedback_atomic_cluster_commits`.

**Files:**
- Modify: `src-tauri/src/bridge/source.rs:8-25` (the `InboundEvent` enum)
- Modify: `src-tauri/src/bridge/foundry/mod.rs:20-45` (`handle_inbound` impl + the existing test module)
- Modify: `src-tauri/src/bridge/roll20/mod.rs:18-25` (`handle_inbound` impl)
- Modify: `src-tauri/src/bridge/mod.rs:269-327` (the `match event` block in `handle_connection`)

Tests: required for the new Foundry `actor_deleted → CharacterRemoved` mapping (bridge protocol decoding).

- [ ] **Step 1: Write the failing test for `actor_deleted` event mapping**

Edit `src-tauri/src/bridge/foundry/mod.rs`. Inside the existing `#[cfg(test)] mod tests` block (around `:92-118`), append:

```rust
    #[tokio::test]
    async fn actor_deleted_inbound_produces_character_removed_event() {
        let source = FoundrySource;
        let msg = json!({
            "type": "actor_deleted",
            "actor_id": "actor-99",
        });
        let events = source.handle_inbound(msg).await.expect("handles");
        assert_eq!(events.len(), 1);
        match &events[0] {
            InboundEvent::CharacterRemoved { source: src, source_id } => {
                assert_eq!(*src, SourceKind::Foundry);
                assert_eq!(source_id, "actor-99");
            }
            other => panic!("expected CharacterRemoved event, got {other:?}"),
        }
    }
```

- [ ] **Step 2: Run the new test — expect compile failure**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib bridge::foundry::tests::actor_deleted_inbound_produces_character_removed_event
```

Expected: **compile error** — `InboundEvent::CharacterRemoved` doesn't exist yet.

- [ ] **Step 3: Replace `InboundEvent::CharactersUpdated` with three new variants**

Edit `src-tauri/src/bridge/source.rs:8-25`. Replace the existing enum (currently has `CharactersUpdated`, `RollReceived`, `ItemDeleted`) with:

```rust
/// One event emitted from a single inbound frame. A frame may yield
/// zero, one, or many events.
#[derive(Debug, Clone)]
pub enum InboundEvent {
    /// Source pushed a full bulk snapshot. The bridge cache replaces
    /// this source's slice — every entry whose `source` matches is
    /// dropped, then `characters` are inserted. Empty `characters` is
    /// legal and means "this source now has zero characters".
    CharactersSnapshot {
        source: crate::bridge::types::SourceKind,
        characters: Vec<CanonicalCharacter>,
    },
    /// One character was added or changed. The bridge cache inserts or
    /// overwrites a single entry keyed by `(source, source_id)`.
    CharacterUpdated(CanonicalCharacter),
    /// One character was removed from its source. The bridge cache
    /// evicts the entry keyed by `(source, source_id)`.
    CharacterRemoved {
        source: crate::bridge::types::SourceKind,
        source_id: String,
    },
    /// Source pushed a roll result.
    RollReceived(CanonicalRoll),
    /// Foundry-side item deletion — frontend modifier rows tied to
    /// this item must be reaped. Caller in `bridge::mod` runs the DB
    /// delete and emits `modifiers://rows-reaped`.
    ItemDeleted {
        source: crate::bridge::types::SourceKind,
        source_id: String,
        item_id: String,
    },
}
```

Keep the existing `BridgeSource` trait below it unchanged.

- [ ] **Step 4: Update Foundry `handle_inbound` to emit the new events**

Edit `src-tauri/src/bridge/foundry/mod.rs:20-45`. Replace the body of `handle_inbound` (the `match parsed` block):

```rust
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String> {
        let parsed: FoundryInbound = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        match parsed {
            FoundryInbound::Actors { actors } => {
                let canonical: Vec<_> = actors.iter().map(translate::to_canonical).collect();
                Ok(vec![InboundEvent::CharactersSnapshot {
                    source: crate::bridge::types::SourceKind::Foundry,
                    characters: canonical,
                }])
            }
            FoundryInbound::ActorUpdate { actor } => {
                let canonical = translate::to_canonical(&actor);
                Ok(vec![InboundEvent::CharacterUpdated(canonical)])
            }
            FoundryInbound::ActorDeleted { actor_id } => {
                Ok(vec![InboundEvent::CharacterRemoved {
                    source: crate::bridge::types::SourceKind::Foundry,
                    source_id: actor_id,
                }])
            }
            // Hello / Error captured pre-trait in bridge::handle_connection.
            FoundryInbound::Hello { .. } => Ok(vec![]),
            FoundryInbound::Error { .. } => Ok(vec![]),
            FoundryInbound::RollResult { message } => {
                let canonical = translate_roll::to_canonical_roll(&message);
                Ok(vec![InboundEvent::RollReceived(canonical)])
            }
            FoundryInbound::ItemDeleted { actor_id, item_id } => {
                Ok(vec![InboundEvent::ItemDeleted {
                    source: crate::bridge::types::SourceKind::Foundry,
                    source_id: actor_id,
                    item_id,
                }])
            }
        }
    }
```

(The existing `build_set_attribute` and `build_refresh` methods are unchanged.)

- [ ] **Step 5: Update Roll20 `handle_inbound`**

Edit `src-tauri/src/bridge/roll20/mod.rs:18-25`. Replace the `handle_inbound` body:

```rust
    async fn handle_inbound(&self, msg: Value) -> Result<Vec<InboundEvent>, String> {
        let parsed: InboundMsg = serde_json::from_value(msg).map_err(|e| e.to_string())?;
        match parsed {
            InboundMsg::Characters { characters } => {
                let canonical: Vec<_> = characters.iter().map(translate::to_canonical).collect();
                Ok(vec![InboundEvent::CharactersSnapshot {
                    source: crate::bridge::types::SourceKind::Roll20,
                    characters: canonical,
                }])
            }
            InboundMsg::CharacterUpdate { character } => {
                let canonical = translate::to_canonical(&character);
                Ok(vec![InboundEvent::CharacterUpdated(canonical)])
            }
        }
    }
```

(Other methods unchanged.)

- [ ] **Step 6: Update `bridge/mod.rs::accept_loop` event match arms**

Edit `src-tauri/src/bridge/mod.rs:269-327`. Locate the existing `match source.handle_inbound(parsed).await { Ok(events) => { for event in events { match event { ... } } } }` block. Replace the inner `match event` arms with:

```rust
                    match event {
                        InboundEvent::CharactersSnapshot { source, characters } => {
                            // Replace this source's slice of the cache, then emit.
                            {
                                let mut chars = state.characters.lock().await;
                                chars.retain(|_, c| c.source != source);
                                for c in &characters {
                                    chars.insert(c.key(), c.clone());
                                }
                            }
                            let snapshot: Vec<CanonicalCharacter> =
                                state.characters.lock().await.values().cloned().collect();
                            let _ = handle.emit("bridge://characters-updated", snapshot);

                            // Foundry-only: world-scoped saved-character reconciliation.
                            // Use world_title to match what CharacterCardShell stores on save.
                            // If world is unknown, skip — fail-safe (would otherwise stamp
                            // every saved foundry row from every world).
                            if source == SourceKind::Foundry {
                                let world = state.source_info.lock().await
                                    .get(&SourceKind::Foundry)
                                    .and_then(|i| i.world_title.clone());
                                if let (Some(world), Some(db_state)) =
                                    (world, handle.try_state::<crate::DbState>())
                                {
                                    let pool = std::sync::Arc::clone(&db_state.0);
                                    let ids: Vec<String> = characters.iter()
                                        .map(|c| c.source_id.clone())
                                        .collect();
                                    if let Err(e) = crate::db::saved_character::db_reconcile_vtt_presence(
                                        &pool, &world, &ids,
                                    ).await {
                                        eprintln!(
                                            "[bridge:foundry] reconcile_vtt_presence failed: {e}"
                                        );
                                    }
                                }
                            }
                        }
                        InboundEvent::CharacterUpdated(c) => {
                            {
                                let mut chars = state.characters.lock().await;
                                chars.insert(c.key(), c.clone());
                            }
                            let snapshot: Vec<CanonicalCharacter> =
                                state.characters.lock().await.values().cloned().collect();
                            let _ = handle.emit("bridge://characters-updated", snapshot);

                            // Side-effect: clear deleted_in_vtt_at if a saved row matches.
                            // Mostly cosmetic — Foundry undo regenerates actor _id in most
                            // cases. Snapshot reconciliation is the primary clear path.
                            if let Some(db_state) = handle.try_state::<crate::DbState>() {
                                let pool = std::sync::Arc::clone(&db_state.0);
                                if let Err(e) = crate::db::saved_character::db_clear_deleted_in_vtt(
                                    &pool, c.source, &c.source_id,
                                ).await {
                                    eprintln!(
                                        "[bridge:{}] clear_deleted_in_vtt failed: {e}",
                                        c.source.as_str()
                                    );
                                }
                            }
                        }
                        InboundEvent::CharacterRemoved { source, source_id } => {
                            let key = format!("{}:{}", source.as_str(), source_id);
                            {
                                let mut chars = state.characters.lock().await;
                                chars.remove(&key);
                            }
                            let snapshot: Vec<CanonicalCharacter> =
                                state.characters.lock().await.values().cloned().collect();
                            let _ = handle.emit("bridge://characters-updated", snapshot);

                            if let Some(db_state) = handle.try_state::<crate::DbState>() {
                                let pool = std::sync::Arc::clone(&db_state.0);
                                if let Err(e) = crate::db::saved_character::db_mark_deleted_in_vtt(
                                    &pool, source, &source_id,
                                ).await {
                                    eprintln!(
                                        "[bridge:{}] mark_deleted_in_vtt failed: {e}",
                                        source.as_str()
                                    );
                                }
                            }
                        }
                        InboundEvent::RollReceived(roll) => {
                            state.push_roll(roll.clone()).await;
                            let _ = handle.emit("bridge://roll-received", &roll);
                        }
                        InboundEvent::ItemDeleted { source, source_id, item_id } => {
                            // (existing arm — unchanged; keep the existing body verbatim,
                            // including the modifier reap call and the rows-reaped emit)
                            let pool = match handle.try_state::<crate::DbState>() {
                                Some(s) => Arc::clone(&s.0),
                                None => {
                                    eprintln!(
                                        "[bridge:{}] ItemDeleted: no DbState managed, skipping reap",
                                        kind.as_str()
                                    );
                                    continue;
                                }
                            };
                            match crate::db::modifier::db_delete_by_advantage_binding(
                                &pool, &source, &source_id, &item_id,
                            ).await {
                                Ok(ids) if !ids.is_empty() => {
                                    let _ = handle.emit(
                                        "modifiers://rows-reaped",
                                        serde_json::json!({ "ids": ids }),
                                    );
                                }
                                Ok(_) => {}
                                Err(e) => {
                                    eprintln!(
                                        "[bridge:{}] ItemDeleted reap failed: {e}",
                                        kind.as_str()
                                    );
                                }
                            }
                        }
                    }
```

Note: `SourceKind` may need to be re-imported in this file if it isn't already. Check the existing `use crate::bridge::types::{...}` line near the top of `mod.rs` and add `SourceKind` to it if missing.

- [ ] **Step 7: Run all bridge tests — expect green**

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib bridge
```

Expected: **all pass**, including the new `actor_deleted_inbound_produces_character_removed_event` test from Step 1 and the existing `item_deleted_inbound_produces_modifier_reap_event` test.

- [ ] **Step 8: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. Frontend has no consumer of the new variants yet — only the existing `bridge://characters-updated` event (whose payload shape is unchanged). The TS mirror is updated in the next task.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/bridge/source.rs \
        src-tauri/src/bridge/foundry/mod.rs \
        src-tauri/src/bridge/roll20/mod.rs \
        src-tauri/src/bridge/mod.rs
git commit -m "$(cat <<'EOF'
feat(bridge): replace CharactersUpdated with Snapshot/Updated/Removed triad

Split the overloaded InboundEvent::CharactersUpdated into three precise
variants. CharactersSnapshot replaces a source's slice of the cache
(fixes the monotonic-growth bug); CharacterUpdated inserts one entry;
CharacterRemoved evicts one entry. Both source impls emit the new
shapes; accept_loop's match arms wire DB side-effects (mark / clear /
reconcile) onto the saved_characters store.

Foundry-only reconciliation is world-scoped via SourceInfo.world_title
to prevent cross-world false positives.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §1, §2.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Frontend type mirror + `savedCharacters` store wiring

**Files:**
- Modify: `src/lib/saved-characters/api.ts` (add field to interface)
- Modify: `src/store/savedCharacters.svelte.ts` (subscribe to bridge event, call refresh)

Tests: not required. Type mirror + event subscription is wiring. `verify.sh` is the gate.

- [ ] **Step 1: Add `deletedInVttAt` to the TS `SavedCharacter` interface**

Edit `src/lib/saved-characters/api.ts`. Find the `SavedCharacter` interface (it has fields `id`, `source`, `sourceId`, `foundryWorld`, `name`, `canonical`, `savedAt`, `lastUpdatedAt`). Add the new field after `lastUpdatedAt`:

```ts
export interface SavedCharacter {
  id: number;
  source: SourceKind;
  sourceId: string;
  foundryWorld: string | null;
  name: string;
  canonical: BridgeCharacter;
  savedAt: string;
  lastUpdatedAt: string;
  /** ISO-8601 timestamp set by the bridge when the live actor was
   *  observed deleted from its source VTT. null = not known to be deleted.
   *  Owned by the bridge; saving / updating does not touch this. */
  deletedInVttAt: string | null;
}
```

- [ ] **Step 2: Subscribe `savedCharacters` store to `bridge://characters-updated`**

Edit `src/store/savedCharacters.svelte.ts`. At the top, add the Tauri event import (if not already imported):

```ts
import { listen } from '@tauri-apps/api/event';
```

Modify the `ensureLoaded` method to install the subscription after the initial fetch. Find the existing `ensureLoaded`:

```ts
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
  },
```

Replace it with:

```ts
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
    // Re-fetch the saved list whenever the bridge updates — that's how
    // the new deletedInVttAt flag (stamped server-side by the bridge
    // reconciliation paths) propagates to the UI. One local SQL query
    // per event; cheap.
    await listen('bridge://characters-updated', () => {
      void refresh();
    });
  },
```

- [ ] **Step 3: Run `npm run check`**

```bash
npm run check
```

Expected: clean. The new field is plumbed through; the new subscription compiles.

- [ ] **Step 4: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/saved-characters/api.ts src/store/savedCharacters.svelte.ts
git commit -m "$(cat <<'EOF'
feat(saved-characters): mirror deletedInVttAt + subscribe to bridge updates

Adds the new column's TS mirror to the SavedCharacter interface and
wires the savedCharacters store to re-fetch on every
bridge://characters-updated event. This is how the deletion flag
(written server-side by bridge reconciliation) propagates to the UI
without manual reload.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §3.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: `CharacterCardShell` — Forget button + "deleted" badge

**Files:**
- Modify: `src/lib/components/CharacterCardShell.svelte`

Tests: not required (no frontend test framework per ARCHITECTURE.md §10).

- [ ] **Step 1: Add the Forget button and the deleted badge**

Edit `src/lib/components/CharacterCardShell.svelte`. Replace the entire `<div class="shell-rail">` block (currently at `:26-46`) with the version below. Two changes:
1. Add a new `<span class="vtt-deleted-badge">` next to the existing `drift-badge`, conditional on `saved?.deletedInVttAt`.
2. Add a `<button class="btn-forget">` to the saved-branch action group, one-click forget with no confirm.

```svelte
<div class="card-shell">
  <div class="shell-rail">
    <span class="drag" aria-hidden="true" title="Drag (reserved for GM screen)">⋮⋮</span>
    <SourceAttributionChip source={character.source} />
    {#if drift}
      <span class="drift-badge" title="Live differs from saved snapshot">drift</span>
    {/if}
    {#if saved?.deletedInVttAt}
      <span class="vtt-deleted-badge"
        title="Deleted in {character.source === 'foundry' ? 'Foundry' : 'Roll20'} on {saved.deletedInVttAt}">deleted</span>
    {/if}
    <span class="rail-spacer"></span>
    <div class="actions">
      {#if saved}
        <button type="button" class="btn-save"
          onclick={() => onCompare(saved, character)}>Compare</button>
        <button type="button" class="btn-save"
          onclick={() => savedCharacters.update(saved.id, character)}
          disabled={savedCharacters.loading}>Update saved</button>
        <button type="button" class="btn-save btn-forget"
          onclick={() => savedCharacters.delete(saved.id)}
          disabled={savedCharacters.loading}>Forget saved</button>
      {:else}
        <button type="button" class="btn-save"
          onclick={saveCharacter}
          disabled={savedCharacters.loading}>Save locally</button>
      {/if}
    </div>
  </div>
  <CharacterCard {character} />
</div>
```

- [ ] **Step 2: Add the badge + Forget button styles**

In the same file's `<style>` block (currently at `:50-93`), add the following two rules **inside the `<style>` block, after the existing `.drift-badge` rule**. No new CSS tokens — composites of existing ones.

```css
  .vtt-deleted-badge {
    background: color-mix(in srgb, var(--text-muted) 40%, transparent);
    color: var(--text-primary);
    font-size: calc(0.6rem * var(--card-scale, 1));
    padding: 0 0.4em;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-weight: 600;
  }
  .btn-forget {
    color: var(--text-muted);
  }
  .btn-forget:hover:not(:disabled) {
    color: var(--accent-amber);
    border-color: var(--accent-amber);
  }
```

- [ ] **Step 3: Run `npm run check`**

```bash
npm run check
```

Expected: clean. Svelte 5 runes mode + TypeScript should accept the new conditional render and the additional button.

- [ ] **Step 4: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/CharacterCardShell.svelte
git commit -m "$(cat <<'EOF'
feat(campaign): Forget-saved button + vtt-deleted badge on card shell

Adds the one-click [Forget saved] action and the "deleted" badge that
fires whenever saved.deletedInVttAt is set. No confirm prompt (single-
user offline tool); badge styling composes existing tokens, no new CSS
tokens introduced.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §3.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: GM Screen — pair saved with rows and render the badge

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (accept new `saved?` prop; render badge)
- Modify: `src/tools/GmScreen.svelte` (compute `saved` per row; pass into `CharacterRow`)

Tests: not required.

- [ ] **Step 1: Add a `saved?` prop to `CharacterRow`**

Edit `src/lib/components/gm-screen/CharacterRow.svelte`. Find the `Props` interface (around `:15-19`). Extend it:

```ts
  import type { SavedCharacter } from '$lib/saved-characters/api';

  interface Props {
    character: BridgeCharacter;
    activeFilterTags: Set<string>;
    showHidden: boolean;
    saved?: SavedCharacter | null;
  }
  let { character, activeFilterTags, showHidden, saved = null }: Props = $props();
```

Locate the row's visible header strip (where the character's name / source pip / drift badge currently render — search for an existing badge or the character name to find the spot). Insert immediately after that name/source block, conditional render:

```svelte
    {#if saved?.deletedInVttAt}
      <span class="vtt-deleted-badge"
        title="Deleted in {character.source === 'foundry' ? 'Foundry' : 'Roll20'}">deleted</span>
    {/if}
```

Add the matching style at the end of the file's `<style>` block (same composition as the card shell — copy verbatim from Task 5 Step 2; the duplication is acceptable per the spec's "two render sites" reality):

```css
  .vtt-deleted-badge {
    background: color-mix(in srgb, var(--text-muted) 40%, transparent);
    color: var(--text-primary);
    font-size: 0.6rem;
    padding: 0 0.4em;
    border-radius: 999px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    font-weight: 600;
  }
```

Note: `var(--card-scale)` does NOT exist in `CharacterRow`'s scope (that token is set by `Campaign.svelte`'s density toggle). Use a literal `0.6rem` here.

- [ ] **Step 2: Pass `saved` into `CharacterRow` from `GmScreen.svelte`**

Edit `src/tools/GmScreen.svelte`. The `displayCharacters` derivation at `:60-67` builds a unified list of `BridgeCharacter`s. The `CharacterRow` invocation is at `:170-174`. Update it to compute and pass `saved`:

Add this helper inside the `<script>` block (place it near the other derivations, e.g. just after `displayCharacters`):

```ts
  // Per-row saved match, used to render the vtt-deleted badge in CharacterRow.
  // For live rows: lookup via savedCharacters.findMatch.
  // For saved-only rows: look up by source+sourceId in the saved list (the
  //   character object IS the saved canonical, but we need the SavedCharacter
  //   wrapper to read deletedInVttAt).
  function findSaved(c: BridgeCharacter): { id: number; deletedInVttAt: string | null } | null {
    const match = savedCharacters.list.find(
      s => s.source === c.source && s.sourceId === c.source_id,
    );
    return match ?? null;
  }
```

Update the `CharacterRow` invocation to pass it:

```svelte
            <CharacterRow
              character={char}
              activeFilterTags={modifiers.activeFilterTags}
              showHidden={modifiers.showHidden}
              saved={findSaved(char)}
            />
```

(Note: `CharacterRow`'s `saved` prop is typed `SavedCharacter | null | undefined`; passing a narrower `{ id, deletedInVttAt }` works because the component only reads `deletedInVttAt`. If TS complains, widen `findSaved`'s return to `SavedCharacter | null` by returning the full match object instead.)

- [ ] **Step 3: Run `npm run check`**

```bash
npm run check
```

Expected: clean. If TS narrows the `findSaved` return type and complains about the assignment, change `findSaved` to return `SavedCharacter | null` (return the `match` directly without the projection).

- [ ] **Step 4: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte src/tools/GmScreen.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): render vtt-deleted badge on character rows

CharacterRow gains an optional saved prop; GmScreen pairs each row with
its SavedCharacter via savedCharacters.findMatch. Same badge shape as
the campaign card shell — no new tokens.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §3.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Foundry module — split `deleteActor` out of the shared hook loop

**Files:**
- Modify: `vtmtools-bridge/scripts/translate.js:41-55` (`hookActorChanges`)

Tests: not required (no JS test harness; manual smoke validates in Task 9).

- [ ] **Step 1: Replace `hookActorChanges`**

Edit `vtmtools-bridge/scripts/translate.js:41-55`. Replace the existing `hookActorChanges` function with:

```js
export function hookActorChanges(socket) {
  // updateActor / createActor: send the full actor payload as actor_update.
  for (const ev of ["updateActor", "createActor"]) {
    Hooks.on(ev, (actor) => {
      if (!socket || socket.readyState !== WebSocket.OPEN) return;
      try {
        socket.send(JSON.stringify({
          type: "actor_update",
          actor: actorToWire(actor),
        }));
      } catch (e) {
        console.warn(`[${MODULE_ID}] failed to push ${ev}:`, e);
      }
    });
  }
  // deleteActor: send actor_deleted with just the id. The actor object
  // passed to the hook is the just-deleted record; shipping its body
  // would mislead the desktop into re-caching a corpse. The deletion IS
  // the message. Matches the deleteItem path's item_deleted shape.
  Hooks.on("deleteActor", (actor) => {
    if (!socket || socket.readyState !== WebSocket.OPEN) return;
    try {
      socket.send(JSON.stringify({
        type: "actor_deleted",
        actor_id: actor.id,
      }));
    } catch (e) {
      console.warn(`[${MODULE_ID}] failed to push deleteActor:`, e);
    }
  });
}
```

- [ ] **Step 2: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. `verify.sh` doesn't typecheck the Foundry module JS — this is a smoke step, not a correctness check. Real correctness is validated end-to-end in Task 9.

- [ ] **Step 3: Commit**

```bash
git add vtmtools-bridge/scripts/translate.js
git commit -m "$(cat <<'EOF'
fix(foundry-bridge): send actor_deleted on deleteActor hook

Previously deleteActor was routed through the shared loop and sent
actor_update with the just-deleted actor's body — the desktop received
it as an update and re-cached the corpse, producing the bug where
deleted actors persisted in Campaign and GM Screen. Now split out:
actor_deleted carries only actor_id, matching the deleteItem precedent.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §1.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: `ARCHITECTURE.md` updates

**Files:**
- Modify: `ARCHITECTURE.md` (§2 Bridge domain, §6 Invariants, §10 Testing)

Tests: not applicable (docs).

- [ ] **Step 1: Update §2 Bridge domain — `InboundEvent` enum**

Edit `ARCHITECTURE.md`. Find the `InboundEvent` Rust code block in §2 (search for `pub enum InboundEvent {`). Replace it with the new 5-variant shape:

```rust
/// One event emitted from a single inbound frame. A frame may yield zero,
/// one, or many events.
pub enum InboundEvent {
    /// Bulk truth from one source. The cache replaces this source's
    /// slice (drops every prior entry whose `source` matches, then
    /// inserts the new set). Empty `characters` is legal — "this source
    /// now has zero characters". Fires from Roll20 `characters` /
    /// Foundry `actors`.
    CharactersSnapshot { source: SourceKind, characters: Vec<CanonicalCharacter> },
    /// One character added or updated. Cache inserts/overwrites one entry.
    CharacterUpdated(CanonicalCharacter),
    /// One character removed from its source. Cache evicts one entry.
    /// Foundry emits this from `deleteActor`; Roll20's wire protocol has
    /// no per-event deletion yet (relies on snapshot reconciliation).
    CharacterRemoved { source: SourceKind, source_id: String },
    RollReceived(CanonicalRoll),
    ItemDeleted { source: SourceKind, source_id: String, item_id: String },
}
```

Also update the prose that follows (e.g. "`Vec<CanonicalCharacter>` events carries the merged cache across all connected sources") if it still references the old single-variant model. Adjust to reflect the new tri-variant split.

- [ ] **Step 2: Add an invariant to §6**

Find the §6 Invariants bullet list. Add this bullet (place it near the other bridge-state bullets):

```markdown
- The merged characters cache is source-slice-authoritative on
  `CharactersSnapshot`: when one source sends a fresh snapshot, every
  prior cache entry from that source is dropped before the new set is
  inserted. The cache never carries entries from a source whose latest
  snapshot omitted them. This is the post-fix replacement for the
  earlier merge-only semantic that produced ghost-character bugs.
```

- [ ] **Step 3: Update §10 Testing — list new test modules**

Find the §10 list of `#[cfg(test)] mod tests` modules (currently lists `shared/dice.rs`, `shared/resonance.rs`, `db/dyscrasia.rs`, `db/chronicle.rs`, `db/node.rs`, `db/edge.rs`, `tools/export.rs`). Add the new ones:

```markdown
- Rust unit tests live as `#[cfg(test)] mod tests` inside each
  source file. Current test modules: `shared/dice.rs`,
  `shared/resonance.rs`, `db/dyscrasia.rs`, `db/chronicle.rs`,
  `db/node.rs`, `db/edge.rs`, `db/saved_character.rs`,
  `tools/export.rs`, `bridge/foundry/mod.rs`,
  `bridge/foundry/types.rs`. (Run
  `grep -rn "#\[cfg(test)\]" src-tauri/src` to confirm current
  state before editing; `db/chronicle.rs` currently carries two
  `#[cfg(test)]` annotations.)
```

(Adjust the wording to match the existing sentence shape; the list above is illustrative — keep the rest of the paragraph as-is.)

- [ ] **Step 4: Run `verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. Docs-only change.

- [ ] **Step 5: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
docs(architecture): document InboundEvent triad and snapshot-slice invariant

§2 Bridge domain replaces the documented InboundEvent::CharactersUpdated
with the new Snapshot/Updated/Removed triad. §6 adds the source-slice-
authoritative cache invariant. §10 picks up the new test modules added
in this branch.

Per spec docs/superpowers/specs/2026-05-14-vtt-actor-deletion-lifecycle-design.md §4.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Task 9: End-to-end manual smoke test + branch code review

This is the final verification gate per CLAUDE.md ("After ALL plan tasks are committed, run a SINGLE `code-review:code-review` against the full branch diff"). No file modifications.

- [ ] **Step 1: Build + run the app**

```bash
npm run tauri dev
```

Wait for the Foundry connection to come up (green pip in Campaign tool's status bar). If you can't reach a real Foundry instance, document this and skip to the code review — the Rust test suite already covers the deterministic logic, and the JS-side wiring is mechanically straightforward.

- [ ] **Step 2: Walk through the 7 smoke scenarios from the spec**

For each scenario, observe the UI and either tick it as a pass or document the deviation:

1. **Saved + deleted in live:** With Foundry connected and one actor `[Save locally]`'d, delete the actor in Foundry. Expect: live row disappears from Campaign and GM Screen; saved row shows a "deleted" badge with a `[Forget saved]` button.
2. **One-click forget:** Click `[Forget saved]`. Expect: row vanishes immediately, no confirm prompt.
3. **Unsaved + deleted in live:** Delete an unsaved actor in Foundry. Expect: row disappears entirely from both tools, no badge, no residue.
4. **Re-create:** Re-create an actor in Foundry that was previously saved + flagged deleted (same `_id` if Foundry preserves it on undo). Expect: badge clears automatically.
5. **Restart persistence:** Restart vtmtools with a previously-deleted-and-saved actor. Expect: badge is still present (persisted via the new DB column).
6. **Connection race:** Open Campaign while Foundry is connecting (race). Expect: no transient false-positive badges.
7. **World-switch isolation (CRITICAL — the regression guard):** Save characters from Foundry world A. Close Foundry, open Foundry to world B, let it connect to vtmtools. Expect: saved characters from world A show NO "deleted" badge.

- [ ] **Step 3: Run the full Rust test suite once more for confidence**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 4: Run code review against the branch**

Invoke the project's `code-review:code-review` skill against the full branch diff (every task's commits since the branch-point). The CLAUDE.md "Lean plan execution" override states this is the ONLY quality-review pass for the plan — no per-task reviewer subagents were run.

If the review surfaces blocking issues, fix them in a new commit (do NOT amend prior commits). If non-blocking suggestions, decide per-issue whether to address now or defer to a follow-up issue.

- [ ] **Step 5: (If everything green) prepare for merge**

If the user authorizes merging:
1. Confirm the branch passes `verify.sh` and the code review.
2. Surface the proposed `Closes #N` footer if a GitHub issue maps to this work (per CLAUDE.md "Closes #N" convention).
3. Wait for the user to merge or open a PR — do NOT push or merge without explicit authorization.

---

## Plan self-review (executed before handing this plan back)

**Spec coverage check:**
- §1 Bridge layer wire/types/events/handler → Tasks 1, 3 (Foundry types), Task 3 (event enum + handler), Task 7 (JS wire). ✓
- §2 Saved-character backend (migration, type mirror, 3 helpers, bridge call-sites) → Task 1 (migration + type), Task 2 (helpers + tests), Task 3 (call-sites). ✓
- §3 Frontend UX (store wiring, Forget button, deleted badge, GM row badge) → Tasks 4, 5, 6. ✓
- §4 ARCHITECTURE.md updates → Task 8. ✓
- Verification (Rust tests, manual smoke) → Tasks 1/2/3 (Rust), Task 9 (smoke). ✓

**Placeholder scan:**
- No "TBD" / "TODO" / "implement later" markers in any task. ✓
- Every code step shows the actual code. ✓
- All file paths are exact. ✓

**Type consistency:**
- `ReconcileStats { stamped: u64, cleared: u64 }` defined in Task 2; used in Task 3's reconcile call (return discarded — only the error path is logged). ✓
- `db_mark_deleted_in_vtt` / `db_clear_deleted_in_vtt` / `db_reconcile_vtt_presence` signatures consistent across Tasks 2 and 3. ✓
- `InboundEvent::{CharactersSnapshot, CharacterUpdated, CharacterRemoved}` variant names + payload shapes match between source.rs (Task 3 Step 3), source impls (Task 3 Steps 4-5), and the cache handler (Task 3 Step 6). ✓
- `deletedInVttAt` (TS camelCase, Tasks 4-6) mirrors `deleted_in_vtt_at` (Rust snake_case, Tasks 1-3) via the `#[serde(rename_all = "camelCase")]` already on `SavedCharacter`. ✓
