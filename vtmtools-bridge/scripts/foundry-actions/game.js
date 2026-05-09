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
  const advancedDice = msg.advanced_dice
    ?? WOD5E.api.getAdvancedDice({ actor });
  const label = msg.flavor ?? deriveFlavorFromPaths(paths);
  const rollMode = msg.roll_mode ?? "roll";
  const poolModifier = msg.pool_modifier ?? 0;

  // Direct-Roll-API path: empty paths (rouse-style) OR caller specified a
  // pool_modifier (popover semantics). Bypassing RollFromDataset here is
  // deliberate — it avoids double-counting any modifier card that has been
  // pushed to the sheet via GM Screen Plan C (those bonuses would also be
  // auto-applied via Foundry's selectors-based situational-bonus pipeline).
  // The popover's poolModifier already encodes the GM's intent.
  if (paths.length === 0 || poolModifier !== 0) {
    const basicDice = computeBasicDice(actor, paths) + poolModifier;
    await WOD5E.api.Roll({
      basicDice,
      advancedDice,
      actor,
      difficulty: msg.difficulty,
      flavor: label,
      quickRoll: true,
      rollMode,
    });
    return;
  }

  // RollFromDataset path: auto-applies sheet bonuses via the WoD5e selectors
  // pipeline. Used for non-popover callers (e.g., a future stat-button click
  // that wants the full sheet-bonus expansion). Selectors stay caller-supplied.
  await WOD5E.api.RollFromDataset({
    dataset: {
      valuePaths: paths.join(" "),
      label,
      difficulty: msg.difficulty,
      selectDialog: false,            // never pop the GM picker from outside Foundry
      quickRoll: true,                // skip the modifier dialog inside WOD5eDice.Roll (fix 2631d2e)
      advancedDice,
      // WoD5e's _onConfirmRoll calls `dataset.selectors.split(...)` — it wants
      // the pre-split string, not the array. Same convention as valuePaths
      // (fix fbb3050).
      selectors: (msg.selectors ?? []).join(" "),
      rollMode,
    },
    actor,
  });
}

/**
 * Walks each path against actor.system and sums the numeric leaf values.
 * Returns 0 for paths that don't resolve to numbers (defensive — actor data
 * shape may have nulls or missing keys). Intentionally does NOT cap at any
 * ceiling; respects whatever value Foundry stores.
 */
function computeBasicDice(actor, paths) {
  let sum = 0;
  for (const path of paths) {
    // path like "attributes.strength.value"; walk against actor.system.
    const v = path.split(".").reduce((obj, key) => obj?.[key], actor.system);
    if (typeof v === "number") sum += v;
  }
  return sum;
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
