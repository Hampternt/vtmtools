// VTM5e (WoD5e v5.x) actor → wire format. Path constants are NOT hardcoded
// here — we send actor.system raw and let the Tauri-side
// foundry/translate.rs pick paths from
// docs/reference/foundry-vtm5e-paths.md. This keeps the JS module
// schema-agnostic; if WoD5e renames a field, only the Rust path
// constants need updating.

const MODULE_ID = "vtmtools-bridge";

export function actorToWire(actor) {
  return {
    id: actor.id,
    name: actor.name,
    owner: pickPlayerOwner(actor),
    system: actor.system,
    // Embedded item documents (merits, flaws, backgrounds, weapons,
    // disciplines, resonance, specialties, etc.). Each carries its own
    // `system` and `effects`. Filtering happens downstream — see
    // src/lib/foundry/raw.ts.
    items: actor.items.contents.map((i) => i.toObject()),
    // Actor-level ActiveEffect documents. Item-attached effects ride
    // along inside `items[].effects` and are NOT duplicated here.
    effects: actor.effects.contents.map((e) => e.toObject()),
  };
}

/// Returns the user id of the first non-GM owner with OWNER permission
/// (level 3), or null if the actor is GM-only. Mirrors Roll20's
/// `controlled_by` semantics.
function pickPlayerOwner(actor) {
  const ownership = actor.ownership ?? {};
  for (const [uid, level] of Object.entries(ownership)) {
    if (level !== 3) continue;
    if (uid === "default") continue;
    const user = game.users?.get?.(uid);
    if (user && !user.isGM) return uid;
  }
  return null;
}

export function hookActorChanges(socket) {
  for (const ev of ["updateActor", "createActor", "deleteActor"]) {
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
}

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

export function hookEffectChanges(socket) {
  for (const ev of ["createActiveEffect", "updateActiveEffect", "deleteActiveEffect"]) {
    Hooks.on(ev, (effect) => {
      if (!socket || socket.readyState !== WebSocket.OPEN) return;
      // Effect parent can be either an Actor (actor-level effect) or an Item
      // (item-attached effect, in which case the Actor is the item's parent).
      // Skip world-level effects and any other unexpected attachment.
      const parent = effect?.parent;
      let actor = null;
      if (parent?.documentName === "Actor") {
        actor = parent;
      } else if (parent?.documentName === "Item" && parent.parent?.documentName === "Actor") {
        actor = parent.parent;
      }
      if (!actor) return;
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
