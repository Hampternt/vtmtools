# Library Sync — Plan A: Library schema foundation

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plan A tasks are committed, defer the `code-review:code-review` until Plan C is also done (single review across the full Phase 4 branch diff).
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** Promote the `tags_json`-based kind discrimination on `advantages` to a real `kind` column (CHECK-gated enum) and add a nullable `source_attribution` TEXT column for FVTT-import provenance. Backfill from existing tags. Update the `Advantage` Rust struct + TS mirror + seed corpus + AdvantagesManager UI (kind chip only — source-attribution chip lands in Plan C). No FVTT bridge work in this plan.

**Architecture:** Single migration on a live table using the proven `ALTER TABLE ADD COLUMN … CHECK(…)` + `UPDATE … CASE WHEN` pattern shipped in `0007_add_modifier_zone.sql`. Backfill priority for ambiguous tag combinations: `merit > flaw > background > boon` (highest priority wins). `tags_json` stays for free-form taxonomy (Feeding / Social / Supernatural / etc.) — only the kind-discriminator role moves to the new column. Existing consumers (`db/modifier.rs`, `tools/character.rs`, `db/saved_character.rs`) read the row by `id` only and do not depend on the discriminator at the DB layer, so the schema change is non-breaking for them.

**Tech Stack:** Rust (`sqlx`, `serde`, `serde_json`), TypeScript / Svelte 5 runes, SQLite.

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 4 (sketch), refined by the architect-advisor recommendation captured on branch `claude/brainstorm-library-features-Cf0B8` (commit at end of plan).

**Architecture reference:** `ARCHITECTURE.md` §3 (Storage strategy — `is_custom` invariant), §4 (IPC inventory — touches `db::advantage::*` command surface but adds zero new commands), §6 (Invariants — destructive reseed per ADR 0002 stays intact), §9 ("Add a schema change" seam).

**Depends on:** nothing.

**Unblocks:** Plan B (push uses `kind` to map to `featuretype`), Plan C (pull writes `kind` + `source_attribution` directly).

**Issues:** None directly — Plan A is a prerequisite that doesn't close any milestone-4 issue on its own. Cite as "Refs #13 #14 #15 (foundation)" on commits.

---

## File structure

### New files
- `src-tauri/migrations/0009_advantages_kind_and_source.sql` — ADD COLUMN `kind` (CHECK-gated enum) + ADD COLUMN `source_attribution` (nullable TEXT/JSON); backfill `kind` from `tags_json`.

### Modified files
- `src-tauri/src/shared/types.rs` — `Advantage` struct gains `kind: AdvantageKind` and `source_attribution: Option<serde_json::Value>`; new `AdvantageKind` enum (`Merit` / `Flaw` / `Background` / `Boon`) with `#[serde(rename_all = "snake_case")]`.
- `src-tauri/src/db/advantage.rs` — `db_list`, `db_insert`, `db_update` thread the two new fields through. Tests updated for new struct shape; new tests for kind round-trip + source_attribution round-trip + filter-by-kind helper.
- `src-tauri/src/db/seed.rs` — `SeedRow` struct gains `kind: AdvantageKind`; existing rows annotated (Merit/Background/Flaw — no Boons in corebook); `seed_advantages` writes `kind` on INSERT. Tag arrays unchanged (still carry the "Merit"/"Background"/"Flaw" tag for backwards-display compatibility).
- `src/types.ts` — mirror `AdvantageKind` enum + extended `Advantage` interface.
- `src/lib/advantages/api.ts` — `AdvantageInput` gains `kind` (required); typed wrappers thread it through.
- `src/tools/AdvantagesManager.svelte` — render a per-row `kind` chip; add "kind" tags to the existing tag-filter row (or wire a parallel `activeKinds: Set<AdvantageKind>` filter — see Task 6 for details).
- `src/lib/components/AdvantageForm.svelte` — add a `kind` select control (required, defaults to `merit`).
- `ARCHITECTURE.md` — §6 invariants paragraph documenting the `is_custom` tri-state semantic shift (corebook = 0 / hand-authored local = 1 + null attribution / FVTT-imported = 1 + non-null attribution); §9 "Add a library kind" seam paragraph codifying the partitioning rule (same row shape → polymorphic table with `kind`; different row shape → own table).

### Files explicitly NOT touched
- `src-tauri/src/bridge/**` — Plan B territory
- `src-tauri/src/tools/library_push.rs` (new) — Plan B territory
- `src-tauri/src/db/modifier.rs::materialize_advantage_modifier` — reads advantage by `id`, doesn't depend on tag-based kind. No change needed (verify in Task 4).
- `src-tauri/src/tools/character.rs::character_add_advantage` / `character_remove_advantage` — same reasoning.
- `src-tauri/src/db/saved_character.rs::db_add_advantage` — already validates featuretype against `merit/flaw/background/boon` (line 226); we'll verify it still composes correctly in Task 4 but should not require changes (it takes featuretype as a parameter, not from advantage row).
- `src/lib/saved-characters/diff.ts` — verify it doesn't depend on advantage tag strings (Task 4 audit).
- `dyscrasias` table and `db/dyscrasia.rs` — separate row shape, deferred per partitioning rule.

---

## Task overview

| # | Task | Depends on | Tests |
|---|---|---|---|
| 1 | Create migration `0009_advantages_kind_and_source.sql` + verify it applies + backfill is correct | none | YES (sqlx-migrate-runtime — verify CHECK constraint rejects bad values; verify backfill priority) |
| 2 | Add `AdvantageKind` enum + extend `Advantage` struct in `shared/types.rs` | 1 | NO (struct definition only) |
| 3 | Update `db/advantage.rs` to read/write the new columns + update existing tests + add new tests | 2 | YES (3 new tests minimum) |
| 4 | Update `db/seed.rs` to write `kind` on INSERT; annotate existing seed rows | 2 | NO (seed is idempotent; covered by Task 1's runtime check) |
| 5 | Audit non-advantage consumers for breakage; update `db/saved_character.rs::db_add_advantage` IFF audit finds a gap | 3, 4 | NO (audit task; tests only if gap found) |
| 6 | Mirror types in `src/types.ts` + extend `src/lib/advantages/api.ts` | 3 | NO |
| 7 | Update `AdvantagesManager.svelte` (kind chip + kind filter) + `AdvantageForm.svelte` (kind select) | 6 | NO (UI; covered by `npm run build` + manual smoke) |
| 8 | Update `ARCHITECTURE.md` §6 (tri-state `is_custom`) + §9 ("Add a library kind") | 7 | NO |
| 9 | Final verification gate | all | runs `./scripts/verify.sh` |

Tasks 3 and 4 are independent of each other (different functions in different files) and can dispatch in parallel after Task 2. Tasks 5–8 are sequential.

---

## Task 1: Create migration `0009_advantages_kind_and_source.sql`

**Goal:** Add `kind` (CHECK-gated, NOT NULL with default 'merit') and `source_attribution` (nullable TEXT) columns. Backfill `kind` from `tags_json` using the priority-cascade `merit > flaw > background > boon`.

**Files:**
- Create: `src-tauri/migrations/0009_advantages_kind_and_source.sql`

**Anti-scope:** Do NOT touch any other migration. Do NOT alter `dyscrasias` or any other table. Do NOT drop `tags_json` — it stays as a free-form taxonomy column.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §3 (migrations applied via `sqlx::migrate!`; `PRAGMA foreign_keys = ON` enabled on pool). The `ALTER TABLE … ADD COLUMN … CHECK(…)` pattern is proven in `0007_add_modifier_zone.sql`.

**Tests required:** YES — the verification step rebuilds the dev DB and confirms backfill correctness.

- [ ] **Step 1: Write the migration**

Create `src-tauri/migrations/0009_advantages_kind_and_source.sql`:

```sql
-- Adds kind and source_attribution columns to advantages.
--
-- kind disambiguates the polymorphic table (Phase 4 storage decision); was
-- previously inferred from tags_json string-matching. tags_json stays for
-- free-form taxonomy ("Feeding", "Social", "Supernatural", "VTM 5e", etc.).
--
-- source_attribution carries FVTT-import provenance (Phase 4 issue #15).
-- NULL = hand-authored locally (whether corebook seed or GM custom).
-- Non-null = imported from a Foundry world; JSON shape:
--   { "source": "foundry", "world_title": "...", "world_id": "...",
--     "system_version": "...", "imported_at": "ISO-8601" }
-- The exact shape is enforced at the application layer, not by SQLite.

ALTER TABLE advantages
    ADD COLUMN kind TEXT NOT NULL DEFAULT 'merit'
    CHECK(kind IN ('merit', 'flaw', 'background', 'boon'));

ALTER TABLE advantages
    ADD COLUMN source_attribution TEXT;

-- Backfill kind from tags_json. Priority cascade: merit > flaw > background
-- > boon. Highest priority wins via CASE WHEN ordering (first match returns).
-- Rows tagged with none of the four kind-strings stay at the default 'merit'.
UPDATE advantages
   SET kind = CASE
       WHEN tags_json LIKE '%"Merit"%'      THEN 'merit'
       WHEN tags_json LIKE '%"Flaw"%'       THEN 'flaw'
       WHEN tags_json LIKE '%"Background"%' THEN 'background'
       WHEN tags_json LIKE '%"Boon"%'       THEN 'boon'
       ELSE 'merit'
   END;
```

- [ ] **Step 2: Verify the migration compiles**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: clean. `sqlx::migrate!` is a compile-time macro that ensures all migration files parse.

- [ ] **Step 3: Verify backfill correctness on a dev DB**

The dev DB lives at the path returned by `app.path().app_data_dir()` (Tauri-managed). Easiest path: delete `~/.local/share/com.hampternt.vtmtools/vtmtools.db` (or whatever the dev path is — check `tauri.conf.json`), run `npm run tauri dev`, exit immediately after the world is seeded, then inspect the table:

```bash
sqlite3 ~/.local/share/com.hampternt.vtmtools/vtmtools.db \
  "SELECT name, kind, tags_json FROM advantages ORDER BY kind, name;"
```

Expected output (verify against `seed.rs::seed_rows`):
- 4 merits (Iron Gullet, Eat Food, Bloodhound, Beautiful) → `kind = 'merit'`
- 4 backgrounds (Allies, Contacts, Haven, Resources) → `kind = 'background'`
- 2 flaws (Prey Exclusion, Enemy) → `kind = 'flaw'`
- 0 boons (no boons in V5 corebook seed)

If counts mismatch: inspect `tags_json` on the affected rows; the LIKE pattern requires the tag value to be quoted in the JSON exactly as `"Merit"` / `"Flaw"` / `"Background"` / `"Boon"` — case-sensitive.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green. Existing `db::advantage` tests still pass because `db_list` doesn't yet reference the new column (Task 3 changes that).

- [ ] **Step 5: Commit**

```
git add src-tauri/migrations/0009_advantages_kind_and_source.sql
git commit -m "$(cat <<'EOF'
Add `kind` + `source_attribution` columns to advantages

Promotes tags-based kind discrimination to a CHECK-gated column.
Backfills `kind` from existing tags_json with priority cascade
merit > flaw > background > boon. `tags_json` stays for free-form
taxonomy.

Foundation for Phase 4 library sync. Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 2: Add `AdvantageKind` enum + extend `Advantage` struct

**Goal:** Mirror the SQL CHECK enum on the Rust side as a typed `AdvantageKind`; extend the `Advantage` struct to carry `kind` + `source_attribution`.

**Files:**
- Modify: `src-tauri/src/shared/types.rs` (around line 126 where `Advantage` lives)

**Anti-scope:** Do NOT change the existing `tags`/`properties` fields. Do NOT introduce a typed `SourceAttribution` struct (architect anti-recommendation: stay JSON-blob for v1; promote to typed enum when a second source materializes).

**Depends on:** Task 1

**Invariants cited:** ARCHITECTURE.md §3 (`shared/types.rs` is the single source of truth for cross-boundary types).

**Tests required:** NO — struct definition only; runtime tests come in Task 3.

- [ ] **Step 1: Add `AdvantageKind` enum**

In `src-tauri/src/shared/types.rs`, immediately before the `pub struct Advantage` block (around line 124), add:

```rust
/// Kind discriminator for the polymorphic `advantages` library table.
/// Mirrors the SQL CHECK constraint in `0009_advantages_kind_and_source.sql`.
///
/// Partitioning rule (ARCHITECTURE.md §9): same row shape → same table with
/// kind; different row shape → own table. The four variants here share the
/// Advantage row shape AND the `actor.create_feature` push contract
/// (foundry helper roadmap §5). Dyscrasias and (future) disciplines have
/// different row shapes and get their own tables.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdvantageKind {
    Merit,
    Flaw,
    Background,
    Boon,
}
```

- [ ] **Step 2: Extend the `Advantage` struct**

Edit the existing `pub struct Advantage` block:

```rust
/// A library entry for a VTM 5e Merit, Background, Flaw, or Boon.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Advantage {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub kind: AdvantageKind,
    pub tags: Vec<String>,
    pub properties: Vec<Field>,
    pub is_custom: bool,
    /// FVTT-import provenance. None = hand-authored locally (corebook or
    /// GM custom). Some = imported from a Foundry world; JSON shape
    /// described in the migration comment. Promoted to a tagged enum
    /// when a second source materializes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_attribution: Option<serde_json::Value>,
}
```

The `kind` field is placed between `description` and `tags` (matches the SQL column order from the migration). `#[serde(default)]` on `source_attribution` makes deserialization tolerant of pre-migration payloads in tests.

- [ ] **Step 3: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: the build now fails everywhere `Advantage` is constructed without `kind` — this is the safety net. Note the failing locations: `db/advantage.rs` (insert helper, tests), `db/seed.rs` (not yet — seed writes raw SQL), and any tests in `db/modifier.rs` / `db/saved_character.rs` / `tools/character.rs` that construct `Advantage` values directly.

- [ ] **Step 4: Commit (after Task 3 + 4 land — DO NOT commit Task 2 alone)**

Task 2's diff doesn't compile in isolation (struct construction sites break). The implementer subagent should leave the working tree dirty after Task 2 and proceed directly to Tasks 3 and 4. The commit for Task 2 lands as part of Task 3's commit (atomic struct extension + first consumer update).

If using the single-implementer-per-task pattern strictly, dispatch Tasks 2 + 3 to the same implementer with the instruction "land Tasks 2 and 3 as a single atomic commit titled 'Extend Advantage struct + db/advantage.rs for kind/source_attribution'."

---

## Task 3: Update `db/advantage.rs` to read/write the new columns

**Goal:** Thread `kind` and `source_attribution` through every helper in `db/advantage.rs`. Add three new tests: kind round-trip on insert, source-attribution round-trip on insert, and a `list_by_kind` filter helper (no Tauri command — internal helper for Plan C's importer).

**Files:**
- Modify: `src-tauri/src/db/advantage.rs`

**Anti-scope:** Do NOT add any new `#[tauri::command]` in this task. (Plan C adds `import_advantage_from_world`; nothing in Plan A is FVTT-aware.) Do NOT touch `roll_random_advantage`'s tag-filter logic — kind filtering happens at the Tauri layer in Plan C, not here.

**Depends on:** Task 2

**Invariants cited:** ARCHITECTURE.md §5 (only `db/*` talks to SQLite), §7 (`db/advantage.*` error prefix on `Err` strings).

**Tests required:** YES — 3 new tests minimum (kind round-trip, source-attribution round-trip, `db_list_by_kind` filter). Plus all 11 existing tests must still pass.

- [ ] **Step 1: Update internal helpers**

In `src-tauri/src/db/advantage.rs`:

- Replace `db_list`'s SELECT column list and row-mapping to include `kind, source_attribution`. Parse `kind` via `serde_json::from_str` of a one-element wrapper or just match the string manually (`"merit" => AdvantageKind::Merit`, …).
- Add new internal helper `db_list_by_kind(pool, kind: AdvantageKind) -> Result<Vec<Advantage>, String>` running `SELECT … FROM advantages WHERE kind = ? ORDER BY is_custom ASC, id ASC`.
- Extend `db_insert` to take `kind: AdvantageKind` + `source_attribution: Option<&serde_json::Value>` parameters; serialize `source_attribution` via `serde_json::to_string` (or pass `None → SQL NULL`); INSERT 7 columns now.
- Extend `db_update` similarly (also accept `kind` so the GM can re-tag a custom advantage's kind via the form).

Use these column-string helpers (keep them inline in the file):

```rust
fn kind_to_str(k: AdvantageKind) -> &'static str {
    match k {
        AdvantageKind::Merit      => "merit",
        AdvantageKind::Flaw       => "flaw",
        AdvantageKind::Background => "background",
        AdvantageKind::Boon       => "boon",
    }
}

fn str_to_kind(s: &str) -> Result<AdvantageKind, String> {
    match s {
        "merit"      => Ok(AdvantageKind::Merit),
        "flaw"       => Ok(AdvantageKind::Flaw),
        "background" => Ok(AdvantageKind::Background),
        "boon"       => Ok(AdvantageKind::Boon),
        other        => Err(format!("db/advantage: unknown kind: {other}")),
    }
}
```

- [ ] **Step 2: Update Tauri command signatures**

Extend `add_advantage` and `update_advantage` to accept `kind: AdvantageKind` (required, before `tags`). Frontend wrapper signature changes in Task 6.

`list_advantages`, `delete_advantage`, `roll_random_advantage` are unchanged at the Tauri boundary — they only consume the row by id or list-all-then-filter-in-rust.

- [ ] **Step 3: Update the in-memory test schema**

In the `#[cfg(test)] mod tests` block, the `test_pool()` helper currently creates `advantages` without the new columns. Update its CREATE TABLE to mirror the production schema post-migration:

```rust
async fn test_pool() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE advantages (
            id                 INTEGER PRIMARY KEY AUTOINCREMENT,
            name               TEXT NOT NULL,
            description        TEXT NOT NULL DEFAULT '',
            tags_json          TEXT NOT NULL DEFAULT '[]',
            properties_json    TEXT NOT NULL DEFAULT '[]',
            is_custom          INTEGER NOT NULL DEFAULT 0,
            kind               TEXT NOT NULL DEFAULT 'merit'
                CHECK(kind IN ('merit','flaw','background','boon')),
            source_attribution TEXT
        )"
    ).execute(&pool).await.unwrap();
    pool
}
```

Note: column order matches the post-migration table (ALTER TABLE appends, so `kind` + `source_attribution` are at the end in the real DB; tests should mirror that).

- [ ] **Step 4: Update existing tests for the new `kind` parameter**

Every `db_insert(&pool, "Name", "desc", &tags, &props)` call in tests becomes `db_insert(&pool, "Name", "desc", AdvantageKind::Merit, None, &tags, &props)` (or pick the appropriate kind). The 3 `roll_random_*` tests can keep merits/backgrounds/flaws but explicitly pass the matching `AdvantageKind` value.

- [ ] **Step 5: Add three new tests**

```rust
#[tokio::test]
async fn kind_round_trips_through_insert_and_list() {
    let pool = test_pool().await;
    db_insert(&pool, "F1", "", AdvantageKind::Flaw, None, &[], &[]).await.unwrap();
    db_insert(&pool, "B1", "", AdvantageKind::Background, None, &[], &[]).await.unwrap();
    db_insert(&pool, "M1", "", AdvantageKind::Merit, None, &[], &[]).await.unwrap();
    let rows = db_list(&pool).await.unwrap();
    let by_name: std::collections::HashMap<_,_> = rows.iter().map(|r| (r.name.clone(), r.kind)).collect();
    assert_eq!(by_name["F1"], AdvantageKind::Flaw);
    assert_eq!(by_name["B1"], AdvantageKind::Background);
    assert_eq!(by_name["M1"], AdvantageKind::Merit);
}

#[tokio::test]
async fn source_attribution_round_trips_through_insert_and_list() {
    let pool = test_pool().await;
    let attribution = serde_json::json!({
        "source": "foundry",
        "world_title": "Chronicles of Chicago",
        "imported_at": "2026-05-14T12:00:00Z",
    });
    db_insert(&pool, "Imported Merit", "", AdvantageKind::Merit, Some(&attribution), &[], &[]).await.unwrap();
    db_insert(&pool, "Local Merit",    "", AdvantageKind::Merit, None, &[], &[]).await.unwrap();
    let rows = db_list(&pool).await.unwrap();
    let by_name: std::collections::HashMap<_,_> =
        rows.iter().map(|r| (r.name.clone(), r.source_attribution.clone())).collect();
    assert_eq!(by_name["Imported Merit"].as_ref().unwrap()["world_title"], "Chronicles of Chicago");
    assert!(by_name["Local Merit"].is_none());
}

#[tokio::test]
async fn list_by_kind_filters_correctly() {
    let pool = test_pool().await;
    db_insert(&pool, "F1", "", AdvantageKind::Flaw, None, &[], &[]).await.unwrap();
    db_insert(&pool, "F2", "", AdvantageKind::Flaw, None, &[], &[]).await.unwrap();
    db_insert(&pool, "M1", "", AdvantageKind::Merit, None, &[], &[]).await.unwrap();
    let flaws = db_list_by_kind(&pool, AdvantageKind::Flaw).await.unwrap();
    assert_eq!(flaws.len(), 2);
    assert!(flaws.iter().all(|r| r.kind == AdvantageKind::Flaw));
}
```

- [ ] **Step 6: Run `cargo test`**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::advantage`

Expected: all existing tests pass (with kind=Merit injected at the unused-discriminator sites); 3 new tests pass.

- [ ] **Step 7: Run `./scripts/verify.sh`**

Expected: `cargo check`, `cargo test`, `npm run check`, frontend build all green. `npm run check` may now fail at the TS boundary because `Advantage` has new required fields — Task 6 fixes the TS side; if `verify.sh` fails here, it's expected and Task 6 will resolve it. Move on to Task 4 (also independent of TS).

- [ ] **Step 8: Commit (atomic with Task 2)**

```
git add src-tauri/src/shared/types.rs src-tauri/src/db/advantage.rs
git commit -m "$(cat <<'EOF'
Extend Advantage struct + db/advantage.rs for kind/source_attribution

Adds AdvantageKind enum (merit/flaw/background/boon) and threads it
through every helper. source_attribution stays a JSON blob (Value) for
v1; promotion to typed enum deferred until a second source materializes.
Three new tests cover kind round-trip, source-attribution round-trip,
and the new internal db_list_by_kind filter helper.

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 4: Update `db/seed.rs` to write `kind` on INSERT

**Goal:** The destructive reseed (ADR 0002) must populate the new `kind` column on every corebook row. Annotate each `SeedRow` with its kind; emit `kind` in the INSERT.

**Files:**
- Modify: `src-tauri/src/db/seed.rs`

**Anti-scope:** Do NOT change the corebook seed content (names, descriptions, levels). Do NOT touch `seed_dyscrasias` — separate table, untouched by Phase 4.

**Depends on:** Task 2 (`AdvantageKind` enum must exist).

**Invariants cited:** ARCHITECTURE.md §6 ([ADR 0002](docs/adr/0002-destructive-reseed.md) — `is_custom = 0` rows are reseeded on every startup; this invariant is preserved). The new `kind` column is `NOT NULL`, so the INSERT must include it explicitly (the SQL `DEFAULT 'merit'` is a migration safety net for legacy rows; seed must be explicit).

**Tests required:** NO — the migration runtime check in Task 1 already verified seed kinds match expectations. Seed correctness is enforced by Task 1 Step 3's manual verification.

- [ ] **Step 1: Extend `SeedRow` struct**

In `src-tauri/src/db/seed.rs` (around line 120), add a `kind` field:

```rust
struct SeedRow {
    name: &'static str,
    description: &'static str,
    kind: crate::shared::types::AdvantageKind,
    tags: &'static [&'static str],
    level: Option<i64>,
    level_max: Option<i64>,
    source: &'static str,
}
```

- [ ] **Step 2: Annotate every `SeedRow` literal**

In `fn seed_rows()`, add `kind: AdvantageKind::Merit` (or `Flaw` / `Background`) to each `SeedRow { … }` literal. Cross-check with the existing `tags` array — the kind matches whichever of "Merit" / "Flaw" / "Background" appears there.

Currently in seed (per inspection):
- Iron Gullet, Eat Food, Bloodhound, Beautiful → `AdvantageKind::Merit`
- Allies, Contacts, Haven, Resources → `AdvantageKind::Background`
- Prey Exclusion, Enemy → `AdvantageKind::Flaw`
- (No Boons in V5 corebook seed.)

The `tags` array keeps its `"Merit"` / `"Background"` / `"Flaw"` string for backward UI compatibility (the existing tag-filter row in AdvantagesManager will keep working until Task 7 replaces it with the kind filter).

Add an import line at the top of the file if it doesn't already pull `AdvantageKind`: `use crate::shared::types::AdvantageKind;`.

- [ ] **Step 3: Update `seed_advantages` to emit `kind`**

Change the INSERT statement and bind list:

```rust
sqlx::query(
    "INSERT INTO advantages (name, description, kind, tags_json, properties_json, is_custom)
     VALUES (?, ?, ?, ?, ?, 0)"
)
.bind(row.name)
.bind(row.description)
.bind(kind_to_str(row.kind))  // local helper; mirror the one in db/advantage.rs
.bind(&tags_json)
.bind(&props_json)
.execute(pool)
.await?;
```

Add the local `kind_to_str` helper at the top of `seed.rs` (duplicating the one in `db/advantage.rs` is acceptable — small private helper, no abstraction wanted per the YAGNI override).

`source_attribution` is omitted from the INSERT — it defaults to NULL, which is correct for corebook rows.

- [ ] **Step 4: Run `cargo check`**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: clean.

- [ ] **Step 5: Verify the destructive reseed still works**

Delete the dev DB, run `npm run tauri dev`, exit immediately, then:

```bash
sqlite3 ~/.local/share/com.hampternt.vtmtools/vtmtools.db \
  "SELECT kind, COUNT(*) FROM advantages WHERE is_custom = 0 GROUP BY kind;"
```

Expected:
- `background  4`
- `flaw        2`
- `merit       4`

If counts are off, the seed annotation is wrong — go back to Step 2.

- [ ] **Step 6: Run `./scripts/verify.sh`**

Expected: green (cargo side). TS side may still fail until Task 6 — acceptable; verify cargo passes.

- [ ] **Step 7: Commit**

```
git add src-tauri/src/db/seed.rs
git commit -m "$(cat <<'EOF'
Seed corebook advantages with explicit kind discriminator

Annotates every SeedRow with AdvantageKind; seed_advantages now emits
the kind column on INSERT. Preserves ADR 0002 destructive reseed
invariant (is_custom = 0 rows replaced on every startup).

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 5: Audit non-advantage consumers for breakage

**Goal:** Confirm that `db/modifier.rs`, `tools/character.rs`, `db/saved_character.rs`, and `src/lib/saved-characters/diff.ts` don't depend on the now-deprecated tag-based kind discrimination. If any does, surface the gap and fix it in this task. If audit is clean, the task is a no-op commit-wise.

**Files (audit only — modifications conditional):**
- Inspect: `src-tauri/src/db/modifier.rs::materialize_advantage_modifier` (and its callers)
- Inspect: `src-tauri/src/tools/character.rs::character_add_advantage` and `character_remove_advantage`
- Inspect: `src-tauri/src/db/saved_character.rs::db_add_advantage`
- Inspect: `src/lib/saved-characters/diff.ts` (any reference to advantage tag strings)

**Anti-scope:** Do NOT refactor working code. Only fix if a consumer is genuinely broken by the new struct shape (i.e., constructs `Advantage { … }` directly without `kind` / `source_attribution`).

**Depends on:** Task 3 (`Advantage` struct extension applied)

**Invariants cited:** ARCHITECTURE.md §11 anti-scope discipline.

**Tests required:** NO (audit task)

- [ ] **Step 1: Compile-check the audit**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

If clean: every consumer reads `Advantage` via serde deserialization or by `id` — no breakage. Skip to Step 4.

If there are compilation errors: each one is a site that constructs `Advantage { … }` literally. For each error, **add `kind: AdvantageKind::Merit, source_attribution: None,` to the literal** (the test sites in modifier.rs/character.rs are test fixtures that construct fake advantage rows — `Merit` is the safe default discriminator for fixture rows).

- [ ] **Step 2: Verify `db/saved_character.rs::db_add_advantage`**

Read `src-tauri/src/db/saved_character.rs` lines ~215-240. The function validates featuretype against `merit/flaw/background/boon` via its own match. **Confirmation:** this function takes featuretype as a parameter (probably from a frontend call), not from a row in `advantages`. No change needed.

- [ ] **Step 3: Verify `diff.ts`**

Read `src/lib/saved-characters/diff.ts`. Search for any reference to `advantage`, `tags`, `merit`, `flaw`, `background`. The diff projection is about character-on-actor specialties/items, not local-library advantages — should be unrelated.

Confirmation: the file `src/lib/saved-characters/diff.ts` is referenced by `src/components/CompareModal.svelte` and does NOT consume `listAdvantages`. No change needed.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: cargo green; TS still failing (Task 6 resolves).

- [ ] **Step 5: Commit IF changes were made**

If Step 1 required edits to test fixtures:

```
git add -A
git commit -m "$(cat <<'EOF'
Update Advantage test fixtures for kind/source_attribution

Test sites that construct Advantage literals receive kind: Merit and
source_attribution: None defaults. No production behavior change.

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

If no changes were needed: skip the commit, leave a note in the task tracker that audit was clean.

---

## Task 6: Mirror types in `src/types.ts` + extend `src/lib/advantages/api.ts`

**Goal:** Cross the IPC boundary cleanly — TS types match the Rust struct shape post-Task 3; the typed wrappers thread `kind` through `addAdvantage` and `updateAdvantage`. No UI changes yet.

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/advantages/api.ts`

**Anti-scope:** Do NOT call `invoke(...)` directly anywhere (CLAUDE.md rule). Do NOT touch any `.svelte` component.

**Depends on:** Task 3 (`Advantage` Rust struct + Tauri command signatures extended)

**Invariants cited:** ARCHITECTURE.md §4 (typed frontend API wrappers; components never call `invoke` directly).

**Tests required:** NO (TS-only; `npm run check` is the gate).

- [ ] **Step 1: Mirror `AdvantageKind` in `src/types.ts`**

Add a discriminated string union (matches Rust's `#[serde(rename_all = "snake_case")]`):

```ts
export type AdvantageKind = 'merit' | 'flaw' | 'background' | 'boon';
```

- [ ] **Step 2: Extend the `Advantage` interface**

Find the existing `Advantage` interface in `src/types.ts` (it mirrors the Rust struct field-for-field with camelCase). Add the two new fields:

```ts
export interface Advantage {
  id: number;
  name: string;
  description: string;
  kind: AdvantageKind;
  tags: string[];
  properties: Field[];
  isCustom: boolean;
  /**
   * FVTT-import provenance. undefined = hand-authored locally
   * (corebook or GM custom). Shape (when defined):
   *   { source: 'foundry', worldTitle: string, worldId?: string,
   *     systemVersion?: string, importedAt: string /* ISO-8601 */ }
   * Stays as a free-form object until a second source materializes.
   */
  sourceAttribution?: Record<string, unknown>;
}
```

Note: the `serde(rename_all = "camelCase")` on the Rust struct converts `source_attribution → sourceAttribution` automatically. Use `sourceAttribution` on the TS side.

- [ ] **Step 3: Extend `AdvantageInput` and wrappers in `api.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';
import type { Advantage, AdvantageKind, Field } from '../../types';

export type AdvantageInput = {
  name: string;
  description: string;
  kind: AdvantageKind;
  tags: string[];
  properties: Field[];
};

export function listAdvantages(): Promise<Advantage[]> {
  return invoke<Advantage[]>('list_advantages');
}

export function addAdvantage(input: AdvantageInput): Promise<Advantage> {
  return invoke<Advantage>('add_advantage', input);
}

export function updateAdvantage(id: number, input: AdvantageInput): Promise<void> {
  return invoke<void>('update_advantage', { id, ...input });
}

export function deleteAdvantage(id: number): Promise<void> {
  return invoke<void>('delete_advantage', { id });
}

export function rollRandomAdvantage(tags: string[]): Promise<Advantage | null> {
  return invoke<Advantage | null>('roll_random_advantage', { tags });
}
```

- [ ] **Step 4: Run `npm run check`**

Expected: errors at every `addAdvantage(...)` / `updateAdvantage(...)` call site that doesn't yet pass `kind`. Those are in `AdvantagesManager.svelte` and `AdvantageForm.svelte` (and possibly tests). Task 7 fixes them.

- [ ] **Step 5: Commit**

```
git add src/types.ts src/lib/advantages/api.ts
git commit -m "$(cat <<'EOF'
Mirror AdvantageKind + sourceAttribution in TS types and api wrapper

Frontend Advantage interface gains kind (required) and sourceAttribution
(optional, free-form object). AdvantageInput requires kind for add/update.
UI consumers update in next task.

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 7: Update `AdvantagesManager.svelte` + `AdvantageForm.svelte`

**Goal:** Render a kind chip on each advantage card; add a kind filter to the existing filter row; the add/edit form gains a required kind selector. NO source-attribution chip yet (Plan C ships that).

**Files:**
- Modify: `src/tools/AdvantagesManager.svelte`
- Modify: `src/lib/components/AdvantageForm.svelte`
- Modify: `src/lib/components/AdvantageCard.svelte` (if kind chip renders inside the card — verify which component owns the per-row chip strip)

**Anti-scope:** Do NOT add a FVTT-source chip or import button. Do NOT change card layout/CSS tokens beyond adding the kind chip — reuse the existing tag-chip styling. Do NOT call `invoke()` directly.

**Depends on:** Task 6

**Invariants cited:** ARCHITECTURE.md §4 (typed wrappers only), §6 (no hardcoded hex; use CSS tokens from `:root` in `+layout.svelte`).

**Tests required:** NO (UI; manual smoke covers it). The verify gate is `npm run check` + `npm run build`.

- [ ] **Step 1: Add a `kind` selector to `AdvantageForm.svelte`**

Open `src/lib/components/AdvantageForm.svelte`. Locate the existing name/description/tags inputs. Add a `kind` selector (HTML `<select>` is sufficient — matches the existing form aesthetic):

- Add `let kind: AdvantageKind = $state(props.initial?.kind ?? 'merit');` near the top of the script block.
- Add a labeled `<select>` between the name input and the tags input, with options `merit / flaw / background / boon`. Use existing form-row CSS classes.
- Include `kind` in the `AdvantageInput` payload passed to the parent's save handler.

- [ ] **Step 2: Display a kind chip on each row in `AdvantagesManager.svelte`**

The exact location depends on whether per-row chips render in `AdvantagesManager.svelte` (top-level) or inside `AdvantageCard.svelte`. Read both files first.

If the chip strip lives in `AdvantageCard.svelte`, add the kind chip there with `data-kind={advantage.kind}` for CSS hooks. If it lives at the manager level, add it there.

Chip rendering — reuse the tag-chip pattern, but with a `data-kind` attribute so CSS in the same file can color-code:

```svelte
<span class="chip kind-chip" data-kind={adv.kind}>{capitalize(adv.kind)}</span>
```

CSS — add to the existing `<style>` block (or `AdvantageCard.svelte`'s style block):

```css
.kind-chip[data-kind="merit"]      { background: var(--accent-merit, var(--accent)); }
.kind-chip[data-kind="flaw"]       { background: var(--accent-flaw, var(--danger)); }
.kind-chip[data-kind="background"] { background: var(--accent-background, var(--accent-muted)); }
.kind-chip[data-kind="boon"]       { background: var(--accent-boon, var(--accent-strong)); }
```

The CSS uses `var(...)` with fallbacks so no token additions are required in `+layout.svelte` for v1; if the GM wants distinct accents per kind later, they're added to `:root` then.

- [ ] **Step 3: Add a kind filter to the existing filter row**

In `AdvantagesManager.svelte`, the existing `activeTags: Set<string>` filter already covers tag-based filtering. Decide between:

- **Option A (recommended, smaller):** Re-derive the `"Merit"` / `"Flaw"` / `"Background"` tags as kind-aware filter chips. Existing rows still carry these tag strings (Task 4 preserved them), so the tag filter implicitly covers kind. Add a small visual treatment so kind-tag chips render distinctly (e.g. `tags-row .kind-tag-chip` styling — match the `data-kind` accent above).
- **Option B (cleaner but more change):** Replace the tag-filter row with parallel "Kind" + "Tag" rows. New state `activeKinds: Set<AdvantageKind>` + new derived `matchesKind`. More code; cleaner separation. Add `__all__` sentinel matching the existing pattern.

Pick Option A for Plan A; Option B becomes a follow-up if the kind/tag conflation in the UI confuses users.

- [ ] **Step 4: Wire `kind` into Add / Edit dispatches**

Find every call to `addAdvantage({ ... })` and `updateAdvantage(id, { ... })` in `AdvantagesManager.svelte`. Each must now pass `kind`. The form's `kind` state (Task 1) is the source.

- [ ] **Step 5: Run `npm run check` then `npm run build`**

Expected: both green.

- [ ] **Step 6: Manual smoke**

Run `npm run tauri dev`. Open the Advantages tool.

- ✅ Existing rows render with their kind chip (3 merits / 2 flaws / 4 backgrounds visible — corebook seed counts).
- ✅ Tag filter still works (clicking "Merit" tag filters to merits).
- ✅ Add Advantage form has a kind selector; defaults to "Merit"; saving a new "Boon" row shows up with the boon chip color.
- ✅ Edit existing custom row: kind selector preselected; changing kind + saving persists across app restart.

- [ ] **Step 7: Run `./scripts/verify.sh`**

Expected: full green.

- [ ] **Step 8: Commit**

```
git add src/tools/AdvantagesManager.svelte src/lib/components/AdvantageForm.svelte src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
Render kind chip and kind selector in AdvantagesManager

Each row now shows its kind discriminator visually. Add/Edit form
requires a kind selection (defaults to Merit). CSS color-coding via
data-kind attribute with sensible token fallbacks; no hardcoded hex.

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 8: Update `ARCHITECTURE.md` §6 + §9

**Goal:** Document the two new invariants introduced by Plan A: (1) `is_custom` semantics shifted from binary to tri-state (after Plan C lands), and (2) the partitioning rule for adding a library kind.

**Files:**
- Modify: `ARCHITECTURE.md`

**Anti-scope:** Do NOT update the IPC inventory in §4 (Plan A adds no new commands). Plan B and Plan C will update it when they add their commands.

**Depends on:** Task 7

**Invariants cited:** ARCHITECTURE.md is itself the source of truth; this task amends it.

**Tests required:** NO (docs).

- [ ] **Step 1: Add §6 paragraph on `is_custom` semantics**

In `ARCHITECTURE.md` §6 Invariants, find the existing paragraph about destructive reseed (or add adjacent to it). Add:

> - **`advantages.is_custom` tri-state read.** Pre-Phase-4 the column was binary (0 = corebook reseed-managed, 1 = GM hand-authored). Post-Phase-4 imported FVTT rows are also `is_custom = 1` (so destructive reseed leaves them alone — same survival semantics as hand-authored) BUT they're visually distinguished by a non-null `source_attribution` JSON column. The tri-state is therefore:
>   - `is_custom = 0` → corebook seed; replaced on every startup (ADR 0002).
>   - `is_custom = 1 AND source_attribution IS NULL` → GM hand-authored locally; survives reseed; editable in AdvantagesManager.
>   - `is_custom = 1 AND source_attribution IS NOT NULL` → FVTT-imported; survives reseed; UI shows source chip with world title.
>
>   UI filters and reaper helpers MUST treat the latter two as semantically distinct "local" vs. "imported" states despite identical persistence flags.

- [ ] **Step 2: Add §9 paragraph on "Add a library kind"**

In `ARCHITECTURE.md` §9 Extensibility seams (after "Add a VTT bridge source" and before "Add a card-shaped surface"), insert:

> - **Add a library kind.** Phase 4 partitioning rule: **same row shape → polymorphic table with a `kind` discriminator column; different row shape → its own table.** The four featuretype variants (`merit`, `flaw`, `background`, `boon`) share an identical row shape AND an identical Foundry push contract (`actor.create_feature` accepts featuretype as a payload field — foundry helper roadmap §5), so they share the polymorphic `advantages` table. Dyscrasias have a distinct shape (`resonance_type`, `bonus`) and their own table. Disciplines (when they land) will have yet another distinct shape (power tree, Amalgam refs, level-gated powers) and their own table. To add a new variant that shares the advantage row shape: extend `AdvantageKind` enum in `shared/types.rs`, update the SQL CHECK constraint via a new migration, annotate any new seed rows, and wire the chip in `AdvantagesManager.svelte`. To add a new variant that does NOT share the row shape: new table + new `db/<kind>.rs` module + new manager tool, following the dyscrasia pattern.

- [ ] **Step 3: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 4: Commit**

```
git add ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
Document is_custom tri-state + library-kind partitioning rule

§6 records the post-Phase-4 semantic shift on advantages.is_custom
(reseed-managed / local / imported). §9 codifies when a new library
kind shares the polymorphic table vs gets its own.

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 9: Final verification gate

**Goal:** Confirm Plan A end-to-end before handing off to Plan B / Plan C.

- [ ] **Step 1: Run `./scripts/verify.sh`**

Expected: full green.

- [ ] **Step 2: Smoke-test against a clean dev DB**

Delete the dev DB; `npm run tauri dev`; open Advantages tool; verify:

- 10 corebook rows present (4 merit / 2 flaw / 4 background).
- Each row shows the correct kind chip.
- Add a custom Boon → appears with boon-colored chip.
- Restart app → custom row survives; corebook reseeds (kind preserved on corebook).

- [ ] **Step 3: Update plan checkbox state**

Mark every Task 1–8 step as `[x]` in this file. Commit:

```
git add docs/superpowers/plans/2026-05-14-library-sync-plan-a-library-schema-foundation.md
git commit -m "$(cat <<'EOF'
Mark Plan A tasks complete

Plan A (library schema foundation) shipped. Unblocks Plan B (push) and
Plan C (pull + attribution + dedup).

Refs #13 #14 #15.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

Plan A is complete. Plan B and Plan C may now dispatch (Plan B in parallel since #13 is independent of Plan C; Plan C is sequential after Plan B because its UI for source-attribution depends on Plan B's item subscription enabling the pull path).
