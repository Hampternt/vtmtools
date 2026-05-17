// Foundry storyteller.* helper executors.
//
// World-level operations not tied to a single actor. The storyteller.* umbrella
// is reserved by name in foundry helper roadmap §5; v1 milestone-4
// ships exactly one helper: storyteller.create_world_item (used by Library Sync
// push button to create a Foundry-world-level Item doc that lives in
// the world's Items sidebar / compendium, not embedded on an actor).

const MODULE_ID = "vtmtools-bridge";

/**
 * Create a world-level Item document.
 * Wire shape (validated Rust-side; see build_create_world_item):
 *   { type: "storyteller.create_world_item", name, featuretype, description, points }
 * Effect: Item.create({ type: "feature", name,
 *                       system: { featuretype, description, points } })
 *         at world level (no parent actor).
 * Failure modes: Foundry permission errors (GM-only — should not occur
 *                given the bridge runs in a GM session); duplicate name
 *                is NOT rejected (Foundry allows duplicate-name items).
 * Idempotency: NOT idempotent. Re-running creates a duplicate row.
 *              Dedup is Plan C's concern (auto-version-suffix on pull).
 */
async function createWorldItem(msg) {
  try {
    await Item.create({
      type: "feature",
      name: msg.name,
      system: {
        featuretype: msg.featuretype,
        description: msg.description ?? "",
        points: typeof msg.points === "number" ? msg.points : 0,
      },
    });
  } catch (err) {
    console.error(`[${MODULE_ID}] storyteller.create_world_item failed:`, err);
    ui.notifications?.error(`vtmtools: could not create world item: ${err?.message ?? err}`);
    throw err;
  }
}

export const handlers = {
  "storyteller.create_world_item": createWorldItem,
};
