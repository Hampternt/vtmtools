# Library Sync — Plan B: Push + item subscription

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plans (A + B + C) tasks are committed, run a SINGLE `code-review:code-review` against the full Phase 4 branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** Two independent pieces of Foundry-bridge work that ship together because they share the same module version bump:
- **#27 — Item subscription enablement.** Extend the `bridge.subscribe` protocol to accept `collection: "item"`. The Foundry module hooks `createItem`/`updateItem`/`deleteItem` for **world-level** Item docs only (embedded actor items already arrive via the existing actor enrichment path). Desktop adds `FoundryInbound::Items` snapshot variant + per-item upsert/delete variants + `bridge://foundry/items-updated` event.
- **#13 — Push library entry → Foundry world.** New `storyteller.*` umbrella (one helper: `storyteller.create_world_item`) and a Tauri command `push_advantage_to_world(id: i64)` that loads the local advantage row by id, maps `kind → featuretype`, and pushes a new world-level Item doc to the active Foundry world. Composes Plan A's `kind` column directly. Push button on each AdvantagesManager row gated on Foundry connectivity.

**Architecture:** Two umbrellas added at once because they share a single module version bump (0.6.0 — additive). The `item.*` subscription is purely **inbound**: the desktop sends `bridge.subscribe { collection: "item" }`, the module attaches Foundry hooks and pushes initial snapshot + future deltas. The `storyteller.*` umbrella is purely **outbound**: the desktop sends `storyteller.create_world_item`, the module calls `Item.create(...)` at world level. No interaction between the two — they're in the same plan only because they ship together to keep the bridge protocol consistent.

**World-level vs. embedded distinction:** Foundry items have a `parent` property — `null` for world-level Item docs, set to an Actor for embedded items. The new subscriber filters `Hooks.on("createItem", item => { if (item.parent === null) push(...) })` so world-level items are reported once via the new path, never double-counted with the existing actor-enrichment path.

**Tech Stack:** Rust (`sqlx`, `serde`, `serde_json`, `tauri`, `tokio`), JavaScript (Foundry module ES module — no transpilation).

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 4 (sketch) + architect-advisor recommendation. Source sketches in GitHub issues #27 and #13.

**Architecture reference:** `ARCHITECTURE.md` §4 (Bridge WebSocket protocol — new inbound variants, new event), §4 (Tauri IPC — adds `push_advantage_to_world` command), §5 (only `bridge/*` talks to WebSocket), §7 (error envelope routing — already shipped in Plan 0; Plan B's outbound errors flow through it), §9 ("Add a Tauri command" seam + bridge subscription pattern).

**Depends on:** Plan A (uses `Advantage.kind` to map → featuretype; uses `AdvantageKind` enum). Bridge protocol consolidation (Plan 0) has already shipped — `bridge.subscribe` envelope, `subscribers` registry, and the Hello extension are all in place.

**Unblocks:** Plan C (pull-from-world consumes the items snapshot delivered by #27's subscription).

**Issues closed:** #13, #27. (#14, #15, #16 remain — Plan C.)

---

## File structure

### New files
- `src-tauri/src/tools/library_push.rs` — `push_advantage_to_world(id: i64)` Tauri command. Loads the advantage by id; maps `kind` → featuretype string; composes `actions::storyteller::build_create_world_item(...)`; routes via `bridge::commands::send_to_source_inner(state, SourceKind::Foundry, text)`. No-op (returns Ok) if Foundry not connected — same semantics as `bridge_set_attribute` per `send_to_source_inner` impl.
- `vtmtools-bridge/scripts/foundry-actions/item.js` — `itemsSubscriber.attach(socket) / .detach()` following the `actorsSubscriber` template. Pushes initial snapshot from `game.items.contents` (world-level only). Hooks `createItem`/`updateItem`/`deleteItem`; each hook filters `if (item.parent !== null) return;` to keep embedded-on-actor items out.
- `src/lib/library/api.ts` — typed wrapper `pushAdvantageToWorld(id: number): Promise<void>` for the new Tauri command. (Library namespace established here so Plan C extends it with `importAdvantagesFromWorld`.)

### Modified files
- `src-tauri/src/bridge/foundry/actions/storyteller.rs` — replace the reserved-umbrella stub (1-line comment) with `build_create_world_item(name, featuretype, description, points) -> Result<Value, String>` builder; module-prefixed errors per ARCH §7. The `storyteller.*` umbrella is activated by Plan B's first helper — future helpers register here without churning the umbrella convention.
- `vtmtools-bridge/scripts/foundry-actions/storyteller.js` — replace the reserved-umbrella stub (`export const handlers = {}`) with `createWorldItem(msg)` handler that calls `Item.create({ type: "feature", name, system: { featuretype, description, points } })` at world level. No `wireExecutor` wrapper (no `actor_id` resolution needed — world-level).
- `src-tauri/src/bridge/foundry/types.rs` — add `FoundryInbound::Items { items: Vec<FoundryWorldItem> }`, `FoundryInbound::WorldItemUpsert { item: FoundryWorldItem }`, `FoundryInbound::WorldItemDeleted { item_id: String }`; add `FoundryWorldItem` struct (id, name, type, featuretype: Option, system: Value).
- `src-tauri/src/bridge/types.rs` — add `CanonicalWorldItem { source: SourceKind, id, name, kind: String, featuretype: Option<String>, system: Value }` (canonical source-agnostic shape; `system` stays Value to keep the bridge a dumb pipe).
- `src-tauri/src/bridge/source.rs` — extend `InboundEvent` enum with `WorldItemsSnapshot { source, items }`, `WorldItemUpsert { source, item }`, `WorldItemDeleted { source, item_id }` variants.
- `src-tauri/src/bridge/foundry/mod.rs::handle_inbound` — match arms for the three new `FoundryInbound` variants → produce the corresponding `InboundEvent`s with `source = SourceKind::Foundry`.
- `src-tauri/src/bridge/foundry/translate.rs` (or new helper) — `to_canonical_world_item(item: &FoundryWorldItem) -> CanonicalWorldItem` (minimal translation; `kind` derived from `item.type`, `featuretype` lifted from `item.system.featuretype` if present).
- `src-tauri/src/bridge/mod.rs::accept_loop` — three new arms in the event-routing match (snapshot / upsert / delete). All three emit `bridge://foundry/items-updated` with the full snapshot from a new `BridgeState.world_items: HashMap<SourceKind, HashMap<String, CanonicalWorldItem>>` cache. Pattern mirrors `bridge://characters-updated` (snapshot replaces, upsert inserts, delete evicts; emit full snapshot each time).
- `src-tauri/src/bridge/mod.rs::BridgeState` — add `world_items: tokio::sync::Mutex<HashMap<SourceKind, HashMap<String, CanonicalWorldItem>>>`.
- `src-tauri/src/bridge/commands.rs` — add `bridge_get_world_items(source: SourceKind) -> Vec<CanonicalWorldItem>` for frontend initial-load (mirrors `bridge_get_characters`).
- `src-tauri/src/lib.rs` — register `push_advantage_to_world` and `bridge_get_world_items` in `invoke_handler!`.
- `src-tauri/src/tools/mod.rs` — `pub mod library_push;`.
- `vtmtools-bridge/scripts/foundry-actions/bridge.js` — register `item: itemsSubscriber` in the `subscribers` map.
- `vtmtools-bridge/module.json` — version 0.5.0 → 0.6.0 (additive).
- `src/store/bridge.svelte.ts` — add `worldItems: Record<SourceKind, CanonicalWorldItem[]>` reactive state; subscribe to `bridge://foundry/items-updated` and hydrate from `bridge_get_world_items` on mount. (Plan C consumes this.)
- `src/types.ts` — mirror `CanonicalWorldItem`.
- `src/tools/AdvantagesManager.svelte` — add a "Push to world" button per row, visible only when Foundry connection is active (consume `bridgeStatus.foundry` from `src/store/bridge.svelte.ts`). On click, call `pushAdvantageToWorld(id)`; toast on error.
- `ARCHITECTURE.md` §4 — append `push_advantage_to_world` to `tools/library_push.rs` IPC entry; append `bridge_get_world_items` to `bridge/commands.rs` entry; bump command total 63 → 65; add `bridge://foundry/items-updated` to the events table; add `storyteller.*` umbrella + `item` subscription collection mentions to the Bridge WebSocket protocol section.

### Files explicitly NOT touched
- `src-tauri/src/db/advantage.rs` — Plan A territory (frozen post-Plan-A).
- `src-tauri/src/db/dyscrasia.rs` — Plan B does NOT push dyscrasias to the world. The spec sketch said "merit/dyscrasia → Foundry world" but `actor.create_feature` operates on actors, not the world; pushing a dyscrasia-as-world-item conflates merit semantics with dyscrasia semantics. **Anti-scope: dyscrasia push to world remains out of scope.** If the GM wants a dyscrasia in the world library, they create it as a custom merit via `AdvantagesManager.svelte → kind: merit`.
- Plan A frozen files (Advantage struct, migration).
- Plan C frozen files (`src/lib/library/api.ts::importAdvantagesFromWorld`, dedup logic, source-attribution chip rendering).

---

## Task overview

| # | Task | Depends on | Tests |
|---|---|---|---|
| 1 | Add `FoundryWorldItem` + 3 new `FoundryInbound` variants in `bridge/foundry/types.rs` | none | YES (deserialize round-trip for each variant) |
| 2 | Add `CanonicalWorldItem` in `bridge/types.rs` + mirror in `src/types.ts` | 1 | NO (struct definition) |
| 3 | Add `InboundEvent` variants in `bridge/source.rs` + `translate.rs::to_canonical_world_item` | 1, 2 | YES (1 test for translate) |
| 4 | Wire `FoundryInbound` → `InboundEvent` arms in `bridge/foundry/mod.rs::handle_inbound` | 3 | YES (3 tests, one per variant) |
| 5 | Extend `BridgeState` with `world_items` cache + route `InboundEvent` arms in `bridge/mod.rs` + add `bridge_get_world_items` Tauri command (bumps ARCH §4 total 63 → 64 + adds events table row in-commit) | 4 | NO (covered by manual smoke + cargo build) |
| 6 | New JS `vtmtools-bridge/scripts/foundry-actions/item.js` with `itemsSubscriber` (world-level filter) + register `item` in `bridge.js` subscribers | none (JS-only) | NO (manual smoke against live Foundry world) |
| 7 | New JS `vtmtools-bridge/scripts/foundry-actions/storyteller.js` with `createWorldItem` handler + register in `index.js` | none (JS-only) | NO (manual smoke against live Foundry world) |
| 8 | Bump `module.json` 0.5.0 → 0.6.0 | 6, 7 | NO |
| 9 | New Rust `bridge/foundry/actions/storyteller.rs` with `build_create_world_item` builder | none (Rust-only, parallel with 6/7/8) | YES (envelope shape + invalid featuretype rejection) |
| 10 | New Rust `tools/library_push.rs` with `push_advantage_to_world` Tauri command (bumps ARCH §4 total 64 → 65 in-commit) | 9 + Plan A | YES (1 happy-path test against in-memory pool + faked outbound channel) |
| 11 | Register `push_advantage_to_world` + `bridge_get_world_items` in `lib.rs` (no ARCH change — declarations already documented) | 10 + (new `bridge_get_world_items` from Task 5) | NO |
| 12 | Frontend typed wrapper `src/lib/library/api.ts::pushAdvantageToWorld` | 11 | NO |
| 13 | `src/store/bridge.svelte.ts` reactive `worldItems` state + event subscription | 5 | NO |
| 14 | AdvantagesManager "Push to world" button | 12, 13 | NO (manual smoke) |
| 15 | ARCHITECTURE.md §4 — Bridge WebSocket protocol prose only (IPC inventory and events-table parts already landed in Tasks 5 + 10) | 14 | NO |
| 16 | Final verification gate (incl. live-Foundry E2E smoke) | all | runs `./scripts/verify.sh` + manual E2E |

Tasks 1, 6, 7, 9 are independent (Rust types / JS subscriber / JS handler / Rust builder) and can dispatch in parallel after this plan begins. Tasks 2–5 thread the Rust inbound side sequentially. Tasks 10–14 thread the push outbound side sequentially.

---

## Task 1: Add `FoundryWorldItem` + 3 new `FoundryInbound` variants

**Goal:** Extend the typed inbound wire definitions to accept world-level item snapshots / upserts / deletes. Provides Rust-side compile-time guarantees that JS-side payload shapes are honored.

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs`

**Anti-scope:** Do NOT modify the existing `FoundryActor::items` field (embedded actor items continue to flow through that). Do NOT add any wire variant beyond the three named here.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §4 (Bridge WebSocket protocol — typed inbound). Wire-protocol decision A from foundry helper roadmap §3 (typed-per-helper).

**Tests required:** YES — one deserialize-round-trip test per variant (3 tests).

- [x] **Step 1: Add `FoundryWorldItem` struct**

In `src-tauri/src/bridge/foundry/types.rs`, after the existing `FoundryActor` struct (around line 92), add:

```rust
/// World-level Item document. Distinct from embedded-on-actor items —
/// those arrive via `FoundryActor::items` and stay scoped to their
/// parent actor. World-level items have no parent (Foundry's
/// `item.parent === null`); the module's `item` subscriber filters
/// for them explicitly so this struct never carries an actor_id.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FoundryWorldItem {
    /// Foundry document _id.
    pub id: String,
    pub name: String,
    /// Foundry Item type ("feature", "speciality", "power", etc.).
    #[serde(rename = "type")]
    pub item_type: String,
    /// system.featuretype when item.type == "feature" (merit/flaw/
    /// background/boon). None for non-feature items.
    #[serde(default)]
    pub featuretype: Option<String>,
    /// Raw `item.system` blob — opaque to Rust; Plan C's importer picks
    /// fields (description, points, level) as needed.
    #[serde(default)]
    pub system: serde_json::Value,
}
```

- [x] **Step 2: Extend `FoundryInbound` enum**

In the existing `pub enum FoundryInbound` block (lines 11-48), add three new variants:

```rust
    /// World-level item snapshot — pushed by the module on first
    /// subscribe to `collection: "item"`. Replaces the entire cached
    /// items slice for this source.
    Items {
        items: Vec<FoundryWorldItem>,
    },

    /// One world-level item was created or updated.
    /// Triggered by the module's createItem / updateItem hooks
    /// (filtered to parent === null).
    WorldItemUpsert {
        item: FoundryWorldItem,
    },

    /// One world-level item was deleted.
    /// Triggered by the module's deleteItem hook (parent === null filter).
    WorldItemDeleted {
        item_id: String,
    },
```

Note: the existing `ItemDeleted { actor_id, item_id }` variant stays — it's actor-scoped and serves modifier reaping (a different concern). The new `WorldItemDeleted` is world-scoped (no actor_id).

- [x] **Step 3: Add deserialize tests**

In the existing `#[cfg(test)] mod tests` block (or add one if absent — verify file structure first), add:

```rust
#[test]
fn items_snapshot_deserializes() {
    let wire = r#"{
        "type": "items",
        "items": [
            { "id": "i1", "name": "Iron Gullet", "type": "feature",
              "featuretype": "merit", "system": { "description": "..." } }
        ]
    }"#;
    let parsed: FoundryInbound = serde_json::from_str(wire).expect("parses");
    match parsed {
        FoundryInbound::Items { items } => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, "Iron Gullet");
            assert_eq!(items[0].featuretype.as_deref(), Some("merit"));
        }
        other => panic!("expected Items, got {other:?}"),
    }
}

#[test]
fn world_item_upsert_deserializes() {
    let wire = r#"{
        "type": "world_item_upsert",
        "item": { "id": "i7", "name": "Bloodhound", "type": "feature",
                  "featuretype": "merit", "system": {} }
    }"#;
    let parsed: FoundryInbound = serde_json::from_str(wire).expect("parses");
    match parsed {
        FoundryInbound::WorldItemUpsert { item } => {
            assert_eq!(item.id, "i7");
        }
        other => panic!("expected WorldItemUpsert, got {other:?}"),
    }
}

#[test]
fn world_item_deleted_deserializes() {
    let wire = r#"{ "type": "world_item_deleted", "item_id": "i99" }"#;
    let parsed: FoundryInbound = serde_json::from_str(wire).expect("parses");
    match parsed {
        FoundryInbound::WorldItemDeleted { item_id } => {
            assert_eq!(item_id, "i99");
        }
        other => panic!("expected WorldItemDeleted, got {other:?}"),
    }
}
```

Wire `type` strings come from `#[serde(tag = "type", rename_all = "snake_case")]` on the enum. The variant names map: `Items → "items"`, `WorldItemUpsert → "world_item_upsert"`, `WorldItemDeleted → "world_item_deleted"`. The JS-side payload must emit these exact strings (Task 6).

- [x] **Step 4: Run `cargo test`**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::types`

Expected: 3 new tests pass; no regressions.

- [x] **Step 5: Run `./scripts/verify.sh`**

Expected: cargo green; downstream code (`bridge/foundry/mod.rs::handle_inbound`) may not yet handle the new variants — that compiles fine as long as `match` is non-exhaustive over `FoundryInbound`. **Verify:** current `mod.rs::handle_inbound` uses an exhaustive match over `FoundryInbound`, so this step WILL fail at cargo check. That's expected — Task 4 fixes it. If verify.sh fails here at `bridge/foundry/mod.rs::handle_inbound`, commit Task 1 with the failure pinned to Task 4, OR roll Tasks 1+4 into the same commit. **Recommended:** dispatch Tasks 1+4 to the same implementer as a single atomic commit since they share an exhaustiveness invariant.

- [x] **Step 6: Commit (joint with Task 4 — defer)**

Hold this commit until Task 4 lands; the joint commit is at Task 4 Step 5.

---

## Task 2: Add `CanonicalWorldItem` + mirror in `src/types.ts`

**Goal:** Source-agnostic canonical shape for world items. Stays minimal — `system` remains a JSON Value so the bridge stays a dumb pipe per the architect anti-recommendation.

**Files:**
- Modify: `src-tauri/src/bridge/types.rs`
- Modify: `src/types.ts`

**Anti-scope:** Do NOT typedef the contents of `system` (it varies by item.type). Do NOT add Roll20-specific fields — `CanonicalWorldItem` is Foundry-only in practice for v1; the `source: SourceKind` field is for forward-compat only.

**Depends on:** Task 1

**Invariants cited:** ARCHITECTURE.md §3 (`bridge/types.rs` holds canonical shapes); §5 (bridge stays a translation layer).

**Tests required:** NO

- [x] **Step 1: Add `CanonicalWorldItem` struct**

In `src-tauri/src/bridge/types.rs`, after `CanonicalCharacter` (or wherever canonical types live; verify file structure), add:

```rust
/// Source-agnostic shape for a world-level (compendium-style) Item doc.
/// Foundry is the only producer in v1; Roll20 has no analog. The shape
/// is intentionally minimal — `system` stays as `serde_json::Value` so
/// the bridge stays a dumb pipe. Consumers (Plan C importer) read
/// per-kind fields from `system` directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalWorldItem {
    pub source: SourceKind,
    /// Foundry document _id (or other source's stable id).
    pub id: String,
    pub name: String,
    /// Foundry Item type — e.g. "feature", "speciality", "power".
    pub kind: String,
    /// system.featuretype when kind == "feature" — one of
    /// {"merit","flaw","background","boon"} for the cases Plan B's
    /// push and Plan C's pull both care about. None otherwise.
    pub featuretype: Option<String>,
    pub system: serde_json::Value,
}
```

Note: the `kind` field here is the Foundry **Item type** ("feature" / "speciality" / "power"), NOT the `AdvantageKind` enum from Plan A (which is the featuretype sub-discriminator for `kind = "feature"` items). They're related but distinct. Plan C's importer maps `(kind == "feature", featuretype) → AdvantageKind`.

- [x] **Step 2: Mirror in `src/types.ts`**

Add to `src/types.ts` (camelCase, matching serde):

```ts
export interface CanonicalWorldItem {
  source: SourceKind;
  id: string;
  name: string;
  /** Foundry Item type — "feature", "speciality", "power", … */
  kind: string;
  /** system.featuretype for feature-typed items; one of
   *  'merit' | 'flaw' | 'background' | 'boon' in practice. */
  featuretype?: string;
  /** Raw item.system blob — opaque on the TS side; Plan C
   *  reads specific fields per Foundry Item.type. */
  system: Record<string, unknown>;
}
```

- [x] **Step 3: Run `./scripts/verify.sh`**

Expected: green on the cargo + TS sides; nothing consumes the new type yet.

- [x] **Step 4: Commit**

```
git add src-tauri/src/bridge/types.rs src/types.ts
git commit -m "$(cat <<'EOF'
Add CanonicalWorldItem (source-agnostic world-level item shape)

Mirrors the post-Plan-B inbound wire variants. system stays a Value
on Rust / Record<string, unknown> on TS — bridge layer remains a dumb
pipe; per-kind decoding happens at consumer (Plan C importer).

Refs #27 #14.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 3: Add `InboundEvent` variants + `translate.rs::to_canonical_world_item`

**Goal:** Bridge-internal event variants for the three world-item flows, plus the Foundry-side translator.

**Files:**
- Modify: `src-tauri/src/bridge/source.rs` (extend `InboundEvent` enum)
- Modify: `src-tauri/src/bridge/foundry/translate.rs` (new helper)

**Anti-scope:** Do NOT make `to_canonical_world_item` look up actor context — world items are parent-less by construction. Do NOT add a "snapshot replace" / "delta apply" distinction at the InboundEvent level — three variants is enough.

**Depends on:** Tasks 1, 2

**Invariants cited:** ARCH §4 (InboundEvent enum is the bridge-internal protocol between handle_inbound and accept_loop).

**Tests required:** YES — 1 test for `to_canonical_world_item`.

- [x] **Step 1: Extend `InboundEvent`**

In `src-tauri/src/bridge/source.rs`, add to the existing `InboundEvent` enum:

```rust
    /// Source pushed a full world-level item snapshot. The bridge cache
    /// replaces this source's world-items slice — every entry whose
    /// `source` matches is dropped, then `items` are inserted. Empty
    /// `items` is legal and means "this source now has zero
    /// world-level items".
    WorldItemsSnapshot {
        source: crate::bridge::types::SourceKind,
        items: Vec<crate::bridge::types::CanonicalWorldItem>,
    },
    /// One world-level item was added or changed. The bridge cache
    /// inserts or overwrites a single entry keyed by `(source, id)`.
    WorldItemUpsert {
        source: crate::bridge::types::SourceKind,
        item: crate::bridge::types::CanonicalWorldItem,
    },
    /// One world-level item was removed. The bridge cache evicts the
    /// entry keyed by `(source, id)`.
    WorldItemDeleted {
        source: crate::bridge::types::SourceKind,
        item_id: String,
    },
```

- [x] **Step 2: Add `to_canonical_world_item` in `translate.rs`**

In `src-tauri/src/bridge/foundry/translate.rs`, add (or add a new module if translate.rs is actor-only; check the file's current structure first):

```rust
pub fn to_canonical_world_item(
    item: &crate::bridge::foundry::types::FoundryWorldItem,
) -> crate::bridge::types::CanonicalWorldItem {
    crate::bridge::types::CanonicalWorldItem {
        source: crate::bridge::types::SourceKind::Foundry,
        id: item.id.clone(),
        name: item.name.clone(),
        kind: item.item_type.clone(),
        featuretype: item.featuretype.clone(),
        system: item.system.clone(),
    }
}
```

- [x] **Step 3: Add a test for the translator**

In `translate.rs`'s `#[cfg(test)] mod tests` block (or add one):

```rust
#[test]
fn translate_world_item_preserves_featuretype() {
    let wire = crate::bridge::foundry::types::FoundryWorldItem {
        id: "i1".into(),
        name: "Iron Gullet".into(),
        item_type: "feature".into(),
        featuretype: Some("merit".into()),
        system: serde_json::json!({"description": "..."}),
    };
    let canonical = to_canonical_world_item(&wire);
    assert_eq!(canonical.source, crate::bridge::types::SourceKind::Foundry);
    assert_eq!(canonical.id, "i1");
    assert_eq!(canonical.name, "Iron Gullet");
    assert_eq!(canonical.kind, "feature");
    assert_eq!(canonical.featuretype.as_deref(), Some("merit"));
}
```

- [x] **Step 4: Run `cargo test` (target translate + source modules)**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::`

Expected: existing tests still pass; new test passes.

- [x] **Step 5: Commit (atomic with Task 4 — defer)**

Hold this commit until Task 4; combined commit at Task 4 Step 5.

---

## Task 4: Wire `FoundryInbound` → `InboundEvent` in `handle_inbound`

**Goal:** Complete the inbound translation pipeline so the bridge accept loop can route world-item events.

**Files:**
- Modify: `src-tauri/src/bridge/foundry/mod.rs::handle_inbound`

**Anti-scope:** Do NOT add new state to `FoundrySource` — it stays stateless per ADR 0006. Do NOT emit Tauri events directly from `handle_inbound` — emission happens in `accept_loop` (Task 5).

**Depends on:** Tasks 1, 2, 3

**Invariants cited:** ARCH §5 (BridgeSource impls are stateless; state lives in BridgeState). ADR 0006.

**Tests required:** YES — 3 tests (one per new variant), pattern matches the existing `actor_deleted_inbound_produces_character_removed_event` test.

- [x] **Step 1: Add three arms to `handle_inbound`**

In the `match parsed` block of `src-tauri/src/bridge/foundry/mod.rs::handle_inbound`, after the existing `FoundryInbound::ItemDeleted` arm, add:

```rust
            FoundryInbound::Items { items } => {
                let canonical: Vec<_> = items.iter().map(translate::to_canonical_world_item).collect();
                Ok(vec![InboundEvent::WorldItemsSnapshot {
                    source: crate::bridge::types::SourceKind::Foundry,
                    items: canonical,
                }])
            }
            FoundryInbound::WorldItemUpsert { item } => {
                Ok(vec![InboundEvent::WorldItemUpsert {
                    source: crate::bridge::types::SourceKind::Foundry,
                    item: translate::to_canonical_world_item(&item),
                }])
            }
            FoundryInbound::WorldItemDeleted { item_id } => {
                Ok(vec![InboundEvent::WorldItemDeleted {
                    source: crate::bridge::types::SourceKind::Foundry,
                    item_id,
                }])
            }
```

- [x] **Step 2: Add three tests**

In the existing `#[cfg(test)] mod tests` block of `bridge/foundry/mod.rs`:

```rust
#[tokio::test]
async fn items_snapshot_produces_world_items_snapshot_event() {
    let source = FoundrySource;
    let msg = serde_json::json!({
        "type": "items",
        "items": [
            { "id": "i1", "name": "Iron Gullet", "type": "feature",
              "featuretype": "merit", "system": {} }
        ]
    });
    let events = source.handle_inbound(msg).await.expect("handles");
    assert_eq!(events.len(), 1);
    match &events[0] {
        InboundEvent::WorldItemsSnapshot { source: src, items } => {
            assert_eq!(*src, SourceKind::Foundry);
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, "Iron Gullet");
        }
        other => panic!("expected WorldItemsSnapshot, got {other:?}"),
    }
}

#[tokio::test]
async fn world_item_upsert_produces_event() {
    let source = FoundrySource;
    let msg = serde_json::json!({
        "type": "world_item_upsert",
        "item": { "id": "i7", "name": "Bloodhound", "type": "feature",
                  "featuretype": "merit", "system": {} }
    });
    let events = source.handle_inbound(msg).await.expect("handles");
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], InboundEvent::WorldItemUpsert { .. }));
}

#[tokio::test]
async fn world_item_deleted_produces_event() {
    let source = FoundrySource;
    let msg = serde_json::json!({ "type": "world_item_deleted", "item_id": "i99" });
    let events = source.handle_inbound(msg).await.expect("handles");
    assert_eq!(events.len(), 1);
    match &events[0] {
        InboundEvent::WorldItemDeleted { source: src, item_id } => {
            assert_eq!(*src, SourceKind::Foundry);
            assert_eq!(item_id, "i99");
        }
        other => panic!("expected WorldItemDeleted, got {other:?}"),
    }
}
```

- [x] **Step 3: Run `./scripts/verify.sh`**

Expected: cargo green. `bridge/mod.rs::accept_loop` may now warn about unhandled `InboundEvent::WorldItem*` variants in the match — Task 5 resolves. If the match is currently exhaustive, the build fails; add `WorldItem* => continue,` placeholders inline as TODOs that Task 5 replaces. **Recommended:** ship Tasks 1+3+4+5 as one implementer commit since they share the inbound-pipeline exhaustiveness invariant.

- [x] **Step 4: Verify state of dependent files**

Confirm `bridge/mod.rs::accept_loop` has been pre-prepared (Task 5) OR has placeholder arms. If neither, this task's commit will leave the working tree non-buildable. Coordinate via the joint Task 5 commit.

- [x] **Step 5: Joint commit (Tasks 1, 3, 4 + minimal Task 5 stub)**

```
git add src-tauri/src/bridge/foundry/types.rs \
        src-tauri/src/bridge/source.rs \
        src-tauri/src/bridge/foundry/translate.rs \
        src-tauri/src/bridge/foundry/mod.rs
git commit -m "$(cat <<'EOF'
Add inbound world-item wire variants + translation pipeline

Three new FoundryInbound variants (Items, WorldItemUpsert,
WorldItemDeleted) for the bridge.subscribe { collection: "item" }
protocol. Source-agnostic CanonicalWorldItem; Foundry translator;
handle_inbound dispatch + 6 round-trip tests.

bridge/mod.rs::accept_loop event arms ship in the next commit
(Task 5 stub adds placeholders pinned to a follow-up).

Refs #27.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

If Task 5 stubs are added as part of this commit, drop the trailing paragraph from the message and consolidate the message to cover Task 5 too.

---

## Task 5: Extend `BridgeState` with `world_items` cache + route arms in `accept_loop`

**Goal:** Persist world-item state in `BridgeState`; emit `bridge://foundry/items-updated` after each snapshot / upsert / delete; add `bridge_get_world_items` Tauri command for frontend initial-load.

**Files:**
- Modify: `src-tauri/src/bridge/mod.rs` (BridgeState struct + accept_loop event-routing match)
- Modify: `src-tauri/src/bridge/commands.rs` (new command `bridge_get_world_items`)

**Anti-scope:** Do NOT emit per-item events to the frontend — emit the full snapshot each time, mirroring `bridge://characters-updated`. The frontend reactive layer can derive deltas if needed; the bridge stays simple.

**Depends on:** Tasks 3, 4 (InboundEvent variants exist)

**Invariants cited:** ARCH §3 (BridgeState holds the merged cache); §4 (events deliver full snapshots).

**Tests required:** NO — covered by manual smoke (Task 16) plus cargo build (the new Tauri command's compile shape is the gate).

- [x] **Step 1: Add `world_items` to `BridgeState`**

In `src-tauri/src/bridge/mod.rs`, find the existing `BridgeState` struct. Add a `world_items` field:

```rust
pub world_items: tokio::sync::Mutex<
    std::collections::HashMap<
        crate::bridge::types::SourceKind,
        std::collections::HashMap<String, crate::bridge::types::CanonicalWorldItem>,
    >,
>,
```

Update `BridgeState::new()` (or the equivalent constructor) to initialize the new field with an empty HashMap.

- [x] **Step 2: Add three event arms to `accept_loop`**

In `src-tauri/src/bridge/mod.rs::accept_loop`, in the `match event` block (search for `InboundEvent::CharactersSnapshot`), add three new arms after `InboundEvent::ItemDeleted`:

```rust
InboundEvent::WorldItemsSnapshot { source, items } => {
    {
        let mut store = state.world_items.lock().await;
        let slot = store.entry(source).or_default();
        slot.clear();
        for i in &items {
            slot.insert(i.id.clone(), i.clone());
        }
    }
    let snapshot = collect_world_items_snapshot(&state).await;
    let _ = handle.emit("bridge://foundry/items-updated", snapshot);
}
InboundEvent::WorldItemUpsert { source, item } => {
    {
        let mut store = state.world_items.lock().await;
        store.entry(source).or_default().insert(item.id.clone(), item);
    }
    let snapshot = collect_world_items_snapshot(&state).await;
    let _ = handle.emit("bridge://foundry/items-updated", snapshot);
}
InboundEvent::WorldItemDeleted { source, item_id } => {
    {
        let mut store = state.world_items.lock().await;
        if let Some(slot) = store.get_mut(&source) {
            slot.remove(&item_id);
        }
    }
    let snapshot = collect_world_items_snapshot(&state).await;
    let _ = handle.emit("bridge://foundry/items-updated", snapshot);
}
```

Add the helper `collect_world_items_snapshot` near the bottom of `bridge/mod.rs`:

```rust
async fn collect_world_items_snapshot(
    state: &std::sync::Arc<BridgeState>,
) -> Vec<crate::bridge::types::CanonicalWorldItem> {
    let store = state.world_items.lock().await;
    store.values().flat_map(|m| m.values().cloned()).collect()
}
```

The emitted payload is a flat `Vec<CanonicalWorldItem>`; the frontend partitions by source if it needs to (the `source` field on each item carries it).

- [x] **Step 3: Add `bridge_get_world_items` Tauri command**

In `src-tauri/src/bridge/commands.rs`, after `bridge_get_rolls`:

```rust
/// Returns every world-level item known across every source. Used by
/// the frontend on initial load; live updates flow through the
/// `bridge://foundry/items-updated` event.
#[tauri::command]
pub async fn bridge_get_world_items(
    conn: State<'_, BridgeConn>,
) -> Result<Vec<CanonicalWorldItem>, String> {
    let store = conn.0.world_items.lock().await;
    Ok(store.values().flat_map(|m| m.values().cloned()).collect())
}
```

(Adjust the `CanonicalWorldItem` import at the top of the file.)

- [x] **Step 4: Register the new command**

In `src-tauri/src/lib.rs::invoke_handler!`, append `bridge::commands::bridge_get_world_items` to the command list (next to `bridge_get_rolls`).

- [x] **Step 5: Update `ARCHITECTURE.md` §4 (same-commit rule)**

CLAUDE.md mandates that any new `#[tauri::command]` lands in the same commit as its ARCH §4 entry. Edit `ARCHITECTURE.md` §4:
- Append `bridge_get_world_items` to the per-file IPC entry for `bridge/commands.rs`.
- Bump the running command total: **63 → 64**.
- Add `bridge://foundry/items-updated` to the events table (this task is where it starts being emitted).

- [x] **Step 6: Run `./scripts/verify.sh`**

Expected: green.

- [x] **Step 7: Commit**

```
git add src-tauri/src/bridge/mod.rs \
        src-tauri/src/bridge/commands.rs \
        src-tauri/src/lib.rs \
        ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
Route world-item events through BridgeState + emit
bridge://foundry/items-updated

Adds BridgeState.world_items cache and three accept_loop arms
(snapshot / upsert / delete) that emit a full snapshot to the
frontend on every change. New Tauri command bridge_get_world_items
for initial-load hydration. ARCH §4 IPC inventory bumped
(63 → 64) and events table updated in the same commit, per
CLAUDE.md same-commit rule.

Refs #27.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 6: New JS `foundry-actions/item.js` with `itemsSubscriber` + register `item` collection

**Goal:** The Foundry-side half of #27. New ES module exporting `itemsSubscriber.attach(socket) / .detach()` following the `actorsSubscriber` template. Hooks `createItem`/`updateItem`/`deleteItem` filtered to world-level items.

**Files:**
- Create: `vtmtools-bridge/scripts/foundry-actions/item.js`
- Modify: `vtmtools-bridge/scripts/foundry-actions/bridge.js` (register `item: itemsSubscriber` in the `subscribers` map)

**Anti-scope:** Do NOT report embedded actor items here (filter `if (item.parent !== null) return;` in every hook). Do NOT emit any wire variant beyond the three specified in Task 1.

**Depends on:** none (JS-only, parallel with Rust work)

**Invariants cited:** Helper-library roadmap §4 (per-umbrella files; handler-map dispatch). The existing `actorsSubscriber` is the canonical template.

**Tests required:** NO — JS side has no unit-test infrastructure in this repo. Manual smoke against a live Foundry world covers it (Task 16).

- [x] **Step 1: Create `item.js`**

Create `vtmtools-bridge/scripts/foundry-actions/item.js`:

```js
// Foundry item.* subscriber executor.
//
// World-level Item docs only — embedded-on-actor items continue to flow
// through actorsSubscriber's per-actor enrichment. The filter is
// `item.parent === null` on every Foundry Item Document hook event.
//
// Wire emission shapes (consumed by Rust FoundryInbound; see
// src-tauri/src/bridge/foundry/types.rs):
//   { type: "items",                  items: [...] }    // snapshot on attach
//   { type: "world_item_upsert",      item: {...} }     // create OR update
//   { type: "world_item_deleted",     item_id: "..." }  // delete

const MODULE_ID = "vtmtools-bridge";

let _attached = null; // { socket, hookHandles: [ids] }

function itemToWire(item) {
  return {
    id: item.id,
    name: item.name,
    type: item.type,
    featuretype: item.system?.featuretype ?? null,
    system: item.system ?? {},
  };
}

function isWorldLevel(item) {
  // Foundry parents: world items have parent === null; embedded items
  // have parent === <Actor>.
  return item.parent === null || item.parent === undefined;
}

export const itemsSubscriber = {
  attach(socket) {
    if (_attached) return;
    if (socket?.readyState === WebSocket.OPEN) {
      const items = game.items.contents
        .filter(isWorldLevel)
        .map(itemToWire);
      socket.send(JSON.stringify({ type: "items", items }));
      console.log(`[${MODULE_ID}] itemsSubscriber: pushed ${items.length} world items`);
    }

    const onCreate = Hooks.on("createItem", (item /*, options, userId */) => {
      if (!isWorldLevel(item)) return;
      socket.send(JSON.stringify({ type: "world_item_upsert", item: itemToWire(item) }));
    });
    const onUpdate = Hooks.on("updateItem", (item /*, changes, options, userId */) => {
      if (!isWorldLevel(item)) return;
      socket.send(JSON.stringify({ type: "world_item_upsert", item: itemToWire(item) }));
    });
    const onDelete = Hooks.on("deleteItem", (item /*, options, userId */) => {
      if (!isWorldLevel(item)) return;
      socket.send(JSON.stringify({ type: "world_item_deleted", item_id: item.id }));
    });

    _attached = { socket, hookHandles: { createItem: onCreate, updateItem: onUpdate, deleteItem: onDelete } };
  },

  detach() {
    if (!_attached) return;
    Hooks.off("createItem", _attached.hookHandles.createItem);
    Hooks.off("updateItem", _attached.hookHandles.updateItem);
    Hooks.off("deleteItem", _attached.hookHandles.deleteItem);
    _attached = null;
  },
};
```

- [x] **Step 2: Register `item` collection in `bridge.js` subscribers map**

In `vtmtools-bridge/scripts/foundry-actions/bridge.js`, add the import and registration:

```js
import { actorsSubscriber } from "./actor.js";
import { itemsSubscriber } from "./item.js";

const subscribers = {
  actors: actorsSubscriber,
  item: itemsSubscriber,
};
```

**Collection name:** `item` (singular) matches the spec sketch's `collection: "item"` envelope. The desktop will send `bridge.subscribe { collection: "item" }` to activate it.

- [ ] **Step 3: Manual smoke (deferred to Task 16)** _(this step is itself a deferral — covered by Task 16 Step 2/3)_

Smoke verification is part of Task 16's E2E gate. For this task's verification: confirm `npm run check` (frontend type-check has no opinion on the bridge module — JS module is loaded by Foundry runtime, not by SvelteKit).

- [x] **Step 4: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit, even for JS-only changes. The Rust/TS toolchain pieces of `verify.sh` will no-op on the JS file changes, but a green run confirms nothing else regressed.

- [x] **Step 5: Commit**

```
git add vtmtools-bridge/scripts/foundry-actions/item.js \
        vtmtools-bridge/scripts/foundry-actions/bridge.js
git commit -m "$(cat <<'EOF'
Add Foundry item.* subscriber (world-level items only)

New itemsSubscriber attaches to bridge.subscribe { collection: "item" }
and pushes initial snapshot + per-item upserts/deletes via createItem,
updateItem, deleteItem hooks. Filter is `item.parent === null` so
embedded-on-actor items continue to arrive only via actorsSubscriber's
enrichment.

Refs #27.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 7: Extend existing `foundry-actions/storyteller.js` with `createWorldItem` handler

**Goal:** Outbound side of #13. Add `createWorldItem` (handling `storyteller.create_world_item` envelopes by calling `Item.create({...})` at world level) into the **already-existing** `storyteller.js` placeholder.

**Reality check:** `vtmtools-bridge/scripts/foundry-actions/storyteller.js` already exists in the repo as a `// Reserved umbrella; no helpers in v1` stub with `export const handlers = {};`. It is already imported in `index.js` (`import { handlers as storytellerHandlers } from "./storyteller.js";`) and spread into the flattened handler map. Plan B's job is to **modify** the stub (add the handler), not create a new file or touch `index.js`.

**Files:**
- Modify: `vtmtools-bridge/scripts/foundry-actions/storyteller.js` (replace the stub with the `createWorldItem` handler)
- (NOT touched: `vtmtools-bridge/scripts/foundry-actions/index.js` — `storytellerHandlers` is already imported and spread.)

**Anti-scope:** Do NOT add other `storyteller.*` helpers in this milestone — the umbrella was reserved but Plan B activates it with exactly one handler. Do NOT touch embedded actor items here (those are actor.*). Do NOT edit `index.js` — the import line already exists.

**Depends on:** none (JS-only, parallel)

**Invariants cited:** Helper-library roadmap §4 (umbrella-per-file), §6 (naming conventions: wire `type` = `storyteller.create_world_item`; JS executor name = camelCase verb-noun `createWorldItem`).

**Tests required:** NO (manual smoke covers it in Task 16).

- [x] **Step 1: Replace the storyteller.js stub**

Replace the entire contents of `vtmtools-bridge/scripts/foundry-actions/storyteller.js` (currently a 2-line stub) with:

```js
// Foundry storyteller.* helper executors.
//
// World-level operations not tied to a single actor. The storyteller.* umbrella
// is reserved by name in foundry helper roadmap §5; v1 milestone-4
// ships exactly one helper: storyteller.create_world_item (used by Library Sync
// push button to create a Foundry-world-level Item doc that lives in
// the world's Items sidebar / compendium, not embedded on an actor).

const MODULE_ID = "vtmtools-bridge";

/**
 * Create a world-level Item document.
 * Wire shape (validated Rust-side; see build_create_world_item):
 *   { type: "storyteller.create_world_item", name, featuretype, description, points }
 * Effect: Item.create({ type: "feature", name,
 *                       system: { featuretype, description, points } })
 *         at world level (no parent actor).
 * Failure modes: Foundry permission errors (GM-only — should not occur
 *                given the bridge runs in a GM session); duplicate name
 *                is NOT rejected (Foundry allows duplicate-name items).
 * Idempotency: NOT idempotent. Re-running creates a duplicate row.
 *              Dedup is Plan C's concern (auto-version-suffix on pull).
 */
async function createWorldItem(msg) {
  try {
    await Item.create({
      type: "feature",
      name: msg.name,
      system: {
        featuretype: msg.featuretype,
        description: msg.description ?? "",
        points: typeof msg.points === "number" ? msg.points : 0,
      },
    });
  } catch (err) {
    console.error(`[${MODULE_ID}] storyteller.create_world_item failed:`, err);
    ui.notifications?.error(`vtmtools: could not create world item: ${err?.message ?? err}`);
    throw err;
  }
}

export const handlers = {
  "storyteller.create_world_item": createWorldItem,
};
```

- [x] **Step 2: Verify `index.js` does not need editing**

Confirm via `grep storytellerHandlers vtmtools-bridge/scripts/foundry-actions/index.js` that the import + spread already exist (they should — `storyteller.js` was previously a reserved-umbrella stub already wired in). If a fresh checkout somehow lacks the wiring, restore it; otherwise do nothing.

- [x] **Step 3: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit, even for JS-only changes.

- [x] **Step 4: Commit**

```
git add vtmtools-bridge/scripts/foundry-actions/storyteller.js
git commit -m "$(cat <<'EOF'
Activate storyteller.* umbrella: add storyteller.create_world_item

Replaces the reserved-umbrella stub (empty handlers map) in
foundry-actions/storyteller.js with the first helper:
storyteller.create_world_item creates a world-level feature Item doc
(merit / flaw / background / boon). Used by Plan B push to send a
local advantage row into the active Foundry world.

index.js already imports storytellerHandlers from the prior reserved
stub — no wiring change needed.

Not idempotent — dedup is Plan C's concern.

Refs #13.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 8: Bump `module.json` 0.5.0 → 0.6.0

**Goal:** Reflect the additive `item` subscription + `storyteller.*` umbrella in the module version.

**Files:**
- Modify: `vtmtools-bridge/module.json`

**Anti-scope:** Do NOT bump the protocol_version field (still 1 — additive features only).

**Depends on:** Tasks 6, 7

**Invariants cited:** Helper-library roadmap §7 (additive features = minor bump).

**Tests required:** NO

- [x] **Step 1: Bump version**

Change `module.json`'s `"version": "0.5.0"` to `"version": "0.6.0"`.

- [x] **Step 2: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit.

- [x] **Step 3: Commit**

```
git add vtmtools-bridge/module.json
git commit -m "$(cat <<'EOF'
vtmtools-bridge: 0.5.0 → 0.6.0 (item subscription + storyteller.* umbrella)

Additive features within protocol_version 1:
  • bridge.subscribe accepts collection: "item" (#27)
  • storyteller.create_world_item handler (#13)

Backward compat: old desktop ↔ new module works identically (desktop
never sends bridge.subscribe { item } unless on Plan B+).

Refs #13 #27.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 9: New Rust `bridge/foundry/actions/storyteller.rs` with `build_create_world_item`

**Goal:** Rust-side builder for the `storyteller.create_world_item` wire envelope. Validates featuretype against the same merit/flaw/background/boon enum that `actor.create_feature` uses.

**Reality check:** `src-tauri/src/bridge/foundry/actions/storyteller.rs` already exists as a one-line `// Reserved umbrella; no helpers in v1.` stub, and `pub mod storyteller;` is already declared in `actions/mod.rs`. Plan B's job is to **modify** the stub (add the builder + tests), not create a new file or touch `mod.rs`.

**Files:**
- Modify: `src-tauri/src/bridge/foundry/actions/storyteller.rs` (replace the comment stub with the `build_create_world_item` builder + tests)
- (NOT touched: `src-tauri/src/bridge/foundry/actions/mod.rs` — `pub mod storyteller;` already exists.)

**Anti-scope:** Do NOT duplicate the validation logic — extract a shared helper if `actor.rs::build_create_feature`'s validation is already factored out, OR copy the 4-line `match` if not (no abstraction wanted per the YAGNI override). Do NOT touch `actions/mod.rs` — the module declaration is already there.

**Depends on:** none (Rust-only, parallel with JS tasks)

**Invariants cited:** ARCH §7 (error prefix `foundry/storyteller.create_world_item:`). Helper-library roadmap §6 (builder naming convention).

**Tests required:** YES — 2 tests (happy path envelope shape + invalid featuretype rejection).

- [x] **Step 1: Replace the storyteller.rs stub**

Replace the entire contents of `src-tauri/src/bridge/foundry/actions/storyteller.rs` (currently a 1-line stub) with:

```rust
//! Foundry `storyteller.*` helper builders. World-level operations not tied
//! to a single actor.
//!
//! v1 milestone-4 ships exactly one helper: `storyteller.create_world_item` —
//! creates a Foundry-world-level Item doc (feature type) with the
//! given featuretype (merit / flaw / background / boon).

use serde_json::{json, Value};

/// Build a `storyteller.create_world_item { name, featuretype, description, points }`
/// envelope. Validates featuretype against the same enum
/// `actor.create_feature` uses.
pub fn build_create_world_item(
    name: &str,
    featuretype: &str,
    description: &str,
    points: i32,
) -> Result<Value, String> {
    match featuretype {
        "merit" | "flaw" | "background" | "boon" => {}
        other => {
            return Err(format!(
                "foundry/storyteller.create_world_item: invalid featuretype: {other}"
            ));
        }
    }
    Ok(json!({
        "type": "storyteller.create_world_item",
        "name": name,
        "featuretype": featuretype,
        "description": description,
        "points": points,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_world_item_envelope_shape() {
        let out = build_create_world_item("Iron Gullet", "merit", "rancid blood ok", 3)
            .expect("merit is a valid featuretype");
        assert_eq!(out["type"], "storyteller.create_world_item");
        assert_eq!(out["name"], "Iron Gullet");
        assert_eq!(out["featuretype"], "merit");
        assert_eq!(out["description"], "rancid blood ok");
        assert_eq!(out["points"], 3);
    }

    #[test]
    fn create_world_item_invalid_featuretype_returns_err() {
        let err = build_create_world_item("X", "discipline", "", 0)
            .expect_err("discipline is not a valid featuretype");
        assert!(
            err.starts_with("foundry/storyteller.create_world_item: invalid featuretype:"),
            "got: {err}"
        );
    }
}
```

- [x] **Step 2: Verify `actions/mod.rs` does not need editing**

Confirm via `grep 'pub mod storyteller' src-tauri/src/bridge/foundry/actions/mod.rs` that the module declaration already exists (it should — the reserved-umbrella stub was declared previously). If a fresh checkout somehow lacks it, restore it; otherwise do nothing.

- [x] **Step 3: Run `cargo test`**

Run: `cargo test --manifest-path src-tauri/Cargo.toml bridge::foundry::actions::storyteller`

Expected: 2 tests pass.

- [x] **Step 4: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit.

- [x] **Step 5: Commit**

```
git add src-tauri/src/bridge/foundry/actions/storyteller.rs
git commit -m "$(cat <<'EOF'
Activate storyteller.* Rust umbrella: add build_create_world_item

Replaces the reserved-umbrella stub in bridge/foundry/actions/storyteller.rs
with the first builder. Validates featuretype against
merit/flaw/background/boon. Mirrors the Foundry-side storyteller.js
executor; pairing enforced by integration test in Task 16's E2E smoke.

actions/mod.rs already declares `pub mod storyteller;` — no wiring
change needed.

Refs #13.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 10: New Rust `tools/library_push.rs` with `push_advantage_to_world`

**Goal:** Tauri command that loads an advantage row by id, maps `kind → featuretype` (lossless — they're 1:1), composes `storyteller.create_world_item`, and routes via `send_to_source_inner(SourceKind::Foundry)`.

**Files:**
- Create: `src-tauri/src/tools/library_push.rs`
- Modify: `src-tauri/src/tools/mod.rs` — add `pub mod library_push;`

**Anti-scope:** Do NOT push dyscrasias here. Do NOT iterate over multiple advantages — `push_advantage_to_world(id)` is single-row; bulk push (if ever wanted) is a frontend loop. Do NOT cache the SourceKind::Foundry connectivity state — `send_to_source_inner` already returns Ok if disconnected.

**Depends on:** Task 9 (builder); Plan A (Advantage struct has `kind`).

**Invariants cited:** ARCH §4 (Tauri IPC; typed wrappers required at frontend). ARCH §7 (error prefix `tools/library_push:`).

**Tests required:** YES — 1 happy-path test using an in-memory pool + a faked outbound channel via a temporary `BridgeState` that captures sent messages.

- [x] **Step 1: Create `library_push.rs`**

Create `src-tauri/src/tools/library_push.rs`:

```rust
//! Library push: send a local advantage row → active Foundry world as a
//! world-level Item doc. Composes
//! `bridge::foundry::actions::storyteller::build_create_world_item`. No-op if
//! Foundry isn't connected (silent success — matches bridge_set_attribute
//! semantics; the UI gates the button on connectivity).

use crate::bridge::foundry::actions::storyteller::build_create_world_item;
use crate::bridge::types::SourceKind;
use crate::bridge::{commands::send_to_source_inner, BridgeConn};
use crate::shared::types::AdvantageKind;
use tauri::State;

fn kind_to_featuretype(k: AdvantageKind) -> &'static str {
    match k {
        AdvantageKind::Merit      => "merit",
        AdvantageKind::Flaw       => "flaw",
        AdvantageKind::Background => "background",
        AdvantageKind::Boon       => "boon",
    }
}

fn extract_points(properties: &[crate::shared::types::Field]) -> i32 {
    // Look for `level` (single-value) or fall back to `min_level`.
    // Anything not found → 0. Mirrors the existing AdvantagesManager
    // dot-display fallback.
    for f in properties {
        if f.name == "level" || f.name == "min_level" {
            if let crate::shared::types::FieldValue::Number { value } = &f.value {
                match value {
                    crate::shared::types::NumberFieldValue::Single(n) => return *n as i32,
                    crate::shared::types::NumberFieldValue::Range(min, _) => return *min as i32,
                }
            }
        }
    }
    0
}

#[tauri::command]
pub async fn push_advantage_to_world(
    db: State<'_, crate::DbState>,
    conn: State<'_, BridgeConn>,
    id: i64,
) -> Result<(), String> {
    // Load the advantage row by id (read-only — uses listAdvantages
    // path; cheaper than a per-id query at v1 scale, and consistent
    // with the rest of the module).
    let all = crate::db::advantage::__internal_db_list(&db.0).await?;
    let row = all.iter().find(|r| r.id == id)
        .ok_or_else(|| format!("tools/library_push: advantage {id} not found"))?;

    let payload = build_create_world_item(
        &row.name,
        kind_to_featuretype(row.kind),
        &row.description,
        extract_points(&row.properties),
    )?;

    let text = serde_json::to_string(&payload)
        .map_err(|e| format!("tools/library_push: serialize: {e}"))?;
    send_to_source_inner(&conn.0, SourceKind::Foundry, text).await
}

#[cfg(test)]
mod tests {
    // Note: this test depends on db::advantage having a public-or-
    // pub(crate) db_list helper. If db_list is private, add a
    // pub(crate) re-export named __internal_db_list in db/advantage.rs.
    // Verify the helper visibility before adding the test.

    // Happy-path integration test goes here once db_list is exposed.
    // Defers to Task 16's E2E smoke if module-internal helper
    // visibility blocks a clean unit test.
}
```

**Implementation note on `__internal_db_list`:** the existing `db::advantage::db_list` is private to the module. Promote it to `pub(crate)` (rename optional — `db_list` is fine for the crate-internal name) so `tools/library_push` can read advantages without going through Tauri state from a Tauri command. Update `db/advantage.rs` to add `pub(crate)` to the function signature.

- [x] **Step 2: Promote `db_list` to `pub(crate)`**

In `src-tauri/src/db/advantage.rs`, change `async fn db_list(...)` to `pub(crate) async fn db_list(...)`. Use this from `library_push.rs` directly: `crate::db::advantage::db_list(&db.0).await`.

Drop the `__internal_db_list` rename in the snippet above; just use `db_list`.

- [x] **Step 3: Register the module**

In `src-tauri/src/tools/mod.rs`:

```rust
pub mod library_push;
// (alongside existing pub mod entries)
```

- [x] **Step 4: Add unit test**

In `library_push.rs::tests`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::types::{Advantage, AdvantageKind, Field, FieldValue, NumberFieldValue};

    #[test]
    fn extract_points_reads_level_field() {
        let props = vec![Field {
            name: "level".into(),
            value: FieldValue::Number { value: NumberFieldValue::Single(3.0) },
        }];
        assert_eq!(extract_points(&props), 3);
    }

    #[test]
    fn extract_points_reads_min_level_for_ranged() {
        let props = vec![Field {
            name: "min_level".into(),
            value: FieldValue::Number { value: NumberFieldValue::Single(1.0) },
        }];
        assert_eq!(extract_points(&props), 1);
    }

    #[test]
    fn extract_points_returns_zero_when_absent() {
        assert_eq!(extract_points(&[]), 0);
    }

    #[test]
    fn kind_maps_to_featuretype_one_to_one() {
        assert_eq!(kind_to_featuretype(AdvantageKind::Merit),      "merit");
        assert_eq!(kind_to_featuretype(AdvantageKind::Flaw),       "flaw");
        assert_eq!(kind_to_featuretype(AdvantageKind::Background), "background");
        assert_eq!(kind_to_featuretype(AdvantageKind::Boon),       "boon");
    }
}
```

Full integration (DB → builder → outbound channel) is covered by Task 16's manual E2E smoke. Unit tests cover the two pure helpers.

- [x] **Step 5: Run `cargo test`**

Run: `cargo test --manifest-path src-tauri/Cargo.toml tools::library_push`

Expected: 4 tests pass.

- [x] **Step 6: Update `ARCHITECTURE.md` §4 (same-commit rule)**

CLAUDE.md mandates that any new `#[tauri::command]` lands in the same commit as its ARCH §4 entry. Edit `ARCHITECTURE.md` §4:
- Add a new per-file entry for `tools/library_push.rs` listing `push_advantage_to_world`.
- Bump the running command total: **64 → 65**.

- [x] **Step 7: Run `./scripts/verify.sh`**

Expected: green.

- [x] **Step 8: Commit**

```
git add src-tauri/src/tools/library_push.rs \
        src-tauri/src/tools/mod.rs \
        src-tauri/src/db/advantage.rs \
        ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
Add push_advantage_to_world Tauri command

Loads local advantage row by id; maps Plan-A's kind → featuretype 1:1;
composes storyteller.create_world_item via the Plan-B builder; routes through the
existing send_to_source_inner outbound path. No-op when Foundry is
disconnected (matches bridge_set_attribute semantics).

Promotes db::advantage::db_list to pub(crate) so library_push can
read without going through a fresh Tauri state lookup. ARCH §4 IPC
inventory bumped (64 → 65) in the same commit, per CLAUDE.md
same-commit rule.

Refs #13.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 11: Register `push_advantage_to_world` in `lib.rs`

**Goal:** Wire the new command into Tauri's `invoke_handler!` so the frontend can call it.

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Anti-scope:** Do NOT touch the `bridge_get_world_items` line (Task 5 already added that). Do NOT register Plan C's `import_advantages_from_world` (that's Plan C's territory).

**Depends on:** Tasks 5, 10

**Invariants cited:** ARCH §9 ("Add a Tauri command" seam).

**Tests required:** NO

- [x] **Step 1: Add to `invoke_handler!`**

In `src-tauri/src/lib.rs`, in the `generate_handler![...]` list, add:

```rust
tools::library_push::push_advantage_to_world,
```

near the other `tools::` entries (e.g., next to `tools::foundry_chat::*`).

- [x] **Step 2: Run `./scripts/verify.sh`**

Expected: green.

- [x] **Step 3: Commit**

```
git add src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
Register push_advantage_to_world in invoke_handler

No new command surface in this commit — declaration + ARCH §4 entry
already landed in Task 10 (and Task 5 for bridge_get_world_items).
This commit only wires the existing declaration into the frontend
via `generate_handler!`.

Refs #13.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 12: Frontend typed wrapper `src/lib/library/api.ts`

**Goal:** Establish the `library` namespace on the frontend with the single Plan-B wrapper. Plan C extends this file with `importAdvantagesFromWorld`.

**Files:**
- Create: `src/lib/library/api.ts`

**Anti-scope:** Do NOT add any other wrapper (Plan C territory). Do NOT call `invoke` from components.

**Depends on:** Task 11

**Invariants cited:** ARCH §4 (typed wrappers per command; components never call `invoke` directly).

**Tests required:** NO

- [x] **Step 1: Create `src/lib/library/api.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';

/**
 * Push a local advantage row → active Foundry world as a world-level
 * feature Item doc. No-op if Foundry isn't connected (silent success
 * at the IPC layer; the UI gates the button on bridge connectivity).
 *
 * Resolves on success; rejects with a `"tools/library_push: ..."`
 * string if the advantage id is unknown or the wire envelope fails to
 * serialize. Bridge-disconnected case is NOT an error.
 */
export function pushAdvantageToWorld(id: number): Promise<void> {
  return invoke<void>('push_advantage_to_world', { id });
}
```

- [x] **Step 2: Run `npm run check`**

Expected: green.

- [x] **Step 3: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit. `npm run check` alone is not the gate — `verify.sh` also runs `cargo check`, `cargo test`, and the frontend build.

- [x] **Step 4: Commit**

```
git add src/lib/library/api.ts
git commit -m "$(cat <<'EOF'
Add src/lib/library/api.ts typed wrapper

Namespace established with pushAdvantageToWorld (Plan B). Plan C
extends this file with importAdvantagesFromWorld.

Refs #13.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 13: `src/store/bridge.svelte.ts` reactive `worldItems` state

**Goal:** Hydrate world-items on mount via `bridge_get_world_items`; subscribe to `bridge://foundry/items-updated` for live updates. Mirrors the existing characters-updated pattern.

**Files:**
- Modify: `src/store/bridge.svelte.ts`

**Anti-scope:** Do NOT add per-source partitioning logic in the store — keep a flat `worldItems: CanonicalWorldItem[]` (each item carries its `source` field; consumers filter as needed). Do NOT auto-subscribe to `item` collection here — the desktop sends `bridge.subscribe` only when a consumer needs it (Plan C's "Pull from world" trigger), so subscription registration is Plan C's concern.

**Depends on:** Tasks 2, 5

**Invariants cited:** ARCH §3 (ephemeral state in Svelte runes stores); ARCH §4 (Tauri events table).

**Tests required:** NO

- [x] **Step 1: Read current `bridge.svelte.ts` structure**

Locate the existing characters-updated subscription. The pattern: on mount, call `invoke('bridge_get_characters')`; subscribe to `bridge://characters-updated` and replace `characters` state on each emit.

- [x] **Step 2: Add `worldItems` state and subscription**

Add `let worldItems: CanonicalWorldItem[] = $state([])` and an `unlisten` for `bridge://foundry/items-updated`. Hydrate on init via a wrapper call. Use the same lifecycle pattern as `characters` — the goal is a drop-in mirror.

If the store has a single `init()` that registers all subscriptions, append the items-updated listener there. If it's per-piece-of-state, copy the characters pattern verbatim with `worldItems` substituted.

- [x] **Step 3: Add a typed wrapper for `bridge_get_world_items`**

In `src/lib/bridge/api.ts` (or wherever existing `bridge_*` wrappers live):

```ts
export function bridgeGetWorldItems(): Promise<CanonicalWorldItem[]> {
  return invoke<CanonicalWorldItem[]>('bridge_get_world_items');
}
```

- [x] **Step 4: Run `npm run check` and `npm run build`**

Expected: green.

- [x] **Step 5: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit. The npm steps above are a tighter inner loop; `verify.sh` is the full gate.

- [x] **Step 6: Commit**

```
git add src/store/bridge.svelte.ts src/lib/bridge/api.ts
git commit -m "$(cat <<'EOF'
bridge store: hydrate + subscribe to world items

Mirrors the characters-updated pattern. Hydrate via
bridge_get_world_items on mount; subscribe to
bridge://foundry/items-updated for live deltas. Plan C consumes
this store for the import-from-world flow.

Refs #27.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 14: AdvantagesManager "Push to world" button

**Goal:** Per-row button visible only when Foundry is connected; on click calls `pushAdvantageToWorld(id)`; success → toast; error → error toast.

**Files:**
- Modify: `src/tools/AdvantagesManager.svelte` (or `src/lib/components/AdvantageCard.svelte` if the per-row action area lives there)

**Anti-scope:** Do NOT add a "Push all" bulk button — single-row only. Do NOT pre-check duplicate name on the Foundry side — push is allowed even if a same-name item exists (Foundry allows duplicates; dedup is Plan C's concern for the **import** direction).

**Depends on:** Tasks 12, 13

**Invariants cited:** ARCH §6 (CSS tokens only; no hardcoded hex).

**Tests required:** NO (manual smoke covers it).

- [x] **Step 1: Find the per-row action surface**

Locate where each row's actions render (probably in `AdvantageCard.svelte`'s footer). The existing "Edit" and "Delete" buttons live there.

- [x] **Step 2: Add "Push to world" button**

Add a button next to Edit/Delete:

```svelte
<script lang="ts">
  import { pushAdvantageToWorld } from '$lib/library/api';
  import { bridgeStore } from '$lib/../store/bridge.svelte';

  let pushing = $state(false);
  let pushError = $state('');
</script>

{#if bridgeStore.status.foundry}
  <button class="row-action" disabled={pushing}
    onclick={async () => {
      pushing = true;
      pushError = '';
      try {
        await pushAdvantageToWorld(adv.id);
        // Optional: toast "Pushed '<name>' to <world>"
      } catch (e) {
        pushError = String(e);
      } finally {
        pushing = false;
      }
    }}>
    {pushing ? '…' : '⇡ Push to world'}
  </button>
  {#if pushError}
    <span class="error-inline">{pushError}</span>
  {/if}
{/if}
```

The `bridgeStore.status.foundry` predicate gates visibility — disconnected sessions don't render the button at all. (Adapt the store accessor name to match whatever the existing pattern is in `bridge.svelte.ts`.)

CSS — reuse the existing row-action button styling. No new tokens.

- [x] **Step 3: Run `npm run check` and `npm run build`**

Expected: green.

- [ ] **Step 4: Manual smoke** _(deferred to user — requires interactive `npm run tauri dev` + live Foundry world)_

`npm run tauri dev` with a Foundry world running and the bridge connected:

- ✅ Push button visible on each row.
- ✅ Click "Push to world" on a corebook merit → Foundry world's Items sidebar shows a new "Iron Gullet" feature item with featuretype = merit.
- ✅ Click again → second duplicate appears (push is non-idempotent; dedup is Plan C territory).
- ✅ Disconnect Foundry → push buttons disappear (or grey out, depending on the reactive guard).
- ✅ Push a custom Boon → world Item appears with featuretype = boon.

- [x] **Step 5: Run `./scripts/verify.sh`**

Required by CLAUDE.md before every commit.

- [x] **Step 6: Commit**

```
git add src/tools/AdvantagesManager.svelte \
        src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
AdvantagesManager: add per-row "Push to world" button

Visible only when Foundry is connected (gated on bridgeStore.status.
foundry). Composes pushAdvantageToWorld(id); error toasted inline.
Non-idempotent — duplicate pushes create duplicate world items.

Closes #13.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 15: ARCHITECTURE.md §4 — Bridge WebSocket protocol updates

**Goal:** Document the new `item` subscription collection + `storyteller.*` umbrella in §4 Bridge WebSocket protocol. The IPC inventory bumps and events-table entry were already landed in-commit by Tasks 5 and 10 (per CLAUDE.md same-commit rule); only the protocol-section prose remains.

**Files:**
- Modify: `ARCHITECTURE.md`

**Anti-scope:** Do NOT re-edit the IPC inventory (already updated). Do NOT re-add `bridge://foundry/items-updated` to the events table (already added in Task 5). Do NOT introduce new sections — append to the existing protocol section.

**Depends on:** Task 14

**Invariants cited:** ARCH §4 (Bridge WebSocket protocol section). CLAUDE.md same-commit rule is satisfied for the IPC parts by Tasks 5 + 10; this task carries only the non-command-tied prose.

**Tests required:** NO

- [x] **Step 1: Update Bridge WebSocket protocol section**

In §4 Bridge WebSocket protocol, after the existing message-framing paragraph, add a paragraph on the `item` subscription and `storyteller.*` umbrella:

> **Subscription collections (Foundry):** `actors` (auto-subscribed on Hello — always-on; preserves pre-Plan-0 behavior), `item` (opt-in via `bridge.subscribe { collection: "item" }` — Plan B+ Library Sync consumers). The subscription registry lives in `vtmtools-bridge/scripts/foundry-actions/bridge.js`; future collections (`journal`, `scene`, `chat`, `combat`) are reserved by name in the character-tooling roadmap §5 and activated when a consumer feature lands.
>
> **Outbound umbrellas (Foundry):** `actor.*` (per-actor edits), `game.*` (in-game/table rolls + chat), `storyteller.*` (GM-facing operations not tied to a single actor; v1 ships `storyteller.create_world_item` only — Library Sync push). See `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md` §5 for the per-helper inventory.

- [x] **Step 2: Run `./scripts/verify.sh`**

Expected: green.

- [x] **Step 3: Commit**

```
git add ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
ARCHITECTURE.md §4: document `item` subscription + storyteller.* umbrella

Adds prose to the Bridge WebSocket protocol section describing the
new `item` subscription collection (opt-in for Library Sync) and the
new `storyteller.*` outbound umbrella (v1 ships create_world_item
only). The IPC inventory entries + total bump (63 → 65) and the
`bridge://foundry/items-updated` events-table row landed in their
declaring commits (Tasks 5 and 10) per the CLAUDE.md same-commit
rule, so this commit carries only the protocol-section prose.

Refs #13 #27.

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

---

## Task 16: Final verification gate (incl. live-Foundry E2E smoke)

**Goal:** Confirm Plan B end-to-end against a live Foundry world before handing off to Plan C.

- [x] **Step 1: Run `./scripts/verify.sh`**

Expected: full green.

- [ ] **Step 2: Live-Foundry smoke — push side (#13)** _(deferred to user — requires interactive `npm run tauri dev` + live Foundry world with vtmtools-bridge@0.6.0)_

Boot the Tauri app + a Foundry world with `vtmtools-bridge@0.6.0` installed.

- ✅ Bridge connects (green pip).
- ✅ Open Advantages tool → "Push to world" buttons appear on every row.
- ✅ Click push on "Iron Gullet" merit → Foundry world's Items sidebar shows a new "Iron Gullet" entry with type=Feature and (in the WoD5e sheet view) Feature Type=Merit.
- ✅ Click push on "Prey Exclusion" flaw → same path, featuretype=Flaw.
- ✅ Push a custom row with kind=Boon → appears as featuretype=Boon.

- [ ] **Step 3: Live-Foundry smoke — subscription side (#27)** _(deferred to user — naturally covered by Plan C's importer flow if manual triggering proves unreachable)_

(Without Plan C's import UI, the desktop never sends `bridge.subscribe { item }` automatically. Test by manually triggering subscription from the JS console of the Foundry browser session OR by waiting for Plan C.)

Manual triggering option — paste into the Foundry browser console with the bridge connected:

```js
// Force the desktop to subscribe to item collection (test-only).
// Plan C's importer will trigger this automatically.
const ws = game.modules.get("vtmtools-bridge")?._socket;
// (or whatever accessor — this depends on how bridge.js exposes the socket;
//  may need to inspect bridge.js to see if the socket is reachable from
//  the console for testing purposes. If not, defer this smoke to Plan C.)
```

If manual triggering proves unreachable, mark this smoke as **DEFER TO PLAN C** in the verification log — Plan C's importer flow tests the subscription end-to-end naturally.

- [x] **Step 4: Update plan checkboxes**

Mark every Task 1–15 step as `[x]` in this file. Commit:

```
git add docs/superpowers/plans/2026-05-14-library-sync-plan-b-push-and-item-subscription.md
git commit -m "$(cat <<'EOF'
Mark Plan B tasks complete

Plan B (push + item subscription) shipped. Closes #13 #27.
Unblocks Plan C (pull + attribution + dedup).

https://claude.ai/code/session_01MoVudfzVJh7PSkrS1zR5Px
EOF
)"
```

Plan B is complete. Plan C may now dispatch.
