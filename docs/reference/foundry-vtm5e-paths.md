# Foundry WoD5e — VTM Actor Data Paths

Verified actor schema paths for the [WoD5e Foundry system](https://github.com/WoD5E-Developers/wod5e), used by `src-tauri/src/bridge/foundry/translate.rs` to map Foundry actor data into the canonical bridge character model.

## Pinned source

| Field | Value |
|---|---|
| Repo | `https://github.com/WoD5E-Developers/wod5e` |
| Inspected version | `5.3.17` (system.json) |
| Inspected commit | `d16b5d960a42de4912e7a8986e57a2fbbea54f78` (2026-04-23) |
| Foundry compatibility | minimum V14, verified V14.360 |
| System id | `wod5e` |
| Vampire actor type | `vampire` (also: `mortal`, `spc`, `ghoul`, `hunter`, `werewolf`, `group`) |

## Canonical → WoD5e path mapping

All paths are dot-notation under `actor.system.<…>`. Schema source:
- `system/actor/data-models/base-actor-model.js` (health, willpower)
- `system/actor/data-models/fields/vampire-fields.js` (hunger, humanity, blood)

| Canonical field | WoD5e path | Default | Notes |
|---|---|---|---|
| hunger | `system.hunger.value` | 1 | 0–5 |
| (hunger max) | `system.hunger.max` | 5 | rarely modified |
| humanity | `system.humanity.value` | 7 | |
| humanity_stains | `system.humanity.stains` | 0 | |
| blood_potency | `system.blood.potency` | 0 | |
| (blood generation) | `system.blood.generation` | "" | string, e.g. "13th" |
| health.max | `system.health.max` | 5 | total boxes |
| health.superficial | `system.health.superficial` | 0 | superficial damage taken |
| health.aggravated | `system.health.aggravated` | 0 | aggravated damage taken |
| (health remaining) | `system.health.value` | 5 | NOT derived — see note below |
| willpower.max | `system.willpower.max` | 5 | |
| willpower.superficial | `system.willpower.superficial` | 0 | |
| willpower.aggravated | `system.willpower.aggravated` | 0 | |
| (willpower remaining) | `system.willpower.value` | 5 | NOT derived — see note below |

Health/willpower follow V5 rules: total = max, damage = superficial + aggravated. The system tracks all four fields independently.

**Caveat — `value` is independent, not derived.** Initial doc claimed `system.health.value = max − aggravated − superficial`. The 2026-04-26 captured sample (`foundry-vtm5e-actor-sample.json`) disproves this: that character has `max:6, aggravated:0, superficial:0, value:5`. With zero damage and `value !== max`, the relationship is clearly not arithmetic. Treat `value` as an independent field WoD5e tracks alongside max/aggravated/superficial; the canonical translator ignores it and only reads max/aggravated/superficial.

## Apply-attribute routing (apply-roll write-back)

The frontend's `bridge_set_attribute(source, source_id, name, value)` arrives at the Foundry source's `build_set_attribute`, which must translate `name` into a Foundry-specific operation. The mapping below covers what Roll20's apply path uses today:

| Canonical `name` | Foundry operation |
|---|---|
| `hunger` | `actor.update({ "system.hunger.value": <int> })` |
| `humanity` | `actor.update({ "system.humanity.value": <int> })` |
| `humanity_stains` | `actor.update({ "system.humanity.stains": <int> })` |
| `blood_potency` | `actor.update({ "system.blood.potency": <int> })` |
| `health_superficial` | `actor.update({ "system.health.superficial": <int> })` |
| `health_aggravated` | `actor.update({ "system.health.aggravated": <int> })` |
| `willpower_superficial` | `actor.update({ "system.willpower.superficial": <int> })` |
| `willpower_aggravated` | `actor.update({ "system.willpower.aggravated": <int> })` |
| `resonance` | **Item creation** — see below |

## Resonance is an Item, not a field

Unlike Roll20, WoD5e stores the active Resonance as an [Item document](https://github.com/WoD5E-Developers/wod5e/blob/main/system/item/vtm/resonance-item-sheet.js) attached to the actor (`actor.items` collection, `type: "resonance"`). Setting resonance is therefore not a field update — it's an `Actor.createEmbeddedDocuments("Item", [...])` call (and may need to delete the previous Resonance item first).

The Foundry source's `build_set_attribute("resonance", value)` must build a wire message of a different shape than the standard `update_actor`:

```json
{
  "type": "create_item",
  "actor_id": "<id>",
  "item_type": "resonance",
  "item_name": "<value>",
  "replace_existing": true
}
```

The Foundry module's `handleInbound` then calls something like:

```js
const existing = actor.items.filter(i => i.type === "resonance");
if (msg.replace_existing && existing.length) {
  await actor.deleteEmbeddedDocuments("Item", existing.map(i => i.id));
}
await actor.createEmbeddedDocuments("Item", [{
  type: "resonance",
  name: msg.item_name,
}]);
```

This keeps `name` opaque to the frontend while letting each source impl interpret it correctly.

## Ownership

Foundry actor ownership is on `actor.ownership`, a map `{ <userId>: <permissionLevel> }` where `3` (`OWNER`) means full control. The translator should pick the first non-GM owner ID, or `null` if only the GM owns the actor — analogous to Roll20's `controlled_by` field.

## Live capture sample

A real, captured wire blob (the exact JSON the Foundry module sends to the Tauri app inside `FoundryInbound::ActorUpdate`) is checked in at [`foundry-vtm5e-actor-sample.json`](./foundry-vtm5e-actor-sample.json). Use it as ground truth for the wire shape — better than re-reading the WoD5e source whenever the translator needs adjusting.

### Shape findings beyond the canonical fields

The sample shows that **WoD5e attaches all three splat schemas (vampire / werewolf / hunter) to every actor**, regardless of the actor's `type`. Default-zero fields just sit there until the GM populates them. Practical consequence: the canonical Foundry translator can read vampire fields straight off any actor, but a future feature could surface werewolf/hunter data without schema gymnastics.

Splat-specific shapes worth knowing if/when the Campaign tool extends Foundry support:

| Path | Shape | Splat |
|---|---|---|
| `system.attributes.<name>.value` | int 0-5; `<name>` ∈ str/dex/sta/cha/man/com/int/wits/resolve | shared |
| `system.skills.<name>.value` | int 0-5; ~27 skills | shared |
| `system.disciplines.<name>` | `{ value: int, visible: bool, selected: bool, powers: [], description: "" }` | vampire |
| `system.selectedDiscipline` | discipline name string (e.g. `"auspex"`) | vampire |
| `system.headers.{ambition,desire,sire,touchstones,tenets,…}` | strings | shared |
| `system.rage.value` / `system.rage.max` | int | werewolf |
| `system.balance.harano.value` / `system.balance.hauglosk.value` | int | werewolf |
| `system.crinosHealth.{max,superficial,aggravated,value}` | health-track-like | werewolf |
| `system.gifts.<name>.value` | int 0-5; `<name>` ∈ tribe/auspice keys | werewolf |
| `system.renown.{glory,honor,wisdom}.value` | int | werewolf |
| `system.edges.<name>.{value,visible,perks,pools,description}` | hunter-specific advantage tree | hunter |

### Disciplines, in detail

Unlike Roll20 (which uses repeating-section attribute names like `repeating_disciplines_<id>_discipline`), Foundry stores disciplines as a **flat keyed object** under `system.disciplines`. Each discipline has:
- `value`: dot rating (0-5)
- `visible`: shown on the sheet
- `selected`: which discipline is currently active for rolls (only one is `true`)
- `powers`: array of power objects (per-discipline)
- `description`: free-form text

The list of discipline keys observed in the sample: `alchemy, animalism, auspex, celerity, ceremonies, dominate, fortitude, obfuscate, oblivion, potence, presence, protean, rituals, sorcery`. Note that `ceremonies`, `rituals`, `sorcery`, and `alchemy` are **discipline-extensions** (Blood Sorcery rituals, Oblivion ceremonies, etc.), not core V5 disciplines — they coexist with their parent disciplines in the same map.
