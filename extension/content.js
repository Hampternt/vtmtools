'use strict';

const WS_URL = 'ws://localhost:7423';
let ws = null;
let reconnectDelay = 1000;      // starts at 1s, doubles up to 30s
let reconnectTimer = null;

// ── WebSocket lifecycle ──────────────────────────────────────────────────────

function connect() {
  if (ws) return;

  ws = new WebSocket(WS_URL);

  ws.addEventListener('open', () => {
    console.log('[vtmtools] Connected to desktop app');
    reconnectDelay = 1000;
    readAllCharacters();
  });

  ws.addEventListener('message', (event) => {
    try {
      const msg = JSON.parse(event.data);
      if (msg.type === 'refresh') {
        readAllCharacters();
      } else if (msg.type === 'send_chat' && msg.message) {
        sendChat(msg.message);
      }
    } catch (e) {
      console.warn('[vtmtools] Failed to parse message from app:', e);
    }
  });

  ws.addEventListener('close', () => {
    ws = null;
    console.log(`[vtmtools] Disconnected — reconnecting in ${reconnectDelay}ms`);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = null;
      connect();
    }, reconnectDelay);
    reconnectDelay = Math.min(reconnectDelay * 2, 30_000);
  });

  ws.addEventListener('error', () => {
    // The 'close' event always fires after 'error', so cleanup is handled there.
  });
}

function sendToApp(payload) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(payload));
  }
}

// ── Roll20 data reading ──────────────────────────────────────────────────────

async function fetchAttributes(charId) {
  try {
    const res = await fetch(`/character/${charId}/attributes`, {
      credentials: 'same-origin',
    });
    if (!res.ok) return [];
    return await res.json();
  } catch (e) {
    console.warn(`[vtmtools] Failed to fetch attributes for ${charId}:`, e);
    return [];
  }
}

async function buildCharacter(model) {
  const rawAttrs = await fetchAttributes(model.id);
  return {
    id: model.id,
    name: model.get('name') ?? 'Unknown',
    controlled_by: model.get('controlledby') ?? '',
    attributes: rawAttrs.map(a => ({
      name: a.name,
      current: String(a.current ?? ''),
      max: String(a.max ?? ''),
    })),
  };
}

async function readAllCharacters() {
  const models = window.Campaign?.characters?.models;
  if (!models || models.length === 0) {
    console.log('[vtmtools] No characters found in Campaign yet');
    return;
  }

  const characters = await Promise.all(models.map(buildCharacter));
  sendToApp({ type: 'characters', characters });
  console.log(`[vtmtools] Sent ${characters.length} characters to app`);
}

async function sendCharacterUpdate(model) {
  const character = await buildCharacter(model);
  sendToApp({ type: 'character_update', character });
}

function sendChat(message) {
  // Roll20's global chat input function. Available when the editor is loaded.
  if (typeof d20?.textchat?.doChatInput === 'function') {
    d20.textchat.doChatInput(message);
  } else {
    console.warn('[vtmtools] d20.textchat.doChatInput not available');
  }
}

// ── Backbone change listeners ────────────────────────────────────────────────

function setupBackboneListeners() {
  const characters = window.Campaign?.characters;
  if (!characters) return;

  // Listen for changes on existing character models.
  characters.models.forEach(model => {
    model.on('change', () => sendCharacterUpdate(model));
  });

  // Listen for newly added characters (e.g. if GM adds one mid-session).
  characters.on('add', (model) => {
    model.on('change', () => sendCharacterUpdate(model));
    sendCharacterUpdate(model);
  });

  console.log(
    `[vtmtools] Backbone listeners set on ${characters.models.length} characters`
  );
}

// ── Startup: wait for Roll20 Campaign to initialise ─────────────────────────
// Roll20 loads asynchronously. window.Campaign.characters may not be populated
// immediately when the content script runs. Poll until it's ready.

function waitForCampaign(retries = 0) {
  const chars = window.Campaign?.characters;
  if (chars?.models) {
    console.log('[vtmtools] Campaign ready, models:', chars.models.length);
    connect();
    setupBackboneListeners();
  } else if (retries < 120) {
    // Retry up to 120 times × 500ms = 60 seconds
    if (retries === 20) {
      // After 10s, log what we can see to help debug
      console.log('[vtmtools] Still waiting... Campaign:',
        window.Campaign ? 'exists' : 'undefined',
        '| characters:', chars ? 'exists' : 'undefined',
        '| models:', chars?.models !== undefined ? JSON.stringify(chars.models) : 'undefined'
      );
    }
    setTimeout(() => waitForCampaign(retries + 1), 500);
  } else {
    console.warn('[vtmtools] Roll20 Campaign never became available after 60s.',
      'Campaign:', window.Campaign ? 'exists' : 'undefined',
      '| characters:', window.Campaign?.characters ? 'exists' : 'undefined'
    );
  }
}

waitForCampaign();
