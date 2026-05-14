# Hunger / Remorse / Contested Roll Composites — Design

**GitHub issue:** [#11](https://github.com/Hampternt/vtmtools/issues/11) — *Hunger / remorse / contested rolls (compose Phase 1 primitives)*

**Roadmap origin:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 3.

**Status:** spec — implementation plan pending (`docs/superpowers/plans/2026-05-12-hunger-remorse-contested-rolls.md`).

---

## §1 Scope

Three composite V5 dice helpers that build on the existing Phase 1 primitives in `src-tauri/src/shared/v5/`:

1. **`rouse_check`** — single-die hunger test (1d10 vs 6+); on failure, Hunger increases by 1.
2. **`remorse_check`** — Humanity preservation roll after acts against Humanity; on total failure, Humanity drops by 1.
3. **`contested_check`** — two opposed `skill_check`s producing a winner, margin, and tie-handling.

Each composite ships with:
- A Rust orchestrator in `src-tauri/src/shared/v5/<name>.rs` (pure logic, RNG-injected).
- Typed input / result structs in `src-tauri/src/shared/v5/types.rs`.
- A synchronous Tauri command in `src-tauri/src/tools/<name>.rs` that wraps the orchestrator with `rand::thread_rng()`.
- A TS wrapper + types in `src/lib/v5/api.ts`.
- `#[cfg(test)] mod tests` per file, deterministic via `StdRng::seed_from_u64`.

No new UI in this scope. Consumer features (character-sheet buttons, GM screen contested panel) integrate the helpers in their own specs.

---

## §2 Architectural fit

### §2.1 File layout (after this work)

```
src-tauri/src/shared/v5/
├── mod.rs              (extended: 3 new `pub mod` lines, re-exports)
├── types.rs            (extended: 3 input + 3 result struct pairs, ContestedSide enum)
├── pool.rs             (unchanged)
├── dice.rs             (unchanged)
├── interpret.rs        (unchanged)
├── difficulty.rs       (unchanged)
├── message.rs          (unchanged)
├── skill_check.rs      (unchanged)
├── rouse.rs            NEW
├── remorse.rs          NEW
└── contested.rs        NEW

src-tauri/src/tools/
├── mod.rs              (extended)
├── skill_check.rs      (unchanged)
├── rouse.rs            NEW
├── remorse.rs          NEW
└── contested.rs        NEW

src/lib/v5/
└── api.ts              (extended: 3 new TS interfaces + 3 new wrapper functions)
```

Three new Tauri commands registered in `src-tauri/src/lib.rs`:
- `roll_rouse_check`
- `roll_remorse_check`
- `roll_contested_check`

**Message formatters stay with their composites.** Each new module (`rouse.rs`, `remorse.rs`, `contested.rs`) contains a private `format_*` helper. The existing `shared/v5/message.rs` keeps its single responsibility (formatter for `skill_check`); we do NOT add the new formatters there. Rationale: Phase 1 split files by *layer* (pool, dice, interpret, …) for the single `skill_check` pipeline. Each new composite is its own vertical feature with a unique message shape — co-locating the formatter with the orchestrator keeps each composite self-contained.

### §2.2 Approach selection

Three approaches were considered:

**A. Typed-per-composite orchestrators** *(selected)* — Each composite owns its input/result types and a thin orchestrator. Reuses `roll_pool` / `interpret` / `compare` directly where applicable. Matches the Phase 1 leaf-per-file layout and aligns with the typed-per-helper decision from the Foundry helper-library roadmap §3.

**B. Generalize `SkillCheckInput`** — Rejected: rouse has no attribute/skill at all and no hunger dice, remorse has a fixed DV 1 and no hunger dice. Optionality-bloat on the existing struct would make the public IPC shape lie about what each call actually carries.

**C. Lower-level shared core** — Extract a `roll_at_difficulty(parts, hunger, difficulty, rng)` primitive below `skill_check` and route every helper through it. Rejected: introduces a second orchestrator parallel to `skill_check` for negligible code reduction. The existing leaves already compose cleanly; the wins from a sub-orchestrator are too marginal to justify the new layer.

---

## §3 `rouse_check` (`shared/v5/rouse.rs`)

### §3.1 V5 mechanic recap

- Standard Rouse: roll 1d10. Pass on 6–10, fail on 1–5.
- Failure: Hunger +1 (caller applies the delta; the helper just reports it).
- Blood Potency ≥ 2 grants re-roll on certain Discipline-cost Rouse Checks (roll 2d10, take higher). The *policy* of which Rouses qualify is BP/level-dependent and out of scope here; the helper just exposes "extra dice, take best".

### §3.2 Rust types

```rust
// in shared/v5/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouseCheckInput {
    pub character_name: Option<String>,
    /// 0 = standard 1d10. 1 = 2d10 take-highest (BP re-roll). 2+ supported.
    pub extra_dice: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RouseCheckResult {
    /// All rolled values in roll order. Length = 1 + extra_dice.
    pub dice: Vec<u8>,
    /// max(dice).
    pub best: u8,
    pub passed: bool,         // best >= 6
    pub hunger_delta: u8,     // 0 on pass, 1 on fail
    pub message: String,
}
```

### §3.3 Orchestrator + pure evaluator

Per the Phase 1 pattern (`pool::build_pool` pure, `dice::roll_pool` is the only RNG site), the orchestrator separates RNG from logic so the evaluation step is directly testable without seed-hunting.

```rust
/// Pure evaluator: given the rolled dice, produce best, passed, hunger_delta.
/// Direct unit tests provide explicit dice values without depending on RNG.
pub fn evaluate_rouse(dice: &[u8]) -> (u8, bool, u8) {
    let best = *dice.iter().max().expect("at least one die");
    let passed = best >= 6;
    let hunger_delta: u8 = if passed { 0 } else { 1 };
    (best, passed, hunger_delta)
}

pub fn rouse_check<R: Rng + ?Sized>(
    input: &RouseCheckInput,
    rng: &mut R,
) -> RouseCheckResult {
    let n = 1 + input.extra_dice as usize;
    let dice: Vec<u8> = (0..n).map(|_| rng.gen_range(1..=10)).collect();
    let (best, passed, hunger_delta) = evaluate_rouse(&dice);
    let message = format_rouse(input, &dice, best, passed);
    RouseCheckResult { dice, best, passed, hunger_delta, message }
}
```

Message format: `"<Name>'s Rouse check · <Pass|Fail> · best <N> (rolled [d1, d2, …])"` (name omitted if `None`).

### §3.4 Rationale: not using `PoolSpec` / `RollResult`

The Phase 1 types model `PoolPart`s with `regular_count + hunger_count`. Rouse has neither — it's a single d10 (or N d10s) with no Hunger semantics. Forcing it through `build_pool` would require a fake `PoolPart` and a `regular_count` overload that obscures intent. A bare `Vec<u8>` is more honest.

The `dice: Vec<u8>` field also preserves the raw rolls in order, so a future consumer can reconstruct "you got it first try" vs "you re-rolled into success" by inspecting `dice[0] >= 6` vs `best >= 6 && dice[0] < 6`. The helper itself doesn't surface this distinction (it's policy-dependent — the consumer feature knows whether the first die "counts" or is purely cosmetic), but the data is preserved.

### §3.5 Tests (`#[cfg(test)] mod tests`)

Two-tier split mirroring Phase 1:

**Pure-evaluator tests (no RNG):**
- `evaluate_rouse(&[6])` → `(6, true, 0)` — boundary pass.
- `evaluate_rouse(&[5])` → `(5, false, 1)` — boundary fail.
- `evaluate_rouse(&[1])` → `(1, false, 1)`, `evaluate_rouse(&[10])` → `(10, true, 0)`.
- `evaluate_rouse(&[3, 8])` → `(8, true, 0)` — extra_dice=1 take-best (one fail + one pass = pass).
- `evaluate_rouse(&[2, 4, 5])` → `(5, false, 1)` — extra_dice=2 all-fail.

**Orchestrator tests (seeded RNG):**
- `extra_dice = 0` rolls exactly 1 die.
- `extra_dice = 1` rolls exactly 2 dice.
- `extra_dice = 3` rolls exactly 4 dice.
- Deterministic with seeded `StdRng`: same seed → same dice and message.
- Message contains character name when set; omits owner prefix when `None`.

---

## §4 `remorse_check` (`shared/v5/remorse.rs`)

### §4.1 V5 mechanic recap

Authoritative reference: `/home/hampter/.local/share/FoundryVTT/Data/systems/wod5e/system/actor/vtm/scripts/roll-remorse.js:23`. The Foundry WoD5e implementation is:

```js
const dicePool = Math.max(10 - humanity - stains + activeModifiers.totalValue, 1)
```

- Pool = `max(10 − humanity − stains, 1)` regular dice. No Hunger dice. No attribute/skill — pool is fixed by character state. (Pool semantics: "unfilled Humanity boxes that don't currently contain Stains" — the closer to losing Humanity you are, the larger the pool.)
- Difficulty = 1 success ("at least one die ≥6" per the Foundry module's `hasSuccess` check).
- Stains clear to 0 regardless of pass/fail (matches V5 corebook + WoD5e module behavior).
- Failure additionally drops Humanity by 1 (clamped at 0).

### §4.2 Rust types

```rust
// in shared/v5/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemorseCheckInput {
    pub character_name: Option<String>,
    pub humanity: u8,    // 0..=10
    pub stains: u8,      // 0..=10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemorseCheckResult {
    pub spec: PoolSpec,
    pub roll: RollResult,
    pub tally: Tally,
    pub outcome: Outcome,
    /// -1 on failure (any failure to get ≥1 success), else 0. Caller applies to character state.
    pub humanity_delta: i8,
    /// Stains value after the roll. ALWAYS 0 — stains clear regardless of outcome,
    /// matching WoD5e Foundry module behavior. Exposed as a field rather than a doc-only
    /// comment so consumers can blindly `actor.stains = result.stains_after`.
    pub stains_after: u8,
    pub message: String,
}
```

### §4.3 Orchestrator

```rust
pub fn remorse_check<R: Rng + ?Sized>(
    input: &RemorseCheckInput,
    rng: &mut R,
) -> RemorseCheckResult {
    // Pool = max(10 - humanity - stains, 1). Use i16 for safe subtraction.
    let raw = 10i16 - (input.humanity as i16) - (input.stains as i16);
    let pool_size = raw.max(1) as u8;
    let spec = PoolSpec {
        parts: vec![PoolPart {
            name: "Remorse (10 - Humanity - Stains)".into(),
            level: pool_size,
        }],
        regular_count: pool_size,
        hunger_count: 0,
    };
    let roll = roll_pool(&spec, rng);
    let tally = interpret(&roll);
    let outcome = compare(&tally, 1);                       // DV 1
    // Failure here = any roll that doesn't pass DV 1, i.e. successes == 0.
    let humanity_delta: i8 = if outcome.passed { 0 } else { -1 };
    let message = format_remorse(input, &outcome);
    RemorseCheckResult {
        spec, roll, tally, outcome,
        humanity_delta,
        stains_after: 0,           // ALWAYS 0 per WoD5e behavior
        message,
    }
}
```

Message format: `"<Name>'s Remorse check · <Success|Failure[· Humanity −1]> · <N successes>"`.

### §4.4 Edge cases

- `humanity = 0, stains = 0` → pool = `max(10 - 0 - 0, 1)` = 10 regular dice (max remorse chance — almost no Humanity left, easy to feel it).
- `humanity = 10, stains = 0` → pool = `max(10 - 10, 1)` = 1 die (high-Humanity character barely feels the need).
- `humanity = 5, stains = 3` → pool = `max(10 - 5 - 3, 1)` = 2 dice.
- `humanity = 7, stains = 5` → pool = `max(10 - 7 - 5, 1)` = `max(-2, 1)` = 1 die (clamped).

### §4.5 Tests

- Pool size = `max(10 - humanity - stains, 1)` for representative inputs (covering both clamped and non-clamped paths).
- Pool never has Hunger dice (`spec.hunger_count == 0`).
- Difficulty is 1 (`outcome.difficulty == 1`).
- `outcome.passed == false` ⇒ `humanity_delta == -1`.
- `outcome.passed == true` ⇒ `humanity_delta == 0`.
- `stains_after == 0` for both pass and fail cases.
- Edge: `humanity = 10, stains = 0` → pool = 1 (not 0).
- Edge: `humanity = 7, stains = 5` → pool = 1 (clamp engaged when 10 − humanity − stains ≤ 0).
- Edge: `humanity = 0, stains = 0` → pool = 10 (no underflow / clamp on the upper side).

---

## §5 `contested_check` (`shared/v5/contested.rs`)

### §5.1 V5 mechanic recap

Per `docs/reference/v5-combat-rules.md` §"Core Mechanic":
- Both sides roll. Winner = more successes.
- Margin = difference.
- Tie: defender wins; margin = 1 + weapon damage.

For non-combat contested rolls (e.g. Persuasion vs Intelligence + Awareness), the tie-handling rule is application-specific. The helper exposes both knobs.

### §5.2 Rust types

```rust
// in shared/v5/types.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContestedSide { Attacker, Defender }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestedCheckInput {
    pub attacker: SkillCheckInput,
    pub defender: SkillCheckInput,
    /// Who wins on equal successes. V5 combat default = Defender.
    pub tie_goes_to: ContestedSide,
    /// Bonus added to margin on a tie. Combat callers pass weapon damage
    /// here (V5 rule: tie margin = 1 + weapon damage). Non-combat callers
    /// pass 0.
    pub tie_margin_bonus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestedCheckResult {
    pub attacker: SkillCheckResult,
    pub defender: SkillCheckResult,
    pub winner: ContestedSide,
    /// On non-tie: |attacker.successes - defender.successes|.
    /// On tie: 1 + tie_margin_bonus (matches V5 combat tie rule when bonus = weapon damage).
    pub margin: i32,
    pub was_tie: bool,
    pub message: String,
}
```

### §5.3 Orchestrator + pure resolver

Same split as `rouse.rs`: a pure `resolve_contested` function takes already-known success counts and produces winner / margin / was_tie. The orchestrator calls `skill_check` twice and delegates.

```rust
/// Pure resolver: given the success counts of both sides, produce (winner, margin, was_tie).
/// Direct unit tests bypass RNG.
pub fn resolve_contested(
    attacker_successes: u8,
    defender_successes: u8,
    tie_goes_to: ContestedSide,
    tie_margin_bonus: i32,
) -> (ContestedSide, i32, bool) {
    let a = attacker_successes as i32;
    let d = defender_successes as i32;
    if a == d {
        (tie_goes_to, 1 + tie_margin_bonus, true)
    } else if a > d {
        (ContestedSide::Attacker, a - d, false)
    } else {
        (ContestedSide::Defender, d - a, false)
    }
}

pub fn contested_check<R: Rng + ?Sized>(
    input: &ContestedCheckInput,
    rng: &mut R,
) -> ContestedCheckResult {
    let attacker = skill_check(&input.attacker, rng);
    let defender = skill_check(&input.defender, rng);
    let (winner, margin, was_tie) = resolve_contested(
        attacker.outcome.successes,
        defender.outcome.successes,
        input.tie_goes_to,
        input.tie_margin_bonus,
    );
    let message = format_contested(input, &attacker, &defender, winner, margin, was_tie);
    ContestedCheckResult { attacker, defender, winner, margin, was_tie, message }
}
```

Difficulty handling: `skill_check` already computes `outcome.successes` regardless of input difficulty. The contested layer ignores each side's individual difficulty when comparing — contested resolution is purely successes-vs-successes. (Callers can still pass a difficulty to each side if they want the bestial / messy / critical flags computed against a meaningful DV; for pure contested, set both to 0.)

Message format: `"<Attacker name> vs <Defender name> · <Attacker|Defender> wins · margin <N>[· tie]"`.

### §5.4 Issue #11 open question — opponent input shape

The issue asks: "auto-select by selected character vs manual stat entry". **Resolved**: the backend stays neutral. Both sides arrive as `SkillCheckInput`. The frontend at consumer time picks the input mode — typically:
- A "selected character vs selected character" path that derives both sides from the character store.
- A "selected character vs manual stats" path for off-sheet opponents (mortals, ad-hoc NPCs).
- An "all-manual" path for GM-imagined scenarios.

The decision is per-feature UI and does not belong in the helper.

### §5.5 Tests

**Pure-resolver tests (no RNG):**
- `resolve_contested(5, 3, Defender, 0)` → `(Attacker, 2, false)`.
- `resolve_contested(2, 4, Defender, 0)` → `(Defender, 2, false)`.
- `resolve_contested(4, 4, Defender, 0)` → `(Defender, 1, true)` — tie default.
- `resolve_contested(4, 4, Attacker, 3)` → `(Attacker, 4, true)` — tie + weapon damage.
- `resolve_contested(0, 0, Defender, 0)` → `(Defender, 1, true)` — both whiff.

**Orchestrator tests (seeded RNG):**
- Both sides' `SkillCheckResult` fields are populated (spec, roll, tally, outcome, message).
- Deterministic with seeded `StdRng`: same seed → same winner, margin, was_tie, message.
- Each side's roll respects its own input (verify by passing distinct attribute/skill/hunger and checking the resulting `SkillCheckResult.spec.parts` and pool sizes).

---

## §6 IPC layer

Three new commands in `src-tauri/src/tools/`:

```rust
// tools/rouse.rs
#[tauri::command]
pub fn roll_rouse_check(input: RouseCheckInput) -> RouseCheckResult {
    let mut rng = rand::thread_rng();
    shared::v5::rouse::rouse_check(&input, &mut rng)
}

// tools/remorse.rs   (analogous)
// tools/contested.rs (analogous)
```

Registered in `src-tauri/src/lib.rs` alongside the existing `roll_skill_check`. All three are synchronous (`pub fn`, not `pub async fn`) — no I/O.

### §6.1 TS wrappers (`src/lib/v5/api.ts`)

Add three exported interfaces (camelCase mirroring Rust's `rename_all`) and three async wrapper functions:

```ts
export async function rollRouseCheck(input: RouseCheckInput): Promise<RouseCheckResult>;
export async function rollRemorseCheck(input: RemorseCheckInput): Promise<RemorseCheckResult>;
export async function rollContestedCheck(input: ContestedCheckInput): Promise<ContestedCheckResult>;
```

---

## §7 Out of scope

Deferred intentionally to keep #11 focused on the issue text:

| Item | Reason | Where it belongs |
|---|---|---|
| Multi-opponent defenses (dodge-all, fight-all-back, mix) | Combat-flow primitives, not single rolls | Future combat-tracker feature spec |
| Frenzy checks | Not in issue title | Separate dice composite if/when needed |
| Initiative rolls | Optional in V5, not requested | — |
| Bestial Failure → Remorse trigger chain | Game-flow composition, not a helper | Consumer feature (character-sheet "resolve bestial" flow) |
| Roll-history ring integration | Local rolls don't push to the ring today; unifying belongs to #9 / #12 follow-ups | Issue #9 (roll-source toggle) and a small #12 follow-up |

**Continuity note on the Rolls tool:** the existing `roll_skill_check` command does not push into the bridge `roll_history` ring — only Foundry-mirrored rolls reach the Rolls tool today. Shipping #11 does NOT introduce this gap; it inherits it. Local rouse/remorse/contested rolls will likewise be invisible to the Rolls tool until the #9 path lands. This is a known continuity, not a regression.
| Character-state mutation (apply `hunger_delta`, `humanity_delta`) | Composites return deltas as data | Consumer features writing through `character_set_field` |
| BP-driven rouse re-roll policy | `extra_dice` exposes the mechanic; policy of "which Rouses re-roll at which BP" is character-context | Consumer features (Discipline-activation button decides) |
| WoD5e parity (mirror these into Foundry via `game.roll_v5_pool`) | Foundry side already owns rouse / remorse internally | Issue #9 path (Foundry roll dispatch) |

---

## §8 Verification

- `./scripts/verify.sh` must stay green after each commit (`npm run check`, `cargo check`, `cargo test`, frontend build).
- Cargo tests added per file (rouse: 6 cases; remorse: 7 cases; contested: 6 cases — estimated; exact counts in the plan).
- No new TypeScript runtime tests in this scope; the TS wrappers are thin `invoke` calls and gain coverage from manual smoke + future consumer-feature tests.
- Manual smoke: in `npm run tauri dev`, call each new command from the devtools console (`window.__TAURI__.core.invoke('roll_rouse_check', { input: { characterName: 'Test', extraDice: 0 } })`) and confirm shape round-trip.

---

## §9 Risks and non-regressions

- Pure-additive change to `shared/v5/`. No existing function signature changes.
- `mod.rs` additions are append-only.
- `lib.rs` command registry gains three lines.
- TS `api.ts` gains three interfaces + three wrappers — additive.
- No DB schema changes.
- No bridge protocol changes.
- No frontend route changes.

Existing skill-check tests, foundry roll-mirror tests, character-card tests, GM-screen tests, and bridge tests should all remain unaffected.

---

## §10 Open questions (deferred — not blocking implementation)

1. **Roll-history unification.** When (or whether) to push local `roll_skill_check` / `roll_rouse_check` / etc. results into the bridge `roll_history` ring so the Rolls tool shows them. Default: defer to #9 (roll-source toggle), which is the natural feature where "where did this roll come from" becomes a first-class concept.
2. **Aggregated turn helpers.** A "blood action" composite that bundles up to three Rouse Checks (Blood Surge + Blood Heal + Discipline activation) per turn — out of scope here, useful for the GM screen later.
3. **Rouse "reasons" enum.** Tag each Rouse with what caused it (BloodSurge, BloodHeal, Discipline, Bestial, etc.) for log filtering. Probably belongs on the Rolls-tool side, not the helper.

---

## §11 References

- Phase 1 primitives: `src-tauri/src/shared/v5/{types,pool,dice,interpret,difficulty,message,skill_check}.rs`
- V5 dice mechanics: `docs/reference/v5-combat-rules.md`
- Character-tooling roadmap (origin): `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 3
- Foundry helper roadmap (architectural sibling): `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`
- ARCHITECTURE.md §9 "Add a tool" and §10 testing invariant
