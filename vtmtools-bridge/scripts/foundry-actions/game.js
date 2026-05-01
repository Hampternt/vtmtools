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

  // WoD5e's _onConfirmRoll consumes a few dataset fields as space-separated
  // strings (valuePaths, selectors) and pulls quickRoll/selectDialog as
  // booleans. quickRoll skips the modifier dialog inside WOD5eDice.Roll;
  // selectDialog skips the splat-picker dialog earlier in RollFromDataset.
  const dataset = {
    valuePaths: paths.join(" "),
    label,
    difficulty: msg.difficulty,
    selectDialog: false,
    quickRoll: true,
    advancedDice,
    selectors: (msg.selectors ?? []).join(" "),
  };
  await WOD5E.api.RollFromDataset({ dataset, actor });
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
