# GM Screen Live-Data Priority — Design Spec

**Date:** 2026-05-13
**Status:** Drafted (pre-plan)
**Dependency:** `feat/foundry-bridge-item-hooks` (must land first)

## 1. Problem statement

The GM screen renders a card for each advantage-bound modifier on a
character. When a merit is deleted on the live Foundry sheet, the card
persists on the GM screen — its bonus contents disappear but the card
shell stays. Root cause: `CharacterRow.svelte` deliberately surfaces
`CharacterModifier` rows whose `binding.item_id` no longer matches any
live item, rendered as `isStale: true` orphans (see lines 154-160 of
`CharacterRow.svelte`).

The user expected: "live data is authoritative — if Foundry deletes the
item, the card should disappear." Current behavior: "saved data outlives
the live item." This was intentional in the override-feature design
(saved data shadowing live read-through), but the orphan-survival aspect
was not. The override feature should disambiguate:

- **Saved data wins for *content*** — bonus values, captured labels,
  notes (the override feature; deliberate; preserve).
- **Saved data must NOT win for *existence*** — a deleted live item must
  not leave a zombie card (the bug; fix this).

## 2. Decisions made (from brainstorming)

| Decision | Choice | Rationale |
|---|---|---|
| Orphan policy | Auto-delete on confirmed deletion | User accepts "no recovery" UX in exchange for clean mental model |
| Trigger | Explicit `item_deleted` wire shape, fired by `deleteItem` Foundry hook | Discrete signal; resilient to any partial actor_update; matches user mental model |
| Update policy | Unchanged — overrides keep shadowing live mutations | The override feature's purpose is exactly content-shadowing; bonus drift can be a future feature |
| Scope | Foundry only; advantage-bound rows only | Roll20 has no item-deletion model; free-floating modifiers are by definition local-only and not affected |

## 3. Architecture

Two complementary changes, both required:

### 3.1 Frontend filter (the immediate visual fix)

In `src/lib/components/gm-screen/CharacterRow.svelte`, simplify the
card-entry derivation: **an advantage-bound `CharacterModifier` renders
iff its `binding.item_id` is present in the current character's
`items[]`.** Otherwise, it is filtered out entirely.

The current Loop 2 branch that adds advantage-bound orphans with
`isStale: true` is removed. Free-floating modifiers continue to render
unconditionally via the existing `m.binding.kind === 'free'` path.

This handles both connected mode (live `items[]`) and the
`savedAsBridge` offline fallback (snapshot `items[]`) with the same
rule. Any pre-existing advantage-bound modifier whose item is missing
from the current character simply stops appearing.

### 3.2 Backend cleanup (data hygiene)

A new wire shape from `vtmtools-bridge` triggers a surgical DB delete
when a live item is removed from a Foundry actor. The frontend
filter alone would leave dead rows accumulating in
`character_modifiers`; the explicit reaping signal eliminates them.

## 4. Wire protocol — new shape

Sent from `vtmtools-bridge/scripts/translate.js` inside the
`deleteItem` branch of `hookItemChanges`:

```js
Hooks.on("deleteItem", (item) => {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  const actor = item?.parent;
  if (!actor || actor.documentName !== "Actor") return;

  // Existing (from feat/foundry-bridge-item-hooks): keep bridge state in sync.
  socket.send(JSON.stringify({
    type: "actor_update",
    actor: actorToWire(actor),
  }));

  // NEW: explicit cleanup signal.
  socket.send(JSON.stringify({
    type: "item_deleted",
    actor_id: actor.id,
    item_id: item.id,
  }));
});
```

The two messages serve different purposes:

- `actor_update` continues to refresh `bridge.characters[i].items[]` so
  the frontend filter has accurate state to filter against.
- `item_deleted` is the discrete cleanup signal — the only trigger for
  any DB row deletion.

`createItem` and `updateItem` paths in `hookItemChanges` are unchanged
(they send only `actor_update`).

## 5. Rust handling

### 5.1 New FoundryInbound variant

In `src-tauri/src/bridge/foundry/types.rs`, add:

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FoundryInbound {
    // ... existing variants ...
    ItemDeleted {
        actor_id: String,
        item_id: String,
    },
}
```

### 5.2 New InboundEvent variant

In `src-tauri/src/bridge/source.rs`, add:

```rust
pub enum InboundEvent {
    // ... existing variants ...
    ModifierRowsReaped { ids: Vec<i64> },
}
```

### 5.3 Foundry handler

In `src-tauri/src/bridge/foundry/mod.rs`, add an `ItemDeleted` arm to
`handle_inbound`:

```rust
FoundryInbound::ItemDeleted { actor_id, item_id } => {
    // Delete matching advantage-bound modifier rows.
    let ids = crate::db::modifier::delete_by_advantage_binding(
        &self.pool, "foundry", &actor_id, &item_id,
    )
    .await
    .map_err(|e| e.to_string())?;

    return Ok(vec![InboundEvent::ModifierRowsReaped { ids }]);
}
```

**Pool injection:** `FoundrySource` currently has no pool reference.
Either:

- Add `pub pool: sqlx::SqlitePool` field to `FoundrySource`,
  populated at construction in `lib.rs`, OR
- Route `ItemDeleted` differently (e.g. handle in
  `src-tauri/src/bridge/mod.rs` outside the trait, similar to how
  `Error`/`Hello` are routed pre-trait).

The plan task will resolve which option matches the existing pattern
better — the second is closer to how Hello/Error are routed today; the
first is closer to how regular `actor_update` flows.

### 5.4 New DB function

In `src-tauri/src/db/modifier.rs`:

```rust
/// Deletes all advantage-bound modifier rows for the given (source,
/// source_id) whose binding_item_id matches. Returns the ids of
/// deleted rows. Idempotent — returns empty Vec if no matches.
/// Gated to binding_kind='advantage' so free-floating modifiers are
/// never affected.
pub async fn delete_by_advantage_binding(
    pool: &SqlitePool,
    source: &str,
    source_id: &str,
    item_id: &str,
) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar!(
        r#"DELETE FROM character_modifiers
           WHERE source = ?1
             AND source_id = ?2
             AND binding_kind = 'advantage'
             AND binding_item_id = ?3
           RETURNING id as "id!: i64""#,
        source, source_id, item_id,
    )
    .fetch_all(pool)
    .await
}
```

### 5.5 Event emission

In `src-tauri/src/bridge/mod.rs`, add to the inbound-event match block:

```rust
InboundEvent::ModifierRowsReaped { ids } if !ids.is_empty() => {
    let _ = handle.emit("modifiers://rows-reaped", &json!({ "ids": ids }));
}
InboundEvent::ModifierRowsReaped { .. } => {}
```

## 6. Frontend handling

### 6.1 CharacterRow rendering simplification

In `src/lib/components/gm-screen/CharacterRow.svelte`, the
`cardEntries` derivation collapses. The `CardEntry` discriminated union
loses the `isStale` field on the `materialized` variant:

```ts
type CardEntry =
  | { kind: 'materialized'; mod: CharacterModifier }
  | { kind: 'virtual'; virt: VirtualCard };

let cardEntries = $derived.by((): CardEntry[] => {
  const entries: CardEntry[] = [];

  // (1) Live advantage items → matched (with mod) or virtual (no mod).
  for (const item of advantageItems) {
    const matched = charMods.find(
      m => m.binding.kind === 'advantage' && m.binding.item_id === item._id
    );
    if (matched) {
      entries.push({ kind: 'materialized', mod: matched });
    } else {
      entries.push({ kind: 'virtual', virt: {
        item, name: item.name,
        description: ((item.system as Record<string, unknown>)?.description as string | undefined) ?? '',
      }});
    }
  }

  // (2) Free-floating modifiers only. Advantage-orphan branch removed.
  for (const m of charMods) {
    if (m.binding.kind === 'free') {
      entries.push({ kind: 'materialized', mod: m });
    }
  }

  return entries;
});
```

Any downstream code reading `entry.isStale` (e.g. the
`<ModifierCard isStale={...} />` prop pass-through) is removed along
with the field — see §6.3.

### 6.2 Modifier store event listener

In `src/store/modifiers.svelte.ts`, subscribe to the new event during
`ensureLoaded`:

```ts
listen<{ ids: number[] }>('modifiers://rows-reaped', (e) => {
  for (const id of e.payload.ids) dropRow(id);
});
```

`dropRow` already exists. No refetch needed — the payload carries the
exact ids to remove.

### 6.3 Dead code removal

| File | Element | Action |
|---|---|---|
| `src/store/modifiers.svelte.ts` | `_showOrphans` state + accessor | Remove |
| `src/lib/components/gm-screen/CharacterRow.svelte` | `isStale` field on CardEntry, all uses | Remove |
| `src/lib/components/gm-screen/ModifierCard.svelte` | `isStale` prop, its styling, any UI hint copy referencing stale state | Remove |
| GM screen settings (if any UI shows "show orphans") | Toggle button | Remove |

A grep for `isStale` and `showOrphans`/`_showOrphans` should produce an
exhaustive list during planning.

## 7. Edge cases and guarantees

| Case | Behavior |
|---|---|
| Foundry connected; live item deleted | `actor_update` refreshes `items[]`; `item_deleted` deletes any matching CharacterModifier row; frontend filter hides the card; `modifiers://rows-reaped` removes the store row |
| Foundry disconnected, viewing SavedCharacter snapshot | Snapshot's `items[]` drives filter the same way; no `item_deleted` ever fires while disconnected; orphans (if any) just don't render |
| Pre-existing orphans in DB (from before this change) | Stop rendering immediately (filter); persist in DB until naturally reaped or until the user manually deletes them |
| World-directory item deleted (no actor parent) | `hookItemChanges` guard skips it; no message sent |
| Same item deleted multiple times | `delete_by_advantage_binding` is idempotent — second call returns empty `ids`; no frontend update |
| Manual `modifiers.delete()` race with `item_deleted` | Both delete by id (manual) or by binding (auto). Manual delete already drops the row from the store; `item_deleted` returns empty ids on the now-missing row → no-op |
| Free-floating modifier whose user manually entered a stale `item_id` somehow | Not possible: free-floating rows have `binding.kind='free'` and no `item_id`. Reaper is gated on `binding_kind='advantage'`. |
| Roll20 character with materialized modifier | `item_deleted` is Foundry-only; no Roll20 path |

## 8. Out of scope

- **Bonus-content drift between live and override** — `* Saved local override` asterisk already communicates "this is saved data." Drift detection is a separate feature (likely a future "pull live values" action).
- **Whole-character SavedCharacter snapshot freshness** — the snapshot is intentionally a point-in-time copy. Not reaped.
- **Roll20** — no item-deletion model on that side.
- **ActiveEffect deletion** — Task 2 of the item-hooks branch covers
  `deleteActiveEffect`, which fires `actor_update` and keeps live state
  fresh. The current spec only addresses *item* deletion; ActiveEffect
  rows are not currently mapped to CharacterModifier rows, so no
  reaping is needed for them.

## 9. Testing strategy

- **Rust unit tests** for `delete_by_advantage_binding`:
  - returns ids for matches
  - returns empty for no match (idempotent)
  - does not delete `binding_kind='free'` rows
  - does not delete rows for other `(source, source_id)` tuples
- **Rust integration test** for the foundry handler arm: feeding
  `{type:'item_deleted', ...}` JSON through `handle_inbound` returns
  `InboundEvent::ModifierRowsReaped { ids }` with expected ids.
- **Frontend smoke test (manual)** after merge:
  1. Connect Foundry; load a character with an advantage merit
  2. Materialize the merit (engage / save as override)
  3. Delete the merit on the Foundry sheet
  4. Expect: card disappears from GM screen within ~1s; DB row gone
- **Frontend negative-path smoke**: free-floating modifier never deletes
  when an unrelated item is deleted.
- **`./scripts/verify.sh`** is the standing gate before any commit.

## 10. Rollout sequencing

1. Land `feat/foundry-bridge-item-hooks` first (Task 1 commit on the
   open branch, Task 2 commit, smoke-test gate, merge to master).
2. Branch this work from post-merge master: `feat/gm-screen-live-data-priority`.
3. Implement in plan-driven order: DB function + tests → Rust handler
   arm + tests → wire shape on bridge → frontend filter simplification
   → store listener → dead-code removal → manual smoke test.
4. Single final `code-review:code-review` pass against the full branch
   diff per the lean-execution workflow override in `CLAUDE.md`.
5. Merge to master; close the parent feature issue (if one is created)
   with completion-evidence per
   [[feedback_completion_evidence_on_issues]].
