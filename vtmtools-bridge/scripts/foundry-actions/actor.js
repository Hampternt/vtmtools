// Foundry actor.* helper executors.
// See docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md.

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

async function applyDyscrasia(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) return;

  const existing = actor.items.filter(
    (i) =>
      i.type === "feature" &&
      i.system?.featuretype === "merit" &&
      typeof i.name === "string" &&
      i.name.startsWith("Dyscrasia: "),
  );
  if (msg.replace_existing && existing.length) {
    await actor.deleteEmbeddedDocuments(
      "Item",
      existing.map((i) => i.id),
    );
  }

  await actor.createEmbeddedDocuments("Item", [
    {
      type: "feature",
      name: `Dyscrasia: ${msg.dyscrasia_name}`,
      system: {
        featuretype: "merit",
        description: msg.merit_description_html,
        points: 0,
      },
    },
  ]);

  const current = actor.system?.privatenotes ?? "";
  const next =
    current.trim() === ""
      ? msg.notes_line
      : `${current}\n${msg.notes_line}`;
  await actor.update({ "system.privatenotes": next });
}

export const handlers = {
  "actor.update_field": wireExecutor(updateField),  // was: update_actor
  "actor.create_item_simple": wireExecutor(createItemSimple),  // was: create_item
  "actor.apply_dyscrasia": applyDyscrasia,  // was: apply_dyscrasia
};
