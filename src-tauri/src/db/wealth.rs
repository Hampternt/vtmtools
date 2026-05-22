//! Wealth-tracking DB module (V1).
//!
//! See `docs/superpowers/specs/2026-05-20-wealth-tracking-design.md`.
//!
//! Pattern note: this is the per-domain single-file convention used by
//! `advantage.rs` (1221 lines), `modifier.rs` (946), `saved_character.rs`
//! (1046). The spec's nested-module proposal is overruled to follow repo
//! pattern.
//!
//! Error convention: `Result<T, String>` with `format!("db/wealth.fn: {}", e)`
//! prefixes — matches the rest of `src-tauri/src/db/**`. No `thiserror` /
//! `anyhow` deps in this repo (verified against Cargo.toml).

use crate::shared::wealth::*;
use sqlx::SqlitePool;

// -------------------------------------------------------------------------
// JSON helpers
// -------------------------------------------------------------------------

fn serialize_transfer_delay(d: &TransferDelay) -> Result<String, String> {
    serde_json::to_string(d).map_err(|e| format!("db/wealth.serialize_transfer_delay: {}", e))
}

fn deserialize_transfer_delay(s: &str) -> Result<TransferDelay, String> {
    serde_json::from_str(s).map_err(|e| format!("db/wealth.deserialize_transfer_delay: {}", e))
}

// -------------------------------------------------------------------------
// Row decoder
// -------------------------------------------------------------------------

fn build_wallet(
    character_id: i64,
    liquid: f64,
    banked: f64,
    currency_name: String,
    delay_json: Option<String>,
    created_at: String,
    updated_at: String,
) -> Result<Wallet, String> {
    let transfer_delay = match delay_json {
        Some(s) if !s.is_empty() => Some(deserialize_transfer_delay(&s)?),
        _ => None,
    };
    Ok(Wallet {
        character_id,
        liquid,
        banked,
        currency_name,
        transfer_delay,
        created_at,
        updated_at,
    })
}

// -------------------------------------------------------------------------
// Wallet primitives
// -------------------------------------------------------------------------

/// Enable wealth tracking for `character_id`. Idempotent: if a wallet row
/// already exists for the character, returns it untouched (balances and
/// settings preserved). Per spec: row presence == opted-in.
pub async fn enable_wallet(pool: &SqlitePool, character_id: i64) -> Result<Wallet, String> {
    sqlx::query("INSERT OR IGNORE INTO character_wallets (character_id) VALUES (?)")
        .bind(character_id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/wealth.enable_wallet: {}", e))?;

    get_wallet(pool, character_id)
        .await?
        .ok_or_else(|| format!("db/wealth.enable_wallet: wallet not found for character {}", character_id))
}

/// Delete the wallet for `character_id`. Cascades to character_assets via
/// the FK in 0012_add_wealth.sql. Idempotent: no-op when the wallet is
/// absent (returns `Ok(())`).
pub async fn disable_wallet(pool: &SqlitePool, character_id: i64) -> Result<(), String> {
    sqlx::query("DELETE FROM character_wallets WHERE character_id = ?")
        .bind(character_id)
        .execute(pool)
        .await
        .map_err(|e| format!("db/wealth.disable_wallet: {}", e))?;
    Ok(())
}

/// Return the wallet for `character_id`, or `None` when not opted in.
pub async fn get_wallet(pool: &SqlitePool, character_id: i64) -> Result<Option<Wallet>, String> {
    let row: Option<(i64, f64, f64, String, Option<String>, String, String)> = sqlx::query_as(
        "SELECT character_id, liquid, banked, currency_name, transfer_delay_json, created_at, updated_at
           FROM character_wallets
          WHERE character_id = ?",
    )
    .bind(character_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("db/wealth.get_wallet: {}", e))?;

    match row {
        Some((id, liquid, banked, cur, delay_json, created, updated)) => {
            Ok(Some(build_wallet(id, liquid, banked, cur, delay_json, created, updated)?))
        }
        None => Ok(None),
    }
}

/// Apply a signed delta to the wallet's `liquid` bucket. Negative balances
/// are permitted per spec (GMs model debt narratively). Errors when the
/// wallet is absent.
pub async fn adjust_liquid(
    pool: &SqlitePool,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, String> {
    let res = sqlx::query(
        "UPDATE character_wallets
            SET liquid = liquid + ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(delta)
    .bind(character_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/wealth.adjust_liquid: {}", e))?;

    if res.rows_affected() == 0 {
        return Err(format!(
            "db/wealth.adjust_liquid: wallet not found for character {}",
            character_id
        ));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or_else(|| format!("db/wealth.adjust_liquid: wallet not found for character {}", character_id))
}

/// Apply a signed delta to the wallet's `banked` bucket. Same rules as
/// `adjust_liquid`.
pub async fn adjust_banked(
    pool: &SqlitePool,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, String> {
    let res = sqlx::query(
        "UPDATE character_wallets
            SET banked = banked + ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(delta)
    .bind(character_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/wealth.adjust_banked: {}", e))?;

    if res.rows_affected() == 0 {
        return Err(format!(
            "db/wealth.adjust_banked: wallet not found for character {}",
            character_id
        ));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or_else(|| format!("db/wealth.adjust_banked: wallet not found for character {}", character_id))
}

/// Atomically move `amount` from `from` to `to`. Same-bucket transfers are
/// rejected. Per spec: overdraw is permitted (resulting balance may go
/// negative). Wrapped in a single transaction so partial-failure cannot
/// desync the two columns.
pub async fn transfer_funds(
    pool: &SqlitePool,
    character_id: i64,
    from: Bucket,
    to: Bucket,
    amount: f64,
) -> Result<Wallet, String> {
    if from == to {
        return Err("db/wealth.transfer_funds: cannot transfer to the same bucket".to_string());
    }

    let (col_from, col_to) = match (from, to) {
        (Bucket::Liquid, Bucket::Banked) => ("liquid", "banked"),
        (Bucket::Banked, Bucket::Liquid) => ("banked", "liquid"),
        // Same-bucket already returned above.
        _ => unreachable!("same-bucket handled above"),
    };

    // Column names are not user input — they're picked from a closed enum,
    // so the format! interpolation is not a SQL-injection vector. The `?`
    // placeholders are still bound parametrically.
    let sql = format!(
        "UPDATE character_wallets
            SET {col_from} = {col_from} - ?,
                {col_to}   = {col_to}   + ?,
                updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?"
    );

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("db/wealth.transfer_funds.begin: {}", e))?;

    let res = sqlx::query(&sql)
        .bind(amount)
        .bind(amount)
        .bind(character_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("db/wealth.transfer_funds.update: {}", e))?;

    if res.rows_affected() == 0 {
        return Err(format!(
            "db/wealth.transfer_funds: wallet not found for character {}",
            character_id
        ));
    }

    tx.commit()
        .await
        .map_err(|e| format!("db/wealth.transfer_funds.commit: {}", e))?;

    get_wallet(pool, character_id)
        .await?
        .ok_or_else(|| format!("db/wealth.transfer_funds: wallet not found for character {}", character_id))
}

/// Update the wallet's display currency label (e.g. "$", "£", "🜚"). Does
/// not touch balances.
pub async fn set_currency_name(
    pool: &SqlitePool,
    character_id: i64,
    name: &str,
) -> Result<Wallet, String> {
    let res = sqlx::query(
        "UPDATE character_wallets
            SET currency_name = ?, updated_at = CURRENT_TIMESTAMP
          WHERE character_id = ?",
    )
    .bind(name)
    .bind(character_id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/wealth.set_currency_name: {}", e))?;

    if res.rows_affected() == 0 {
        return Err(format!(
            "db/wealth.set_currency_name: wallet not found for character {}",
            character_id
        ));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or_else(|| format!("db/wealth.set_currency_name: wallet not found for character {}", character_id))
}

/// Update the wallet's optional transfer-delay hint (forward-compat
/// scaffolding for the V2 timed-transfer feature — V1 stores but does not
/// enforce). Pass `None` to clear.
pub async fn set_transfer_delay(
    pool: &SqlitePool,
    character_id: i64,
    delay: Option<TransferDelay>,
) -> Result<Wallet, String> {
    let json = match delay {
        Some(ref d) => Some(serialize_transfer_delay(d)?),
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
    .await
    .map_err(|e| format!("db/wealth.set_transfer_delay: {}", e))?;

    if res.rows_affected() == 0 {
        return Err(format!(
            "db/wealth.set_transfer_delay: wallet not found for character {}",
            character_id
        ));
    }

    get_wallet(pool, character_id)
        .await?
        .ok_or_else(|| format!("db/wealth.set_transfer_delay: wallet not found for character {}", character_id))
}

// -------------------------------------------------------------------------
// Tauri commands
// -------------------------------------------------------------------------

#[tauri::command]
pub async fn wealth_enable_wallet(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
) -> Result<Wallet, String> {
    enable_wallet(&pool.0, character_id).await
}

#[tauri::command]
pub async fn wealth_disable_wallet(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
) -> Result<(), String> {
    disable_wallet(&pool.0, character_id).await
}

#[tauri::command]
pub async fn wealth_get_wallet(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
) -> Result<Option<Wallet>, String> {
    get_wallet(&pool.0, character_id).await
}

#[tauri::command]
pub async fn wealth_adjust_liquid(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, String> {
    adjust_liquid(&pool.0, character_id, delta).await
}

#[tauri::command]
pub async fn wealth_adjust_banked(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
    delta: f64,
) -> Result<Wallet, String> {
    adjust_banked(&pool.0, character_id, delta).await
}

#[tauri::command]
pub async fn wealth_transfer_funds(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
    from: Bucket,
    to: Bucket,
    amount: f64,
) -> Result<Wallet, String> {
    transfer_funds(&pool.0, character_id, from, to, amount).await
}

#[tauri::command]
pub async fn wealth_set_currency_name(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
    name: String,
) -> Result<Wallet, String> {
    set_currency_name(&pool.0, character_id, &name).await
}

#[tauri::command]
pub async fn wealth_set_transfer_delay(
    pool: tauri::State<'_, crate::DbState>,
    character_id: i64,
    delay: Option<TransferDelay>,
) -> Result<Wallet, String> {
    set_transfer_delay(&pool.0, character_id, delay).await
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    use std::str::FromStr;

    /// In-memory pool with full migrations applied and `foreign_keys=ON`.
    /// FK enforcement is required so the `disable_wallet` cascade test is
    /// meaningful, and so `enable_wallet` correctly resolves the wallet's
    /// FK to `saved_characters(id)`.
    async fn test_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .foreign_keys(true);
        let pool = SqlitePool::connect_with(opts).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    /// Insert a `saved_characters` row and return its id. The wealth tables
    /// FK to `saved_characters(id)` (see the note at the top of
    /// 0012_add_wealth.sql), so the test character has to live there.
    async fn fresh_pool_with_character(name: &str) -> (SqlitePool, i64) {
        let pool = test_pool().await;
        let id: i64 = sqlx::query_scalar(
            "INSERT INTO saved_characters (source, source_id, name, canonical_json)
             VALUES ('foundry', ?, ?, '{}')
             RETURNING id",
        )
        .bind(format!("test-{}", name))
        .bind(name)
        .fetch_one(&pool)
        .await
        .expect("insert saved_characters");
        (pool, id)
    }

    // ---- Lifecycle ------------------------------------------------------

    #[tokio::test]
    async fn enable_wallet_creates_zeroed_row_and_is_idempotent() {
        let (pool, char_id) = fresh_pool_with_character("Alice").await;

        // First enable — fresh row with zeros + default currency.
        let w = enable_wallet(&pool, char_id).await.unwrap();
        assert_eq!(w.character_id, char_id);
        assert_eq!(w.liquid, 0.0);
        assert_eq!(w.banked, 0.0);
        assert_eq!(w.currency_name, "$");
        assert!(w.transfer_delay.is_none());

        // Mutate, then re-enable. Idempotent INSERT OR IGNORE: must NOT
        // clobber existing balances.
        adjust_liquid(&pool, char_id, 500.0).await.unwrap();
        let w2 = enable_wallet(&pool, char_id).await.unwrap();
        assert_eq!(
            w2.liquid, 500.0,
            "enable_wallet on already-enabled must NOT clobber balances"
        );

        let got = get_wallet(&pool, char_id).await.unwrap();
        assert!(got.is_some());
        assert_eq!(got.unwrap().liquid, 500.0);
    }

    #[tokio::test]
    async fn get_wallet_returns_none_when_not_opted_in() {
        let (pool, char_id) = fresh_pool_with_character("Bob").await;
        let got = get_wallet(&pool, char_id).await.unwrap();
        assert!(got.is_none(), "characters without a wallet row must return None");
    }

    // ---- Adjust arithmetic ---------------------------------------------

    #[tokio::test]
    async fn adjust_liquid_handles_positive_negative_zero() {
        let (pool, char_id) = fresh_pool_with_character("Carol").await;
        enable_wallet(&pool, char_id).await.unwrap();

        let w = adjust_liquid(&pool, char_id, 100.0).await.unwrap();
        assert_eq!(w.liquid, 100.0);

        let w = adjust_liquid(&pool, char_id, -25.0).await.unwrap();
        assert_eq!(w.liquid, 75.0);

        let w = adjust_liquid(&pool, char_id, 0.0).await.unwrap();
        assert_eq!(w.liquid, 75.0);

        // Spec: negative balances are permitted (GMs model debt narratively).
        let w = adjust_liquid(&pool, char_id, -1000.0).await.unwrap();
        assert_eq!(w.liquid, -925.0);
    }

    #[tokio::test]
    async fn adjust_banked_independent_of_liquid() {
        let (pool, char_id) = fresh_pool_with_character("Dave").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 100.0).await.unwrap();

        let w = adjust_banked(&pool, char_id, 250.0).await.unwrap();
        assert_eq!(w.liquid, 100.0, "banked adjustments must not touch liquid");
        assert_eq!(w.banked, 250.0);
    }

    // ---- Transfer -------------------------------------------------------

    #[tokio::test]
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
        assert!(
            (w.liquid + w.banked - total_before).abs() < f64::EPSILON,
            "transfer must conserve total balance"
        );
    }

    #[tokio::test]
    async fn transfer_funds_same_bucket_is_error() {
        let (pool, char_id) = fresh_pool_with_character("Frank").await;
        enable_wallet(&pool, char_id).await.unwrap();
        adjust_liquid(&pool, char_id, 100.0).await.unwrap();

        let err = transfer_funds(&pool, char_id, Bucket::Liquid, Bucket::Liquid, 50.0).await;
        let msg = err.unwrap_err();
        assert!(
            msg.contains("same bucket"),
            "expected same-bucket rejection, got: {msg}"
        );
    }

    #[tokio::test]
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

    // ---- Disable + cascade ---------------------------------------------

    #[tokio::test]
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

        // Spec: disable on non-existent wallet is a no-op, not an error.
        disable_wallet(&pool, char_id)
            .await
            .expect("second disable must be no-op, not error");
    }
}
