// Flattens per-umbrella handler exports into one map for bridge.js::handleInbound.
import { handlers as actorHandlers } from "./actor.js";
import { handlers as bridgeHandlers } from "./bridge.js";
import { handlers as gameHandlers } from "./game.js";
import { handlers as storytellerHandlers } from "./storyteller.js";

export const handlers = {
  ...actorHandlers,
  ...bridgeHandlers,
  ...gameHandlers,
  ...storytellerHandlers,
};
