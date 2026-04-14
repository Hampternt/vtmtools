'use strict';

const WS_URL = 'ws://localhost:7423';
let ws = null;
let reconnectDelay = 1000;      // starts at 1s, doubles up to 30s
let reconnectTimer = null;

// Per-character debounce: collapses a burst of Firebase attrib add/change
// events into a single sendCharacterUpdate call per character.
const pendingUpdates = new Map();

function scheduleCharacterUpdate(model) {
  if (pendingUpdates.has(model.id)) {
    clearTimeout(pendingUpdates.get(model.id));
  }
  pendingUpdates.set(model.id, setTimeout(() => {
    pendingUpdates.delete(model.id);
    sendCharacterUpdate(model);
  }, 200));
}

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

function buildCharacter(model) {
  // model.attribs is a Backbone collection kept in sync with Firebase.
  // Using it directly avoids the legacy REST endpoint which only supports
  // numeric IDs and returns 404 for Jumpgate (Firebase push ID) games.
  const attribs = model.attribs?.models ?? [];
  return {
    id: model.id,
    name: model.get('name') ?? 'Unknown',
    controlled_by: model.get('controlledby') ?? '',
    attributes: attribs.map(a => ({
      name: a.get('name'),
      current: String(a.get('current') ?? ''),
      max: String(a.get('max') ?? ''),
    })),
  };
}

function readAllCharacters() {
  const models = window.Campaign?.characters?.models;
  if (!models || models.length === 0) {
    console.log('[vtmtools] No characters found in Campaign yet');
    return;
  }

  const characters = models.map(buildCharacter);
  sendToApp({ type: 'characters', characters });
  console.log(`[vtmtools] Sent ${characters.length} characters to app`);
}

function sendCharacterUpdate(model) {
  const character = buildCharacter(model);
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

function watchModel(model) {
  // Character-level changes (name, bio, etc.)
  model.on('change', () => scheduleCharacterUpdate(model));

  const attribs = model.attribs;
  if (!attribs) return;

  // Attribute changes arrive via Firebase child_added/child_changed events,
  // which Backbone exposes as add/change on attribs. Each attribute fires
  // separately, so we debounce via scheduleCharacterUpdate.
  attribs.on('add change remove', () => scheduleCharacterUpdate(model));

  // Roll20 Jumpgate lazily activates the Firebase attrib subscription only
  // when a character sheet is opened. Call fetch() ourselves to trigger that
  // subscription without needing to open the sheet UI.
  attribs.fetch();
}

function setupBackboneListeners() {
  const characters = window.Campaign?.characters;
  if (!characters) return;

  // Attach listeners to any characters already in the collection.
  characters.models.forEach(watchModel);

  // Listen for newly added characters (e.g. if GM adds one mid-session).
  characters.on('add', (model) => {
    watchModel(model);
    scheduleCharacterUpdate(model);
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
    setTimeout(() => waitForCampaign(retries + 1), 500);
  } else {
    console.warn('[vtmtools] Roll20 Campaign never became available after 60s.',
      'Campaign:', window.Campaign ? 'exists' : 'undefined',
      '| characters:', window.Campaign?.characters ? 'exists' : 'undefined'
    );
  }
}

waitForCampaign();
