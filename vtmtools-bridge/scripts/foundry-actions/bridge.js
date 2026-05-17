// Foundry bridge.* helper executors.
//
// The `subscribers` registry maps a collection name → an object exposing
// `attach(socket)` and `detach()`. A subscriber is responsible for hooking
// the relevant Foundry Document hooks and pushing data over the socket.
//
// Phase 1 (Plan 0) ships the registry and the `actors` subscriber (after
// Task 12 refactor). Phase 5+ subscribers (journal, scene, item, chat,
// combat) will register themselves here when their consumer features land.

import { actorsSubscriber } from "./actor.js";
import { itemsSubscriber } from "./item.js";

const subscribers = {
  actors: actorsSubscriber,
  item: itemsSubscriber,
};

const active = new Map(); // collection -> attached subscriber
let currentSocket = null;

/** Called by bridge.js after the socket is open. Stores the socket so
 *  subscribe can hand it to the subscriber. */
export function setSocket(socket) {
  currentSocket = socket;
}

/** Called by bridge.js on close. Detaches all subscribers cleanly. */
export function clearAll() {
  for (const [_name, sub] of active) sub.detach();
  active.clear();
  currentSocket = null;
}

/** Subscribe handler. msg = { type: "bridge.subscribe", collection: "<name>" } */
export async function handleSubscribe(msg) {
  const collection = msg?.collection;
  if (!collection) throw new Error("missing_collection");
  const sub = subscribers[collection];
  if (!sub) throw new Error(`no_such_collection:${collection}`);
  if (active.has(collection)) return;
  if (!currentSocket) throw new Error("no_socket");
  sub.attach(currentSocket);
  active.set(collection, sub);
}

/** Unsubscribe handler. msg = { type: "bridge.unsubscribe", collection: "<name>" } */
export async function handleUnsubscribe(msg) {
  const collection = msg?.collection;
  if (!collection) throw new Error("missing_collection");
  const sub = active.get(collection);
  if (!sub) return;
  sub.detach();
  active.delete(collection);
}

export const handlers = {
  "bridge.subscribe": handleSubscribe,
  "bridge.unsubscribe": handleUnsubscribe,
};
