// vtmtools Desktop Bridge
// Connects the Foundry GM browser session to a vtmtools Tauri app
// running on the same machine. Sends actor data on hooks, applies
// inbound updates through actor.update / createEmbeddedDocuments.

import { handlers } from "./foundry-actions/index.js";
import * as bridgeUmbrella from "./foundry-actions/bridge.js";
import { actorToWire } from "./translate.js";
import { init as initRollsHook } from "./foundry-hooks/rolls.js";

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
  // vtmtools roll mirror: subscribe to createChatMessage. The hook stays
  // registered for the world-session lifetime; teardown happens on world reload.
  initRollsHook(() => socket);
});

function connect() {
  if (socket) return;

  socket = new WebSocket(BRIDGE_URL);

  socket.addEventListener("open", async () => {
    console.log(`[${MODULE_ID}] connected to ${BRIDGE_URL}`);
    reconnectDelay = 1000;
    bridgeUmbrella.setSocket(socket);
    socket.send(JSON.stringify({
      type: "hello",
      protocol_version: 1,
      world_id: game.world?.id ?? null,
      world_title: game.world?.title ?? null,
      system_id: game.system?.id ?? null,
      system_version: game.system?.version ?? null,
      capabilities: ["actors"],
    }));
    // Auto-subscribe `actors` to preserve today's always-send-actors
    // semantics. Future tools may send `bridge.subscribe` for other
    // collections; the desktop never has to manage `actors`.
    try {
      await bridgeUmbrella.handleSubscribe({ collection: "actors" });
    } catch (err) {
      console.error(`[${MODULE_ID}] actors auto-subscribe failed:`, err);
    }
    updateStatusPip(true);
  });

  socket.addEventListener("message", (e) => {
    let msg;
    try { msg = JSON.parse(e.data); } catch { return; }
    handleInbound(msg);
  });

  socket.addEventListener("close", () => {
    bridgeUmbrella.clearAll();
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
  console.log(`[${MODULE_ID}] pushed ${actors.length} actors (refresh)`);
}

async function handleInbound(msg) {
  if (msg.type === "refresh") {
    pushAllActors();
    return;
  }
  const handler = handlers[msg.type];
  if (!handler) {
    console.warn(`[${MODULE_ID}] unknown inbound type:`, msg.type);
    return;
  }
  try {
    await handler(msg);
  } catch (err) {
    console.error(`[${MODULE_ID}] handler ${msg.type} threw:`, err);
    ui.notifications?.error(`vtmtools: ${msg.type} failed — ${err.message}`);
    // Send error envelope back to desktop.
    const code = err.message?.split(":")[0] || "unknown";
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({
        type: "error",
        refers_to: msg.type,
        request_id: null,
        code,
        message: String(err.message ?? err),
      }));
    }
  }
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
