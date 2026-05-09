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

  // Always route non-empty value_paths through RollFromDataset — the proven
  // path that's been working since FHL Phase 2 shipped. The earlier
  // pool_modifier-triggered direct-Roll branch was abandoned because
  // WOD5E.api.Roll's invocation signature didn't survive contact with a
  // real Foundry runtime.
  //
  // pool_modifier (Plan C wire field) is currently IGNORED on the JS side.
  // The popover's modifier-sum display is informational; to actually inject
  // a bonus into the dice, the GM should push the modifier card to the
  // sheet first via the existing GM Screen "push to Foundry" button —
  // RollFromDataset's selectors-based auto-apply will then pick it up.
  if (paths.length === 0) {
    // Rouse-style: zero basic dice + caller-supplied advanced dice (hunger /
    // rage). RollFromDataset can't represent an empty pool, so use direct
    // Roll for this narrow case (this is the original pre-Plan-C behavior).
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
