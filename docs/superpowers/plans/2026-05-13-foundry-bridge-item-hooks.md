# Foundry Bridge Item & ActiveEffect Hooks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Subscribe the Foundry-side bridge module to Foundry's `createItem` / `updateItem` / `deleteItem` and `createActiveEffect` / `updateActiveEffect` / `deleteActiveEffect` hooks so player-side item and effect changes push live to vtmtools (the GM screen currently stays stale until an unrelated actor-level event happens).

**Architecture:** Add two new exported functions in `vtmtools-bridge/scripts/translate.js` that mirror the existing `hookActorChanges` pattern. Each subscribes to its three hooks and, on fire, walks `parent` (and for ActiveEffect on an Item, `parent.parent`) up to the owning Actor, then calls `actorToWire(actor)` and sends the existing `actor_update` wire shape. Wire shape is unchanged: the Rust backend's `FoundryInbound::ActorUpdate` and the frontend's `bridge://characters-updated` listener already handle this path correctly.

**Tech Stack:** Plain ES modules running inside the FoundryVTT (V11+) game-client JS environment. WebSocket connection to vtmtools localhost:7424. No build step, no bundler, no test framework — Foundry loads `module.json` and pulls these `.js` files directly.

**Spec:** None (user authorized skipping spec for this small change). Plan is the design.

---

## File structure

| File | Action | Responsibility |
|---|---|---|
| `vtmtools-bridge/scripts/translate.js` | Modify | Add `hookItemChanges(socket)` and `hookEffectChanges(socket)` adjacent to the existing `hookActorChanges`. |
| `vtmtools-bridge/scripts/foundry-actions/actor.js` | Modify | Import both new functions and call them from `actorsSubscriber.attach(socket)` immediately after the existing `hookActorChanges(socket)` call. |

No other files change. The Rust backend (`src-tauri/src/bridge/foundry/mod.rs`) already routes `actor_update` messages into `to_canonical` and emits `bridge://characters-updated` — no changes needed there or in the frontend.

## Risk surface (read before implementing)

1. **`item.parent` can be null** — for world-directory items not embedded on any actor. The new code must skip these.
2. **`item.parent` could theoretically not be an Actor** — defensive check via `parent.documentName === "Actor"` matches what Foundry guarantees.
3. **ActiveEffect lives on either an Actor or an Item** — for item-attached effects (transferable status effects from disciplines, etc.), `effect.parent` is the Item; the Actor is `effect.parent.parent`. The new code walks up exactly one level if needed.
4. **Socket may be closed at hook-fire time** — already handled by the existing pattern (`!socket || socket.readyState !== WebSocket.OPEN`). The new code copies that guard.
5. **`actorToWire` may throw** — wrap in try/catch matching the existing pattern.
6. **No hook detach mechanism exists** — the existing `hookActorChanges` registers hooks for the world-session lifetime with no teardown. The new functions match this. The existing TODO comment in `actor.js` (`"Full hook unregister is a translate.js follow-up if needed."`) covers both old and new. Do NOT introduce new detach plumbing in this plan.
7. **Bulk operations** — adding 5 effects in a tight loop sends 5 full-actor frames. On localhost this is fine per the project's "don't filter bridge payloads for size" convention. Not optimized here.
8. **No automated test framework** for the Foundry-side module. The smoke test is the only correctness gate, and it requires you to have Foundry running with a WoD5e world and the bridge connected.

## Verify gate

The project rule says every plan task ending in a commit MUST run `./scripts/verify.sh` first. The script runs `npm run check` + `cargo test` + frontend build — none of which touches the Foundry-side JS module. It will pass trivially (~30 seconds) because no main-repo file is changed by these tasks. Still mandatory per project CLAUDE.md.

---

## Task 1: Item-level hooks (createItem / updateItem / deleteItem)

**Files:**
- Modify: `vtmtools-bridge/scripts/translate.js` (add `hookItemChanges`)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (import + call)

**Tests:** none — no test framework on the bridge module side. Manual smoke is the gate.

- [ ] **Step 1: Add `hookItemChanges` to `translate.js`**

In `vtmtools-bridge/scripts/translate.js`, immediately after the existing `hookActorChanges` function (current code at line 41–54), add a new exported function. The new function should match the existing pattern's style exactly — same socket guard, same try/catch shape, same `console.warn` prefix — to keep the file uniform:

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
      } catch (e) {
        console.warn(`[${MODULE_ID}] failed to push ${ev}:`, e);
      }
    });
  }
}
```

`MODULE_ID` and `actorToWire` are both already in scope in this file (declared at the top of `translate.js`).

- [ ] **Step 2: Import + call from `actor.js`**

In `vtmtools-bridge/scripts/foundry-actions/actor.js`:

(a) Update the import on line 4 from:
```js
import { actorToWire, hookActorChanges } from "../translate.js";
```
to:
```js
import { actorToWire, hookActorChanges, hookItemChanges } from "../translate.js";
```

(b) In `actorsSubscriber.attach(socket)` (currently lines 22–30), add the call to `hookItemChanges(socket)` immediately after the existing `hookActorChanges(socket)` line. The updated function body:

```js
  attach(socket) {
    if (_attached) return;
    if (socket?.readyState === WebSocket.OPEN) {
      const actors = game.actors.contents.map(actorToWire);
      socket.send(JSON.stringify({ type: "actors", actors }));
      console.log(`[${MODULE_ID}] actorsSubscriber: pushed ${actors.length} actors`);
    }
    hookActorChanges(socket);
    hookItemChanges(socket);
    _attached = { socket };
  },
```

This preserves the existing `attach()` invariant — initial actors snapshot still goes out before any hooks register, so vtmtools never sees an ActorUpdate for an actor it doesn't already have a snapshot of.

- [ ] **Step 3: Run `./scripts/verify.sh`**

Mandatory per project CLAUDE.md. The Foundry-side bridge module isn't touched by any of verify.sh's gates (`npm run check`, `cargo test`, frontend build), so this will pass trivially.

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`

If it fails for a reason unrelated to these changes (e.g. an unrelated test broke), investigate before committing — don't suppress.

- [ ] **Step 4: Manual smoke test**

This step requires Foundry running with a WoD5e world and at least one V5 actor with at least one merit (or any item). The bridge must be connected (browser dev-tools console should show `[vtmtools-bridge] actorsSubscriber: pushed N actors` after `Hooks.once("ready")`).

If you cannot run Foundry yourself, surface this step's instructions to the user and ask them to perform it before you commit. Do NOT commit without smoke evidence.

Smoke checks (in order):

1. **Initial state.** Open vtmtools, go to the GM Screen. Confirm the test character renders a card for each of its existing merits/flaws/backgrounds/boons.

2. **Delete an item.** In Foundry, open the character sheet, right-click on a merit, choose Delete. In Foundry's dev-tools console, you should see the new hook fire (no specific log, just no warnings). In vtmtools, the corresponding card should disappear from the GM Screen within ~1 second.

3. **Add an item.** In Foundry, drag a merit from the world directory onto the character sheet (or create one inline). In vtmtools, a new card for that merit should appear within ~1 second.

4. **Edit an item.** In Foundry, edit an existing merit's `bonuses[]` value (e.g. change a +2 to +3). In vtmtools, the bonus line on the card should update to show the new value within ~1 second.

5. **Negative case — world-directory item.** In Foundry, open the Items directory (NOT a character sheet), create a new free-floating item there, then delete it. vtmtools should NOT log any error and the GM Screen should not refresh (no parent actor → skipped). Confirm by checking the browser dev-tools console for absence of `[vtmtools-bridge] failed to push deleteItem:` warnings.

If any of these fail, do NOT commit. Diagnose first. Common failure modes:
- Foundry hook didn't fire → check the bridge module is loaded and active for the GM user
- Hook fired but actor was null → world-item case; harmless skip
- Hook fired with a `documentName` other than `"Actor"` → unusual; would log a silent skip too

- [ ] **Step 5: Commit**

Stage exactly these two files. Do not stage the plan file (gitignored).

```bash
git add vtmtools-bridge/scripts/translate.js vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge): push actor updates on item create/update/delete

Subscribes the Foundry-side bridge module to createItem / updateItem /
deleteItem and forwards a full actor_update wire frame whenever an item
embedded on an actor changes. The Rust backend's ActorUpdate handler and
the frontend's bridge://characters-updated listener already consume this
wire shape — fix is one-sided.

Fixes the symptom where deleting a merit on the Foundry sheet left a
stale card on the GM Screen until an unrelated actor-level event fired.
World-directory items (no actor parent) are silently skipped.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: ActiveEffect-level hooks (createActiveEffect / updateActiveEffect / deleteActiveEffect)

**Files:**
- Modify: `vtmtools-bridge/scripts/translate.js` (add `hookEffectChanges`)
- Modify: `vtmtools-bridge/scripts/foundry-actions/actor.js` (import + call)

**Tests:** none — no test framework. Manual smoke is the gate.

This task extends the same pattern to ActiveEffect changes. ActiveEffects drive Foundry's modifier system for status conditions (frenzy, blinded, hunger-frenzy, etc.) and for transferred effects from items. Same staleness problem applies: editing or removing an effect doesn't push without this hook.

The wrinkle vs. Task 1: an ActiveEffect's parent can be an Actor OR an Item. For item-attached effects (the common case for transferable disciplines / merits with active-effect children), the effect's parent is the Item, and the Item's parent is the Actor. The function walks up one level if needed.

- [ ] **Step 1: Add `hookEffectChanges` to `translate.js`**

In `vtmtools-bridge/scripts/translate.js`, immediately after the `hookItemChanges` function added in Task 1, append:

```js
export function hookEffectChanges(socket) {
  for (const ev of ["createActiveEffect", "updateActiveEffect", "deleteActiveEffect"]) {
    Hooks.on(ev, (effect) => {
      if (!socket || socket.readyState !== WebSocket.OPEN) return;
      // ActiveEffect.parent is either Actor (actor-level effect) or Item
      // (item-attached effect, often with transfer: true). Walk up one
      // level if we landed on an Item.
      let actor = effect?.parent;
      if (actor?.documentName === "Item") actor = actor.parent;
      // Skip world-directory effects and any case where we couldn't
      // resolve up to an Actor.
      if (!actor || actor.documentName !== "Actor") return;
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
}
```

- [ ] **Step 2: Import + call from `actor.js`**

In `vtmtools-bridge/scripts/foundry-actions/actor.js`:

(a) Update the import to include the new function:
```js
import { actorToWire, hookActorChanges, hookItemChanges, hookEffectChanges } from "../translate.js";
```

(b) In `actorsSubscriber.attach(socket)`, add the call immediately after `hookItemChanges(socket)` (which Task 1 placed after `hookActorChanges(socket)`):

```js
  attach(socket) {
    if (_attached) return;
    if (socket?.readyState === WebSocket.OPEN) {
      const actors = game.actors.contents.map(actorToWire);
      socket.send(JSON.stringify({ type: "actors", actors }));
      console.log(`[${MODULE_ID}] actorsSubscriber: pushed ${actors.length} actors`);
    }
    hookActorChanges(socket);
    hookItemChanges(socket);
    hookEffectChanges(socket);
    _attached = { socket };
  },
```

- [ ] **Step 3: Run `./scripts/verify.sh`**

Mandatory per project CLAUDE.md. Same trivial-pass expectation as Task 1.

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`

- [ ] **Step 4: Manual smoke test**

Same Foundry-running prerequisite as Task 1. If you cannot run Foundry, surface to the user and wait for smoke evidence before committing.

Smoke checks (in order):

1. **Actor-level effect, create.** In Foundry, open the character sheet, navigate to the Effects tab, create a new ActiveEffect (e.g. name it "Test Effect", give it a single change row). In vtmtools, observe that the GM Screen card refreshes — concretely, if the actor's `raw.effects[]` is exposed anywhere (it currently is via `foundryEffects(actor)`), the new entry should appear. If no UI consumer of `effects[]` is visible on the GM Screen, confirm via the browser dev-tools that `bridge.characters` (the Svelte store) has been updated with the new effect by inspecting `actor.raw.effects`.

2. **Actor-level effect, delete.** Right-click the just-created effect, choose Delete. The effect should disappear from `bridge.characters` within ~1 second.

3. **Item-attached effect, create.** Open an existing merit (one with no current effect attached), navigate to its Effects sub-tab, create a new ActiveEffect on it. In vtmtools, the actor's `raw.items[].effects[]` for that item should now include the new entry.

4. **Item-attached effect, delete.** Delete that effect. The item's `effects[]` entry should disappear.

5. **Negative case — world-directory effect.** Not directly creatable via Foundry's UI (Effects always live on a parent), but if you create an item in the world directory and attach an effect to it, then delete the effect, vtmtools should silently skip the push (no parent Actor exists). Confirm by absence of `[vtmtools-bridge] failed to push deleteActiveEffect:` warnings.

If any of these fail, diagnose before committing. Likely failure mode:
- `effect.parent` returns something unexpected → log `effect.parent.documentName` temporarily to see what Foundry handed you. WoD5e v5.x targets Foundry V11+, where the contract is documented as "Actor or Item".

- [ ] **Step 5: Commit**

```bash
git add vtmtools-bridge/scripts/translate.js vtmtools-bridge/scripts/foundry-actions/actor.js
git commit -m "$(cat <<'EOF'
feat(bridge): push actor updates on ActiveEffect create/update/delete

Subscribes the Foundry-side bridge module to createActiveEffect /
updateActiveEffect / deleteActiveEffect and forwards a full actor_update
wire frame whenever an effect on an actor (or on an item embedded in an
actor) changes. Walks effect.parent up one level if the immediate parent
is an Item rather than an Actor.

Completes the live-update coverage for actor-side mutations alongside
the item hooks added in the previous commit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-review

**Spec coverage:** No spec exists per user authorization. The plan IS the design. Each risk-surface item enumerated above maps to specific guard code in the tasks:

| Risk | Where addressed |
|---|---|
| `item.parent` null (world item) | Task 1 Step 1 — `if (!actor || actor.documentName !== "Actor") return;` |
| `item.parent` not an Actor | Task 1 Step 1 — same guard, `documentName === "Actor"` check |
| Effect parent is Item, needs walk-up | Task 2 Step 1 — `if (actor?.documentName === "Item") actor = actor.parent;` then re-guard |
| Effect lands without resolving to Actor | Task 2 Step 1 — final guard, same shape |
| Socket closed at hook-fire | Both tasks Step 1 — `!socket || socket.readyState !== WebSocket.OPEN` |
| `actorToWire` throws | Both tasks Step 1 — try/catch with console.warn |
| Module-load race vs. initial actors snapshot | Preserved by existing `attach()` invariant — hooks register AFTER the initial snapshot is sent |
| No automated test | Manual smoke step in each task; mandatory before commit |
| Bulk-operation traffic | Documented in Risk Surface; not optimized (per `feedback_dont_filter_bridge_payload`) |

**Placeholder scan:** No "TBD" / "TODO" / vague phrases. Each step has the exact code or command to run.

**Type consistency:**
- `hookItemChanges(socket)` and `hookEffectChanges(socket)` both take a `socket` parameter — match.
- Both call `actorToWire(actor)` consistently (the only argument the existing function expects).
- The import statement in Task 2 Step 2 is the FULL list (`actorToWire, hookActorChanges, hookItemChanges, hookEffectChanges`) — superset of what Task 1 Step 2 left it at (`actorToWire, hookActorChanges, hookItemChanges`). No conflict.
- Wire shape `{ type: "actor_update", actor: ... }` is identical across both new functions and matches the existing `hookActorChanges` and the Rust backend's `FoundryInbound::ActorUpdate` variant.

**Commit footer:** No specific open GitHub issue tracks this. If you want to retroactively open one (e.g. "Live updates on item/effect mutations") and reference it via `Closes #N` in the commit footers, that's a user-authorization decision — not made by the executing agent.
