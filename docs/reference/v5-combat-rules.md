# V5 Combat Rules Reference

Source: [V5 Homebrew Wiki — Combat Primer](https://www.v5homebrew.com/wiki/Combat_Primer) by ElmerG, vetted by V5 developer Karim Muammar.

This is a detailed mechanical reference for Vampire: The Masquerade 5th Edition physical conflict rules, distilled from the Combat Primer and core book references (pages 123–130, 295–302).

---

## Core Mechanic

All combat is a **contested roll**: attacker pool vs defender pool. Whoever gets more successes wins; the difference is the **margin**. The winner deals margin as damage (+ weapon modifier if applicable). On a **tie**, margin = 1 + weapon damage.

---

## Attack Pools

Attribute + Skill combinations determined by the narrative. Common pools:

| Style | Pool | Notes |
|---|---|---|
| Unarmed (punch, kick, claw) | **Str + Brawl** | Martial arts, bodyblows, vampire claws |
| Light melee weapon | **Dex + Melee** | Knives, short swords, rapiers, whips, light spears |
| Heavy melee weapon | **Str + Melee** | Axes, hammers, greatswords, broadswords, naginatas |
| Thrown weapon | **Dex + Athletics** | Knives, shuriken, stakes |
| Firearms (precision/calm) | **Composure + Firearms** | Knowing when/where to fire; SWAT-style |
| Firearms (speed/reflex) | **Dex + Firearms** | Quick draw, erratic targets, close-range snap shots |
| Firearms (endurance/sniping) | **Resolve + Firearms** | Long waits, suppressing fire, mental duress shooting |

The ST picks the appropriate pool based on the narrative of what the character is actually doing. This list is not exhaustive.

---

## Defense Pools

| Style | Pool | Notes |
|---|---|---|
| Dodge | **Dex + Athletics** | Speed and athleticism to avoid being hit |
| Parry / block (non-lethal) | **Brawl or Melee pool** | Using fighting skill to redirect, parry, or bind |
| Ranged weapon in close combat | **Str + Firearms** | −2 penalty for weapons bigger than a pistol (pg. 302) |

- Non-lethal defense (parry/block) **cannot normally defend against Firearms** unless the ST rules it narratively appropriate (e.g., the gunner is in melee range with the defender).
- This distinction (dodge vs non-lethal defense) was implied in V5 core and clarified in Hunter: The Reckoning 5th Edition (H5 pg. 118–119).

---

## Multiple Opponents

When facing multiple attackers in a single turn, the defender chooses one of three approaches:

### Fight All Back (Split Attack Pool)
- Split your attack pool however you choose across all targets.
- Each attacker rolls their **full pool** against your split portion.
- Example: Pool of 8, split 2/3/3 against attackers with pool 6 each → rolls of 2v6, 3v6, 3v6.

### Dodge All (Cumulative Penalty)
- Use full dodge pool against the first attacker.
- Each subsequent attacker faces your dodge pool with a **cumulative −1**.
- Example: Dodge pool 6 → 6 vs A, 5 vs B, 4 vs C.
- Only the attackers can deal damage (you're purely evading).

### Mix Attack + Dodge (Pool Switching)
- Use your **attack pool** against one target (contested as normal).
- Switch to your **dodge pool** for the remaining attackers, with **cumulative −1** starting from the first dodge.
- Karim confirmed: you DO switch pools when mixing, despite the pg. 125 example not showing this clearly.
- Example: Attack pool 8 vs target C; then dodge pool 6 −1 = 5 vs A; dodge pool 6 −2 = 4 vs B.

---

## Damage

- Winner of the contest deals **margin as damage** (Superficial or Aggravated depending on source).
- Unarmed attacks deal **Superficial** damage to vampires.
- Bladed/piercing melee weapons deal **Superficial** to vampires (halved as normal).
- **Aggravated** sources to vampires: fire, sunlight, vampire fangs, and certain supernatural powers.
- Superficial damage to vampires is **halved (round down)** before being applied.
- Weapon damage modifiers are added to the margin.

---

## Ranged Combat Details

### Ranged vs Ranged (Shootout)
- Both characters roll their **Firearms attack pool** against each other (contested).
- Cover is assumed (lean out, fire, duck back).
- Higher successes = margin = damage.

### Ranged vs Dodging Target
- Attacker rolls Firearms pool.
- Defender rolls **Dex + Athletics** (dodge), modified by cover:

| Cover Situation | Modifier |
|---|---|
| Good cover (dumpsters, fire escapes, pillars) | **+1 to dodge pool** |
| No cover (open street, empty ground) | **−2 to dodge pool** |

### Ranged vs Immobilized/Stationary Target (Static Difficulty)
- If the target **cannot or will not move**, there is no contested roll.
- Attacker rolls against **static Difficulty 1** (1 success needed to hit).
- All additional successes beyond 1 become margin = damage.
- Applies when: physically pinned, tied up, choosing not to move, or in a position where dodging is narratively impossible (e.g., fleeing down a narrow corridor).
- Difficulty 1 is consistent with Surprise Attacks and Lightning Strike (not Difficulty 2, despite some book references).

### Ranged Weapons in Close Combat
- If engaged in melee/brawl range by an attacker, the Ranged user must use **Str + Firearms**.
- **−2 penalty** for weapons bigger than a pistol (pg. 302).

---

## Turn Structure

### Phase 0: Precombat / Mediation
- Can the fight be avoided? Is there genuine risk or dramatic purpose?
- If lopsided (5 vampires vs 2 street thugs), narrate the outcome.
- If between PCs, offer Mediation (discuss outcome, apply appropriate consequences). Use Concessions rules (pg. 295) as guidelines.
- If no mediation possible, proceed to conflict.

### Phase 1: Declaration and Movement
- All characters declare **ACTION** + **TARGET/INTENT**.
  - "While stepping back, I'm going to shoot my gun at the red-headed hunter."
  - "I am running forward and going to claw the burly hunter in the gut."
- Handle all movement. Minor Actions for extra movement assessed here.
- Perform Minor Actions: draw weapon, reload, send a text, activate round-long Disciplines, Blood Heal via Rouse Check.
- Minor Actions impose penalties on Primary Action.

### Phase 2: Conflict (resolved in sub-phases)

#### Conflict A: Minor Actions
Any remaining Minor Actions that haven't been handled. All must be accounted for due to the penalties they levy.

#### Conflict B: Currently-Engaged Physical Conflict
- Ongoing brawl/melee from prior turns (characters who kept pace with each other).
- Handle in descending initiative order if needed.
- Blood Surge declared and activated **before** the dice roll.
- Ranged users engaged in melee must use the Ranged Weapons in Close Combat rule.

#### Conflict C: Ranged Conflict
- All ranged-only conflict.
- Cover rules apply (see Ranged Combat Details above).
- Blood Surge declared before the dice roll.
- Ranged users who become newly engaged in melee this phase use Close Combat rule.

#### Conflict D: Newly-Engaged Physical Conflict
- Characters who just closed distance this turn.
- Handle in descending initiative order if needed.
- Blood Surge declared before dice roll.

#### Conflict E: Everything Else
- Non-combat dice pools: picking locks, opening safes, crossing obstacles, etc.
- Blood Surge declared here if applicable.

Then loop back to Phase 1 for the next turn. Repeat until conflict resolves.

---

## Advanced Conflict Options (Bolt-Ons)

These are not a separate system — they're additions to base conflict for more important or dramatic fights.

### Maneuver
- Narrative action to gain an advantage: flanking, aiming, providing a distraction, stunting.
- Roll an appropriate pool (e.g., Dex + Stealth to flank, Wits + Awareness to aim under pressure).
- ST awards a **dice bonus** based on action and success: +1 or +2 for reasonable actions, max +3.
- **+1 additional on a Critical Win.**
- Compare to "stunting" in Scion/Exalted.

### Block
- Impede a target's action (e.g., suppressing fire, shoving obstacles at them).
- Roll appropriate pool; successes become the Block threshold.
- Target must **beat your Block successes** before they can perform their chosen action.
- If the target fails to overcome the Block, they don't lose their action — they can declare a different action, but must try again next turn against the original target.

### All-Out Attack
- Add dice to attack pool, but you **cannot defend** this turn.
- Suboptimal in most situations. Best used against weaker opponents you want to finish quickly, or as a desperation move.
- Damage application detailed on pg. 298.

### All-Out Defense
- Add dice to defense pool, but you **cannot attack** this turn.
- Good when cover is available. Purely defensive posture.

### Surprise Attacks
- Attacker rolls vs **Difficulty 1** (target gets no defense pool).
- Straightforward per book.

### Initiative
- **Dex + Athletics** contest to determine acting order.
- **Optional** — simultaneous resolution works better in practice.
- Breaks down with split pools.
- Useful mainly for **tie-breaking** or when one character acting before another matters narratively.
- Historical note: ported from V5 alpha where characters acted in strict initiative order, making combat even more lethal.

---

## Blood in Combat

| Action | Cost | Timing | Notes |
|---|---|---|---|
| **Blood Surge** | 1 Rouse Check | Declare before the roll | Adds dice to **one roll** this turn only (not entire turn/scene) |
| **Blood Heal** | 1 Rouse Check | Any point during the turn | Heal Superficial damage |
| **Discipline activation** | Per Discipline cost | When the action is taken | Max **one Discipline activation per turn** |

- You may perform one Rouse Check per turn per each separate thing that requires one (Blood Surge + Blood Heal + Discipline = 3 separate Rouse Checks are legal in one turn).

---

## Quick Conflict Methods

For shorter or lopsided fights, STs can use:

- **Three Turns and Out / Three, Two, Done** (pg. 130 / pg. 295): Cap combat at ~3 rounds and narrate conclusion. Good for lopsided conflicts or pacing.
- **One Roll** (pg. 296): Single contested roll resolves the entire conflict.
- **Concessions** (pg. 295): Negotiated outcome before or during conflict.

All follow the same phase structure, just compressed.

---

## Page References (V5 Core)

| Topic | Page |
|---|---|
| Conflicts / Contests | 123 |
| Conflict order of operations | 124–125 |
| Three Turns and Out | 130 |
| Concessions | 295 |
| One Roll conflict | 296 |
| Advance / Maneuver / Block | 296–298 |
| All-Out Attack | 298 |
| Initiative | 300 |
| Surprise Attacks | 301 |
| Ranged Combat / Close Combat | 302 |
