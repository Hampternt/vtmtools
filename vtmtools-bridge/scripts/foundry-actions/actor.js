// Foundry actor.* helper executors.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md.

import { actorToWire, hookActorChanges, hookItemChanges, hookEffectChanges } from "../translate.js";

const MODULE_ID = "vtmtools-bridge";

let _attached = null; // { socket } when attached, else null

/**
 * The actors subscriber. Encapsulates the "push initial actors + hook
 * future changes" behavior previously inlined in bridge.js's open handler.
 *
 * INVARIANT: attach() pushes the initial Actors frame BEFORE registering
 * hooks, ensuring the desktop never sees an ActorUpdate without the prior
 * Actors snapshot. Calling detach() unregisters the active flag; the
 * socket is not closed (bridge.js owns the socket lifecycle). Full hook
 * unregister is a translate.js follow-up if needed.
 */
export const actorsSubscriber = {
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

  detach() {
    _attached = null;
  },
};

const wireExecutor = (fn) => async (msg) => {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[vtmtools-bridge] actor not found: ${msg.actor_id}`);
    return;
  }
  await fn(actor, msg);
};

async function updateField(actor, msg) {
  await actor.update({ [msg.path]: msg.value });
}

async function updateItemField(actor, msg) {
  const item = actor.items.get(msg.item_id);
  if (!item) {
    console.warn(`[vtmtools-bridge] item not found on actor ${msg.actor_id}: ${msg.item_id}`);
    return;
  }
  await item.update({ [msg.path]: msg.value });
}

async function createItemSimple(actor, msg) {
  if (msg.replace_existing) {
    const existing = actor.items.filter((i) => i.type === msg.item_type);
    if (existing.length) {
      await actor.deleteEmbeddedDocuments(
        "Item",
        existing.map((i) => i.id),
      );
    }
  }
  await actor.createEmbeddedDocuments("Item", [
    { type: msg.item_type, name: msg.item_name },
  ]);
}

async function deleteItemById(actor, msg) {
  await actor.deleteEmbeddedDocuments("Item", [msg.item_id]);
}

async function deleteItemsByPrefix(actor, msg) {
  const matches = actor.items.filter(
    (i) =>
      i.type === msg.item_type &&
      (msg.featuretype === null ||
        msg.featuretype === undefined ||
        i.system?.featuretype === msg.featuretype) &&
      typeof i.name === "string" &&
      i.name.startsWith(msg.name_prefix),
  );
  if (matches.length === 0) return;
  await actor.deleteEmbeddedDocuments(
    "Item",
    matches.map((i) => i.id),
  );
}

async function createFeature(actor, msg) {
  await actor.createEmbeddedDocuments("Item", [
    {
      type: "feature",
      name: msg.name,
      system: {
        featuretype: msg.featuretype,
        description: msg.description,
        points: msg.points,
      },
    },
  ]);
}

async function replacePrivateNotes(actor, msg) {
  await actor.update({ "system.privatenotes": msg.full_text });
}

async function appendPrivateNotesLine(actor, msg) {
  const current = actor.system?.privatenotes ?? "";
  const next =
    current.trim() === "" ? msg.line : `${current}\n${msg.line}`;
  await actor.update({ "system.privatenotes": next });
}

// Composes deleteItemsByPrefix + createFeature + appendPrivateNotesLine;
// single inbound-handler tick preserves atomicity.
async function applyDyscrasia(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) return;

  if (msg.replace_existing) {
    await deleteItemsByPrefix(actor, {
      item_type: "feature",
      featuretype: "merit",
      name_prefix: "Dyscrasia: ",
    });
  }

  await createFeature(actor, {
    featuretype: "merit",
    name: `Dyscrasia: ${msg.dyscrasia_name}`,
    description: msg.merit_description_html,
    points: 0,
  });

  await appendPrivateNotesLine(actor, { line: msg.notes_line });
}

export const handlers = {
  "actor.update_field": wireExecutor(updateField),
  "actor.update_item_field": wireExecutor(updateItemField),
  "actor.create_item_simple": wireExecutor(createItemSimple),
  "actor.delete_item_by_id": wireExecutor(deleteItemById),
  "actor.delete_items_by_prefix": wireExecutor(deleteItemsByPrefix),
  "actor.create_feature": wireExecutor(createFeature),
  "actor.replace_private_notes": wireExecutor(replacePrivateNotes),
  "actor.append_private_notes_line": wireExecutor(appendPrivateNotesLine),
  "actor.apply_dyscrasia": applyDyscrasia,
};
