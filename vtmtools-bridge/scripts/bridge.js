// vtmtools Desktop Bridge
// Connects the Foundry GM browser session to a vtmtools Tauri app
// running on the same machine. Sends actor data on hooks, applies
// inbound updates through actor.update / createEmbeddedDocuments.

import { actorToWire, hookActorChanges } from "./translate.js";

const BRIDGE_URL = "wss://localhost:7424";
const MODULE_ID = "vtmtools-bridge";

let socket = null;
let reconnectDelay = 1000;
let reconnectTimer = null;

Hooks.once("ready", async () => {
  if (!game.user.isGM) {
    console.log(`[${MODULE_ID}] non-GM session, bridge stays closed`);
    return;
  }
  connect();
});

function connect() {
  if (socket) return;

  socket = new WebSocket(BRIDGE_URL);

  socket.addEventListener("open", () => {
    console.log(`[${MODULE_ID}] connected to ${BRIDGE_URL}`);
    reconnectDelay = 1000;
    socket.send(JSON.stringify({ type: "hello" }));
    pushAllActors();
    hookActorChanges(socket);
    updateStatusPip(true);
  });

  socket.addEventListener("message", (e) => {
    let msg;
    try { msg = JSON.parse(e.data); } catch { return; }
    handleInbound(msg);
  });

  socket.addEventListener("close", () => {
    socket = null;
    updateStatusPip(false);
    console.log(`[${MODULE_ID}] disconnected — retrying in ${reconnectDelay}ms`);
    if (reconnectTimer) clearTimeout(reconnectTimer);
    reconnectTimer = setTimeout(connect, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
  });

  socket.addEventListener("error", () => {
    // 'close' fires after 'error' — let it handle cleanup. If this is the
    // first connection, surface the cert hint.
    if (reconnectDelay === 1000) {
      ui.notifications?.warn(
        `${MODULE_ID}: could not reach desktop app on ${BRIDGE_URL}. ` +
        `If this is the first connection, visit https://localhost:7424 in this ` +
        `browser and accept the certificate warning. See module README.`,
        { permanent: true }
      );
    }
  });
}

function pushAllActors() {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  const actors = game.actors.contents.map(actorToWire);
  socket.send(JSON.stringify({ type: "actors", actors }));
  console.log(`[${MODULE_ID}] pushed ${actors.length} actors`);
}

async function handleInbound(msg) {
  if (msg.type === "refresh") {
    pushAllActors();
    return;
  }
  if (msg.type === "update_actor") {
    const actor = game.actors.get(msg.actor_id);
    if (!actor) return;
    await actor.update({ [msg.path]: msg.value });
    return;
  }
  if (msg.type === "create_item") {
    const actor = game.actors.get(msg.actor_id);
    if (!actor) return;
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
    return;
  }
  if (msg.type === "apply_dyscrasia") {
    const actor = game.actors.get(msg.actor_id);
    if (!actor) return;

    // (1) Delete prior dyscrasia merits. Filter is name-prefix-based —
    // any feature Item with featuretype="merit" whose name starts with
    // "Dyscrasia: " is treated as tool-managed and clobbered. Documented
    // limitation in spec §2.
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

    // (2) Create the new dyscrasia merit.
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

    // (3) Append timestamped audit line to private notes. Empty notes →
    // bare line; existing content → newline-prefixed append.
    const current = actor.system?.privatenotes ?? "";
    const next =
      current.trim() === ""
        ? msg.notes_line
        : `${current}\n${msg.notes_line}`;
    await actor.update({ "system.privatenotes": next });
    return;
  }
  console.warn(`[${MODULE_ID}] unknown inbound type:`, msg.type);
}

// Tiny status pip in the player list footer so the GM sees connection
// state without opening the console. Filled in next to README.
function updateStatusPip(connected) {
  const el = document.querySelector(`.${MODULE_ID}-pip`);
  if (!el) return;
  el.classList.toggle("connected", connected);
  el.title = connected ? "vtmtools: connected" : "vtmtools: disconnected";
}

Hooks.on("renderPlayerList", (_app, html) => {
  const root = html instanceof HTMLElement ? html : html[0];
  if (!root || root.querySelector(`.${MODULE_ID}-pip`)) return;
  const pip = document.createElement("div");
  pip.className = `${MODULE_ID}-pip`;
  pip.title = socket?.readyState === WebSocket.OPEN
    ? "vtmtools: connected"
    : "vtmtools: disconnected";
  if (socket?.readyState === WebSocket.OPEN) pip.classList.add("connected");
  root.appendChild(pip);
});
