# V5 Dice Helper Library Implementation Plan (Plan 3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a pure-functional V5 dice helper library at `src-tauri/src/shared/v5/`. Provides primitives (pool builder, dice roller, success/critical interpreter, difficulty comparator, message formatter) and a thin orchestrator (`skill_check`). Future tools (skill check UI, combat tool, conflict tool, hunger rolls, remorse rolls) compose the primitives directly.

**Architecture:** Two layers max. Each leaf is a pure function with `&mut impl rand::Rng` injection where dice randomness is needed (newer pattern than the existing `shared/dice.rs` which uses `thread_rng()` directly — RNG injection enables deterministic seeded tests). The orchestrator is shallow plumbing: 5 sequential leaf calls, no branches. A single `roll_skill_check` Tauri command exposes the orchestrator to the frontend; primitives stay Rust-internal until a future plan needs them externally.

**Tech Stack:** Rust (`rand`, `serde`, `tauri`), TypeScript / Svelte 5.

**Spec:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md`

**V5 mechanics reference:** `docs/reference/v5-combat-rules.md`

**Depends on:** none — fully independent. Can run in parallel with Plans 0/1/2 in separate worktrees.

---

## File structure

### New files
- `src-tauri/src/shared/v5/mod.rs` — re-exports
- `src-tauri/src/shared/v5/types.rs` — all shared types in one place
- `src-tauri/src/shared/v5/pool.rs` — `build_pool` + tests
- `src-tauri/src/shared/v5/dice.rs` — `roll_pool` + tests
- `src-tauri/src/shared/v5/interpret.rs` — `interpret` + tests
- `src-tauri/src/shared/v5/difficulty.rs` — `compare` + tests
- `src-tauri/src/shared/v5/message.rs` — `format_skill_check` + tests
- `src-tauri/src/shared/v5/skill_check.rs` — orchestrator + integration test
- `src-tauri/src/tools/skill_check.rs` — `roll_skill_check` Tauri command
- `src/lib/v5/api.ts` — typed wrapper

### Modified files
- `src-tauri/src/shared/mod.rs` — `pub mod v5;`
- `src-tauri/src/tools/mod.rs` — `pub mod skill_check;`
- `src-tauri/src/lib.rs` — register `tools::skill_check::roll_skill_check` in `invoke_handler!`

### Files explicitly NOT touched
- `src-tauri/src/bridge/**` (Plan 0)
- `src-tauri/src/db/**` (Plan 1)
- Any `.svelte` component (no UI consumer in Phase 1)
- `src-tauri/src/shared/dice.rs` and `shared/resonance.rs` (existing — leave alone)

---

## Task overview

| # | Task | Depends on |
|---|---|---|
| 1 | Scaffold `shared/v5/` directory: `mod.rs` + empty leaf modules | none |
| 2 | Define types in `shared/v5/types.rs` | 1 |
| 3 | Implement `build_pool` + tests in `shared/v5/pool.rs` | 2 |
| 4 | Implement `roll_pool` + tests in `shared/v5/dice.rs` | 2 |
| 5 | Implement `interpret` + tests in `shared/v5/interpret.rs` | 2 |
| 6 | Implement `compare` + tests in `shared/v5/difficulty.rs` | 2 |
| 7 | Implement `format_skill_check` + tests in `shared/v5/message.rs` | 2 |
| 8 | Implement `skill_check` orchestrator + integration test in `shared/v5/skill_check.rs` | 3, 4, 5, 6, 7 |
| 9 | Add `roll_skill_check` Tauri command in `tools/skill_check.rs` | 8 |
| 10 | Register command in `lib.rs` | 9 |
| 11 | Add typed wrapper in `src/lib/v5/api.ts` | 10 |
| 12 | Final verification gate | all |

Tasks 3, 4, 5, 6, 7 are independent of each other (each adds a non-overlapping leaf module + tests). Subagent dispatch can run them in parallel after Task 2.

---

## Task 1: Scaffold `shared/v5/` directory

**Files:**
- Create: `src-tauri/src/shared/v5/mod.rs`
- Create: `src-tauri/src/shared/v5/types.rs` (empty stub)
- Create: `src-tauri/src/shared/v5/pool.rs` (empty stub)
- Create: `src-tauri/src/shared/v5/dice.rs` (empty stub)
- Create: `src-tauri/src/shared/v5/interpret.rs` (empty stub)
- Create: `src-tauri/src/shared/v5/difficulty.rs` (empty stub)
- Create: `src-tauri/src/shared/v5/message.rs` (empty stub)
- Create: `src-tauri/src/shared/v5/skill_check.rs` (empty stub)
- Modify: `src-tauri/src/shared/mod.rs`

**Anti-scope:** No types, no functions, no tests yet. Stubs only.

**Depends on:** none

**Invariants cited:** ARCHITECTURE.md §5 (module organization).

- [ ] **Step 1: Create the seven leaf module stubs**

Each leaf file gets a one-line module-level doc comment. Example for `pool.rs`:

```rust
//! V5 dice pool assembly: combines an attribute, a skill, and an optional
//! specialty into a PoolSpec ready to roll. See spec §3.4.
```

Same shape for the other five leaves (replace the description appropriately):

- `dice.rs`: `//! Pure dice rolling. RNG-injected for deterministic tests.`
- `interpret.rs`: `//! Convert RollResult into a Tally (successes, crits, hunger flags).`
- `difficulty.rs`: `//! Compare a Tally against a difficulty to produce an Outcome.`
- `message.rs`: `//! Format the human-facing summary string for a skill check.`
- `skill_check.rs`: `//! Orchestrator: assembles → rolls → interprets → compares → formats.`
- `types.rs`: `//! All V5 helper types in one place. Cross-leaf shared shapes.`

- [ ] **Step 2: Create `shared/v5/mod.rs`**

```rust
pub mod types;
pub mod pool;
pub mod dice;
pub mod interpret;
pub mod difficulty;
pub mod message;
pub mod skill_check;
```

- [ ] **Step 3: Add `pub mod v5;` to `shared/mod.rs`**

Edit `src-tauri/src/shared/mod.rs`:

```rust
pub mod types;
pub mod dice;
pub mod resonance;
pub mod v5;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/shared/v5/ src-tauri/src/shared/mod.rs
git commit -m "feat(shared/v5): scaffold v5 helper module with leaf stubs (Plan 3 task 1)"
```

---

## Task 2: Define types in `shared/v5/types.rs`

**Files:**
- Modify: `src-tauri/src/shared/v5/types.rs`

**Anti-scope:** No functions yet. Just types + derives.

**Depends on:** Task 1

**Invariants cited:** ARCHITECTURE.md §2 (canonical domain types), §6 (`#[serde(rename_all = "camelCase")]` for cross-IPC types).

- [ ] **Step 1: Replace `types.rs` with the full type set**

```rust
//! All V5 helper types in one place. Cross-leaf shared shapes.

use serde::{Deserialize, Serialize};

/// One named contributor to a V5 dice pool — typically an Attribute or a Skill.
/// Specialty is represented as a synthetic `PoolPart` named "Specialty: <name>"
/// with `level: 1` so the dice-rolling step is uniform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolPart {
    pub name: String,
    pub level: u8,   // 0..=5 in V5; specialty contributions are always 1
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DieKind { Regular, Hunger }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Die {
    pub kind: DieKind,
    pub value: u8,   // 1..=10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolSpec {
    pub parts: Vec<PoolPart>,
    pub regular_count: u8,
    pub hunger_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollResult {
    pub parts: Vec<PoolPart>,
    /// Pool-order: regulars first, then hunger.
    pub dice: Vec<Die>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    pub successes: u8,           // dice ≥6 + 2*crit_pairs
    pub crit_pairs: u8,          // tens / 2
    pub is_critical: bool,       // crit_pairs ≥ 1
    pub is_messy_critical: bool, // critical AND ≥1 hunger 10
    pub has_hunger_one: bool,    // ≥1 hunger 1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutcomeFlags {
    pub critical: bool,
    pub messy: bool,
    pub bestial_failure: bool,   // !passed AND has_hunger_one
    pub total_failure: bool,     // successes == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub successes: u8,
    pub difficulty: u8,
    pub margin: i32,             // successes - difficulty
    pub passed: bool,
    pub flags: OutcomeFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCheckInput {
    pub character_name: Option<String>,   // for message formatting
    pub attribute: PoolPart,
    pub skill: PoolPart,
    pub hunger: u8,                       // 0..=5; 0 = mortal/non-vampire
    pub specialty: Option<String>,        // Some(name) → +1 die
    pub difficulty: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCheckResult {
    pub spec: PoolSpec,
    pub roll: RollResult,
    pub tally: Tally,
    pub outcome: Outcome,
    pub message: String,
}
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/shared/v5/types.rs
git commit -m "feat(shared/v5): define helper types (Plan 3 task 2)"
```

---

## Task 3: Implement `build_pool` + tests

**Files:**
- Modify: `src-tauri/src/shared/v5/pool.rs`

**Anti-scope:** Do NOT touch any other v5 leaf.

**Depends on:** Task 2.

**Invariants cited:** ARCHITECTURE.md §10 (`#[cfg(test)] mod tests` per file).

- [ ] **Step 1: Write the failing tests**

Replace `src-tauri/src/shared/v5/pool.rs` with:

```rust
//! V5 dice pool assembly.

use crate::shared::v5::types::{PoolPart, PoolSpec, SkillCheckInput};

pub fn build_pool(input: &SkillCheckInput) -> PoolSpec {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(attr: u8, skill: u8, hunger: u8, specialty: Option<&str>) -> SkillCheckInput {
        SkillCheckInput {
            character_name: None,
            attribute: PoolPart { name: "Strength".into(), level: attr },
            skill: PoolPart { name: "Brawl".into(), level: skill },
            hunger,
            specialty: specialty.map(String::from),
            difficulty: 0,
        }
    }

    #[test]
    fn pool_size_is_attribute_plus_skill() {
        let spec = build_pool(&input(3, 4, 0, None));
        assert_eq!(spec.regular_count + spec.hunger_count, 7);
        assert_eq!(spec.parts.len(), 2);
    }

    #[test]
    fn specialty_adds_one_die_with_labeled_part() {
        let spec = build_pool(&input(3, 4, 0, Some("bare-knuckle")));
        assert_eq!(spec.regular_count + spec.hunger_count, 8);
        assert_eq!(spec.parts.len(), 3);
        assert!(spec.parts[2].name.contains("bare-knuckle"));
        assert_eq!(spec.parts[2].level, 1);
    }

    #[test]
    fn hunger_replaces_regular_dice_one_for_one() {
        let spec = build_pool(&input(3, 4, 2, None));
        assert_eq!(spec.regular_count, 5);
        assert_eq!(spec.hunger_count, 2);
    }

    #[test]
    fn hunger_capped_at_pool_size() {
        let spec = build_pool(&input(2, 1, 5, None)); // pool 3, hunger 5
        assert_eq!(spec.regular_count, 0);
        assert_eq!(spec.hunger_count, 3);
    }

    #[test]
    fn zero_pool_is_zero_dice() {
        let spec = build_pool(&input(0, 0, 0, None));
        assert_eq!(spec.regular_count + spec.hunger_count, 0);
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::pool`
Expected: 5 fails (`todo!()` panics).

- [ ] **Step 2: Implement `build_pool`**

Replace the `todo!()` body:

```rust
pub fn build_pool(input: &SkillCheckInput) -> PoolSpec {
    let mut parts = Vec::with_capacity(3);
    parts.push(input.attribute.clone());
    parts.push(input.skill.clone());
    if let Some(name) = &input.specialty {
        parts.push(PoolPart {
            name: format!("Specialty: {}", name),
            level: 1,
        });
    }

    let pool_size: u8 = parts.iter().map(|p| p.level).sum();
    let hunger_count = input.hunger.min(pool_size);
    let regular_count = pool_size - hunger_count;

    PoolSpec { parts, regular_count, hunger_count }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::pool`
Expected: 5 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/v5/pool.rs
git commit -m "feat(shared/v5): implement build_pool with hunger clamp + specialty labeling (Plan 3 task 3)"
```

---

## Task 4: Implement `roll_pool` + tests

**Files:**
- Modify: `src-tauri/src/shared/v5/dice.rs`

**Anti-scope:** Do NOT touch other leaves.

**Depends on:** Task 2.

**Invariants cited:** ARCHITECTURE.md §10. RNG injection enables deterministic seeded tests.

- [ ] **Step 1: Write the failing tests**

Replace `src-tauri/src/shared/v5/dice.rs` with:

```rust
//! Pure dice rolling. RNG-injected for deterministic tests.

use crate::shared::v5::types::{Die, DieKind, PoolPart, PoolSpec, RollResult};
use rand::Rng;

pub fn roll_pool<R: Rng + ?Sized>(spec: &PoolSpec, rng: &mut R) -> RollResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn spec(reg: u8, hung: u8) -> PoolSpec {
        PoolSpec {
            parts: vec![
                PoolPart { name: "X".into(), level: reg + hung },
            ],
            regular_count: reg,
            hunger_count: hung,
        }
    }

    #[test]
    fn roll_count_matches_spec() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = roll_pool(&spec(5, 2), &mut rng);
        assert_eq!(r.dice.len(), 7);
    }

    #[test]
    fn regulars_first_then_hunger_in_pool_order() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = roll_pool(&spec(3, 2), &mut rng);
        assert_eq!(r.dice.len(), 5);
        assert_eq!(r.dice[0].kind, DieKind::Regular);
        assert_eq!(r.dice[1].kind, DieKind::Regular);
        assert_eq!(r.dice[2].kind, DieKind::Regular);
        assert_eq!(r.dice[3].kind, DieKind::Hunger);
        assert_eq!(r.dice[4].kind, DieKind::Hunger);
    }

    #[test]
    fn dice_values_are_one_through_ten() {
        let mut rng = StdRng::seed_from_u64(42);
        let r = roll_pool(&spec(10, 0), &mut rng);
        for d in &r.dice {
            assert!(d.value >= 1 && d.value <= 10, "die value out of range: {}", d.value);
        }
    }

    #[test]
    fn deterministic_with_same_seed() {
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);
        let a = roll_pool(&spec(5, 2), &mut rng_a);
        let b = roll_pool(&spec(5, 2), &mut rng_b);
        let av: Vec<_> = a.dice.iter().map(|d| d.value).collect();
        let bv: Vec<_> = b.dice.iter().map(|d| d.value).collect();
        assert_eq!(av, bv);
    }

    #[test]
    fn parts_are_copied_into_result() {
        let s = spec(3, 0);
        let parts_in = s.parts.clone();
        let mut rng = StdRng::seed_from_u64(1);
        let r = roll_pool(&s, &mut rng);
        assert_eq!(r.parts.len(), parts_in.len());
        assert_eq!(r.parts[0].name, parts_in[0].name);
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::dice`
Expected: 5 fails.

- [ ] **Step 2: Implement `roll_pool`**

Replace the `todo!()` body:

```rust
pub fn roll_pool<R: Rng + ?Sized>(spec: &PoolSpec, rng: &mut R) -> RollResult {
    let total = (spec.regular_count + spec.hunger_count) as usize;
    let mut dice = Vec::with_capacity(total);
    for _ in 0..spec.regular_count {
        dice.push(Die { kind: DieKind::Regular, value: rng.gen_range(1..=10) });
    }
    for _ in 0..spec.hunger_count {
        dice.push(Die { kind: DieKind::Hunger, value: rng.gen_range(1..=10) });
    }
    RollResult { parts: spec.parts.clone(), dice }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::dice`
Expected: 5 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/v5/dice.rs
git commit -m "feat(shared/v5): implement roll_pool with RNG injection (Plan 3 task 4)"
```

---

## Task 5: Implement `interpret` + tests

**Files:**
- Modify: `src-tauri/src/shared/v5/interpret.rs`

**Anti-scope:** Do NOT touch other leaves.

**Depends on:** Task 2.

**Invariants cited:** ARCHITECTURE.md §10. **V5 mechanics:** dice ≥6 = 1 success; pair of 10s = +2 successes; messy = critical (≥1 pair) + ≥1 hunger 10; tally records `has_hunger_one` for use by `compare`.

- [ ] **Step 1: Write the failing tests**

Replace `src-tauri/src/shared/v5/interpret.rs` with:

```rust
//! Convert RollResult into a Tally (successes, crits, hunger flags).

use crate::shared::v5::types::{Die, DieKind, PoolPart, RollResult, Tally};

pub fn interpret(result: &RollResult) -> Tally {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn die(kind: DieKind, value: u8) -> Die { Die { kind, value } }

    fn result_from(values: &[(DieKind, u8)]) -> RollResult {
        RollResult {
            parts: vec![PoolPart { name: "X".into(), level: values.len() as u8 }],
            dice: values.iter().map(|(k, v)| die(*k, *v)).collect(),
        }
    }

    #[test]
    fn no_dice_no_successes() {
        let r = result_from(&[]);
        let t = interpret(&r);
        assert_eq!(t.successes, 0);
        assert_eq!(t.crit_pairs, 0);
        assert!(!t.is_critical);
    }

    #[test]
    fn dice_six_through_nine_each_count_one_success() {
        let r = result_from(&[
            (DieKind::Regular, 6), (DieKind::Regular, 7),
            (DieKind::Regular, 8), (DieKind::Regular, 9),
            (DieKind::Regular, 5), (DieKind::Regular, 1),
        ]);
        let t = interpret(&r);
        assert_eq!(t.successes, 4);
        assert_eq!(t.crit_pairs, 0);
        assert!(!t.is_critical);
    }

    #[test]
    fn two_tens_yield_four_successes_one_crit_pair() {
        let r = result_from(&[(DieKind::Regular, 10), (DieKind::Regular, 10)]);
        let t = interpret(&r);
        assert_eq!(t.successes, 4);  // 2 from tens + 2 bonus
        assert_eq!(t.crit_pairs, 1);
        assert!(t.is_critical);
        assert!(!t.is_messy_critical);
    }

    #[test]
    fn four_tens_yield_eight_successes_two_crit_pairs() {
        let r = result_from(&[
            (DieKind::Regular, 10), (DieKind::Regular, 10),
            (DieKind::Regular, 10), (DieKind::Regular, 10),
        ]);
        let t = interpret(&r);
        assert_eq!(t.successes, 8);  // 4 + 2*2 bonus
        assert_eq!(t.crit_pairs, 2);
    }

    #[test]
    fn three_tens_yield_five_successes_one_pair_plus_lone() {
        let r = result_from(&[
            (DieKind::Regular, 10), (DieKind::Regular, 10),
            (DieKind::Regular, 10),
        ]);
        let t = interpret(&r);
        assert_eq!(t.successes, 5);  // 3 from tens + 2 bonus from one pair
        assert_eq!(t.crit_pairs, 1);
    }

    #[test]
    fn hunger_ten_in_pair_marks_messy() {
        let r = result_from(&[(DieKind::Hunger, 10), (DieKind::Regular, 10)]);
        let t = interpret(&r);
        assert!(t.is_critical);
        assert!(t.is_messy_critical);
    }

    #[test]
    fn hunger_ten_alone_is_not_messy_without_pair() {
        let r = result_from(&[(DieKind::Hunger, 10), (DieKind::Regular, 7)]);
        let t = interpret(&r);
        assert!(!t.is_critical);
        assert!(!t.is_messy_critical);
        assert_eq!(t.successes, 2);  // one ten + one ≥6 = 2
    }

    #[test]
    fn hunger_one_recorded_in_tally() {
        let r = result_from(&[(DieKind::Hunger, 1), (DieKind::Regular, 7)]);
        let t = interpret(&r);
        assert!(t.has_hunger_one);
    }

    #[test]
    fn regular_one_does_not_set_hunger_one() {
        let r = result_from(&[(DieKind::Regular, 1), (DieKind::Hunger, 5)]);
        let t = interpret(&r);
        assert!(!t.has_hunger_one);
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::interpret`
Expected: 9 fails.

- [ ] **Step 2: Implement `interpret`**

Replace the `todo!()` body:

```rust
pub fn interpret(result: &RollResult) -> Tally {
    let mut tens: u8 = 0;
    let mut hunger_tens: u8 = 0;
    let mut hunger_ones: u8 = 0;
    let mut nontens_six_plus: u8 = 0;

    for d in &result.dice {
        if d.value == 10 {
            tens += 1;
            if d.kind == DieKind::Hunger { hunger_tens += 1; }
        } else if d.value >= 6 {
            nontens_six_plus += 1;
        }
        if d.kind == DieKind::Hunger && d.value == 1 {
            hunger_ones += 1;
        }
    }

    let crit_pairs = tens / 2;
    let successes = nontens_six_plus + tens + crit_pairs * 2;
    let is_critical = crit_pairs >= 1;
    let is_messy_critical = is_critical && hunger_tens >= 1;

    Tally {
        successes,
        crit_pairs,
        is_critical,
        is_messy_critical,
        has_hunger_one: hunger_ones > 0,
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::interpret`
Expected: 9 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/v5/interpret.rs
git commit -m "feat(shared/v5): implement interpret with V5 success/crit/messy logic (Plan 3 task 5)"
```

---

## Task 6: Implement `compare` + tests

**Files:**
- Modify: `src-tauri/src/shared/v5/difficulty.rs`

**Anti-scope:** Do NOT touch other leaves.

**Depends on:** Task 2.

**Invariants cited:** ARCHITECTURE.md §10.

- [ ] **Step 1: Write the failing tests**

Replace `src-tauri/src/shared/v5/difficulty.rs` with:

```rust
//! Compare a Tally against a difficulty to produce an Outcome.

use crate::shared::v5::types::{Outcome, OutcomeFlags, Tally};

pub fn compare(tally: &Tally, difficulty: u8) -> Outcome {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tally(successes: u8, hunger_one: bool, critical: bool, messy: bool) -> Tally {
        Tally {
            successes,
            crit_pairs: if critical { 1 } else { 0 },
            is_critical: critical,
            is_messy_critical: messy,
            has_hunger_one: hunger_one,
        }
    }

    #[test]
    fn passed_with_positive_margin() {
        let o = compare(&tally(5, false, false, false), 2);
        assert!(o.passed);
        assert_eq!(o.margin, 3);
        assert!(!o.flags.bestial_failure);
        assert!(!o.flags.total_failure);
    }

    #[test]
    fn exactly_meeting_difficulty_passes() {
        let o = compare(&tally(2, false, false, false), 2);
        assert!(o.passed);
        assert_eq!(o.margin, 0);
    }

    #[test]
    fn below_difficulty_fails() {
        let o = compare(&tally(1, false, false, false), 3);
        assert!(!o.passed);
        assert_eq!(o.margin, -2);
    }

    #[test]
    fn zero_successes_is_total_failure() {
        let o = compare(&tally(0, false, false, false), 1);
        assert!(o.flags.total_failure);
    }

    #[test]
    fn failure_with_hunger_one_is_bestial() {
        let o = compare(&tally(1, true, false, false), 3);
        assert!(o.flags.bestial_failure);
    }

    #[test]
    fn pass_with_hunger_one_is_not_bestial() {
        let o = compare(&tally(3, true, false, false), 1);
        assert!(o.passed);
        assert!(!o.flags.bestial_failure);
    }

    #[test]
    fn total_failure_with_hunger_one_sets_both_flags() {
        let o = compare(&tally(0, true, false, false), 1);
        assert!(o.flags.total_failure);
        assert!(o.flags.bestial_failure);
    }

    #[test]
    fn critical_flag_propagates_to_outcome_flags() {
        let o = compare(&tally(4, false, true, false), 1);
        assert!(o.passed);
        assert!(o.flags.critical);
    }

    #[test]
    fn messy_flag_propagates() {
        let o = compare(&tally(4, false, true, true), 1);
        assert!(o.flags.messy);
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::difficulty`
Expected: 9 fails.

- [ ] **Step 2: Implement `compare`**

```rust
pub fn compare(tally: &Tally, difficulty: u8) -> Outcome {
    let margin: i32 = (tally.successes as i32) - (difficulty as i32);
    let passed = margin >= 0;
    let total_failure = tally.successes == 0;
    let bestial_failure = !passed && tally.has_hunger_one;

    Outcome {
        successes: tally.successes,
        difficulty,
        margin,
        passed,
        flags: OutcomeFlags {
            critical: tally.is_critical,
            messy: tally.is_messy_critical,
            bestial_failure,
            total_failure,
        },
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::difficulty`
Expected: 9 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/v5/difficulty.rs
git commit -m "feat(shared/v5): implement compare with bestial/total flag logic (Plan 3 task 6)"
```

---

## Task 7: Implement `format_skill_check` + tests

**Files:**
- Modify: `src-tauri/src/shared/v5/message.rs`

**Anti-scope:** Do NOT touch other leaves.

**Depends on:** Task 2.

**Invariants cited:** ARCHITECTURE.md §10. Message format is presentational — golden tests are acceptable.

- [ ] **Step 1: Write the failing tests**

Replace `src-tauri/src/shared/v5/message.rs` with:

```rust
//! Format the human-facing summary string for a skill check.

use crate::shared::v5::types::{Outcome, PoolPart, RollResult, SkillCheckInput};

pub fn format_skill_check(
    input: &SkillCheckInput,
    _result: &RollResult,
    outcome: &Outcome,
) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::v5::types::{Die, DieKind, OutcomeFlags};

    fn input_with(name: Option<&str>) -> SkillCheckInput {
        SkillCheckInput {
            character_name: name.map(String::from),
            attribute: PoolPart { name: "Strength".into(), level: 4 },
            skill: PoolPart { name: "Brawl".into(), level: 3 },
            hunger: 2,
            specialty: None,
            difficulty: 2,
        }
    }

    fn empty_result() -> RollResult {
        RollResult { parts: vec![], dice: vec![] }
    }

    fn outcome(successes: u8, difficulty: u8, passed: bool, flags: OutcomeFlags) -> Outcome {
        Outcome {
            successes,
            difficulty,
            margin: (successes as i32) - (difficulty as i32),
            passed,
            flags,
        }
    }

    fn flags(critical: bool, messy: bool, bestial: bool, total: bool) -> OutcomeFlags {
        OutcomeFlags { critical, messy, bestial_failure: bestial, total_failure: total }
    }

    #[test]
    fn includes_character_name_when_set() {
        let i = input_with(Some("Charlotte"));
        let o = outcome(5, 2, true, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("Charlotte"), "expected Charlotte in: {s}");
    }

    #[test]
    fn omits_owner_when_name_is_none() {
        let i = input_with(None);
        let o = outcome(5, 2, true, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(!s.starts_with("'s"));
        assert!(s.starts_with("Strength + Brawl"));
    }

    #[test]
    fn marks_messy_critical() {
        let i = input_with(Some("Charlotte"));
        let o = outcome(5, 2, true, flags(true, true, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("Messy Critical"));
    }

    #[test]
    fn marks_bestial_failure() {
        let i = input_with(Some("Tessa"));
        let o = outcome(1, 3, false, flags(false, false, true, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("Bestial Failure"));
    }

    #[test]
    fn shows_margin_and_dv() {
        let i = input_with(None);
        let o = outcome(5, 2, true, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("+3"), "expected +3 in: {s}");
        assert!(s.contains("DV 2"), "expected DV 2 in: {s}");
    }

    #[test]
    fn shows_negative_margin_for_failure() {
        let i = input_with(None);
        let o = outcome(1, 3, false, flags(false, false, false, false));
        let s = format_skill_check(&i, &empty_result(), &o);
        assert!(s.contains("-2"), "expected -2 in: {s}");
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::message`
Expected: 6 fails.

- [ ] **Step 2: Implement `format_skill_check`**

```rust
pub fn format_skill_check(
    input: &SkillCheckInput,
    _result: &RollResult,
    outcome: &Outcome,
) -> String {
    // Owner prefix: "Charlotte's Strength + Brawl check" or just "Strength + Brawl check".
    let head = match input.character_name.as_deref() {
        Some(name) if !name.is_empty() => format!(
            "{}'s {} + {} check",
            name, input.attribute.name, input.skill.name
        ),
        _ => format!("{} + {} check", input.attribute.name, input.skill.name),
    };

    // Embellishment: pick the most specific descriptor; flags ordered by priority.
    let descriptor = if outcome.flags.messy {
        "Messy Critical"
    } else if outcome.flags.critical && outcome.passed {
        "Critical"
    } else if outcome.flags.bestial_failure && outcome.flags.total_failure {
        "Bestial Total Failure"
    } else if outcome.flags.bestial_failure {
        "Bestial Failure"
    } else if outcome.flags.total_failure {
        "Total Failure"
    } else if outcome.passed {
        "Success"
    } else {
        "Failure"
    };

    // Margin sign: "+3" / "-2".
    let margin_str = if outcome.margin >= 0 {
        format!("+{}", outcome.margin)
    } else {
        format!("{}", outcome.margin)
    };

    format!(
        "{} · {} · {} ({} vs DV {})",
        head, descriptor, margin_str, outcome.successes, outcome.difficulty
    )
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::message`
Expected: 6 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/shared/v5/message.rs
git commit -m "feat(shared/v5): implement format_skill_check (Plan 3 task 7)"
```

---

## Task 8: Implement `skill_check` orchestrator + integration test

**Files:**
- Modify: `src-tauri/src/shared/v5/skill_check.rs`

**Anti-scope:** Orchestrator is shallow plumbing. No new logic.

**Depends on:** Tasks 3, 4, 5, 6, 7.

**Invariants cited:** ARCHITECTURE.md §10.

- [ ] **Step 1: Write the failing test**

Replace `src-tauri/src/shared/v5/skill_check.rs` with:

```rust
//! Orchestrator: assembles → rolls → interprets → compares → formats.

use crate::shared::v5::pool::build_pool;
use crate::shared::v5::dice::roll_pool;
use crate::shared::v5::interpret::interpret;
use crate::shared::v5::difficulty::compare;
use crate::shared::v5::message::format_skill_check;
use crate::shared::v5::types::{SkillCheckInput, SkillCheckResult};
use rand::Rng;

pub fn skill_check<R: Rng + ?Sized>(
    input: &SkillCheckInput,
    rng: &mut R,
) -> SkillCheckResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared::v5::types::PoolPart;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn sample_input() -> SkillCheckInput {
        SkillCheckInput {
            character_name: Some("Charlotte".into()),
            attribute: PoolPart { name: "Strength".into(), level: 4 },
            skill: PoolPart { name: "Brawl".into(), level: 3 },
            hunger: 2,
            specialty: None,
            difficulty: 2,
        }
    }

    #[test]
    fn deterministic_with_seed() {
        let mut rng_a = StdRng::seed_from_u64(42);
        let mut rng_b = StdRng::seed_from_u64(42);
        let a = skill_check(&sample_input(), &mut rng_a);
        let b = skill_check(&sample_input(), &mut rng_b);
        assert_eq!(a.outcome.successes, b.outcome.successes);
        assert_eq!(a.outcome.margin, b.outcome.margin);
        assert_eq!(a.message, b.message);
    }

    #[test]
    fn pipeline_produces_message_referencing_character() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = skill_check(&sample_input(), &mut rng);
        assert!(r.message.contains("Charlotte"));
        assert!(r.message.contains("Strength"));
        assert!(r.message.contains("Brawl"));
    }

    #[test]
    fn pool_size_matches_attribute_plus_skill_plus_hunger_clamp() {
        let mut rng = StdRng::seed_from_u64(7);
        let r = skill_check(&sample_input(), &mut rng);
        // attribute 4 + skill 3 = pool 7; hunger 2 → 5 regular + 2 hunger.
        assert_eq!(r.spec.regular_count, 5);
        assert_eq!(r.spec.hunger_count, 2);
        assert_eq!(r.roll.dice.len(), 7);
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::skill_check`
Expected: 3 fails.

- [ ] **Step 2: Implement the orchestrator**

```rust
pub fn skill_check<R: Rng + ?Sized>(
    input: &SkillCheckInput,
    rng: &mut R,
) -> SkillCheckResult {
    let spec    = build_pool(input);
    let roll    = roll_pool(&spec, rng);
    let tally   = interpret(&roll);
    let outcome = compare(&tally, input.difficulty);
    let message = format_skill_check(input, &roll, &outcome);
    SkillCheckResult { spec, roll, tally, outcome, message }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5::skill_check`
Expected: 3 passed.

- [ ] **Step 4: Run all v5 tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared::v5`
Expected: 32 passed (5 pool + 5 dice + 9 interpret + 9 difficulty + 6 message + 3 skill_check, less any I miscounted).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/shared/v5/skill_check.rs
git commit -m "feat(shared/v5): implement skill_check orchestrator (Plan 3 task 8)"
```

---

## Task 9: Add `roll_skill_check` Tauri command

**Files:**
- Create: `src-tauri/src/tools/skill_check.rs`
- Modify: `src-tauri/src/tools/mod.rs`

**Anti-scope:** Do NOT register in `lib.rs` yet (Task 10). Do NOT add UI consumer.

**Depends on:** Task 8.

**Invariants cited:** ARCHITECTURE.md §4 (Tauri IPC commands).

- [ ] **Step 1: Create `tools/skill_check.rs`**

```rust
//! Tauri command wrapping shared::v5::skill_check::skill_check with thread_rng().
//! Synchronous — no I/O.

use crate::shared::v5::skill_check::skill_check;
use crate::shared::v5::types::{SkillCheckInput, SkillCheckResult};

#[tauri::command]
pub fn roll_skill_check(input: SkillCheckInput) -> SkillCheckResult {
    let mut rng = rand::thread_rng();
    skill_check(&input, &mut rng)
}
```

- [ ] **Step 2: Add `pub mod skill_check;` to `tools/mod.rs`**

Edit `src-tauri/src/tools/mod.rs` to add the line:

```rust
pub mod skill_check;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/tools/skill_check.rs src-tauri/src/tools/mod.rs
git commit -m "feat(tools): add roll_skill_check Tauri command (Plan 3 task 9)"
```

---

## Task 10: Register command in `lib.rs`

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Anti-scope:** Do NOT touch other entries.

**Depends on:** Task 9.

**Invariants cited:** ARCHITECTURE.md §4.

- [ ] **Step 1: Add the entry**

In `src-tauri/src/lib.rs`'s `invoke_handler(tauri::generate_handler![…])` list, near `tools::resonance::roll_resonance`:

```rust
            tools::resonance::roll_resonance,
            tools::skill_check::roll_skill_check,
            tools::export::export_result_to_md,
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --manifest-path src-tauri/Cargo.toml`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register roll_skill_check command (Plan 3 task 10)"
```

---

## Task 11: Add typed wrapper `src/lib/v5/api.ts`

**Files:**
- Create: `src/lib/v5/api.ts`

**Anti-scope:** No UI consumer in Phase 1.

**Depends on:** Task 10.

**Invariants cited:** ARCHITECTURE.md §4 (typed wrappers), §5 (no `invoke()` from components).

- [ ] **Step 1: Create the wrapper**

```ts
import { invoke } from '@tauri-apps/api/core';

export interface PoolPart {
  name: string;
  level: number;
}

export type DieKind = 'regular' | 'hunger';

export interface Die {
  kind: DieKind;
  value: number;
}

export interface PoolSpec {
  parts: PoolPart[];
  regularCount: number;
  hungerCount: number;
}

export interface RollResult {
  parts: PoolPart[];
  dice: Die[];
}

export interface Tally {
  successes: number;
  critPairs: number;
  isCritical: boolean;
  isMessyCritical: boolean;
  hasHungerOne: boolean;
}

export interface OutcomeFlags {
  critical: boolean;
  messy: boolean;
  bestialFailure: boolean;
  totalFailure: boolean;
}

export interface Outcome {
  successes: number;
  difficulty: number;
  margin: number;
  passed: boolean;
  flags: OutcomeFlags;
}

export interface SkillCheckInput {
  characterName: string | null;
  attribute: PoolPart;
  skill: PoolPart;
  hunger: number;        // 0..=5
  specialty: string | null;
  difficulty: number;
}

export interface SkillCheckResult {
  spec: PoolSpec;
  roll: RollResult;
  tally: Tally;
  outcome: Outcome;
  message: string;
}

export async function rollSkillCheck(input: SkillCheckInput): Promise<SkillCheckResult> {
  return await invoke<SkillCheckResult>('roll_skill_check', { input });
}
```

- [ ] **Step 2: Verify type-check**

Run: `npm run check`
Expected: clean.

- [ ] **Step 3: Commit**

```bash
git add src/lib/v5/api.ts
git commit -m "feat(v5): add typed rollSkillCheck wrapper (Plan 3 task 11)"
```

---

## Task 12: Final verification gate

**Files:** none — verification only.

**Depends on:** all previous.

- [ ] **Step 1: Run `./scripts/verify.sh`**

```bash
./scripts/verify.sh
```

Expected: green. `cargo test` should report ~32 new passing tests under `shared::v5::*`.

- [ ] **Step 2: Manual one-shot dev-tools smoke**

(Optional in Phase 1 since no UI consumer exists; useful confidence check.)

In `npm run tauri dev`, open the desktop dev tools console and run:

```js
await window.__TAURI_INTERNALS__.invoke('roll_skill_check', {
  input: {
    characterName: 'Charlotte',
    attribute: { name: 'Strength', level: 4 },
    skill: { name: 'Brawl', level: 3 },
    hunger: 2,
    specialty: 'bare-knuckle',
    difficulty: 2,
  },
});
```

Expected: a `SkillCheckResult` object with `spec`, `roll`, `tally`, `outcome`, `message`. The `message` should look like `"Charlotte's Strength + Brawl check · … · ±N (X vs DV 2)"`.

- [ ] **Step 3: Commit any fixups**

```bash
git status --short
```

If clean, no commit. Otherwise:

```bash
git add -A
git commit -m "chore: Plan 3 verification fixups"
```

---

## Self-review checklist

- [x] Spec § 3.4 module structure (`shared/v5/{mod, types, pool, dice, interpret, difficulty, message, skill_check}.rs`) — Tasks 1, 2.
- [x] Spec § 3.4 types (PoolPart, Die, DieKind, PoolSpec, RollResult, Tally, Outcome, OutcomeFlags, SkillCheckInput, SkillCheckResult) — Task 2.
- [x] Spec § 3.4 `build_pool` (parts assembly, hunger clamp, specialty as fake part) — Task 3.
- [x] Spec § 3.4 `roll_pool` (RNG injected, regulars-then-hunger order) — Task 4.
- [x] Spec § 3.4 `interpret` (V5 success/crit/messy mechanics; 2×10 → 4 successes; 4×10 → 8; 3×10 → 5) — Task 5.
- [x] Spec § 3.4 `compare` (margin, passed, total_failure, bestial_failure) — Task 6.
- [x] Spec § 3.4 `format_skill_check` (descriptor priority, margin sign, DV display) — Task 7.
- [x] Spec § 3.4 `skill_check` orchestrator (5 calls, no branches) — Task 8.
- [x] Spec § 3.3 `roll_skill_check` Tauri command (sync, thread_rng) — Tasks 9, 10.
- [x] Spec § 3.5 typed wrapper `src/lib/v5/api.ts` — Task 11.
- [x] Spec § 3.6 testing strategy (per-primitive `#[cfg(test)] mod tests`) — Tasks 3–8.
- [x] No placeholders / TBDs.
- [x] All tasks include explicit anti-scope.
- [x] Verification gate runs `./scripts/verify.sh` — Task 12.
