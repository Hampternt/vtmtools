# Library Sync — Plan C: Pull + attribution + dedup

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plan C tasks are committed (i.e., after the last of Plans A + B + C lands), run a SINGLE `code-review:code-review` against the full Phase 4 branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** Close the import half of Library Sync:
- **#14 — Pull library items ← Foundry storyteller.** Add a "Pull from active world" action to the Advantages tool. On click, send `bridge.subscribe { collection: "item" }` (Plan B shipped the wire path), wait for the snapshot, filter to feature-type items (merit/flaw/background/boon), present a confirmation summary, then write rows to the local `advantages` table with `is_custom = 1` + populated `source_attribution`. Disciplines are out of scope for this milestone (deferred per architect recommendation; reservation paragraph added to ARCHITECTURE.md in Plan A).
- **#15 — Source-attribution surfacing.** Render a source chip on each AdvantagesManager row whose `source_attribution` is non-null. Reuses the `SourceAttributionChip` component already shipped in Phase 1.
- **#16 — Dedup / conflict resolution.** Auto-version-suffix on local name collision: `<name> (FVTT — <world>)`. No interactive prompt, no review modal. Collision is detected against the SAME `(name, kind, source_attribution.world)` triple — two same-name merits from different worlds coexist by world-suffix; pulling the SAME merit twice from the SAME world is treated as an update (preserves row id, refreshes attribution timestamp).

**Architecture:** Plan B's `item` subscription delivers `CanonicalWorldItem` rows into `BridgeState.world_items` and emits `bridge://foundry/items-updated` on every change. Plan C's importer is a Tauri command `import_advantages_from_world` that snapshot-reads the bridge cache (no new wire trip — the data is already in-memory), filters to `kind == "feature"` items with featuretype ∈ {merit, flaw, background, boon}, applies dedup-suffix logic against existing local rows, INSERTs new rows / UPDATEs matching world+name rows, and returns a per-row outcome list (`[{name, kind, action: 'inserted' | 'updated' | 'skipped'}]`) for UI summary.

**Subscription lifecycle:** The "Pull from world" button is the ONLY consumer of the `item` subscription in v1. Plan C's import flow:
1. Frontend calls `bridge_subscribe(SourceKind::Foundry, 'item')` (a thin wrapper new in Plan C, since auto-subscribe-on-mount would be wasteful).
2. Waits up to 2 seconds for the next `bridge://foundry/items-updated` event (or uses the existing cache if already populated).
3. Calls `import_advantages_from_world()` Tauri command.
4. Frontend optionally calls `bridge_unsubscribe(SourceKind::Foundry, 'item')` to stop further updates (low cost to leave subscribed; recommend leaving subscribed for the session so re-imports show fresh data).

The Tauri-side subscribe/unsubscribe wrappers are new — Plan 0 shipped the `bridge.subscribe` JSON envelope, but no Tauri command exposed it (auto-subscribe-actors was hardcoded in `bridge.js`). Plan C is the first consumer to need a dynamic subscribe path.

**Tech Stack:** Rust (`sqlx`, `serde`, `serde_json`, `tauri`, `chrono`), TypeScript / Svelte 5 runes, SQLite.

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 4 (sketch) + architect-advisor recommendation. Source sketches in GitHub issues #14, #15, #16.

**Architecture reference:** `ARCHITECTURE.md` §3 (storage strategy — `is_custom` tri-state post-Plan-A), §4 (IPC inventory — adds 3 new commands), §5 (only `db/*` talks to SQLite), §7 (error prefix `db/advantage.import:`), §9 ("Add a Tauri command" + the "Add a library kind" rule from Plan A).

**Depends on:**
- Plan A — `Advantage.kind` enum + `source_attribution` column + tri-state `is_custom`.
- Plan B — `bridge.subscribe { collection: "item" }` wire path + `BridgeState.world_items` cache + `bridge://foundry/items-updated` event.

**Issues closed:** #14, #15, #16. Milestone 4 complete.

---

## File structure

### New files
- `src/lib/library/importer.ts` — pure TS functions: `dedupeKey(name, kind, world)`, `collideAndSuffix(name, world)`, `summarizeImport(outcomes)`. Pure helpers; testable without IPC. Used by the frontend "Pull" button to format the post-import toast.
- `src-tauri/src/db/advantage_import.rs` (or extend `db/advantage.rs` — see Task 1 for which) — `import_advantages_from_world(state, world_title)` Tauri command that reads the bridge cache, dedupes, inserts/updates, returns outcomes.
- (Optionally) `src/lib/components/PullFromWorldButton.svelte` — small wrapper for the button + loading state + summary modal/toast. Decide in Task 6 whether this earns its own component or stays inline in AdvantagesManager.

### Modified files
- `src-tauri/src/db/advantage.rs` — add `db_find_by_dedup_key(pool, name, kind, world) -> Result<Option<Advantage>, String>` (looks up `WHERE name = ? AND kind = ? AND json_extract(source_attribution, '$.worldTitle') = ?`); add `db_upsert_imported(pool, name, kind, description, properties, source_attribution) -> Result<ImportOutcome, String>` that runs the dedup-suffix logic.
- `src-tauri/src/bridge/commands.rs` — add `bridge_subscribe(source: SourceKind, collection: String)` + `bridge_unsubscribe(source: SourceKind, collection: String)` Tauri commands that compose `bridge::foundry::actions::bridge::build_subscribe / build_unsubscribe` and route via `send_to_source_inner`.
- `src-tauri/src/lib.rs` — register `import_advantages_from_world`, `bridge_subscribe`, `bridge_unsubscribe` in `invoke_handler!`.
- `src/lib/library/api.ts` — extend with `importAdvantagesFromWorld(): Promise<ImportOutcome[]>` + `subscribeToWorldItems(): Promise<void>` + `unsubscribeFromWorldItems(): Promise<void>` typed wrappers.
- `src/tools/AdvantagesManager.svelte` — add "Pull from world" button (Foundry-connected only); on click → subscribe → wait/snapshot → import → summary toast. Add source chip on rows with non-null `sourceAttribution`. Add tri-state filter chip (corebook / local / imported) to the existing filter row.
- `src/lib/components/AdvantageCard.svelte` — render the source chip (reuse Phase 1's `SourceAttributionChip`).
- `src/types.ts` — add `ImportOutcome` discriminated union (`'inserted' | 'updated' | 'skipped'`).
- `ARCHITECTURE.md` §4 — append `bridge_subscribe`, `bridge_unsubscribe`, `import_advantages_from_world` to IPC inventory; bump total 65 → 68.

### Files explicitly NOT touched
- `src-tauri/src/bridge/foundry/types.rs` — Plan B frozen.
- `src-tauri/src/bridge/foundry/actions/storyteller.rs` — Plan B frozen (push side only; Plan C is pull).
- `vtmtools-bridge/scripts/*` — Plan B frozen (the JS-side `item` subscriber + `storyteller.*` umbrella are stable).
- `src-tauri/src/db/dyscrasia.rs` — separate row shape; out of scope.
- Disciplines schema — deferred per partitioning rule.

---

## Task overview

| # | Task | Depends on | Tests |
|---|---|---|---|
| 1 | Add `db::advantage::db_find_by_foundry_id` + `db_collides_locally` + `db_upsert_imported` + tests | Plan A | YES (6 tests; includes re-pull-of-secondary-world regression case) |
| 2 | Add `import_advantages_from_world` Tauri command (reads bridge cache + DB; bumps ARCH §4 total 65 → 66 in-commit) | 1 | YES (1 integration test against in-memory pool + fake `world_items` cache) |
| 3 | Add `bridge_subscribe` + `bridge_unsubscribe` Tauri commands (bumps ARCH §4 total 66 → 68 in-commit) | none (independent of 1/2) | NO (compose existing builders; covered by manual smoke in Task 8) |
| 4 | Register 3 new commands in `lib.rs` (no ARCH change — declarations already documented) | 1, 2, 3 | NO |
| 5 | Extend `src/lib/library/api.ts` + add `importer.ts` pure helpers + tests | 4 | YES (vitest if available; otherwise skip — verify the testing convention in the repo first) |
| 6 | Update `AdvantagesManager.svelte` with Pull button + source chip + tri-state filter | 5 + Plan A's existing UI work | NO (manual smoke covers it) |
| 7 | ~~ARCHITECTURE.md §4 updates~~ — REMOVED; work distributed inline into Tasks 2 and 3 per CLAUDE.md same-commit rule | — | — |
| 8 | Final verification gate (incl. live-Foundry pull smoke) | all | runs `./scripts/verify.sh` + manual E2E |

Tasks 1 and 3 are independent and can dispatch in parallel. Task 2 depends on Task 1. Tasks 5–6 are sequential after Task 4. Task 7 is a placeholder (skip).

---

## Task 1: DB helpers — `db_find_by_foundry_id` + `db_collides_locally` + `db_upsert_imported`

**Goal:** Two new internal helpers in `db/advantage.rs` that own the dedup-and-import logic at the DB layer. Tauri-callable wrapper sits on top in Task 2.

**Dedup rule (architect lock-in):** Two imported advantages are "the same" iff they share `(source_attribution->>foundryId, source_attribution->>worldTitle)` — i.e. the same Foundry document `_id` in the same Foundry world. The Foundry document `_id` is stable across re-pulls and is already carried on `CanonicalWorldItem.id` (Plan B). Keying dedup on the **immutable** Foundry id (not the mutable display name) preserves re-pull idempotency even after a row's display name has been suffixed to resolve a local name collision. The display name remains the suffix-collision key (for surface uniqueness in the UI), but it is no longer the identity key. If both rows are local (`source_attribution IS NULL`), `name + kind` is the only available key — but that case is impossible at import time because every imported row has non-null attribution. The unique-suffix collision case for import is: imported `<name>` collides with an existing LOCAL or differently-attributed row of same `(name, kind)` → suffix the import's `name` to `<name> (FVTT — <world>)` before INSERT; the row is still identified by its `foundryId` going forward.

**Files:**
- Modify: `src-tauri/src/db/advantage.rs`

**Anti-scope:** Do NOT add a Tauri command in this task. Do NOT touch the existing `db_insert` / `db_update` / `db_delete` paths — the import path is a parallel function with its own semantics.

**Depends on:** Plan A (`AdvantageKind`, `source_attribution` column).

**Invariants cited:** ARCH §5 (only `db/*` talks to SQLite), §7 (error prefix `db/advantage.upsert_imported:` and `db/advantage.find_by_foundry_id:`).

**Tests required:** YES — 6 tests minimum (one per dedup branch + the re-pull-of-secondary-world regression case).

- [ ] **Step 1: Add `db_find_by_foundry_id`**

Primary identity lookup, keyed on the immutable Foundry document id (carried via `source_attribution.foundryId`) scoped by world title. This is the lookup that makes re-pulls idempotent regardless of name suffixing.

In `src-tauri/src/db/advantage.rs`, near the other internal helpers:

```rust
pub(crate) async fn db_find_by_foundry_id(
    pool: &SqlitePool,
    foundry_id: &str,
    world_title: &str,
) -> Result<Option<Advantage>, String> {
    let rows = sqlx::query(
        "SELECT id, name, description, tags_json, properties_json, is_custom,
                kind, source_attribution
         FROM advantages
         WHERE json_extract(source_attribution, '$.foundryId')  = ?
           AND json_extract(source_attribution, '$.worldTitle') = ?
         LIMIT 1"
    )
    .bind(foundry_id)
    .bind(world_title)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/advantage.find_by_foundry_id: {e}"))?;

    match rows {
        Some(r) => {
            let tags_json: String = r.get("tags_json");
            let properties_json: String = r.get("properties_json");
            let kind_db: String = r.get("kind");
            let source_attribution: Option<String> = r.try_get("source_attribution").ok();
            Ok(Some(Advantage {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                kind: str_to_kind(&kind_db)?,
                tags: deserialize_tags(&tags_json)?,
                properties: deserialize_properties(&properties_json)?,
                is_custom: r.get::<bool, _>("is_custom"),
                source_attribution: source_attribution
                    .and_then(|s| serde_json::from_str(&s).ok()),
            }))
        }
        None => Ok(None),
    }
}
```

- [ ] **Step 2: Add `db_collides_locally`**

Helper that detects collision against any row with the same name+kind regardless of attribution (for the suffix path):

```rust
pub(crate) async fn db_collides_locally(
    pool: &SqlitePool,
    name: &str,
    kind: AdvantageKind,
) -> Result<bool, String> {
    let kind_str = kind_to_str(kind);
    let row = sqlx::query("SELECT COUNT(*) as c FROM advantages WHERE name = ? AND kind = ?")
        .bind(name)
        .bind(kind_str)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("db/advantage.collides_locally: {e}"))?;
    let c: i64 = row.get("c");
    Ok(c > 0)
}
```

- [ ] **Step 3: Add the `ImportOutcome` enum + `db_upsert_imported`**

Add to `shared/types.rs` (since it crosses the IPC boundary):

```rust
/// Outcome of one import attempt for a single world item.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ImportOutcome {
    /// Row didn't exist locally. Inserted as-is (may include world-suffix
    /// in the name if a same-name local row already existed).
    Inserted { id: i64, name: String, kind: AdvantageKind },
    /// Same (name, kind, world) already imported here. Updated description
    /// + bumped imported_at; row id preserved.
    Updated  { id: i64, name: String, kind: AdvantageKind },
    /// Filtered out (non-feature item.kind, or unrecognized featuretype).
    Skipped  { reason: String, name: String },
}
```

Then in `db/advantage.rs`:

```rust
use crate::shared::types::ImportOutcome;

/// Insert-or-update an imported advantage row.
///
/// Dedup logic (identity by immutable Foundry id, NOT by display name):
///   1. If a row exists with (foundryId, worldTitle) =
///      (source_attribution.foundryId, source_attribution.worldTitle)
///      → UPDATE description + bump source_attribution.importedAt;
///      preserve row id AND the stored (possibly-suffixed) name; return Updated.
///   2. Else if a local row exists with (name, kind) but DIFFERENT
///      attribution (or null attribution) → suffix the import name as
///      "<name> (FVTT — <world>)"; INSERT; return Inserted.
///   3. Else → INSERT as-is; return Inserted.
///
/// Why foundryId is the identity key (not name): once a row's name has been
/// suffixed in Case 2, a name-keyed lookup on the next re-pull would miss
/// (the stored name no longer matches the incoming `name` parameter) and
/// fall through to Case 2 again, INSERTing a duplicate. The Foundry document
/// `_id` is stable across re-pulls, so it's the only safe identity key.
pub(crate) async fn db_upsert_imported(
    pool: &SqlitePool,
    name: &str,
    kind: AdvantageKind,
    description: &str,
    properties: &[Field],
    source_attribution: &serde_json::Value,
) -> Result<ImportOutcome, String> {
    let world_title = source_attribution.get("worldTitle")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "db/advantage.upsert_imported: source_attribution missing worldTitle".to_string())?;
    let foundry_id = source_attribution.get("foundryId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "db/advantage.upsert_imported: source_attribution missing foundryId".to_string())?;

    // Case 1: same (foundryId, worldTitle) — Update in place. Preserves the
    // stored name (which may be suffixed) so re-pulls are idempotent.
    if let Some(existing) = db_find_by_foundry_id(pool, foundry_id, world_title).await? {
        let props_json = serialize_properties(properties)?;
        let attr_str = serde_json::to_string(source_attribution)
            .map_err(|e| format!("db/advantage.upsert_imported: serialize attribution: {e}"))?;
        sqlx::query(
            "UPDATE advantages
             SET description = ?, properties_json = ?, source_attribution = ?
             WHERE id = ?"
        )
        .bind(description)
        .bind(&props_json)
        .bind(&attr_str)
        .bind(existing.id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/advantage.upsert_imported: update: {e}"))?;

        return Ok(ImportOutcome::Updated {
            id: existing.id,
            name: existing.name,
            kind: existing.kind,
        });
    }

    // Case 2: name+kind collision with different attribution → suffix.
    let final_name = if db_collides_locally(pool, name, kind).await? {
        format!("{name} (FVTT — {world_title})")
    } else {
        name.to_string()
    };

    // Case 2 / Case 3: INSERT.
    let props_json = serialize_properties(properties)?;
    let tags_json = serialize_tags(&Vec::<String>::new())?; // tags empty for imports; GM can edit later
    let attr_str = serde_json::to_string(source_attribution)
        .map_err(|e| format!("db/advantage.upsert_imported: serialize attribution: {e}"))?;
    let kind_str = kind_to_str(kind);

    let result = sqlx::query(
        "INSERT INTO advantages
            (name, description, kind, tags_json, properties_json, is_custom, source_attribution)
         VALUES (?, ?, ?, ?, ?, 1, ?)"
    )
    .bind(&final_name)
    .bind(description)
    .bind(kind_str)
    .bind(&tags_json)
    .bind(&props_json)
    .bind(&attr_str)
    .execute(pool)
    .await
    .map_err(|e| format!("db/advantage.upsert_imported: insert: {e}"))?;

    Ok(ImportOutcome::Inserted {
        id: result.last_insert_rowid(),
        name: final_name,
        kind,
    })
}
```

- [ ] **Step 4: Tests**

In the existing `#[cfg(test)] mod tests` block:

```rust
/// Build a source_attribution JSON blob with a stable foundryId. Tests
/// must pass a deterministic id so re-pull cases produce hit/miss reliably.
fn world_attribution(world: &str, foundry_id: &str) -> serde_json::Value {
    serde_json::json!({
        "source": "foundry",
        "worldTitle": world,
        "foundryId": foundry_id,
        "importedAt": "2026-05-14T12:00:00Z",
    })
}

#[tokio::test]
async fn upsert_imported_new_row_inserts() {
    let pool = test_pool().await;
    let out = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit,
        "desc", &[], &world_attribution("Chicago", "chi_iron")).await.unwrap();
    assert!(matches!(out, ImportOutcome::Inserted { name, .. } if name == "Iron Gullet"));
}

#[tokio::test]
async fn upsert_imported_same_world_updates_in_place() {
    let pool = test_pool().await;
    let first = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit,
        "desc1", &[], &world_attribution("Chicago", "chi_iron")).await.unwrap();
    let first_id = match first { ImportOutcome::Inserted { id, .. } => id, _ => panic!() };

    // Same foundryId + worldTitle on second call → Updated.
    let second = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit,
        "desc2 (revised)", &[], &world_attribution("Chicago", "chi_iron")).await.unwrap();

    match second {
        ImportOutcome::Updated { id, .. } => assert_eq!(id, first_id),
        other => panic!("expected Updated, got {other:?}"),
    }
    let rows = db_list(&pool).await.unwrap();
    assert_eq!(rows.iter().filter(|r| r.name == "Iron Gullet").count(), 1);
    assert_eq!(rows.iter().find(|r| r.id == first_id).unwrap().description, "desc2 (revised)");
}

#[tokio::test]
async fn upsert_imported_different_world_suffixes_name() {
    let pool = test_pool().await;
    // Different foundryIds across worlds — typical of two unrelated Foundry instances.
    db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit, "d", &[],
        &world_attribution("Chicago", "chi_iron")).await.unwrap();

    let second = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit, "d", &[],
        &world_attribution("Berlin", "ber_iron")).await.unwrap();

    match second {
        ImportOutcome::Inserted { name, .. } => {
            assert_eq!(name, "Iron Gullet (FVTT — Berlin)");
        }
        other => panic!("expected Inserted with suffix, got {other:?}"),
    }
}

#[tokio::test]
async fn upsert_imported_suffixes_against_local_row() {
    let pool = test_pool().await;
    // Pre-existing local (non-imported) row.
    db_insert(&pool, "Iron Gullet", "local desc", AdvantageKind::Merit, None, &[], &[]).await.unwrap();

    let imported = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit, "fvtt desc", &[],
        &world_attribution("Chicago", "chi_iron")).await.unwrap();

    match imported {
        ImportOutcome::Inserted { name, .. } => {
            assert_eq!(name, "Iron Gullet (FVTT — Chicago)");
        }
        other => panic!("expected Inserted with suffix, got {other:?}"),
    }
    let rows = db_list(&pool).await.unwrap();
    let names: Vec<_> = rows.iter().map(|r| r.name.as_str()).collect();
    assert!(names.contains(&"Iron Gullet"));
    assert!(names.contains(&"Iron Gullet (FVTT — Chicago)"));
}

#[tokio::test]
async fn upsert_imported_repull_of_secondary_world_updates_in_place() {
    // Regression test: previously, a re-pull of a suffixed (secondary-world)
    // row created a duplicate because the dedup key was name-based and the
    // stored name had been mutated. With foundryId-keyed dedup it must Update.
    let pool = test_pool().await;

    // Pull 1, world Chicago: INSERTs "Iron Gullet" (no suffix).
    let chi = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit, "v1", &[],
        &world_attribution("Chicago", "chi_iron")).await.unwrap();
    assert!(matches!(chi, ImportOutcome::Inserted { ref name, .. } if name == "Iron Gullet"));

    // Pull 1, world Berlin: collision → INSERTs suffixed "Iron Gullet (FVTT — Berlin)".
    let ber1 = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit, "v1", &[],
        &world_attribution("Berlin", "ber_iron")).await.unwrap();
    let ber_id = match ber1 {
        ImportOutcome::Inserted { id, name, .. } => {
            assert_eq!(name, "Iron Gullet (FVTT — Berlin)");
            id
        }
        other => panic!("expected suffixed Inserted, got {other:?}"),
    };

    // Pull 2, world Berlin (re-pull of same Foundry item): MUST Update the
    // existing suffixed row, NOT insert a duplicate.
    let ber2 = db_upsert_imported(&pool, "Iron Gullet", AdvantageKind::Merit, "v2", &[],
        &world_attribution("Berlin", "ber_iron")).await.unwrap();
    match ber2 {
        ImportOutcome::Updated { id, .. } => assert_eq!(id, ber_id),
        other => panic!("expected Updated on re-pull of secondary world, got {other:?}"),
    }
    let rows = db_list(&pool).await.unwrap();
    let berlin_rows = rows.iter().filter(|r| r.name == "Iron Gullet (FVTT — Berlin)").count();
    assert_eq!(berlin_rows, 1, "re-pull of secondary-world item must update in place, not duplicate");
    let chicago_rows = rows.iter().filter(|r| r.name == "Iron Gullet").count();
    assert_eq!(chicago_rows, 1, "Chicago row must be untouched by Berlin re-pull");
}

#[tokio::test]
async fn find_by_foundry_id_returns_none_for_local_row() {
    let pool = test_pool().await;
    db_insert(&pool, "Allies", "local", AdvantageKind::Background, None, &[], &[]).await.unwrap();
    let found = db_find_by_foundry_id(&pool, "any_id", "Chicago").await.unwrap();
    assert!(found.is_none(), "local row (NULL attribution) must never match a foundryId lookup");
}
```

- [ ] **Step 5: Run `cargo test`**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::advantage`

Expected: all existing tests + 6 new tests pass.

- [ ] **Step 6: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 7: Commit**

```
git add src-tauri/src/db/advantage.rs src-tauri/src/shared/types.rs
git commit -m "$(cat <<'EOF'
db/advantage: add dedup-and-import helpers

db_find_by_foundry_id looks up (source_attribution.foundryId,
source_attribution.worldTitle) — identity by the immutable Foundry
document _id, not the mutable display name. db_collides_locally
detects any (name, kind) match regardless of attribution (used for
the suffix path). db_upsert_imported is the import-flow workhorse:
  • same (foundryId, world) → UPDATE in place (idempotent re-pulls,
    even after a previous suffix mutated the stored name)
  • different (foundryId, world) but same (name, kind) → auto-suffix
    "(FVTT — <world>)" and INSERT
  • no collision → straight INSERT

ImportOutcome enum carries Inserted / Updated / Skipped variants
across the IPC boundary. 6 new tests cover every branch, including
the re-pull-of-secondary-world regression case (foundryId-keyed dedup
preserves idempotency even after a row's name was suffixed).

Refs #14 #15 #16.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 2: `import_advantages_from_world` Tauri command

**Goal:** Read `BridgeState.world_items` for the Foundry source, filter to feature-type items, call `db_upsert_imported` for each, return the outcome list. Single-shot Tauri command; the frontend is responsible for triggering subscription first (Task 6 wires that).

**Files:**
- Modify: `src-tauri/src/db/advantage.rs` (add the Tauri command)

**Anti-scope:** Do NOT subscribe to `item` collection here — the subscription has to be triggered before the command runs (frontend handles via `bridge_subscribe` in Task 3). Do NOT delete local rows here (deletion is the GM's manual concern). Do NOT operate on dyscrasias.

**Depends on:** Task 1

**Invariants cited:** ARCH §4 (typed wrappers required), §7 (error prefix `db/advantage.import_from_world:`).

**Tests required:** YES — 1 integration test that constructs an in-memory `BridgeState` with a faked `world_items` cache and asserts the outcome list.

- [ ] **Step 1: Add the Tauri command**

In `src-tauri/src/db/advantage.rs`:

```rust
use crate::bridge::{BridgeConn, types::SourceKind};

/// Import feature-type world items from the active Foundry world into
/// the local advantages library. Pulls from BridgeState.world_items —
/// the frontend must have subscribed to `item` collection beforehand.
///
/// Filter: only `kind == "feature"` items with featuretype ∈ {merit,
/// flaw, background, boon}. Other item kinds (speciality, power, etc.)
/// are silently filtered (returned as Skipped outcomes for UI summary).
///
/// Per-item: composes db_upsert_imported. Returns Vec<ImportOutcome>
/// in iteration order (which is HashMap order — unspecified, but the
/// frontend doesn't care about ordering).
#[tauri::command]
pub async fn import_advantages_from_world(
    db: tauri::State<'_, crate::DbState>,
    bridge: tauri::State<'_, BridgeConn>,
) -> Result<Vec<ImportOutcome>, String> {
    // Pull world snapshot + source info.
    let world_title = {
        let info = bridge.0.source_info.lock().await;
        info.get(&SourceKind::Foundry)
            .and_then(|i| i.world_title.clone())
            .ok_or_else(|| "db/advantage.import_from_world: no active Foundry world (connect first?)".to_string())?
    };
    let world_id = {
        let info = bridge.0.source_info.lock().await;
        info.get(&SourceKind::Foundry)
            .and_then(|i| i.world_id.clone())
    };
    let system_version = {
        let info = bridge.0.source_info.lock().await;
        info.get(&SourceKind::Foundry)
            .and_then(|i| i.system_version.clone())
    };

    let items: Vec<crate::bridge::types::CanonicalWorldItem> = {
        let store = bridge.0.world_items.lock().await;
        store.get(&SourceKind::Foundry)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    };

    let now = chrono::Utc::now().to_rfc3339();

    let mut outcomes = Vec::with_capacity(items.len());
    for item in items {
        if item.kind != "feature" {
            outcomes.push(ImportOutcome::Skipped {
                reason: format!("non-feature item kind: {}", item.kind),
                name: item.name,
            });
            continue;
        }
        let ft = match item.featuretype.as_deref() {
            Some("merit")      => AdvantageKind::Merit,
            Some("flaw")       => AdvantageKind::Flaw,
            Some("background") => AdvantageKind::Background,
            Some("boon")       => AdvantageKind::Boon,
            other => {
                outcomes.push(ImportOutcome::Skipped {
                    reason: format!("unknown featuretype: {:?}", other),
                    name: item.name,
                });
                continue;
            }
        };

        // Extract description + points from item.system.
        let description = item.system.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let points = item.system.get("points")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        // Build properties (level field) for downstream UI consistency.
        let properties: Vec<Field> = if points > 0 {
            vec![Field {
                name: "level".into(),
                value: FieldValue::Number { value: NumberFieldValue::Single(points as f64) },
            }]
        } else {
            vec![]
        };

        // Per-item attribution carries the immutable Foundry document _id
        // (used by db_upsert_imported as the dedup identity key). Shared
        // fields are cloned in from the per-call locals.
        let attribution = serde_json::json!({
            "source": "foundry",
            "worldTitle": world_title,
            "worldId": world_id,
            "systemVersion": system_version,
            "foundryId": item.id,
            "importedAt": now,
        });

        let outcome = db_upsert_imported(
            &db.0,
            &item.name,
            ft,
            &description,
            &properties,
            &attribution,
        ).await?;
        outcomes.push(outcome);
    }

    Ok(outcomes)
}
```

Add the necessary imports at the top of `db/advantage.rs`:

```rust
use crate::shared::types::{Field, FieldValue, NumberFieldValue};
```

(`chrono` is in `Cargo.toml` already — verify; if not, this is a YAGNI win — use `std::time::SystemTime` + manual ISO-8601 formatting via existing helpers.)

- [ ] **Step 2: Integration test**

```rust
#[tokio::test]
async fn import_from_world_filters_non_feature_and_unknown_featuretype() {
    let pool = test_pool().await;

    // Stub out a BridgeState with a faked world_items cache.
    // The test helper needs to construct a minimal BridgeConn — see
    // src-tauri/src/bridge/mod.rs for the existing test-state helper
    // if it exists, otherwise add a fixture here using
    // BridgeState::new_test() pattern.

    // For v1, this integration test is BEST-EFFORT: if BridgeState's
    // test fixtures don't yet exist (likely they don't), defer the
    // integration test to manual E2E and only ship unit tests for
    // db_upsert_imported (Task 1). Document this gap in the task
    // tracker.

    // Pseudo-test sketch:
    //   let bridge = BridgeConn::new_test_with_world_items(vec![
    //       CanonicalWorldItem { kind: "feature", featuretype: Some("merit".into()), ... },
    //       CanonicalWorldItem { kind: "speciality", featuretype: None, ... },
    //       CanonicalWorldItem { kind: "feature", featuretype: Some("discipline".into()), ... },
    //   ]);
    //   let outcomes = import_advantages_from_world(db_state, bridge).await.unwrap();
    //   assert_eq!(outcomes.len(), 3);
    //   assert!(matches!(outcomes[0], ImportOutcome::Inserted { .. }));
    //   assert!(matches!(outcomes[1], ImportOutcome::Skipped { reason, .. } if reason.contains("non-feature")));
    //   assert!(matches!(outcomes[2], ImportOutcome::Skipped { reason, .. } if reason.contains("unknown featuretype")));
}
```

**Implementation note:** if `BridgeConn` lacks test-fixture support, mark this test `#[ignore]` with a comment pointing to Task 8's E2E smoke, OR refactor `import_advantages_from_world` to take an injectable items-source — both are fine. The 5 unit tests on `db_upsert_imported` (Task 1) carry the dedup logic correctness; this integration test would only cover the filter+orchestration shell.

- [ ] **Step 3: Run `cargo test`**

Expected: existing tests still pass; new integration test passes OR is marked `#[ignore]` with rationale.

- [ ] **Step 4: Update `ARCHITECTURE.md` §4 (same-commit rule)**

CLAUDE.md mandates that any new `#[tauri::command]` lands in the same commit as its ARCH §4 entry. Edit `ARCHITECTURE.md` §4:
- Append `import_advantages_from_world` to the per-file IPC entry for `db/advantage.rs`.
- Bump the running command total: **65 → 66**.

- [ ] **Step 5: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 6: Commit**

```
git add src-tauri/src/db/advantage.rs ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
Add import_advantages_from_world Tauri command

Reads BridgeState.world_items for the Foundry source, filters to
feature-type items with merit/flaw/background/boon featuretype,
composes db_upsert_imported per row. Returns Vec<ImportOutcome>
for the frontend summary toast. Stamps source_attribution with
world title + world id + system version + foundryId + ISO-8601
importedAt (foundryId is the dedup identity key — see Task 1).

Subscription to `item` collection must be triggered by the frontend
before calling (Task 6); this command is a snapshot reader. ARCH §4
IPC inventory bumped (65 → 66) in the same commit, per CLAUDE.md
same-commit rule.

Refs #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 3: `bridge_subscribe` + `bridge_unsubscribe` Tauri commands

**Goal:** First-class Tauri commands for dynamically subscribing to a Foundry collection. Plan 0 shipped the wire envelope; Plan C is the first dynamic consumer.

**Files:**
- Modify: `src-tauri/src/bridge/commands.rs`

**Anti-scope:** Do NOT couple these to Foundry — they accept a `SourceKind` parameter so future per-source subscriptions work. Do NOT track subscribed state in `BridgeState` here — the source (module) tracks it; desktop is fire-and-forget at the IPC layer.

**Depends on:** none (uses existing `actions::bridge::build_subscribe / build_unsubscribe` and `send_to_source_inner`).

**Invariants cited:** ARCH §4 (Tauri IPC), §7 (error prefix `bridge/subscribe:` / `bridge/unsubscribe:`).

**Tests required:** NO — composes existing primitives; manual smoke in Task 8.

- [ ] **Step 1: Add the commands**

In `src-tauri/src/bridge/commands.rs`:

```rust
use crate::bridge::foundry::actions::bridge as bridge_actions;

/// Send `bridge.subscribe { collection }` to the named source. No-op
/// if the source isn't connected. Per-source dispatch: in v1 only
/// Foundry implements bridge.* subscriptions; Roll20 silently ignores.
#[tauri::command]
pub async fn bridge_subscribe(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    collection: String,
) -> Result<(), String> {
    // v1: only Foundry implements bridge.subscribe. Future sources may
    // route via SourceKind here.
    if source != SourceKind::Foundry {
        return Err(format!("bridge/subscribe: source {source:?} does not support subscriptions"));
    }
    let payload = bridge_actions::build_subscribe(&collection);
    let text = serde_json::to_string(&payload)
        .map_err(|e| format!("bridge/subscribe: serialize: {e}"))?;
    send_to_source_inner(&conn.0, source, text).await
}

/// Send `bridge.unsubscribe { collection }` to the named source.
#[tauri::command]
pub async fn bridge_unsubscribe(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    collection: String,
) -> Result<(), String> {
    if source != SourceKind::Foundry {
        return Err(format!("bridge/unsubscribe: source {source:?} does not support subscriptions"));
    }
    let payload = bridge_actions::build_unsubscribe(&collection);
    let text = serde_json::to_string(&payload)
        .map_err(|e| format!("bridge/unsubscribe: serialize: {e}"))?;
    send_to_source_inner(&conn.0, source, text).await
}
```

- [ ] **Step 2: Update `ARCHITECTURE.md` §4 (same-commit rule)**

CLAUDE.md mandates that any new `#[tauri::command]` lands in the same commit as its ARCH §4 entry. Edit `ARCHITECTURE.md` §4:
- Append `bridge_subscribe` and `bridge_unsubscribe` to the per-file IPC entry for `bridge/commands.rs`.
- Bump the running command total: **66 → 68**.

- [ ] **Step 3: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 4: Commit**

```
git add src-tauri/src/bridge/commands.rs ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
Add bridge_subscribe / bridge_unsubscribe Tauri commands

First dynamic consumer of Plan 0's bridge.subscribe envelope. Plan C
import flow drives these from the AdvantagesManager Pull button.
v1 accepts only SourceKind::Foundry (Roll20 returns an error string
rather than silently ignoring — gives the frontend a clearer signal).
ARCH §4 IPC inventory bumped (66 → 68) in the same commit, per
CLAUDE.md same-commit rule.

Refs #14.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 4: Register 3 new commands in `lib.rs`

**Goal:** Wire all new commands into the invoke handler.

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Depends on:** Tasks 1, 2, 3

**Tests required:** NO

- [ ] **Step 1: Add three lines**

In `invoke_handler!`:

```rust
db::advantage::import_advantages_from_world,
bridge::commands::bridge_subscribe,
bridge::commands::bridge_unsubscribe,
```

(Cluster `import_advantages_from_world` near `db::advantage::*`; cluster the `bridge_*` ones near other `bridge::commands::*` entries.)

- [ ] **Step 2: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 3: Commit**

```
git add src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Register Plan C commands in invoke_handler

import_advantages_from_world, bridge_subscribe, bridge_unsubscribe.
No new command surface in this commit — declarations + ARCH §4
entries already landed in Tasks 2 and 3 (total bumped to 68 in
those commits). This commit only wires existing declarations into
the frontend via `generate_handler!`.

Refs #14 #16.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 5: Frontend typed wrappers + `importer.ts` helpers

**Goal:** Extend `src/lib/library/api.ts` with the three new wrappers; add `importer.ts` with the pure summarization helpers.

**Files:**
- Modify: `src/lib/library/api.ts`
- Create: `src/lib/library/importer.ts`
- Modify: `src/types.ts` (add `ImportOutcome` discriminated union)

**Anti-scope:** Do NOT call `invoke` from components. Do NOT add a SubscriptionState store — the frontend can fire-and-forget; the post-import refresh of the advantages list is the only state-of-interest.

**Depends on:** Task 4

**Invariants cited:** ARCH §4 (typed wrappers per Tauri command).

**Tests required:** NO required, but if the repo has vitest set up, add 2-3 tests for the pure `importer.ts` helpers.

- [ ] **Step 1: Add `ImportOutcome` to `src/types.ts`**

```ts
export type AdvantageKind = 'merit' | 'flaw' | 'background' | 'boon';

export type ImportOutcome =
  | { action: 'inserted'; id: number; name: string; kind: AdvantageKind }
  | { action: 'updated';  id: number; name: string; kind: AdvantageKind }
  | { action: 'skipped';  reason: string; name: string };
```

(Verify `AdvantageKind` is already in `src/types.ts` from Plan A — should be.)

- [ ] **Step 2: Extend `src/lib/library/api.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';
import type { ImportOutcome } from '../../types';

export function pushAdvantageToWorld(id: number): Promise<void> {
  return invoke<void>('push_advantage_to_world', { id });
}

export function importAdvantagesFromWorld(): Promise<ImportOutcome[]> {
  return invoke<ImportOutcome[]>('import_advantages_from_world');
}

export function subscribeToWorldItems(): Promise<void> {
  return invoke<void>('bridge_subscribe',
    { source: 'foundry', collection: 'item' });
}

export function unsubscribeFromWorldItems(): Promise<void> {
  return invoke<void>('bridge_unsubscribe',
    { source: 'foundry', collection: 'item' });
}
```

**SourceKind serialization note:** Rust's `SourceKind` enum serializes as `"foundry"` / `"roll20"` (snake_case). Verify by inspecting `src-tauri/src/bridge/types.rs` for the serde attribute on `SourceKind` (around line 5).

- [ ] **Step 3: Create `src/lib/library/importer.ts`**

```ts
import type { ImportOutcome } from '../../types';

export interface ImportSummary {
  inserted: number;
  updated: number;
  skipped: number;
  details: ImportOutcome[];
}

/**
 * Summarize a list of import outcomes for the UI toast.
 * Pure; no IPC; no side effects.
 */
export function summarizeImport(outcomes: ImportOutcome[]): ImportSummary {
  let inserted = 0, updated = 0, skipped = 0;
  for (const o of outcomes) {
    if (o.action === 'inserted')      inserted++;
    else if (o.action === 'updated')  updated++;
    else                              skipped++;
  }
  return { inserted, updated, skipped, details: outcomes };
}

/**
 * Format the summary as a single-line toast message.
 * "Imported 4 new (2 updated, 1 skipped) from Chronicles of Chicago"
 */
export function summaryAsToast(summary: ImportSummary, worldTitle: string): string {
  const parts: string[] = [];
  parts.push(`Imported ${summary.inserted} new`);
  const suffixes: string[] = [];
  if (summary.updated > 0) suffixes.push(`${summary.updated} updated`);
  if (summary.skipped > 0) suffixes.push(`${summary.skipped} skipped`);
  if (suffixes.length > 0) parts.push(`(${suffixes.join(', ')})`);
  parts.push(`from ${worldTitle}`);
  return parts.join(' ');
}
```

- [ ] **Step 4: Run `npm run check`**

Expected: green.

- [ ] **Step 5: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit. The npm check above is a fast inner-loop signal; `verify.sh` is the gate.

- [ ] **Step 6: Commit**

```
git add src/lib/library/api.ts src/lib/library/importer.ts src/types.ts
git commit -m "$(cat <<'EOF'
Add Plan C frontend wrappers + import summarization helpers

api.ts: importAdvantagesFromWorld, subscribeToWorldItems,
unsubscribeFromWorldItems. Composes Plan B's pushAdvantageToWorld for
the full Library Sync IPC surface.

importer.ts: pure summarizeImport + summaryAsToast helpers used by
AdvantagesManager's Pull button (Task 6).

ImportOutcome TS discriminated union mirrors the Rust enum.

Refs #14.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 6: AdvantagesManager UI — Pull button, source chip, tri-state filter

**Goal:** The visible deliverable of Phase 4. Three discrete UI additions:
1. "⇣ Pull from active world" button (Foundry-connected only).
2. Source chip on each row whose `sourceAttribution` is non-null. Reuses Phase 1's `SourceAttributionChip.svelte`.
3. Tri-state filter row: `Corebook / Local / Imported` (mutually exclusive with each other but additive to the existing tag and kind filters).

**Files:**
- Modify: `src/tools/AdvantagesManager.svelte`
- Modify: `src/lib/components/AdvantageCard.svelte` (source chip slot)
- Verify: `src/lib/components/SourceAttributionChip.svelte` is reusable here — actual prop shape is `{ source: SourceKind; worldTitle?: string | null }` (confirmed against current `$props()` destructure).

**Anti-scope:** Do NOT show an interactive collision-resolution modal — auto-suffix is locked in (#16 decision). Do NOT add a "Re-pull" button on individual rows — pull-all-and-update is the v1 model. Do NOT add per-row Delete for imported items (the existing Delete button on custom rows works for them too — they're `is_custom = 1`).

**Depends on:** Tasks 4, 5 (commands + wrappers).

**Invariants cited:** ARCH §4 (typed wrappers only — no direct `invoke`), §6 (CSS tokens; no hardcoded hex).

**Tests required:** NO (UI; manual smoke covers it).

- [ ] **Step 1: Verify `SourceAttributionChip.svelte`**

Read `src/lib/components/SourceAttributionChip.svelte`. The actual prop names are `source: SourceKind` and `worldTitle?: string | null` — pass `<SourceAttributionChip source="foundry" worldTitle={adv.sourceAttribution?.worldTitle} />`. (The earlier sketch using `world=` was wrong — Svelte silently drops unknown props and `npm run check` will surface the TS error.) If the prop shape ever changes, the import-version chip can be inline:

```svelte
{#if adv.sourceAttribution}
  <span class="chip source-chip" data-source="foundry">
    FVTT · {adv.sourceAttribution.worldTitle}
  </span>
{/if}
```

- [ ] **Step 2: Add the "Pull from active world" button**

In `AdvantagesManager.svelte`'s toolbar area (near the existing Sort/Filter controls):

```svelte
<script lang="ts">
  import {
    importAdvantagesFromWorld,
    subscribeToWorldItems,
    unsubscribeFromWorldItems,
  } from '$lib/library/api';
  import { summarizeImport, summaryAsToast } from '$lib/library/importer';
  import { bridgeStore } from '$lib/../store/bridge.svelte';

  let pulling = $state(false);
  let pullSummary = $state('');
  let pullError = $state('');

  async function pullFromWorld() {
    pulling = true;
    pullSummary = '';
    pullError = '';
    try {
      await subscribeToWorldItems();
      // Wait briefly for the snapshot to arrive — 1.5s covers
      // typical Foundry response latency on localhost. The
      // bridge cache may already be hot (subscribed in prior pull
      // this session); the wait is no-cost in that case.
      await new Promise(r => setTimeout(r, 1500));
      const outcomes = await importAdvantagesFromWorld();
      const summary = summarizeImport(outcomes);
      const worldTitle = bridgeStore.sourceInfo?.foundry?.worldTitle ?? 'Foundry';
      pullSummary = summaryAsToast(summary, worldTitle);
      // Reload local rows so the new imports + chips render.
      await loadAll();
    } catch (e) {
      pullError = String(e);
    } finally {
      pulling = false;
    }
  }
</script>

{#if bridgeStore.status.foundry}
  <button class="toolbar-action" disabled={pulling} onclick={pullFromWorld}>
    {pulling ? 'Pulling…' : '⇣ Pull from world'}
  </button>
  {#if pullSummary}
    <span class="toolbar-status">{pullSummary}</span>
  {/if}
  {#if pullError}
    <span class="toolbar-error">{pullError}</span>
  {/if}
{/if}
```

(Adapt `bridgeStore` accessors to match Plan B's actual exports.)

- [ ] **Step 3: Add source chip on rows**

In `src/lib/components/AdvantageCard.svelte`, near the kind chip from Plan A:

```svelte
{#if adv.sourceAttribution}
  <span class="chip source-chip" data-source={adv.sourceAttribution.source}>
    FVTT · {adv.sourceAttribution.worldTitle}
  </span>
{/if}
```

CSS:

```css
.source-chip[data-source="foundry"] {
  background: var(--accent-foundry, var(--accent-muted));
  font-size: 0.75rem;
}
```

- [ ] **Step 4: Add tri-state filter**

Extend `AdvantagesManager.svelte`'s state:

```ts
type ProvenanceFilter = 'all' | 'corebook' | 'local' | 'imported';
let provenanceFilter: ProvenanceFilter = $state('all');

function matchesProvenance(adv: Advantage): boolean {
  switch (provenanceFilter) {
    case 'all':      return true;
    case 'corebook': return !adv.isCustom;
    case 'local':    return adv.isCustom && !adv.sourceAttribution;
    case 'imported': return adv.isCustom && !!adv.sourceAttribution;
  }
}
```

Include `matchesProvenance` in the existing `visible` derived chain:

```ts
const visible = $derived(
  sortRows(allEntries.filter(e => matchesTags(e) && matchesQuery(e) && matchesProvenance(e)))
);
```

Render a small chip row with 4 buttons (`All / Corebook / Local / Imported`) using the same chip styling as tags. Mutually exclusive — clicking one sets it as `provenanceFilter`.

- [ ] **Step 5: Run `npm run check` and `npm run build`**

Expected: green.

- [ ] **Step 6: Manual smoke**

`npm run tauri dev` with a Foundry world running and `vtmtools-bridge@0.6.0`:

- ✅ Pull button visible when Foundry connected; disappears on disconnect.
- ✅ Click pull → "Imported X new (Y updated, Z skipped) from <world>" toast appears.
- ✅ New rows show "FVTT · <world>" source chip + their kind chip.
- ✅ Click pull again → same rows now show as "updated" in the toast count.
- ✅ Create an `Iron Gullet` merit locally (custom). Pull from a world containing `Iron Gullet`. Result: a new row `Iron Gullet (FVTT — <world>)` appears alongside the local one.
- ✅ Tri-state filter: clicking "Imported" hides corebook + local rows; clicking "Local" hides corebook + imported; "Corebook" hides custom (both flavors).
- ✅ The existing Edit button on an imported row works (GM can curate imports).
- ✅ **Re-pull regression check** (Plan C Task 1's foundryId-keyed dedup): create the suffixed `Iron Gullet (FVTT — <world>)` row above, then click Pull again. The suffixed row count must stay at 1 (Updated, not Inserted-duplicate).

- [ ] **Step 7: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit.

- [ ] **Step 8: Commit**

```
git add src/tools/AdvantagesManager.svelte \
        src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
AdvantagesManager: Pull button + source chip + tri-state filter

Closes the visible half of Library Sync:
  • "⇣ Pull from world" button (Foundry-connected) drives subscribe
    → wait → import → toast summary → reload.
  • Per-row source chip on imported rows ("FVTT · <world>").
  • Tri-state provenance filter (Corebook / Local / Imported)
    composes with the existing tag, kind, and search filters.

Auto-suffix dedup (#16 decision): same-name + same-world → update
in place; same-name + different-world → "<name> (FVTT — <world>)".

Closes #14 #15 #16.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 7: ~~ARCHITECTURE.md §4 updates~~ — REMOVED (work distributed into Tasks 2 and 3)

The original "Task 7" consolidated all Plan C ARCH §4 IPC inventory edits into a single trailing commit. CLAUDE.md's same-commit rule ("Never add a `#[tauri::command]` without updating ARCHITECTURE.md §4 IPC inventory in the same commit") makes that consolidation pattern invalid. The IPC inventory bumps now travel inline with the declaring commits:

- `import_advantages_from_world` → ARCH §4 updated in **Task 2** (total 65 → 66).
- `bridge_subscribe` + `bridge_unsubscribe` → ARCH §4 updated in **Task 3** (total 66 → 68).

There is no separate ARCH §4 prose work for Plan C (unlike Plan B, which has the storyteller.* umbrella + subscription-collection paragraph in its Task 15). Task 4's `invoke_handler!` registration commit therefore lands no ARCH change.

The task overview table's row for Task 7 stays as a numbered placeholder for readability; the implementer should skip it.

---

## Task 8: Final verification gate (incl. live-Foundry E2E)

**Goal:** Confirm Phase 4 (Plans A + B + C) end-to-end before the single branch-wide `code-review:code-review`.

- [ ] **Step 1: Run `./scripts/verify.sh`**

Expected: full green.

- [ ] **Step 2: Live-Foundry E2E — full Library Sync round trip**

Boot the Tauri app + a Foundry world with `vtmtools-bridge@0.6.0`.

**Push round trip (#13 — already smoke-tested in Plan B, re-verify):**
- ✅ Push a corebook merit → appears in Foundry world Items sidebar.

**Pull round trip (#14, #15, #16):**
- ✅ In Foundry, manually add a few feature items to the world (one of each kind: a Merit, a Flaw, a Background; bonus: a non-feature item like a speciality to verify Skipped outcome).
- ✅ Click "⇣ Pull from world" in AdvantagesManager. Wait ~2s.
- ✅ Toast reports "Imported 3 new (1 skipped) from <world>".
- ✅ New rows appear with kind chips + "FVTT · <world>" source chips.
- ✅ Click Pull again immediately → "Imported 0 new (3 updated, 1 skipped)".
- ✅ Modify the Foundry-side description of a merit; pull again → row's description updates in place; the toast shows it as Updated.
- ✅ Delete an imported row locally via the existing Delete button. Pull again → re-inserts.
- ✅ Tri-state filter "Imported" shows only the 3 imported rows.

**Cross-world dedup (#16):**
- ✅ Swap to a different Foundry world (different `world_title`). Add a Merit named the same as an already-imported one. Pull. Result: new row with `(FVTT — <new-world>)` suffix. Both rows visible in the Library; both show source chips with their respective world names.

- [ ] **Step 3: Update plan checkboxes**

Mark every Task 1–7 step as `[x]` in this file. Commit:

```
git add docs/superpowers/plans/2026-05-14-library-sync-plan-c-pull-attribution-dedup.md
git commit -m "$(cat <<'EOF'
Mark Plan C tasks complete

Plan C (pull + attribution + dedup) shipped. Closes #14 #15 #16.
Milestone 4 (Library Sync) complete.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

- [ ] **Step 4: Single branch-wide code review**

Per CLAUDE.md's lean-execution override, NOW (and not before) invoke `code-review:code-review` against the full Phase 4 branch diff (the union of Plan A + Plan B + Plan C commits). The reviewer agent's report informs any follow-up fix commits before merging.

- [ ] **Step 5: Milestone-close hygiene**

After review-driven fixes (if any) land:
- Confirm all five milestone-4 issues are closed by the per-plan `Closes #N` commit footers.
- Confirm the milestone-4 project-board column reflects the closures (auto-handled by GitHub if `Closes #N` footers are present).
- Plan C is complete. Library Sync (Milestone 4) is complete.
