// Foundry item.* subscriber executor.
//
// World-level Item docs only — embedded-on-actor items continue to flow
// through actorsSubscriber's per-actor enrichment. The filter is
// `item.parent === null` on every Foundry Item Document hook event.
//
// Wire emission shapes (consumed by Rust FoundryInbound; see
// src-tauri/src/bridge/foundry/types.rs):
//   { type: "items",                  items: [...] }    // snapshot on attach
//   { type: "world_item_upsert",      item: {...} }     // create OR update
//   { type: "world_item_deleted",     item_id: "..." }  // delete

const MODULE_ID = "vtmtools-bridge";

let _attached = null; // { socket, hookHandles: [ids] }

function itemToWire(item) {
  return {
    id: item.id,
    name: item.name,
    type: item.type,
    featuretype: item.system?.featuretype ?? null,
    system: item.system ?? {},
  };
}

function isWorldLevel(item) {
  // Foundry parents: world items have parent === null; embedded items
  // have parent === <Actor>.
  return item.parent === null || item.parent === undefined;
}

export const itemsSubscriber = {
  attach(socket) {
    if (_attached) return;
    if (socket?.readyState === WebSocket.OPEN) {
      const items = game.items.contents
        .filter(isWorldLevel)
        .map(itemToWire);
      socket.send(JSON.stringify({ type: "items", items }));
      console.log(`[${MODULE_ID}] itemsSubscriber: pushed ${items.length} world items`);
    }

    const onCreate = Hooks.on("createItem", (item /*, options, userId */) => {
      if (!isWorldLevel(item)) return;
      socket.send(JSON.stringify({ type: "world_item_upsert", item: itemToWire(item) }));
    });
    const onUpdate = Hooks.on("updateItem", (item /*, changes, options, userId */) => {
      if (!isWorldLevel(item)) return;
      socket.send(JSON.stringify({ type: "world_item_upsert", item: itemToWire(item) }));
    });
    const onDelete = Hooks.on("deleteItem", (item /*, options, userId */) => {
      if (!isWorldLevel(item)) return;
      socket.send(JSON.stringify({ type: "world_item_deleted", item_id: item.id }));
    });

    _attached = { socket, hookHandles: { createItem: onCreate, updateItem: onUpdate, deleteItem: onDelete } };
  },

  detach() {
    if (!_attached) return;
    Hooks.off("createItem", _attached.hookHandles.createItem);
    Hooks.off("updateItem", _attached.hookHandles.updateItem);
    Hooks.off("deleteItem", _attached.hookHandles.deleteItem);
    _attached = null;
  },
};
