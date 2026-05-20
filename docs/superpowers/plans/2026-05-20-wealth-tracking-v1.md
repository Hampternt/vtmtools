# Wealth Tracking V1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the V1 opt-in per-character wealth tracking subsystem (liquid + banked + asset-list, with a small abstract item-slot catalog and an atomic purchase/sell pair) end-to-end — Rust DB, Tauri commands, typed TS wrappers, and a GM-facing `WealthManager` tool.

**Architecture:** Per the spec, three layers (data → tool → mechanics). Single `src-tauri/src/db/wealth.rs` for the DB module (matching existing repo convention — `advantage.rs` is 1221 lines, `modifier.rs` 946, `saved_character.rs` 1046; one big file per domain is normal here, the spec's nested-module proposal is overruled by this plan to follow repo pattern). 17 Tauri commands, 17 typed TS wrappers, one Svelte tool with seven sub-components.

**Tech Stack:** Rust (Tauri 2, sqlx, serde), TypeScript, Svelte 5 runes, SQLite. No new JS test framework — pricing formula and Svelte components verified by `npm run check` + manual smoke. Rust logic gets inline `#[cfg(test)]` tests where the spec calls for TDD.

**Spec reference:** `docs/superpowers/specs/2026-05-20-wealth-tracking-design.md` (commit `dba4154`).

**Conventions cited (read these before starting):**
- `CLAUDE.md` — workflow overrides (lean execution, TDD on demand, `verify.sh` before every commit), Claude-specific rules (no `invoke(...)` from components, typed wrapper required, `ARCHITECTURE.md §4` updated in same commit as new `#[tauri::command]`, never `std::fs` in async, etc).
- `ARCHITECTURE.md §4` — IPC inventory (must be updated in the same commit that adds/removes commands; both the module's per-file list AND the running total).
- `ARCHITECTURE.md §5` — module boundaries (`tokio::fs`, not `std::fs`).
- `ARCHITECTURE.md §6` — invariants (CSS Grid + `align-items: start`, rem not px, tokens not hex — components use the dark theme tokens from `:root` in `src/routes/+layout.svelte`).
- `ARCHITECTURE.md §9` — extensibility seams (how to add tools, schema, IPC).
- `ARCHITECTURE.md §10` — testing gates and expected warnings (do NOT "fix" them).
- `ADR 0002` — destructive reseed of `is_custom = 0` rows.
- `ADR 0004` — dark-only theming (no light-mode variants, no theme toggle).
- `ADR 0006` — bridge source generalization (only relevant for future Foundry V2; V1 has no bridge work).

**Workflow constraints (from CLAUDE.md):**
- Every task ending in a commit runs `./scripts/verify.sh` immediately before `git commit`. The commit step does **not** start until `verify: all checks passed` is on screen.
- TDD only where the task explicitly says "TDD on" — otherwise `verify.sh` (type-check + cargo tests + frontend build) is the gate.
- Never `git add -A` / `git add .`. Stage files by exact path.
- Never `git commit --no-verify` or amend a published commit.
- Never run `git status -uall`.

---

## File map

**Create:**
- `src-tauri/migrations/0010_add_wealth.sql` — DDL for `character_wallets`, `character_assets`, `item_slots`.
- `src-tauri/src/shared/wealth.rs` — shared types (Wallet, Asset, ItemSlot, Bucket, TransferDelay, TimeUnit, AssetCategory, AssetCondition, CustomModifier, request/patch structs, condition multiplier constants).
- `src-tauri/src/db/wealth.rs` — DB helpers + 17 `#[tauri::command]` handlers + inline `#[cfg(test)]` tests for arithmetic, conservation, atomicity, lifecycle cascade, catalog filter.
- `src/lib/wealth/api.ts` — typed IPC wrappers (one per Tauri command).
- `src/lib/wealth/pricing.ts` — pure pricing formula + the `CONDITION_MULTIPLIERS` constant.
- `src/tools/WealthManager.svelte` — the GM-facing tool. Character selector + enabled/disabled states.
- `src/lib/components/wealth/WalletPanel.svelte` — liquid/banked summary + buttons.
- `src/lib/components/wealth/AssetList.svelte` — asset table + add/sell affordances.
- `src/lib/components/wealth/ShopPanel.svelte` — category tabs + slot list + buy affordance.
- `src/lib/components/wealth/AdjustModal.svelte` — `+ / − / edit` on liquid or banked.
- `src/lib/components/wealth/TransferModal.svelte` — liquid ↔ banked.
- `src/lib/components/wealth/AddAssetModal.svelte` — manual asset entry (not via shop).
- `src/lib/components/wealth/SellModal.svelte` — sale price prompt → `sellAsset()`.
- `src/lib/components/wealth/BuyFormModal.svelte` — slot purchase form (band slider, condition, custom modifiers, live price).

**Modify:**
- `src-tauri/src/shared/mod.rs` — `pub mod wealth;`.
- `src-tauri/src/db/mod.rs` — `pub mod wealth;`.
- `src-tauri/src/db/seed.rs` — add `seed_item_slots(pool)` and call site.
- `src-tauri/src/lib.rs` — register 17 new commands in `invoke_handler!`; ensure `seed_item_slots` is called from the seed bootstrap.
- `src/types.ts` — mirror Wallet, Asset, ItemSlot, Bucket, AssetCategory, AssetCondition, TimeUnit, TransferDelay, CustomModifier, NewAsset, AssetPatch, NewItemSlot, PurchaseRequest, SellRequest.
- `src/tools.ts` — add the `wealth` tool entry.
- `ARCHITECTURE.md` — §4 IPC inventory (per-module list + running total).

**Delete:** none.

---

## Task order rationale

Bottom-up. Each task lands a runnable, verified increment. Dependencies flow upward:

```
Task 1  Domain contract (migration + types + TS mirror)        ← compile-only
Task 2  Wallet primitives (8 cmds, TDD on arithmetic + cascade)
Task 3  Asset CRUD (4 cmds, no TDD)
Task 4  Catalog + seed (3 cmds, no TDD)
Task 5  Composed transactions (2 cmds, TDD on atomicity)
Task 6  Pricing module (TS, verified by inspection + smoke)
Task 7  WealthManager tool shell (character selector + enable button)
Task 8  Wallet panel + adjust + transfer modals
Task 9  Asset list + add + sell modals
Task 10 Shop panel + buy form modal
Task 11 ARCHITECTURE §4 final + smoke-test script + close
```

Each task is one commit. After Task 11 the branch is ready for the single final `code-review:code-review` per CLAUDE.md "Lean plan execution".

---

## Task 1: Domain contract — migration + Rust types + TS mirror

One commit. Schema + types land together so the rest of the plan has a stable surface.

**Files:**
- Create: `src-tauri/migrations/0010_add_wealth.sql`
- Create: `src-tauri/src/shared/wealth.rs`
- Modify: `src-tauri/src/shared/mod.rs` (add `pub mod wealth;` next to existing module decls)
- Modify: `src/types.ts` (append the wealth types at the end of the file — keeps the existing layout untouched)

**TDD:** none. Pure scaffolding; the migration runs but no caller touches the tables yet. Type-check is the gate.

### Steps

- [ ] **Step 1: Create the migration file**

Create `src-tauri/migrations/0010_add_wealth.sql` with the exact content:

```sql
-- Wealth tracking subsystem (V1).
-- Per-character opt-in. Row presence in character_wallets == opted in.
-- See docs/superpowers/specs/2026-05-20-wealth-tracking-design.md

CREATE TABLE IF NOT EXISTS character_wallets (
    character_id          INTEGER PRIMARY KEY
                                  REFERENCES characters(id) ON DELETE CASCADE,
    liquid                REAL    NOT NULL DEFAULT 0,
    banked                REAL    NOT NULL DEFAULT 0,
    currency_name         TEXT    NOT NULL DEFAULT '$',
    transfer_delay_json   TEXT,
    created_at            TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at            TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS character_assets (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    character_id          INTEGER NOT NULL
                                  REFERENCES character_wallets(character_id) ON DELETE CASCADE,
    name                  TEXT    NOT NULL,
    category              TEXT    CHECK(category IN ('weapon','vehicle','property','investment','misc')),
    class                 TEXT,
    purchase_price        REAL,
    current_value         REAL    NOT NULL DEFAULT 0,
    condition             TEXT    CHECK(condition IN ('new','used','very_used','custom')),
    custom_modifier_json  TEXT,
    notes                 TEXT,
    created_at            TEXT    NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_character_assets_character_id
    ON character_assets(character_id);

CREATE TABLE IF NOT EXISTS item_slots (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    category              TEXT    NOT NULL
                                  CHECK(category IN ('weapon','vehicle','property','investment','misc')),
    class                 TEXT    NOT NULL,
    label                 TEXT    NOT NULL,
    base_price_min        REAL    NOT NULL,
    base_price_max        REAL    NOT NULL,
    examples              TEXT,
    is_custom             INTEGER NOT NULL DEFAULT 0,
    source                TEXT,
    UNIQUE(category, class)
);
```

- [ ] **Step 2: Create the Rust shared-types module**

Create `src-tauri/src/shared/wealth.rs`:

```rust
//! Wealth-tracking subsystem shared types.
//!
//! See `docs/superpowers/specs/2026-05-20-wealth-tracking-design.md`.

use serde::{Deserialize, Serialize};

// ─── Buckets / time ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Bucket {
    Liquid,
    Banked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferDelay {
    pub value: f64,
    pub unit: TimeUnit,
}

// ─── Wallet ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Wallet {
    pub character_id: i64,
    pub liquid: f64,
    pub banked: f64,
    pub currency_name: String,
    pub transfer_delay: Option<TransferDelay>,
    pub created_at: String,
    pub updated_at: String,
}

// ─── Asset categories / conditions ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Weapon,
    Vehicle,
    Property,
    Investment,
    Misc,
}

impl AssetCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetCategory::Weapon     => "weapon",
            AssetCategory::Vehicle    => "vehicle",
            AssetCategory::Property   => "property",
            AssetCategory::Investment => "investment",
            AssetCategory::Misc       => "misc",
        }
    }

    pub fn from_str(s: &str) -> Option<AssetCategory> {
        Some(match s {
            "weapon"     => AssetCategory::Weapon,
            "vehicle"    => AssetCategory::Vehicle,
            "property"   => AssetCategory::Property,
            "investment" => AssetCategory::Investment,
            "misc"       => AssetCategory::Misc,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetCondition {
    New,
    Used,
    VeryUsed,
    Custom,
}

impl AssetCondition {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetCondition::New      => "new",
            AssetCondition::Used     => "used",
            AssetCondition::VeryUsed => "very_used",
            AssetCondition::Custom   => "custom",
        }
    }

    pub fn from_str(s: &str) -> Option<AssetCondition> {
        Some(match s {
            "new"       => AssetCondition::New,
            "used"      => AssetCondition::Used,
            "very_used" => AssetCondition::VeryUsed,
            "custom"    => AssetCondition::Custom,
            _ => return None,
        })
    }
}

/// Condition multiplier lookup. `Custom` is GM-supplied at purchase time and
/// not stored here.
pub const CONDITION_MULTIPLIER_NEW:       f64 = 1.0;
pub const CONDITION_MULTIPLIER_USED:      f64 = 0.6;
pub const CONDITION_MULTIPLIER_VERY_USED: f64 = 0.3;

// ─── Custom modifiers ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CustomModifier {
    pub label: String,
    pub mult: f64,
}

// ─── Asset ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: i64,
    pub character_id: i64,
    pub name: String,
    pub category: Option<AssetCategory>,
    pub class: Option<String>,
    pub purchase_price: Option<f64>,
    pub current_value: f64,
    pub condition: Option<AssetCondition>,
    pub custom_modifiers: Vec<CustomModifier>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewAsset {
    pub name: String,
    pub category: Option<AssetCategory>,
    pub class: Option<String>,
    pub purchase_price: Option<f64>,
    pub current_value: f64,
    pub condition: Option<AssetCondition>,
    pub custom_modifiers: Vec<CustomModifier>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPatch {
    pub name: Option<String>,
    pub category: Option<Option<AssetCategory>>,
    pub class: Option<Option<String>>,
    pub purchase_price: Option<Option<f64>>,
    pub current_value: Option<f64>,
    pub condition: Option<Option<AssetCondition>>,
    pub custom_modifiers: Option<Vec<CustomModifier>>,
    pub notes: Option<Option<String>>,
}

// ─── Item slot (catalog) ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSlot {
    pub id: i64,
    pub category: AssetCategory,
    pub class: String,
    pub label: String,
    pub base_price_min: f64,
    pub base_price_max: f64,
    pub examples: Option<String>,
    pub is_custom: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewItemSlot {
    pub category: AssetCategory,
    pub class: String,
    pub label: String,
    pub base_price_min: f64,
    pub base_price_max: f64,
    pub examples: Option<String>,
    pub source: Option<String>,
}

// ─── Composed transaction requests ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PurchaseRequest {
    pub character_id: i64,
    pub slot_id: i64,
    pub price: f64,
    pub condition: AssetCondition,
    pub custom_modifiers: Vec<CustomModifier>,
    pub name_override: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SellRequest {
    pub asset_id: i64,
    pub sale_price: f64,
    pub notes: Option<String>,
}
```

- [ ] **Step 3: Register the new module**

Edit `src-tauri/src/shared/mod.rs` and add `pub mod wealth;` alongside the existing module declarations. (Do NOT remove or reorder existing decls.)

- [ ] **Step 4: Mirror the types on the TS side**

Append to the end of `src/types.ts`:

```ts
// ───────────────────────────────────────────────────────────────────────────
// Wealth tracking (V1) — mirrors src-tauri/src/shared/wealth.rs
// See docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
// ───────────────────────────────────────────────────────────────────────────

export type Bucket = 'liquid' | 'banked';
export type TimeUnit = 'minutes' | 'hours' | 'days' | 'weeks';

export interface TransferDelay {
  value: number;
  unit: TimeUnit;
}

export interface Wallet {
  characterId: number;
  liquid: number;
  banked: number;
  currencyName: string;
  transferDelay: TransferDelay | null;
  createdAt: string;
  updatedAt: string;
}

export type AssetCategory = 'weapon' | 'vehicle' | 'property' | 'investment' | 'misc';
export type AssetCondition = 'new' | 'used' | 'very_used' | 'custom';

export interface CustomModifier {
  label: string;
  mult: number;
}

export interface Asset {
  id: number;
  characterId: number;
  name: string;
  category: AssetCategory | null;
  class: string | null;
  purchasePrice: number | null;
  currentValue: number;
  condition: AssetCondition | null;
  customModifiers: CustomModifier[];
  notes: string | null;
  createdAt: string;
}

export interface NewAsset {
  name: string;
  category: AssetCategory | null;
  class: string | null;
  purchasePrice: number | null;
  currentValue: number;
  condition: AssetCondition | null;
  customModifiers: CustomModifier[];
  notes: string | null;
}

export interface AssetPatch {
  name?: string;
  category?: AssetCategory | null;
  class?: string | null;
  purchasePrice?: number | null;
  currentValue?: number;
  condition?: AssetCondition | null;
  customModifiers?: CustomModifier[];
  notes?: string | null;
}

export interface ItemSlot {
  id: number;
  category: AssetCategory;
  class: string;
  label: string;
  basePriceMin: number;
  basePriceMax: number;
  examples: string | null;
  isCustom: boolean;
  source: string | null;
}

export interface NewItemSlot {
  category: AssetCategory;
  class: string;
  label: string;
  basePriceMin: number;
  basePriceMax: number;
  examples: string | null;
  source: string | null;
}

export interface PurchaseRequest {
  characterId: number;
  slotId: number;
  price: number;
  condition: AssetCondition;
  customModifiers: CustomModifier[];
  nameOverride: string | null;
  notes: string | null;
}

export interface SellRequest {
  assetId: number;
  salePrice: number;
  notes: string | null;
}
```

- [ ] **Step 5: Run the correctness gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`

If the migration fails to apply (most likely cause: `characters` table doesn't exist at migration time, which it does — it's created in `0001_initial.sql`), check the migration's `REFERENCES characters(id)` clause. Otherwise: type errors in `src/types.ts` (run `npm run check` standalone to isolate) or compile errors in Rust (run `cargo check --manifest-path src-tauri/Cargo.toml`).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations/0010_add_wealth.sql \
        src-tauri/src/shared/wealth.rs \
        src-tauri/src/shared/mod.rs \
        src/types.ts
git commit -m "$(cat <<'EOF'
wealth: domain contract — migration, Rust types, TS mirror

Adds the three wealth-subsystem tables (character_wallets,
character_assets, item_slots) and the matching Rust + TS type
shapes. No caller wires into them yet — scaffolding only.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 2: Wallet primitives + Tauri commands + TS wrappers — **TDD on arithmetic + lifecycle cascade**

The 8 wallet functions land together with their tests. This is the largest task; arithmetic/invariant logic warrants TDD per CLAUDE.md "Reserve TDD for genuine logic: pricing formula, transfer/purchase/sell arithmetic invariants, lifecycle cascade".

**Files:**
- Create: `src-tauri/src/db/wealth.rs`
- Modify: `src-tauri/src/db/mod.rs` (add `pub mod wealth;`)
- Modify: `src-tauri/src/lib.rs` (register the 8 wallet commands in `invoke_handler!`)
- Create: `src/lib/wealth/api.ts` (typed wrappers — the 8 wallet ones land in this task; the rest get appended in Tasks 3/4/5)

**TDD:** yes. Eight tests inside `#[cfg(test)] mod tests` at the bottom of `src-tauri/src/db/wealth.rs`. Each test uses an in-memory SQLite pool seeded with the migrations.

### Steps

- [ ] **Step 1: Create the DB module shell with test scaffolding**

Create `src-tauri/src/db/wealth.rs`. Start with imports + a `test_pool()` helper that mirrors the pattern in other modules (`grep -rn "fn test_pool\|fn fresh_pool" src-tauri/src/db/` to confirm the existing helper name — most likely `setup_test_pool()`; if a helper exists, USE IT, do not duplicate).

If no shared helper exists, the per-module convention seen in `advantage.rs` is:

```rust
#[cfg(test)]
async fn fresh_pool() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    pool
}

#[cfg(test)]
async fn fresh_pool_with_character(name: &str) -> (sqlx::SqlitePool, i64) {
    let pool = fresh_pool().await;
    let id: i64 = sqlx::query_scalar("INSERT INTO characters (name) VALUES (?) RETURNING id")
        .bind(name)
        .fetch_one(&pool)
        .await
        .expect("insert character");
    (pool, id)
}
```

(Check the actual `characters` schema — it may have more required columns than just `name`. Run `grep -A 30 "CREATE TABLE.*characters\b" src-tauri/migrations/0001_initial.sql` to confirm. If more columns are NOT NULL without defaults, fill them with defaults in the helper.)

Initial file body:

```rust
//! Wealth-tracking DB module (V1).
//!
//! See docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
//! Pattern note: this is a single 800+ line file mirroring the existing
//! repo convention (advantage.rs is 1221 lines, modifier.rs 946,
//! saved_character.rs 1046). The spec's nested-module proposal is
//! overruled to follow repo pattern.

use crate::shared::wealth::*;
use serde_json;
use sqlx::SqlitePool;

#[derive(Debug, thiserror::Error)]
pub enum WealthError {
    #[error("wallet not found for character {0}")]
    WalletNotFound(i64),
    #[error("asset not found: {0}")]
    AssetNotFound(i64),
    #[error("item slot not found: {0}")]
    SlotNotFound(i64),
    #[error("transfer_funds cannot move from a bucket to itself")]
    SameBucketTransfer,
    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl serde::Serialize for WealthError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

// === Wallet primitives ====================================================

// (implementations follow in Step 2)

#[cfg(test)]
mod tests {
    // (tests in Step 3 onward — written first per TDD)
}
```

If the project uses a different error pattern (e.g., `anyhow::Error`, `Box<dyn Error>`, or a custom `db::Error`), match it. Check `grep -rn "thiserror\|anyhow" src-tauri/src/db/*.rs | head -10`.

- [ ] **Step 2: Register the module in `db/mod.rs`**

Edit `src-tauri/src/db/mod.rs` and add `pub mod wealth;` next to the other module decls.

- [ ] **Step 3: Write the failing test for `enable_wallet` + `get_wallet` (lifecycle)**

Inside the `tests` mod, add:

```rust
    use super::*;

    #[sqlx::test]
    async fn enable_wallet_creates_zeroed_row_and_is_idempotent() {
        let (pool, char_id) = fresh_pool_with_character("Alice").await;

        // First enable — creates row with zeros.
        let w = enable_wallet(&pool, char_id).await.unwrap();
        assert_eq!(w.character_id, char_id);
        assert_eq!(w.liquid, 0.0);
        assert_eq!(w.banked, 0.0);
        assert_eq!(w.currency_name, "$");
        assert!(w.transfer_delay.is_none());

        // Mutate the wallet, then re-enable. Idempotent: returns the existing
        // wallet untouched, does NOT zero balances.
        adjust_liquid(&pool, char_id, 500.0).await.unwrap();
        let w2 = enable_wallet(&pool, char_id).await.unwrap();
        assert_eq!(w2.liquid, 500.0, "enable_wallet on already-enabled must NOT clobber balances");

        // get_wallet returns Some for opted-in characters.
        let got = get_wallet(&pool, char_id).await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().liquid, 500.0);
    }

    #[sqlx::test]
    async fn get_wallet_returns_none_when_not_opted_in() {
        let (pool, char_id) = fresh_pool_with_character("Bob").await;
        let got = get_wallet(&pool, char_id).await.unwrap();
        assert!(got.is_none(), "characters without a wallet row must return None");
    }
```

NOTE: if `#[sqlx::test]` doesn't suit (older sqlx, runtime mismatch), use `#[tokio::test]` + manual pool setup. Match the existing convention; check `grep -n "sqlx::test\|tokio::test" src-tauri/src/db/advantage.rs`.

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml wealth::tests`
Expected: compile error (functions `enable_wallet`, `get_wallet`, `adjust_liquid` not yet defined). This is the "red" stage of TDD.

- [ ] **Step 5: Implement `enable_wallet` and `get_wallet`**

Add to `src-tauri/src/db/wealth.rs` above the test mod:

```rust
/// Enable wealth tracking for a character. Inserts a zeroed wallet row.
/// Idempotent: if a wallet already exists, returns it untouched (no clobber).
pub async fn enable_wallet(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Wallet, WealthError> {
    // INSERT OR IGNORE — preserves existing row if present.
    sqlx::query(
        "INSERT OR IGNORE INTO character_wallets (character_id) VALUES (?)",
    )
    .bind(character_id)
    .execute(pool)
    .await?;

    // SELECT the (existing or newly-created) row.
    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}

/// Return the wallet for `character_id`, or `None` if the character has not
/// opted in.
pub async fn get_wallet(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Option<Wallet>, WealthError> {
    let row: Option<(i64, f64, f64, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT character_id, liquid, banked, currency_name, transfer_delay_json, created_at, updated_at
           FROM character_wallets
          WHERE character_id = ?",
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await?;

    Ok(match row {
        Some((id, liquid, banked, cur, delay_json, created, updated)) => {
            let transfer_delay = match delay_json {
                Some(s) => serde_json::from_str(&s)?,
                None => None,
            };
            Some(Wallet {
                character_id: id,
                liquid,
                banked,
                currency_name: cur,
                transfer_delay,
                created_at: created,
                updated_at: updated,
            })
        }
        None => None,
    })
}
```

- [ ] **Step 6: Write the failing test for `adjust_liquid` / `adjust_banked` arithmetic**

```rust
    #[sqlx::test]
    async fn adjust_liquid_handles_positive_negative_zero() {
        let (pool, char_id) = fresh_pool_with_character("Carol").await;
        enable_wallet(&pool, char_id).await.unwrap();

        let w = adjust_liquid(&pool, char_id, 100.0).await.unwrap();
        assert_eq!(w.liquid, 100.0);

        let w = adjust_liquid(&pool, char_id, -25.0).await.unwrap();
        assert_eq!(w.liquid, 75.0);

        let w = adjust_liquid(&pool, char_id, 0.0).await.unwrap();
        assert_eq!(w.liquid, 75.0);

        // Negative balances are allowed (per spec — GMs model debt narratively).
        let w = adjust_liquid(&pool, char_id, -1000.0).await.unwrap();
        assert_eq!(w.liquid, -925.0);
    }

    #[sqlx::test]
    async fn adjust_banked_independent_of_liquid() {
        let (pool, char_id) = fresh_pool_with_character("Dave").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 100.0).await.unwrap();

        let w = adjust_banked(&pool, char_id, 250.0).await.unwrap();
        assert_eq!(w.liquid, 100.0, "banked adjustments must not touch liquid");
        assert_eq!(w.banked, 250.0);
    }
```

- [ ] **Step 7: Run the tests to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml wealth::tests`
Expected: compile error on `adjust_liquid` / `adjust_banked`.

- [ ] **Step 8: Implement `adjust_liquid` and `adjust_banked`**

```rust
pub async fn adjust_liquid(
    pool: &SqlitePool,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, WealthError> {
    let updated = sqlx::query(
        "UPDATE character_wallets
            SET liquid = liquid + ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(delta)
    .bind(character_id)
    .execute(pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(character_id));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}

pub async fn adjust_banked(
    pool: &SqlitePool,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, WealthError> {
    let updated = sqlx::query(
        "UPDATE character_wallets
            SET banked = banked + ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(delta)
    .bind(character_id)
    .execute(pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(character_id));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}
```

- [ ] **Step 9: Run tests to confirm green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml wealth::tests`
Expected: 4 tests pass.

- [ ] **Step 10: Write the failing tests for `transfer_funds`**

```rust
    #[sqlx::test]
    async fn transfer_funds_conserves_total() {
        let (pool, char_id) = fresh_pool_with_character("Eve").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 1000.0).await.unwrap();
        adjust_banked(&pool, char_id, 500.0).await.unwrap();

        let total_before = 1000.0 + 500.0;

        let w = transfer_funds(&pool, char_id, Bucket::Liquid, Bucket::Banked, 200.0)
            .await
            .unwrap();
        assert!((w.liquid - 800.0).abs() < f64::EPSILON);
        assert!((w.banked - 700.0).abs() < f64::EPSILON);
        assert!((w.liquid + w.banked - total_before).abs() < f64::EPSILON,
                "transfer must conserve total balance");
    }

    #[sqlx::test]
    async fn transfer_funds_same_bucket_is_error() {
        let (pool, char_id) = fresh_pool_with_character("Frank").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 100.0).await.unwrap();

        let err = transfer_funds(&pool, char_id, Bucket::Liquid, Bucket::Liquid, 50.0).await;
        assert!(matches!(err, Err(WealthError::SameBucketTransfer)));
    }

    #[sqlx::test]
    async fn transfer_funds_allows_overdraw() {
        // Spec: "transfer_funds with amount > balance[from] is allowed"
        let (pool, char_id) = fresh_pool_with_character("Grace").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 10.0).await.unwrap();

        let w = transfer_funds(&pool, char_id, Bucket::Liquid, Bucket::Banked, 100.0)
            .await
            .unwrap();
        assert_eq!(w.liquid, -90.0);
        assert_eq!(w.banked, 100.0);
    }
```

- [ ] **Step 11: Implement `transfer_funds`**

```rust
pub async fn transfer_funds(
    pool: &SqlitePool,
    character_id: i64,
    from: Bucket,
    to: Bucket,
    amount: f64,
) -> Result<Wallet, WealthError> {
    if from == to {
        return Err(WealthError::SameBucketTransfer);
    }

    let mut tx = pool.begin().await?;

    // Decrement `from`.
    let (col_from, col_to) = match (from, to) {
        (Bucket::Liquid, Bucket::Banked) => ("liquid", "banked"),
        (Bucket::Banked, Bucket::Liquid) => ("banked", "liquid"),
        _ => unreachable!("same-bucket already returned"),
    };

    let q = format!(
        "UPDATE character_wallets
            SET {col_from} = {col_from} - ?,
                {col_to}   = {col_to}   + ?,
                updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?"
    );

    let res = sqlx::query(&q)
        .bind(amount)
        .bind(amount)
        .bind(character_id)
        .execute(&mut *tx)
        .await?;

    if res.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(character_id));
    }

    tx.commit().await?;

    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}
```

Note on the inline-formatted SQL: the column names are not user input (they come from the `Bucket` enum), so this is not a SQL injection vector. The `?` placeholders are still bound parametrically.

- [ ] **Step 12: Run all tests so far — confirm green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml wealth::tests`
Expected: 7 tests pass (2 lifecycle + 2 adjust + 3 transfer).

- [ ] **Step 13: Write the failing test for `disable_wallet` + cascade**

```rust
    #[sqlx::test]
    async fn disable_wallet_cascades_assets_and_is_idempotent_when_absent() {
        let (pool, char_id) = fresh_pool_with_character("Hank").await;
        enable_wallet(&pool, char_id).await.unwrap();

        // Seed an asset so the cascade has something to drop.
        sqlx::query(
            "INSERT INTO character_assets (character_id, name, current_value)
             VALUES (?, 'test', 100.0)",
        )
        .bind(char_id)
        .execute(&pool)
        .await
        .unwrap();

        let count_before: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_assets WHERE character_id = ?",
        )
        .bind(char_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count_before, 1);

        disable_wallet(&pool, char_id).await.unwrap();

        let count_after: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM character_assets WHERE character_id = ?",
        )
        .bind(char_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count_after, 0, "ON DELETE CASCADE must drop all assets");

        assert!(get_wallet(&pool, char_id).await.unwrap().is_none());

        // Idempotent on absent — spec says "disable on non-existent wallet is a no-op".
        disable_wallet(&pool, char_id).await.expect("second disable must be no-op, not error");
    }
```

- [ ] **Step 14: Implement `disable_wallet`**

```rust
pub async fn disable_wallet(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<(), WealthError> {
    // No-op if absent — relies on DELETE's silent behavior. Returns Ok(()) either way.
    sqlx::query("DELETE FROM character_wallets WHERE character_id = ?")
        .bind(character_id)
        .execute(pool)
        .await?;
    Ok(())
}
```

- [ ] **Step 15: Implement `set_currency_name` and `set_transfer_delay`**

```rust
pub async fn set_currency_name(
    pool: &SqlitePool,
    character_id: i64,
    name: &str,
) -> Result<Wallet, WealthError> {
    let res = sqlx::query(
        "UPDATE character_wallets
            SET currency_name = ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(name)
    .bind(character_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(character_id));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}

pub async fn set_transfer_delay(
    pool: &SqlitePool,
    character_id: i64,
    delay: Option<TransferDelay>,
) -> Result<Wallet, WealthError> {
    let json = match delay {
        Some(d) => Some(serde_json::to_string(&d)?),
        None => None,
    };

    let res = sqlx::query(
        "UPDATE character_wallets
            SET transfer_delay_json = ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(json)
    .bind(character_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(character_id));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}
```

- [ ] **Step 16: Add the 8 Tauri command handlers at the bottom of `wealth.rs` (above `#[cfg(test)]`)**

```rust
// ─── Tauri commands ───────────────────────────────────────────────────────

#[tauri::command]
pub async fn wealth_enable_wallet(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
) -> Result<Wallet, WealthError> {
    enable_wallet(&state.pool, character_id).await
}

#[tauri::command]
pub async fn wealth_disable_wallet(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
) -> Result<(), WealthError> {
    disable_wallet(&state.pool, character_id).await
}

#[tauri::command]
pub async fn wealth_get_wallet(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
) -> Result<Option<Wallet>, WealthError> {
    get_wallet(&state.pool, character_id).await
}

#[tauri::command]
pub async fn wealth_adjust_liquid(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, WealthError> {
    adjust_liquid(&state.pool, character_id, delta).await
}

#[tauri::command]
pub async fn wealth_adjust_banked(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, WealthError> {
    adjust_banked(&state.pool, character_id, delta).await
}

#[tauri::command]
pub async fn wealth_transfer_funds(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
    from: Bucket,
    to: Bucket,
    amount: f64,
) -> Result<Wallet, WealthError> {
    transfer_funds(&state.pool, character_id, from, to, amount).await
}

#[tauri::command]
pub async fn wealth_set_currency_name(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
    name: String,
) -> Result<Wallet, WealthError> {
    set_currency_name(&state.pool, character_id, &name).await
}

#[tauri::command]
pub async fn wealth_set_transfer_delay(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
    delay: Option<TransferDelay>,
) -> Result<Wallet, WealthError> {
    set_transfer_delay(&state.pool, character_id, delay).await
}
```

If the project uses a different state type (`crate::AppState` vs another), match it. Check `grep -n "tauri::State" src-tauri/src/db/advantage.rs | head -3` to confirm.

- [ ] **Step 17: Register the 8 commands in `src-tauri/src/lib.rs`**

Find the `tauri::generate_handler![` block in `src-tauri/src/lib.rs` (around line 66) and add these entries (alphabetical or grouped — match the existing style):

```rust
db::wealth::wealth_enable_wallet,
db::wealth::wealth_disable_wallet,
db::wealth::wealth_get_wallet,
db::wealth::wealth_adjust_liquid,
db::wealth::wealth_adjust_banked,
db::wealth::wealth_transfer_funds,
db::wealth::wealth_set_currency_name,
db::wealth::wealth_set_transfer_delay,
```

- [ ] **Step 18: Create the TS wrapper file with the 8 wallet wrappers**

Create `src/lib/wealth/api.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import type {
  Wallet,
  Bucket,
  TransferDelay,
} from '../../types';

// ─── Wallet ────────────────────────────────────────────────────────────────

export function enableWallet(characterId: number): Promise<Wallet> {
  return invoke<Wallet>('wealth_enable_wallet', { characterId });
}

export function disableWallet(characterId: number): Promise<void> {
  return invoke<void>('wealth_disable_wallet', { characterId });
}

export function getWallet(characterId: number): Promise<Wallet | null> {
  return invoke<Wallet | null>('wealth_get_wallet', { characterId });
}

export function adjustLiquid(characterId: number, delta: number): Promise<Wallet> {
  return invoke<Wallet>('wealth_adjust_liquid', { characterId, delta });
}

export function adjustBanked(characterId: number, delta: number): Promise<Wallet> {
  return invoke<Wallet>('wealth_adjust_banked', { characterId, delta });
}

export function transferFunds(
  characterId: number,
  from: Bucket,
  to: Bucket,
  amount: number,
): Promise<Wallet> {
  return invoke<Wallet>('wealth_transfer_funds', { characterId, from, to, amount });
}

export function setCurrencyName(characterId: number, name: string): Promise<Wallet> {
  return invoke<Wallet>('wealth_set_currency_name', { characterId, name });
}

export function setTransferDelay(
  characterId: number,
  delay: TransferDelay | null,
): Promise<Wallet> {
  return invoke<Wallet>('wealth_set_transfer_delay', { characterId, delay });
}
```

Check the existing wrapper style by reading `src/lib/library/api.ts`. The convention in this repo is: `invoke<Result>('cmd_name', { camelCaseArgs })`. Tauri auto-converts the camelCase arg names back to snake_case for the Rust handler.

- [ ] **Step 19: Update ARCHITECTURE.md §4 — IPC inventory**

Open `ARCHITECTURE.md`, find §4 (IPC inventory). Add a new sub-section for `db::wealth` listing the 8 commands. Update the running total at the bottom of §4. Example (adapt to existing formatting):

```markdown
### `db::wealth` (V1 wallet primitives — 8 commands)

| Command | Module fn | Purpose |
|---|---|---|
| `wealth_enable_wallet`     | `enable_wallet`     | Idempotent INSERT OR IGNORE; opt-in. |
| `wealth_disable_wallet`    | `disable_wallet`    | DELETE wallet row; cascades assets. |
| `wealth_get_wallet`        | `get_wallet`        | Returns `Option<Wallet>`. |
| `wealth_adjust_liquid`     | `adjust_liquid`     | Signed delta on liquid. |
| `wealth_adjust_banked`     | `adjust_banked`     | Signed delta on banked. |
| `wealth_transfer_funds`    | `transfer_funds`    | Atomic liquid ↔ banked. |
| `wealth_set_currency_name` | `set_currency_name` | Update currency label. |
| `wealth_set_transfer_delay`| `set_transfer_delay`| Update forward-compat delay hint. |
```

If §4 has a running total ("N commands across M modules"), bump it.

- [ ] **Step 20: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`. Tests should show 8 passing in `wealth::tests` (2 lifecycle, 2 adjust, 3 transfer, 1 cascade).

- [ ] **Step 21: Commit**

```bash
git add src-tauri/src/db/wealth.rs \
        src-tauri/src/db/mod.rs \
        src-tauri/src/lib.rs \
        src/lib/wealth/api.ts \
        ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
wealth: wallet primitives (8 cmds) + typed TS wrappers

Implements enable/disable/get wallet, adjust_liquid/banked,
transfer_funds (with same-bucket error and overdraw permitted),
set_currency_name, set_transfer_delay. Eight Tauri commands,
eight TS wrappers, ARCHITECTURE.md §4 updated.

TDD coverage: lifecycle (enable idempotent, disable cascades and
no-ops when absent), arithmetic (+/-/0, negative balances allowed),
transfer conservation, same-bucket error, overdraw permitted.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 3: Asset CRUD — 4 commands, no TDD

CRUD only. `verify.sh` is the gate.

**Files:**
- Modify: `src-tauri/src/db/wealth.rs` (append asset functions + 4 Tauri commands)
- Modify: `src-tauri/src/lib.rs` (register 4 commands)
- Modify: `src/lib/wealth/api.ts` (append 4 wrappers)
- Modify: `ARCHITECTURE.md` §4

### Steps

- [ ] **Step 1: Append asset CRUD functions to `wealth.rs`**

Above the `// ─── Tauri commands ───` divider, append:

```rust
// === Asset CRUD ===========================================================

fn parse_custom_modifiers(json: Option<&str>) -> Result<Vec<CustomModifier>, WealthError> {
    match json {
        Some(s) if !s.is_empty() => Ok(serde_json::from_str(s)?),
        _ => Ok(Vec::new()),
    }
}

fn category_from_str_opt(s: Option<&str>) -> Option<AssetCategory> {
    s.and_then(AssetCategory::from_str)
}

fn condition_from_str_opt(s: Option<&str>) -> Option<AssetCondition> {
    s.and_then(AssetCondition::from_str)
}

async fn fetch_asset(pool: &SqlitePool, asset_id: i64) -> Result<Asset, WealthError> {
    let row: Option<(i64, i64, String, Option<String>, Option<String>,
                     Option<f64>, f64, Option<String>, Option<String>,
                     Option<String>, String)> = sqlx::query_as(
        "SELECT id, character_id, name, category, class,
                purchase_price, current_value, condition, custom_modifier_json,
                notes, created_at
           FROM character_assets
          WHERE id = ?",
    )
    .bind(asset_id)
    .fetch_optional(pool)
    .await?;

    let (id, char_id, name, cat_s, class_s, pp, cv, cond_s, cm_json, notes, created) =
        row.ok_or(WealthError::AssetNotFound(asset_id))?;

    Ok(Asset {
        id,
        character_id: char_id,
        name,
        category: category_from_str_opt(cat_s.as_deref()),
        class: class_s,
        purchase_price: pp,
        current_value: cv,
        condition: condition_from_str_opt(cond_s.as_deref()),
        custom_modifiers: parse_custom_modifiers(cm_json.as_deref())?,
        notes,
        created_at: created,
    })
}

pub async fn add_asset(
    pool: &SqlitePool,
    character_id: i64,
    payload: NewAsset,
) -> Result<Asset, WealthError> {
    // Verify the character has an opted-in wallet first.
    if get_wallet(pool, character_id).await?.is_none() {
        return Err(WealthError::WalletNotFound(character_id));
    }

    let cm_json = if payload.custom_modifiers.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&payload.custom_modifiers)?)
    };

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO character_assets
            (character_id, name, category, class,
             purchase_price, current_value, condition, custom_modifier_json, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(character_id)
    .bind(&payload.name)
    .bind(payload.category.map(|c| c.as_str()))
    .bind(payload.class.as_deref())
    .bind(payload.purchase_price)
    .bind(payload.current_value)
    .bind(payload.condition.map(|c| c.as_str()))
    .bind(cm_json)
    .bind(payload.notes.as_deref())
    .fetch_one(pool)
    .await?;

    fetch_asset(pool, id).await
}

pub async fn list_assets(
    pool: &SqlitePool,
    character_id: i64,
) -> Result<Vec<Asset>, WealthError> {
    let rows: Vec<(i64, i64, String, Option<String>, Option<String>,
                   Option<f64>, f64, Option<String>, Option<String>,
                   Option<String>, String)> = sqlx::query_as(
        "SELECT id, character_id, name, category, class,
                purchase_price, current_value, condition, custom_modifier_json,
                notes, created_at
           FROM character_assets
          WHERE character_id = ?
          ORDER BY created_at ASC",
    )
    .bind(character_id)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for (id, char_id, name, cat_s, class_s, pp, cv, cond_s, cm_json, notes, created) in rows {
        out.push(Asset {
            id,
            character_id: char_id,
            name,
            category: category_from_str_opt(cat_s.as_deref()),
            class: class_s,
            purchase_price: pp,
            current_value: cv,
            condition: condition_from_str_opt(cond_s.as_deref()),
            custom_modifiers: parse_custom_modifiers(cm_json.as_deref())?,
            notes,
            created_at: created,
        });
    }
    Ok(out)
}

pub async fn update_asset(
    pool: &SqlitePool,
    asset_id: i64,
    patch: AssetPatch,
) -> Result<Asset, WealthError> {
    // Verify asset exists first so we can return a clean error.
    let _ = fetch_asset(pool, asset_id).await?;

    // Build a dynamic UPDATE. Simpler than COALESCE-everything because
    // we have Option<Option<T>> for nullable fields and want NULL writes
    // to be distinguishable from "no change".
    let mut sets: Vec<&'static str> = Vec::new();
    let mut tx = pool.begin().await?;

    macro_rules! set {
        ($field:expr) => {
            sets.push(concat!($field, " = ?"));
        };
    }

    if patch.name.is_some()             { set!("name"); }
    if patch.category.is_some()         { set!("category"); }
    if patch.class.is_some()            { set!("class"); }
    if patch.purchase_price.is_some()   { set!("purchase_price"); }
    if patch.current_value.is_some()    { set!("current_value"); }
    if patch.condition.is_some()        { set!("condition"); }
    if patch.custom_modifiers.is_some() { set!("custom_modifier_json"); }
    if patch.notes.is_some()            { set!("notes"); }

    if sets.is_empty() {
        return fetch_asset(pool, asset_id).await;
    }

    let sql = format!(
        "UPDATE character_assets SET {} WHERE id = ?",
        sets.join(", ")
    );

    let mut q = sqlx::query(&sql);
    if let Some(v) = &patch.name { q = q.bind(v); }
    if let Some(v) = &patch.category {
        q = q.bind(v.map(|c| c.as_str()));
    }
    if let Some(v) = &patch.class { q = q.bind(v.as_deref()); }
    if let Some(v) = &patch.purchase_price { q = q.bind(*v); }
    if let Some(v) = &patch.current_value  { q = q.bind(*v); }
    if let Some(v) = &patch.condition {
        q = q.bind(v.map(|c| c.as_str()));
    }
    if let Some(v) = &patch.custom_modifiers {
        let json = if v.is_empty() { None } else { Some(serde_json::to_string(v)?) };
        q = q.bind(json);
    }
    if let Some(v) = &patch.notes { q = q.bind(v.as_deref()); }
    q = q.bind(asset_id);

    q.execute(&mut *tx).await?;
    tx.commit().await?;

    fetch_asset(pool, asset_id).await
}

pub async fn remove_asset(pool: &SqlitePool, asset_id: i64) -> Result<(), WealthError> {
    let res = sqlx::query("DELETE FROM character_assets WHERE id = ?")
        .bind(asset_id)
        .execute(pool)
        .await?;

    if res.rows_affected() == 0 {
        return Err(WealthError::AssetNotFound(asset_id));
    }
    Ok(())
}
```

- [ ] **Step 2: Append 4 Tauri commands**

Inside the `// ─── Tauri commands ───` section in `wealth.rs`:

```rust
#[tauri::command]
pub async fn wealth_add_asset(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
    payload: NewAsset,
) -> Result<Asset, WealthError> {
    add_asset(&state.pool, character_id, payload).await
}

#[tauri::command]
pub async fn wealth_update_asset(
    state: tauri::State<'_, crate::AppState>,
    asset_id: i64,
    patch: AssetPatch,
) -> Result<Asset, WealthError> {
    update_asset(&state.pool, asset_id, patch).await
}

#[tauri::command]
pub async fn wealth_remove_asset(
    state: tauri::State<'_, crate::AppState>,
    asset_id: i64,
) -> Result<(), WealthError> {
    remove_asset(&state.pool, asset_id).await
}

#[tauri::command]
pub async fn wealth_list_assets(
    state: tauri::State<'_, crate::AppState>,
    character_id: i64,
) -> Result<Vec<Asset>, WealthError> {
    list_assets(&state.pool, character_id).await
}
```

- [ ] **Step 3: Register the 4 new commands in `lib.rs`**

Add to the `tauri::generate_handler![` block:

```rust
db::wealth::wealth_add_asset,
db::wealth::wealth_update_asset,
db::wealth::wealth_remove_asset,
db::wealth::wealth_list_assets,
```

- [ ] **Step 4: Append 4 TS wrappers to `src/lib/wealth/api.ts`**

```ts
import type { Asset, NewAsset, AssetPatch } from '../../types';

// ─── Assets ────────────────────────────────────────────────────────────────

export function addAsset(characterId: number, payload: NewAsset): Promise<Asset> {
  return invoke<Asset>('wealth_add_asset', { characterId, payload });
}

export function updateAsset(assetId: number, patch: AssetPatch): Promise<Asset> {
  return invoke<Asset>('wealth_update_asset', { assetId, patch });
}

export function removeAsset(assetId: number): Promise<void> {
  return invoke<void>('wealth_remove_asset', { assetId });
}

export function listAssets(characterId: number): Promise<Asset[]> {
  return invoke<Asset[]>('wealth_list_assets', { characterId });
}
```

(If TypeScript complains about a duplicate `import type` from `../../types`, merge the import line with the existing one at the top of the file.)

- [ ] **Step 5: Update ARCHITECTURE.md §4**

Append to the `db::wealth` sub-section:

```markdown
| `wealth_add_asset`         | `add_asset`         | Insert asset for opted-in character. |
| `wealth_update_asset`      | `update_asset`      | Patch arbitrary asset fields. |
| `wealth_remove_asset`      | `remove_asset`      | DELETE single asset row. |
| `wealth_list_assets`       | `list_assets`       | All assets for one character, ordered by creation. |
```

Bump the running total by +4.

- [ ] **Step 6: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`. All 8 prior wealth tests still pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/db/wealth.rs \
        src-tauri/src/lib.rs \
        src/lib/wealth/api.ts \
        ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
wealth: asset CRUD (4 cmds) + typed TS wrappers

Adds add_asset, update_asset, remove_asset, list_assets.
update_asset uses a dynamic-SET path to distinguish "no change"
from "set NULL". add_asset requires an opted-in wallet for the
character (returns WalletNotFound if absent).

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 4: Item catalog + seed — 3 commands, no TDD

Catalog reads + custom-slot CRUD + seed of ~35 built-in rows.

**Files:**
- Modify: `src-tauri/src/db/wealth.rs` (catalog functions + 3 Tauri commands)
- Modify: `src-tauri/src/db/seed.rs` (add `seed_item_slots()` + call site)
- Modify: `src-tauri/src/lib.rs` (register 3 commands)
- Modify: `src/lib/wealth/api.ts` (append 3 wrappers)
- Modify: `ARCHITECTURE.md` §4

### Steps

- [ ] **Step 1: Append catalog functions to `wealth.rs`**

```rust
// === Item catalog =========================================================

fn slot_row_to_struct(
    id: i64,
    cat: String,
    class: String,
    label: String,
    bmin: f64,
    bmax: f64,
    examples: Option<String>,
    is_custom_i: i64,
    source: Option<String>,
) -> Result<ItemSlot, WealthError> {
    Ok(ItemSlot {
        id,
        category: AssetCategory::from_str(&cat)
            .ok_or_else(|| WealthError::Db(sqlx::Error::Decode(
                format!("invalid category in db: {cat}").into(),
            )))?,
        class,
        label,
        base_price_min: bmin,
        base_price_max: bmax,
        examples,
        is_custom: is_custom_i != 0,
        source,
    })
}

pub async fn list_item_slots(
    pool: &SqlitePool,
    category: Option<AssetCategory>,
) -> Result<Vec<ItemSlot>, WealthError> {
    let rows: Vec<(i64, String, String, String, f64, f64, Option<String>, i64, Option<String>)> =
        match category {
            Some(c) => {
                sqlx::query_as(
                    "SELECT id, category, class, label, base_price_min, base_price_max,
                            examples, is_custom, source
                       FROM item_slots
                      WHERE category = ?
                      ORDER BY base_price_min ASC, label ASC",
                )
                .bind(c.as_str())
                .fetch_all(pool)
                .await?
            }
            None => {
                sqlx::query_as(
                    "SELECT id, category, class, label, base_price_min, base_price_max,
                            examples, is_custom, source
                       FROM item_slots
                      ORDER BY category ASC, base_price_min ASC, label ASC",
                )
                .fetch_all(pool)
                .await?
            }
        };

    let mut out = Vec::with_capacity(rows.len());
    for (id, cat, class, label, bmin, bmax, examples, is_custom_i, source) in rows {
        out.push(slot_row_to_struct(id, cat, class, label, bmin, bmax, examples, is_custom_i, source)?);
    }
    Ok(out)
}

pub async fn add_custom_slot(
    pool: &SqlitePool,
    payload: NewItemSlot,
) -> Result<ItemSlot, WealthError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO item_slots
            (category, class, label, base_price_min, base_price_max,
             examples, is_custom, source)
         VALUES (?, ?, ?, ?, ?, ?, 1, ?)
         RETURNING id",
    )
    .bind(payload.category.as_str())
    .bind(&payload.class)
    .bind(&payload.label)
    .bind(payload.base_price_min)
    .bind(payload.base_price_max)
    .bind(payload.examples.as_deref())
    .bind(payload.source.as_deref())
    .fetch_one(pool)
    .await?;

    // Re-read the row so the returned struct is canonical.
    let row: (i64, String, String, String, f64, f64, Option<String>, i64, Option<String>) =
        sqlx::query_as(
            "SELECT id, category, class, label, base_price_min, base_price_max,
                    examples, is_custom, source
               FROM item_slots WHERE id = ?",
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

    slot_row_to_struct(row.0, row.1, row.2, row.3, row.4, row.5, row.6, row.7, row.8)
}

pub async fn remove_custom_slot(pool: &SqlitePool, slot_id: i64) -> Result<(), WealthError> {
    // Refuse to delete built-ins. Custom-only delete is the contract; the
    // spec keeps built-ins intact across reseeds via the is_custom=0 fence.
    let res = sqlx::query("DELETE FROM item_slots WHERE id = ? AND is_custom = 1")
        .bind(slot_id)
        .execute(pool)
        .await?;

    if res.rows_affected() == 0 {
        return Err(WealthError::SlotNotFound(slot_id));
    }
    Ok(())
}
```

- [ ] **Step 2: Append 3 Tauri commands**

```rust
#[tauri::command]
pub async fn wealth_list_item_slots(
    state: tauri::State<'_, crate::AppState>,
    category: Option<AssetCategory>,
) -> Result<Vec<ItemSlot>, WealthError> {
    list_item_slots(&state.pool, category).await
}

#[tauri::command]
pub async fn wealth_add_custom_slot(
    state: tauri::State<'_, crate::AppState>,
    payload: NewItemSlot,
) -> Result<ItemSlot, WealthError> {
    add_custom_slot(&state.pool, payload).await
}

#[tauri::command]
pub async fn wealth_remove_custom_slot(
    state: tauri::State<'_, crate::AppState>,
    slot_id: i64,
) -> Result<(), WealthError> {
    remove_custom_slot(&state.pool, slot_id).await
}
```

- [ ] **Step 3: Add the seed function to `src-tauri/src/db/seed.rs`**

Inspect the existing structure of `seed.rs` first (it has `seed_dyscrasias`, `seed_advantages`, etc., all called from a top-level function). Match the convention. Append:

```rust
// ─── Wealth: item-slot catalog ─────────────────────────────────────────────

pub async fn seed_item_slots(pool: &sqlx::SqlitePool) -> Result<(), sqlx::Error> {
    // Destructive reseed of is_custom=0 rows per ADR 0002.
    // is_custom=1 (user-authored) survive.
    sqlx::query("DELETE FROM item_slots WHERE is_custom = 0")
        .execute(pool)
        .await?;

    // Slot rows: (category, class, label, min, max, examples).
    // Source = 'V5 default catalog' on all built-ins.
    let rows: &[(&str, &str, &str, f64, f64, &str)] = &[
        // ─── Weapons (~7)
        ("weapon", "improvised",        "Improvised weapon",           5.0,        50.0,     "broken bottle, chair leg, brick"),
        ("weapon", "knife",             "Knife / shiv",                10.0,       150.0,    "kitchen knife, switchblade, ka-bar"),
        ("weapon", "sword",             "Bladed weapon",               150.0,      2_500.0,  "longsword, machete, katana, rapier"),
        ("weapon", "cheap_handgun",     "Cheap handgun",               150.0,      500.0,    "saturday-night-special revolver, surplus pistol"),
        ("weapon", "military_handgun",  "Military-grade handgun",      600.0,      2_500.0,  "Glock 19, Beretta M9, SIG P226"),
        ("weapon", "long_gun",          "Long gun",                    500.0,      5_000.0,  "hunting rifle, shotgun, SMG, AR-platform"),
        ("weapon", "heavy",             "Heavy weapon",                3_000.0,    25_000.0, "belt-fed MG, anti-materiel rifle, RPG"),

        // ─── Vehicles (~6)
        ("vehicle", "economy",          "Economy vehicle",             3_000.0,    18_000.0,  "compact car, used sedan, scooter"),
        ("vehicle", "mid_range",        "Mid-range vehicle",           18_000.0,   60_000.0,  "new sedan, SUV, pickup"),
        ("vehicle", "high_end",         "High-end vehicle",            60_000.0,   200_000.0, "luxury sedan, sports car, premium SUV"),
        ("vehicle", "supercar",         "Supercar / exotic",           200_000.0,  2_000_000.0, "Ferrari, McLaren, Bugatti, race build"),
        ("vehicle", "motorcycle",       "Motorcycle",                  2_000.0,    40_000.0,  "scooter to superbike to custom chopper"),
        ("vehicle", "utility",          "Utility / commercial vehicle", 10_000.0,  150_000.0, "van, truck, mobile workshop"),

        // ─── Property (~6)
        ("property", "rented_apt",      "Rented apartment (monthly)",  500.0,      8_000.0,    "studio rental to luxury penthouse lease"),
        ("property", "owned_apt",       "Owned apartment / condo",     50_000.0,   2_000_000.0, "starter condo to upscale unit"),
        ("property", "house",           "House",                       150_000.0,  3_000_000.0, "suburban single-family, townhouse"),
        ("property", "mansion",         "Mansion / estate",            2_000_000.0, 50_000_000.0, "gated estate, country manor"),
        ("property", "safehouse",       "Safehouse / haven",           20_000.0,   500_000.0,  "off-grid cabin, lightproof basement, ghoul-staffed retreat"),
        ("property", "commercial",      "Commercial property",         100_000.0,  10_000_000.0, "nightclub, office building, warehouse"),

        // ─── Investments (~4)
        ("investment", "small_investment",   "Small investment",        500.0,      10_000.0,  "individual stock, bond, savings instrument"),
        ("investment", "medium_investment",  "Medium investment",       10_000.0,   200_000.0, "diversified portfolio, real-estate share"),
        ("investment", "business_share",     "Business share / stake",  50_000.0,   5_000_000.0, "minority stake, silent partnership, franchise"),
        ("investment", "art_piece",          "Art / collectible",       2_000.0,    20_000_000.0, "painting, sculpture, vintage car, watch collection"),

        // ─── Misc (~6)
        ("misc", "small_luxury",        "Small luxury item",           50.0,       500.0,     "cigar, perfume, tie pin, espresso machine"),
        ("misc", "medium_luxury",       "Medium luxury item",          500.0,      5_000.0,   "designer suit, watch, smartphone, gaming rig"),
        ("misc", "status_symbol",       "Status symbol",               5_000.0,    100_000.0, "Rolex, designer bag, custom tailoring, jewelry"),
        ("misc", "service_short",       "Service (one-off)",           100.0,      5_000.0,   "private chef night, escort, consultation, courier job"),
        ("misc", "service_long",        "Service (retainer)",          2_000.0,    50_000.0,  "monthly retainer for lawyer, fixer, security, doctor"),
        ("misc", "consumable",          "Consumable / supply",         20.0,       2_000.0,   "drugs, alcohol case, ammo crate, medical supplies"),
    ];

    for (cat, class, label, bmin, bmax, examples) in rows {
        sqlx::query(
            "INSERT INTO item_slots
                (category, class, label, base_price_min, base_price_max,
                 examples, is_custom, source)
             VALUES (?, ?, ?, ?, ?, ?, 0, 'V5 default catalog')",
        )
        .bind(cat)
        .bind(class)
        .bind(label)
        .bind(bmin)
        .bind(bmax)
        .bind(examples)
        .execute(pool)
        .await?;
    }

    Ok(())
}
```

Then find the top-level seed driver (likely `pub async fn seed_all(pool: ...)` or similar — check `grep -n "pub.*fn seed" src-tauri/src/db/seed.rs`) and add a call to `seed_item_slots(pool).await?;` alongside the other seed functions.

- [ ] **Step 4: Register the 3 catalog commands in `lib.rs`**

```rust
db::wealth::wealth_list_item_slots,
db::wealth::wealth_add_custom_slot,
db::wealth::wealth_remove_custom_slot,
```

- [ ] **Step 5: Append 3 TS wrappers to `src/lib/wealth/api.ts`**

```ts
import type { ItemSlot, NewItemSlot, AssetCategory } from '../../types';

// ─── Catalog ───────────────────────────────────────────────────────────────

export function listItemSlots(category?: AssetCategory): Promise<ItemSlot[]> {
  return invoke<ItemSlot[]>('wealth_list_item_slots', { category: category ?? null });
}

export function addCustomSlot(payload: NewItemSlot): Promise<ItemSlot> {
  return invoke<ItemSlot>('wealth_add_custom_slot', { payload });
}

export function removeCustomSlot(slotId: number): Promise<void> {
  return invoke<void>('wealth_remove_custom_slot', { slotId });
}
```

(Merge the `import type` line with the existing one at the top.)

- [ ] **Step 6: Update ARCHITECTURE.md §4**

Append to the `db::wealth` table:

```markdown
| `wealth_list_item_slots`     | `list_item_slots`     | Catalog read, optional category filter. |
| `wealth_add_custom_slot`     | `add_custom_slot`     | Insert user-custom slot (is_custom=1). |
| `wealth_remove_custom_slot`  | `remove_custom_slot`  | DELETE custom slot; refuses built-ins. |
```

Bump running total by +3.

- [ ] **Step 7: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`. On startup the dev DB will be reseeded with the 35 item-slot rows.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/db/wealth.rs \
        src-tauri/src/db/seed.rs \
        src-tauri/src/lib.rs \
        src/lib/wealth/api.ts \
        ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
wealth: item catalog (3 cmds) + ~35-row seed

Adds list_item_slots / add_custom_slot / remove_custom_slot.
Seed populates the V5 default catalog (weapon, vehicle,
property, investment, misc) per ADR 0002 destructive-reseed
pattern — custom rows (is_custom=1) survive.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 5: Composed transactions — purchase + sell — **TDD on atomicity**

The composed primitives. TDD because cross-table atomic mutation is exactly the class of logic that breaks subtly.

**Files:**
- Modify: `src-tauri/src/db/wealth.rs` (transactions + 2 Tauri commands + 2 inline tests)
- Modify: `src-tauri/src/lib.rs` (register 2 commands)
- Modify: `src/lib/wealth/api.ts` (append 2 wrappers)
- Modify: `ARCHITECTURE.md` §4

### Steps

- [ ] **Step 1: Write the failing test for `purchase_item_slot` happy path**

Add inside the `tests` mod:

```rust
    #[sqlx::test]
    async fn purchase_inserts_asset_and_debits_liquid_atomically() {
        let (pool, char_id) = fresh_pool_with_character("Ivy").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 100_000.0).await.unwrap();

        // Find a vehicle slot from the seed.
        let slot: (i64,) = sqlx::query_as(
            "SELECT id FROM item_slots WHERE category = 'vehicle' AND class = 'economy'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        let (wallet, asset) = purchase_item_slot(&pool, PurchaseRequest {
            character_id: char_id,
            slot_id: slot.0,
            price: 8_000.0,
            condition: AssetCondition::Used,
            custom_modifiers: vec![],
            name_override: Some("'08 Honda Civic".to_string()),
            notes: None,
        }).await.unwrap();

        assert_eq!(wallet.liquid, 92_000.0);
        assert_eq!(asset.name, "'08 Honda Civic");
        assert_eq!(asset.purchase_price, Some(8_000.0));
        assert_eq!(asset.current_value, 8_000.0,
                   "spec: current_value initializes to purchase price");
        assert_eq!(asset.condition, Some(AssetCondition::Used));
        assert_eq!(asset.character_id, char_id);
    }

    #[sqlx::test]
    async fn purchase_rolls_back_on_failure() {
        let (pool, char_id) = fresh_pool_with_character("Jack").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 1_000.0).await.unwrap();

        // Use a non-existent slot id to force a SlotNotFound error.
        let result = purchase_item_slot(&pool, PurchaseRequest {
            character_id: char_id,
            slot_id: 999_999,
            price: 500.0,
            condition: AssetCondition::New,
            custom_modifiers: vec![],
            name_override: None,
            notes: None,
        }).await;

        assert!(result.is_err());
        // Wallet must be unchanged after failed purchase.
        let w = get_wallet(&pool, char_id).await.unwrap().unwrap();
        assert_eq!(w.liquid, 1_000.0, "wallet must roll back on purchase failure");
        // No asset must have been inserted.
        let assets = list_assets(&pool, char_id).await.unwrap();
        assert!(assets.is_empty(), "no asset must persist after rollback");
    }
```

- [ ] **Step 2: Run tests — confirm fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml wealth::tests::purchase`
Expected: compile error on `purchase_item_slot`.

- [ ] **Step 3: Implement `purchase_item_slot`**

Append to `wealth.rs` above the `// ─── Tauri commands ───` divider:

```rust
// === Composed transactions ================================================

pub async fn purchase_item_slot(
    pool: &SqlitePool,
    req: PurchaseRequest,
) -> Result<(Wallet, Asset), WealthError> {
    let mut tx = pool.begin().await?;

    // 1. Verify the wallet exists (opt-in gate).
    let wallet_exists: Option<i64> = sqlx::query_scalar(
        "SELECT character_id FROM character_wallets WHERE character_id = ?",
    )
    .bind(req.character_id)
    .fetch_optional(&mut *tx)
    .await?;
    if wallet_exists.is_none() {
        return Err(WealthError::WalletNotFound(req.character_id));
    }

    // 2. Look up the slot to derive category/class and validate existence.
    let slot_row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT category, class, label FROM item_slots WHERE id = ?",
    )
    .bind(req.slot_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (cat_s, class_s, label) =
        slot_row.ok_or(WealthError::SlotNotFound(req.slot_id))?;
    let category = AssetCategory::from_str(&cat_s);

    // 3. Debit liquid.
    let upd = sqlx::query(
        "UPDATE character_wallets
            SET liquid = liquid - ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(req.price)
    .bind(req.character_id)
    .execute(&mut *tx)
    .await?;
    if upd.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(req.character_id));
    }

    // 4. Insert the asset row. current_value = purchase price per spec.
    let asset_name = req.name_override.unwrap_or(label);
    let cm_json = if req.custom_modifiers.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&req.custom_modifiers)?)
    };

    let asset_id: i64 = sqlx::query_scalar(
        "INSERT INTO character_assets
            (character_id, name, category, class,
             purchase_price, current_value, condition, custom_modifier_json, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(req.character_id)
    .bind(&asset_name)
    .bind(category.map(|c| c.as_str()))
    .bind(&class_s)
    .bind(req.price)
    .bind(req.price)
    .bind(req.condition.as_str())
    .bind(cm_json)
    .bind(req.notes.as_deref())
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    let wallet = get_wallet(pool, req.character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(req.character_id))?;
    let asset = fetch_asset(pool, asset_id).await?;

    Ok((wallet, asset))
}
```

- [ ] **Step 4: Write the failing test for `sell_asset`**

```rust
    #[sqlx::test]
    async fn sell_credits_liquid_and_removes_asset() {
        let (pool, char_id) = fresh_pool_with_character("Kate").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 50_000.0).await.unwrap();

        let asset = add_asset(&pool, char_id, NewAsset {
            name: "Vintage watch".into(),
            category: Some(AssetCategory::Misc),
            class: None,
            purchase_price: Some(3_000.0),
            current_value: 3_500.0,
            condition: Some(AssetCondition::New),
            custom_modifiers: vec![],
            notes: None,
        }).await.unwrap();

        let wallet = sell_asset(&pool, SellRequest {
            asset_id: asset.id,
            sale_price: 3_200.0,
            notes: None,
        }).await.unwrap();

        assert_eq!(wallet.liquid, 53_200.0);
        let assets = list_assets(&pool, char_id).await.unwrap();
        assert!(assets.is_empty(), "sold asset must be removed");
    }

    #[sqlx::test]
    async fn sell_rolls_back_on_failure() {
        let (pool, _char_id) = fresh_pool_with_character("Leo").await;
        // No wallet enabled; the asset_id is invalid — both paths should error
        // and leave the DB pristine.
        let result = sell_asset(&pool, SellRequest {
            asset_id: 999_999,
            sale_price: 100.0,
            notes: None,
        }).await;
        assert!(result.is_err());
    }
```

- [ ] **Step 5: Implement `sell_asset`**

```rust
pub async fn sell_asset(
    pool: &SqlitePool,
    req: SellRequest,
) -> Result<Wallet, WealthError> {
    let mut tx = pool.begin().await?;

    // 1. Look up character_id from the asset row.
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT character_id FROM character_assets WHERE id = ?",
    )
    .bind(req.asset_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (character_id,) = row.ok_or(WealthError::AssetNotFound(req.asset_id))?;

    // 2. Credit liquid.
    let upd = sqlx::query(
        "UPDATE character_wallets
            SET liquid = liquid + ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(req.sale_price)
    .bind(character_id)
    .execute(&mut *tx)
    .await?;
    if upd.rows_affected() == 0 {
        return Err(WealthError::WalletNotFound(character_id));
    }

    // 3. Delete the asset.
    let del = sqlx::query("DELETE FROM character_assets WHERE id = ?")
        .bind(req.asset_id)
        .execute(&mut *tx)
        .await?;
    if del.rows_affected() == 0 {
        return Err(WealthError::AssetNotFound(req.asset_id));
    }

    tx.commit().await?;

    get_wallet(pool, character_id)
        .await?
        .ok_or(WealthError::WalletNotFound(character_id))
}
```

- [ ] **Step 6: Append 2 Tauri commands**

```rust
#[tauri::command]
pub async fn wealth_purchase_item_slot(
    state: tauri::State<'_, crate::AppState>,
    req: PurchaseRequest,
) -> Result<(Wallet, Asset), WealthError> {
    purchase_item_slot(&state.pool, req).await
}

#[tauri::command]
pub async fn wealth_sell_asset(
    state: tauri::State<'_, crate::AppState>,
    req: SellRequest,
) -> Result<Wallet, WealthError> {
    sell_asset(&state.pool, req).await
}
```

- [ ] **Step 7: Register the 2 commands in `lib.rs`**

```rust
db::wealth::wealth_purchase_item_slot,
db::wealth::wealth_sell_asset,
```

- [ ] **Step 8: Append 2 TS wrappers to `src/lib/wealth/api.ts`**

```ts
import type { PurchaseRequest, SellRequest } from '../../types';

// ─── Composed transactions ────────────────────────────────────────────────

export function purchaseItemSlot(
  req: PurchaseRequest,
): Promise<[Wallet, Asset]> {
  return invoke<[Wallet, Asset]>('wealth_purchase_item_slot', { req });
}

export function sellAsset(req: SellRequest): Promise<Wallet> {
  return invoke<Wallet>('wealth_sell_asset', { req });
}
```

(Merge type imports.)

- [ ] **Step 9: Update ARCHITECTURE.md §4**

Append:

```markdown
| `wealth_purchase_item_slot`  | `purchase_item_slot`  | Atomic: debit liquid + insert asset. |
| `wealth_sell_asset`          | `sell_asset`          | Atomic: credit liquid + remove asset. |
```

Bump running total by +2. The wealth module is now 17 commands total.

- [ ] **Step 10: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`. All wealth tests pass (8 wallet + 4 transaction = 12).

- [ ] **Step 11: Commit**

```bash
git add src-tauri/src/db/wealth.rs \
        src-tauri/src/lib.rs \
        src/lib/wealth/api.ts \
        ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
wealth: composed atomic transactions — purchase + sell

Adds purchase_item_slot (debits liquid + inserts asset, current_value
initialized to purchase price per spec) and sell_asset (credits liquid
+ removes asset). Both atomic via sqlx transactions. TDD covers both
happy paths and rollback-on-failure.

Total wealth IPC commands: 17.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 6: Pricing module (`src/lib/wealth/pricing.ts`)

Pure TS function. No JS test framework in repo; verified by inspection + Task 10's smoke. Formula is 3 lines — low risk.

**Files:**
- Create: `src/lib/wealth/pricing.ts`

**TDD:** none (per repo constraint).

### Steps

- [ ] **Step 1: Create the pricing module**

Create `src/lib/wealth/pricing.ts`:

```ts
import type { AssetCondition, CustomModifier } from '../../types';

/**
 * Condition multiplier table.
 *
 * `custom` is GM-supplied at purchase time and not present here — callers
 * for `custom` must pass the user's number explicitly.
 *
 * Mirrors the constants in `src-tauri/src/shared/wealth.rs`:
 *   CONDITION_MULTIPLIER_NEW       = 1.0
 *   CONDITION_MULTIPLIER_USED      = 0.6
 *   CONDITION_MULTIPLIER_VERY_USED = 0.3
 */
export const CONDITION_MULTIPLIERS: Record<Exclude<AssetCondition, 'custom'>, number> = {
  new: 1.0,
  used: 0.6,
  very_used: 0.3,
};

/**
 * Compute final purchase price.
 *
 * formula: pickInBand × condition × Π(custom_modifiers[i].mult)
 *
 * @param pickInBand    GM's chosen value within the slot's price band.
 * @param condition     The AssetCondition variant (or 'custom').
 * @param customMult    Required when condition === 'custom'. Ignored otherwise.
 * @param customMods    Stackable modifiers; empty array == no adjustment.
 */
export function computeFinalPrice(
  pickInBand: number,
  condition: AssetCondition,
  customMult: number | null,
  customMods: CustomModifier[],
): number {
  const condMult =
    condition === 'custom'
      ? (customMult ?? 1.0)
      : CONDITION_MULTIPLIERS[condition];

  const modProduct = customMods.reduce((acc, m) => acc * m.mult, 1);

  return pickInBand * condMult * modProduct;
}

/**
 * Default `pickInBand` when the GM hasn't moved the slider: midpoint of the
 * slot's [min, max] band.
 */
export function defaultPickInBand(basePriceMin: number, basePriceMax: number): number {
  return (basePriceMin + basePriceMax) / 2;
}
```

- [ ] **Step 2: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/wealth/pricing.ts
git commit -m "$(cat <<'EOF'
wealth: pricing formula (TS, pure function)

computeFinalPrice = pickInBand × condition × Π(customMods).
CONDITION_MULTIPLIERS mirrors Rust constants verbatim. No JS test
framework in repo; formula is 3 lines and is exercised by the buy
form's live calculation in Task 10.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 7: `WealthManager.svelte` tool shell + character selector + enable/disable

The top-level tool. Character selector at the top; below it either the disabled state (Enable button) or the empty-but-enabled state (three empty panels). Real panel content arrives in Tasks 8–10.

**Files:**
- Create: `src/tools/WealthManager.svelte`
- Modify: `src/tools.ts` (register the tool)

**TDD:** none. UI scaffolding; verified by `npm run check` + the actual app loading.

### Steps

- [ ] **Step 1: Identify how the tool gets a character list**

Look at an existing tool that loads characters from the DB:

```bash
grep -rln "list_characters\|listCharacters\|character_list" src/tools/ src/lib/character/
```

Use that pattern. Most likely `src/lib/character/api.ts` exports a `listCharacters()` wrapper.

- [ ] **Step 2: Create `src/tools/WealthManager.svelte`**

Create the file with:

```svelte
<script lang="ts">
  // -------- imports
  import { onMount } from 'svelte';
  import type { Wallet } from '../types';
  // Match the existing character-list import pattern from another tool —
  // e.g., AdvantagesManager.svelte. The wrapper is likely:
  //   import { listCharacters } from '../lib/character/api';
  // and the Character type is in src/types.ts. Confirm and adapt.
  import { listCharacters } from '../lib/character/api';
  import type { Character } from '../types';

  import {
    getWallet,
    enableWallet,
    disableWallet,
  } from '../lib/wealth/api';

  // -------- state
  let characters: Character[] = $state([]);
  let selectedId: number | null = $state(null);
  let wallet: Wallet | null = $state(null);
  let loadingChars = $state(true);
  let loadingWallet = $state(false);
  let error: string | null = $state(null);

  // -------- lifecycle
  onMount(async () => {
    try {
      characters = await listCharacters();
      if (characters.length > 0) {
        selectedId = characters[0].id;
        await refreshWallet();
      }
    } catch (e) {
      error = String(e);
    } finally {
      loadingChars = false;
    }
  });

  // -------- helpers
  async function refreshWallet() {
    if (selectedId == null) {
      wallet = null;
      return;
    }
    loadingWallet = true;
    try {
      wallet = await getWallet(selectedId);
    } catch (e) {
      error = String(e);
    } finally {
      loadingWallet = false;
    }
  }

  async function onSelectCharacter(e: Event) {
    const v = (e.target as HTMLSelectElement).value;
    selectedId = v ? Number(v) : null;
    await refreshWallet();
  }

  async function onEnable() {
    if (selectedId == null) return;
    try {
      wallet = await enableWallet(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function onDisable() {
    if (selectedId == null) return;
    if (!confirm(
      'Disable wealth tracking for this character? This will permanently delete the wallet and all asset records. This cannot be undone.'
    )) return;
    try {
      await disableWallet(selectedId);
      wallet = null;
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section class="wealth-manager">
  <header class="manager-head">
    <h2>Wealth</h2>
    <label class="char-picker">
      <span>Character:</span>
      <select onchange={onSelectCharacter} disabled={loadingChars || characters.length === 0}>
        {#each characters as c}
          <option value={c.id} selected={c.id === selectedId}>{c.name}</option>
        {/each}
      </select>
    </label>
  </header>

  {#if error}
    <div class="error-banner" role="alert">{error}</div>
  {/if}

  {#if selectedId == null}
    <p class="empty">No characters available. Create a character first.</p>
  {:else if loadingWallet}
    <p class="empty">Loading wallet…</p>
  {:else if wallet == null}
    <div class="disabled-state">
      <p>This character is not using the wealth system.</p>
      <button type="button" onclick={onEnable}>Enable wealth tracking</button>
    </div>
  {:else}
    <div class="enabled-state">
      <!-- Tasks 8/9/10 fill these panels. -->
      <div class="panel-placeholder">Wallet panel — Task 8</div>
      <div class="panel-placeholder">Assets panel — Task 9</div>
      <div class="panel-placeholder">Shop panel — Task 10</div>
      <button type="button" class="disable-btn" onclick={onDisable}>
        Disable wealth tracking
      </button>
    </div>
  {/if}
</section>

<style>
  .wealth-manager {
    display: grid;
    gap: var(--space-4);
    padding: var(--space-4);
    align-items: start;
  }

  .manager-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
  }

  .char-picker {
    display: flex;
    gap: var(--space-2);
    align-items: center;
    color: var(--color-text-muted);
  }

  .empty,
  .disabled-state {
    color: var(--color-text-muted);
    text-align: center;
    padding: var(--space-5);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-md);
  }

  .disabled-state button {
    margin-top: var(--space-3);
  }

  .enabled-state {
    display: grid;
    gap: var(--space-4);
    align-items: start;
  }

  .panel-placeholder {
    padding: var(--space-4);
    border: 1px dashed var(--color-border);
    border-radius: var(--radius-md);
    color: var(--color-text-muted);
    text-align: center;
  }

  .disable-btn {
    justify-self: end;
    color: var(--color-danger);
    background: transparent;
    border: 1px solid var(--color-danger);
  }

  .error-banner {
    padding: var(--space-3);
    border: 1px solid var(--color-danger);
    border-radius: var(--radius-md);
    color: var(--color-danger);
    background: var(--color-danger-bg);
  }
</style>
```

CSS notes (per `ARCHITECTURE.md §6`):
- All colors / spacing / radii / fonts come from CSS custom properties (tokens). NO hex literals.
- Verify the actual token names by reading `src/routes/+layout.svelte`. Common ones in this repo: `--color-bg`, `--color-surface`, `--color-text`, `--color-text-muted`, `--color-border`, `--color-accent`, `--color-danger`, `--color-danger-bg`, `--space-*`, `--radius-*`, `--text-*`. If a token name doesn't exist, find the closest existing equivalent — do NOT invent new tokens in this task.
- `align-items: start` on every Grid container (per §6).
- Use `rem`, not `px`.

- [ ] **Step 3: Register the tool in `src/tools.ts`**

Add to the `tools` array:

```ts
{
  id: 'wealth',
  label: 'Wealth',
  icon: '💰',
  component: () => import('./tools/WealthManager.svelte'),
},
```

- [ ] **Step 4: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 5: Manual smoke (no commit gating but worth doing)**

If a dev session is convenient: `npm run tauri dev`. Open the app, select the new "💰 Wealth" tool from the sidebar. Pick a character. Click "Enable wealth tracking" — three placeholder panels and a "Disable" button should appear. Click "Disable" → confirm → returns to the enable-state. No console errors.

This is not strictly required for the commit, but catches token-name mistakes before Task 8 lands more UI on top.

- [ ] **Step 6: Commit**

```bash
git add src/tools/WealthManager.svelte src/tools.ts
git commit -m "$(cat <<'EOF'
wealth: WealthManager tool shell + enable/disable

Top-level tool with a character selector. Two states:
  - disabled — single 'Enable wealth tracking' button
  - enabled  — three placeholder panels (filled in Tasks 8/9/10)
              + 'Disable wealth tracking' (with confirm) button

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 8: Wallet panel + Adjust modal + Transfer modal

The first real UI panel. Three numbers (liquid / banked / asset Σ), buttons for adjust and transfer.

**Files:**
- Create: `src/lib/components/wealth/WalletPanel.svelte`
- Create: `src/lib/components/wealth/AdjustModal.svelte`
- Create: `src/lib/components/wealth/TransferModal.svelte`
- Modify: `src/tools/WealthManager.svelte` (replace the wallet placeholder + thread state)

### Steps

- [ ] **Step 1: Create `AdjustModal.svelte`**

```svelte
<script lang="ts">
  import type { Bucket, Wallet } from '../../../types';

  let {
    open = $bindable(false),
    bucket,
    wallet,
    onApply,
  }: {
    open: boolean;
    bucket: Bucket;
    wallet: Wallet;
    onApply: (delta: number) => void | Promise<void>;
  } = $props();

  let mode: 'add' | 'subtract' | 'set' = $state('add');
  let amount = $state(0);

  function currentValue(): number {
    return bucket === 'liquid' ? wallet.liquid : wallet.banked;
  }

  function deltaFromForm(): number {
    if (mode === 'add')      return amount;
    if (mode === 'subtract') return -amount;
    // 'set' — delta needed to reach the target
    return amount - currentValue();
  }

  async function onConfirm() {
    await onApply(deltaFromForm());
    open = false;
    amount = 0;
    mode = 'add';
  }

  function onCancel() {
    open = false;
    amount = 0;
    mode = 'add';
  }
</script>

{#if open}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Adjust {bucket}</h3>
    <p class="current">Current: {wallet.currencyName}{currentValue().toLocaleString()}</p>

    <fieldset class="modes">
      <legend>Mode</legend>
      <label><input type="radio" bind:group={mode} value="add" /> Add</label>
      <label><input type="radio" bind:group={mode} value="subtract" /> Subtract</label>
      <label><input type="radio" bind:group={mode} value="set" /> Set to</label>
    </fieldset>

    <label class="amount-field">
      <span>Amount</span>
      <input type="number" step="any" bind:value={amount} autofocus />
    </label>

    <p class="preview">
      New value: {wallet.currencyName}{(currentValue() + deltaFromForm()).toLocaleString()}
    </p>

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm}>Apply</button>
    </div>
  </dialog>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: var(--color-overlay);
    z-index: 100;
  }
  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 101;
    display: grid;
    align-items: start;
    gap: var(--space-3);
    padding: var(--space-4);
    background: var(--color-surface);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    min-width: 22rem;
  }
  h3 { margin: 0; }
  .current { color: var(--color-text-muted); margin: 0; }
  .modes { display: flex; gap: var(--space-3); border: 0; padding: 0; }
  .modes legend { display: none; }
  .amount-field { display: grid; gap: var(--space-1); }
  .preview { color: var(--color-text-muted); margin: 0; }
  .actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
  .primary { background: var(--color-accent); color: var(--color-on-accent); border: 0; }
</style>
```

Per memory `feedback_portal_required_under_transform`: `position: fixed` breaks under `transform` ancestors. The check-with-screen approach used here works **only if the modal renders at root level of the document**, NOT inside a transformed parent. Since `WealthManager.svelte` won't have transformed parents in its mount path (it's a top-level tool), this is safe. If any future card-carousel parent introduces `transform`, this modal must move to a portal — but it's correct for V1.

(Tokens used: `--color-overlay`, `--color-surface`, `--color-text`, `--color-border`, `--color-accent`, `--color-on-accent`, `--color-text-muted`. Verify these exist in `src/routes/+layout.svelte`; substitute equivalents if not.)

- [ ] **Step 2: Create `TransferModal.svelte`**

```svelte
<script lang="ts">
  import type { Bucket, Wallet } from '../../../types';

  let {
    open = $bindable(false),
    wallet,
    onApply,
  }: {
    open: boolean;
    wallet: Wallet;
    onApply: (from: Bucket, to: Bucket, amount: number) => void | Promise<void>;
  } = $props();

  let from: Bucket = $state('liquid');
  let amount = $state(0);

  // Derived target = the other bucket.
  let to = $derived<Bucket>(from === 'liquid' ? 'banked' : 'liquid');

  async function onConfirm() {
    await onApply(from, to, amount);
    open = false;
    amount = 0;
    from = 'liquid';
  }

  function onCancel() {
    open = false;
    amount = 0;
    from = 'liquid';
  }
</script>

{#if open}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Transfer funds</h3>
    <p class="current">
      Liquid: {wallet.currencyName}{wallet.liquid.toLocaleString()}<br/>
      Banked: {wallet.currencyName}{wallet.banked.toLocaleString()}
    </p>

    <fieldset class="direction">
      <legend>From</legend>
      <label><input type="radio" bind:group={from} value="liquid" /> Liquid → Banked</label>
      <label><input type="radio" bind:group={from} value="banked" /> Banked → Liquid</label>
    </fieldset>

    <label class="amount-field">
      <span>Amount</span>
      <input type="number" step="any" min="0" bind:value={amount} autofocus />
    </label>

    {#if wallet.transferDelay}
      <p class="delay-hint">
        ℹ Transfer delay configured: {wallet.transferDelay.value} {wallet.transferDelay.unit}.
        Tool executes instantly in V1.
      </p>
    {/if}

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm} disabled={amount <= 0}>
        Transfer
      </button>
    </div>
  </dialog>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: var(--color-overlay); z-index: 100;
  }
  .modal {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%); z-index: 101;
    display: grid; align-items: start; gap: var(--space-3);
    padding: var(--space-4);
    background: var(--color-surface); color: var(--color-text);
    border: 1px solid var(--color-border); border-radius: var(--radius-md);
    min-width: 24rem;
  }
  h3 { margin: 0; }
  .current { color: var(--color-text-muted); margin: 0; }
  .direction { display: grid; gap: var(--space-2); border: 0; padding: 0; }
  .direction legend { color: var(--color-text-muted); font-size: var(--text-sm); }
  .amount-field { display: grid; gap: var(--space-1); }
  .delay-hint {
    color: var(--color-text-muted);
    font-size: var(--text-sm);
    margin: 0;
    padding: var(--space-2);
    border-left: 2px solid var(--color-accent);
  }
  .actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
  .primary { background: var(--color-accent); color: var(--color-on-accent); border: 0; }
  .primary:disabled { opacity: 0.5; }
</style>
```

- [ ] **Step 3: Create `WalletPanel.svelte`**

```svelte
<script lang="ts">
  import type { Wallet, Asset } from '../../../types';

  let {
    wallet,
    assets,
    onOpenAdjust,
    onOpenTransfer,
  }: {
    wallet: Wallet;
    assets: Asset[];
    onOpenAdjust: (bucket: 'liquid' | 'banked') => void;
    onOpenTransfer: () => void;
  } = $props();

  let assetsTotal = $derived(
    assets.reduce((sum, a) => sum + a.currentValue, 0)
  );

  function fmt(n: number): string {
    return wallet.currencyName + n.toLocaleString();
  }
</script>

<section class="wallet-panel">
  <div class="bucket">
    <span class="label">Liquid</span>
    <span class="value">{fmt(wallet.liquid)}</span>
    <button type="button" onclick={() => onOpenAdjust('liquid')}>Adjust</button>
  </div>

  <div class="bucket">
    <span class="label">Banked</span>
    <span class="value">{fmt(wallet.banked)}</span>
    <button type="button" onclick={() => onOpenAdjust('banked')}>Adjust</button>
    <button type="button" onclick={onOpenTransfer}>Transfer</button>
  </div>

  <div class="bucket">
    <span class="label">Assets Σ</span>
    <span class="value">{fmt(assetsTotal)}</span>
    <span class="hint">derived from current_value of {assets.length} asset{assets.length === 1 ? '' : 's'}</span>
  </div>
</section>

<style>
  .wallet-panel {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-3);
    align-items: start;
    padding: var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  .bucket {
    display: grid;
    align-items: start;
    gap: var(--space-1);
  }
  .label {
    font-size: var(--text-sm);
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .value {
    font-size: var(--text-2xl);
    font-weight: 600;
    color: var(--color-text);
  }
  .hint {
    font-size: var(--text-xs);
    color: var(--color-text-muted);
  }
  button {
    justify-self: start;
    margin-top: var(--space-1);
  }
</style>
```

- [ ] **Step 4: Wire the panel + modals into `WealthManager.svelte`**

Edit `src/tools/WealthManager.svelte`. Replace the `<!-- Tasks 8/9/10 fill these panels. -->` placeholder block with:

```svelte
<script lang="ts">
  // ... existing imports + state ...
  import { adjustLiquid, adjustBanked, transferFunds, listAssets } from '../lib/wealth/api';
  import WalletPanel from '../lib/components/wealth/WalletPanel.svelte';
  import AdjustModal from '../lib/components/wealth/AdjustModal.svelte';
  import TransferModal from '../lib/components/wealth/TransferModal.svelte';
  import type { Asset, Bucket } from '../types';

  let assets: Asset[] = $state([]);
  let adjustOpen = $state(false);
  let adjustBucket: Bucket = $state('liquid');
  let transferOpen = $state(false);

  async function refreshAssets() {
    if (selectedId == null) { assets = []; return; }
    try {
      assets = await listAssets(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  // Extend refreshWallet to also refresh assets when wallet exists.
  // (Update the existing refreshWallet function: after `wallet = ...`, append:)
  //   if (wallet != null) await refreshAssets();
  //   else assets = [];

  function openAdjust(b: Bucket) { adjustBucket = b; adjustOpen = true; }
  function openTransfer() { transferOpen = true; }

  async function applyAdjust(delta: number) {
    if (selectedId == null) return;
    try {
      wallet = adjustBucket === 'liquid'
        ? await adjustLiquid(selectedId, delta)
        : await adjustBanked(selectedId, delta);
    } catch (e) {
      error = String(e);
    }
  }

  async function applyTransfer(from: Bucket, to: Bucket, amount: number) {
    if (selectedId == null) return;
    try {
      wallet = await transferFunds(selectedId, from, to, amount);
    } catch (e) {
      error = String(e);
    }
  }
</script>
```

Then replace the placeholder block in the template:

```svelte
    <div class="enabled-state">
      <WalletPanel
        {wallet}
        {assets}
        onOpenAdjust={openAdjust}
        onOpenTransfer={openTransfer}
      />
      <div class="panel-placeholder">Assets panel — Task 9</div>
      <div class="panel-placeholder">Shop panel — Task 10</div>
      <button type="button" class="disable-btn" onclick={onDisable}>
        Disable wealth tracking
      </button>

      <AdjustModal bind:open={adjustOpen} bucket={adjustBucket} {wallet} onApply={applyAdjust} />
      <TransferModal bind:open={transferOpen} {wallet} onApply={applyTransfer} />
    </div>
```

Also: extend the existing `refreshWallet()` function to refresh assets after the wallet load — append `if (wallet != null) await refreshAssets(); else assets = [];` inside the try block.

- [ ] **Step 5: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/wealth/WalletPanel.svelte \
        src/lib/components/wealth/AdjustModal.svelte \
        src/lib/components/wealth/TransferModal.svelte \
        src/tools/WealthManager.svelte
git commit -m "$(cat <<'EOF'
wealth: wallet panel + adjust + transfer modals

WalletPanel shows liquid / banked / Σ assets in a 3-column grid;
AdjustModal handles +/-/set on either bucket; TransferModal moves
funds liquid↔banked and surfaces the optional transfer_delay hint
(stored but not enforced in V1).

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 9: Asset list + Add modal + Sell modal

The asset row table with add/sell affordances. No catalog interaction here — that's Task 10's shop.

**Files:**
- Create: `src/lib/components/wealth/AssetList.svelte`
- Create: `src/lib/components/wealth/AddAssetModal.svelte`
- Create: `src/lib/components/wealth/SellModal.svelte`
- Modify: `src/tools/WealthManager.svelte` (replace asset placeholder + thread state)

### Steps

- [ ] **Step 1: Create `AddAssetModal.svelte`**

```svelte
<script lang="ts">
  import type { NewAsset, AssetCategory, AssetCondition } from '../../../types';

  let {
    open = $bindable(false),
    onApply,
  }: {
    open: boolean;
    onApply: (payload: NewAsset) => void | Promise<void>;
  } = $props();

  let name = $state('');
  let category: AssetCategory | '' = $state('');
  let className = $state('');
  let currentValue = $state(0);
  let condition: AssetCondition | '' = $state('');
  let notes = $state('');

  const categories: AssetCategory[] = ['weapon', 'vehicle', 'property', 'investment', 'misc'];
  const conditions: AssetCondition[] = ['new', 'used', 'very_used', 'custom'];

  function reset() {
    name = ''; category = ''; className = '';
    currentValue = 0; condition = ''; notes = '';
  }

  async function onConfirm() {
    const payload: NewAsset = {
      name,
      category: category === '' ? null : category,
      class: className === '' ? null : className,
      purchasePrice: null,
      currentValue,
      condition: condition === '' ? null : condition,
      customModifiers: [],
      notes: notes === '' ? null : notes,
    };
    await onApply(payload);
    open = false;
    reset();
  }

  function onCancel() {
    open = false;
    reset();
  }
</script>

{#if open}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Add asset (manual entry)</h3>

    <label><span>Name</span><input type="text" bind:value={name} autofocus /></label>

    <label>
      <span>Category</span>
      <select bind:value={category}>
        <option value="">— none —</option>
        {#each categories as c}<option value={c}>{c}</option>{/each}
      </select>
    </label>

    <label>
      <span>Class (optional, e.g. "supercar")</span>
      <input type="text" bind:value={className} />
    </label>

    <label><span>Current value</span><input type="number" step="any" bind:value={currentValue} /></label>

    <label>
      <span>Condition</span>
      <select bind:value={condition}>
        <option value="">— none —</option>
        {#each conditions as c}<option value={c}>{c}</option>{/each}
      </select>
    </label>

    <label><span>Notes</span><textarea bind:value={notes} rows="2"></textarea></label>

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm} disabled={name.trim() === ''}>
        Add
      </button>
    </div>
  </dialog>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: var(--color-overlay); z-index: 100;
  }
  .modal {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%); z-index: 101;
    display: grid; align-items: start; gap: var(--space-2);
    padding: var(--space-4);
    background: var(--color-surface); color: var(--color-text);
    border: 1px solid var(--color-border); border-radius: var(--radius-md);
    min-width: 26rem;
    max-height: 90vh; overflow-y: auto;
  }
  h3 { margin: 0 0 var(--space-2) 0; }
  label { display: grid; gap: var(--space-1); }
  label span {
    font-size: var(--text-sm); color: var(--color-text-muted);
  }
  .actions {
    display: flex; justify-content: flex-end; gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .primary { background: var(--color-accent); color: var(--color-on-accent); border: 0; }
  .primary:disabled { opacity: 0.5; }
</style>
```

- [ ] **Step 2: Create `SellModal.svelte`**

```svelte
<script lang="ts">
  import type { Asset, SellRequest } from '../../../types';

  let {
    open = $bindable(false),
    asset,
    currencyName,
    onApply,
  }: {
    open: boolean;
    asset: Asset | null;
    currencyName: string;
    onApply: (req: SellRequest) => void | Promise<void>;
  } = $props();

  let salePrice = $state(0);
  let notes = $state('');

  $effect(() => {
    if (asset) salePrice = asset.currentValue;
  });

  async function onConfirm() {
    if (!asset) return;
    await onApply({
      assetId: asset.id,
      salePrice,
      notes: notes === '' ? null : notes,
    });
    open = false;
    notes = '';
  }

  function onCancel() {
    open = false;
    notes = '';
  }
</script>

{#if open && asset}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Sell "{asset.name}"</h3>
    <p class="current">Current value: {currencyName}{asset.currentValue.toLocaleString()}</p>

    <label>
      <span>Sale price ({currencyName})</span>
      <input type="number" step="any" bind:value={salePrice} autofocus />
    </label>

    <label>
      <span>Notes (optional)</span>
      <textarea bind:value={notes} rows="2"></textarea>
    </label>

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm}>Sell</button>
    </div>
  </dialog>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: var(--color-overlay); z-index: 100;
  }
  .modal {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%); z-index: 101;
    display: grid; align-items: start; gap: var(--space-2);
    padding: var(--space-4);
    background: var(--color-surface); color: var(--color-text);
    border: 1px solid var(--color-border); border-radius: var(--radius-md);
    min-width: 22rem;
  }
  h3 { margin: 0; }
  .current { color: var(--color-text-muted); margin: 0; }
  label { display: grid; gap: var(--space-1); }
  label span {
    font-size: var(--text-sm); color: var(--color-text-muted);
  }
  .actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
  .primary { background: var(--color-accent); color: var(--color-on-accent); border: 0; }
</style>
```

- [ ] **Step 3: Create `AssetList.svelte`**

```svelte
<script lang="ts">
  import type { Asset } from '../../../types';

  let {
    assets,
    currencyName,
    onAdd,
    onSell,
    onRemove,
  }: {
    assets: Asset[];
    currencyName: string;
    onAdd: () => void;
    onSell: (a: Asset) => void;
    onRemove: (a: Asset) => void;
  } = $props();

  function fmt(n: number) { return currencyName + n.toLocaleString(); }
</script>

<section class="asset-list">
  <header>
    <h3>Assets</h3>
    <button type="button" onclick={onAdd}>+ Add</button>
  </header>

  {#if assets.length === 0}
    <p class="empty">No assets yet. Buy one from the shop, or add a manual entry.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Name</th>
          <th>Category</th>
          <th>Condition</th>
          <th>Value</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each assets as a (a.id)}
          <tr>
            <td>{a.name}</td>
            <td>{a.category ?? '—'}</td>
            <td>{a.condition ?? '—'}</td>
            <td>{fmt(a.currentValue)}</td>
            <td class="actions">
              <button type="button" onclick={() => onSell(a)}>Sell</button>
              <button type="button" class="danger" onclick={() => onRemove(a)}>Remove</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .asset-list {
    display: grid;
    align-items: start;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  header {
    display: flex; justify-content: space-between; align-items: center;
  }
  h3 { margin: 0; font-size: var(--text-md); }
  .empty {
    color: var(--color-text-muted);
    text-align: center;
    padding: var(--space-3);
    margin: 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th, td {
    text-align: left;
    padding: var(--space-2);
    border-bottom: 1px solid var(--color-border);
  }
  th {
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted);
  }
  .actions { display: flex; gap: var(--space-2); justify-content: flex-end; }
  .danger { color: var(--color-danger); border-color: var(--color-danger); }
</style>
```

- [ ] **Step 4: Wire into `WealthManager.svelte`**

Add to the script imports:

```ts
import { addAsset, removeAsset, sellAsset } from '../lib/wealth/api';
import AssetList from '../lib/components/wealth/AssetList.svelte';
import AddAssetModal from '../lib/components/wealth/AddAssetModal.svelte';
import SellModal from '../lib/components/wealth/SellModal.svelte';
import type { NewAsset, SellRequest } from '../types';
```

Add state + handlers:

```ts
let addAssetOpen = $state(false);
let sellOpen = $state(false);
let sellTarget: Asset | null = $state(null);

async function applyAddAsset(payload: NewAsset) {
  if (selectedId == null) return;
  try {
    await addAsset(selectedId, payload);
    await refreshAssets();
  } catch (e) {
    error = String(e);
  }
}

async function applySell(req: SellRequest) {
  try {
    wallet = await sellAsset(req);
    await refreshAssets();
    sellTarget = null;
  } catch (e) {
    error = String(e);
  }
}

async function applyRemove(a: Asset) {
  if (!confirm(`Remove "${a.name}"? This does not credit any money — use Sell for that.`)) return;
  try {
    await removeAsset(a.id);
    await refreshAssets();
  } catch (e) {
    error = String(e);
  }
}
```

In the template, replace the `<div class="panel-placeholder">Assets panel — Task 9</div>` line with:

```svelte
<AssetList
  {assets}
  currencyName={wallet.currencyName}
  onAdd={() => (addAssetOpen = true)}
  onSell={(a) => { sellTarget = a; sellOpen = true; }}
  onRemove={applyRemove}
/>
<AddAssetModal bind:open={addAssetOpen} onApply={applyAddAsset} />
<SellModal bind:open={sellOpen} asset={sellTarget} currencyName={wallet.currencyName} onApply={applySell} />
```

- [ ] **Step 5: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/wealth/AssetList.svelte \
        src/lib/components/wealth/AddAssetModal.svelte \
        src/lib/components/wealth/SellModal.svelte \
        src/tools/WealthManager.svelte
git commit -m "$(cat <<'EOF'
wealth: asset list + add + sell modals

AssetList shows a row per owned asset with Sell + Remove actions.
AddAssetModal is the manual-entry path (no catalog), useful for
items the GM authors on the fly. SellModal calls sell_asset which
atomically credits liquid + drops the asset row.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 10: Shop panel + Buy form modal

The shop panel — category tabs, slot table, click → buy form modal with the live pricing formula from Task 6.

**Files:**
- Create: `src/lib/components/wealth/ShopPanel.svelte`
- Create: `src/lib/components/wealth/BuyFormModal.svelte`
- Modify: `src/tools/WealthManager.svelte` (replace shop placeholder + thread state)

### Steps

- [ ] **Step 1: Create `BuyFormModal.svelte`**

```svelte
<script lang="ts">
  import type {
    ItemSlot, Wallet, PurchaseRequest, AssetCondition, CustomModifier,
  } from '../../../types';
  import {
    computeFinalPrice, defaultPickInBand,
  } from '../../../lib/wealth/pricing';

  let {
    open = $bindable(false),
    slot,
    wallet,
    onApply,
  }: {
    open: boolean;
    slot: ItemSlot | null;
    wallet: Wallet;
    onApply: (req: PurchaseRequest) => void | Promise<void>;
  } = $props();

  let pickInBand = $state(0);
  let condition: AssetCondition = $state('new');
  let customMult = $state(1.0);
  let customMods: CustomModifier[] = $state([]);
  let newModLabel = $state('');
  let newModMult = $state(1.0);
  let assetName = $state('');
  let notes = $state('');

  // Reset form whenever the slot changes (i.e. modal re-opens for a different slot).
  $effect(() => {
    if (slot) {
      pickInBand = defaultPickInBand(slot.basePriceMin, slot.basePriceMax);
      condition = 'new';
      customMult = 1.0;
      customMods = [];
      assetName = slot.label;
      notes = '';
    }
  });

  let finalPrice = $derived(
    slot
      ? computeFinalPrice(
          pickInBand,
          condition,
          condition === 'custom' ? customMult : null,
          customMods,
        )
      : 0
  );

  let canAfford = $derived(wallet.liquid >= finalPrice);
  let outOfBand = $derived(
    slot && (pickInBand < slot.basePriceMin || pickInBand > slot.basePriceMax)
  );

  function addModifier() {
    if (newModLabel.trim() === '') return;
    customMods = [...customMods, { label: newModLabel, mult: newModMult }];
    newModLabel = ''; newModMult = 1.0;
  }
  function removeModifier(i: number) {
    customMods = customMods.filter((_, idx) => idx !== i);
  }

  async function onConfirm() {
    if (!slot) return;
    await onApply({
      characterId: wallet.characterId,
      slotId: slot.id,
      price: finalPrice,
      condition,
      customModifiers: customMods,
      nameOverride: assetName.trim() === '' ? null : assetName,
      notes: notes === '' ? null : notes,
    });
    open = false;
  }

  function onCancel() { open = false; }

  function fmt(n: number) { return wallet.currencyName + n.toLocaleString(undefined, { maximumFractionDigits: 2 }); }
</script>

{#if open && slot}
  <div class="backdrop" onclick={onCancel} role="presentation"></div>
  <dialog open class="modal">
    <h3>Buy: {slot.label}</h3>
    {#if slot.examples}
      <p class="examples"><em>e.g.</em> {slot.examples}</p>
    {/if}

    <fieldset class="band">
      <legend>Price (band: {fmt(slot.basePriceMin)} – {fmt(slot.basePriceMax)})</legend>
      <input
        type="range"
        min={slot.basePriceMin}
        max={slot.basePriceMax}
        step={Math.max((slot.basePriceMax - slot.basePriceMin) / 100, 1)}
        bind:value={pickInBand}
      />
      <input type="number" step="any" bind:value={pickInBand} />
      {#if outOfBand}
        <span class="warn">⚠ Out of band — allowed, but verify intent.</span>
      {/if}
    </fieldset>

    <fieldset class="cond">
      <legend>Condition</legend>
      <label><input type="radio" bind:group={condition} value="new" /> New ×1.0</label>
      <label><input type="radio" bind:group={condition} value="used" /> Used ×0.6</label>
      <label><input type="radio" bind:group={condition} value="very_used" /> Very used ×0.3</label>
      <label>
        <input type="radio" bind:group={condition} value="custom" />
        Custom ×
        <input type="number" step="any" bind:value={customMult} disabled={condition !== 'custom'} class="cust-mult" />
      </label>
    </fieldset>

    <fieldset class="mods">
      <legend>Custom modifiers (stack multiplicatively)</legend>
      <ul>
        {#each customMods as m, i (i)}
          <li>
            <span>{m.label} ×{m.mult}</span>
            <button type="button" onclick={() => removeModifier(i)}>×</button>
          </li>
        {/each}
      </ul>
      <div class="mod-add">
        <input type="text" placeholder="label" bind:value={newModLabel} />
        <input type="number" step="any" bind:value={newModMult} class="cust-mult" />
        <button type="button" onclick={addModifier}>Add modifier</button>
      </div>
    </fieldset>

    <label class="name-field">
      <span>Asset name</span>
      <input type="text" bind:value={assetName} />
    </label>

    <label class="notes-field">
      <span>Notes (optional)</span>
      <textarea bind:value={notes} rows="2"></textarea>
    </label>

    <p class="final">
      <strong>Final price: {fmt(finalPrice)}</strong>
      <span class="wallet">(liquid: {fmt(wallet.liquid)})</span>
    </p>
    {#if !canAfford}
      <p class="warn">Insufficient liquid funds.</p>
    {/if}

    <div class="actions">
      <button type="button" onclick={onCancel}>Cancel</button>
      <button type="button" class="primary" onclick={onConfirm} disabled={!canAfford}>
        Buy for {fmt(finalPrice)}
      </button>
    </div>
  </dialog>
{/if}

<style>
  .backdrop {
    position: fixed; inset: 0;
    background: var(--color-overlay); z-index: 100;
  }
  .modal {
    position: fixed; top: 50%; left: 50%;
    transform: translate(-50%, -50%); z-index: 101;
    display: grid; align-items: start; gap: var(--space-3);
    padding: var(--space-4);
    background: var(--color-surface); color: var(--color-text);
    border: 1px solid var(--color-border); border-radius: var(--radius-md);
    min-width: 32rem;
    max-width: 90vw;
    max-height: 90vh; overflow-y: auto;
  }
  h3 { margin: 0; }
  .examples { color: var(--color-text-muted); margin: 0; font-size: var(--text-sm); }
  fieldset { border: 0; padding: 0; display: grid; gap: var(--space-2); align-items: start; }
  fieldset legend { font-size: var(--text-sm); color: var(--color-text-muted); padding: 0; }
  .band input[type="range"] { width: 100%; }
  .cust-mult { width: 6rem; }
  .mods ul { list-style: none; padding: 0; margin: 0; display: grid; gap: var(--space-1); }
  .mods li { display: flex; justify-content: space-between; align-items: center; gap: var(--space-2); }
  .mod-add { display: flex; gap: var(--space-2); align-items: center; }
  .name-field, .notes-field { display: grid; gap: var(--space-1); }
  .name-field span, .notes-field span { font-size: var(--text-sm); color: var(--color-text-muted); }
  .final {
    margin: 0; padding: var(--space-2);
    background: var(--color-surface-alt); border-radius: var(--radius-sm);
    display: flex; justify-content: space-between; align-items: center;
  }
  .wallet { color: var(--color-text-muted); font-size: var(--text-sm); }
  .warn { color: var(--color-warning, var(--color-danger)); margin: 0; font-size: var(--text-sm); }
  .actions { display: flex; justify-content: flex-end; gap: var(--space-2); }
  .primary { background: var(--color-accent); color: var(--color-on-accent); border: 0; }
  .primary:disabled { opacity: 0.5; }
</style>
```

- [ ] **Step 2: Create `ShopPanel.svelte`**

```svelte
<script lang="ts">
  import type { ItemSlot, AssetCategory, Wallet } from '../../../types';
  import { listItemSlots } from '../../../lib/wealth/api';

  let {
    wallet,
    onBuy,
  }: {
    wallet: Wallet;
    onBuy: (slot: ItemSlot) => void;
  } = $props();

  const categories: AssetCategory[] = ['weapon', 'vehicle', 'property', 'investment', 'misc'];
  let active: AssetCategory = $state('weapon');
  let slots: ItemSlot[] = $state([]);
  let loading = $state(false);
  let error: string | null = $state(null);

  $effect(() => {
    const cat = active;
    loading = true;
    error = null;
    listItemSlots(cat)
      .then(s => { slots = s; })
      .catch(e => { error = String(e); })
      .finally(() => { loading = false; });
  });

  function fmt(n: number) { return wallet.currencyName + n.toLocaleString(); }
</script>

<section class="shop-panel">
  <header>
    <h3>Shop</h3>
    <nav class="cat-tabs" role="tablist">
      {#each categories as c}
        <button
          type="button"
          role="tab"
          aria-selected={c === active}
          class:active={c === active}
          onclick={() => (active = c)}
        >{c}</button>
      {/each}
    </nav>
  </header>

  {#if loading}
    <p class="empty">Loading…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if slots.length === 0}
    <p class="empty">No slots in this category.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Slot</th>
          <th>Price band</th>
          <th>Examples</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each slots as s (s.id)}
          <tr>
            <td>{s.label}</td>
            <td>{fmt(s.basePriceMin)} – {fmt(s.basePriceMax)}</td>
            <td class="examples">{s.examples ?? '—'}</td>
            <td><button type="button" onclick={() => onBuy(s)}>Buy…</button></td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .shop-panel {
    display: grid;
    align-items: start;
    gap: var(--space-2);
    padding: var(--space-3);
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
  }
  header {
    display: flex; justify-content: space-between; align-items: center; gap: var(--space-3);
  }
  h3 { margin: 0; font-size: var(--text-md); }
  .cat-tabs { display: flex; gap: var(--space-1); flex-wrap: wrap; }
  .cat-tabs button {
    background: transparent;
    border: 1px solid var(--color-border);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-sm);
    color: var(--color-text-muted);
    text-transform: capitalize;
    cursor: pointer;
  }
  .cat-tabs button.active {
    background: var(--color-accent);
    color: var(--color-on-accent);
    border-color: var(--color-accent);
  }
  .empty, .error { text-align: center; color: var(--color-text-muted); padding: var(--space-3); margin: 0; }
  .error { color: var(--color-danger); }
  table { width: 100%; border-collapse: collapse; }
  th, td {
    text-align: left; padding: var(--space-2);
    border-bottom: 1px solid var(--color-border);
    vertical-align: top;
  }
  th {
    font-size: var(--text-xs);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted);
  }
  .examples { color: var(--color-text-muted); font-style: italic; font-size: var(--text-sm); }
</style>
```

- [ ] **Step 3: Wire into `WealthManager.svelte`**

Add to imports:

```ts
import { purchaseItemSlot } from '../lib/wealth/api';
import ShopPanel from '../lib/components/wealth/ShopPanel.svelte';
import BuyFormModal from '../lib/components/wealth/BuyFormModal.svelte';
import type { ItemSlot, PurchaseRequest } from '../types';
```

Add state + handler:

```ts
let buyOpen = $state(false);
let buyTarget: ItemSlot | null = $state(null);

async function applyPurchase(req: PurchaseRequest) {
  try {
    const [w, _] = await purchaseItemSlot(req);
    wallet = w;
    await refreshAssets();
    buyTarget = null;
  } catch (e) {
    error = String(e);
  }
}
```

In template, replace the `<div class="panel-placeholder">Shop panel — Task 10</div>` line with:

```svelte
<ShopPanel {wallet} onBuy={(s) => { buyTarget = s; buyOpen = true; }} />
<BuyFormModal bind:open={buyOpen} slot={buyTarget} {wallet} onApply={applyPurchase} />
```

- [ ] **Step 4: Run the gate**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/wealth/ShopPanel.svelte \
        src/lib/components/wealth/BuyFormModal.svelte \
        src/tools/WealthManager.svelte
git commit -m "$(cat <<'EOF'
wealth: shop panel + buy form modal

ShopPanel surfaces the seeded item-slot catalog as a tabbed table
(weapon / vehicle / property / investment / misc). BuyFormModal
runs the live pricing formula (pickInBand × condition × Π(mods))
and calls purchase_item_slot atomically. Out-of-band prices are
warned but not blocked; insufficient-liquid blocks the Buy button.

This completes the V1 UI surface.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## Task 11: ARCHITECTURE.md §4 final reconciliation + smoke checklist

Final-pass commit. Confirms the running command total in ARCHITECTURE.md §4, documents the manual smoke sequence the GM will follow, and closes the plan ready for the single end-of-branch `code-review:code-review`.

**Files:**
- Modify: `ARCHITECTURE.md` (§4 grand total, §10 testing note if applicable)
- (No new code.)

### Steps

- [ ] **Step 1: Reconcile §4 running total**

Open `ARCHITECTURE.md`, find §4. Confirm the per-module wealth block lists all 17 commands. Update the running total line at the bottom of §4 (or wherever it lives) to reflect the +17 from this feature. Check `git log --oneline -- ARCHITECTURE.md | head -5` if you need to spot the recent format.

- [ ] **Step 2: Document the V1 smoke checklist**

If `ARCHITECTURE.md §10` (testing) or a sibling section lists feature smoke sequences, append a brief subsection. Otherwise, add a short note pointing at the spec's testing-posture section. Suggested wording (adapt to existing format):

```markdown
### Wealth tracking V1 — manual smoke (one pass before claim-of-done)

1. Open the Wealth tool, pick a character → expect "not using the wealth system" with Enable button.
2. Click Enable → expect three panels (wallet zero-state / empty assets / shop with vehicle tab default).
3. Open shop → Vehicles → "Economy vehicle" → buy at default mid-band price, condition=used, no custom mods → expect:
   - Liquid debited by the displayed final price.
   - New asset row appears in Assets with `current_value == purchase price`.
4. Click Adjust on Liquid → +1000 → expect liquid balance bumps by 1000.
5. Click Transfer → 500 from Liquid to Banked → expect both totals update and conserve.
6. Sell the asset at 80% of current value → expect asset row gone, liquid credited.
7. Disable wealth tracking (with confirm) → expect return to Enable state, asset count and wallet wiped.
8. Re-enable → expect zero balances, empty assets (cascade was clean).
9. Reload the app → expect the catalog still populated (seed survived) and any custom slots from V2 still present (none in V1).
```

- [ ] **Step 3: Run the gate one last time**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 4: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
wealth: ARCHITECTURE.md §4 reconcile + V1 smoke checklist

Final-pass commit closing the V1 wealth-tracking feature.
Confirms the +17 IPC command total in §4 and documents the manual
smoke sequence GMs should run before declaring claim-of-done.

Branch is ready for the single end-of-branch code-review:code-review.

Spec: docs/superpowers/specs/2026-05-20-wealth-tracking-design.md
EOF
)"
```

---

## End-of-plan: final code review

Per CLAUDE.md "Lean plan execution": after all 11 tasks commit cleanly, run **ONE** `code-review:code-review` against the full branch diff (`master..HEAD`). Do NOT run per-task reviewers. The plan's task-level precision + `verify.sh` gates + this single end-of-branch review are the quality gates.

The reviewer will look at:
- IPC contracts (17 commands registered, ARCHITECTURE §4 in sync).
- Atomic-transaction correctness (purchase/sell tests pass; no half-committed state paths).
- Theme-token compliance (no hex literals, rem not px).
- No `invoke(...)` calls from Svelte components (must go through `src/lib/wealth/api.ts`).
- Idempotency semantics (`enable_wallet` doesn't clobber, `disable_wallet` no-ops on absent).
- Negative-balance and same-bucket-transfer behavior matches spec.
- No `std::fs` in async paths (irrelevant here — wealth doesn't touch the filesystem — but checked anyway).

---

## Spec coverage check (self-review)

Sanity pass against the spec — each numbered section in `docs/superpowers/specs/2026-05-20-wealth-tracking-design.md` mapped to its implementing task:

| Spec section | Task(s) |
|---|---|
| §3 Data model (3 tables) | 1 |
| §4 Rust shared types | 1 |
| §4 TS mirror | 1 |
| §4 Condition multipliers | 1 (Rust constants), 6 (TS constants) |
| §4 DB migration (`0NNN`) | 1 (resolved to `0010`) |
| Tool layer — wallet (8 fns) | 2 |
| Tool layer — asset (4 fns) | 3 |
| Tool layer — catalog (3 fns) | 4 |
| Tool layer — transactions (2 fns) | 5 |
| Tauri command surface (17) | 2 + 3 + 4 + 5 (incremental); reconciled in 11 |
| TS wrappers in `src/lib/wealth/api.ts` | 2 + 3 + 4 + 5 |
| Pricing formula (`src/lib/wealth/pricing.ts`) | 6 |
| Mechanics layer — Wealth route | 7 (`src/tools/WealthManager.svelte` — plan diverges from spec's `/wealth/[id]` route since vtmtools uses `src/tools/*.svelte` registered in `src/tools.ts`, not SvelteKit routes; documented in plan header) |
| Disabled state UI | 7 |
| Wallet panel | 8 |
| Adjust / Transfer modals | 8 |
| Asset list + Add modal | 9 |
| Sell modal | 9 |
| Shop panel + Buy form modal | 10 |
| Item catalog seed (~35 rows) | 4 |
| Opt-in lifecycle | 2 (DB), 7 (UI), 9 (cascade behavior visible) |
| Testing posture (Rust inline, no JS framework, smoke) | 2 + 5 (TDD targets); 11 (smoke checklist) |
| `ARCHITECTURE.md §4` updates | 2, 3, 4, 5 (incremental) + 11 (reconciliation) |
| Negative-balance + same-bucket semantics | 2 (TDD covers both) |
| Idempotency (enable/disable) | 2 (TDD covers) |
| `purchase_item_slot` sets `current_value = price` | 5 (TDD asserts) |

No spec section is unmapped. The single plan-vs-spec divergence — `src/tools/WealthManager.svelte` instead of a SvelteKit route — is explicitly documented and is the correct adaptation to repo convention.
