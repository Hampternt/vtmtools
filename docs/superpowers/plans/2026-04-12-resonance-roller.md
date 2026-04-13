# Resonance Roller Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the vtmtools Tauri 2 desktop app foundation and the Resonance dice roller — a GM tool that automates VTM 5e feeding rolls with adjustable probability weights.

**Architecture:** Rust backend handles all dice logic and database access as Tauri commands; SvelteKit frontend renders the step-wizard UI and live summary panel. A shared Rust module layer makes the resonance logic importable by any future tool. SQLite (via sqlx) stores Dyscrasia tables. A tool registry pattern in Svelte makes adding future tools a one-file change.

**Tech Stack:** Rust, Tauri 2, SvelteKit, Svelte, TypeScript, sqlx 0.7 (SQLite), rand 0.8, serde/serde_json, tauri-plugin-updater 2, GitHub Actions

---

## File Map

```
vtmtools/
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   ├── migrations/
│   │   └── 0001_initial.sql            # dyscrasias table schema
│   └── src/
│       ├── main.rs                     # calls lib::run()
│       ├── lib.rs                      # Tauri builder, plugin registration, command registration
│       ├── shared/
│       │   ├── mod.rs
│       │   ├── types.rs                # Temperament, ResonanceType, SliderLevel, all DTOs
│       │   ├── dice.rs                 # roll_d10, advantage_roll, weighted_resonance_pick
│       │   └── resonance.rs            # roll_temperament, roll_resonance_type, check_acute
│       ├── tools/
│       │   ├── mod.rs
│       │   └── resonance.rs            # Tauri #[command] wrappers calling shared::
│       └── db/
│           ├── mod.rs
│           ├── seed.rs                 # inserts canonical Dyscrasia entries if table empty
│           └── dyscrasia.rs            # CRUD: list, add, update, delete dyscrasia entries
├── src/
│   ├── app.html
│   ├── routes/
│   │   ├── +layout.svelte             # sidebar + main pane shell
│   │   └── +page.svelte               # redirects to /resonance
│   ├── tools.ts                        # tool registry: {id, label, icon, component}[]
│   ├── store/
│   │   └── toolEvents.ts              # lightweight pub/sub Svelte store
│   ├── tools/
│   │   └── Resonance.svelte           # full Resonance Roller page
│   └── lib/
│       └── components/
│           ├── Sidebar.svelte          # renders tool registry list
│           ├── ResonanceSlider.svelte  # 7-step Impossible→Guaranteed slider
│           ├── TemperamentConfig.svelte # dice count + threshold band config
│           └── ResultCard.svelte       # step-by-step result reveal + Export button
├── docs/
│   └── superpowers/
│       ├── specs/
│       │   └── 2026-04-12-resonance-roller-design.md
│       └── plans/
│           └── 2026-04-12-resonance-roller.md  (this file)
└── .github/
    └── workflows/
        └── release.yml                # build + publish Linux/Windows Tauri bundles on tag
```

---

## Task 1: Write spec doc and scaffold Tauri 2 + SvelteKit project

**Files:**
- Create: `docs/superpowers/specs/2026-04-12-resonance-roller-design.md`
- Create: entire project scaffold via `cargo create-tauri-app`

- [ ] **Step 1: Install prerequisites**

```bash
# Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Node.js 18+ (if not installed — use nvm or system package)
# Tauri system dependencies on Linux:
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# Tauri CLI
cargo install tauri-cli --version "^2.0" --locked
```

- [ ] **Step 2: Scaffold the project**

Run from `/home/hampter/projects/vtmtools/`:

```bash
cargo install create-tauri-app --locked
cargo create-tauri-app . \
  --template svelte-ts \
  --identifier com.vtmtools.app \
  --app-name vtmtools
```

If prompted interactively: select **Svelte**, **TypeScript**, keep defaults.

Expected output: project files created, no errors.

- [ ] **Step 3: Verify the scaffold builds**

```bash
cd /home/hampter/projects/vtmtools
npm install
cargo tauri dev
```

Expected: a native window opens with the default Svelte welcome screen. Close it with Ctrl+C.

- [ ] **Step 4: Write spec doc**

Create `docs/superpowers/specs/2026-04-12-resonance-roller-design.md` with the following content:

```markdown
# vtmtools — Resonance Roller Design Spec
Date: 2026-04-12

## Stack
Tauri 2 + SvelteKit + Rust + SQLite (sqlx) + tauri-plugin-updater

## Architecture
- Shared Rust modules in src-tauri/src/shared/ (dice.rs, resonance.rs, types.rs)
- Tool-specific Tauri commands in src-tauri/src/tools/
- DB access in src-tauri/src/db/ (dyscrasia.rs, seed.rs)
- Tool registry in src/tools.ts — add a new entry to add a new tool
- Inter-tool pub/sub via src/store/toolEvents.ts

## Dice Mechanics
1. Roll d10 for temperament: 1-5 Negligible, 6-8 Fleeting, 9-10 Intense
2. If Fleeting or Intense: roll resonance type (weighted by GM sliders)
   - Default: Phlegmatic 1-3, Melancholy 4-6, Choleric 7-8, Sanguine 9-10
3. If Intense: roll acute check — 9-10 = Acute
4. If Acute: roll or GM-pick from Dyscrasia table for that resonance type

## Temperament Modifiers
- Advantage/disadvantage: roll N dice (1-5), take highest or lowest
- Threshold shift: GM adjusts Negligible/Fleeting/Intense band boundaries directly

## Resonance Type Weighting
7-step slider per type: Impossible/ExtremelyUnlikely/Unlikely/Neutral/Likely/ExtremelyLikely/Guaranteed
Multipliers: 0 / 0.1× / 0.5× / 1× / 2× / 4× / locks to 100%
Applied against base probabilities (30/30/20/20), then normalised.

## Dyscrasia Tables
One table per resonance type in SQLite. Canonical entries seeded on first run.
Custom entries: add/edit/delete supported. Roll random or GM-pick both exposed.

## UI Layout
Step wizard (left panel) + live summary (right panel) + result card (replaces wizard after roll).
Gothic VTM aesthetic: dark background, blood-red accents, parchment result cards.

## Export
format_to_md(json) — pure Rust function, no DB access. Saves .md to ~/Documents/vtmtools/.

## Auto-update
tauri-plugin-updater pointed at GitHub releases. Checks on app launch.
CI: .github/workflows/release.yml builds Linux + Windows on tag push.
```

- [ ] **Step 5: Initial commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri 2 + SvelteKit project with spec doc"
```

---

## Task 2: Set up Rust module structure and shared types

**Files:**
- Create: `src-tauri/src/shared/mod.rs`
- Create: `src-tauri/src/shared/types.rs`
- Create: `src-tauri/src/tools/mod.rs`
- Create: `src-tauri/src/db/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add dependencies to Cargo.toml**

Open `src-tauri/Cargo.toml`. Add to `[dependencies]`:

```toml
rand = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "migrate"] }
tokio = { version = "1", features = ["full"] }
tauri-plugin-updater = "2"
```

- [ ] **Step 2: Create module declaration files**

Create `src-tauri/src/shared/mod.rs`:
```rust
pub mod types;
pub mod dice;
pub mod resonance;
```

Create `src-tauri/src/tools/mod.rs`:
```rust
pub mod resonance;
```

Create `src-tauri/src/db/mod.rs`:
```rust
pub mod dyscrasia;
pub mod seed;
```

- [ ] **Step 3: Declare modules in lib.rs**

Open `src-tauri/src/lib.rs`. Add at the top (before the existing `run` function or replace the file wholesale):

```rust
mod shared;
mod tools;
mod db;

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
            // tools::export::export_result_to_md is added in Task 11
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Write shared types**

Create `src-tauri/src/shared/types.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Temperament {
    Negligible,
    Fleeting,
    Intense,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResonanceType {
    Phlegmatic,
    Melancholy,
    Choleric,
    Sanguine,
}

/// 7-step slider level for resonance type weighting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum SliderLevel {
    Impossible,
    ExtremelyUnlikely,
    Unlikely,
    Neutral,
    Likely,
    ExtremelyLikely,
    Guaranteed,
}

impl SliderLevel {
    /// Maps slider level to a weight multiplier applied against the base probability
    pub fn multiplier(&self) -> f64 {
        match self {
            SliderLevel::Impossible => 0.0,
            SliderLevel::ExtremelyUnlikely => 0.1,
            SliderLevel::Unlikely => 0.5,
            SliderLevel::Neutral => 1.0,
            SliderLevel::Likely => 2.0,
            SliderLevel::ExtremelyLikely => 4.0,
            SliderLevel::Guaranteed => f64::INFINITY,
        }
    }
}

/// GM-configurable temperament roll options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemperamentConfig {
    /// How many d10s to roll (1–5). Result is best or worst of the pool.
    pub dice_count: u8,
    /// true = take highest die (biases toward Intense), false = take lowest
    pub take_highest: bool,
    /// Upper bound (inclusive) for Negligible result. Default 5.
    pub negligible_max: u8,
    /// Upper bound (inclusive) for Fleeting result. Default 8. Intense = above this.
    pub fleeting_max: u8,
}

impl Default for TemperamentConfig {
    fn default() -> Self {
        Self {
            dice_count: 1,
            take_highest: true,
            negligible_max: 5,
            fleeting_max: 8,
        }
    }
}

/// GM-configurable weighting for resonance type selection
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResonanceWeights {
    pub phlegmatic: SliderLevel,
    pub melancholy: SliderLevel,
    pub choleric: SliderLevel,
    pub sanguine: SliderLevel,
}

impl Default for ResonanceWeights {
    fn default() -> Self {
        Self {
            phlegmatic: SliderLevel::Neutral,
            melancholy: SliderLevel::Neutral,
            choleric: SliderLevel::Neutral,
            sanguine: SliderLevel::Neutral,
        }
    }
}

/// Full GM config passed to a roll
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RollConfig {
    pub temperament: TemperamentConfig,
    pub weights: ResonanceWeights,
}

impl Default for RollConfig {
    fn default() -> Self {
        Self {
            temperament: TemperamentConfig::default(),
            weights: ResonanceWeights::default(),
        }
    }
}

/// A Dyscrasia entry from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DyscrasiaEntry {
    pub id: i64,
    pub resonance_type: ResonanceType,
    pub name: String,
    pub description: String,
    pub bonus: String,
    pub is_custom: bool,
}

/// Full result of one resonance roll sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResonanceRollResult {
    /// All dice rolled for temperament (for display)
    pub temperament_dice: Vec<u8>,
    /// The die value that determined temperament
    pub temperament_die: u8,
    pub temperament: Temperament,
    /// Set if temperament is Fleeting or Intense
    pub resonance_type: Option<ResonanceType>,
    /// d10 rolled for resonance (display only — weighted pick determines actual result)
    pub resonance_die: Option<u8>,
    /// Set if temperament is Intense
    pub acute_die: Option<u8>,
    pub is_acute: bool,
    /// Populated after GM rolls/picks Dyscrasia (not auto-populated here)
    pub dyscrasia: Option<DyscrasiaEntry>,
}
```

- [ ] **Step 5: Verify it compiles**

```bash
cd /home/hampter/projects/vtmtools
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | head -40
```

Expected: may have warnings but no errors. Fix any import issues.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat: add rust module structure and shared types"
```

---

## Task 3: Implement dice primitives (TDD)

**Files:**
- Create: `src-tauri/src/shared/dice.rs`

- [ ] **Step 1: Write the test module first**

Create `src-tauri/src/shared/dice.rs` with tests only:

```rust
use crate::shared::types::{ResonanceType, ResonanceWeights, SliderLevel};

/// Rolls a single d10. Returns 1–10 (0 on the die = 10).
pub fn roll_d10() -> u8 {
    todo!()
}

/// Rolls `count` d10s. Returns (selected_value, all_rolls).
/// If take_highest=true, returns the max; otherwise returns the min.
pub fn advantage_roll(count: u8, take_highest: bool) -> (u8, Vec<u8>) {
    todo!()
}

/// Selects a ResonanceType using weighted probability.
/// Base probabilities: Phlegmatic 30%, Melancholy 30%, Choleric 20%, Sanguine 20%.
/// Each type's base probability is multiplied by its slider level multiplier,
/// then results are normalised. "Guaranteed" bypasses normalisation entirely.
pub fn weighted_resonance_pick(weights: &ResonanceWeights) -> ResonanceType {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roll_d10_stays_in_range() {
        for _ in 0..1000 {
            let r = roll_d10();
            assert!(r >= 1 && r <= 10, "roll_d10 returned {r}");
        }
    }

    #[test]
    fn advantage_roll_single_die_matches_roll_d10_range() {
        let (val, rolls) = advantage_roll(1, true);
        assert_eq!(rolls.len(), 1);
        assert!(val >= 1 && val <= 10);
        assert_eq!(val, rolls[0]);
    }

    #[test]
    fn advantage_roll_take_highest_returns_max() {
        // With a large pool we can't guarantee the max is selected deterministically,
        // so we verify the contract: result == max of rolls.
        for _ in 0..100 {
            let (val, rolls) = advantage_roll(3, true);
            assert_eq!(val, *rolls.iter().max().unwrap());
        }
    }

    #[test]
    fn advantage_roll_take_lowest_returns_min() {
        for _ in 0..100 {
            let (val, rolls) = advantage_roll(3, false);
            assert_eq!(val, *rolls.iter().min().unwrap());
        }
    }

    #[test]
    fn weighted_resonance_guaranteed_always_returns_that_type() {
        let weights = ResonanceWeights {
            phlegmatic: SliderLevel::Guaranteed,
            melancholy: SliderLevel::Neutral,
            choleric: SliderLevel::Neutral,
            sanguine: SliderLevel::Neutral,
        };
        for _ in 0..50 {
            assert_eq!(weighted_resonance_pick(&weights), ResonanceType::Phlegmatic);
        }
    }

    #[test]
    fn weighted_resonance_impossible_never_returns_that_type() {
        let weights = ResonanceWeights {
            phlegmatic: SliderLevel::Impossible,
            melancholy: SliderLevel::Neutral,
            choleric: SliderLevel::Neutral,
            sanguine: SliderLevel::Neutral,
        };
        for _ in 0..200 {
            assert_ne!(weighted_resonance_pick(&weights), ResonanceType::Phlegmatic);
        }
    }

    #[test]
    fn weighted_resonance_all_neutral_returns_all_types_over_many_rolls() {
        let weights = ResonanceWeights::default();
        let mut counts = std::collections::HashMap::new();
        for _ in 0..2000 {
            let t = weighted_resonance_pick(&weights);
            *counts.entry(format!("{t:?}")).or_insert(0u32) += 1;
        }
        // All four types must appear at least once
        assert!(counts.contains_key("Phlegmatic"), "Phlegmatic never appeared");
        assert!(counts.contains_key("Melancholy"), "Melancholy never appeared");
        assert!(counts.contains_key("Choleric"), "Choleric never appeared");
        assert!(counts.contains_key("Sanguine"), "Sanguine never appeared");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd /home/hampter/projects/vtmtools
cargo test --manifest-path src-tauri/Cargo.toml shared::dice 2>&1
```

Expected: compile error — `todo!()` panics / unresolved. That's correct.

- [ ] **Step 3: Implement dice primitives**

Replace the `todo!()` bodies in `src-tauri/src/shared/dice.rs`:

```rust
use rand::Rng;
use crate::shared::types::{ResonanceType, ResonanceWeights, SliderLevel};

pub fn roll_d10() -> u8 {
    let n: u8 = rand::thread_rng().gen_range(0..10);
    if n == 0 { 10 } else { n }
}

pub fn advantage_roll(count: u8, take_highest: bool) -> (u8, Vec<u8>) {
    let count = count.max(1).min(5);
    let rolls: Vec<u8> = (0..count).map(|_| roll_d10()).collect();
    let selected = if take_highest {
        *rolls.iter().max().unwrap()
    } else {
        *rolls.iter().min().unwrap()
    };
    (selected, rolls)
}

pub fn weighted_resonance_pick(weights: &ResonanceWeights) -> ResonanceType {
    // Base probabilities (must sum to 1.0)
    let base = [
        (ResonanceType::Phlegmatic, 0.30_f64),
        (ResonanceType::Melancholy, 0.30_f64),
        (ResonanceType::Choleric,   0.20_f64),
        (ResonanceType::Sanguine,   0.20_f64),
    ];

    let multipliers = [
        weights.phlegmatic.multiplier(),
        weights.melancholy.multiplier(),
        weights.choleric.multiplier(),
        weights.sanguine.multiplier(),
    ];

    // Guaranteed: return immediately without normalising
    if let Some(i) = multipliers.iter().position(|m| m.is_infinite()) {
        return base[i].0.clone();
    }

    // Apply multipliers to base probabilities
    let weighted: Vec<f64> = base.iter().zip(&multipliers)
        .map(|((_, b), m)| b * m)
        .collect();

    let total: f64 = weighted.iter().sum();

    // All weights are zero — fall back to uniform
    if total == 0.0 {
        let i = rand::thread_rng().gen_range(0..4);
        return base[i].0.clone();
    }

    let pick: f64 = rand::thread_rng().gen_range(0.0..total);
    let mut cumulative = 0.0;
    for (i, w) in weighted.iter().enumerate() {
        cumulative += w;
        if pick < cumulative {
            return base[i].0.clone();
        }
    }
    base[3].0.clone() // Sanguine fallback (floating-point edge)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::dice 2>&1
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/shared/dice.rs
git commit -m "feat: implement dice primitives with tests"
```

---

## Task 4: Implement resonance roll logic (TDD)

**Files:**
- Create: `src-tauri/src/shared/resonance.rs`

- [ ] **Step 1: Write the test module first**

Create `src-tauri/src/shared/resonance.rs` with stubs + tests:

```rust
use crate::shared::dice::{advantage_roll, roll_d10, weighted_resonance_pick};
use crate::shared::types::*;

/// Rolls temperament according to config. Returns (selected_die, all_dice, Temperament).
pub fn roll_temperament(config: &TemperamentConfig) -> (u8, Vec<u8>, Temperament) {
    todo!()
}

/// Rolls the resonance type (returns a display die + weighted pick result).
pub fn roll_resonance_type(weights: &ResonanceWeights) -> (u8, ResonanceType) {
    todo!()
}

/// Rolls the Acute check (9–10 = Acute). Returns (die, is_acute).
pub fn check_acute() -> (u8, bool) {
    todo!()
}

/// Executes the full roll sequence from a RollConfig.
/// Does NOT populate the dyscrasia field — that requires a DB call done in the command layer.
pub fn execute_roll(config: &RollConfig) -> ResonanceRollResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> RollConfig {
        RollConfig::default()
    }

    #[test]
    fn temperament_negligible_when_die_lte_negligible_max() {
        // Force a low die value by using a config where negligible_max = 10
        let config = TemperamentConfig {
            dice_count: 1,
            take_highest: true,
            negligible_max: 10,
            fleeting_max: 10,
        };
        for _ in 0..50 {
            let (_, _, t) = roll_temperament(&config);
            assert_eq!(t, Temperament::Negligible);
        }
    }

    #[test]
    fn temperament_intense_when_die_above_fleeting_max() {
        // Force a high die value by setting fleeting_max = 0
        let config = TemperamentConfig {
            dice_count: 1,
            take_highest: true,
            negligible_max: 0,
            fleeting_max: 0,
        };
        for _ in 0..50 {
            let (_, _, t) = roll_temperament(&config);
            assert_eq!(t, Temperament::Intense);
        }
    }

    #[test]
    fn check_acute_returns_true_for_9_or_10() {
        // Run many times — verify the bool matches the die correctly
        for _ in 0..500 {
            let (die, is_acute) = check_acute();
            if die >= 9 {
                assert!(is_acute, "die={die} should be acute");
            } else {
                assert!(!is_acute, "die={die} should not be acute");
            }
        }
    }

    #[test]
    fn execute_roll_negligible_has_no_resonance_type() {
        // Guarantee Negligible by maxing negligible_max
        let config = RollConfig {
            temperament: TemperamentConfig {
                dice_count: 1,
                take_highest: true,
                negligible_max: 10,
                fleeting_max: 10,
            },
            weights: ResonanceWeights::default(),
        };
        let result = execute_roll(&config);
        assert_eq!(result.temperament, Temperament::Negligible);
        assert!(result.resonance_type.is_none());
        assert!(result.acute_die.is_none());
        assert!(!result.is_acute);
    }

    #[test]
    fn execute_roll_intense_always_has_resonance_type_and_acute_die() {
        let config = RollConfig {
            temperament: TemperamentConfig {
                dice_count: 1,
                take_highest: true,
                negligible_max: 0,
                fleeting_max: 0,
            },
            weights: ResonanceWeights::default(),
        };
        let result = execute_roll(&config);
        assert_eq!(result.temperament, Temperament::Intense);
        assert!(result.resonance_type.is_some());
        assert!(result.acute_die.is_some());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::resonance 2>&1
```

Expected: panics from `todo!()`. Correct.

- [ ] **Step 3: Implement resonance logic**

Replace the `todo!()` bodies:

```rust
pub fn roll_temperament(config: &TemperamentConfig) -> (u8, Vec<u8>, Temperament) {
    let (die, all_rolls) = advantage_roll(config.dice_count, config.take_highest);
    let temperament = if die <= config.negligible_max {
        Temperament::Negligible
    } else if die <= config.fleeting_max {
        Temperament::Fleeting
    } else {
        Temperament::Intense
    };
    (die, all_rolls, temperament)
}

pub fn roll_resonance_type(weights: &ResonanceWeights) -> (u8, ResonanceType) {
    let display_die = roll_d10(); // shown to GM for flavour
    let resonance_type = weighted_resonance_pick(weights);
    (display_die, resonance_type)
}

pub fn check_acute() -> (u8, bool) {
    let die = roll_d10();
    (die, die >= 9)
}

pub fn execute_roll(config: &RollConfig) -> ResonanceRollResult {
    let (temperament_die, temperament_dice, temperament) =
        roll_temperament(&config.temperament);

    match temperament {
        Temperament::Negligible => ResonanceRollResult {
            temperament_dice,
            temperament_die,
            temperament: Temperament::Negligible,
            resonance_type: None,
            resonance_die: None,
            acute_die: None,
            is_acute: false,
            dyscrasia: None,
        },
        Temperament::Fleeting => {
            let (res_die, res_type) = roll_resonance_type(&config.weights);
            ResonanceRollResult {
                temperament_dice,
                temperament_die,
                temperament: Temperament::Fleeting,
                resonance_type: Some(res_type),
                resonance_die: Some(res_die),
                acute_die: None,
                is_acute: false,
                dyscrasia: None,
            }
        }
        Temperament::Intense => {
            let (res_die, res_type) = roll_resonance_type(&config.weights);
            let (acute_die, is_acute) = check_acute();
            ResonanceRollResult {
                temperament_dice,
                temperament_die,
                temperament: Temperament::Intense,
                resonance_type: Some(res_type),
                resonance_die: Some(res_die),
                acute_die: Some(acute_die),
                is_acute,
                dyscrasia: None,
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml shared::resonance 2>&1
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/shared/resonance.rs
git commit -m "feat: implement resonance roll logic with tests"
```

---

## Task 5: Set up SQLite schema, migrations, and seed data

**Files:**
- Create: `src-tauri/migrations/0001_initial.sql`
- Create: `src-tauri/src/db/seed.rs`

- [ ] **Step 1: Write the migration**

Create `src-tauri/migrations/0001_initial.sql`:

```sql
CREATE TABLE IF NOT EXISTS dyscrasias (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    resonance_type  TEXT    NOT NULL CHECK(resonance_type IN ('Phlegmatic','Melancholy','Choleric','Sanguine')),
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL,
    bonus           TEXT    NOT NULL,
    is_custom       INTEGER NOT NULL DEFAULT 0
);
```

- [ ] **Step 2: Write seed data**

Create `src-tauri/src/db/seed.rs`:

```rust
use sqlx::SqlitePool;

/// Inserts canonical Dyscrasia entries if the table is empty.
/// Verify these entries against the VTM 5e Corebook before shipping.
pub async fn seed_dyscrasias(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM dyscrasias WHERE is_custom = 0")
        .fetch_one(pool)
        .await?;

    if count.0 > 0 {
        return Ok(()); // already seeded
    }

    let entries: &[(&str, &str, &str, &str)] = &[
        // (resonance_type, name, description, bonus)
        // Phlegmatic — associated with Fortitude and Auspex
        ("Phlegmatic", "Unshakeable Calm",
         "The vessel exists in a state of profound emotional equilibrium, unmoved by fear or chaos.",
         "+2 dice to Fortitude-related rolls"),
        ("Phlegmatic", "Still Waters",
         "The vessel's thoughts run deep and slow, their aura almost impossible to read.",
         "+2 dice to Auspex-related discipline rolls"),
        // Melancholy — associated with Oblivion and Obfuscate
        ("Melancholy", "Haunted",
         "The vessel is touched by loss so deep it leaves a stain on the soul.",
         "+2 dice to Oblivion-related discipline rolls"),
        ("Melancholy", "Hollow",
         "The vessel moves through the world like a ghost, barely present.",
         "+2 dice to Obfuscate-related discipline rolls"),
        // Choleric — associated with Celerity and Potence
        ("Choleric", "Berserker's Blood",
         "The vessel's rage is so pure it feels like a living thing in the veins.",
         "+2 dice to Potence-related discipline rolls"),
        ("Choleric", "Hair-Trigger",
         "Violence lives in the vessel's reflexes; they move before they think.",
         "+2 dice to Celerity-related discipline rolls"),
        // Sanguine — associated with Presence and Blood Sorcery
        ("Sanguine", "True Believer",
         "The vessel's faith or passion is so absolute it warps the blood.",
         "+2 dice to Presence-related discipline rolls"),
        ("Sanguine", "Ecstatic",
         "The vessel exists in a heightened state of bliss that makes their blood almost luminous.",
         "+2 dice to Blood Sorcery-related discipline rolls"),
    ];

    for (resonance_type, name, description, bonus) in entries {
        sqlx::query(
            "INSERT INTO dyscrasias (resonance_type, name, description, bonus, is_custom)
             VALUES (?, ?, ?, ?, 0)"
        )
        .bind(resonance_type)
        .bind(name)
        .bind(description)
        .bind(bonus)
        .execute(pool)
        .await?;
    }

    Ok(())
}
```

- [ ] **Step 3: Verify migration and seed compile**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "error|warning: unused"
```

Expected: no errors. Unused import warnings are fine.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/ src-tauri/src/db/seed.rs
git commit -m "feat: add sqlite migration and dyscrasia seed data"
```

---

## Task 6: Implement Dyscrasia CRUD commands (TDD)

**Files:**
- Create: `src-tauri/src/db/dyscrasia.rs`

- [ ] **Step 1: Write tests**

Create `src-tauri/src/db/dyscrasia.rs` with stubs + tests:

```rust
use sqlx::SqlitePool;
use crate::shared::types::{DyscrasiaEntry, ResonanceType};

pub async fn list_dyscrasias(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
) -> Result<Vec<DyscrasiaEntry>, String> {
    todo!()
}

pub async fn add_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
    name: String,
    description: String,
    bonus: String,
) -> Result<DyscrasiaEntry, String> {
    todo!()
}

pub async fn update_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
    bonus: String,
) -> Result<(), String> {
    todo!()
}

pub async fn delete_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    todo!()
}

pub async fn roll_random_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
) -> Result<Option<DyscrasiaEntry>, String> {
    todo!()
}

// --- SQL helper functions (not Tauri commands — called by the above) ---

async fn db_list(pool: &SqlitePool, rtype: &str) -> Result<Vec<DyscrasiaEntry>, sqlx::Error> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn test_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE dyscrasias (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                resonance_type TEXT NOT NULL,
                name TEXT NOT NULL,
                description TEXT NOT NULL,
                bonus TEXT NOT NULL,
                is_custom INTEGER NOT NULL DEFAULT 0
            )"
        ).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn list_empty_returns_empty_vec() {
        let pool = test_pool().await;
        let result = db_list(&pool, "Choleric").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn insert_and_list_round_trips() {
        let pool = test_pool().await;
        sqlx::query(
            "INSERT INTO dyscrasias (resonance_type, name, description, bonus, is_custom)
             VALUES ('Choleric', 'Rage', 'Pure anger', '+1 Potence', 1)"
        ).execute(&pool).await.unwrap();

        let entries = db_list(&pool, "Choleric").await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Rage");
        assert_eq!(entries[0].resonance_type, ResonanceType::Choleric);
        assert!(entries[0].is_custom);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::dyscrasia 2>&1
```

Expected: compile or todo panic. Correct.

- [ ] **Step 3: Implement Dyscrasia CRUD**

Replace the `todo!()` bodies:

```rust
use rand::Rng;
use sqlx::{Row, SqlitePool};
use crate::shared::types::{DyscrasiaEntry, ResonanceType};

fn rtype_to_str(r: &ResonanceType) -> &'static str {
    match r {
        ResonanceType::Phlegmatic => "Phlegmatic",
        ResonanceType::Melancholy => "Melancholy",
        ResonanceType::Choleric   => "Choleric",
        ResonanceType::Sanguine   => "Sanguine",
    }
}

fn str_to_rtype(s: &str) -> ResonanceType {
    match s {
        "Phlegmatic" => ResonanceType::Phlegmatic,
        "Melancholy" => ResonanceType::Melancholy,
        "Choleric"   => ResonanceType::Choleric,
        _            => ResonanceType::Sanguine,
    }
}

async fn db_list(pool: &SqlitePool, rtype: &str) -> Result<Vec<DyscrasiaEntry>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, resonance_type, name, description, bonus, is_custom
         FROM dyscrasias WHERE resonance_type = ? ORDER BY is_custom ASC, id ASC"
    )
    .bind(rtype)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(|r| DyscrasiaEntry {
        id: r.get("id"),
        resonance_type: str_to_rtype(r.get("resonance_type")),
        name: r.get("name"),
        description: r.get("description"),
        bonus: r.get("bonus"),
        is_custom: r.get::<bool, _>("is_custom"),
    }).collect())
}

#[tauri::command]
pub async fn list_dyscrasias(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
) -> Result<Vec<DyscrasiaEntry>, String> {
    db_list(&pool.0, rtype_to_str(&resonance_type)).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
    name: String,
    description: String,
    bonus: String,
) -> Result<DyscrasiaEntry, String> {
    let rtype = rtype_to_str(&resonance_type);
    let result = sqlx::query(
        "INSERT INTO dyscrasias (resonance_type, name, description, bonus, is_custom)
         VALUES (?, ?, ?, ?, 1)"
    )
    .bind(rtype).bind(&name).bind(&description).bind(&bonus)
    .execute(&*pool.0)
    .await
    .map_err(|e| e.to_string())?;

    Ok(DyscrasiaEntry {
        id: result.last_insert_rowid(),
        resonance_type,
        name,
        description,
        bonus,
        is_custom: true,
    })
}

#[tauri::command]
pub async fn update_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    description: String,
    bonus: String,
) -> Result<(), String> {
    sqlx::query(
        "UPDATE dyscrasias SET name = ?, description = ?, bonus = ? WHERE id = ? AND is_custom = 1"
    )
    .bind(&name).bind(&description).bind(&bonus).bind(id)
    .execute(&*pool.0)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
) -> Result<(), String> {
    sqlx::query("DELETE FROM dyscrasias WHERE id = ? AND is_custom = 1")
        .bind(id)
        .execute(&*pool.0)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn roll_random_dyscrasia(
    pool: tauri::State<'_, crate::DbState>,
    resonance_type: ResonanceType,
) -> Result<Option<DyscrasiaEntry>, String> {
    let entries = db_list(&pool.0, rtype_to_str(&resonance_type))
        .await
        .map_err(|e| e.to_string())?;
    if entries.is_empty() {
        return Ok(None);
    }
    let idx = rand::thread_rng().gen_range(0..entries.len());
    Ok(Some(entries[idx].clone()))
}
```

Add `use rand::Rng;` at the top.

- [ ] **Step 4: Run tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml db::dyscrasia 2>&1
```

Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/db/dyscrasia.rs
git commit -m "feat: implement dyscrasia CRUD with sqlite tests"
```

---

## Task 7: Implement the roll_resonance Tauri command

**Files:**
- Create: `src-tauri/src/tools/resonance.rs`

- [ ] **Step 1: Create the command**

Create `src-tauri/src/tools/resonance.rs`:

```rust
use crate::shared::resonance::execute_roll;
use crate::shared::types::{ResonanceRollResult, RollConfig};

/// Executes the full resonance roll sequence.
/// The dyscrasia field in the result is always None — the GM fetches it
/// separately via roll_random_dyscrasia or picks manually.
#[tauri::command]
pub fn roll_resonance(config: RollConfig) -> ResonanceRollResult {
    execute_roll(&config)
}
```

- [ ] **Step 2: Verify it compiles cleanly**

```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/tools/resonance.rs
git commit -m "feat: add roll_resonance tauri command"
```

---

## Task 8: Set up tool registry, sidebar, and SvelteKit layout

**Files:**
- Create: `src/tools.ts`
- Create: `src/store/toolEvents.ts`
- Create: `src/lib/components/Sidebar.svelte`
- Modify: `src/routes/+layout.svelte`
- Modify: `src/routes/+page.svelte`
- Create: `src/tools/Resonance.svelte` (stub only)

- [ ] **Step 1: Write the tool registry**

Create `src/tools.ts`:

```typescript
import type { ComponentType } from 'svelte';

export interface Tool {
  id: string;
  label: string;
  icon: string; // emoji or SVG string
  component: () => Promise<{ default: ComponentType }>;
}

// Add new tools here — the sidebar renders from this list automatically.
export const tools: Tool[] = [
  {
    id: 'resonance',
    label: 'Resonance Roller',
    icon: '🩸',
    component: () => import('./tools/Resonance.svelte'),
  },
  // Future tools go here, e.g.:
  // { id: 'combat', label: 'Combat Tracker', icon: '⚔️', component: () => import('./tools/Combat.svelte') },
];
```

- [ ] **Step 2: Write the pub/sub tool events store**

Create `src/store/toolEvents.ts`:

```typescript
import { writable, get } from 'svelte/store';
import type { Writable } from 'svelte/store';

export interface ResonanceEvent {
  type: 'resonance_result';
  payload: {
    temperament: string;
    resonanceType: string | null;
    isAcute: boolean;
    dyscrasiaName: string | null;
  };
}

export type ToolEvent = ResonanceEvent;

// Tools publish events here. Other tools subscribe as needed.
export const toolEvents: Writable<ToolEvent | null> = writable(null);

export function publishEvent(event: ToolEvent): void {
  toolEvents.set(event);
}
```

- [ ] **Step 3: Create Sidebar component**

Create `src/lib/components/Sidebar.svelte`:

```svelte
<script lang="ts">
  import { tools } from '../../tools';

  export let activeTool: string;
  export let onSelect: (id: string) => void;
</script>

<nav class="sidebar">
  {#each tools as tool}
    <button
      class="tool-btn"
      class:active={activeTool === tool.id}
      on:click={() => onSelect(tool.id)}
      aria-label={tool.label}
    >
      <span class="icon">{tool.icon}</span>
      <span class="label">{tool.label}</span>
    </button>
  {/each}
</nav>

<style>
  .sidebar {
    width: 200px;
    min-height: 100vh;
    background: #0d0d0d;
    border-right: 1px solid #3a0a0a;
    display: flex;
    flex-direction: column;
    padding: 1rem 0;
  }
  .tool-btn {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.75rem 1rem;
    background: none;
    border: none;
    color: #8a8a8a;
    cursor: pointer;
    font-size: 0.9rem;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }
  .tool-btn:hover {
    background: #1a0505;
    color: #cc2222;
  }
  .tool-btn.active {
    background: #1a0505;
    color: #cc2222;
    border-left: 3px solid #cc2222;
  }
  .icon { font-size: 1.1rem; }
  .label { font-weight: 500; }
</style>
```

- [ ] **Step 4: Write the layout**

Replace `src/routes/+layout.svelte` content:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';
  import { tools } from '../tools';
  import type { ComponentType } from 'svelte';

  let activeTool = tools[0].id;
  let ActiveComponent: ComponentType | null = null;

  async function loadTool(id: string) {
    const tool = tools.find(t => t.id === id);
    if (!tool) return;
    activeTool = id;
    const mod = await tool.component();
    ActiveComponent = mod.default;
  }

  onMount(() => loadTool(activeTool));
</script>

<div class="shell">
  <Sidebar {activeTool} onSelect={loadTool} />
  <main class="content">
    {#if ActiveComponent}
      <svelte:component this={ActiveComponent} />
    {:else}
      <p class="loading">Loading…</p>
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    background: #0d0d0d;
    color: #d4c5a9;
    font-family: 'Georgia', serif;
  }
  .shell {
    display: flex;
    min-height: 100vh;
  }
  .content {
    flex: 1;
    padding: 1.5rem;
    overflow-y: auto;
  }
  .loading {
    color: #555;
    font-style: italic;
  }
</style>
```

Replace `src/routes/+page.svelte` content:

```svelte
<!-- Layout handles routing — this page is intentionally empty -->
```

- [ ] **Step 5: Create stub Resonance.svelte**

Create `src/tools/Resonance.svelte`:

```svelte
<h2>Resonance Roller</h2>
<p>Coming soon.</p>
```

- [ ] **Step 6: Test in dev**

```bash
cargo tauri dev
```

Expected: sidebar appears on the left with the Resonance Roller entry. Clicking it shows "Resonance Roller / Coming soon."

- [ ] **Step 7: Commit**

```bash
git add src/
git commit -m "feat: add tool registry, pub-sub store, and sidebar layout"
```

---

## Task 9: Build shared UI components

**Files:**
- Create: `src/lib/components/ResonanceSlider.svelte`
- Create: `src/lib/components/TemperamentConfig.svelte`
- Create: `src/lib/components/ResultCard.svelte`

- [ ] **Step 1: ResonanceSlider component**

Create `src/lib/components/ResonanceSlider.svelte`:

```svelte
<script lang="ts">
  export let label: string;
  export let value: string = 'neutral';
  export let onChange: (v: string) => void;

  const levels = [
    { id: 'impossible',       display: 'Impossible' },
    { id: 'extremelyUnlikely',display: 'Ext. Unlikely' },
    { id: 'unlikely',         display: 'Unlikely' },
    { id: 'neutral',          display: 'Neutral' },
    { id: 'likely',           display: 'Likely' },
    { id: 'extremelyLikely',  display: 'Ext. Likely' },
    { id: 'guaranteed',       display: 'Guaranteed' },
  ];

  $: selectedIndex = levels.findIndex(l => l.id === value);
</script>

<div class="slider-row">
  <span class="slider-label">{label}</span>
  <div class="steps">
    {#each levels as level, i}
      <button
        class="step"
        class:active={i === selectedIndex}
        class:filled={i <= selectedIndex}
        on:click={() => onChange(level.id)}
        title={level.display}
        aria-label="{label}: {level.display}"
      ></button>
    {/each}
  </div>
  <span class="slider-value">{levels[selectedIndex]?.display ?? ''}</span>
</div>

<style>
  .slider-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
  }
  .slider-label {
    width: 90px;
    font-size: 0.85rem;
    color: #a09070;
  }
  .steps {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  .step {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid #3a1a1a;
    background: #1a0d0d;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    padding: 0;
  }
  .step.filled {
    background: #6b1010;
    border-color: #8a1515;
  }
  .step.active {
    background: #cc2222;
    border-color: #ee3333;
    box-shadow: 0 0 6px #cc222266;
  }
  .slider-value {
    font-size: 0.8rem;
    color: #cc2222;
    width: 90px;
  }
</style>
```

- [ ] **Step 2: TemperamentConfig component**

Create `src/lib/components/TemperamentConfig.svelte`:

```svelte
<script lang="ts">
  export let diceCount: number = 1;
  export let takeHighest: boolean = true;
  export let negligibleMax: number = 5;
  export let fleetingMax: number = 8;

  export let onDiceCountChange: (n: number) => void;
  export let onTakeHighestChange: (b: boolean) => void;
  export let onNegligibleMaxChange: (n: number) => void;
  export let onFleetingMaxChange: (n: number) => void;

  const diceCounts = [1, 2, 3, 4, 5];
</script>

<div class="temp-config">
  <div class="row">
    <label>Dice pool</label>
    <div class="dice-buttons">
      {#each diceCounts as n}
        <button
          class="die-btn"
          class:active={diceCount === n}
          on:click={() => onDiceCountChange(n)}
        >{n}d10</button>
      {/each}
    </div>
  </div>

  {#if diceCount > 1}
    <div class="row">
      <label>Take</label>
      <div class="take-buttons">
        <button class="take-btn" class:active={takeHighest} on:click={() => onTakeHighestChange(true)}>
          Highest (→ Intense)
        </button>
        <button class="take-btn" class:active={!takeHighest} on:click={() => onTakeHighestChange(false)}>
          Lowest (→ Negligible)
        </button>
      </div>
    </div>
  {/if}

  <div class="row">
    <label>Thresholds</label>
    <span class="threshold-display">
      Neg 1–{negligibleMax} / Flee {negligibleMax + 1}–{fleetingMax} / Int {fleetingMax + 1}–10
    </span>
  </div>

  <div class="row">
    <label>Negligible max</label>
    <input
      type="range" min="0" max="9" bind:value={negligibleMax}
      on:input={() => {
        if (negligibleMax >= fleetingMax) fleetingMax = negligibleMax + 1;
        onNegligibleMaxChange(negligibleMax);
      }}
    />
    <span>{negligibleMax}</span>
  </div>

  <div class="row">
    <label>Fleeting max</label>
    <input
      type="range"
      min={negligibleMax + 1}
      max="10"
      bind:value={fleetingMax}
      on:input={() => onFleetingMaxChange(fleetingMax)}
    />
    <span>{fleetingMax}</span>
  </div>
</div>

<style>
  .temp-config { display: flex; flex-direction: column; gap: 0.6rem; }
  .row { display: flex; align-items: center; gap: 0.75rem; }
  label { width: 110px; font-size: 0.85rem; color: #a09070; flex-shrink: 0; }
  .die-btn, .take-btn {
    padding: 0.25rem 0.6rem;
    background: #1a0d0d;
    border: 1px solid #3a1a1a;
    color: #8a8a8a;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.8rem;
    transition: background 0.15s, color 0.15s;
  }
  .die-btn.active, .take-btn.active {
    background: #3a0808;
    color: #cc2222;
    border-color: #cc2222;
  }
  .dice-buttons, .take-buttons { display: flex; gap: 4px; flex-wrap: wrap; }
  .threshold-display { font-size: 0.8rem; color: #cc2222; }
  input[type=range] { accent-color: #cc2222; width: 120px; }
  span { font-size: 0.85rem; color: #d4c5a9; min-width: 20px; }
</style>
```

- [ ] **Step 3: ResultCard component**

Create `src/lib/components/ResultCard.svelte`:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import type { ResonanceRollResult, DyscrasiaEntry } from '../../types';

  export let result: ResonanceRollResult;
  export let resonanceType: string | null;

  let dyscrasias: DyscrasiaEntry[] = [];
  let selectedDyscrasia: DyscrasiaEntry | null = null;
  let loadingDyscrasias = false;

  $: if (result.isAcute && resonanceType) {
    loadDyscrasias(resonanceType);
  }

  async function loadDyscrasias(rtype: string) {
    loadingDyscrasias = true;
    dyscrasias = await invoke<DyscrasiaEntry[]>('list_dyscrasias', { resonanceType: rtype });
    loadingDyscrasias = false;
  }

  async function rollRandomDyscrasia() {
    selectedDyscrasia = await invoke<DyscrasiaEntry | null>('roll_random_dyscrasia', {
      resonanceType: resonanceType
    });
  }

  async function exportToMd() {
    await invoke('export_result_to_md', { result, dyscrasia: selectedDyscrasia });
  }
</script>

<div class="result-card">
  <div class="result-row">
    <span class="label">Temperament</span>
    <span class="value {result.temperament.toLowerCase()}">
      {result.temperament.toUpperCase()}
      <span class="dice-info">
        (rolled {result.temperamentDie}
        {#if result.temperamentDice.length > 1}
          from [{result.temperamentDice.join(', ')}]
        {/if})
      </span>
    </span>
  </div>

  {#if result.resonanceType}
    <div class="result-row">
      <span class="label">Resonance</span>
      <span class="value">{result.resonanceType}</span>
    </div>
  {/if}

  {#if result.acuteDie !== null && result.acuteDie !== undefined}
    <div class="result-row">
      <span class="label">Acute check</span>
      <span class="value {result.isAcute ? 'acute' : ''}">
        {result.isAcute ? 'ACUTE' : 'Not Acute'} (rolled {result.acuteDie})
      </span>
    </div>
  {/if}

  {#if result.isAcute}
    <div class="dyscrasia-section">
      <span class="label">Dyscrasia</span>
      <div class="dyscrasia-actions">
        <button class="action-btn" on:click={rollRandomDyscrasia} disabled={loadingDyscrasias}>
          Roll randomly
        </button>
        <select
          class="pick-select"
          on:change={(e) => {
            const id = parseInt(e.currentTarget.value);
            selectedDyscrasia = dyscrasias.find(d => d.id === id) ?? null;
          }}
        >
          <option value="">— Pick manually —</option>
          {#each dyscrasias as d}
            <option value={d.id}>{d.name}</option>
          {/each}
        </select>
      </div>

      {#if selectedDyscrasia}
        <div class="dyscrasia-detail">
          <strong>{selectedDyscrasia.name}</strong>
          <p>{selectedDyscrasia.description}</p>
          <span class="bonus">{selectedDyscrasia.bonus}</span>
        </div>
      {/if}
    </div>
  {/if}

  <div class="card-footer">
    <button class="export-btn" on:click={exportToMd}>Export to .md</button>
  </div>
</div>

<style>
  .result-card {
    background: #1a0d0d;
    border: 1px solid #3a1a1a;
    border-radius: 6px;
    padding: 1.25rem;
    margin-top: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .result-row { display: flex; align-items: baseline; gap: 1rem; }
  .label { width: 110px; color: #a09070; font-size: 0.85rem; flex-shrink: 0; }
  .value { font-size: 1rem; color: #d4c5a9; font-weight: 600; }
  .value.negligible { color: #6a6a6a; }
  .value.fleeting { color: #cc9922; }
  .value.intense { color: #cc2222; }
  .value.acute { color: #ff4444; text-shadow: 0 0 8px #cc222288; }
  .dice-info { font-size: 0.8rem; color: #666; font-weight: 400; margin-left: 0.5rem; }
  .dyscrasia-section { display: flex; flex-direction: column; gap: 0.5rem; border-top: 1px solid #3a1a1a; padding-top: 0.75rem; }
  .dyscrasia-actions { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
  .action-btn {
    padding: 0.3rem 0.8rem;
    background: #3a0808;
    border: 1px solid #cc2222;
    color: #cc2222;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.85rem;
  }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .pick-select {
    background: #1a0d0d;
    border: 1px solid #3a1a1a;
    color: #d4c5a9;
    padding: 0.25rem;
    border-radius: 3px;
  }
  .dyscrasia-detail {
    background: #0d0505;
    border: 1px solid #2a0a0a;
    border-radius: 4px;
    padding: 0.75rem;
  }
  .dyscrasia-detail strong { color: #cc2222; }
  .dyscrasia-detail p { color: #a09070; font-size: 0.85rem; margin: 0.4rem 0; }
  .bonus { color: #cc9922; font-size: 0.85rem; }
  .card-footer { display: flex; justify-content: flex-end; border-top: 1px solid #3a1a1a; padding-top: 0.75rem; }
  .export-btn {
    padding: 0.3rem 0.8rem;
    background: #1a1a0d;
    border: 1px solid #6a5a20;
    color: #cc9922;
    cursor: pointer;
    border-radius: 3px;
    font-size: 0.8rem;
  }
</style>
```

- [ ] **Step 4: Create shared types file**

Create `src/types.ts`:

```typescript
export interface TemperamentConfig {
  diceCount: number;
  takeHighest: boolean;
  negligibleMax: number;
  fleetingMax: number;
}

export interface ResonanceWeights {
  phlegmatic: string;
  melancholy: string;
  choleric: string;
  sanguine: string;
}

export interface RollConfig {
  temperament: TemperamentConfig;
  weights: ResonanceWeights;
}

export interface DyscrasiaEntry {
  id: number;
  resonanceType: string;
  name: string;
  description: string;
  bonus: string;
  isCustom: boolean;
}

export interface ResonanceRollResult {
  temperamentDice: number[];
  temperamentDie: number;
  temperament: 'Negligible' | 'Fleeting' | 'Intense';
  resonanceType: string | null;
  resonanceDie: number | null;
  acuteDie: number | null;
  isAcute: boolean;
  dyscrasia: DyscrasiaEntry | null;
}
```

- [ ] **Step 5: Verify no TypeScript errors**

```bash
npm run check
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/
git commit -m "feat: add shared UI components and TypeScript types"
```

---

## Task 10: Build the full Resonance Roller UI

**Files:**
- Modify: `src/tools/Resonance.svelte` (replace stub)

- [ ] **Step 1: Write Resonance.svelte**

Replace `src/tools/Resonance.svelte`:

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ResonanceSlider from '$lib/components/ResonanceSlider.svelte';
  import TemperamentConfigComponent from '$lib/components/TemperamentConfig.svelte';
  import ResultCard from '$lib/components/ResultCard.svelte';
  import { publishEvent } from '../store/toolEvents';
  import type { RollConfig, ResonanceRollResult } from '../types';

  let config: RollConfig = {
    temperament: {
      diceCount: 1,
      takeHighest: true,
      negligibleMax: 5,
      fleetingMax: 8,
    },
    weights: {
      phlegmatic: 'neutral',
      melancholy: 'neutral',
      choleric: 'neutral',
      sanguine: 'neutral',
    }
  };

  let result: ResonanceRollResult | null = null;
  let rolling = false;

  // Live summary values (derived from config)
  $: summary = buildSummary(config);

  function buildSummary(c: RollConfig) {
    const t = c.temperament;
    const diceLabel = t.diceCount === 1
      ? '1 die (standard)'
      : `${t.diceCount} dice — take ${t.takeHighest ? 'highest' : 'lowest'}`;
    return {
      dice: diceLabel,
      thresholds: `Neg 1–${t.negligibleMax} / Flee ${t.negligibleMax + 1}–${t.fleetingMax} / Int ${t.fleetingMax + 1}–10`,
    };
  }

  async function roll() {
    rolling = true;
    result = null;
    try {
      result = await invoke<ResonanceRollResult>('roll_resonance', { config });
      if (result) {
        publishEvent({
          type: 'resonance_result',
          payload: {
            temperament: result.temperament,
            resonanceType: result.resonanceType,
            isAcute: result.isAcute,
            dyscrasiaName: result.dyscrasia?.name ?? null,
          }
        });
      }
    } finally {
      rolling = false;
    }
  }
</script>

<div class="page">
  <h1 class="title">Resonance Roller</h1>
  <p class="subtitle">Configure the feeding conditions, then roll.</p>

  <div class="main-layout">
    <!-- LEFT: Step wizard -->
    <div class="steps-panel">
      <section class="step">
        <h3>1. Temperament dice</h3>
        <TemperamentConfigComponent
          diceCount={config.temperament.diceCount}
          takeHighest={config.temperament.takeHighest}
          negligibleMax={config.temperament.negligibleMax}
          fleetingMax={config.temperament.fleetingMax}
          onDiceCountChange={(n) => (config.temperament.diceCount = n)}
          onTakeHighestChange={(b) => (config.temperament.takeHighest = b)}
          onNegligibleMaxChange={(n) => (config.temperament.negligibleMax = n)}
          onFleetingMaxChange={(n) => (config.temperament.fleetingMax = n)}
        />
      </section>

      <section class="step">
        <h3>2. Resonance type odds</h3>
        <ResonanceSlider
          label="Phlegmatic"
          value={config.weights.phlegmatic}
          onChange={(v) => (config.weights.phlegmatic = v)}
        />
        <ResonanceSlider
          label="Melancholy"
          value={config.weights.melancholy}
          onChange={(v) => (config.weights.melancholy = v)}
        />
        <ResonanceSlider
          label="Choleric"
          value={config.weights.choleric}
          onChange={(v) => (config.weights.choleric = v)}
        />
        <ResonanceSlider
          label="Sanguine"
          value={config.weights.sanguine}
          onChange={(v) => (config.weights.sanguine = v)}
        />
      </section>

      <div class="roll-area">
        <button class="roll-btn" on:click={roll} disabled={rolling}>
          {rolling ? 'Rolling…' : '⚀ Roll'}
        </button>
      </div>

      {#if result}
        <ResultCard
          {result}
          resonanceType={result.resonanceType}
        />
      {/if}
    </div>

    <!-- RIGHT: Live summary -->
    <div class="summary-panel">
      <h3>Current Settings</h3>
      <div class="summary-row">
        <span class="sum-label">Temperament</span>
        <span class="sum-value">{summary.dice}</span>
      </div>
      <div class="summary-row">
        <span class="sum-label">Thresholds</span>
        <span class="sum-value">{summary.thresholds}</span>
      </div>
      <div class="summary-row">
        <span class="sum-label">Resonance odds</span>
        <div class="sum-weights">
          {#each Object.entries(config.weights) as [type, level]}
            <span class="weight-pill" class:modified={level !== 'neutral'}>
              {type.charAt(0).toUpperCase() + type.slice(1)}: {level}
            </span>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .page { max-width: 900px; }
  .title { color: #cc2222; font-size: 1.8rem; margin-bottom: 0.25rem; }
  .subtitle { color: #6a5a40; font-size: 0.9rem; margin-bottom: 1.5rem; }
  .main-layout { display: flex; gap: 2rem; align-items: flex-start; }
  .steps-panel { flex: 1; display: flex; flex-direction: column; gap: 1.5rem; }
  .step {
    background: #120808;
    border: 1px solid #2a1010;
    border-radius: 6px;
    padding: 1rem 1.25rem;
  }
  h3 { color: #a09070; font-size: 0.9rem; text-transform: uppercase;
       letter-spacing: 0.08em; margin: 0 0 0.75rem; }
  .roll-area { display: flex; justify-content: center; }
  .roll-btn {
    padding: 0.75rem 2.5rem;
    background: #3a0808;
    border: 2px solid #cc2222;
    color: #cc2222;
    font-size: 1.1rem;
    font-family: 'Georgia', serif;
    cursor: pointer;
    border-radius: 4px;
    transition: background 0.2s, box-shadow 0.2s;
    letter-spacing: 0.05em;
  }
  .roll-btn:hover:not(:disabled) {
    background: #5a0808;
    box-shadow: 0 0 16px #cc222244;
  }
  .roll-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .summary-panel {
    width: 220px;
    flex-shrink: 0;
    background: #120808;
    border: 1px solid #2a1010;
    border-radius: 6px;
    padding: 1rem;
    position: sticky;
    top: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }
  .summary-row { display: flex; flex-direction: column; gap: 0.2rem; }
  .sum-label { font-size: 0.75rem; color: #6a5a40; text-transform: uppercase; letter-spacing: 0.06em; }
  .sum-value { font-size: 0.85rem; color: #d4c5a9; }
  .sum-weights { display: flex; flex-direction: column; gap: 0.2rem; margin-top: 0.2rem; }
  .weight-pill { font-size: 0.8rem; color: #6a6a6a; }
  .weight-pill.modified { color: #cc2222; }
</style>
```

- [ ] **Step 2: Smoke test in dev**

```bash
cargo tauri dev
```

Expected: The full step wizard renders. Sliders and dice config are interactive. Summary panel updates as you change options. Roll button calls the Rust command and displays the result card.

- [ ] **Step 3: Commit**

```bash
git add src/tools/Resonance.svelte
git commit -m "feat: build resonance roller step wizard UI"
```

---

## Task 11: Implement MD export command

**Files:**
- Create: `src-tauri/src/tools/export.rs`
- Modify: `src-tauri/src/tools/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the export command**

Create `src-tauri/src/tools/export.rs`:

```rust
use serde_json::Value;
use std::path::PathBuf;
use tauri::Manager;

/// Formats a JSON value to a Markdown string. Pure function — no DB access.
pub fn format_to_md(json: &Value) -> String {
    let mut md = String::new();
    md.push_str("# VTM Roll Result\n\n");

    if let Some(obj) = json.as_object() {
        for (key, val) in obj {
            let label = key
                .chars()
                .enumerate()
                .map(|(i, c)| if i == 0 { c.to_uppercase().next().unwrap_or(c) } else { c })
                .collect::<String>()
                .replace('_', " ");
            let value_str = match val {
                Value::String(s) => s.clone(),
                Value::Bool(b)   => b.to_string(),
                Value::Number(n) => n.to_string(),
                Value::Null      => "—".to_string(),
                _                => val.to_string(),
            };
            md.push_str(&format!("**{label}:** {value_str}\n\n"));
        }
    }

    md
}

#[tauri::command]
pub async fn export_result_to_md(
    app: tauri::AppHandle,
    result: Value,
    dyscrasia: Option<Value>,
) -> Result<String, String> {
    let mut combined = serde_json::json!({});
    if let Some(obj) = result.as_object() {
        for (k, v) in obj { combined[k] = v.clone(); }
    }
    if let Some(d) = dyscrasia {
        combined["dyscrasia"] = d;
    }
    combined["exported_at"] = Value::String(chrono::Local::now().to_rfc2822());

    let md = format_to_md(&combined);

    let export_dir = app.path().document_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("vtmtools");
    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let filename = format!("resonance_{}.md",
        chrono::Local::now().format("%Y%m%d_%H%M%S"));
    let path = export_dir.join(&filename);
    std::fs::write(&path, &md).map_err(|e| e.to_string())?;

    Ok(path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_to_md_includes_keys_and_values() {
        let json = json!({ "temperament": "Intense", "is_acute": true });
        let md = format_to_md(&json);
        assert!(md.contains("Intense"));
        assert!(md.contains("true"));
    }
}
```

Add `chrono = { version = "0.4", features = ["local-offset"] }` to `src-tauri/Cargo.toml` dependencies.

- [ ] **Step 2: Register the module and command**

Add to `src-tauri/src/tools/mod.rs`:
```rust
pub mod export;
```

Add to the `invoke_handler` in `src-tauri/src/lib.rs`:
```rust
tools::export::export_result_to_md,
```

Add to the capabilities file `src-tauri/capabilities/default.json` — ensure `fs:allow-write-file`, `fs:allow-create-dir`, and `fs:allow-document-dir` are present (or use `fs:default` if available in your Tauri version).

- [ ] **Step 3: Run export tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml tools::export 2>&1
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/
git commit -m "feat: implement md export command"
```

---

## Task 12: Configure auto-updater and GitHub Actions CI

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `.github/workflows/release.yml`

- [ ] **Step 1: Configure the updater in tauri.conf.json**

Open `src-tauri/tauri.conf.json`. Add/merge the `plugins` section:

```json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/YOUR_USERNAME/vtmtools/releases/latest/download/latest.json"
      ],
      "dialog": true,
      "pubkey": ""
    }
  }
}
```

Replace `YOUR_USERNAME` with your actual GitHub username. The `pubkey` field will be populated with a signing key after running `cargo tauri signer generate` in Step 3.

- [ ] **Step 2: Generate an updater signing key**

```bash
cargo tauri signer generate -w ~/.tauri/vtmtools.key
```

This outputs a private key (save securely) and a public key. Copy the public key string into the `pubkey` field in `tauri.conf.json`.

Add the private key as a GitHub secret named `TAURI_SIGNING_PRIVATE_KEY` in your repo settings (Settings → Secrets → Actions).

- [ ] **Step 3: Create the release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

jobs:
  release:
    strategy:
      fail-fast: false
      matrix:
        include:
          - platform: ubuntu-22.04
            args: ''
          - platform: windows-latest
            args: ''

    runs-on: ${{ matrix.platform }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Linux deps
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'npm'

      - name: Setup Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Rust cache
        uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - name: Install frontend deps
        run: npm install

      - name: Build and release
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: 'vtmtools ${{ github.ref_name }}'
          releaseBody: 'See CHANGELOG for details.'
          releaseDraft: false
          prerelease: false
          args: ${{ matrix.args }}
```

- [ ] **Step 4: Commit**

```bash
git add .github/ src-tauri/tauri.conf.json
git commit -m "feat: configure auto-updater and github actions release CI"
```

---

## Task 13: End-to-end smoke test

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1
```

Expected: all tests pass.

- [ ] **Step 2: Run TypeScript check**

```bash
npm run check
```

Expected: no errors.

- [ ] **Step 3: Full dev run smoke test**

```bash
cargo tauri dev
```

Manually verify the following in the app:
- [ ] Sidebar shows "Resonance Roller" with blood-drop icon
- [ ] Dice pool options (1–5 dice) are selectable; "Take highest/lowest" appears when > 1 die selected
- [ ] Threshold sliders update the summary panel in real time
- [ ] Resonance type sliders update the summary panel in real time
- [ ] Roll button triggers a result card
- [ ] Negligible result: no resonance type, no acute check shown
- [ ] Fleeting result: resonance type shown, no acute check
- [ ] Intense result: resonance type shown, acute die shown
- [ ] Acute result: Dyscrasia roll and pick buttons appear; selecting one shows the entry
- [ ] Export button creates a `.md` file in `~/Documents/vtmtools/`

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "chore: end-to-end smoke test verified"
```

---

## Verification Summary

| What | How |
|---|---|
| Dice logic correctness | `cargo test shared::dice` — 5 tests |
| Resonance sequence logic | `cargo test shared::resonance` — 5 tests |
| Dyscrasia CRUD | `cargo test db::dyscrasia` — 3 tests |
| MD export | `cargo test tools::export` — 1 test |
| Full UI flow | `cargo tauri dev` + manual smoke test checklist |
| Cross-platform build | Push a `v*` tag, CI builds Linux + Windows bundles |
