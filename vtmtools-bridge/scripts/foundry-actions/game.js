// Foundry game.* helper executors.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

const MODULE_ID = "vtmtools-bridge";

async function rollV5Pool(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[${MODULE_ID}] game.roll_v5_pool: actor not found: ${msg.actor_id}`);
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  const paths = msg.value_paths ?? [];
  const advancedDice =
    msg.advanced_dice ?? WOD5E.api.getAdvancedDice({ actor });
  const label = msg.flavor ?? deriveFlavorFromPaths(paths);

  if (paths.length === 0) {
    // Rouse-style: zero basic dice + caller-supplied advanced dice. Use the
    // direct Roll API since RollFromDataset cannot represent an empty pool.
    await WOD5E.api.Roll({
      basicDice: 0,
      advancedDice,
      actor,
      difficulty: msg.difficulty,
      flavor: label,
      quickRoll: true,
    });
    return;
  }

  await WOD5E.api.RollFromDataset({
    dataset: {
      valuePaths: paths.join(" "),
      label,
      difficulty: msg.difficulty,
      selectDialog: false, // never pop the GM dialog from outside Foundry
      advancedDice,
      selectors: msg.selectors ?? [],
    },
    actor,
  });
}

async function postChatAsActor(msg) {
  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(
      `[${MODULE_ID}] game.post_chat_as_actor: actor not found: ${msg.actor_id}`,
    );
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  await ChatMessage.create({
    speaker: ChatMessage.getSpeaker({ actor }),
    content: msg.content,
    flavor: msg.flavor ?? null,
    rollMode: msg.roll_mode ?? "roll",
  });
}

function deriveFlavorFromPaths(paths) {
  if (!paths || paths.length === 0) return "Roll";
  return paths
    .map((p) => p.split(".").slice(-2)[0]) // "skills.brawl.value" → "brawl"
    .map((s) => s.charAt(0).toUpperCase() + s.slice(1))
    .join(" + ");
}

export const handlers = {
  "game.roll_v5_pool": rollV5Pool,
  "game.post_chat_as_actor": postChatAsActor,
};
