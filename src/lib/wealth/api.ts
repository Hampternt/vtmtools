// Typed Tauri-IPC wrappers for the wealth-tracking subsystem (V1 wallet
// primitives). Frontend components must call these instead of `invoke(...)`
// directly — see ARCHITECTURE.md §4 ("Typed frontend API wrapper modules").
//
// Tauri 2 auto-converts camelCase JS keys to snake_case Rust params, so
// e.g. `characterId` on this side becomes `character_id` on the Rust side.
// Returned shapes match the Rust types in src-tauri/src/shared/wealth.rs.

import { invoke } from '@tauri-apps/api/core';
import type {
  Asset,
  AssetCategory,
  AssetPatch,
  Bucket,
  ItemSlot,
  NewAsset,
  NewItemSlot,
  PurchaseRequest,
  SellRequest,
  TransferDelay,
  Wallet,
} from '../../types';

// ─── Wallet ────────────────────────────────────────────────────────────────

/** Opt a character into wealth tracking. Idempotent — re-enabling does not
 *  clobber existing balances or settings. */
export function enableWallet(characterId: number): Promise<Wallet> {
  return invoke<Wallet>('wealth_enable_wallet', { characterId });
}

/** Delete the wallet (cascades to all assets). No-op when the wallet
 *  is already absent. */
export function disableWallet(characterId: number): Promise<void> {
  return invoke<void>('wealth_disable_wallet', { characterId });
}

/** Read the wallet, returning `null` when the character has not opted in. */
export function getWallet(characterId: number): Promise<Wallet | null> {
  return invoke<Wallet | null>('wealth_get_wallet', { characterId });
}

/** Apply a signed delta to the liquid bucket. Negatives allowed (debt). */
export function adjustLiquid(characterId: number, delta: number): Promise<Wallet> {
  return invoke<Wallet>('wealth_adjust_liquid', { characterId, delta });
}

/** Apply a signed delta to the banked bucket. Negatives allowed (debt). */
export function adjustBanked(characterId: number, delta: number): Promise<Wallet> {
  return invoke<Wallet>('wealth_adjust_banked', { characterId, delta });
}

/** Atomically move `amount` from `from` to `to`. Rejects same-bucket;
 *  overdraw is permitted per spec. */
export function transferFunds(
  characterId: number,
  from: Bucket,
  to: Bucket,
  amount: number,
): Promise<Wallet> {
  return invoke<Wallet>('wealth_transfer_funds', { characterId, from, to, amount });
}

/** Update the wallet's display currency label (e.g. "$", "£", "🜚"). */
export function setCurrencyName(characterId: number, name: string): Promise<Wallet> {
  return invoke<Wallet>('wealth_set_currency_name', { characterId, name });
}

/** Set or clear the optional transfer-delay hint (V2 timed-transfer
 *  scaffolding — stored but not enforced in V1). */
export function setTransferDelay(
  characterId: number,
  delay: TransferDelay | null,
): Promise<Wallet> {
  return invoke<Wallet>('wealth_set_transfer_delay', { characterId, delay });
}

// ─── Assets ────────────────────────────────────────────────────────────────

/** Insert a new asset for an opted-in character. Rejects when the
 *  character has no wallet row (must call `enableWallet` first). */
export function addAsset(characterId: number, payload: NewAsset): Promise<Asset> {
  return invoke<Asset>('wealth_add_asset', { characterId, payload });
}

/** Patch arbitrary fields on an existing asset. `AssetPatch` uses
 *  `field: undefined` to mean "no change" and `field: null` to mean
 *  "clear" (for nullable fields). */
export function updateAsset(assetId: number, patch: AssetPatch): Promise<Asset> {
  return invoke<Asset>('wealth_update_asset', { assetId, patch });
}

/** Delete a single asset row. */
export function removeAsset(assetId: number): Promise<void> {
  return invoke<void>('wealth_remove_asset', { assetId });
}

/** All assets owned by `characterId`, oldest-first by creation. */
export function listAssets(characterId: number): Promise<Asset[]> {
  return invoke<Asset[]>('wealth_list_assets', { characterId });
}

// ─── Catalog ───────────────────────────────────────────────────────────────

/** List item-slot catalog rows. With no `category` argument, returns the
 *  whole catalog. Filtered list is ordered by `(base_price_min, label)`. */
export function listItemSlots(category?: AssetCategory): Promise<ItemSlot[]> {
  return invoke<ItemSlot[]>('wealth_list_item_slots', { category: category ?? null });
}

/** Insert a user-authored catalog row (`is_custom = 1`). Built-ins seeded
 *  on startup survive in the same table; custom rows persist across
 *  reseeds per ADR 0002. */
export function addCustomSlot(payload: NewItemSlot): Promise<ItemSlot> {
  return invoke<ItemSlot>('wealth_add_custom_slot', { payload });
}

/** Delete a custom catalog row. Refuses to delete built-ins (surfaces the
 *  same "not found" error). */
export function removeCustomSlot(slotId: number): Promise<void> {
  return invoke<void>('wealth_remove_custom_slot', { slotId });
}

// ─── Composed transactions ────────────────────────────────────────────────

/** Atomic purchase: debit `liquid` by `req.price` and insert an asset
 *  derived from the catalog slot. `current_value` initializes to the
 *  purchase price per spec. Either both mutations land or neither does. */
export function purchaseItemSlot(req: PurchaseRequest): Promise<[Wallet, Asset]> {
  return invoke<[Wallet, Asset]>('wealth_purchase_item_slot', { req });
}

/** Atomic sale: credit the owning character's `liquid` by `req.salePrice`
 *  and delete the asset row. The character_id is resolved server-side
 *  from the asset row. */
export function sellAsset(req: SellRequest): Promise<Wallet> {
  return invoke<Wallet>('wealth_sell_asset', { req });
}
