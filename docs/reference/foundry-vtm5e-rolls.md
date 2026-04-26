# Foundry WoD5e — Rolls API

Roll-system reference for the [WoD5e Foundry system](https://github.com/WoD5E-Developers/wod5e), pinned to commit `d16b5d960a` (v5.3.17). Companion to [`foundry-vtm5e-paths.md`](./foundry-vtm5e-paths.md) (actor schema) and [`foundry-vtm5e-actor-sample.json`](./foundry-vtm5e-actor-sample.json) (live wire shape).

This doc describes how to **trigger a roll**, **read a roll result**, and **listen for rolls** so vtmtools can later integrate live dice mirrors and outbound roll triggers.

---

## Public API surface

WoD5e exposes its API on `window.WOD5E.api` (set in `system/main.js:64-75`). The full surface:

| Function | Purpose |
|---|---|
| `WOD5E.api.Roll(opts)` | Direct roll — caller supplies `basicDice` and `advancedDice` counts |
| `WOD5E.api.RollFromDataset({ dataset, actor })` | Higher-level — caller supplies skill/attribute/discipline names; system computes the pool and prompts a dialog |
| `WOD5E.api.PromptRoll({ actor })` | Opens the "Select Roll" dialog with no preset (full GM-driven choice) |
| `WOD5E.api.getBasicDice({ valuePaths, flatMod, actor })` | Sums actor data at given dot-paths into a basic-dice count |
| `WOD5E.api.getAdvancedDice({ actor })` | Auto-derives advanced dice: hunger for vampires, rage for werewolves, 0 for mortals/ghouls |
| `WOD5E.api.generateLabelAndLocalize({ string, type })` | i18n helper — translates an internal name (e.g. `"brawl"`, type `"skills"`) to its localized label |

All methods are static. The class is `wod5eAPI` in `system/api/wod5e-api.js`.

## Triggering a roll — two shapes

### Shape A: Direct roll (you compute the dice pool)

For when vtmtools already knows the pool size (e.g. "roll 7 vs difficulty 4 with the actor's hunger as advanced dice"):

```js
await WOD5E.api.Roll({
  basicDice: 7,                                   // attribute + skill + modifiers
  advancedDice: actor.system.hunger.value,        // hunger (vampires) / rage (werewolves)
  actor,
  difficulty: 4,
  flavor: 'Brawl + Strength',
  quickRoll: false,                               // true = skip the modifier dialog
  rollMode: 'roll',                               // 'roll' | 'gmroll' | 'blindroll' | 'selfroll'
  rerollHunger: false,
  // optional state flags:
  willpowerDamage: 0,
  increaseHunger: false,
  decreaseRage: false,
})
```

Full param reference: `system/api/wod5e-api.js:5-28`.

### Shape B: Dataset-driven (you supply names, system computes the pool)

This is what the sheet's dice buttons use. Internally calls `Roll`:

```js
await WOD5E.api.RollFromDataset({
  dataset: {
    valuePaths: 'attributes.strength.value skills.brawl.value',  // space-separated dot paths
    label: 'Strength + Brawl',
    difficulty: 4,
    selectDialog: false,                          // true → open the splat-aware select dialog
  },
  actor: game.actors.get('ObCGftjZjCvpPBdN'),
})
```

Or, with `selectDialog: true`, pop the GM picker:

```js
await WOD5E.api.RollFromDataset({
  dataset: { selectDialog: true, difficulty: 3 },
  actor,
})
```

The dataset can also carry `skill`, `attribute`, `discipline`, `renown` (string keys, pre-selected in the dialog), `useAbsoluteValue` + `absoluteValue` (numeric override), and `resistance: true` (adds the `resistance` selector for situational modifiers).

## Dice mechanics

Each splat has its own `Die` class (`system/dice/splat-dice.js`) registered as a custom Foundry dice term:

| Code | Class | Splat | Type | Faces |
|---|---|---|---|---|
| `m` | `MortalDie` | mortal/ghoul | basic | 1-5 fail, 6-9 success, 10 critical |
| `v` | `VampireDie` | vampire | basic | same as mortal |
| `g` | `VampireHungerDie` | vampire | advanced | **1 = bestial fail**, 2-5 fail, 6-9 success, 10 critical (messy) |
| `w` | `WerewolfDie` | werewolf | basic | same as mortal |
| `r` | `WerewolfRageDie` | werewolf | advanced | rage failures track separately for frenzy |
| `h` | `HunterDie` | hunter | basic | same as mortal |
| `s` | `HunterDesperationDie` | hunter | advanced | desperation crit-fails apply Despair |

### Roll formula

Built by `generateRollFormula` (`system/scripts/rolls/roll-formula.js`):

| Splat | Formula |
|---|---|
| mortal | `<basic>dm cs>5` |
| vampire | `<basic>dv cs>5 + <hunger>dg cs>5` (with `kh<n>` if rerolling hunger) |
| werewolf | `<basic>dw cs>5 + <rage>dr cs>5` |
| hunter | `<basic>dh cs>5 + <desp>ds cs>5` |

`cs>5` (count-success-greater-than-5) means each die showing 6 or higher contributes 1 to the total. The custom `WOD5eRoll._evaluate` then adds `floor(total_10s / 2) * 2` for the V5 critical bonus (every two 10s = +2 successes).

### Resource side-effects

- **Vampire**, with `increaseHunger: true` and any `g` die showing 1-5, hunger increments (`_increaseHunger`).
- **Werewolf**, with `decreaseRage: true` and any `r` die showing brutal/critfail, rage decrements (`_decreaseRage`).
- These run in `WOD5eDice.Roll` after `roll.evaluate()`, before `roll.toMessage()`.

## Roll result shape

`WOD5eRoll` extends `foundry.dice.Roll`. After `await roll.toMessage(...)`:

- The dice + formula are serialized into the `ChatMessage`'s `rolls` array (`message.rolls[0]` — Foundry standard).
- `roll.system` (string: `mortal | vampire | werewolf | hunter`) is preserved across `toJSON` / `fromJSON` via the override at `system-rolls.js:74-82`.
- `roll.basicDice` and `roll.advancedDice` are live getters returning the corresponding term out of `roll.terms`.
- `roll.total` includes the V5 critical bonus (already added in `_evaluate`).

## Chat message shape

WoD5e overrides `CONFIG.ChatMessage.documentClass` with `WoDChatMessage` and the template with `chat-message-default.hbs` (`system/main.js:106-107`).

When a roll posts to chat, the resulting `ChatMessage` carries:

- Standard Foundry fields: `_id`, `speaker`, `rolls: [<serialized WOD5eRoll>]`, `content` (rendered HTML), `flavor` (the title), `rollMode`.
- A `wod5e` flag scope is reserved but **mostly unused on regular rolls** — verified empty (`flags: {}`) on a captured live roll. The one observed flag is `flags.wod5e.isRollPrompt` on interactive roll-prompt messages (GM clicked "Send to chat" instead of rolling); other clients re-roll into them via `system.wod5e` socket events.

**`rollMessageData` is NOT persisted.** The blob with `totalResult`, `margin`, `criticals`, `critFails`, `enrichedResultLabel` etc. (built by `generateRollMessageData` in `system/scripts/rolls/roll-message.js`) is computed render-time and rendered into the message's HTML — it is not saved as structured data on the message. Verified by capturing a real message: `flags: {}` after a vampire roll. **Implication for vtmtools:** to mirror rolls live, parse `message.rolls[0]` (raw dice results + formula) and **recompute** success / critical / bestial categories on our side. Re-implementing `generateRollMessageData`'s classification logic in Rust is straightforward — see the per-die-class result tables below.

### Splat detection — read the formula, not `roll.system`

The `roll.system` property (set by `WOD5eRoll`'s constructor) is unreliable downstream of `toJSON` / chat-message rehydration — captured live as `undefined`. The robust signal is the formula's dice term denominations, which `generateRollFormula` always emits per splat:

| Formula contains | Splat |
|---|---|
| `dv` and `dg` | vampire |
| `dw` and `dr` | werewolf |
| `dh` and `ds` | hunter |
| only `dm` | mortal |

So vtmtools' inbound translator can derive splat with a regex on `roll.formula` rather than trusting `roll.system`.

### Live sample

A captured `ChatMessage` (12 vampire dice + 0 hunger, total 3 successes) is checked in at [`foundry-vtm5e-roll-sample.json`](./foundry-vtm5e-roll-sample.json). Notably it has empty `flags`, empty `flavor`, no `rollMode`, and `rolls[0]` only carries `total` + `formula` — confirming everything above.

The renderer (`generateRollMessageData`) tags each die's display data:

| Die class | Result categories produced |
|---|---|
| Basic (m/v/w/h) | `success` (6-9), `critical` (10), `failure` (1-5) |
| Hunger (g) | `success`, `critical` (messy), `failure` (2-5), `bestial` (1) |
| Rage (r) | `success`, `critical`, `failure`, `brutal` (1) |
| Desperation (s) | `success`, `critical`, `failure`, `criticalFailure` |

These categories map to images via `DiceRegistry.basic[system]` and `DiceRegistry.advanced[system]` (`system/api/def/dice.js`) — useful if vtmtools wants to render dice icons matching Foundry's appearance.

## Hooks to listen for

Standard Foundry roll-lifecycle hooks fire as expected. From outside the system, **prefer hooking `createChatMessage`** — it fires after the roll is committed, the `rolls` array is populated, and any side-effects (hunger ↑, rage ↓) have settled.

```js
Hooks.on('createChatMessage', (message, options, userId) => {
  if (!message.rolls?.length) return;       // not a roll
  const roll = message.rolls[0];
  // roll.system, roll.basicDice, roll.advancedDice, roll.total
  // message.flavor, message.flags.wod5e
  // forward to vtmtools desktop app
});
```

Lifecycle ordering for a typical roll:
1. User clicks dice button on sheet → `_onConfirmRoll` (`system/actor/scripts/roll.js`)
2. `WOD5E.api.RollFromDataset` builds the dataset → `WOD5eDice.Roll`
3. `_evaluate` resolves dice and applies V5 crit bonus
4. Optional dialog (skip with `quickRoll: true`)
5. `roll.toMessage()` posts the chat message
6. `preCreateChatMessage` and `createChatMessage` fire on every connected client
7. Side-effects: `_increaseHunger`, `_decreaseRage`, `_damageWillpower`, etc.

Note on the dialog: by default `Roll` opens an interactive **DialogV2** asking for difficulty + situational modifiers. Pass `quickRoll: true` when triggering from outside (vtmtools doesn't want a Foundry dialog popping up on every API call) — the roll fires immediately with the supplied `basicDice` / `advancedDice` and `difficulty`.

## Internal apply paths (for write-back commands)

If vtmtools wants to update game state without going through a roll, these helpers (`system/scripts/rolls/`) are the surgical entry points:

| Helper | What it does |
|---|---|
| `_damageWillpower(_, _, actor, amount, rollMode)` | Adds `amount` superficial willpower; converts to aggravated if track is full |
| `_increaseHunger(_, _, actor, amount, rollMode)` | Bumps `system.hunger.value`, capped at `system.hunger.max` |
| `_decreaseRage(_, _, actor, amount, rollMode)` | Decrements `system.rage.value`, floored at 0 |
| `_applyOblivionStains(_, _, actor, amount, rollMode)` | Adds `system.humanity.stains` (Oblivion-power feedback) |

These are invoked internally by `WOD5eDice.Roll` based on the `willpowerDamage` / `increaseHunger` / `decreaseRage` opts passed in. They're not exposed via `WOD5E.api`, so calling them from a foreign module would mean an `import` from the module namespace — fragile across system updates. Prefer issuing `actor.update({ "system.hunger.value": <new> })` with a value vtmtools computed itself.

## Selectors for situational modifiers

The roll system supports a "selectors" array driving optional modifier checkboxes. Built up automatically in `RollFromDataset` when the GM picks options in the select-roll dialog. Example output for "Strength + Brawl on a vampire with blood surge":

```js
selectors: ['attributes', 'attributes.strength', 'physical', 'skills', 'skills.brawl', 'blood-surge']
```

These are matched against `actor.system.bonuses[*].selectors` to find which bonuses apply (handled in `getSituationalModifiers` — `system/scripts/rolls/situational-modifiers.js`). Most selectors of interest:

- `attributes`, `attributes.<name>`, `physical|social|mental` (the attribute's category)
- `skills`, `skills.<name>`
- `disciplines`, `disciplines.<name>`
- `renown`, `renown.<name>` (werewolf)
- `blood-surge` (vampire)
- `resistance` (defensive rolls)

## Wire integration sketch (for a future bridge feature)

Two new wire message types would handle rolls cleanly:

**Inbound** (Foundry → vtmtools, fired from `createChatMessage` hook):
```json
{
  "type": "roll_result",
  "actor_id": "ObCGftjZjCvpPBdN",
  "splat": "vampire",
  "title": "Strength + Brawl",
  "flavor": "...",
  "basic_results": [3, 7, 9, 10, 6, 4, 8],
  "advanced_results": [2, 1, 6],
  "total": 5,
  "difficulty": 4,
  "margin": 1,
  "criticals": 1,
  "messy": false,
  "bestial": true,
  "message_id": "..."
}
```

**Outbound** (vtmtools → Foundry, calls `WOD5E.api.Roll`):
```json
{
  "type": "trigger_roll",
  "actor_id": "ObCGftjZjCvpPBdN",
  "value_paths": ["attributes.strength.value", "skills.brawl.value"],
  "advanced_dice": "auto",
  "difficulty": 4,
  "flavor": "Brawl + Strength",
  "quick_roll": true,
  "selectors": ["attributes.strength", "skills.brawl", "physical"]
}
```

Implementation pattern matches the existing `update_actor` / `create_item` flow — `bridge/foundry/mod.rs` adds a `build_trigger_roll` method to its `BridgeSource` impl, the Foundry module's `handleInbound` dispatches `trigger_roll` to `WOD5E.api.Roll`, and a new `Hooks.on("createChatMessage", ...)` in the module emits `roll_result` to the desktop app. No protocol changes needed beyond adding the two message types.
