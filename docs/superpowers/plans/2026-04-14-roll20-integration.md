# Roll20 Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the vtmtools desktop app to a live Roll20 VTM V5 game via a Chrome extension and local WebSocket, displaying all character stats in a new Campaign sidebar tab.

**Architecture:** A Chrome MV3 extension injects a content script into the Roll20 editor page; the script reads all characters via Roll20's REST API, subscribes to Backbone.js change events for live updates, and streams JSON over a WebSocket to `ws://localhost:7423`. The Tauri app hosts the WebSocket server using `tokio-tungstenite`, holds character state in memory, and pushes updates to the Svelte frontend via Tauri events.

**Tech Stack:** Rust (`tokio-tungstenite`, `futures-util`), Svelte 5 runes, TypeScript, Chrome MV3 extension (plain JS, no bundler), Tauri 2 event system (`listen` / `emit`).

---

## File Map

**Create:**
- `extension/manifest.json` — Chrome MV3 manifest; injects content script into `app.roll20.net/editor/*`; grants `host_permissions` for `ws://localhost:7423`
- `extension/background.js` — minimal MV3 service worker (required by Chrome but does nothing yet)
- `extension/content.js` — WebSocket client + Roll20 character reader + Backbone change listeners
- `extension/icons/` — placeholder icons (16/48/128px); Chrome works without them in dev mode
- `src-tauri/src/roll20/types.rs` — `Attribute`, `Character`, `Roll20Conn`, `InboundMsg`, `OutboundMsg` serde types
- `src-tauri/src/roll20/mod.rs` — `Roll20State` struct, `Roll20Conn` newtype, `start_ws_server` async fn, submodule declarations
- `src-tauri/src/roll20/commands.rs` — four Tauri commands: `get_roll20_characters`, `get_roll20_status`, `refresh_roll20_data`, `send_roll20_chat`
- `src/tools/Campaign.svelte` — new tool tab: Phase 1 (connection status + raw attr panel), Phase 2 (formatted stat cards)

**Modify:**
- `src-tauri/Cargo.toml` — add `tokio-tungstenite`, `futures-util`
- `src-tauri/src/lib.rs` — add `mod roll20`, start WS server in setup, register four new commands
- `src/types.ts` — add `Roll20Attribute`, `Roll20Character` TypeScript interfaces
- `src/tools.ts` — register Campaign tool entry

---

## Task 1: Add Rust dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add tokio-tungstenite and futures-util to Cargo.toml**

Open `src-tauri/Cargo.toml`. Add these two lines to the `[dependencies]` section (after the existing `tokio` line):

```toml
tokio-tungstenite = "0.24"
futures-util = "0.3"
```

- [ ] **Step 2: Verify dependencies resolve**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compiles (or errors only in existing code unrelated to this change). Cargo will download and compile the new crates. This may take a minute on first run.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add tokio-tungstenite and futures-util dependencies"
```

---

## Task 2: Define Roll20 Rust types

**Files:**
- Create: `src-tauri/src/roll20/types.rs`
- Create: `src-tauri/src/roll20/mod.rs` (stub only — full implementation in Task 3)

- [ ] **Step 1: Create types.rs**

Create `src-tauri/src/roll20/types.rs` with this exact content:

```rust
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub current: String,
    pub max: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: String,
    pub name: String,
    pub controlled_by: String,
    pub attributes: Vec<Attribute>,
}

/// Inbound messages from the browser extension.
/// `#[serde(tag = "type", rename_all = "snake_case")]` means the JSON field
/// `"type": "characters"` deserialises to the `Characters` variant.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InboundMsg {
    Characters { characters: Vec<Character> },
    CharacterUpdate { character: Character },
}

/// Outbound messages sent to the browser extension.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutboundMsg {
    Refresh,
    SendChat { message: String },
}

/// Shared in-memory state for the Roll20 connection.
pub struct Roll20State {
    pub characters: Mutex<HashMap<String, Character>>,
    pub connected: Mutex<bool>,
    /// Sender half of the channel used to push messages to the WebSocket.
    /// None when no extension is connected.
    pub outbound_tx: Mutex<Option<mpsc::Sender<String>>>,
}

impl Roll20State {
    pub fn new() -> Self {
        Self {
            characters: Mutex::new(HashMap::new()),
            connected: Mutex::new(false),
            outbound_tx: Mutex::new(None),
        }
    }
}

/// Newtype wrapper so Tauri's `.manage()` / `State<>` can hold the Arc.
pub struct Roll20Conn(pub Arc<Roll20State>);
```

- [ ] **Step 2: Create mod.rs stub**

Create `src-tauri/src/roll20/mod.rs` with this content (full implementation added in Task 3):

```rust
pub mod types;
pub mod commands;

pub use types::{Roll20Conn, Roll20State};
```

- [ ] **Step 3: Create commands.rs stub**

Create `src-tauri/src/roll20/commands.rs` with this content (full implementation added in Task 4):

```rust
// Commands implemented in Task 4
```

- [ ] **Step 4: Declare the roll20 module in lib.rs**

Open `src-tauri/src/lib.rs`. Add `mod roll20;` after the existing `mod db;` line:

```rust
mod shared;
mod tools;
mod db;
mod roll20;  // ← add this line
```

- [ ] **Step 5: Compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compiles. If you see "file not found for module `commands`", ensure `src-tauri/src/roll20/commands.rs` exists (even as a stub).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/roll20/
git add src-tauri/src/lib.rs
git commit -m "feat(roll20): define Roll20 types and module skeleton"
```

---

## Task 3: Implement the WebSocket server

**Files:**
- Modify: `src-tauri/src/roll20/mod.rs`

- [ ] **Step 1: Write the full mod.rs**

Replace the entire contents of `src-tauri/src/roll20/mod.rs` with:

```rust
pub mod types;
pub mod commands;

pub use types::{Roll20Conn, Roll20State};

use std::collections::HashMap;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tauri::{AppHandle, Emitter};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::roll20::types::{Character, InboundMsg};

/// Starts the WebSocket server on 127.0.0.1:7423.
/// Accepts one connection at a time. Emits Tauri events to the frontend:
///   - "roll20://connected"
///   - "roll20://disconnected"
///   - "roll20://characters-updated" with payload Vec<Character>
///
/// Spawn this with `tauri::async_runtime::spawn()` during app setup.
pub async fn start_ws_server(state: Arc<Roll20State>, handle: AppHandle) {
    let listener = TcpListener::bind("127.0.0.1:7423")
        .await
        .expect("Failed to bind WebSocket server on port 7423");

    loop {
        let (tcp_stream, _addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("[roll20] TCP accept error: {e}");
                continue;
            }
        };

        let ws_stream = match tokio_tungstenite::accept_async(tcp_stream).await {
            Ok(ws) => ws,
            Err(e) => {
                eprintln!("[roll20] WebSocket handshake error: {e}");
                continue;
            }
        };

        let (mut ws_sink, mut ws_source) = ws_stream.split();

        // Channel: Tauri commands → WebSocket outbound
        let (tx, mut rx) = mpsc::channel::<String>(32);
        *state.outbound_tx.lock().await = Some(tx);
        *state.connected.lock().await = true;
        let _ = handle.emit("roll20://connected", ());

        // Spawn a task to forward channel messages to the WebSocket sink.
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if ws_sink.send(Message::Text(msg)).await.is_err() {
                    break;
                }
            }
        });

        // Process inbound messages from the extension.
        while let Some(msg_result) = ws_source.next().await {
            let text = match msg_result {
                Ok(Message::Text(t)) => t,
                Ok(Message::Close(_)) | Err(_) => break,
                _ => continue,
            };

            match serde_json::from_str::<InboundMsg>(&text) {
                Ok(InboundMsg::Characters { characters }) => {
                    let map: HashMap<String, Character> = characters
                        .into_iter()
                        .map(|c| (c.id.clone(), c))
                        .collect();
                    *state.characters.lock().await = map;
                    let all = all_chars(&state).await;
                    let _ = handle.emit("roll20://characters-updated", all);
                }
                Ok(InboundMsg::CharacterUpdate { character }) => {
                    state
                        .characters
                        .lock()
                        .await
                        .insert(character.id.clone(), character);
                    let all = all_chars(&state).await;
                    let _ = handle.emit("roll20://characters-updated", all);
                }
                Err(e) => eprintln!("[roll20] Parse error: {e}  raw: {text}"),
            }
        }

        // Extension disconnected — clean up.
        *state.connected.lock().await = false;
        *state.outbound_tx.lock().await = None;
        let _ = handle.emit("roll20://disconnected", ());
    }
}

async fn all_chars(state: &Roll20State) -> Vec<Character> {
    state.characters.lock().await.values().cloned().collect()
}
```

- [ ] **Step 2: Compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compiles. Common errors:
- "cannot find type `AppHandle`" → check `use tauri::{AppHandle, Emitter};` is present
- "method `emit` not found" → the `Emitter` trait must be in scope; `use tauri::Emitter;` fixes it
- "use of undeclared crate `futures_util`" → check `futures-util = "0.3"` is in Cargo.toml (note: crate name uses underscore in `use`, hyphen in TOML)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/roll20/mod.rs
git commit -m "feat(roll20): implement WebSocket server with in-memory state"
```

---

## Task 4: Implement Tauri commands

**Files:**
- Modify: `src-tauri/src/roll20/commands.rs`

- [ ] **Step 1: Write commands.rs**

Replace the entire contents of `src-tauri/src/roll20/commands.rs` with:

```rust
use std::sync::Arc;
use tauri::State;

use crate::roll20::types::{Character, OutboundMsg, Roll20Conn};

/// Returns all characters currently known from the Roll20 session.
/// Returns an empty vec if no extension is connected.
#[tauri::command]
pub async fn get_roll20_characters(
    conn: State<'_, Roll20Conn>,
) -> Result<Vec<Character>, String> {
    let chars = conn.0.characters.lock().await;
    Ok(chars.values().cloned().collect())
}

/// Returns true if the browser extension is currently connected.
#[tauri::command]
pub async fn get_roll20_status(
    conn: State<'_, Roll20Conn>,
) -> Result<bool, String> {
    Ok(*conn.0.connected.lock().await)
}

/// Asks the extension to re-read all characters from Roll20.
/// No-op if no extension is connected.
#[tauri::command]
pub async fn refresh_roll20_data(
    conn: State<'_, Roll20Conn>,
) -> Result<(), String> {
    if let Some(tx) = conn.0.outbound_tx.lock().await.as_ref() {
        let msg = serde_json::to_string(&OutboundMsg::Refresh)
            .map_err(|e| e.to_string())?;
        tx.send(msg).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Sends a chat message into the Roll20 game via the extension.
/// No-op if no extension is connected.
#[tauri::command]
pub async fn send_roll20_chat(
    message: String,
    conn: State<'_, Roll20Conn>,
) -> Result<(), String> {
    if let Some(tx) = conn.0.outbound_tx.lock().await.as_ref() {
        let msg = serde_json::to_string(&OutboundMsg::SendChat { message })
            .map_err(|e| e.to_string())?;
        tx.send(msg).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
```

- [ ] **Step 2: Compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compiles. If you see "use of undeclared type `Arc`" in commands.rs, add `use std::sync::Arc;` (already in the code above — double-check the file was saved correctly).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/roll20/commands.rs
git commit -m "feat(roll20): implement Tauri commands for Roll20 state"
```

---

## Task 5: Wire roll20 module into lib.rs

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Read the current lib.rs**

Open `src-tauri/src/lib.rs`. You should see the existing setup with DB pool creation and `block_on`.

- [ ] **Step 2: Add Roll20 state and server spawn**

Inside the `tauri::async_runtime::block_on(async move { ... })` block, add the Roll20 setup **after** `handle.manage(DbState(Arc::new(pool)));`. The full updated lib.rs should look like this:

```rust
mod shared;
mod tools;
mod db;
mod roll20;

use sqlx::SqlitePool;
use std::sync::Arc;
use tauri::Manager;

pub struct DbState(pub Arc<SqlitePool>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let db_path = app_data_dir.join("vtmtools.db");
            let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async move {
                let pool = SqlitePool::connect(&db_url).await
                    .expect("Failed to connect to database");
                sqlx::migrate!("./migrations").run(&pool).await
                    .expect("Failed to run migrations");
                db::seed::seed_dyscrasias(&pool).await
                    .expect("Failed to seed dyscrasias");
                handle.manage(DbState(Arc::new(pool)));

                // Roll20 WebSocket integration
                let roll20_state = Arc::new(roll20::Roll20State::new());
                let roll20_state_for_ws = Arc::clone(&roll20_state);
                let handle_for_ws = handle.clone();
                handle.manage(roll20::Roll20Conn(roll20_state));
                tauri::async_runtime::spawn(
                    roll20::start_ws_server(roll20_state_for_ws, handle_for_ws)
                );
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            tools::resonance::roll_resonance,
            db::dyscrasia::list_dyscrasias,
            db::dyscrasia::add_dyscrasia,
            db::dyscrasia::update_dyscrasia,
            db::dyscrasia::delete_dyscrasia,
            db::dyscrasia::roll_random_dyscrasia,
            tools::export::export_result_to_md,
            roll20::commands::get_roll20_characters,
            roll20::commands::get_roll20_status,
            roll20::commands::refresh_roll20_data,
            roll20::commands::send_roll20_chat,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Compile-check**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compiles cleanly with no errors.

- [ ] **Step 4: Full build verification**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: `Finished dev [unoptimized + debuginfo] target(s)` with no errors.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat(roll20): wire WebSocket server and commands into Tauri app"
```

---

## Task 6: Add TypeScript types for Roll20 data

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Append Roll20 interfaces to types.ts**

Open `src/types.ts` and append these interfaces at the end of the file (after the existing `HistoryEntry` interface):

```typescript
export interface Roll20Attribute {
  name: string;
  current: string;
  max: string;
}

export interface Roll20Character {
  id: string;
  name: string;
  controlled_by: string;
  attributes: Roll20Attribute[];
}
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```

Expected: no new errors introduced by the type additions.

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat(roll20): add Roll20Character and Roll20Attribute TypeScript types"
```

---

## Task 7: Create Campaign.svelte — Phase 1 (connection status + raw attr panel)

**Files:**
- Create: `src/tools/Campaign.svelte`

This is Phase 1: proves the pipeline end-to-end. Each character shows its name, PC/NPC badge, and a toggleable raw attribute dump. No formatted stat display yet — that comes in Task 11 after verifying attribute names against a live Roll20 session.

- [ ] **Step 1: Create Campaign.svelte**

Create `src/tools/Campaign.svelte` with this content:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { Roll20Character } from '../types';

  let connected = $state(false);
  let characters = $state<Roll20Character[]>([]);
  let lastSync = $state<Date | null>(null);
  let expandedRaw = $state<Set<string>>(new Set());

  $effect(() => {
    invoke<boolean>('get_roll20_status').then(s => { connected = s; });
    invoke<Roll20Character[]>('get_roll20_characters').then(c => { characters = c; });

    const unlisteners = [
      listen<void>('roll20://connected', () => { connected = true; }),
      listen<void>('roll20://disconnected', () => { connected = false; }),
      listen<Roll20Character[]>('roll20://characters-updated', (e) => {
        characters = e.payload;
        lastSync = new Date();
      }),
    ];

    return () => { unlisteners.forEach(p => p.then(u => u())); };
  });

  function toggleRaw(id: string) {
    const next = new Set(expandedRaw);
    if (next.has(id)) { next.delete(id); } else { next.add(id); }
    expandedRaw = next;
  }

  function isPC(char: Roll20Character): boolean {
    return char.controlled_by.trim() !== '';
  }

  function timeSince(d: Date): string {
    const s = Math.floor((Date.now() - d.getTime()) / 1000);
    return s < 60 ? `${s}s ago` : `${Math.floor(s / 60)}m ago`;
  }

  function refresh() {
    invoke('refresh_roll20_data');
  }
</script>

<div class="campaign">
  <!-- Toolbar -->
  <div class="toolbar">
    <div class="status">
      <div class="status-dot" class:connected></div>
      {connected ? 'Connected to Roll20' : 'Not connected'}
    </div>
    {#if connected && lastSync}
      <span class="sync-time">last sync {timeSince(lastSync)}</span>
    {/if}
    <div class="spacer"></div>
    <button class="btn-refresh" onclick={refresh} disabled={!connected}>↺ Refresh</button>
  </div>

  {#if !connected}
    <!-- Disconnected banner -->
    <div class="disconnected-banner">
      <p class="banner-title">No Roll20 session detected</p>
      <p class="banner-body">
        Open your Roll20 game in Chrome with the vtmtools extension enabled.
        This panel connects automatically.
      </p>
    </div>
  {:else if characters.length === 0}
    <div class="disconnected-banner">
      <p class="banner-title">Connected — waiting for characters</p>
      <p class="banner-body">The extension is connected but no character data has arrived yet. Try clicking Refresh.</p>
    </div>
  {:else}
    <!-- Character grid -->
    <div class="char-grid">
      {#each characters as char (char.id)}
        <div class="char-card">
          <div class="card-header">
            <span class="char-name">{char.name}</span>
            <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
              {isPC(char) ? 'PC' : 'NPC'}
            </span>
          </div>

          <div class="card-footer">
            <button class="raw-toggle" onclick={() => toggleRaw(char.id)}>
              raw attrs {expandedRaw.has(char.id) ? '▴' : '▾'}
            </button>
          </div>

          {#if expandedRaw.has(char.id)}
            <div class="raw-panel">
              {#each char.attributes as attr}
                <div class="raw-row">
                  <span class="raw-name">{attr.name}</span>
                  <span class="raw-val">{attr.current}{attr.max ? ' / ' + attr.max : ''}</span>
                </div>
              {/each}
              {#if char.attributes.length === 0}
                <span class="raw-empty">No attributes loaded</span>
              {/if}
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .campaign {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    padding: 1.25rem;
    height: 100%;
    box-sizing: border-box;
  }

  /* Toolbar */
  .toolbar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.82rem;
    color: var(--text-secondary);
  }
  .status-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--text-ghost);
    flex-shrink: 0;
  }
  .status-dot.connected {
    background: #4caf50;
    box-shadow: 0 0 5px #4caf5066;
  }
  .sync-time {
    font-size: 0.72rem;
    color: var(--text-ghost);
  }
  .spacer { flex: 1; }
  .btn-refresh {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    color: var(--text-secondary);
    padding: 0.3rem 0.8rem;
    border-radius: 5px;
    font-size: 0.78rem;
    cursor: pointer;
    transition: border-color 0.15s;
  }
  .btn-refresh:hover:not(:disabled) { border-color: var(--accent); color: var(--text-primary); }
  .btn-refresh:disabled { opacity: 0.4; cursor: default; }

  /* Disconnected banner */
  .disconnected-banner {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 3rem 1rem;
    color: var(--text-ghost);
    text-align: center;
  }
  .banner-title { font-size: 0.9rem; color: var(--text-muted); }
  .banner-body { font-size: 0.78rem; line-height: 1.6; max-width: 280px; }

  /* Character grid */
  .char-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
    align-items: start;
  }

  /* Character card */
  .char-card {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 7px;
    overflow: hidden;
  }
  .char-card:hover { border-color: var(--border-surface); }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.7rem 0.9rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .char-name {
    flex: 1;
    font-size: 0.88rem;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .badge {
    font-size: 0.62rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .badge.pc { background: #2a1515; color: var(--accent); border: 1px solid #3a1e1e; }
  .badge.npc { background: #151528; color: #7986cb; border: 1px solid #1e1e3a; }

  .card-footer {
    padding: 0.3rem 0.9rem;
    display: flex;
    justify-content: flex-end;
  }
  .raw-toggle {
    font-size: 0.65rem;
    color: var(--text-ghost);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.1rem 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .raw-toggle:hover { color: var(--text-muted); }

  /* Raw attribute dump */
  .raw-panel {
    border-top: 1px solid var(--border-faint);
    padding: 0.5rem 0.9rem;
    max-height: 180px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .raw-row {
    display: flex;
    gap: 0.5rem;
    font-size: 0.7rem;
    font-family: monospace;
    line-height: 1.7;
  }
  .raw-name { color: var(--text-muted); flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .raw-val { color: var(--accent); flex-shrink: 0; }
  .raw-empty { font-size: 0.7rem; color: var(--text-ghost); font-style: italic; }
</style>
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```

Expected: no errors. If you see "Property 'listen' does not exist", check that `@tauri-apps/api` v2 is installed: `npm list @tauri-apps/api`.

- [ ] **Step 3: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(roll20): add Campaign tool tab — Phase 1 raw data panel"
```

---

## Task 8: Register Campaign tool in tools.ts

**Files:**
- Modify: `src/tools.ts`

- [ ] **Step 1: Add Campaign entry**

Open `src/tools.ts`. Add the Campaign entry to the `tools` array:

```typescript
export const tools: Tool[] = [
  {
    id: 'resonance',
    label: 'Resonance Roller',
    icon: '🩸',
    component: () => import('./tools/Resonance.svelte'),
  },
  {
    id: 'dyscrasias',
    label: 'Dyscrasias',
    icon: '📋',
    component: () => import('./tools/DyscrasiaManager.svelte'),
  },
  {
    id: 'campaign',
    label: 'Campaign',
    icon: '🗺️',
    component: () => import('./tools/Campaign.svelte'),
  },
];
```

- [ ] **Step 2: Type-check**

```bash
npm run check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/tools.ts
git commit -m "feat(roll20): register Campaign tab in tool registry"
```

---

## Task 9: Create the browser extension

**Files:**
- Create: `extension/manifest.json`
- Create: `extension/background.js`

- [ ] **Step 1: Create extension directory**

```bash
mkdir -p extension/icons
```

- [ ] **Step 2: Create manifest.json**

Create `extension/manifest.json`:

```json
{
  "manifest_version": 3,
  "name": "vtmtools Roll20 Bridge",
  "version": "0.1.0",
  "description": "Connects vtmtools desktop app to a Roll20 VTM V5 game session.",

  "permissions": [],
  "host_permissions": [
    "https://app.roll20.net/*",
    "ws://localhost:7423/*"
  ],

  "background": {
    "service_worker": "background.js"
  },

  "content_scripts": [
    {
      "matches": ["https://app.roll20.net/editor/*"],
      "js": ["content.js"],
      "run_at": "document_idle"
    }
  ],

  "icons": {
    "16": "icons/icon16.png",
    "48": "icons/icon48.png",
    "128": "icons/icon128.png"
  }
}
```

**Note:** Chrome won't error if the icon files are missing during development — it just shows a default puzzle piece. You can skip creating icons and come back to it later.

- [ ] **Step 3: Create background.js**

Create `extension/background.js`:

```javascript
// Minimal MV3 service worker.
// Chrome requires a service_worker entry in manifest.json.
// Future use: extension options page, badge updates.
chrome.runtime.onInstalled.addListener(() => {
  console.log('[vtmtools] Extension installed.');
});
```

- [ ] **Step 4: Commit**

```bash
git add extension/
git commit -m "feat(roll20): add Chrome extension manifest and service worker"
```

---

## Task 10: Implement content script — WebSocket connection and initial character read

**Files:**
- Create: `extension/content.js`

- [ ] **Step 1: Create content.js**

Create `extension/content.js`:

```javascript
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
  if (window.Campaign?.characters?.models) {
    connect();
    setupBackboneListeners();
  } else if (retries < 30) {
    // Retry up to 30 times × 500ms = 15 seconds
    setTimeout(() => waitForCampaign(retries + 1), 500);
  } else {
    console.warn('[vtmtools] Roll20 Campaign never became available');
  }
}

waitForCampaign();
```

- [ ] **Step 2: Commit**

```bash
git add extension/content.js
git commit -m "feat(roll20): implement content script with WS connection and character read"
```

---

## Task 11: End-to-end integration test

No automated test suite exists. Verify the full pipeline manually.

- [ ] **Step 1: Start the Tauri app in dev mode**

```bash
npm run tauri dev
```

Wait for the app to launch. You should see the "Campaign" tab in the sidebar.

- [ ] **Step 2: Load the extension in Chrome**

1. Open `chrome://extensions`
2. Enable "Developer mode" (toggle top-right)
3. Click "Load unpacked"
4. Select the `extension/` directory in the vtmtools project

You should see "vtmtools Roll20 Bridge" appear in the list.

- [ ] **Step 3: Open a Roll20 game**

Navigate to `https://app.roll20.net/editor/` with a VTM V5 campaign that has characters with filled attributes.

- [ ] **Step 4: Verify connection**

In vtmtools, click "Campaign" tab. Expected:
- Status dot turns green
- "Connected to Roll20" text appears

If still disconnected after ~10 seconds, open the browser console (F12) on the Roll20 page and look for `[vtmtools]` log lines. Common issues:
- "WebSocket connection to 'ws://localhost:7423' failed" → vtmtools app is not running
- No logs at all → extension is not loaded or not matched to the URL

- [ ] **Step 5: Verify characters appear**

Expected: character cards appear with each character's name and PC/NPC badge.

- [ ] **Step 6: Verify raw panel**

Click "raw attrs ▾" on any character card. Expected: a scrollable list of attribute name/value pairs from Roll20.

**Important:** Look at the raw attributes for your characters. Note the exact names used for hunger, health, willpower, humanity, and blood_potency. You will need these exact names for Task 12 (formatted stat cards). The names may look like: `hunger`, `health`, `willpower`, `humanity`, `blood_potency` — or they may include prefixes or suffixes depending on which Roll20 VTM V5 sheet template you are using.

- [ ] **Step 7: Verify live updates**

Have a player change their Hunger on their character sheet in Roll20. Within a few seconds the raw panel in vtmtools should show the updated value. If not, the Backbone `change` event may not be firing for attribute-level changes (the attribute collection may be separate). Click "↺ Refresh" to manually re-read all data — this always works reliably.

---

## Task 12: Phase 2 — Formatted character stat cards

**Pre-requisite:** Complete Task 11. You need the exact attribute names from your Roll20 sheet before proceeding. This task assumes the standard VTM V5 Roll20 sheet attribute names listed below. If your sheet uses different names, update the `ATTR` constants at the top of the script block.

**Files:**
- Modify: `src/tools/Campaign.svelte`

- [ ] **Step 1: Replace the Campaign.svelte script block**

Replace the `<script lang="ts">` block in `src/tools/Campaign.svelte` with:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import type { Roll20Character, Roll20Attribute } from '../types';

  // ── Attribute name constants ────────────────────────────────────────────
  // Update these if your Roll20 sheet uses different names.
  // Find the correct names using the raw attr panel in Phase 1.
  const ATTR = {
    hunger:        'hunger',
    health:        'health',
    healthMax:     'health_max',
    healthAgg:     'health_aggravated',
    willpower:     'willpower',
    willpowerMax:  'willpower_max',
    humanity:      'humanity',
    bloodPotency:  'blood_potency',
  } as const;

  // ── State ───────────────────────────────────────────────────────────────
  let connected = $state(false);
  let characters = $state<Roll20Character[]>([]);
  let lastSync = $state<Date | null>(null);
  let expandedRaw = $state<Set<string>>(new Set());

  $effect(() => {
    invoke<boolean>('get_roll20_status').then(s => { connected = s; });
    invoke<Roll20Character[]>('get_roll20_characters').then(c => { characters = c; });

    const unlisteners = [
      listen<void>('roll20://connected', () => { connected = true; }),
      listen<void>('roll20://disconnected', () => { connected = false; }),
      listen<Roll20Character[]>('roll20://characters-updated', (e) => {
        characters = e.payload;
        lastSync = new Date();
      }),
    ];

    return () => { unlisteners.forEach(p => p.then(u => u())); };
  });

  // ── Helpers ─────────────────────────────────────────────────────────────
  function attr(attributes: Roll20Attribute[], name: string): number {
    const a = attributes.find(a => a.name === name);
    return a ? (parseInt(a.current, 10) || 0) : 0;
  }

  function attrMax(attributes: Roll20Attribute[], name: string, fallback: number): number {
    const a = attributes.find(a => a.name === name);
    if (a && a.max) return parseInt(a.max, 10) || fallback;
    return fallback;
  }

  function toggleRaw(id: string) {
    const next = new Set(expandedRaw);
    if (next.has(id)) { next.delete(id); } else { next.add(id); }
    expandedRaw = next;
  }

  function isPC(char: Roll20Character): boolean {
    return char.controlled_by.trim() !== '';
  }

  function timeSince(d: Date): string {
    const s = Math.floor((Date.now() - d.getTime()) / 1000);
    return s < 60 ? `${s}s ago` : `${Math.floor(s / 60)}m ago`;
  }

  function refresh() {
    invoke('refresh_roll20_data');
  }

  // Build arrays for dot/box tracks
  function dots(filled: number, total: number): boolean[] {
    return Array.from({ length: total }, (_, i) => i < filled);
  }
</script>
```

- [ ] **Step 2: Replace the character grid section in the template**

Find the `<!-- Character grid -->` section inside the `{:else}` block and replace it with:

```svelte
    <!-- Character grid -->
    <div class="char-grid">
      {#each characters as char (char.id)}
        {@const hunger      = attr(char.attributes, ATTR.hunger)}
        {@const health      = attr(char.attributes, ATTR.health)}
        {@const healthMax   = attrMax(char.attributes, ATTR.healthMax, health)}
        {@const healthAgg   = attr(char.attributes, ATTR.healthAgg)}
        {@const willpower   = attr(char.attributes, ATTR.willpower)}
        {@const wpMax       = attrMax(char.attributes, ATTR.willpowerMax, willpower)}
        {@const humanity    = attr(char.attributes, ATTR.humanity)}
        {@const bp          = attr(char.attributes, ATTR.bloodPotency)}

        <div class="char-card">
          <div class="card-header">
            <span class="char-name">{char.name}</span>
            <span class="badge" class:pc={isPC(char)} class:npc={!isPC(char)}>
              {isPC(char) ? 'PC' : 'NPC'}
            </span>
          </div>

          <div class="card-body">
            <!-- Hunger (1–5 dots, crimson) -->
            <div class="stat-row">
              <span class="stat-label">Hunger</span>
              <div class="track">
                {#each dots(hunger, 5) as filled}
                  <div class="dot" class:hunger={filled}></div>
                {/each}
              </div>
            </div>

            <!-- Health (boxes; aggravated shown as striped) -->
            <div class="stat-row">
              <span class="stat-label">Health</span>
              <div class="track">
                {#each Array.from({ length: healthMax }, (_, i) => i) as i}
                  <div
                    class="box"
                    class:filled={i < health - healthAgg}
                    class:aggravated={i >= health - healthAgg && i < health}
                  ></div>
                {/each}
              </div>
            </div>

            <!-- Willpower (boxes, indigo) -->
            <div class="stat-row">
              <span class="stat-label">Willpower</span>
              <div class="track">
                {#each Array.from({ length: wpMax }, (_, i) => i) as i}
                  <div class="box willpower" class:filled={i < willpower}></div>
                {/each}
              </div>
            </div>

            <!-- Humanity (1–10 dots, indigo) -->
            <div class="stat-row">
              <span class="stat-label">Humanity</span>
              <div class="track">
                {#each dots(humanity, 10) as filled}
                  <div class="dot" class:humanity={filled}></div>
                {/each}
              </div>
            </div>

            <!-- Blood Potency (single number, amber) -->
            <div class="stat-row">
              <span class="stat-label">Blood Potency</span>
              <span class="bp-value">{bp}</span>
            </div>
          </div>

          <div class="card-footer">
            <button class="raw-toggle" onclick={() => toggleRaw(char.id)}>
              raw attrs {expandedRaw.has(char.id) ? '▴' : '▾'}
            </button>
          </div>

          {#if expandedRaw.has(char.id)}
            <div class="raw-panel">
              {#each char.attributes as a}
                <div class="raw-row">
                  <span class="raw-name">{a.name}</span>
                  <span class="raw-val">{a.current}{a.max ? ' / ' + a.max : ''}</span>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {/each}
    </div>
```

- [ ] **Step 3: Append stat display styles to the `<style>` block**

Add these rules inside the existing `<style>` block (after the `.raw-empty` rule):

```css
  /* Stat rows */
  .card-body {
    padding: 0.65rem 0.9rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .stat-row {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .stat-label {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-ghost);
    font-weight: 600;
  }
  .track {
    display: flex;
    gap: 3px;
    flex-wrap: wrap;
  }

  /* Dots (Hunger, Humanity) */
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    border: 1px solid var(--border-surface);
    background: transparent;
  }
  .dot.hunger {
    background: var(--accent);
    border-color: var(--accent);
    box-shadow: 0 0 4px color-mix(in srgb, var(--accent) 50%, transparent);
  }
  .dot.humanity {
    background: #7986cb;
    border-color: #7986cb;
  }

  /* Boxes (Health, Willpower) */
  .box {
    width: 11px;
    height: 11px;
    border: 1px solid var(--border-surface);
    border-radius: 2px;
    background: transparent;
  }
  .box.filled {
    background: var(--accent);
    border-color: var(--accent);
  }
  .box.willpower.filled {
    background: #7986cb;
    border-color: #7986cb;
  }
  .box.aggravated {
    border-color: var(--border-surface);
    background-image: repeating-linear-gradient(
      45deg,
      var(--accent) 0,
      var(--accent) 1px,
      transparent 0,
      transparent 50%
    );
    background-size: 4px 4px;
  }

  /* Blood Potency */
  .bp-value {
    font-size: 1.05rem;
    font-weight: 700;
    color: var(--accent-amber);
  }
```

- [ ] **Step 4: Type-check**

```bash
npm run check
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(roll20): Phase 2 — formatted stat cards with Hunger, Health, Willpower, Humanity"
```

---

## Task 13: Final verification

- [ ] **Step 1: Run the full app**

```bash
npm run tauri dev
```

- [ ] **Step 2: Confirm Phase 2 stat cards render**

Open Roll20 in Chrome with a character that has attributes filled in. In vtmtools Campaign tab, confirm:
- Hunger shows correct number of crimson dots (1–5)
- Health shows correct filled/aggravated boxes
- Willpower shows indigo boxes
- Humanity shows indigo dots (1–10)
- Blood Potency shows amber number

If stats show all empty (all dots/boxes unfilled), the attribute names in the `ATTR` constants don't match your Roll20 sheet. Open the raw panel to find the correct names and update the `ATTR` object at the top of Campaign.svelte's script block.

- [ ] **Step 3: Final commit**

```bash
git add -p   # review any remaining changes
git commit -m "feat(roll20): complete Roll20 integration — Campaign tool, extension, WebSocket bridge"
```
