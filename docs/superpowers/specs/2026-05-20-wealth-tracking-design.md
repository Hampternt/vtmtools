# Wealth Tracking — Design Spec

**Date:** 2026-05-20
**Status:** Design — pending user review
**Phase scope:** This spec covers **V1** only. V2 capabilities (named stores, Foundry sync, time-tracker enforcement, income/job engines, asset auto-drift, Resources-advantage baseline-income helper) are designed-for, not built.

## Overview

vtmtools already models character wealth implicitly through the V5 **Resources** advantage — a 1–5 dot Background a player can buy if they want it. Resources is intentionally abstract: one number that handwaves liquid cash, savings, and assets together. Some groups want more granularity (real currency totals, named owned items, distinct bank-vs-cash, named investments). Some groups don't.

This spec introduces a **second, opt-in wealth subsystem** that lives alongside Resources without replacing or auto-deriving from it. The GM enables it per-character (one click). The character then has:

- A **liquid balance** (cash on hand).
- A **banked balance** (stored funds; an optional `transfer_delay` field is stored but not enforced in V1 — it's a forward-compat hook for a future in-game time tracker).
- A **list of owned assets** with rich metadata (name, category, class, purchase price, current value, condition, custom modifiers, notes) — this list IS the "non-liquid" third bucket.

Spending and earning go through small, named **tool-layer primitives** (`adjust_liquid`, `adjust_banked`, `transfer_funds`, asset CRUD). Higher-order mechanics — shopping, gifting, selling, future income/job engines — are *compositions* of these primitives in the mechanics layer; they do not appear in the tool layer. This three-layer split (data → tool → mechanics) is the architectural through-line of the feature and the reason future extensions land cleanly.

V1 ships GM-only, in vtmtools. Foundry-side player visibility is V2.

## Decisions made on the agent's recommendation (flag during review if wrong)

The user explicitly approved per-character opt-in, the three-bucket data shape, the abstract-slot catalog, the manual-only earning model, and the dedicated-route UI placement. The items below are agent defaults inside that envelope — override during spec review if any feels wrong:

- **Opt-in encoding = absence of `character_wallets` row.** No `wealth_enabled` flag column on `characters`. Enabling INSERTs a zeroed row; disabling DELETEs (with confirm) and cascades assets. Sparse-opt-in via row-presence is the cleanest SQLite encoding and mirrors the existing per-character-row patterns elsewhere in the codebase.
- **`currency_name` defaults to `'$'`** but is freeform text. No enum, no currency-conversion math (V1 is single-currency-per-character).
- **`transfer_delay_json` is a nullable TEXT column** holding `{ value: number, unit: 'minutes'|'hours'|'days'|'weeks' } | null`. The field is *stored* and *surfaced in the UI as a hint*, but the V1 tool does not gate transfers on it — transfers complete instantly. The future time-tracker will read this field; no spec change required at that point.
- **Condition is a modifier, not a catalog row.** The four built-in condition tiers (`new` 1.0×, `used` 0.6×, `very_used` 0.3×, `custom` <gm-input>) are a hardcoded TS/Rust constant. The catalog row count stays ~35; adding a new condition tier later is a constant-edit, not a migration.
- **Custom modifiers are an optional JSON list on the asset row** (`custom_modifier_json`), shape `[{ label: string, mult: number }, …]`. Surfaced in the UI as stackable chips. Not normalized into a separate table — they're per-asset flavor, never queried across characters.
- **`pick_in_band` (the GM's choice within a price band) defaults to the band midpoint** with a slider for adjustment. No RNG default. The GM may type any value, even outside the band, and the tool warns but does not block.
- **Purchases are atomic.** `adjust_liquid(-final_price)` + `add_asset(...)` happen in a single SQLite transaction; if either fails, both roll back. Implemented as a single Tauri command `purchase_item_slot` that calls into the tool layer.
- **`item_slots` are seeded in `src-tauri/src/db/seed.rs`** alongside Advantages, following the existing `is_custom = 0` for built-ins / `is_custom = 1` for user-added pattern. Custom rows survive reseeds; the destructive-reseed of built-ins (per ADR 0002) applies normally.
- **The Wealth route is mounted at `/wealth/[character_id]`** as a top-level route, not nested under `/character/[id]/...`. This matches the existing `Domains Manager` / `Advantages Library` routing posture (top-level tool routes, character context passed as a param) rather than the per-character editor patterns.
- **No bridge-layer changes in V1.** The wallet/asset data is not added to `BridgeCharacter` or `SavedCharacter` shapes. V2's Foundry sync will add it then; until then, wealth state is purely vtmtools-side and survives saves only via direct SQLite reads (which is fine — there's no save/load step for this data; SQLite is the source of truth).
- **Negative balances are allowed.** `adjust_liquid(c, -1000)` succeeds even if it pushes liquid below zero. The GM may want to model debt, loans, or overspends narratively. The shop's buy form is the *only* path that gates affordability (the [Buy] button is disabled when `liquid < final_price`); every other primitive accepts any signed delta without validation. This keeps the tool layer dumb and the policy in the mechanics layer where it belongs.
- **`transfer_funds(c, X, X, n)` is a hard error**, not a no-op. Same-bucket transfers are almost certainly bugs in calling code; failing loudly surfaces them. Returns `Err(WealthError::SameBucketTransfer)`.
- **`transfer_funds` with `amount > balance[from]` is allowed** (consistent with negative balances above). The bucket the GM transfers from goes negative; transfers are not affordability-gated at the tool layer.
- **`enable_wallet` on an already-enabled character is idempotent.** Implementation: `INSERT OR IGNORE` then `SELECT`. Returns the existing wallet untouched (no clobbering of balances or assets). No error.
- **`disable_wallet` on a non-existent wallet is a no-op.** Returns `Ok(())` without doing anything. No error. Removes a foot-gun for callers that double-disable.
- **`purchase_item_slot` initializes `current_value = price`** on the newly-created asset row (where `price` is the GM's final price after band + condition + custom modifiers). Without this, freshly-bought assets would show up worth $0 by default — the schema's `current_value REAL NOT NULL DEFAULT 0` is for *manually-added* assets where the GM may not know the value yet, not for purchases.

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  MECHANICS LAYER  (Svelte: route + components)               │
│  /wealth/[character_id]                                      │
│    ├─ Wallet panel  (liquid / banked / transfer)             │
│    ├─ Asset list    (add / edit / remove / sell)             │
│    └─ Shop panel    (category filter → slot row → buy modal) │
│  Future mechanics (out of scope V1):                         │
│    income engines · job functions · named-store catalogs ·   │
│    Foundry actor.* writebacks · time-tracker enforcement     │
└────────────────────────────┬─────────────────────────────────┘
                             │ calls typed wrappers
┌────────────────────────────▼─────────────────────────────────┐
│  TOOL LAYER  (typed wrappers ↔ Tauri commands ↔ Rust DB)     │
│  src/lib/wealth/api.ts   ←→   src-tauri/src/db/wealth/       │
│    ├─ wallet.rs       enable/disable/get/adjust/transfer     │
│    ├─ asset.rs        add/update/remove/list                 │
│    ├─ catalog.rs      list_slots/add_custom_slot             │
│    └─ transactions.rs atomic purchase + atomic sell          │
└────────────────────────────┬─────────────────────────────────┘
                             │ SQL (sqlx, sqlite)
┌────────────────────────────▼─────────────────────────────────┐
│  DATA LAYER  (SQLite, new migration `0NNN_add_wealth.sql`)   │
│    character_wallets   (1:1 with characters, opt-in by row)  │
│    character_assets    (1:N with character_wallets)          │
│    item_slots          (global, seeded catalog)              │
└──────────────────────────────────────────────────────────────┘
```

The three layers cleanly mirror the existing pattern of, e.g., `domains` (`src/lib/domains/` ↔ `src-tauri/src/db/domain*.rs`) and `advantages` (`src/lib/library/api.ts` ↔ `src-tauri/src/db/advantage*.rs`). New mechanics drop into the mechanics layer without touching the tool layer or the data layer.

---

## Domain shape changes

### Rust (`src-tauri/src/shared/wealth.rs` — new module)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Bucket {
    Liquid,
    Banked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferDelay {
    pub value: f64,
    pub unit: TimeUnit,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Weapon,
    Vehicle,
    Property,
    Investment,
    Misc,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetCondition {
    New,
    Used,
    VeryUsed,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomModifier {
    pub label: String,
    pub mult: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: i64,
    pub character_id: i64,
    pub name: String,
    pub category: Option<AssetCategory>,
    pub class: Option<String>,           // slot reference (e.g., "economy")
    pub purchase_price: Option<f64>,
    pub current_value: f64,
    pub condition: Option<AssetCondition>,
    pub custom_modifiers: Vec<CustomModifier>,
    pub notes: Option<String>,
    pub created_at: String,
}

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
```

The `Default` and `serde(default)` patterns from the project's modifier shape (`src-tauri/src/shared/modifier.rs`) apply where partial payloads might land — though V1 has no legacy migration burden, so this is mostly forward-looking.

### TS mirror (`src/types.ts`)

```ts
// Wealth subsystem
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
```

### Condition multipliers (`src-tauri/src/shared/wealth.rs` + TS mirror)

A hardcoded constant table, not a DB-backed lookup. Adding a tier in the future is a constant edit and a release; not a migration.

```rust
pub const CONDITION_MULTIPLIER_NEW: f64 = 1.0;
pub const CONDITION_MULTIPLIER_USED: f64 = 0.6;
pub const CONDITION_MULTIPLIER_VERY_USED: f64 = 0.3;
// Custom is GM-supplied at purchase time; not a constant.
```

### DB migration (`src-tauri/migrations/0NNN_add_wealth.sql`)

```sql
-- Per-character wallet. Row presence == opted in to wealth tracking.
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

-- Owned assets (the "non-liquid" third bucket as a list of objects).
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

-- Global abstract catalog. ~35 seeded rows; user-extensible.
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

Migration number `0NNN` is allocated at plan time (look at the highest existing migration). The destructive-reseed path in `src-tauri/src/db/seed.rs` (per ADR 0002) will receive the new `item_slots` seed pass; user-custom rows (`is_custom = 1`) are preserved across reseeds per the existing pattern. `character_wallets` and `character_assets` are user data and are never reseeded.

---

## Tool layer — function inventory

Each function is one Rust function in `src-tauri/src/db/wealth/` AND one `#[tauri::command]` registered in `src-tauri/src/lib.rs` (per CLAUDE.md §4 — failing to update §4's inventory in the same commit is a documented hazard). Each command gets one typed wrapper in `src/lib/wealth/api.ts`.

### Wallet (`src-tauri/src/db/wealth/wallet.rs`)

```rust
pub fn enable_wallet(pool, character_id: i64) -> Result<Wallet>;
pub fn disable_wallet(pool, character_id: i64) -> Result<()>;
pub fn get_wallet(pool, character_id: i64) -> Result<Option<Wallet>>;

pub fn adjust_liquid(pool, character_id: i64, delta: f64) -> Result<Wallet>;
pub fn adjust_banked(pool, character_id: i64, delta: f64) -> Result<Wallet>;
pub fn transfer_funds(pool, character_id: i64,
                      from: Bucket, to: Bucket, amount: f64) -> Result<Wallet>;
pub fn set_currency_name(pool, character_id: i64, name: &str) -> Result<Wallet>;
pub fn set_transfer_delay(pool, character_id: i64,
                          delay: Option<TransferDelay>) -> Result<Wallet>;
```

### Asset (`src-tauri/src/db/wealth/asset.rs`)

```rust
pub struct NewAsset { /* required: character_id, name, current_value; optional rest */ }
pub struct AssetPatch { /* all fields Option<…> */ }

pub fn add_asset(pool, character_id: i64, payload: NewAsset) -> Result<Asset>;
pub fn update_asset(pool, asset_id: i64, patch: AssetPatch) -> Result<Asset>;
pub fn remove_asset(pool, asset_id: i64) -> Result<()>;
pub fn list_assets(pool, character_id: i64) -> Result<Vec<Asset>>;
```

### Catalog (`src-tauri/src/db/wealth/catalog.rs`)

```rust
pub fn list_item_slots(pool, category: Option<AssetCategory>) -> Result<Vec<ItemSlot>>;
pub fn add_custom_slot(pool, payload: NewItemSlot) -> Result<ItemSlot>;
pub fn remove_custom_slot(pool, slot_id: i64) -> Result<()>;
```

### Composed (`src-tauri/src/db/wealth/transactions.rs`)

```rust
pub struct PurchaseRequest {
    pub character_id: i64,
    pub slot_id: i64,
    pub price: f64,                       // final price chosen by GM
    pub condition: AssetCondition,
    pub custom_modifiers: Vec<CustomModifier>,
    pub name_override: Option<String>,    // GM-narrated name for the asset
    pub notes: Option<String>,
}

pub struct SellRequest {
    pub asset_id: i64,
    pub sale_price: f64,                  // GM-chosen sale price (defaults in UI to current_value)
    pub notes: Option<String>,            // optional ledger note for the wallet adjustment
}

/// Atomic: adjust_liquid(-price) + add_asset(...) with current_value = price
/// in one transaction. On any failure, both roll back.
pub fn purchase_item_slot(pool, req: PurchaseRequest) -> Result<(Wallet, Asset)>;

/// Atomic: adjust_liquid(+sale_price) + remove_asset(asset_id) in one transaction.
/// On any failure, both roll back. Resolves character_id from the asset row.
pub fn sell_asset(pool, req: SellRequest) -> Result<Wallet>;
```

Purchase and sell are intentionally **symmetric** atomic primitives. Both live at the tool layer because cross-table mutation needs a single SQLite transaction to guarantee no half-finished state, and Tauri commands are not a transactional boundary across calls — meaning the mechanics layer cannot reliably compose `adjust_liquid` + `remove_asset` itself.

### Tauri command surface

```
wealth_enable_wallet         wealth_disable_wallet         wealth_get_wallet
wealth_adjust_liquid         wealth_adjust_banked          wealth_transfer_funds
wealth_set_currency_name     wealth_set_transfer_delay
wealth_add_asset             wealth_update_asset           wealth_remove_asset
wealth_list_assets
wealth_list_item_slots       wealth_add_custom_slot        wealth_remove_custom_slot
wealth_purchase_item_slot    wealth_sell_asset
```

17 commands. Every command registered in `invoke_handler` in the same commit it lands in `lib.rs`. `ARCHITECTURE.md §4` IPC inventory updated in the same commit — both the module's per-file list AND the running total. (See CLAUDE.md "Claude-specific rules".)

### Typed TS wrappers (`src/lib/wealth/api.ts`)

```ts
export async function enableWallet(characterId: number): Promise<Wallet>;
export async function disableWallet(characterId: number): Promise<void>;
export async function getWallet(characterId: number): Promise<Wallet | null>;

export async function adjustLiquid(characterId: number, delta: number): Promise<Wallet>;
export async function adjustBanked(characterId: number, delta: number): Promise<Wallet>;
export async function transferFunds(characterId: number,
                                    from: Bucket, to: Bucket, amount: number): Promise<Wallet>;
export async function setCurrencyName(characterId: number, name: string): Promise<Wallet>;
export async function setTransferDelay(characterId: number,
                                       delay: TransferDelay | null): Promise<Wallet>;

export async function addAsset(characterId: number, payload: NewAsset): Promise<Asset>;
export async function updateAsset(assetId: number, patch: AssetPatch): Promise<Asset>;
export async function removeAsset(assetId: number): Promise<void>;
export async function listAssets(characterId: number): Promise<Asset[]>;

export async function listItemSlots(category?: AssetCategory): Promise<ItemSlot[]>;
export async function addCustomSlot(payload: NewItemSlot): Promise<ItemSlot>;
export async function removeCustomSlot(slotId: number): Promise<void>;

export async function purchaseItemSlot(req: PurchaseRequest): Promise<{ wallet: Wallet; asset: Asset }>;
export async function sellAsset(req: SellRequest): Promise<Wallet>;
```

No Svelte component invokes `invoke(...)` directly — every IPC goes through `src/lib/wealth/api.ts` (per CLAUDE.md "Claude-specific rules").

---

## Mechanics layer — UI & flows

### Route

`src/routes/wealth/[characterId]/+page.svelte` (or whichever exact path matches the existing top-level-tool routing convention; the plan task will verify).

### Disabled state (no `character_wallets` row)

A single card centered on the route:

```
This character is not using the wealth system.
[ Enable wealth tracking ]
```

Click → `enableWallet(characterId)` → page re-renders with the enabled state below.

### Enabled state — three panels stacked

```
┌─ WEALTH — <Character Name> ────────────────────────────────────┐
│ Currency: [ $ ]   Transfer delay: [ instant ▾ ]   [⚙ Disable] │
├────────────────────────────────────────────────────────────────┤
│  LIQUID                  BANKED                  ASSETS Σ      │
│  $4,500                  $32,000                 $145,000      │
│  [+] [−] [edit]          [+] [−] [→ Transfer] [edit]           │
├────────────────────────────────────────────────────────────────┤
│  ASSETS                                              [+ Add]   │
│  ● 2019 Sedan          used         $12,000         [⋯ Sell]   │
│  ● Downtown Loft       new          $130,000        [⋯ Sell]   │
│  ● Vintage Watch       custom       $3,000          [⋯ Sell]   │
├────────────────────────────────────────────────────────────────┤
│  SHOP                                                          │
│  Category: ( Weapons │ Vehicles │ Property │ Asset │ Misc )    │
│  ─────────────────────────────────────────────────────────     │
│  Slot           Price band       Action                        │
│  Economy        $5,000–$15,000   [ Open buy form ]             │
│  Mid-range      $25,000–$60,000  [ Open buy form ]             │
│  …                                                             │
└────────────────────────────────────────────────────────────────┘
```

### Modals

- **Adjust modal** (`+` / `−` / `edit` on liquid or banked) — numeric input, signed delta, optional note (note is V2; V1 just stores the delta).
- **Transfer modal** — amount + direction (`liquid → banked` or `banked → liquid`). Shows the `transfer_delay` hint if set, but executes instantly in V1.
- **Add Asset modal** — name, category, class (optional), current value, condition, custom modifiers (stackable chips), notes. No slot required (manual entry path for items not in the catalog).
- **Sell modal** — asks for sale price (defaults to `current_value`), then calls `sellAsset({asset_id, sale_price})` which performs the atomic `adjust_liquid(+price) + remove_asset(id)` in a single Rust transaction.
- **Buy form** — opens when clicking a shop row. Fields:
  - **Slot**: pre-filled (e.g., "Economy" vehicle).
  - **Final price**: slider between `base_price_min` and `base_price_max`, default midpoint. GM may type any value.
  - **Condition**: dropdown (new / used / very_used / custom). If custom, second input asks for the multiplier directly.
  - **Custom modifiers**: list of `{label, mult}` chips, [+ Add modifier] button.
  - **Asset name**: free text, default to the slot label (GM types "2019 Honda Civic" or whatever).
  - **Notes**: free text.
  - Computed: `final = pick_in_band × condition_mult × Π(custom_mods)`. Shown live.
  - [ Buy for $<final> ] button → `purchaseItemSlot({...})`. Disabled if `liquid < final`.

### Per-character entry point

V1 ships *no* character-card affordance for the Wealth route. The GM navigates via the top-level tools route (sidebar / route picker). A character-card "Wealth" chip is a small follow-up but explicitly out of scope here to keep the V1 surface tight.

---

## Pricing formula

```
final_price = pick_in_band(base_price_min, base_price_max)
            × condition_multiplier
            × Π(custom_modifiers[i].mult)
```

- `pick_in_band` defaults to `(min + max) / 2`; GM-adjustable via slider; GM may type out-of-band values (UI warns, does not block).
- `condition_multiplier`: lookup against the hardcoded constant table; `custom` reads a GM-supplied multiplier instead.
- `custom_modifiers`: empty by default; multiplicative product when present. Each is a `{label: string, mult: number}` pair; the label is stored verbatim on the resulting asset for audit.
- Final number is rounded to the nearest currency unit (default: 1.0, i.e. whole dollars; V2 can introduce per-currency rounding rules if needed).

The formula lives in **one place** — the buy-form computation function in `src/lib/wealth/pricing.ts`. The Rust `purchase_item_slot` command takes the *already-computed* `price: f64` as input; it does not re-compute. This keeps pricing visible to the GM at decision time and avoids hidden divergence between UI display and DB write.

---

## Item catalog seed (~35 rows)

Lives in `src-tauri/src/db/seed.rs` alongside the Advantages seed. Final list will be detailed in the implementation plan; the rough composition:

| Category  | Class examples                                                                  | Count |
|-----------|----------------------------------------------------------------------------------|-------|
| weapon    | improvised, knife, sword, cheap_handgun, military_handgun, long_gun, heavy       | ~7    |
| vehicle   | economy, mid_range, high_end, supercar, motorcycle, utility                      | ~6    |
| property  | rented_apt, owned_apt, house, mansion, safehouse, commercial                     | ~6    |
| investment| small_investment, medium_investment, business_share, art_piece                   | ~4    |
| misc      | small_luxury, medium_luxury, status_symbol, service_short, service_long, consumable | ~6 |

Price bands are GM-tunable post-seed (V2 may surface an "edit slot" UI; V1 only allows adding/removing user-custom slots, not editing built-ins). Built-ins are seeded with `source = 'V5 default catalog'`; user-customs with `source = NULL`.

---

## Opt-in lifecycle

```
[ no character_wallets row ]
        │
        │  Click "Enable wealth tracking"
        ▼
[ character_wallets row exists, liquid=0, banked=0, no assets ]
        │
        │  GM uses the system
        ▼
[ wallet + N assets ]
        │
        │  Click "Disable" (with confirm() dialog)
        ▼
[ row deleted, ON DELETE CASCADE drops all character_assets rows ]
```

Disable is **destructive** in V1 to keep the data model honest (row absence is the source of truth for "opted in"). A future "soft disable" (preserve data, hide from UI) can be added by introducing an `is_active` flag on the wallet row without disturbing any other shape. Confirm dialog uses the existing `confirm()` pattern (e.g., `CharacterRow.svelte` reset-card flow).

---

## Foundry bridge (V2 — designed-for, not built)

V1 ships no Foundry sync. When V2 is designed, it will compose `actor.*` helpers per the **foundry-helper-library roadmap** (`docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md`). Likely shape:

- One `actor.set_field` per scalar (`flags.vtmtools.wealth.liquid`, `flags.vtmtools.wealth.banked`, `flags.vtmtools.wealth.currency_name`).
- A serialized assets list on `flags.vtmtools.wealth.assets` (an array of asset objects).
- A read-only journal page or in-sheet panel rendered from those flags so players see their own wealth in Foundry.

Because the V1 data model is fully relational and field-addressable, V2 will be a mechanical mapping job — no V1 schema changes anticipated. The `transfer_delay` field, once the time-tracker exists, will be enforced at the mechanics layer (or in a helper above the tool layer); the tool layer's `transfer_funds` stays the same.

---

## Out of scope (V1)

- **Income tick automation** — mechanics-layer concern; composes `adjust_liquid` in a future loop.
- **Time-tracker enforcement** — `transfer_delay_json` is stored, not gated.
- **Inter-character money flow primitives** — `gift(a, b, amount) = adjust_liquid(a, -x) + adjust_liquid(b, +x)`; lives in mechanics layer when needed.
- **Named per-campaign stores** — the catalog is global in V1. Per-campaign stores layer on top by adding a `stores` table + `store_inventory` join table later.
- **Asset auto-drift / depreciation** — `current_value` is GM-edited only.
- **Resources-advantage baseline-income helper** — deferred to V2. User explicitly wanted this layered on top of manual entry ("mix of 4 and manual" during brainstorm — option 4 = the Resources-dot helper). It will land as a mechanics-layer composition that reads the character's Resources dot rating and applies `adjust_liquid(c, dots × per_dot_baseline)`. V1 ships only the underlying primitive; no auto-derive runs yet.
- **Foundry sync** — V2 (above).
- **Coterie shared purse** — interesting future feature; would be a separate "coterie_wallets" table with similar shape, not part of V1.
- **Currency conversion** — `currency_name` is a label only; no math between currencies.
- **Bridge / Save payload inclusion** — wallet/asset data is not added to `BridgeCharacter` / `SavedCharacter` in V1; SQLite is the source of truth and no save/load flow exists for this data yet.

---

## Testing posture

Rust unit tests live in `src-tauri/src/db/wealth/{wallet,asset,catalog,purchase}.rs` (or sibling `*_test.rs` if that matches repo convention; plan task will confirm). Coverage targets:

- **Wallet arithmetic** — `adjust_liquid` / `adjust_banked` with positive, negative, and zero deltas; `f64` precision sanity at large magnitudes.
- **Transfer conservation** — `transfer_funds(a, liquid, banked, x)` results in `liquid' = liquid - x, banked' = banked + x` and total invariant `liquid' + banked' == liquid + banked` (within `f64` epsilon).
- **Lifecycle cascade** — enable → add asset → disable → assets table contains 0 rows for that character.
- **Catalog filter** — `list_item_slots(Some(Vehicle))` returns only vehicles.
- **Purchase atomicity** — induce a failure in `add_asset` post-`adjust_liquid` and confirm the transaction rolls back (wallet unchanged).
- **Custom slot uniqueness** — `add_custom_slot` rejects duplicate `(category, class)` pairs per the schema's `UNIQUE` constraint.

Frontend: `npm run check` is the type-gate. Manual smoke-test of the wealth route (enable → shop a vehicle → see asset → sell asset → disable) is required before claim-of-done. No new Svelte unit tests in V1 (existing project posture is type-check + manual smoke; per CLAUDE.md "tests on demand").

`./scripts/verify.sh` is the commit gate for every plan task.

---

## Modularity & forward-compat checklist

Per user emphasis: the V1 must be a *foundation* future capabilities slot into cleanly. The following seams are deliberate:

1. **Three-layer split** (data ↔ tool ↔ mechanics) — new mechanics never touch the data layer or the tool layer's surface; they compose primitives.
2. **`transfer_delay_json` forward-compat field** — defined now, enforced by a future time-tracker without schema change.
3. **`custom_modifier_json` on assets** — extensible per-asset metadata without column churn.
4. **`is_custom` flag on `item_slots`** — V2 can add an admin UI for editing built-ins without changing the table.
5. **`Bucket` enum** — adding more buckets ("vault", "offshore") later means one enum variant + one constraint update, not a UI rewrite.
6. **`AssetCategory` enum** — same; adding "magical_item" etc. is a one-line change.
7. **No bridge-payload coupling in V1** — V2 Foundry sync adds fields to `BridgeCharacter` then; no retroactive churn on V1 saves.
8. **`purchase_item_slot` as a composed primitive** — proves the "mechanics compose tool functions" pattern; future composites (`sell_asset`, `gift_funds`, `tick_income`) follow the same shape.
9. **Pricing formula in `src/lib/wealth/pricing.ts`** — pure function, easy to unit-test, easy to swap (e.g., for an alt-RPG-system pricing rule) without touching DB or Rust.
10. **Top-level route** — UI re-mounts elsewhere (character card, GM Screen) without touching the route itself; the route is a thin shell over typed wrappers.

---

## Glossary

- **Wallet** — the `character_wallets` row for one character.
- **Liquid** — cash on hand. Instantly usable.
- **Banked** — stored funds. Mechanically the same as liquid in V1; semantically distinct; has an optional `transfer_delay` hint.
- **Asset** — one row in `character_assets`. The third "non-liquid" bucket is the *list* of these.
- **Item slot** — one row in `item_slots`. An abstract catalog entry (category × class + price band). Not a specific SKU.
- **Class** — within-category subtype (e.g., `economy`, `supercar`, `small_luxury`).
- **Condition** — a multiplier tier applied at purchase time. Stored on the resulting asset.
- **Custom modifier** — a `{label, mult}` adjustment chip applied at purchase or edited on an asset.
- **Pick-in-band** — GM's chosen price within the slot's `(min, max)` band.
- **Final price** — `pick_in_band × condition × Π(custom_mods)`. What the GM actually pays.
- **Bucket** — `Liquid | Banked`. Used by `transfer_funds`.

---

## Open questions for review

None blocking. The agent-default decisions section above lists items the user did not explicitly approve; flag any during review.
