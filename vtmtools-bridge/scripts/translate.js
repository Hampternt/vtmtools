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
