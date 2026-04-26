# vtmtools Desktop Bridge

A Foundry VTT module that mirrors actor data into the [vtmtools](../) desktop app over WebSocket. Designed for GMs running WoD5e (Vampire: The Masquerade 5th Edition) who use vtmtools alongside Foundry for resonance rolls, combat tracking, and character mirrors.

The bridge runs **only in the GM's browser session**. Players' browsers never connect to your desktop app.

## Install

1. In Foundry's setup screen, go to **Add-on Modules → Install Module**.
2. Paste this manifest URL into the **Manifest URL** field:
   ```
   https://github.com/Hampternt/vtmtools/releases/latest/download/module.json
   ```
3. Click **Install**.
4. Enable the module in your world (**Manage Modules → vtmtools Desktop Bridge**).

## First-time cert acceptance

The bridge dials `wss://localhost:7424` on the GM's machine. Because Foundry is commonly served over HTTPS (Forge, Molten, any TLS-proxied self-host), the browser blocks plain `ws://` to localhost — so the desktop app ships a self-signed certificate for `localhost`. **You only do this once per browser:**

1. Make sure the vtmtools desktop app is running (it generates the cert on first launch).
2. Open a new tab in the same browser you use for Foundry.
3. Visit **<https://localhost:7424>**.
4. The browser will warn about an untrusted certificate. Click **Advanced** → **Proceed to localhost (unsafe)**.
5. You can close the tab. The browser now trusts the cert for the session.
6. Reload your Foundry world. The module should connect — look for the green pip in the player list footer.

If the module shows a "could not reach desktop app" warning, repeat step 3.

## What it does

- On world ready (GM only): pushes the full list of actors to the desktop app.
- On `updateActor` / `createActor` / `deleteActor` hooks: pushes the changed actor.
- On inbound messages from the desktop app:
  - `update_actor` → calls `actor.update({ <path>: <value> })` (e.g. setting hunger).
  - `create_item` → creates an Item document on the actor (used for resonance, since WoD5e stores it as an Item, not a system field).
  - `refresh` → re-pushes all actors.

## Status pip

A small dot appears at the bottom of the Foundry player list:

- 🔴 red — disconnected (desktop app not running, or cert not yet accepted)
- 🟢 green — connected

## Troubleshooting

**"could not reach desktop app on wss://localhost:7424"**
The desktop app isn't running, or the cert hasn't been accepted yet. See "First-time cert acceptance" above.

**Connects, but actors don't appear in the desktop app**
Check the browser console (F12) for `[vtmtools-bridge]` messages. The `[vtmtools-bridge] pushed N actors` log means the data was sent.

**Players see the warning toast about cert acceptance**
The module only initializes for GMs. If a non-GM sees this, the GM check is failing — file a bug.

## Compatibility

- Foundry V12+, verified through V14.
- WoD5e system v5.x.
- Other systems work for actor mirroring, but the apply-attribute path (hunger, humanity, etc.) assumes the WoD5e schema documented in [`docs/reference/foundry-vtm5e-paths.md`](../docs/reference/foundry-vtm5e-paths.md).
