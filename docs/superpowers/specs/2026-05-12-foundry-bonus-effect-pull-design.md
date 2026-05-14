# Foundry-Bonus Auto-Display & Per-Item Override (Spec)

**Date:** 2026-05-12
**Feature:** GM Screen renders Foundry actor `system.bonuses[]` as effect cards via read-through, with per-item saved overrides that surgically round-trip on push.
**Status:** Approved design. Implementation plan not yet authorized.
**Related work:** Builds on the existing `gm_screen_push_to_foundry` flow that already mirrors Pool effects to `system.bonuses[]`. This spec adds the inverse (display direction) and tightens push semantics when the modifier was created as an override.

---

## 1. Goal

When a GM has a Foundry actor connected and selected, the GM Screen card automatically shows the actor's merit/flaw/background/boon bonuses as effect rows — without any save action and without DB rows being created. The GM can optionally "save" a per-item local override that supersedes the live read-through, edit it, see a yellow asterisk that marks the card as origin-from-saved-copy (so the GM knows the data is local-overriding rather than live), and surgically push it back so Foundry reflects the override without trampling unrelated player work.

## 2. Background / motivation

The push direction is already implemented in `src-tauri/src/tools/gm_screen.rs::do_push_to_foundry`: a `CharacterModifier` with `binding: Advantage { item_id }` and Pool effects mirrors itself to that item's `system.bonuses[]`, idempotent for re-press via a `"GM Screen #<id>: <name>"` source-tag prefix.

The pull direction is not implemented. The GM currently sees Foundry's bonuses only by switching to Foundry itself, and any GM-Screen-side modifier is hand-rolled with no link back to the merit/flaw it's meant to mirror. This spec closes the loop with the minimum schema and surface area:

- Read-through (no save) covers the common case: the GM just wants to see what the player has.
- Saved override covers the scene-buff case: the GM wants to temporarily change a value for the table.
- Surgical push covers the round-trip case: after editing the override, pushing should make Foundry display match — without destroying parallel player work.

## 3. Behavior

### 3.1 Display path (read-through)

For each item on the connected Foundry actor:

1. Filter the item's `system.bonuses[]` to entries with `activeWhen.check == "always"`.
2. Exclude bonuses whose `source` starts with `"GM Screen #"` (these are own previous pushes; including them would re-display modifiers already rendered natively).
3. If the filtered list is non-empty AND no saved override exists for the item, render a synthesized effect card derived live from those bonuses.
4. Otherwise (no qualifying bonuses AND no saved override), do not render anything for the item.

The synthesized card is regenerated every render from the current `BridgeState`. No DB writes occur on display.

### 3.2 Saved-override path

A "saved override" is a `CharacterModifier` with:
- `binding: Advantage { item_id: X }` for the item being overridden, AND
- `foundryCapturedLabels` non-empty (this field distinguishes overrides from hand-rolled modifiers that happen to be bound to a Foundry item — see §4).

If a saved override exists for item X, it renders **instead** of the read-through for item X. The read-through walker skips item X.

### 3.3 Yellow asterisk (origin indicator)

On a saved-override card, render a yellow asterisk in the corner. Its purpose is to signal **origin**: "this card's data comes from your saved local copy, which supersedes the live Foundry read-through." It does NOT indicate drift between saved and live state.

```
show_asterisk = foundryCapturedLabels.non_empty
                AND binding.kind == "advantage"
```

The two conditions are equivalent in practice — `foundryCapturedLabels` is only ever populated by the "Save as local override" action, which always creates an `Advantage`-bound modifier. The advantage check is belt-and-suspenders against future code that might populate captured labels without that binding.

**Why origin and not drift:** an earlier draft of this section defined the asterisk as a mismatch indicator comparing the modifier's pool effects against the item's current always-active live bonuses (excluding own-tagged pushes). That comparison was structurally broken: after a successful surgical push the only always-active bonuses on the item ARE the own-tagged ones, which the read-through walker filters out — so the comparison saw an empty live_set against a populated saved_set and the asterisk lit permanently. Switching to origin signaling makes the indicator's meaning monotonic (visible whenever a saved override exists; absent otherwise) and removes the dependency on a comparison that can't naturally clear.

**Existing virtual-mark `*` is orthogonal:** virtual cards (no DB row) carry their own asterisk meaning "not yet customized — reading Foundry directly". Saved-override cards carry the new origin asterisk meaning "your saved copy, supersedes Foundry". A card is either virtual or materialized — never both — so exactly one asterisk per card, each pointing at a distinct origin state.

**Drift detection is out of scope for v1.** If GMs ask for it later, a separate spec adds a snapshot comparison or a different visual.

### 3.4 Conditional bonuses

Bonuses with `activeWhen.check != "always"` are dropped from the read-through. They are not rendered as effect rows and not included in `live_set`.

For each item that has dropped conditionals, render a small badge `(N conditionals)` attached to whichever card is rendered for the item:

- Item with `always` bonuses + no saved override → badge on the synthesized read-through card.
- Item with `always` bonuses + saved override → badge on the override card.
- Item with ONLY conditional bonuses (no `always`) and no saved override → no card is rendered for the item; the badge is not surfaced. (Conditionals on otherwise-empty items are invisible in v1. Out of scope to fix.)
- Item with conditional bonuses + saved override but no qualifying `always` bonuses → badge on the override card.

Hovering the badge shows a tooltip listing each skipped bonus's `source` label and its `activeWhen.check` reason.

### 3.5 "Save as local override" button

Per read-through item card. Clicking creates a `CharacterModifier` via the existing create-modifier flow with:

- `binding: Advantage { item_id: <this item's _id> }`
- `effects`: one Pool effect per `activeWhen: always` bonus on the item (excluding own-tagged), with `delta = b.value` and `paths = b.paths`
- `foundryCapturedLabels`: the `source` strings of those captured bonuses, in order encountered
- `name`: the item's name (e.g. "Resilience")
- `description`: empty
- `is_active: true`
- `is_hidden: false`
- `tags: []`
- `originTemplateId: null`

From that moment, the read-through skips this item; the saved override renders. If the user later deletes the override (via the existing modifier UI), the read-through resumes automatically.

### 3.6 Push semantics

When `gm_screen_push_to_foundry(modifierId)` is called:

- **If `foundry_captured_labels` is empty** (hand-rolled modifier, current behavior preserved):
  1. Filter the item's existing `system.bonuses[]` to remove bonuses where `is_ours(b, modifier_id)` (matches `"GM Screen #<id>"` source prefix).
  2. Append the modifier's Pool effects as new bonuses tagged `source_tag(modifier_id, modifier_name)`.
  3. Write the updated `system.bonuses[]` to the item.

- **If `foundry_captured_labels` is non-empty** (saved override, surgical):
  1. Filter the item's existing `system.bonuses[]` to remove bonuses where:
     - `is_ours(b, modifier_id)`, OR
     - `b.source IN foundry_captured_labels` AND `b.activeWhen.check == "always"`.
  2. Append the modifier's Pool effects as new bonuses tagged `source_tag(modifier_id, modifier_name)`.
  3. Write the updated `system.bonuses[]` to the item.

Player-added bonuses whose `source` label is NOT in `foundry_captured_labels` survive the push.

### 3.7 Documented behavior: new player bonuses after override creation

If the player adds a new bonus to the same item after the GM saved the override (e.g. they add a "Frenzy Bonus" while the override is named "Resilience" and captured `["Buff Modifier"]`):

- Read-through doesn't see the new bonus (the saved override is rendering instead of the read-through for this item).
- Surgical push removes only `"Buff Modifier"` and own-tagged bonuses, leaving the new player bonus intact. The new player bonus stacks additively with the GM's override on the player's sheet.

This is intentional behavior. The GM can re-engage with the new bonus by deleting the saved override and re-saving (which captures the updated label set) or by editing the override's effects to include an equivalent of the new player bonus.

### 3.8 Disconnect behavior

When the Foundry bridge is disconnected (`BridgeState` has no actor blob for the source_id):

- Read-through produces nothing (no actor data to walk).
- Saved overrides still render from the DB as normal modifiers (they are persisted rows, not bridge-dependent).
- The origin asterisk on saved overrides continues to render (it depends only on the modifier's own state, not on live data).
- "Save as local override" button is not shown (no live bonuses to capture).

On reconnect, the actor blob reappears in `BridgeState` and the read-through walker recomputes from scratch.

## 4. Schema change

`CharacterModifier` gains one field:

**Rust** (`src-tauri/src/shared/modifier.rs`):

```rust
#[serde(default)]
pub foundry_captured_labels: Vec<String>,
```

**TypeScript** (`src/types.ts` mirror):

```ts
foundryCapturedLabels: string[];
```

**DB migration:** add a column to the modifier persistence backing, defaulting to `[]`. Default ensures every existing modifier behaves exactly as today (`foundry_captured_labels` empty → additive push path). No backfill of existing data is required.

**`NewCharacterModifierInput`** also gains the field so the frontend can populate it at "Save as local override" time:

```ts
foundryCapturedLabels: string[];   // empty for hand-rolled, populated for overrides
```

The field is not exposed in `ModifierPatchInput` for v1 — overrides recapture by delete-and-re-save, not by edit.

## 5. Architecture

### 5.1 Frontend (pure JS, no new IPC)

The entire read-through walker, origin-asterisk check, and "Save as local override" handler live in the frontend. Inputs:

- `actor.raw.items[]` via the existing `foundryItems(actor)` lens helper in `src/lib/foundry/raw.ts`
- The list of `CharacterModifier`s for the current actor (already fetched via the existing modifier API)

Outputs:

- An ordered render list of `{ kind: "synth" | "override"; itemId; itemName; effects; showOverride?: bool; conditionalsSkipped?: ConditionalSkipDetail[] }`.

No new IPC command is needed for display. The walker is a deterministic projection over data the frontend already has.

### 5.2 Backend (Rust)

Changes are scoped to:

- `CharacterModifier` struct + serde (add `foundry_captured_labels`)
- `NewCharacterModifierInput` (mirror the new field)
- DB persistence layer (column add, read/write handling)
- `do_push_to_foundry` (extend the merge step to consume `foundry_captured_labels` when non-empty)

No changes to:

- The Foundry bridge ingest path
- `BridgeState`
- The `FoundryActor` / `FoundryInbound` types
- The Roll20 bridge

### 5.3 No new commands

The push command (`gm_screen_push_to_foundry`) already exists and is reused with extended internal logic. No new Tauri command is introduced.

## 6. UX surface

1. **Synthesized read-through card** for each rendered item. Visually distinct from saved-modifier cards (the implementation plan will specify exact styling — likely a muted border or background, no edit affordances, no push button).

2. **"Save as local override" button** on each synthesized card.

3. **Yellow origin asterisk** in the corner of saved-override cards. Indicates "this card's data comes from your saved copy, which supersedes the live Foundry read-through." Always visible on saved overrides; never on virtual / hand-rolled cards.

4. **`(N conditionals)` badge** per item with non-`always` bonuses, with hover tooltip listing them.

5. **No new "revert to live" action** — the GM deletes the saved override via the existing modifier UI to return to read-through.

## 7. Edge cases

| Case | Behavior |
|---|---|
| Item has only conditional bonuses, no override | No card rendered. Badge would have nothing to attach to. Skipped silently. |
| Item has no bonuses at all | Not rendered. |
| Override exists, but item was removed from actor in Foundry | Override still renders (DB row persists) with the origin asterisk. Surgical push removes captured labels (no-op for missing item) and writes our effects to a no-longer-existing item — push fails gracefully or no-ops. **Resolution: out of scope for v1; treat orphaned overrides as dust.** |
| Override's `foundry_captured_labels` includes a label that's been renamed in Foundry | Surgical push removes nothing for that label (no match), writes our effects. The renamed bonus survives on the player's sheet. Origin asterisk continues to show (saved override still exists). |
| Player adds a NEW bonus to the same item after override is saved | Survives push (label not in captured set). The new player bonus stacks additively with the GM's override. See §3.7. |
| Bonus's `paths` field is empty `[]` | Treated as a pathless bonus. Push uses the existing `vec!["".to_string()]` placeholder behavior in `effect_to_bonus`. |
| Two bonuses on one item have the same `source` label | At save time, both labels go into `foundry_captured_labels` (which becomes `["X", "X"]`). On surgical push, removal is by `source IN labels`, so all bonuses with that label are removed. Acceptable. |
| Hand-rolled modifier bound to Foundry item with non-empty bonuses | `foundry_captured_labels` is empty → additive push (current behavior). Origin asterisk does NOT show (guard in §3.3). |

## 8. Out of scope (v1)

- Active Effects (`actor.effects[]` / `item.effects[]`) — bonuses only.
- Conditional-bonus rendering or evaluation (any `activeWhen.check` other than `"always"`).
- Orphan cleanup for saved overrides whose item no longer exists in the actor.
- Per-bonus granularity overrides — overrides are item-level only.
- Automatic recapture of `foundry_captured_labels` on push or refresh — recapture is user-driven by delete-and-re-save.
- Round-trip parity with `displayWhenInactive`, `unless`, and other non-modeled bonus fields — they are not preserved through the override cycle.
- A "revert to live" affordance — modeled by delete-the-override.

## 9. Testing strategy (informational; plan will specify per-task)

The new logic is mostly pure transformation (frontend walker + Rust merge step). Genuine logic worth testing:

- Surgical push merge in `do_push_to_foundry` — input: existing bonuses + override with captured labels → expected output bonuses. Cover: captured-label removal, own-tag removal, intersection of both, preservation of non-captured player bonuses, conditional bonus preservation.
- Read-through walker — input: actor items + saved modifiers → expected render list. Cover: item with bonuses + no override (synthesized), item with bonuses + override (override only), item with only conditionals (skipped + badge), item with no bonuses (skipped).

Origin asterisk has no comparison logic worth a unit test — it's a one-line boolean over `foundryCapturedLabels.length > 0 && binding.kind === 'advantage'`. Wiring covered by the existing `./scripts/verify.sh` gate.

## 10. Drift detection (deferred)

This spec deliberately omits any GM-facing signal for "the live Foundry bonuses have drifted from your saved override." The origin asterisk indicates *origin*, not *drift*. If GM feedback shows that drift visibility matters, a follow-up spec can add a snapshot comparison or a separate badge — but it should be its own design with its own UX validation, not bolted onto the origin marker.
