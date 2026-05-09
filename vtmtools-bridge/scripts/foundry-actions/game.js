// Foundry game.* helper executors.
// See docs/superpowers/specs/2026-05-01-foundry-game-roll-helpers-design.md.

const MODULE_ID = "vtmtools-bridge";

async function rollV5Pool(msg) {
  console.log(`[${MODULE_ID}] game.js loaded — build ROLL-DIAG-1`);

  const actor = game.actors.get(msg.actor_id);
  if (!actor) {
    console.warn(`[${MODULE_ID}] game.roll_v5_pool: actor not found: ${msg.actor_id}`);
    throw new Error(`actor not found: ${msg.actor_id}`);
  }

  const paths = msg.value_paths ?? [];
  // getAdvancedDice is async on WoD5e 5.3.17 — must be awaited here.
  const advancedDice = msg.advanced_dice
    ?? (await WOD5E.api.getAdvancedDice({ actor }));
  const label = msg.flavor ?? deriveFlavorFromPaths(paths);
  const rollMode = msg.roll_mode ?? "roll";

  console.log(`[${MODULE_ID}] rollV5Pool inputs:`, {
    actorId: msg.actor_id,
    actorName: actor.name,
    actorSystem: actor.system?.gamesystem,
    paths,
    pathsType: typeof paths,
    advancedDice,
    advancedDiceType: typeof advancedDice,
    label,
    rollMode,
    difficulty: msg.difficulty,
    difficultyType: typeof msg.difficulty,
    selectors: msg.selectors,
    poolModifier: msg.pool_modifier,
  });

  // Always route non-empty value_paths through RollFromDataset — the proven
  // path that's been working since FHL Phase 2 shipped.
  //
  // pool_modifier (Plan C wire field) is currently IGNORED on the JS side.
  if (paths.length === 0) {
    // Rouse-style: zero basic dice + caller-supplied advanced dice.
    console.log(`[${MODULE_ID}] -> direct Roll (rouse-style, paths=[])`);
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

  const dataset = {
    valuePaths: paths.join(" "),
    label,
    difficulty: msg.difficulty,
    selectDialog: false,
    quickRoll: true,
    selectors: (msg.selectors ?? []).join(" "),
  };
  console.log(`[${MODULE_ID}] -> RollFromDataset with dataset:`, dataset);

  try {
    await WOD5E.api.RollFromDataset({ dataset, actor });
  } catch (err) {
    console.error(`[${MODULE_ID}] RollFromDataset threw:`, err);
    console.error(`[${MODULE_ID}] dataset was:`, dataset);
    console.error(`[${MODULE_ID}] actor.system.attributes:`, actor.system?.attributes);
    console.error(`[${MODULE_ID}] actor.system.skills:`, actor.system?.skills);
    throw err;
  }
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
