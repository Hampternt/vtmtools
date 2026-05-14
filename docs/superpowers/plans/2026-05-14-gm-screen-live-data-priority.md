# GM Screen Live-Data Priority Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Auto-delete advantage-bound `CharacterModifier` rows when their backing Foundry item is deleted, and stop rendering item-level orphans on the GM screen.

**Architecture:** New explicit `item_deleted` wire shape from the Foundry bridge JS (riding on the just-shipped `deleteItem` hook) flows through a new `InboundEvent::ItemDeleted` variant. The bridge dispatcher in `src-tauri/src/bridge/mod.rs` reaches the DbState via `handle.state::<DbState>()` (Tauri managed state) and calls a new `db_delete_by_advantage_binding`. Deleted ids are emitted via `modifiers://rows-reaped`; the modifier store drops the rows locally. Independently, `CharacterRow.svelte` is simplified to filter advantage-bound modifiers by live `items[]` presence; the `isStale` field is removed.

**Tech Stack:** Rust (sqlx, tokio, serde), TypeScript (Svelte 5 runes), Tauri 2 event bus, JavaScript (Foundry Hooks API).

**Spec:** `docs/superpowers/specs/2026-05-13-gm-screen-live-data-priority-design.md`

**Branch:** `feat/gm-screen-live-data-priority` (created off post-merge master, item-hooks branch already shipped as PR #29)

---

## Resolved deferrals from the spec

### §5.3 Pool injection — DECIDED

**Choice:** Option A — extend `InboundEvent` enum; do the DB delete in the caller in `src-tauri/src/bridge/mod.rs:269-292` using `handle.state::<crate::DbState>()`.

**Why:** smallest surgery. The trait `BridgeSource::handle_inbound` stays stateless (no pool argument added to the interface, no impact on `Roll20Source`). The dispatcher in `bridge/mod.rs` already has `AppHandle` and uses `handle.emit`; getting `DbState` from the handle is one line. Matches the same control-flow shape used for `InboundEvent::RollReceived` at line 284 (caller-side state mutation + event emit).

### §6.3 Dead-code grep — CORRECTED SCOPE

The spec lumped together two unrelated "orphan" concepts. The grep clarifies:

| Symbol | Location | Concern | Action |
|---|---|---|---|
| `isStale` | `CharacterRow.svelte:132,142,156,158,271,427` | **Item-level orphan** — advantage-bound modifier whose item is gone | REMOVE (this feature) |
| `isStale` | `ModifierCard.svelte:14,62,105` | Same item-level orphan rendering | REMOVE (this feature) |
| `_showOrphans` / `showOrphans` | `modifiers.svelte.ts:34,68-69` | **Character-level orphan** — modifier rows whose entire character is gone from live AND saved | KEEP (out of scope) |
| `orphans` derivation | `GmScreen.svelte:39-49` | Same character-level orphan | KEEP (out of scope) |
| Show-orphans toggle UI | `GmScreen.svelte:78-84` | Same character-level orphan | KEEP (out of scope) |
| Orphans section UI | `GmScreen.svelte:130-142` | Same character-level orphan | KEEP (out of scope) |

The "Show orphans" toggle remains useful: when a player leaves the world or a character is deleted entirely (the `deleteActor` hook case, separate from `deleteItem`), the GM still wants a way to find and clean up orphaned modifier rows. That cleanup mechanism is independent of the item-level orphan fix.

---

## File map

| File | Role | Action |
|---|---|---|
| `src-tauri/src/db/modifier.rs` | DB-layer functions for `character_modifiers` table | Add `db_delete_by_advantage_binding` + 4 tests |
| `src-tauri/src/bridge/foundry/types.rs` | Foundry inbound wire types | Add `ItemDeleted` variant to `FoundryInbound` + deserialize test |
| `src-tauri/src/bridge/foundry/mod.rs` | Foundry source impl of `BridgeSource` trait | Add `ItemDeleted` arm to `handle_inbound` + dispatch test |
| `src-tauri/src/bridge/source.rs` | `InboundEvent` enum + `BridgeSource` trait | Add `ItemDeleted { source, source_id, item_id }` variant |
| `src-tauri/src/bridge/mod.rs` | WS dispatcher (caller of `handle_inbound`) | Add match arm for `InboundEvent::ItemDeleted`: DB delete + frontend emit |
| `vtmtools-bridge/scripts/translate.js` | Foundry-side hook subscriber | Extend `deleteItem` branch in `hookItemChanges` to also send `item_deleted` wire |
| `src/lib/components/gm-screen/CharacterRow.svelte` | Per-character card list derivation | Drop `isStale` field; collapse Loop 2 to free-floating only |
| `src/lib/components/gm-screen/ModifierCard.svelte` | Single card rendering | Drop `isStale` prop and its display |
| `src/store/modifiers.svelte.ts` | Modifier runes store | Subscribe to `modifiers://rows-reaped`; call `dropRow` |

---

## Task 1 — DB delete function with tests (TDD)

**Files:**
- Modify: `src-tauri/src/db/modifier.rs` — add function below `db_materialize_advantage` (~line 357); add tests in the existing `mod tests` block at the bottom

**Tests required (per spec §9 "genuine logic"):** yes.

- [ ] **Step 1.1: Write the four failing tests**

Append to the `mod tests { ... }` block at the bottom of `src-tauri/src/db/modifier.rs` (after the existing last test, just before the closing brace of `mod tests`):

```rust
    #[tokio::test]
    async fn delete_by_advantage_binding_returns_ids_for_matches() {
        let pool = fresh_pool().await;
        // Two advantage-bound rows on the same character pointing at item "merit-1".
        sqlx::query(
            r#"INSERT INTO character_modifiers
               (source, source_id, name, binding_json)
               VALUES ('foundry', 'actor-a', 'M1',
                       '{"kind":"advantage","item_id":"merit-1"}'),
                      ('foundry', 'actor-a', 'M2',
                       '{"kind":"advantage","item_id":"merit-1"}')"#,
        )
        .execute(&pool).await.unwrap();

        let ids = db_delete_by_advantage_binding(
            &pool, &SourceKind::Foundry, "actor-a", "merit-1",
        ).await.unwrap();

        assert_eq!(ids.len(), 2, "both matching rows deleted");
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_modifiers"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(remaining, 0);
    }

    #[tokio::test]
    async fn delete_by_advantage_binding_idempotent_when_no_match() {
        let pool = fresh_pool().await;
        let ids = db_delete_by_advantage_binding(
            &pool, &SourceKind::Foundry, "actor-a", "merit-1",
        ).await.unwrap();
        assert!(ids.is_empty());
    }

    #[tokio::test]
    async fn delete_by_advantage_binding_does_not_delete_free_modifiers() {
        let pool = fresh_pool().await;
        sqlx::query(
            r#"INSERT INTO character_modifiers
               (source, source_id, name, binding_json)
               VALUES ('foundry', 'actor-a', 'FreeMod', '{"kind":"free"}')"#,
        )
        .execute(&pool).await.unwrap();

        // Even calling with a junk item_id on the same actor: free rows untouched.
        let ids = db_delete_by_advantage_binding(
            &pool, &SourceKind::Foundry, "actor-a", "merit-1",
        ).await.unwrap();
        assert!(ids.is_empty());

        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_modifiers"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(remaining, 1, "free modifier still present");
    }

    #[tokio::test]
    async fn delete_by_advantage_binding_scoped_to_source_and_source_id() {
        let pool = fresh_pool().await;
        sqlx::query(
            r#"INSERT INTO character_modifiers
               (source, source_id, name, binding_json)
               VALUES ('foundry', 'actor-a', 'OnA',
                       '{"kind":"advantage","item_id":"merit-1"}'),
                      ('foundry', 'actor-b', 'OnB',
                       '{"kind":"advantage","item_id":"merit-1"}'),
                      ('roll20',  'actor-a', 'R20',
                       '{"kind":"advantage","item_id":"merit-1"}')"#,
        )
        .execute(&pool).await.unwrap();

        let ids = db_delete_by_advantage_binding(
            &pool, &SourceKind::Foundry, "actor-a", "merit-1",
        ).await.unwrap();
        assert_eq!(ids.len(), 1, "only the (foundry, actor-a) row");

        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_modifiers"
        ).fetch_one(&pool).await.unwrap();
        assert_eq!(remaining, 2, "actor-b and roll20 rows untouched");
    }
```

- [ ] **Step 1.2: Run tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib db::modifier::tests::delete_by_advantage_binding`

Expected: 4 tests fail with `error[E0425]: cannot find function 'db_delete_by_advantage_binding'`.

- [ ] **Step 1.3: Implement the function**

Insert the following function in `src-tauri/src/db/modifier.rs` immediately after the closing brace of `db_materialize_advantage` (currently around line 357), and BEFORE the `#[tauri::command] pub async fn materialize_advantage_modifier` block:

```rust
/// Delete all advantage-bound modifier rows for `(source, source_id)` whose
/// binding `item_id` matches. Returns the deleted row ids. Idempotent —
/// no matches returns an empty Vec. Gated on `binding_json.kind = 'advantage'`
/// so free-floating modifiers are never affected.
///
/// Triggered by the Foundry `item_deleted` wire shape from
/// `vtmtools-bridge/scripts/translate.js` when a player or GM deletes a merit
/// on the Foundry sheet — see spec §3.2.
pub(crate) async fn db_delete_by_advantage_binding(
    pool: &SqlitePool,
    source: &SourceKind,
    source_id: &str,
    item_id: &str,
) -> Result<Vec<i64>, String> {
    let rows = sqlx::query(
        "DELETE FROM character_modifiers
         WHERE source = ? AND source_id = ?
           AND json_extract(binding_json, '$.kind') = 'advantage'
           AND json_extract(binding_json, '$.item_id') = ?
         RETURNING id"
    )
    .bind(source_to_str(source))
    .bind(source_id)
    .bind(item_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("db/modifier.delete_by_advantage_binding: {e}"))?;

    Ok(rows.iter().map(|r| r.get::<i64, _>("id")).collect())
}
```

- [ ] **Step 1.4: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib db::modifier::tests::delete_by_advantage_binding`

Expected: 4 tests pass.

- [ ] **Step 1.5: Run aggregate verify gate**

Run: `./scripts/verify.sh`

Expected: `verify: all checks passed`.

- [ ] **Step 1.6: Commit**

```bash
git add src-tauri/src/db/modifier.rs
git commit -m "$(cat <<'EOF'
feat(db): db_delete_by_advantage_binding for live-deletion cleanup

Add the DB primitive that removes advantage-bound character_modifiers rows
matching a (source, source_id, item_id) triple. Idempotent; gated on
binding_kind='advantage' so free-floating modifiers are never deleted.

Triggered downstream by the new item_deleted wire shape from the Foundry
bridge — see spec §3.2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2 — FoundryInbound::ItemDeleted variant with deserialize test

**Files:**
- Modify: `src-tauri/src/bridge/foundry/types.rs:11-32` — add variant; tests at `:182-223`

**Tests required:** yes (small deserialize round-trip — matches existing pattern in this file).

- [ ] **Step 2.1: Write the failing test**

Append to the `mod tests { ... }` block in `src-tauri/src/bridge/foundry/types.rs` (after the existing legacy-actor test, before the closing brace of `mod tests`):

```rust
    #[test]
    fn foundry_inbound_deserializes_item_deleted() {
        let wire = r#"{"type":"item_deleted","actor_id":"actor-a","item_id":"merit-1"}"#;
        let parsed: FoundryInbound = serde_json::from_str(wire).expect("parses");
        match parsed {
            FoundryInbound::ItemDeleted { actor_id, item_id } => {
                assert_eq!(actor_id, "actor-a");
                assert_eq!(item_id, "merit-1");
            }
            _ => panic!("expected ItemDeleted, got {parsed:?}"),
        }
    }
```

- [ ] **Step 2.2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib bridge::foundry::types::tests::foundry_inbound_deserializes_item_deleted`

Expected: fails with `no variant named 'ItemDeleted'`.

- [ ] **Step 2.3: Add the variant**

Edit `src-tauri/src/bridge/foundry/types.rs` — locate the `pub enum FoundryInbound` block (lines 11-32). Add a new variant immediately after `RollResult` and BEFORE the closing brace of the enum:

```rust
    /// A Foundry item was deleted from an actor. Triggered by the
    /// `deleteItem` hook in `vtmtools-bridge/scripts/translate.js`.
    /// `actor_id` is the Foundry actor `_id` (canonical `source_id`);
    /// `item_id` is the deleted item's `_id`.
    ItemDeleted {
        actor_id: String,
        item_id: String,
    },
```

- [ ] **Step 2.4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib bridge::foundry::types::tests::foundry_inbound_deserializes_item_deleted`

Expected: passes.

- [ ] **Step 2.5: Run aggregate verify gate**

Run: `./scripts/verify.sh`

Note: this will produce a new `non_exhaustive_patterns` compile error from `bridge::foundry::mod` (since the enum got a new variant). That error is the bridge between Task 2 and Task 3. Verify with:

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "error|non-exhaustive"
```

If the error is exactly the missing `ItemDeleted` arm in `bridge::foundry::mod::handle_inbound`, that's expected and gets resolved in Task 3. **Skip the commit step for Task 2 — bundle this commit with Task 3** (atomic commit per the project rule about not landing intermediate broken states).

---

## Task 3 — InboundEvent::ItemDeleted + FoundrySource arm + dispatcher arm (atomic)

**Files:**
- Modify: `src-tauri/src/bridge/source.rs:9-16` — add InboundEvent variant
- Modify: `src-tauri/src/bridge/foundry/mod.rs:20-38` — add ItemDeleted handler arm
- Modify: `src-tauri/src/bridge/mod.rs:269-292` — add InboundEvent::ItemDeleted dispatcher arm

**Tests required:** one focused unit test on the FoundrySource handler arm (the dispatcher arm in `bridge/mod.rs` is integration-level — manual smoke is its gate per project workflow override).

- [ ] **Step 3.1: Add the InboundEvent variant**

Edit `src-tauri/src/bridge/source.rs`. Locate the `pub enum InboundEvent` block (lines 8-16). Add a new variant after `RollReceived`:

```rust
    /// Foundry-side item deletion — frontend modifier rows tied to this
    /// item must be reaped. Caller in `bridge::mod` runs the DB delete and
    /// emits `modifiers://rows-reaped`. Spec §5.2.
    ItemDeleted {
        source: crate::bridge::types::SourceKind,
        source_id: String,
        item_id: String,
    },
```

- [ ] **Step 3.2: Write the failing handler test**

Append to `src-tauri/src/bridge/foundry/mod.rs`. There is no `#[cfg(test)] mod tests` block in this file currently — add one at the very bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::source::InboundEvent;
    use crate::bridge::types::SourceKind;
    use serde_json::json;

    #[tokio::test]
    async fn item_deleted_inbound_produces_modifier_reap_event() {
        let source = FoundrySource;
        let msg = json!({
            "type": "item_deleted",
            "actor_id": "actor-a",
            "item_id": "merit-1",
        });
        let events = source.handle_inbound(msg).await.expect("handles");
        assert_eq!(events.len(), 1);
        match &events[0] {
            InboundEvent::ItemDeleted { source, source_id, item_id } => {
                assert_eq!(*source, SourceKind::Foundry);
                assert_eq!(source_id, "actor-a");
                assert_eq!(item_id, "merit-1");
            }
            other => panic!("expected ItemDeleted event, got {other:?}"),
        }
    }
}
```

- [ ] **Step 3.3: Run the test — expect compile failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib bridge::foundry::tests::item_deleted_inbound_produces_modifier_reap_event`

Expected: fails to compile because `FoundryInbound::ItemDeleted` isn't handled in `handle_inbound`'s match block.

- [ ] **Step 3.4: Implement the FoundrySource handler arm**

Edit `src-tauri/src/bridge/foundry/mod.rs:20-38`. The current match block (lines 22-35) handles `Actors`, `ActorUpdate`, `Hello`, `Error`, `RollResult`. Add a new arm for `ItemDeleted` immediately after `RollResult`:

```rust
            FoundryInbound::ItemDeleted { actor_id, item_id } => {
                return Ok(vec![InboundEvent::ItemDeleted {
                    source: crate::bridge::types::SourceKind::Foundry,
                    source_id: actor_id,
                    item_id,
                }]);
            }
```

- [ ] **Step 3.5: Run the test — expect a different compile failure**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib bridge::foundry::tests::item_deleted_inbound_produces_modifier_reap_event`

Expected: now fails in `src-tauri/src/bridge/mod.rs:272-289` — the `match event` block is non-exhaustive (missing `InboundEvent::ItemDeleted` arm).

- [ ] **Step 3.6: Implement the dispatcher arm**

Edit `src-tauri/src/bridge/mod.rs:271-289`. Inside the `for event in events` loop, after the `InboundEvent::RollReceived` arm, add:

```rust
                        InboundEvent::ItemDeleted { source, source_id, item_id } => {
                            // Lookup the pool via Tauri's managed-state map.
                            // DbState is registered in lib.rs::run setup,
                            // before bridge servers are spawned, so this
                            // never fails in practice. If it ever did
                            // (no pool managed), skip the reap silently —
                            // a stale orphan card is preferable to a panic.
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
                                Ok(_) => {} // idempotent — no rows matched, nothing to emit
                                Err(e) => {
                                    eprintln!(
                                        "[bridge:{}] ItemDeleted reap failed: {e}",
                                        kind.as_str()
                                    );
                                }
                            }
                        }
```

You will need to add `use std::sync::Arc;` if not already present at the top of `bridge/mod.rs` — check the existing imports (it IS already imported per the file's existing `use std::sync::Arc;` line 17).

- [ ] **Step 3.7: Run the test — expect pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib bridge::foundry::tests::item_deleted_inbound_produces_modifier_reap_event`

Expected: passes.

- [ ] **Step 3.8: Run aggregate verify gate**

Run: `./scripts/verify.sh`

Expected: `verify: all checks passed`.

- [ ] **Step 3.9: Commit (bundles Task 2 + Task 3)**

```bash
git add src-tauri/src/bridge/foundry/types.rs src-tauri/src/bridge/source.rs src-tauri/src/bridge/foundry/mod.rs src-tauri/src/bridge/mod.rs
git commit -m "$(cat <<'EOF'
feat(bridge): handle Foundry item_deleted wire shape

Add FoundryInbound::ItemDeleted variant for the new explicit deletion
signal, route it through InboundEvent::ItemDeleted, and dispatch in the
bridge connection loop: DbState lookup + db_delete_by_advantage_binding
+ frontend emit of `modifiers://rows-reaped` carrying the deleted ids.

If DbState is not yet managed (e.g. during early startup), the reap is
skipped with a log — preferable to a panic.

Spec §5; rolls back the stale orphan-card behavior on the GM screen
once the JS-side wire shape is also shipped (next commit).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4 — Foundry bridge JS sends `item_deleted` wire

**Files:**
- Modify: `vtmtools-bridge/scripts/translate.js` — `hookItemChanges` function (currently lines 56-75)

**Tests required:** no (no JS test infrastructure; manual smoke is the gate per project workflow override).

- [ ] **Step 4.1: Extend the deleteItem branch**

The current `hookItemChanges` (committed in PR #29) loops over all three item events and sends `actor_update` for each. Restructure to special-case `deleteItem` so it ALSO sends `item_deleted`. Replace the existing function (after the line `export function hookItemChanges(socket) {` through its closing brace) with:

```js
export function hookItemChanges(socket) {
  for (const ev of ["createItem", "updateItem", "deleteItem"]) {
    Hooks.on(ev, (item) => {
      if (!socket || socket.readyState !== WebSocket.OPEN) return;
      const actor = item?.parent;
      // Skip world-directory items (parent === null) and the theoretical
      // case of an item embedded somewhere other than an Actor.
      if (!actor || actor.documentName !== "Actor") return;
      try {
        socket.send(JSON.stringify({
          type: "actor_update",
          actor: actorToWire(actor),
        }));
        // Explicit cleanup signal for live-item deletion — backend reaps
        // any advantage-bound character_modifiers row pointing at this
        // item. Sent IN ADDITION to actor_update so bridge state stays
        // accurate AND the modifier row is removed. See spec §3.2 of
        // docs/superpowers/specs/2026-05-13-gm-screen-live-data-priority-design.md.
        if (ev === "deleteItem") {
          socket.send(JSON.stringify({
            type: "item_deleted",
            actor_id: actor.id,
            item_id: item.id,
          }));
        }
      } catch (e) {
        console.warn(`[${MODULE_ID}] failed to push ${ev}:`, e);
      }
    });
  }
}
```

- [ ] **Step 4.2: Run aggregate verify gate**

Run: `./scripts/verify.sh`

Expected: `verify: all checks passed`. The bridge JS isn't in the typechecker/cargo gate, but verify must still pass.

- [ ] **Step 4.3: Commit**

```bash
git add vtmtools-bridge/scripts/translate.js
git commit -m "$(cat <<'EOF'
feat(bridge): send item_deleted wire on Foundry deleteItem hook

Extend hookItemChanges' deleteItem branch to also send an explicit
{type:'item_deleted', actor_id, item_id} message in addition to the
existing actor_update. The backend dispatcher reaps the matching
advantage-bound character_modifiers row.

Both messages are needed:
- actor_update keeps bridge.characters[].items[] in sync (frontend filter).
- item_deleted is the discrete cleanup signal (DB row delete).

Spec §3.2 of 2026-05-13-gm-screen-live-data-priority-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5 — Frontend filter + isStale removal (atomic)

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte:131-162,267-273,420-432` — drop `isStale` and the advantage-orphan branch
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte:11-16,57-65,100-110` — drop the `isStale` prop and its display

**Tests required:** no (Svelte/UI; manual smoke).

- [ ] **Step 5.1: Simplify CharacterRow's cardEntries derivation**

Edit `src/lib/components/gm-screen/CharacterRow.svelte`. Locate lines 130-162 (the `type CardEntry = …` declaration through the end of the `cardEntries` derivation). Replace that entire block with:

```ts
  // Build the card list per spec §8.1 (updated 2026-05-13 — advantage-orphan
  // branch removed; orphan reaping happens backend-side on deleteItem).
  type CardEntry =
    | { kind: 'materialized'; mod: CharacterModifier }
    | { kind: 'virtual'; virt: VirtualCard };

  let cardEntries = $derived.by((): CardEntry[] => {
    const entries: CardEntry[] = [];

    // (2) Walk advantage items, merging with materialized rows.
    for (const item of advantageItems) {
      const matched = charMods.find(m => m.binding.kind === 'advantage' && m.binding.item_id === item._id);
      if (matched) {
        entries.push({ kind: 'materialized', mod: matched });
      } else {
        entries.push({ kind: 'virtual', virt: {
          item,
          name: item.name,
          description: ((item.system as Record<string, unknown>)?.description as string | undefined) ?? '',
        }});
      }
    }

    // (3) Append free-floating modifiers. Advantage-bound orphans no longer
    // render here — the deleteItem hook in vtmtools-bridge triggers a DB delete
    // that arrives via `modifiers://rows-reaped`, removing them from the store.
    for (const m of charMods) {
      if (m.binding.kind === 'free') {
        entries.push({ kind: 'materialized', mod: m });
      }
    }
    return entries;
  });
```

- [ ] **Step 5.2: Drop the isStale check in canPushFor**

Still in `CharacterRow.svelte`. Locate `function canPushFor` (around lines 267-273). The current body contains `if (e.isStale) return false;`. Remove that line; the function becomes:

```ts
  function canPushFor(e: CardEntry): boolean {
    if (character.source !== 'foundry') return false;
    if (e.kind !== 'materialized') return false;            // virtual = no DB row yet
    if (e.mod.binding.kind !== 'advantage') return false;
    return e.mod.effects.some(eff => eff.kind === 'pool');
  }
```

- [ ] **Step 5.3: Remove the isStale prop pass at the ModifierCard call-site**

Still in `CharacterRow.svelte`. Locate the `<ModifierCard ... />` invocation (around line 420-432). One of its props is `isStale={entry.kind === 'materialized' && entry.isStale}`. Remove that ENTIRE prop line. The resulting component invocation should not contain `isStale=`.

- [ ] **Step 5.4: Drop the isStale prop from ModifierCard**

Edit `src/lib/components/gm-screen/ModifierCard.svelte`. Locate the `Props` interface (around lines 11-19). Remove the `isStale?: boolean;` line.

Locate the destructuring block (around lines 61-65) that includes `isStale = false,`. Remove that line from the destructuring.

Locate the rendered display (around line 105) that contains:

```svelte
      {#if isStale}<span class="stale" title="Source merit removed">stale</span>{/if}
```

Remove this entire conditional block. Also remove the `.stale` CSS rule from the component's `<style>` block (search the file for `.stale` and delete the rule + any whitespace around it).

- [ ] **Step 5.5: Run aggregate verify gate**

Run: `./scripts/verify.sh`

Expected: `verify: all checks passed`. The svelte-check warning count may decrease if `isStale` was producing warnings; otherwise unchanged.

If svelte-check reports any NEW errors (not warnings), they will be from a leftover `isStale` reference somewhere. Re-grep:

```bash
grep -rn "isStale" src/
```

Expected: no matches in `src/`. (Per the dead-code grep in the spec resolution, all use sites should be removed.)

- [ ] **Step 5.6: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): drop isStale orphan rendering

Remove the advantage-bound orphan branch from CharacterRow's cardEntries
derivation and drop the isStale field from CardEntry and ModifierCard.

Advantage-bound CharacterModifier rows that no longer have a matching
live item are no longer rendered as stale-orphan cards. Backend cleanup
runs in parallel via the item_deleted wire shape (previous commit).

Character-level orphan UI (Show orphans toggle, orphans section in
GmScreen.svelte) is intentionally retained — it covers a different
case (modifier rows whose entire character disappeared).

Spec §6.1 + §6.3 of 2026-05-13-gm-screen-live-data-priority-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6 — Modifier store listener for `modifiers://rows-reaped`

**Files:**
- Modify: `src/store/modifiers.svelte.ts` — add init that subscribes to the event

**Tests required:** no.

- [ ] **Step 6.1: Add the listener wiring**

Edit `src/store/modifiers.svelte.ts`.

At the top of the file, add the import (just after the existing `import type` block, around line 24):

```ts
import { listen } from '@tauri-apps/api/event';
```

Locate the `ensureLoaded` method (currently around lines 71-75). Replace it with:

```ts
  async ensureLoaded(): Promise<void> {
    if (_initialized) return;
    _initialized = true;
    await refresh();
    // Subscribe to backend-initiated row reaps (live deleteItem from Foundry).
    // The event carries the exact ids to drop — no refetch needed. The store's
    // long-standing "no auto-refetch on bridge state" invariant (see header
    // comment) is preserved: this is an explicit cleanup signal, not a state
    // diff. Spec §6.2 of
    // docs/superpowers/specs/2026-05-13-gm-screen-live-data-priority-design.md.
    void listen<{ ids: number[] }>('modifiers://rows-reaped', (e) => {
      for (const id of e.payload.ids) dropRow(id);
    });
  },
```

- [ ] **Step 6.2: Run aggregate verify gate**

Run: `./scripts/verify.sh`

Expected: `verify: all checks passed`.

- [ ] **Step 6.3: Commit**

```bash
git add src/store/modifiers.svelte.ts
git commit -m "$(cat <<'EOF'
feat(modifiers): listen for modifiers://rows-reaped

Subscribe to the backend's explicit cleanup signal: when an advantage-bound
modifier row is deleted server-side (e.g. because its Foundry item was
deleted), the store drops it from the local list immediately.

The listener is registered inside ensureLoaded so it activates once when
the GM screen is opened. The event payload carries the deleted ids — no
refetch round-trip needed. Preserves the store's "no auto-refetch on
bridge state" invariant (this is a cleanup signal, not a state diff).

Spec §6.2 of 2026-05-13-gm-screen-live-data-priority-design.md.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7 — Manual smoke test

**Tests required:** human-in-the-loop verification before pushing.

- [ ] **Step 7.1: Launch the tool**

Run: `cargo tauri dev` from the project root.

Expected: app boots without DB migration errors. (No schema changes in this branch — same migrations as master.)

- [ ] **Step 7.2: Foundry reload**

In your running Foundry instance, press F5 to reload so the new `translate.js` (with the `item_deleted` send) is loaded.

- [ ] **Step 7.3: Setup**

In the vtmtools GM screen, navigate to a character that has at least one **advantage merit** AND for which you have created a **saved-as-override** card (via the override action). The presence of an override is critical — that's the case that exhibited the orphan bug.

- [ ] **Step 7.4: Trigger the deletion**

In Foundry, open the same character's sheet and delete the merit you overrode.

**Expected behavior:**
1. Within ~1s, the merit's card disappears from the vtmtools GM screen.
2. The card does NOT linger as a stale/empty shell.
3. Foundry's dev console shows no `[vtmtools-bridge] failed to push deleteItem` warnings.
4. `cargo tauri dev` stderr shows no `[bridge:foundry] ItemDeleted reap failed` errors.

- [ ] **Step 7.5: Negative-path check (no override case)**

Pick another merit on the same character that has NO override (a virtual card). Delete it on the Foundry sheet.

**Expected:** card disappears within ~1s (it was virtual — driven by the `items[]` filter already; the new `item_deleted` is a no-op DB delete returning empty `ids`).

- [ ] **Step 7.6: Negative-path check (free-floating modifier untouched)**

If the character has any free-floating modifier (created via "Add new modifier" rather than materialized from an item), confirm it still renders normally.

**Expected:** free-floating modifier persists; nothing about its rendering changes.

- [ ] **Step 7.7: World-directory item check**

In Foundry, navigate to the Items directory (NOT a character's items list). Delete any item there.

**Expected:** no errors in either dev console. The bridge JS guard (`actor.documentName === "Actor"`) skips world-directory items, so neither `actor_update` nor `item_deleted` fires.

If all seven checks pass, proceed. If any fails, report the symptom and the exact step that failed — debug from there before moving on.

---

## Task 8 — Push, open PR, run code-review skill

**Tests required:** PR review must come back clean (or with actionable findings) before merge.

- [ ] **Step 8.1: Push the branch**

Run: `git push -u origin feat/gm-screen-live-data-priority`

- [ ] **Step 8.2: Open the PR**

Run:

```bash
gh pr create --base master --head feat/gm-screen-live-data-priority \
  --title "feat(gm-screen): auto-delete orphan modifiers on live item deletion" \
  --body "$(cat <<'EOF'
## Summary

Auto-delete advantage-bound `CharacterModifier` rows when their backing Foundry item is deleted on the live sheet, and stop rendering item-level orphan cards on the GM screen.

- New `item_deleted` wire shape from the Foundry bridge JS (rides on the `deleteItem` hook shipped in #29)
- New `FoundryInbound::ItemDeleted` variant → `InboundEvent::ItemDeleted` → dispatcher in `bridge/mod.rs` runs `db_delete_by_advantage_binding` → emits `modifiers://rows-reaped`
- Frontend filter in `CharacterRow.svelte` drops the advantage-orphan branch
- `isStale` field/prop removed from `CharacterRow` and `ModifierCard`

Character-level orphan UI (the "Show orphans" toggle in `GmScreen.svelte`) is intentionally retained — it covers a different case (modifier rows whose entire character disappears).

## Test plan

- [x] Rust unit tests for `db_delete_by_advantage_binding`: matches return ids, idempotent, free-floating untouched, scoped to (source, source_id) tuple
- [x] Rust deserialize test for `FoundryInbound::ItemDeleted`
- [x] Rust handler test: `FoundrySource::handle_inbound` returns `InboundEvent::ItemDeleted` for the JSON wire shape
- [x] `./scripts/verify.sh` green pre-commit on every commit
- [x] Manual smoke: delete merit with override → card disappears ~1s; no stderr errors
- [x] Manual smoke: delete merit without override → still disappears (was virtual)
- [x] Manual smoke: free-floating modifier untouched
- [x] Manual smoke: world-directory item delete silently skipped

## Design doc

`docs/superpowers/specs/2026-05-13-gm-screen-live-data-priority-design.md` (gitignored — local design archive).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [ ] **Step 8.3: Run code-review skill on the PR**

Note the PR number from step 8.2's output (e.g. PR #30). Invoke the code-review skill:

```
/code-review <PR_NUMBER>
```

Or pass arguments to the `code-review:code-review` skill: `PR https://github.com/Hampternt/vtmtools/pull/<N>`.

- [ ] **Step 8.4: Address any findings (if review returns issues)**

If the review surfaces issues with confidence ≥ 80, address them with new commits on the branch. Re-run `./scripts/verify.sh` after fixes. The review skill's "Found N issues" comment is the punch list.

- [ ] **Step 8.5: Merge (after clean review)**

```bash
gh pr merge <PR_NUMBER> --merge --delete-branch
git checkout master
git pull --ff-only origin master
git branch -d feat/gm-screen-live-data-priority
```

---

## Rollback / kill-switch notes

- **Reverting the JS-side wire (Task 4)** is sufficient to disable the auto-deletion behavior — the backend tolerates absence of `item_deleted` messages (no orphans get cleaned up, but no errors fire). The frontend filter from Task 5 continues to hide orphans regardless.
- **Reverting Task 5** is independent — it just brings back the orphan rendering. No backend interaction.
- **Reverting Task 3** is the most destructive — would re-introduce a non-exhaustive match. Bundle reverts go all-the-way if needed.
