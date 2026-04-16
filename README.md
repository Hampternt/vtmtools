# VTM Tools

A desktop companion app for **Vampire: The Masquerade 5th Edition** Storytellers and players. It handles the fiddly dice mechanics and bookkeeping so you can focus on the story.

## What's Inside

### Resonance Roller

Roll resonance for feeding scenes in one click. You configure the dice pool, pick whether to take the highest or lowest die, and adjust how likely each resonance type is. The app rolls temperament, picks a resonance type (Phlegmatic, Melancholy, Choleric, or Sanguine), and checks for Acute results — all following V5 rules. If a result is Acute, it automatically pulls up a matching dyscrasia.

Results can be exported to Markdown files saved to your Documents folder, and if you have the Roll20 integration connected, you can write resonance results straight back to your character sheet.

### Dyscrasias

Browse, search, and filter the full list of V5 dyscrasias. Every built-in dyscrasia from the sourcebook is included and kept up to date automatically. You can also add your own custom dyscrasias for homebrew games. Filter by resonance type, search by name or effect, and roll a random dyscrasia when you need one on the fly.

### Campaign (Roll20 Live View)

See your Roll20 characters' stats in real time — hunger, health, willpower, humanity, and blood potency — without switching browser tabs. The app connects to Roll20 through a small browser extension and keeps everything synced live. Handy for Storytellers who want a quick glance at the whole coterie during a session.

## Getting Started

1. **Download the app** — Head to the [Releases](../../releases) page and grab the latest version for your system.
2. **Install and run** — Open the installer and follow the prompts. The app is ready to use right away for resonance rolls and dyscrasias.

### Setting Up Roll20 Integration (Optional)

The Roll20 connection lets the app read character data live from your Roll20 game. It requires a small Chrome extension:

1. Download the `extension` folder from this repository (or from the release).
2. In Chrome, go to `chrome://extensions`.
3. Turn on **Developer mode** (toggle in the top-right corner).
4. Click **Load unpacked** and select the `extension` folder you downloaded.
5. Open your Roll20 game in Chrome — the extension connects automatically.
6. In the app, open the **Campaign** tool to see your characters.

The extension only works while your Roll20 game tab is open. If you close the tab, the app will show it as disconnected and reconnect when you reopen it.

## Future Planned Tools & Expansions

These are on the roadmap but not built yet. Details will change as development happens.

- **Conflict Tracker** — Track combat and social conflicts round by round. Manage initiative, track damage, and keep the action moving without losing your place.
- **Character Builder** — Walk through character creation step by step. Pick clan, attributes, skills, and disciplines with the rules handled for you.
- **NPC Builder** — Quickly spin up NPCs for your chronicle. Generate stat blocks and notes so you always have someone ready when players go off-script.
- **Merits & Flaws Library** — Browse and search all V5 merits and flaws in one place. Filter by type, see descriptions, and reference them during character creation or play.

