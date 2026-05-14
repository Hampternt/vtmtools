# Hunger / Remorse / Contested Roll Composites — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship three V5 dice composites (`rouse_check`, `remorse_check`, `contested_check`) in `src-tauri/src/shared/v5/`, exposed via three Tauri commands and three TS wrappers, with no UI consumer yet — purely a helper-library extension.

**Architecture:** Each composite is a self-contained module under `shared/v5/` that reuses existing Phase 1 leaves (`roll_pool`, `interpret`, `compare`) where applicable. Pure evaluator/resolver functions are extracted from RNG-coupled orchestrators so each can be unit-tested directly. Side-effects (Hunger +1, Humanity −1, Stains→0) are returned as typed deltas in the result; consumers apply them.

**Tech Stack:** Rust (Tauri backend, `rand` for RNG, `serde` for IPC), TypeScript (Tauri `invoke` wrappers).

**Source spec:** `docs/superpowers/specs/2026-05-12-hunger-remorse-contested-rolls-design.md`

**GitHub issue:** #11 — commits use `Refs #11` footer (not `Closes #11`; the issue closes when a consumer feature lands).

**Hard rule reminder:** every task ending in a commit MUST run `./scripts/verify.sh` as the step immediately before the commit step. Expected non-regression warnings are documented in `ARCHITECTURE.md` §10 — do NOT "fix" them.

**Per-task execution model:** one implementer subagent per task. No per-task reviewer subagent. A single full-branch `code-review:code-review` runs after Task 6 commits.

---

## File Structure

**Created files:**
- `src-tauri/src/shared/v5/rouse.rs` — pure `evaluate_rouse` + RNG orchestrator `rouse_check` + tests
- `src-tauri/src/shared/v5/remorse.rs` — `remorse_check` orchestrator (reuses Phase 1 leaves) + tests
- `src-tauri/src/shared/v5/contested.rs` — pure `resolve_contested` + RNG orchestrator `contested_check` + tests
- `src-tauri/src/tools/rouse.rs` — Tauri command wrapping `rouse_check` with `thread_rng()`
- `src-tauri/src/tools/remorse.rs` — Tauri command wrapping `remorse_check` with `thread_rng()`
- `src-tauri/src/tools/contested.rs` — Tauri command wrapping `contested_check` with `thread_rng()`

**Modified files:**
- `src-tauri/src/shared/v5/types.rs` — append 3 input + 3 result struct pairs + `ContestedSide` enum
- `src-tauri/src/shared/v5/mod.rs` — append `pub mod rouse; pub mod remorse; pub mod contested;`
- `src-tauri/src/tools/mod.rs` — append `pub mod rouse; pub mod remorse; pub mod contested;`
- `src-tauri/src/lib.rs` — register 3 new commands in the `invoke_handler!` block
- `src/lib/v5/api.ts` — append 3 new TS interfaces + 3 new wrapper functions

---

## Task 1 — Add types to `shared/v5/types.rs`

**Tests:** none in this task (types-only; tests live in their composite modules).
**Files:**
- Modify: `src-tauri/src/shared/v5/types.rs` (append at end of file)

- [ ] **Step 1: Append the three input/result struct pairs and `ContestedSide` enum to the end of `src-tauri/src/shared/v5/types.rs`.**

```rust
// ---------- Rouse check ----------

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
    pub best: u8,
    pub passed: bool,
    /// 0 on pass, 1 on fail. Caller applies to character state.
    pub hunger_delta: u8,
    pub message: String,
}

// ---------- Remorse check ----------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemorseCheckInput {
    pub character_name: Option<String>,
    pub humanity: u8,
    pub stains: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemorseCheckResult {
    pub spec: PoolSpec,
    pub roll: RollResult,
    pub tally: Tally,
    pub outcome: Outcome,
    /// -1 on failure, 0 on pass.
    pub humanity_delta: i8,
    /// Always 0 — WoD5e clears stains regardless of outcome.
    pub stains_after: u8,
    pub message: String,
}

// ---------- Contested check ----------

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
    /// Bonus added to margin on a tie. Combat callers pass weapon damage; non-combat pass 0.
    pub tie_margin_bonus: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContestedCheckResult {
    pub attacker: SkillCheckResult,
    pub defender: SkillCheckResult,
    pub winner: ContestedSide,
    /// On non-tie: |attacker.successes - defender.successes|. On tie: 1 + tie_margin_bonus.
    pub margin: i32,
    pub was_tie: bool,
    pub message: String,
}
```

- [ ] **Step 2: Run cargo check to confirm types compile.**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: compiles cleanly. Unused-struct lints are silent for `pub` types — fine.

- [ ] **Step 3: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: green. Documented non-regression warnings (ARCHITECTURE.md §10) may appear; do not "fix" them.

- [ ] **Step 4: Commit.**

```bash
git add src-tauri/src/shared/v5/types.rs
git commit -m "$(cat <<'EOF'
feat(v5): add Rouse/Remorse/Contested check types

Pure-types task — input/result structs and ContestedSide enum for the
three Phase 3 composite rolls. Orchestrators land in follow-up commits.

Refs #11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2 — Implement `rouse.rs`

**Tests: required** — Rouse logic is real V5 dice mechanics (pass/fail threshold, hunger delta, multi-die take-best).

**Files:**
- Create: `src-tauri/src/shared/v5/rouse.rs`
- Modify: `src-tauri/src/shared/v5/mod.rs` (append `pub mod rouse;`)

- [ ] **Step 1: Append the module declaration to `src-tauri/src/shared/v5/mod.rs`.**

Edit the file by adding one line at the end (preserving the trailing newline):

```rust
pub mod rouse;
```

The final state of `mod.rs` should be:

```rust
pub mod types;
pub mod pool;
pub mod dice;
pub mod interpret;
pub mod difficulty;
pub mod message;
pub mod skill_check;
pub mod rouse;
```

- [ ] **Step 2: Create `src-tauri/src/shared/v5/rouse.rs` with the pure helper, the RNG orchestrator, the private formatter, and the test module.**

```rust
//! Rouse check — 1d10 (or N d10 take-best) vs 6+. Fail bumps Hunger by 1.
//!
//! Does not use `PoolSpec`/`RollResult` because Rouse has no PoolPart/Hunger
//! semantics — it's a bare dice roll. The `dice: Vec<u8>` field preserves
//! raw rolls so a future consumer can reconstruct "first try" vs "re-rolled
//! into success" without the helper baking in a policy.

use crate::shared::v5::types::{RouseCheckInput, RouseCheckResult};
use rand::Rng;

/// Pure evaluator: given rolled dice, produce (best, passed, hunger_delta).
/// Extracted from `rouse_check` so logic is unit-testable without seeded RNG.
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

fn format_rouse(
    input: &RouseCheckInput,
    dice: &[u8],
    best: u8,
    passed: bool,
) -> String {
    let head = match input.character_name.as_deref() {
        Some(name) if !name.is_empty() => format!("{}'s Rouse check", name),
        _ => "Rouse check".to_string(),
    };
    let verdict = if passed { "Pass" } else { "Fail" };
    let dice_str = dice.iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} · {} · best {} (rolled [{}])", head, verdict, best, dice_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    // ---- Pure-evaluator tests (no RNG) ----

    #[test]
    fn evaluator_boundary_pass_at_six() {
        let (best, passed, delta) = evaluate_rouse(&[6]);
        assert_eq!(best, 6);
        assert!(passed);
        assert_eq!(delta, 0);
    }

    #[test]
    fn evaluator_boundary_fail_at_five() {
        let (best, passed, delta) = evaluate_rouse(&[5]);
        assert_eq!(best, 5);
        assert!(!passed);
        assert_eq!(delta, 1);
    }

    #[test]
    fn evaluator_single_die_extremes() {
        assert_eq!(evaluate_rouse(&[1]), (1, false, 1));
        assert_eq!(evaluate_rouse(&[10]), (10, true, 0));
    }

    #[test]
    fn evaluator_take_best_pass_when_any_die_passes() {
        let (best, passed, delta) = evaluate_rouse(&[3, 8]);
        assert_eq!(best, 8);
        assert!(passed);
        assert_eq!(delta, 0);
    }

    #[test]
    fn evaluator_take_best_fail_when_all_dice_fail() {
        let (best, passed, delta) = evaluate_rouse(&[2, 4, 5]);
        assert_eq!(best, 5);
        assert!(!passed);
        assert_eq!(delta, 1);
    }

    // ---- Orchestrator tests (seeded RNG) ----

    fn input(extra_dice: u8, name: Option<&str>) -> RouseCheckInput {
        RouseCheckInput {
            character_name: name.map(String::from),
            extra_dice,
        }
    }

    #[test]
    fn orchestrator_standard_rolls_one_die() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = rouse_check(&input(0, None), &mut rng);
        assert_eq!(r.dice.len(), 1);
        assert_eq!(r.best, r.dice[0]);
    }

    #[test]
    fn orchestrator_extra_one_rolls_two_dice() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = rouse_check(&input(1, None), &mut rng);
        assert_eq!(r.dice.len(), 2);
        assert_eq!(r.best, *r.dice.iter().max().unwrap());
    }

    #[test]
    fn orchestrator_extra_three_rolls_four_dice() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = rouse_check(&input(3, None), &mut rng);
        assert_eq!(r.dice.len(), 4);
    }

    #[test]
    fn orchestrator_deterministic_with_seed() {
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);
        let a = rouse_check(&input(1, Some("Charlotte")), &mut rng_a);
        let b = rouse_check(&input(1, Some("Charlotte")), &mut rng_b);
        assert_eq!(a.dice, b.dice);
        assert_eq!(a.best, b.best);
        assert_eq!(a.passed, b.passed);
        assert_eq!(a.message, b.message);
    }

    #[test]
    fn orchestrator_message_includes_character_name() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = rouse_check(&input(0, Some("Charlotte")), &mut rng);
        assert!(r.message.contains("Charlotte"));
    }

    #[test]
    fn orchestrator_message_omits_owner_when_name_is_none() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = rouse_check(&input(0, None), &mut rng);
        assert!(r.message.starts_with("Rouse check"));
    }
}
```

- [ ] **Step 3: Run the rouse test module to confirm all 11 tests pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::v5::rouse::tests --quiet
```

Expected: `test result: ok. 11 passed; 0 failed`.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit.**

```bash
git add src-tauri/src/shared/v5/rouse.rs src-tauri/src/shared/v5/mod.rs
git commit -m "$(cat <<'EOF'
feat(v5): add rouse_check composite

Single-die (or N-die take-best) hunger test vs 6+. Pure evaluate_rouse
helper is RNG-free for direct unit testing; orchestrator delegates to it
after rolling. Raw dice preserved in result so consumers can reconstruct
first-try-vs-reroll without the helper baking policy.

Refs #11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3 — Implement `remorse.rs`

**Tests: required** — V5 Remorse pool formula (`max(10 − humanity − stains, 1)`) is the load-bearing mechanic; verified against WoD5e Foundry source `system/actor/vtm/scripts/roll-remorse.js:23`.

**Files:**
- Create: `src-tauri/src/shared/v5/remorse.rs`
- Modify: `src-tauri/src/shared/v5/mod.rs` (append `pub mod remorse;`)

- [ ] **Step 1: Append the module declaration to `src-tauri/src/shared/v5/mod.rs`.**

After this step, `mod.rs` should end with:

```rust
pub mod rouse;
pub mod remorse;
```

- [ ] **Step 2: Create `src-tauri/src/shared/v5/remorse.rs`.**

```rust
//! Remorse check — pool = max(10 - humanity - stains, 1) regular dice, DV 1.
//! Pass: stains clear, humanity unchanged. Fail: stains clear AND humanity -1.
//!
//! Pool formula sourced from WoD5e Foundry module
//! (system/actor/vtm/scripts/roll-remorse.js:23).

use crate::shared::v5::dice::roll_pool;
use crate::shared::v5::difficulty::compare;
use crate::shared::v5::interpret::interpret;
use crate::shared::v5::types::{
    Outcome, PoolPart, PoolSpec, RemorseCheckInput, RemorseCheckResult,
};
use rand::Rng;

pub fn remorse_check<R: Rng + ?Sized>(
    input: &RemorseCheckInput,
    rng: &mut R,
) -> RemorseCheckResult {
    // Pool = max(10 - humanity - stains, 1). Compute in i16 to avoid underflow.
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
    let humanity_delta: i8 = if outcome.passed { 0 } else { -1 };
    let message = format_remorse(input, &outcome);
    RemorseCheckResult {
        spec, roll, tally, outcome,
        humanity_delta,
        stains_after: 0,
        message,
    }
}

fn format_remorse(input: &RemorseCheckInput, outcome: &Outcome) -> String {
    let head = match input.character_name.as_deref() {
        Some(name) if !name.is_empty() => format!("{}'s Remorse check", name),
        _ => "Remorse check".to_string(),
    };
    let descriptor = if outcome.passed { "Success" } else { "Failure · Humanity −1" };
    format!("{} · {} · {} successes", head, descriptor, outcome.successes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn input(humanity: u8, stains: u8) -> RemorseCheckInput {
        RemorseCheckInput {
            character_name: Some("Charlotte".into()),
            humanity,
            stains,
        }
    }

    #[test]
    fn pool_size_full_when_humanity_zero_stains_zero() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = remorse_check(&input(0, 0), &mut rng);
        assert_eq!(r.spec.regular_count, 10);
        assert_eq!(r.spec.hunger_count, 0);
    }

    #[test]
    fn pool_size_one_when_humanity_ten_stains_zero() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = remorse_check(&input(10, 0), &mut rng);
        assert_eq!(r.spec.regular_count, 1);
    }

    #[test]
    fn pool_size_uses_formula_for_normal_case() {
        let mut rng = StdRng::seed_from_u64(1);
        // 10 - 5 - 3 = 2
        let r = remorse_check(&input(5, 3), &mut rng);
        assert_eq!(r.spec.regular_count, 2);
    }

    #[test]
    fn pool_clamped_to_one_when_formula_would_be_negative() {
        let mut rng = StdRng::seed_from_u64(1);
        // 10 - 7 - 5 = -2 → clamp to 1
        let r = remorse_check(&input(7, 5), &mut rng);
        assert_eq!(r.spec.regular_count, 1);
    }

    #[test]
    fn pool_never_has_hunger_dice() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = remorse_check(&input(5, 2), &mut rng);
        assert_eq!(r.spec.hunger_count, 0);
        for d in &r.roll.dice {
            assert_eq!(
                d.kind,
                crate::shared::v5::types::DieKind::Regular,
                "remorse must roll only regular dice"
            );
        }
    }

    #[test]
    fn difficulty_is_one() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = remorse_check(&input(5, 2), &mut rng);
        assert_eq!(r.outcome.difficulty, 1);
    }

    #[test]
    fn stains_after_is_always_zero() {
        // Try both a likely-pass seed and a likely-fail seed.
        let mut rng = StdRng::seed_from_u64(1);
        let r1 = remorse_check(&input(5, 2), &mut rng);
        assert_eq!(r1.stains_after, 0);

        let mut rng2 = StdRng::seed_from_u64(99);
        let r2 = remorse_check(&input(10, 0), &mut rng2);  // tiny pool, fail-prone
        assert_eq!(r2.stains_after, 0);
    }

    #[test]
    fn humanity_delta_matches_pass_fail() {
        // Sweep seeds until we observe each outcome path at least once.
        let mut saw_pass = false;
        let mut saw_fail = false;
        for seed in 0..200u64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let r = remorse_check(&input(10, 0), &mut rng);  // pool = 1, both paths reachable
            if r.outcome.passed {
                assert_eq!(r.humanity_delta, 0, "pass must have humanity_delta = 0");
                saw_pass = true;
            } else {
                assert_eq!(r.humanity_delta, -1, "fail must have humanity_delta = -1");
                saw_fail = true;
            }
            if saw_pass && saw_fail { return; }
        }
        panic!("did not observe both pass and fail in 200 seeds — increase range or check RNG");
    }

    #[test]
    fn message_includes_character_name() {
        let mut rng = StdRng::seed_from_u64(1);
        let r = remorse_check(&input(5, 2), &mut rng);
        assert!(r.message.contains("Charlotte"));
    }
}
```

- [ ] **Step 3: Run the remorse test module to confirm all 9 tests pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::v5::remorse::tests --quiet
```

Expected: `test result: ok. 9 passed; 0 failed`.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit.**

```bash
git add src-tauri/src/shared/v5/remorse.rs src-tauri/src/shared/v5/mod.rs
git commit -m "$(cat <<'EOF'
feat(v5): add remorse_check composite

Pool = max(10 - humanity - stains, 1) regular dice, DV 1. Pass keeps
humanity, fail drops it by 1. Stains clear (stains_after = 0) regardless
of outcome, matching WoD5e Foundry behavior. Formula verified against
system/actor/vtm/scripts/roll-remorse.js:23.

Refs #11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4 — Implement `contested.rs`

**Tests: required** — Tie handling and winner-determination is real V5 combat mechanics.

**Files:**
- Create: `src-tauri/src/shared/v5/contested.rs`
- Modify: `src-tauri/src/shared/v5/mod.rs` (append `pub mod contested;`)

- [ ] **Step 1: Append the module declaration to `src-tauri/src/shared/v5/mod.rs`.**

After this step, `mod.rs` should end with:

```rust
pub mod rouse;
pub mod remorse;
pub mod contested;
```

- [ ] **Step 2: Create `src-tauri/src/shared/v5/contested.rs`.**

```rust
//! Contested check — two `skill_check` runs, winner by successes, tie handling
//! parameterized. V5 combat default: tie goes to defender, margin = 1 + weapon
//! damage (caller passes weapon damage as `tie_margin_bonus`).

use crate::shared::v5::skill_check::skill_check;
use crate::shared::v5::types::{
    ContestedCheckInput, ContestedCheckResult, ContestedSide, SkillCheckResult,
};
use rand::Rng;

/// Pure resolver: given both sides' success counts, produce (winner, margin, was_tie).
/// Extracted from `contested_check` so resolution logic is RNG-free for testing.
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
    let message = format_contested(&attacker, &defender, winner, margin, was_tie);
    ContestedCheckResult {
        attacker,
        defender,
        winner,
        margin,
        was_tie,
        message,
    }
}

fn format_contested(
    attacker: &SkillCheckResult,
    defender: &SkillCheckResult,
    winner: ContestedSide,
    margin: i32,
    was_tie: bool,
) -> String {
    let a_name = attacker
        .roll
        .parts
        .first()
        .map(|p| p.name.as_str())
        .unwrap_or("Attacker");
    let d_name = defender
        .roll
        .parts
        .first()
        .map(|p| p.name.as_str())
        .unwrap_or("Defender");
    let winner_str = match winner {
        ContestedSide::Attacker => "Attacker",
        ContestedSide::Defender => "Defender",
    };
    let tie_suffix = if was_tie { " · tie" } else { "" };
    format!(
        "{} vs {} · {} wins · margin {}{}",
        a_name, d_name, winner_str, margin, tie_suffix
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::v5::types::{PoolPart, SkillCheckInput};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    // ---- Pure-resolver tests (no RNG) ----

    #[test]
    fn resolver_attacker_wins_with_more_successes() {
        let (winner, margin, was_tie) =
            resolve_contested(5, 3, ContestedSide::Defender, 0);
        assert_eq!(winner, ContestedSide::Attacker);
        assert_eq!(margin, 2);
        assert!(!was_tie);
    }

    #[test]
    fn resolver_defender_wins_with_more_successes() {
        let (winner, margin, was_tie) =
            resolve_contested(2, 4, ContestedSide::Defender, 0);
        assert_eq!(winner, ContestedSide::Defender);
        assert_eq!(margin, 2);
        assert!(!was_tie);
    }

    #[test]
    fn resolver_tie_default_goes_to_defender_with_margin_one() {
        let (winner, margin, was_tie) =
            resolve_contested(4, 4, ContestedSide::Defender, 0);
        assert_eq!(winner, ContestedSide::Defender);
        assert_eq!(margin, 1);
        assert!(was_tie);
    }

    #[test]
    fn resolver_tie_with_attacker_winner_and_weapon_damage() {
        let (winner, margin, was_tie) =
            resolve_contested(4, 4, ContestedSide::Attacker, 3);
        assert_eq!(winner, ContestedSide::Attacker);
        assert_eq!(margin, 4);   // 1 + 3
        assert!(was_tie);
    }

    #[test]
    fn resolver_double_whiff_is_tie() {
        let (winner, margin, was_tie) =
            resolve_contested(0, 0, ContestedSide::Defender, 0);
        assert_eq!(winner, ContestedSide::Defender);
        assert_eq!(margin, 1);
        assert!(was_tie);
    }

    // ---- Orchestrator tests (seeded RNG) ----

    fn skill_input(attr_level: u8, skill_level: u8, hunger: u8) -> SkillCheckInput {
        SkillCheckInput {
            character_name: None,
            attribute: PoolPart { name: "Strength".into(), level: attr_level },
            skill: PoolPart { name: "Brawl".into(), level: skill_level },
            hunger,
            specialty: None,
            difficulty: 0,
        }
    }

    #[test]
    fn orchestrator_populates_both_sides_full_results() {
        let mut rng = StdRng::seed_from_u64(7);
        let input = ContestedCheckInput {
            attacker: skill_input(4, 3, 1),
            defender: skill_input(3, 2, 0),
            tie_goes_to: ContestedSide::Defender,
            tie_margin_bonus: 0,
        };
        let r = contested_check(&input, &mut rng);
        // Both SkillCheckResult fields populated.
        assert_eq!(r.attacker.spec.regular_count + r.attacker.spec.hunger_count, 7);
        assert_eq!(r.defender.spec.regular_count + r.defender.spec.hunger_count, 5);
        assert!(!r.attacker.message.is_empty());
        assert!(!r.defender.message.is_empty());
    }

    #[test]
    fn orchestrator_each_side_uses_its_own_input() {
        let mut rng = StdRng::seed_from_u64(7);
        let input = ContestedCheckInput {
            attacker: skill_input(5, 5, 2),
            defender: skill_input(1, 1, 0),
            tie_goes_to: ContestedSide::Defender,
            tie_margin_bonus: 0,
        };
        let r = contested_check(&input, &mut rng);
        // Attacker pool = 10 (5 reg + 5 base, with 2 hunger swapped in → 8 reg + 2 hunger).
        assert_eq!(r.attacker.spec.regular_count, 8);
        assert_eq!(r.attacker.spec.hunger_count, 2);
        // Defender pool = 2 reg, 0 hunger.
        assert_eq!(r.defender.spec.regular_count, 2);
        assert_eq!(r.defender.spec.hunger_count, 0);
    }

    #[test]
    fn orchestrator_deterministic_with_seed() {
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);
        let input = ContestedCheckInput {
            attacker: skill_input(4, 3, 1),
            defender: skill_input(3, 2, 0),
            tie_goes_to: ContestedSide::Defender,
            tie_margin_bonus: 0,
        };
        let a = contested_check(&input, &mut rng_a);
        let b = contested_check(&input, &mut rng_b);
        assert_eq!(a.winner, b.winner);
        assert_eq!(a.margin, b.margin);
        assert_eq!(a.was_tie, b.was_tie);
        assert_eq!(a.message, b.message);
    }
}
```

- [ ] **Step 3: Run the contested test module to confirm all 8 tests pass.**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::v5::contested::tests --quiet
```

Expected: `test result: ok. 8 passed; 0 failed`.

- [ ] **Step 4: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 5: Commit.**

```bash
git add src-tauri/src/shared/v5/contested.rs src-tauri/src/shared/v5/mod.rs
git commit -m "$(cat <<'EOF'
feat(v5): add contested_check composite

Two skill_check runs with success-count comparison. Pure resolve_contested
helper is RNG-free for direct unit testing; tie handling is parameterized
(tie_goes_to + tie_margin_bonus) so combat callers can pass weapon damage
on tie per V5 rules.

Refs #11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5 — Wire Tauri commands

**Tests: not required** (mechanical IPC wiring; behavior already covered by the shared/v5 tests).

**Files:**
- Create: `src-tauri/src/tools/rouse.rs`
- Create: `src-tauri/src/tools/remorse.rs`
- Create: `src-tauri/src/tools/contested.rs`
- Modify: `src-tauri/src/tools/mod.rs` (append three `pub mod` lines)
- Modify: `src-tauri/src/lib.rs` (register three commands in the `invoke_handler!` block)

- [ ] **Step 1: Create `src-tauri/src/tools/rouse.rs`.**

```rust
//! Tauri command wrapping shared::v5::rouse::rouse_check with thread_rng().
//! Synchronous — no I/O.

use crate::shared::v5::rouse::rouse_check;
use crate::shared::v5::types::{RouseCheckInput, RouseCheckResult};

#[tauri::command]
pub fn roll_rouse_check(input: RouseCheckInput) -> RouseCheckResult {
    let mut rng = rand::thread_rng();
    rouse_check(&input, &mut rng)
}
```

- [ ] **Step 2: Create `src-tauri/src/tools/remorse.rs`.**

```rust
//! Tauri command wrapping shared::v5::remorse::remorse_check with thread_rng().
//! Synchronous — no I/O.

use crate::shared::v5::remorse::remorse_check;
use crate::shared::v5::types::{RemorseCheckInput, RemorseCheckResult};

#[tauri::command]
pub fn roll_remorse_check(input: RemorseCheckInput) -> RemorseCheckResult {
    let mut rng = rand::thread_rng();
    remorse_check(&input, &mut rng)
}
```

- [ ] **Step 3: Create `src-tauri/src/tools/contested.rs`.**

```rust
//! Tauri command wrapping shared::v5::contested::contested_check with thread_rng().
//! Synchronous — no I/O.

use crate::shared::v5::contested::contested_check;
use crate::shared::v5::types::{ContestedCheckInput, ContestedCheckResult};

#[tauri::command]
pub fn roll_contested_check(input: ContestedCheckInput) -> ContestedCheckResult {
    let mut rng = rand::thread_rng();
    contested_check(&input, &mut rng)
}
```

- [ ] **Step 4: Append the three module declarations to `src-tauri/src/tools/mod.rs`.**

After this step, `tools/mod.rs` should be:

```rust
pub mod resonance;
pub mod skill_check;
pub mod export;
pub mod foundry_chat;
pub mod character;
pub mod gm_screen;
pub mod rouse;
pub mod remorse;
pub mod contested;
```

- [ ] **Step 5: Register the three new commands in `src-tauri/src/lib.rs`.**

Locate the existing line `tools::skill_check::roll_skill_check,` in the `invoke_handler!` block (around line 68). Add three lines immediately after it so the block reads:

```rust
            tools::skill_check::roll_skill_check,
            tools::rouse::roll_rouse_check,
            tools::remorse::roll_remorse_check,
            tools::contested::roll_contested_check,
```

- [ ] **Step 6: Run cargo check to confirm Rust side wires correctly.**

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: clean compile.

- [ ] **Step 7: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: green. (Frontend build will not yet exercise the new commands; that lands in Task 6.)

- [ ] **Step 8: Commit.**

```bash
git add src-tauri/src/tools/rouse.rs src-tauri/src/tools/remorse.rs src-tauri/src/tools/contested.rs src-tauri/src/tools/mod.rs src-tauri/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(v5): expose rouse/remorse/contested via Tauri commands

Three sync IPC commands wrapping the shared::v5 composites with
thread_rng(). Registered alongside the existing roll_skill_check
in the invoke_handler block.

Refs #11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6 — Add TS wrappers and interfaces

**Tests: not required** (thin `invoke` wrappers; behavior covered by the Rust tests).

**Files:**
- Modify: `src/lib/v5/api.ts` (append 3 interfaces + 3 wrapper functions)

- [ ] **Step 1: Append the new TypeScript interfaces and wrapper functions to `src/lib/v5/api.ts`.**

Add this block at the end of the file:

```ts
// ---------- Rouse check ----------

export interface RouseCheckInput {
  characterName: string | null;
  /** 0 = standard 1d10. 1 = 2d10 take-highest. 2+ supported. */
  extraDice: number;
}

export interface RouseCheckResult {
  dice: number[];
  best: number;
  passed: boolean;
  /** 0 on pass, 1 on fail. Consumer applies to character state. */
  hungerDelta: number;
  message: string;
}

export async function rollRouseCheck(input: RouseCheckInput): Promise<RouseCheckResult> {
  return await invoke<RouseCheckResult>('roll_rouse_check', { input });
}

// ---------- Remorse check ----------

export interface RemorseCheckInput {
  characterName: string | null;
  humanity: number;
  stains: number;
}

export interface RemorseCheckResult {
  spec: PoolSpec;
  roll: RollResult;
  tally: Tally;
  outcome: Outcome;
  /** -1 on failure, 0 on pass. */
  humanityDelta: number;
  /** Always 0 — stains clear regardless of outcome. */
  stainsAfter: number;
  message: string;
}

export async function rollRemorseCheck(input: RemorseCheckInput): Promise<RemorseCheckResult> {
  return await invoke<RemorseCheckResult>('roll_remorse_check', { input });
}

// ---------- Contested check ----------

export type ContestedSide = 'attacker' | 'defender';

export interface ContestedCheckInput {
  attacker: SkillCheckInput;
  defender: SkillCheckInput;
  /** Who wins on equal successes. V5 combat default = 'defender'. */
  tieGoesTo: ContestedSide;
  /** Bonus added to margin on a tie. Combat callers pass weapon damage; non-combat pass 0. */
  tieMarginBonus: number;
}

export interface ContestedCheckResult {
  attacker: SkillCheckResult;
  defender: SkillCheckResult;
  winner: ContestedSide;
  /** On non-tie: |attacker.successes - defender.successes|. On tie: 1 + tieMarginBonus. */
  margin: number;
  wasTie: boolean;
  message: string;
}

export async function rollContestedCheck(input: ContestedCheckInput): Promise<ContestedCheckResult> {
  return await invoke<ContestedCheckResult>('roll_contested_check', { input });
}
```

- [ ] **Step 2: Run TypeScript type-check.**

```bash
npm run check
```

Expected: 0 errors.

- [ ] **Step 3: Run the full verification gate.**

```bash
./scripts/verify.sh
```

Expected: green.

- [ ] **Step 4: Commit.**

```bash
git add src/lib/v5/api.ts
git commit -m "$(cat <<'EOF'
feat(v5): add TS wrappers for rouse/remorse/contested

Three new typed invoke wrappers + interfaces mirroring the Rust IPC
contracts. Closes the round-trip; consumer features (character cards,
GM screen, future Rolls-tool integration) can now call these helpers.

Refs #11

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review Checklist

After all six task commits, run a single full-branch code-review:

- [ ] **`code-review:code-review` against the branch diff vs `master`.**

The review covers spec compliance, code quality, type drift, and verify.sh consistency across all six commits in one pass — matching the project's "lean per-task execution" workflow.

---

## Spec coverage map

| Spec section | Task |
|---|---|
| §1 Scope (rouse / remorse / contested helpers + Tauri + TS) | Tasks 1–6 |
| §2.1 File layout | Tasks 2–6 (each creates the listed file) |
| §2.2 Approach A | Implicit in task structure (no orchestrator generalization) |
| §3.2–§3.5 Rouse types / orchestrator / rationale / tests | Tasks 1–2 |
| §4.2–§4.5 Remorse types / orchestrator / edge cases / tests | Tasks 1, 3 |
| §5.2–§5.5 Contested types / orchestrator / open-question resolution / tests | Tasks 1, 4 |
| §6 IPC layer | Task 5 |
| §6.1 TS wrappers | Task 6 |
| §7 Out of scope | Honored by NOT having tasks for those items |
| §8 Verification | Each task's `verify.sh` step + final code-review |
| §9 Risks (pure-additive) | Every task is additive; no edits to existing function signatures |
| §10 Open questions | Deferred — none addressed in this plan |

---

## Notes for the executor

- **Do NOT `git add docs/superpowers/plans/` or `docs/superpowers/specs/`.** These directories are gitignored. Treat them as local-only workflow scratch.
- **Do NOT amend or rebase prior commits.** New commit per task per CLAUDE.md.
- **Do NOT skip `./scripts/verify.sh`.** The hard rule is project policy.
- **Do NOT close issue #11.** Commits use `Refs #11`; the user closes the issue when consumer features land.
- **Expected non-regression warnings** (per ARCHITECTURE.md §10) may appear in verify.sh output — do not "fix" them.
