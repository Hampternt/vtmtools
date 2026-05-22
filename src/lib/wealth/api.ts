// Typed Tauri-IPC wrappers for the wealth-tracking subsystem (V1 wallet
// primitives). Frontend components must call these instead of `invoke(...)`
// directly — see ARCHITECTURE.md §4 ("Typed frontend API wrapper modules").
//
// Tauri 2 auto-converts camelCase JS keys to snake_case Rust params, so
// e.g. `characterId` on this side becomes `character_id` on the Rust side.
// Returned shapes match the Rust types in src-tauri/src/shared/wealth.rs.

import { invoke } from '@tauri-apps/api/core';
import type { Bucket, TransferDelay, Wallet } from '../../types';

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
