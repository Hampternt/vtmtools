# `character::set_field` router — design spec

> **Status:** designed; ready for plan-writing.
> **Issue:** [#6](https://github.com/Hampternt/vtmtools/issues/6) (Phase 2 — Character editing milestone).
> **Source roadmap:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 2.
> **Audience:** anyone implementing Phase 2 character-editing features (#6, #7, #8) or extending the canonical-name surface in a Phase 2.5 follow-up.

---

## §1 What this is

A single Tauri command — `character_set_field` — that mutates one named field on a character, routing the write to the saved-row DB, the live-bridge wire, or both, per an explicit caller-supplied `WriteTarget`.

This design intentionally keeps the router thin. The principal architectural work lives in a new `src-tauri/src/shared/canonical_fields.rs` module that holds the canonical-name namespace as a typed contract, plus a new path-level patcher on `db/saved_character.rs`. The router itself is a two-call composer.

#6 is the dependency root for Phase 2: both #7 (Stat editing UI) and #8 (Add/remove advantage) name #6 in their cross-cutting interactions, but #8 only shares a design *style* with #6, not code (#6 is field-level; #8 is item-level — see §8).

## §2 Design decisions and their rationale

These decisions are settled. Open questions live in §11.

### §2.1 Routing posture: explicit `WriteTarget`, not auto-routing

The issue body originally framed routing as "saved-only → DB write; live-only → bridge; both → both" — i.e., dispatch on what exists. That conflates "what exists" with "what the caller intends." Reframed: the router takes an explicit `WriteTarget { Live, Saved, Both }`. The UI layer maps button context → target; the router never guesses. "What exists" becomes a precondition check (return Err if target=Live but source disconnected; target=Saved but no saved row).

This dissolves the original conflict-policy question: drift is a render-time concept already handled by Plan 2's diff layer; it's not a write-time decision.

### §2.2 Saved-as-snapshot framing (Q1=B)

Saved cards are *not* independently editable through stat-editor buttons; they remain frozen snapshots whose only mutation paths are the existing "Update saved" full-blob overwrite and the offline-only fallback when no live counterpart exists. As a consequence:
- Phase 2 UI (#7) places +/- buttons on **live** cards by default.
- For saved-only/offline characters, the +/- buttons fall to the saved card with `target=Saved`.
- `target=Both` exists in the router signature for extensibility headroom (per project preference to pre-build per roadmap) but is not invoked by Phase 2 UI.

### §2.3 Atomicity for `Both`: saved-first, partial-success error

When `target=Both` and the live write fails (e.g., Foundry disconnected mid-edit), the router writes the saved row first (succeeds — local), then attempts the live write. On live failure it returns `Err("character/set_field: saved updated, live failed: <reason>")` so the toast tells the GM exactly what state they're in. No rollback. Plan 2's drift badge will continue to surface the divergence; the GM can retry live or hit "Update saved" later.

Rationale: the local write is the GM's notes; those should be correct even if the connection drops. Saved-only is the safer post-failure state. Rollback would mask the real condition (Foundry is offline). Two-phase commit is overkill for a single-user local tool.

### §2.4 Value type at the IPC boundary: `serde_json::Value`

The router accepts `value: serde_json::Value`, preserving int/bool/string distinctions. Frontend sends typed JSON; the saved-side patcher applies it directly to the typed Rust struct (`Option<u8>`, etc.) without re-parsing. The live-side composer stringifies on the way down to the existing `bridge_set_attribute` (whose `value: String` signature is unchanged — Foundry's `parse_value` already round-trips through string today).

This avoids the `"3"` vs `3` ambiguity at the saved-side typed boundary. The lossy step happens once (router → live), not twice (frontend → router → live).

### §2.5 v1 surface: top-level canonical fields only

The Phase 2 router accepts the eight canonical names already used by `bridge/foundry/mod.rs::canonical_to_path`:

```
hunger, humanity, humanity_stains, blood_potency,
health_superficial, health_aggravated,
willpower_superficial, willpower_aggravated
```

Skills/attributes (which live deeper in `canonical.raw.system.attributes.<key>.value` and require source-specific path-walking) and `health.max` / `willpower.max` are deferred to a **Phase 2.5 follow-up**. That follow-up will land alongside the Roll20 canonical-name mapping (see §2.7).

### §2.6 Path namespace single source of truth: `shared/canonical_fields.rs`

A new pure-helper module owns the canonical-name list and the saved-side mutator. Per-source path translators (`canonical_to_foundry_path`, `canonical_to_roll20_attr`) live alongside it and **must** cover the same name set. `cargo test` enforces coverage:

```rust
assert!(ALLOWED_NAMES.iter().all(|n| canonical_to_foundry_path(n).is_some()))
```

Drift between the saved-side mutator and any per-source translator becomes a test failure, not a runtime mystery.

### §2.7 Read-only field policy: client-side only

The router accepts any `name` in `ALLOWED_NAMES`. UI layer (#7) chooses which fields show +/- buttons. Read-only-by-convention fields (e.g., Blood Potency per the issue body's example) are a UX decision, not a data-integrity one. Backend stays a data plumber, not a rules engine.

### §2.8 Roll20 live-editing of canonical names: deferred

Roll20's `BridgeSource::build_set_attribute` is currently a wire passthrough: the inbound `name` becomes the outbound Roll20 attribute name verbatim. The v1 `canonical_to_roll20_attr` returns `None` for every canonical name. The router fails fast on `target=Live` with a Roll20 source and a canonical name:

```
character/set_field: Roll20 live editing of canonical names not yet supported
```

Roll20 *saved-side* editing remains supported in v1 (the saved-side patcher mutates the typed `CanonicalCharacter` struct, which is source-agnostic). Roll20 live canonical editing lands together with the Phase 2.5 skills/attributes follow-up, since both require new per-source name mappings and benefit from being designed together.

---

## §3 Architecture diagram

```
Frontend (Svelte)
  │ characterSetField(target, source, sourceId, name, value)
  ▼
Tauri IPC: character_set_field
  │
  ▼
src-tauri/src/tools/character.rs           ← thin composer (the "router")
  ├─ target=Live   → bridge_set_attribute (existing — ARCH §4)
  ├─ target=Saved  → patch_saved_field    (new — db/saved_character.rs)
  └─ target=Both   → patch_saved_field; THEN bridge_set_attribute
                     (saved-first; live-failure → partial-success Err)

Shared name namespace (single source of truth):
src-tauri/src/shared/canonical_fields.rs   ← new, sibling of v5/
  ├─ ALLOWED_NAMES: &[&str]
  ├─ apply_canonical_field(&mut CanonicalCharacter, name, &Value) -> Result<(), String>
  ├─ canonical_to_foundry_path(name) -> Option<&'static str>
  └─ canonical_to_roll20_attr(name)  -> Option<&'static str>
```

---

## §4 Components

### §4.1 `src-tauri/src/shared/canonical_fields.rs` (new)

```rust
//! Canonical-name namespace for character field updates.
//!
//! Single source of truth for the set of names accepted by
//! `character::set_field` and any other consumer that reads/writes
//! canonical character fields. Per-source translators below MUST
//! cover ALLOWED_NAMES; `cargo test` enforces coverage.

use crate::bridge::types::{CanonicalCharacter, HealthTrack};
use serde_json::Value;

/// The v1 canonical-name surface.
pub const ALLOWED_NAMES: &[&str] = &[
    "hunger",
    "humanity",
    "humanity_stains",
    "blood_potency",
    "health_superficial",
    "health_aggravated",
    "willpower_superficial",
    "willpower_aggravated",
];

/// Apply a canonical-named field to a typed CanonicalCharacter.
///
/// Returns Err on:
///   - unknown name (not in ALLOWED_NAMES),
///   - wrong value type for the field (e.g. string for hunger),
///   - out-of-range integer (e.g. hunger > 5).
pub fn apply_canonical_field(
    c: &mut CanonicalCharacter,
    name: &str,
    value: &Value,
) -> Result<(), String> {
    match name {
        "hunger" => {
            let n = expect_u8_in_range(value, name, 0, 5)?;
            c.hunger = Some(n);
        }
        "humanity" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity = Some(n);
        }
        "humanity_stains" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.humanity_stains = Some(n);
        }
        "blood_potency" => {
            let n = expect_u8_in_range(value, name, 0, 10)?;
            c.blood_potency = Some(n);
        }
        "health_superficial" | "health_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.health, name, n);
        }
        "willpower_superficial" | "willpower_aggravated" => {
            let n = expect_u8_in_range(value, name, 0, 20)?;
            apply_track_field(&mut c.willpower, name, n);
        }
        other => return Err(format!("character/set_field: unknown field '{other}'")),
    }
    Ok(())
}

fn apply_track_field(track: &mut Option<HealthTrack>, name: &str, n: u8) {
    let t = track.get_or_insert(HealthTrack { max: 0, superficial: 0, aggravated: 0 });
    match name {
        s if s.ends_with("_superficial") => t.superficial = n,
        s if s.ends_with("_aggravated")  => t.aggravated  = n,
        _ => {} // unreachable per outer match
    }
}

fn expect_u8_in_range(v: &Value, name: &str, lo: u8, hi: u8) -> Result<u8, String> {
    let n = v.as_u64().ok_or_else(|| format!(
        "character/set_field: '{name}' expects integer {lo}..={hi}, got {}",
        type_label(v),
    ))?;
    if n > hi as u64 || (n as u8) < lo {
        return Err(format!(
            "character/set_field: '{name}' expects integer {lo}..={hi}, got {n}"
        ));
    }
    Ok(n as u8)
}

fn type_label(v: &Value) -> &'static str {
    match v {
        Value::Null    => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_)  => "array",
        Value::Object(_) => "object",
    }
}

/// Foundry system-path mapping. Replaces the inline match in
/// bridge/foundry/mod.rs::canonical_to_path.
pub fn canonical_to_foundry_path(name: &str) -> Option<&'static str> {
    Some(match name {
        "hunger"                 => "system.hunger.value",
        "humanity"               => "system.humanity.value",
        "humanity_stains"        => "system.humanity.stains",
        "blood_potency"          => "system.blood.potency",
        "health_superficial"     => "system.health.superficial",
        "health_aggravated"      => "system.health.aggravated",
        "willpower_superficial"  => "system.willpower.superficial",
        "willpower_aggravated"   => "system.willpower.aggravated",
        _ => return None,
    })
}

/// Roll20 attribute mapping. v1 returns None for every canonical name —
/// Roll20 live editing of canonical names is deferred to Phase 2.5
/// (see §2.8). Roll20 saved-side editing is unaffected.
pub fn canonical_to_roll20_attr(_name: &str) -> Option<&'static str> {
    None
}
```

### §4.2 `src-tauri/src/db/saved_character.rs` (extend)

```rust
async fn db_patch_field(
    pool: &SqlitePool,
    id: i64,
    name: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    // 1. Read row.
    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.patch_field: {e}"))?
        .ok_or_else(|| "db/saved_character.patch_field: not found".to_string())?;

    // 2. Deserialize → mutate via shared helper → re-serialize.
    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.patch_field: deserialize failed: {e}"))?;
    crate::shared::canonical_fields::apply_canonical_field(&mut canonical, name, value)?;
    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.patch_field: serialize failed: {e}"))?;

    // 3. Write back; bump last_updated_at; saved_at unchanged.
    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, name = ?, last_updated_at = datetime('now')
         WHERE id = ?"
    )
    .bind(&new_json)
    .bind(&canonical.name)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.patch_field: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.patch_field: not found".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn patch_saved_field(
    pool: tauri::State<'_, crate::DbState>,
    id: i64,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    db_patch_field(&pool.0, id, &name, &value).await
}
```

`patch_saved_field` is exposed as a public Tauri command (not just a private helper) so it can be tested directly and so future tools can call it without going through the router's target-routing.

### §4.3 `src-tauri/src/tools/character.rs` (new)

```rust
use serde::Deserialize;
use sqlx::Row;
use tauri::State;

use crate::bridge::types::SourceKind;
use crate::shared::canonical_fields::{ALLOWED_NAMES, canonical_to_roll20_attr};

#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WriteTarget { Live, Saved, Both }

#[tauri::command]
pub async fn character_set_field(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    // 0. Fast-fail: unknown name.
    if !ALLOWED_NAMES.contains(&name.as_str()) {
        return Err(format!("character/set_field: unknown field '{name}'"));
    }

    // 1. Roll20 live canonical-name fast-fail (v1 limitation; see §2.8).
    if target != WriteTarget::Saved
        && source == SourceKind::Roll20
        && canonical_to_roll20_attr(&name).is_none()
    {
        return Err(
            "character/set_field: Roll20 live editing of canonical names not yet supported"
                .to_string()
        );
    }

    // 2. Resolve saved id (only if Saved or Both).
    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(&db, source, &source_id).await?)
    } else {
        None
    };

    match target {
        WriteTarget::Saved => {
            crate::db::saved_character::db_patch_field(
                &db.0, saved_id.unwrap(), &name, &value
            ).await
        }
        WriteTarget::Live => {
            forward_live(&bridge, source, &source_id, &name, &value).await
        }
        WriteTarget::Both => {
            // Saved-first; partial-success error on live failure (§2.3).
            crate::db::saved_character::db_patch_field(
                &db.0, saved_id.unwrap(), &name, &value
            ).await
            .map_err(|e| format!("character/set_field: saved write failed: {e}"))?;
            forward_live(&bridge, source, &source_id, &name, &value).await
                .map_err(|e| format!(
                    "character/set_field: saved updated, live failed: {e}"
                ))
        }
    }
}

async fn lookup_saved_id(
    db: &State<'_, crate::DbState>,
    source: SourceKind,
    source_id: &str,
) -> Result<i64, String> {
    let source_str = match source {
        SourceKind::Roll20 => "roll20",
        SourceKind::Foundry => "foundry",
    };
    let row = sqlx::query(
        "SELECT id FROM saved_characters WHERE source = ? AND source_id = ?"
    )
    .bind(source_str)
    .bind(source_id)
    .fetch_optional(&db.0)
    .await
    .map_err(|e| format!("character/set_field: {e}"))?
    .ok_or_else(|| format!(
        "character/set_field: no saved row for {source_str}/{source_id}"
    ))?;
    Ok(row.get("id"))
}

async fn forward_live(
    bridge: &State<'_, crate::BridgeConn>,
    source: SourceKind,
    source_id: &str,
    name: &str,
    value: &serde_json::Value,
) -> Result<(), String> {
    // Stringify for the existing bridge_set_attribute pipeline.
    // Foundry's parse_value re-parses on the other side; lossy step happens
    // exactly once, here.
    let s = match value {
        serde_json::Value::String(s) => s.clone(),
        v => v.to_string(), // numbers/bools serialize as their JSON literal
    };
    // Calls the extracted inner helper (see §4.5) so we don't pass `State`
    // through a non-IPC call site.
    crate::bridge::commands::do_set_attribute(
        &bridge.0,
        source,
        source_id.to_string(),
        name.to_string(),
        s,
    ).await
}
```

### §4.4 `src/lib/character/api.ts` (new)

```ts
// Typed wrapper around character_set_field. Per CLAUDE.md, components must
// NOT call invoke() directly — they go through here.

import { invoke } from '@tauri-apps/api/core';
import type { SourceKind } from '../../types';

export type WriteTarget = 'live' | 'saved' | 'both';

/** Mirrors src-tauri/src/shared/canonical_fields.rs::ALLOWED_NAMES. */
export type CanonicalFieldName =
  | 'hunger'
  | 'humanity'
  | 'humanity_stains'
  | 'blood_potency'
  | 'health_superficial'
  | 'health_aggravated'
  | 'willpower_superficial'
  | 'willpower_aggravated';

export const characterSetField = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  name: CanonicalFieldName,
  value: number | string | boolean | null,
): Promise<void> =>
  invoke<void>('character_set_field', { target, source, sourceId, name, value });
```

### §4.5 Bridge composition refactor (small)

Two small refactors in `src-tauri/src/bridge/`. Wire shape and existing Tauri command signatures unchanged.

**(a) `bridge/foundry/mod.rs::canonical_to_path`** becomes a thin wrapper that delegates to `shared::canonical_fields::canonical_to_foundry_path`, falling back to passthrough for arbitrary `system.*` paths (preserves existing behavior for non-canonical names that consumers may already pass):

```rust
fn canonical_to_path(name: &str) -> String {
    if let Some(p) = crate::shared::canonical_fields::canonical_to_foundry_path(name) {
        return p.to_string();
    }
    if name.starts_with("system.") {
        return name.to_string();
    }
    name.to_string()
}
```

**(b) `bridge/commands.rs::bridge_set_attribute`** gets a small extraction so the router can call its logic without passing `State` through a non-IPC call site. The Tauri command stays; its body becomes a one-liner forward to a new `pub(crate) async fn do_set_attribute`:

```rust
pub(crate) async fn do_set_attribute(
    state: &std::sync::Arc<BridgeState>,
    source: SourceKind,
    source_id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    let source_impl = state
        .sources
        .get(&source)
        .cloned()
        .ok_or_else(|| format!("source {} not registered", source.as_str()))?;
    let payload = source_impl
        .build_set_attribute(&source_id, &name, &value)
        .map_err(|e| format!("bridge/set_attribute: {e}"))?;
    let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
    send_to_source_inner(state, source, text).await
}

#[tauri::command]
pub async fn bridge_set_attribute(
    conn: State<'_, BridgeConn>,
    source: SourceKind,
    source_id: String,
    name: String,
    value: String,
) -> Result<(), String> {
    do_set_attribute(&conn.0, source, source_id, name, value).await
}
```

The existing `send_to_source(&State, ...)` helper gets a sibling `send_to_source_inner(&Arc<BridgeState>, ...)` so the State-bound version forwards to it.

`bridge/roll20/mod.rs` is unchanged in this issue (the v1 `canonical_to_roll20_attr` stub returns None for everything; Roll20's passthrough behavior remains for non-canonical attribute names).

---

## §5 Data flows

### §5.1 Live edit (typical Phase 2 stat-editor click on a Foundry live card)

```
+1 hunger button on live card
  → characterSetField('live', 'foundry', sid, 'hunger', 3)
  → IPC
  → router: target=Live → forward_live → bridge_set_attribute(foundry, sid, 'hunger', '3')
  → FoundrySource::build_set_attribute → canonical_to_foundry_path → 'system.hunger.value'
  → wire: actor.update_field
  → Foundry module applies actor.update; bridge://characters-updated re-renders
  → drift badge appears on the saved card (saved=2, live=3) — Plan 2's diff layer
```

### §5.2 Offline saved-only edit

```
+1 hunger button on saved card (no live counterpart)
  → characterSetField('saved', 'foundry', sid, 'hunger', 3)
  → IPC
  → router: target=Saved → lookup_saved_id → patch_saved_field(id, 'hunger', 3)
  → reads canonical_json, apply_canonical_field, writes back, bumps last_updated_at
  → savedCharacters store refreshes
```

### §5.3 Both target with disconnected Foundry (extensibility headroom)

```
characterSetField('both', 'foundry', sid, 'hunger', 3)
  → router: db_patch_field succeeds
  →         forward_live → bridge_set_attribute fails (no outbound channel)
  → returns Err("character/set_field: saved updated, live failed: <reason>")
  → frontend toast: partial-success message
```

---

## §6 Error handling

Follows ARCHITECTURE.md §7: Rust commands return `Result<T, String>` with module-stable prefixes; frontend catches in API wrappers and surfaces via toast / inline state.

| Scenario | Behavior | Module prefix |
|---|---|---|
| `name` ∉ ALLOWED_NAMES | Validated up front | `character/set_field: unknown field '<name>'` |
| `value` wrong type for field | `apply_canonical_field` returns Err | `character/set_field: 'hunger' expects integer 0..=5, got string` |
| `value` integer out of range | `apply_canonical_field` returns Err | `character/set_field: 'hunger' expects integer 0..=5, got 7` |
| `target=Saved`/`Both`, no saved row for `(source, source_id)` | Up-front lookup fails | `character/set_field: no saved row for foundry/<sid>` |
| `target=Live`, source not connected | `bridge_set_attribute` returns Err (existing path) | passthrough |
| `target=Live`, Roll20 source, canonical name | Stub fast-fail | `character/set_field: Roll20 live editing of canonical names not yet supported` |
| `target=Both`, saved write fails | Fatal — live not attempted | `character/set_field: saved write failed: <reason>` |
| `target=Both`, saved succeeds, live fails | Partial-success error | `character/set_field: saved updated, live failed: <reason>` |
| Foundry rejects asynchronously (later) | Plan 0 `bridge://foundry/error` toast (router unaware) | n/a — handled by Plan 0 path |

---

## §7 Testing

`./scripts/verify.sh` covers the full gate. Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

### `shared/canonical_fields.rs` (`#[cfg(test)] mod tests`)

- `apply_canonical_field` happy path per name (one test per canonical field).
- `apply_canonical_field` rejects wrong type (string for hunger).
- `apply_canonical_field` rejects out-of-range integer (hunger=7).
- `apply_canonical_field` rejects unknown name.
- **Coverage assertion** (structural invariant):
  ```rust
  #[test]
  fn every_allowed_name_has_foundry_path() {
      for n in ALLOWED_NAMES {
          assert!(canonical_to_foundry_path(n).is_some(),
              "missing Foundry path for {n}");
      }
  }
  #[test]
  fn every_allowed_name_applies_via_apply_canonical_field() {
      // Probe every name with a sample valid value; asserts no name
      // falls through to the unknown-field arm.
      for n in ALLOWED_NAMES {
          let mut c = sample_canonical();
          let v = serde_json::json!(0);
          let res = apply_canonical_field(&mut c, n, &v);
          assert!(res.is_ok(), "apply_canonical_field rejected '{n}': {:?}", res.err());
      }
  }
  ```

### `db/saved_character.rs` tests (extend existing module)

- `patch_saved_field` happy path: read → mutate → write → re-read shows new value; `last_updated_at` bumped.
- `patch_saved_field` missing id: `Err("not found")`.
- `patch_saved_field` unknown name: `Err` from apply layer (prefix `character/set_field`).

### `tools/character.rs` tests (`#[cfg(test)] mod tests`)

- `target=Saved` with offline character — DB updated.
- `target=Saved` no matching row — Err `"no saved row for foundry/<sid>"`.
- `target=Live` Foundry source — outbound payload routed (mock `BridgeConn`).
- `target=Live` Roll20 source canonical name — Err fast-fail.
- `target=Both` happy path — saved-first ordering verified by call order on mocks.
- `target=Both` saved fails — live not attempted; error formatting matches.
- `target=Both` saved succeeds, live fails — partial-success error matches `"saved updated, live failed:"` prefix.

### Manual verification

From a Foundry-connected dev session, click a stub stat-editor button (#7's deliverable) on a live card whose character has a saved counterpart:
1. Live card updates (hunger value re-renders from `bridge://characters-updated`).
2. Drift badge appears on the saved counterpart card.
3. "Update saved" button on the live card clears the badge.

For the offline-saved-only path, disconnect Foundry, click a stub button on a saved-only card, observe the saved store refreshing with the new value.

---

## §8 Files inventory

| Action | File | Reason |
|---|---|---|
| Create | `src-tauri/src/shared/canonical_fields.rs` | Namespace SSoT (§4.1) |
| Create | `src-tauri/src/tools/character.rs` | Router / composer (§4.3) |
| Create | `src/lib/character/api.ts` | Typed frontend wrapper (§4.4) |
| Modify | `src-tauri/src/shared/mod.rs` | Add `pub mod canonical_fields;` |
| Modify | `src-tauri/src/tools/mod.rs` | Add `pub mod character;` |
| Modify | `src-tauri/src/db/saved_character.rs` | Add `db_patch_field` + `patch_saved_field` Tauri command + tests (§4.2) |
| Modify | `src-tauri/src/bridge/foundry/mod.rs` | Delegate `canonical_to_path` to shared helper (§4.5a) |
| Modify | `src-tauri/src/bridge/commands.rs` | Extract `do_set_attribute` + `send_to_source_inner` (§4.5b) |
| Modify | `src-tauri/src/lib.rs` | Register `character_set_field` and `patch_saved_field` in `invoke_handler!` |
| Modify | `src/types.ts` | Mirror `WriteTarget` union and `CanonicalFieldName` literal type |

Total: 3 new files, 7 modifications. No new SQL migrations; no new wire variants; no new Tauri events.

Tauri command surface grows from 37 → 39 (`character_set_field`, `patch_saved_field`).

---

## §9 Anti-scope (per ARCH §11)

| Anti-scope | Why |
|---|---|
| **Item-level paths** (specialty values, merit points, advantage points) | Different shape — needs `item_id` + raw-walking. Owned by #8. |
| **Skills/attributes** (`system.skills.brawl.value`) | Live in `canonical.raw`; require source-specific path-walking. Owned by Phase 2.5. |
| **`health.max` / `willpower.max`** | Stable derived values; not typically edited mid-session. Phase 2.5 if needed. |
| **Roll20 live editing of canonical names** | Same Phase 2.5 follow-up; needs Roll20 attr-name mapping table. |
| **Retiring `bridge_set_attribute`** | The router *composes* it; it stays as the Tauri command for raw-attribute callers. |
| **New Plan-0 wire variants** | Phase 2 router uses only `actor.update_field` (already shipped). |
| **New SQL migrations** | Saved-row schema unchanged; only the JSON blob is mutated. |
| **Compare modal changes** | Plan 2's diff already covers all eight canonical names. |
| **Stat editor UI** (#7) | This issue lays the foundation; #7 ships the buttons that call into it. |

---

## §10 How #7 and #8 compose this router

(Made explicit per advisor review — so the future plans don't redesign the router on the fly.)

### #7 — Stat editing UI

Each editable stat on a **live card** wires its +/- buttons to:

```ts
characterSetField('live', char.source, char.source_id, '<canonical-name>', newValue);
```

For an **offline saved-only card**, the same buttons wire to:

```ts
characterSetField('saved', saved.source, saved.sourceId, '<canonical-name>', newValue);
```

No new Tauri commands. No new error-handling pathways. Pure composition of #6.

The "which fields show buttons" decision is #7's policy (e.g., hunger gets +/- buttons; blood potency may not). #6's router accepts any name in ALLOWED_NAMES; #7 is the gatekeeper for which names actually surface buttons.

### #8 — Add/remove advantage

#8 does **not** use `character_set_field`. Advantages are item documents on the Foundry actor; they live at a different shape than canonical fields. #8 composes:

- `actor.create_feature` / `actor.delete_item_by_id` — already-shipped Foundry helpers (FHL Phase 1).
- A new advantage-specific saved-side helper pair: `db/saved_character::add_advantage`, `db/saved_character::remove_advantage`, walking `canonical.raw.items[]`.

#6's router is field-level; #8's analogous helper is item-level. They share **design style** (thin composer over typed primitives) but no code. Naming the parallel pattern explicitly here so #8's plan can mirror this design without re-litigating it.

---

## §11 Open questions

### Resolved during this brainstorm

- ✅ **Routing posture** — explicit `WriteTarget`, not auto-routing on what exists (§2.1).
- ✅ **Saved cards independently editable?** — No; saved stays a snapshot (§2.2).
- ✅ **Atomicity for `Both`** — saved-first, partial-success error on live failure (§2.3).
- ✅ **Value type at IPC** — `serde_json::Value` (§2.4).
- ✅ **v1 surface** — eight top-level canonical fields only (§2.5).
- ✅ **Path namespace SSoT** — `shared/canonical_fields.rs` with cargo-test coverage (§2.6).
- ✅ **Read-only field policy** — client-side only; backend stays a data plumber (§2.7).
- ✅ **Roll20 live canonical editing** — deferred to Phase 2.5 alongside skills/attributes (§2.8).

### Outstanding (deferred to later phases)

- **Phase 2.5**: Roll20 canonical-name attr mappings; skills/attributes path-into-raw patcher.
- **Async-correlation (FHL Phase 0+):** Plan 0 reserved `request_id` on the error envelope. If a future feature needs to correlate an `actor.update_field` failure with a specific call site, that work is additive and lives in the bridge layer — `character_set_field`'s contract doesn't change.

---

## §12 Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

| Stage | What `verify.sh` proves |
|---|---|
| `cargo test` | Coverage assertions (§7); per-target router branches; partial-success error formatting |
| `npm run check` | TS mirror layer (CanonicalFieldName union must match ALLOWED_NAMES) |
| `npm run build` | Typed wrapper compiles; no TS regressions |

Manual gate (done once at the end, not per-task): live drift-badge round-trip on a Foundry-connected dev session.

---

## §13 Pointers

- `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 2 — original sketch this design refines.
- `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md` — `actor.*` umbrella that `bridge_set_attribute` ultimately routes to.
- `ARCHITECTURE.md` §4 (Tauri IPC + bridge protocol), §7 (error handling), §9 (Add a Tauri command), §11 (plan conventions).
- `docs/reference/foundry-vtm5e-paths.md` — WoD5e actor schema (relevant for verifying Foundry path mappings).
