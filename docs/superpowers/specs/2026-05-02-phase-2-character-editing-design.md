# Phase 2 character editing — stat editor + advantages — design spec

> **Status:** designed; ready for plan-writing.
> **Issues:** [#7](https://github.com/Hampternt/vtmtools/issues/7) (Stat editing UI) + [#8](https://github.com/Hampternt/vtmtools/issues/8) (Add/remove advantage) — both in the Phase 2 — Character editing milestone.
> **Source roadmap:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 2.
> **Sibling spec (foundation):** `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md` (already shipped — closes #6). #7 composes its router; #8 mirrors its design style.
> **FHL helper roadmap:** `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md` — `actor.create_feature` + `actor.delete_item_by_id` (Phase 1, shipped) are the two outbound primitives #8 composes.
> **Audience:** anyone implementing the two remaining Phase 2 issues, or extending the diff layer to cover advantages.

---

## §1 What this is

Two surface-level Phase 2 features that together close out character editing:

- **#7 — Stat editing UI:** Inline +/- controls on Campaign cards for the eight canonical fields the `character_set_field` router (#6) already accepts. Pure-frontend; zero new IPC. Live cards write `target=Live`; offline-saved-only cards write `target=Saved`.
- **#8 — Add/remove advantage:** Per-chip "X" remove button on each merit / flaw / background / boon, plus a per-category "+ Add" button that opens an inline form. Composes Foundry's already-shipped `actor.create_feature` and `actor.delete_item_by_id` helpers on the live side; new `db_add_advantage` / `db_remove_advantage` JSON-walking helpers on the saved side. New `tools/character.rs` Tauri commands wrap both. Diff layer extended with `diffAdvantages` (mirrors Plan 2's `diffSpecialties`).

Both features share the `tools/character.rs` module, the `Campaign.svelte` consumer surface, and the `WriteTarget`-explicit routing posture established by #6. Neither introduces new wire variants, new SQL migrations, or new module bumps.

The two features are packaged in one spec because they share the consumer surface (the same Campaign cards), the design style (thin composer over typed primitives), and the rescope decision in §2.1 — but they ship as **two independent plans** (see §10).

## §2 Design decisions and their rationale

These decisions are settled. Open questions live in §13.

### §2.1 #7 v1 surface = canonical fields only (rescope from issue title)

Issue #7's title says "increment/decrement attributes & skills." The set_field router shipped (#6, commit `63aa613`) with `ALLOWED_NAMES` = the eight canonical fields:

```
hunger, humanity, humanity_stains, blood_potency,
health_superficial, health_aggravated,
willpower_superficial, willpower_aggravated
```

Skills/attributes (e.g. `system.skills.brawl.value`) live deeper in `canonical.raw.system.*`, require source-specific path-walking, and are explicitly deferred to **Phase 2.5** by the router spec §2.5 — alongside Roll20 canonical-name attr mappings, since both benefit from being designed together.

This spec resolves the conflict in #7's framing in favor of the router precedent: **#7 v1 ships +/- buttons for the eight canonical fields only**. Skills/attributes are out of scope here; they land in the Phase 2.5 follow-up that also adds Roll20 live canonical editing.

**Action item (handled outside this spec):** The user updates #7's title/body to reflect "canonical-field stat editing" and either creates a new Phase 2.5 issue for "Skills/attributes editing" or defers issue creation until that phase matures. Per the project's roadmap-tracking rule (CLAUDE.md), this spec doesn't auto-create or auto-edit GitHub issues.

The list of which canonical fields actually surface +/- buttons is a UI policy decision — see §2.4.

### §2.2 #8 splits the router shape into one command per verb

The set_field router (#6) is one command — `character_set_field` — gated by an explicit `WriteTarget`. For advantages, the verbs *add* and *remove* take different argument shapes (add carries name/description/points; remove carries an item_id), so collapsing them into a single `character_advantage_set` command would force a tagged-union argument and lose type safety at the IPC boundary.

This spec adds **two** Tauri commands:

```
character_add_advantage(target, source, source_id, featuretype, name, description, points)
character_remove_advantage(target, source, source_id, featuretype, item_id)
```

Both accept the same `WriteTarget { Live, Saved, Both }` enum from #6. Both follow the saved-first / partial-success error pattern from §2.3 of the router spec.

### §2.3 #8 saved-side `_id` strategy: `local-<uuid>` prefix

When an advantage is added saved-side (offline path), the new item enters `canonical.raw.items[]` with a synthesized `_id`. The convention is `local-<uuid>` — matches the "this id was minted by vtmtools, not by Foundry" semantics. Examples: `local-c3d4e5f6-…`.

Self-healing: the next "Update saved" click overwrites the entire `canonical_json` blob from the live `BridgeCharacter`, replacing every `local-*` id with the real Foundry id. Drift detection (Plan 2 + §2.7 below) flags the missing item on the live side until then.

For removal: the `item_id` is whatever the chip's `_id` says — Foundry-minted ids round-trip through saved snapshots unchanged, so `target=Saved` removes by exact match. `target=Live` sends `actor.delete_item_by_id` with the same id; Foundry resolves it server-side.

**Why this shape** — the alternative is a separate `local_advantages` table joined to the saved row. Rejected: the saved blob is already a JSON snapshot of `canonical.raw`; advantages live there as items; the diff layer already walks `raw.items[]` for `diffSpecialties`. Splitting advantages into a sidecar table would force every reader to do a join.

### §2.4 #7 button policy: which canonical fields get +/- controls

Per the router spec §2.7, "read-only-by-convention" is a UX decision, not a backend constraint. The router accepts any name in `ALLOWED_NAMES`. This spec's UI policy:

| Field | +/- buttons? | Rationale |
|---|---|---|
| `hunger` | ✓ | Mid-session resource; primary editing target |
| `humanity` | ✓ | Changes on degeneration / mortal / restoration |
| `humanity_stains` | ✓ | Changes when conviction is broken |
| `health_superficial` | ✓ | Damage / healing tracking |
| `health_aggravated` | ✓ | Damage tracking |
| `willpower_superficial` | ✓ | Damage / regain tracking |
| `willpower_aggravated` | ✓ | Damage tracking |
| `blood_potency` | ✗ (read-only) | Changes are book-keeping events (torpor, diablerie); rare and deserve a separate editor, not an inline +/- |

Blood Potency edits remain available via the existing "Edit raw" workflow (or, post-Phase 2.5, via direct Tauri call). The spec doesn't *block* future builds from surfacing a BP editor — it just keeps the inline +/- noise low.

The buttons are inline next to the existing visual elements (hunger drops, conscience track, health/willpower boxes) — no separate edit-mode toggle. Detail in §4.4.

### §2.5 #8 advantage UI shape: chip-X remove + per-category Add

Each existing feature chip (merit / flaw / background / boon — rendered today by `Campaign.svelte` lines 506–566 from `foundryFeatures(char, 'merit')` etc.) gains an `×` button on hover. Per category, a `+ Add` chip-shaped button at the end of the row opens an inline form:

```
┌─────────────────────────────────┐
│ Add Merit                    × │
│ ┌────────────────────────────┐ │
│ │ Name:        [____________] │ │
│ │ Points:      [_]            │ │
│ │ Description: [____________] │ │
│ │              [____________] │ │
│ │              [Add] [Cancel] │ │
│ └────────────────────────────┘ │
└─────────────────────────────────┘
```

The form lives inline below the row, not in a modal — keeps it consistent with the existing collapsible-section idiom in `Campaign.svelte`. `featuretype` is implicit per row (the user clicked "Add" on the Merits row; the field doesn't need to be in the form).

For the empty-row case (e.g. character has no boons): show a single `+ Add Boon` button below the section title; the row only materializes after the first add.

**Validation:** name must be non-empty; points clamped to 0..=10 (matches existing `feat-dots` rendering capped at 5); description allowed to be empty. Failure to validate disables the Add button.

### §2.6 #8 atomicity for `Both` mirrors the router pattern

Same posture as router §2.3: **saved-first, partial-success error on live failure.**

```
character_add_advantage(target=Both, ...)
  → db_add_advantage(...)              ← saved succeeds (local)
  → forward_live(...)                  ← live fails (Foundry offline)
  → Err("character/add_advantage: saved updated, live failed: <reason>")
```

The frontend toast tells the GM exactly what state they're in. No rollback. Drift badge surfaces the divergence; "Update saved" later reconciles.

For remove: same ordering. Saved-first means the GM's local notes are never inconsistent with intent — the offline state ("I just removed this") wins on the local side, and the live world catches up when reconnected (or stays as-is if the GM gives up on retry).

### §2.7 Diff layer extension: `diffAdvantages` list comparator

Plan 2's `src/lib/saved-characters/diff.ts` already ships `diffSpecialties`, a list-comparator pattern over `raw.items[]` filtered by `type === 'speciality'`. This spec adds `diffAdvantages` as a sibling, filtered by `type === 'feature'` and partitioned by `system.featuretype` (merit / flaw / background / boon).

Comparison key: a stable identity for an advantage item. Choices:
- `_id` — diverges between saved and live for `local-*` items (§2.3) → never matches.
- `name` — round-trips reliably; collision-prone if two advantages share a name (rare in V5).

Spec choice: **`name`** as the matching key, with `featuretype` as the grouping axis. Two merits both named "Iron Will" are an edge case the diff renders as a single entry; the user resolves by renaming. This matches `diffSpecialties`'s `name`-keyed behavior.

Diff entries are added/removed pairs:

```
Merit added:    "Iron Will"
Merit removed:  "Bad Sight"
Flaw added:     "Curse of the Caine"
```

Points-only changes (same name, different `system.points`) render as `"Iron Will: 2 → 3"` — caught by the same iteration.

### §2.8 #8 Roll20: live editing fast-fails (Phase 2.5 follow-up)

Roll20 sheets store advantages in repeating sections (`repeating_merits_<id>_*`), not as feature documents. The wire shape for "create a Roll20 merit row" is undefined in v1 — it'd require new attribute-name conventions and a Roll20 module path.

The router fast-fails on `target=Live` with a Roll20 source:

```
character/add_advantage:    Roll20 live editing of advantages not yet supported
character/remove_advantage: Roll20 live editing of advantages not yet supported
```

Roll20 *saved-side* editing remains supported — `db_add_advantage` walks the typed `canonical.raw` JSON regardless of source kind, so a Roll20 saved character can have advantages added/removed locally. The drift layer surfaces the divergence; the GM updates Roll20 manually until the Phase 2.5 follow-up lands a Roll20 advantage path.

**Drift blindspot for Roll20 saved-side adds:** §4.5's `diffAdvantages` early-returns `[]` for non-Foundry sources because Roll20 advantages live in repeating-section attributes, not in `raw.items[]`. So a GM who adds an advantage to a Roll20 saved-only card will see the chip on the saved card, but if a Roll20 live counterpart later appears, the diff layer **won't** flag the absence — the modal will under-report. Acceptable for v1: Roll20 saved characters with locally-added advantages are an edge case, and Phase 2.5 (which adds Roll20 repeating-section read paths) closes the gap.

### §2.9 Optimistic vs wait-for-bridge UX

Foundry `actor.update_field` and `actor.create_feature` round-trips arrive back on `bridge://characters-updated` within ~50ms of the wire send (single-user localhost). The existing pattern — bridge store re-renders cards from `bridge://characters-updated` events — is sufficient.

UI policy: **buttons enter an `aria-busy` / disabled state for the duration of the IPC call**, then re-enable on either resolution. No optimistic local-state update; the bridge store is the source of truth for live cards. Saved cards re-render from the savedCharacters store, which the IPC handler refreshes.

Buttons that fail show an inline error toast for 4s using the existing toast pattern (per ARCH §7).

---

## §3 Architecture diagram

```
Frontend (Svelte)
  Campaign.svelte
    ├─ Stat editor (#7) — +/- buttons inline on cards
    │   └─ characterSetField(target, source, sid, name, value)   ← via #6's typed wrapper
    │
    └─ Advantage editor (#8) — chip-X + Add form
        ├─ characterAddAdvantage(target, source, sid, ft, name, desc, points)
        └─ characterRemoveAdvantage(target, source, sid, ft, item_id)

src/lib/character/api.ts (extended; #6 already created the file)
  ├─ characterSetField                  ← unchanged (shipped in #6)
  ├─ characterAddAdvantage              ← new (#8)
  └─ characterRemoveAdvantage           ← new (#8)

src/lib/saved-characters/diff.ts (extended; #4 already created the file)
  └─ diffAdvantages                     ← new (#8); composed into diffCharacter

Tauri IPC: 4 commands
  character_set_field           ← shipped in #6
  patch_saved_field             ← shipped in #6
  character_add_advantage       ← new (#8)
  character_remove_advantage    ← new (#8)

src-tauri/src/tools/character.rs (extended; #6 already created the file)
  ├─ character_set_field        ← unchanged (shipped in #6)
  ├─ character_add_advantage    ← new (#8) — composes:
  │     ├─ target=Saved → db_add_advantage
  │     ├─ target=Live  → build_create_feature → send_to_source_inner
  │     └─ target=Both  → saved-first, partial-success error
  └─ character_remove_advantage ← new (#8) — composes:
        ├─ target=Saved → db_remove_advantage
        ├─ target=Live  → build_delete_item_by_id → send_to_source_inner
        └─ target=Both  → saved-first, partial-success error

src-tauri/src/db/saved_character.rs (extended)
  ├─ db_add_advantage    ← new (#8) — JSON-walks canonical.raw.items[]
  └─ db_remove_advantage ← new (#8) — JSON-walks canonical.raw.items[]

src-tauri/src/bridge/foundry/actions/actor.rs (unchanged — pre-shipped)
  ├─ build_create_feature       ← FHL Phase 1, shipped
  └─ build_delete_item_by_id    ← FHL Phase 1, shipped

vtmtools-bridge/scripts/foundry-actions/actor.js (unchanged — pre-shipped)
  ├─ "actor.create_feature"     ← FHL Phase 1, shipped
  └─ "actor.delete_item_by_id"  ← FHL Phase 1, shipped
```

No wire-protocol changes. No SQL migrations. No `BridgeSource` trait additions. No module version bump. The `vtmtools-bridge` Foundry module ships unchanged.

---

## §4 Components

### §4.1 `src-tauri/src/tools/character.rs` — extend with two new commands

Two new Tauri commands and their inner helpers, alongside the existing `character_set_field`:

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureType {
    Merit,
    Flaw,
    Background,
    Boon,
}

impl FeatureType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FeatureType::Merit      => "merit",
            FeatureType::Flaw       => "flaw",
            FeatureType::Background => "background",
            FeatureType::Boon       => "boon",
        }
    }
}

#[tauri::command]
pub async fn character_add_advantage(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    name: String,
    description: String,
    points: i32,
) -> Result<(), String> {
    do_add_advantage(
        &db.0, &bridge.0, target, source, source_id,
        featuretype, name, description, points,
    ).await
}

pub(crate) async fn do_add_advantage(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    name: String,
    description: String,
    points: i32,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("character/add_advantage: empty name".to_string());
    }
    if !(0..=10).contains(&points) {
        return Err(format!(
            "character/add_advantage: points {points} out of range 0..=10"
        ));
    }

    // Roll20 live fast-fail (§2.8).
    if target != WriteTarget::Saved && source == SourceKind::Roll20 {
        return Err(
            "character/add_advantage: Roll20 live editing of advantages not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    let do_saved = || async {
        crate::db::saved_character::db_add_advantage(
            pool,
            saved_id.unwrap(),
            featuretype.as_str(),
            &name,
            &description,
            points,
        )
        .await
    };

    let do_live = || async {
        let payload = crate::bridge::foundry::actions::actor::build_create_feature(
            &source_id,
            featuretype.as_str(),
            &name,
            &description,
            points,
        )
        .map_err(|e| format!("character/add_advantage: {e}"))?;
        let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        crate::bridge::commands::send_to_source_inner(bridge_state, source, text).await
    };

    match target {
        WriteTarget::Saved => do_saved().await,
        WriteTarget::Live  => do_live().await,
        WriteTarget::Both  => {
            do_saved().await
                .map_err(|e| format!("character/add_advantage: saved write failed: {e}"))?;
            do_live().await
                .map_err(|e| format!(
                    "character/add_advantage: saved updated, live failed: {e}"
                ))
        }
    }
}

#[tauri::command]
pub async fn character_remove_advantage(
    db: State<'_, crate::DbState>,
    bridge: State<'_, crate::bridge::BridgeConn>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    item_id: String,
) -> Result<(), String> {
    do_remove_advantage(
        &db.0, &bridge.0, target, source, source_id, featuretype, item_id,
    ).await
}

pub(crate) async fn do_remove_advantage(
    pool: &SqlitePool,
    bridge_state: &Arc<BridgeState>,
    target: WriteTarget,
    source: SourceKind,
    source_id: String,
    featuretype: FeatureType,
    item_id: String,
) -> Result<(), String> {
    if item_id.trim().is_empty() {
        return Err("character/remove_advantage: empty item_id".to_string());
    }

    if target != WriteTarget::Saved && source == SourceKind::Roll20 {
        return Err(
            "character/remove_advantage: Roll20 live editing of advantages not yet supported"
                .to_string(),
        );
    }

    let saved_id: Option<i64> = if target != WriteTarget::Live {
        Some(lookup_saved_id(pool, source, &source_id).await?)
    } else {
        None
    };

    let do_saved = || async {
        crate::db::saved_character::db_remove_advantage(
            pool, saved_id.unwrap(), featuretype.as_str(), &item_id,
        )
        .await
    };

    let do_live = || async {
        let payload = crate::bridge::foundry::actions::actor::build_delete_item_by_id(
            &source_id, &item_id,
        );
        let text = serde_json::to_string(&payload).map_err(|e| e.to_string())?;
        crate::bridge::commands::send_to_source_inner(bridge_state, source, text).await
    };

    match target {
        WriteTarget::Saved => do_saved().await,
        WriteTarget::Live  => do_live().await,
        WriteTarget::Both  => {
            do_saved().await
                .map_err(|e| format!("character/remove_advantage: saved write failed: {e}"))?;
            do_live().await
                .map_err(|e| format!(
                    "character/remove_advantage: saved updated, live failed: {e}"
                ))
        }
    }
}
```

Note: `lookup_saved_id`, `WriteTarget`, and the `BridgeState` plumbing are reused unchanged from #6. The two new helpers slot in alongside `do_set_field` without restructuring the file.

### §4.2 `src-tauri/src/db/saved_character.rs` — extend with two JSON-walking helpers

```rust
/// Append a feature item to canonical.raw.items[]. Item shape matches
/// what Foundry's actor.create_feature executor produces: type=feature,
/// system.featuretype/description/points. The `_id` is a synthesized
/// `local-<uuid>` (§2.3) that survives until the next Update Saved.
pub(crate) async fn db_add_advantage(
    pool: &SqlitePool,
    id: i64,
    featuretype: &str,
    name: &str,
    description: &str,
    points: i32,
) -> Result<(), String> {
    // Validate featuretype matches the FHL contract.
    match featuretype {
        "merit" | "flaw" | "background" | "boon" => {}
        other => return Err(format!(
            "db/saved_character.add_advantage: invalid featuretype: {other}"
        )),
    }

    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.add_advantage: {e}"))?
        .ok_or_else(|| "db/saved_character.add_advantage: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.add_advantage: deserialize failed: {e}"))?;

    let new_item = serde_json::json!({
        "_id": format!("local-{}", uuid::Uuid::new_v4()),
        "type": "feature",
        "name": name,
        "system": {
            "featuretype": featuretype,
            "description": description,
            "points": points,
        },
        "effects": [],
    });

    // canonical.raw is serde_json::Value; append to .items, materializing
    // the array if absent (only triggers for non-Foundry sources or legacy
    // payloads where items[] was never populated).
    let raw = canonical.raw.as_object_mut().ok_or_else(||
        "db/saved_character.add_advantage: canonical.raw is not an object".to_string()
    )?;
    let items = raw.entry("items".to_string())
        .or_insert_with(|| serde_json::Value::Array(vec![]));
    let arr = items.as_array_mut().ok_or_else(||
        "db/saved_character.add_advantage: canonical.raw.items is not an array".to_string()
    )?;
    arr.push(new_item);

    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.add_advantage: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.add_advantage: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.add_advantage: not found".to_string());
    }
    Ok(())
}

pub(crate) async fn db_remove_advantage(
    pool: &SqlitePool,
    id: i64,
    featuretype: &str,
    item_id: &str,
) -> Result<(), String> {
    let row = sqlx::query("SELECT canonical_json FROM saved_characters WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("db/saved_character.remove_advantage: {e}"))?
        .ok_or_else(|| "db/saved_character.remove_advantage: not found".to_string())?;

    let canonical_json: String = row.get("canonical_json");
    let mut canonical: CanonicalCharacter = serde_json::from_str(&canonical_json)
        .map_err(|e| format!("db/saved_character.remove_advantage: deserialize failed: {e}"))?;

    let raw = canonical.raw.as_object_mut().ok_or_else(||
        "db/saved_character.remove_advantage: canonical.raw is not an object".to_string()
    )?;
    let Some(items) = raw.get_mut("items").and_then(|v| v.as_array_mut()) else {
        return Err(format!(
            "db/saved_character.remove_advantage: no item with id '{item_id}'"
        ));
    };
    let original_len = items.len();
    items.retain(|item| {
        let id_match = item.get("_id").and_then(|v| v.as_str()) == Some(item_id);
        let ft_match = item
            .get("system")
            .and_then(|s| s.get("featuretype"))
            .and_then(|v| v.as_str())
            == Some(featuretype);
        !(id_match && ft_match)
    });
    if items.len() == original_len {
        return Err(format!(
            "db/saved_character.remove_advantage: no {featuretype} with id '{item_id}'"
        ));
    }

    let new_json = serde_json::to_string(&canonical)
        .map_err(|e| format!("db/saved_character.remove_advantage: serialize failed: {e}"))?;

    let result = sqlx::query(
        "UPDATE saved_characters
         SET canonical_json = ?, last_updated_at = datetime('now')
         WHERE id = ?",
    )
    .bind(&new_json)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("db/saved_character.remove_advantage: {e}"))?;

    if result.rows_affected() == 0 {
        return Err("db/saved_character.remove_advantage: not found".to_string());
    }
    Ok(())
}
```

The `featuretype` parameter on `db_remove_advantage` is a defense-in-depth check: prevents the UI from accidentally deleting a discipline document via the same id. In practice the UI passes the featuretype it rendered the chip from, so this can never legitimately fail without a UI bug.

`uuid` is **not** currently a dependency of `src-tauri` (verified against `Cargo.toml`). Plan B's first task adds `uuid = { version = "1", features = ["v4"] }` to `[dependencies]`. Alternative: synthesize `local-<i64>` from a monotonic counter or a hash of `(saved_id, name, now)` to avoid the dep — rejected because UUIDs match Foundry's existing `_id` shape (16-char base36 strings) and keep the saved-side stylistically aligned.

### §4.3 `src/lib/character/api.ts` — extend with two new wrappers

```ts
// Existing exports unchanged.

import type { FeatureType } from '../../types';

export const characterAddAdvantage = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  featuretype: FeatureType,
  name: string,
  description: string,
  points: number,
): Promise<void> =>
  invoke<void>('character_add_advantage', {
    target, source, sourceId, featuretype, name, description, points,
  });

export const characterRemoveAdvantage = (
  target: WriteTarget,
  source: SourceKind,
  sourceId: string,
  featuretype: FeatureType,
  itemId: string,
): Promise<void> =>
  invoke<void>('character_remove_advantage', {
    target, source, sourceId, featuretype, itemId,
  });
```

`FeatureType` mirrors the Rust enum:

```ts
// src/types.ts (extend)
export type FeatureType = 'merit' | 'flaw' | 'background' | 'boon';
```

### §4.4 `src/tools/Campaign.svelte` — UI changes (#7 + #8)

**#7 stat editor:**

Live cards (existing render path, lines 331–654 today). Buttons inline next to the existing visualizations. Three locations:

1. **Hunger drops** (lines 349–355): two small buttons flanking the drop row — `−` decreases `hunger` (clamped 0..=5), `+` increases.
2. **Conscience track / Stains** (lines 365–377): two-row inline control below the track — first row `−/+ Humanity`, second row `−/+ Stains`.
3. **Health track** + **Willpower track** (lines 381–405): each track gets a 4-button cluster on the right edge — `−Sup +Sup` and `−Agg +Agg`.

When `char.source === 'roll20'`, all live buttons render but pre-emptively show "Roll20 live editing not supported (Phase 2.5)" in a tooltip and are disabled. The button tier code centralizes this check:

```ts
function liveEditAllowed(char: BridgeCharacter): boolean {
  return char.source === 'foundry';
}
```

Saved cards (existing render path, lines 670–686). When the card is **offline-saved-only** (no live counterpart, i.e. `!characters.some(live => keyEq(live, saved))`), the same +/- controls render on the saved card with `target='saved'`. When a saved card has a live counterpart, no +/- buttons render — the live card owns editing.

The button click handler:

```ts
async function tweakField(
  char: BridgeCharacter,
  field: CanonicalFieldName,
  delta: number,
  current: number,
  range: [number, number],
) {
  const next = clamp(current + delta, range[0], range[1]);
  if (next === current) return;
  try {
    await characterSetField('live', char.source, char.source_id, field, next);
  } catch (e) {
    showToast(String(e));
  }
}
```

Per-field range:
| Field | Range |
|---|---|
| `hunger` | 0..=5 |
| `humanity` | 0..=10 |
| `humanity_stains` | 0..=10 |
| `health_superficial` | 0..=20 |
| `health_aggravated` | 0..=20 |
| `willpower_superficial` | 0..=20 |
| `willpower_aggravated` | 0..=20 |

(Matches `apply_canonical_field`'s ranges in `shared/canonical_fields.rs`.)

**#8 advantage editor:**

The existing feats section (lines 506–586 in Campaign.svelte) is collapsible — `expandedFeats.has(charKey)` gates its render. Adding chip-X / Add controls inside the existing four `feat-row` blocks (Merits / Flaws / Backgrounds / Boons).

Each feature chip becomes a hover-target with an `×` button on the right edge. Click → confirmation prompt ("Remove merit 'Iron Will'?") → `characterRemoveAdvantage(target='live', char.source, char.source_id, 'merit', m._id)`.

Each row (after the chips) gets a `+ Add` chip-shaped button. Click → inline form (§2.5 layout). Submit → `characterAddAdvantage(target='live', ...)`.

Saved cards (offline-only): same pattern with `target='saved'`. `characterRemoveAdvantage` uses the item's `_id` (which is `local-*` for items added saved-side, or the original Foundry id for items captured from a previous "Save locally").

Roll20 live cards: chip-X and Add buttons hidden (parallel to #7's policy).

### §4.5 `src/lib/saved-characters/diff.ts` — extend with `diffAdvantages`

Pattern mirrors `diffSpecialties` (Plan 2). Filter `raw.items[]` by `type === 'feature'`, group by `system.featuretype`, compare by `name`:

```ts
function collectAdvantages(raw: unknown): Record<string, Map<string, number>> {
  // Map<name, points> per featuretype.
  const out: Record<string, Map<string, number>> = {
    merit: new Map(), flaw: new Map(), background: new Map(), boon: new Map(),
  };
  const items = (raw as { items?: unknown[] } | null)?.items ?? [];
  for (const item of items as Array<Record<string, unknown>>) {
    if (item.type !== 'feature') continue;
    const sys = item.system as Record<string, unknown> | undefined;
    const ft = sys?.featuretype as string | undefined;
    const name = item.name as string | undefined;
    if (!ft || !(ft in out) || !name) continue;
    const points = typeof sys?.points === 'number' ? sys.points : 0;
    out[ft].set(name, points);
  }
  return out;
}

function diffAdvantages(saved, live): DiffEntry[] {
  if (saved.source !== 'foundry') return [];
  const savedMap = collectAdvantages(saved.raw);
  const liveMap  = collectAdvantages(live.raw);
  const entries: DiffEntry[] = [];
  for (const ft of ['merit', 'flaw', 'background', 'boon'] as const) {
    const sv = savedMap[ft];
    const lv = liveMap[ft];
    const allNames = new Set([...sv.keys(), ...lv.keys()]);
    for (const name of allNames) {
      const before = sv.get(name);
      const after  = lv.get(name);
      if (before === undefined && after !== undefined) {
        entries.push({
          key: `${ft}.${name}`,
          label: `${cap(ft)}: ${name}`,
          before: '—',
          after: after > 0 ? `+ (${after})` : 'added',
        });
      } else if (after === undefined && before !== undefined) {
        entries.push({
          key: `${ft}.${name}`,
          label: `${cap(ft)}: ${name}`,
          before: before > 0 ? `(${before})` : 'present',
          after: '—',
        });
      } else if (before !== after) {
        entries.push({
          key: `${ft}.${name}`,
          label: `${cap(ft)}: ${name}`,
          before: String(before),
          after: String(after),
        });
      }
    }
  }
  return entries;
}

// Composed into diffCharacter alongside diffSpecialties:
export function diffCharacter(saved, live): DiffEntry[] {
  return [
    ...DIFFABLE_PATHS.map(...).filter(...).map(...),  // existing canonical paths
    ...diffSpecialties(saved, live),                  // existing
    ...diffAdvantages(saved, live),                   // NEW
  ];
}
```

Roll20 saved characters skip via the `source !== 'foundry'` guard — Roll20 advantages live in repeating-section attributes, not feature documents, so this comparator wouldn't apply.

---

## §5 Data flows

### §5.1 `+1 hunger` on a Foundry live card (#7)

```
+ button on hunger drop row
  → tweakField(char, 'hunger', +1, currentHunger, [0, 5])
  → characterSetField('live', 'foundry', sid, 'hunger', 3)
  → IPC character_set_field
  → router target=Live → forward_live → do_set_attribute → build_set_attribute
  → wire: actor.update_field { path: "system.hunger.value", value: "3" }
  → Foundry executes; bridge://characters-updated re-renders card with hunger=3
  → drift badge appears on saved counterpart (if saved.hunger ≠ 3)
```

### §5.2 `+1 humanity` on an offline saved-only Foundry card (#7)

```
+ button on saved card's conscience row
  → tweakField(saved.canonical, 'humanity', +1, currentHumanity, [0, 10])
  → characterSetField('saved', 'foundry', sid, 'humanity', 8)
  → IPC character_set_field
  → router target=Saved → db_patch_field → savedCharacters store reload
  → saved card re-renders with humanity=8
```

### §5.3 Add a Merit on a Foundry live card (#8)

```
+ Add Merit button → inline form → user fills "Iron Will" / 2 / "Description" → Add
  → characterAddAdvantage('live', 'foundry', sid, 'merit', 'Iron Will', 'Description', 2)
  → IPC character_add_advantage
  → router target=Live → build_create_feature(sid, "merit", "Iron Will", "...", 2)
  → wire: actor.create_feature { actor_id, featuretype, name, description, points }
  → Foundry creates an Item document; bridge://characters-updated re-renders
  → new chip "Iron Will (••)" appears in the Merits row
```

### §5.4 Remove a Boon on an offline saved-only Foundry card (#8)

```
× on chip → confirm → characterRemoveAdvantage('saved', 'foundry', sid, 'boon', '<item._id>')
  → IPC character_remove_advantage
  → router target=Saved → db_remove_advantage(saved_id, "boon", item_id)
  → JSON-walk canonical.raw.items[] → retain(...) drops the matching item
  → savedCharacters store reload → chip disappears
```

### §5.5 Roll20 live → fast-fail (#7 + #8)

```
+ button on a Roll20 live card hunger row (UI is pre-disabled but if hit programmatically)
  → characterSetField('live', 'roll20', sid, 'hunger', 3)
  → router fast-fail (router spec §2.8)
  → Err("character/set_field: Roll20 live editing of canonical names not yet supported")
  → toast surfaces error
```

### §5.6 Both target with disconnected Foundry on Add Merit (#8)

```
characterAddAdvantage('both', 'foundry', sid, 'merit', ...)
  → router: db_add_advantage succeeds → saved row gets new local-<uuid> item
  →         build_create_feature → send_to_source_inner → no outbound channel → no-op (Ok)
  → Result: saved updated, live unchanged. drift badge appears.
```

Note: `send_to_source_inner` is a no-op for disconnected sources (returns Ok), matching the existing `bridge_set_attribute` semantics. So `target=Both` against a disconnected Foundry succeeds silently rather than partial-success-erroring. This is consistent with the router's #6 behavior — see router spec test `live_target_disconnected_source_no_op_succeeds`. The partial-success error path triggers only when the Foundry source builds the payload AND fails (e.g., `actor_id` not found at handler time, or a future explicit error envelope).

---

## §6 Error handling

Follows ARCHITECTURE.md §7: Rust commands return `Result<T, String>` with module-stable prefixes; frontend catches in API wrappers and surfaces via toast / inline state.

| Scenario | Behavior | Module prefix |
|---|---|---|
| `name` ∉ ALLOWED_NAMES (#7) | Router fast-fail | `character/set_field: unknown field '<name>'` |
| `name` value out of range (#7) | `apply_canonical_field` returns Err | `character/set_field: '<name>' expects integer LO..=HI, got N` |
| Empty name on add_advantage (#8) | Router fast-fail | `character/add_advantage: empty name` |
| `points` out of range on add_advantage (#8) | Router fast-fail | `character/add_advantage: points N out of range 0..=10` |
| Invalid featuretype (#8 — IPC deserialization) | Tauri returns `Err(serde error)` automatically | n/a |
| Empty `item_id` on remove_advantage (#8) | Router fast-fail | `character/remove_advantage: empty item_id` |
| Roll20 live add/remove advantage (#8) | Router fast-fail (Phase 2.5 deferral, §2.8) | `character/<verb>_advantage: Roll20 live editing of advantages not yet supported` |
| `target=Saved`/`Both`, no saved row (#7+#8) | `lookup_saved_id` Err | `character/<command>: no saved row for <source>/<sid>` |
| `target=Saved` remove, item id not in saved blob (#8) | DB layer Err | `db/saved_character.remove_advantage: no <featuretype> with id '<id>'` |
| `target=Both` saved fails (#7+#8) | Fatal — live not attempted | `character/<command>: saved write failed: <reason>` |
| `target=Both` saved succeeds, live fails (#7+#8) | Partial-success error | `character/<command>: saved updated, live failed: <reason>` |
| Foundry rejects async (later, e.g. actor_id not found at handler time) | Plan 0 `bridge://foundry/error` toast | n/a — handled by Plan 0 path |
| `canonical.raw.items` is not an array (#8 saved-side) | DB layer Err — corrupt blob | `db/saved_character.<verb>_advantage: canonical.raw.items is not an array` |

Errors surface to the GM via the existing toast pattern (Plan 0 introduced `bridge://foundry/error` event handling; this work reuses the same toast infrastructure for command-result errors).

---

## §7 Testing

`./scripts/verify.sh` covers the full gate. Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

### #7 stat editor — tests: NOT required (UI-only, no logic)

#7 is a pure-frontend work item that wires existing `characterSetField` calls to existing UI surfaces. The router and `apply_canonical_field` are already test-covered (#6). The only new logic is `clamp` against per-field ranges and the `liveEditAllowed` Roll20 guard — both trivially correct from inspection.

`verify.sh`'s `npm run check` + `npm run build` are sufficient. Manual gate: connect Foundry, click +/- on each canonical field, verify live card re-renders within a tick and saved counterpart shows drift.

### #8 advantage editor — tests: required

#### `db/saved_character.rs` (extend `#[cfg(test)] mod tests`)

- `db_add_advantage` happy path: read → mutate → re-read; new item present in `canonical.raw.items[]` with `_id` matching `^local-` and the right featuretype/name/description/points.
- `db_add_advantage` invalid featuretype → Err.
- `db_add_advantage` missing id → Err.
- `db_add_advantage` materializes empty `items[]` array if absent (legacy / Roll20 saved blobs).
- `db_remove_advantage` happy path: existing item disappears; `last_updated_at` bumps.
- `db_remove_advantage` missing id → Err.
- `db_remove_advantage` id matches but featuretype mismatch → Err (defense-in-depth).
- `db_remove_advantage` no `items` key → Err.

#### `tools/character.rs` (extend `#[cfg(test)] mod tests`)

- `add_advantage` empty name → Err with `character/add_advantage: empty name`.
- `add_advantage` points 11 → Err with range message.
- `add_advantage` Roll20 live → Err (Phase 2.5 deferral).
- `add_advantage` Roll20 saved → Ok (saved-side editing works for any source).
- `add_advantage` target=Saved → DB updated; live channel unchanged (verify via `_rx` not receiving anything).
- `add_advantage` target=Live → wire payload sent; saved untouched.
- `add_advantage` target=Both happy path → both writes; payload arrives on `_rx`.
- `add_advantage` target=Both saved succeeds, live fails (`AlwaysErrSource` reuse) → partial-success Err; saved row reflects the change.
- `remove_advantage` empty item_id → Err.
- `remove_advantage` Roll20 live → Err.
- `remove_advantage` target=Saved → DB updated.
- `remove_advantage` target=Live → wire payload sent.
- `remove_advantage` target=Both happy path → both writes.

Reuse the `make_bridge_state`, `seed_saved_row`, `StubFoundrySource`, `AlwaysErrSource` helpers from #6's tests. Add a saved-row seed variant that includes `raw.items[]` with one pre-existing feature for remove tests.

### Manual verification (#7 + #8)

From a Foundry-connected dev session against a character that has a saved counterpart:

1. **#7:** Click `+` on hunger → live card re-renders within a tick; drift badge appears on saved counterpart.
2. **#7:** Click `−` on humanity → same flow; conscience track shrinks.
3. **#7:** Disconnect Foundry, click `+` on a saved-only card's humanity → saved store updates.
4. **#7:** Reconnect Foundry → drift badge surfaces on the live counterpart (live unchanged from before disconnect).
5. **#8:** Open feats section, click `+ Add Merit` → form appears; submit "Test Merit" / 2 / "test desc" → chip appears in Merits row within a tick.
6. **#8:** Hover the chip, click `×` → confirmation prompt → confirm → chip disappears.
7. **#8:** With Foundry disconnected, add a merit on a saved-only card → saved card re-renders with new chip; "_id" inspection shows `local-<uuid>`.
8. **Roll20:** All live +/- and chip controls show disabled state with tooltip.

---

## §8 Files inventory

#### #7 (one plan)

| Action | File | Reason |
|---|---|---|
| Modify | `src/tools/Campaign.svelte` | Add inline +/- buttons + handlers + range constants + `liveEditAllowed` |
| Modify | `src/lib/character/api.ts` | (no changes; already shipped) |

Total: 1 modification. No Rust changes, no IPC, no schema, no migrations.

#### #8 (separate plan)

| Action | File | Reason |
|---|---|---|
| Modify | `src-tauri/src/tools/character.rs` | Add `character_add_advantage` + `character_remove_advantage` commands and inner helpers (§4.1) |
| Modify | `src-tauri/src/db/saved_character.rs` | Add `db_add_advantage` + `db_remove_advantage` JSON-walking helpers + tests (§4.2) |
| Modify | `src-tauri/src/lib.rs` | Register the two new commands in `invoke_handler!` |
| Modify | `src/lib/character/api.ts` | Add `characterAddAdvantage` + `characterRemoveAdvantage` typed wrappers (§4.3) |
| Modify | `src/types.ts` | Add `FeatureType` literal type |
| Modify | `src/tools/Campaign.svelte` | Chip-X + Add form (§4.4) |
| Modify | `src/lib/saved-characters/diff.ts` | Add `diffAdvantages` list comparator + compose into `diffCharacter` (§4.5; reuses existing `cap()` helper at line 23) |
| Modify | `src-tauri/Cargo.toml` | Add `uuid = { version = "1", features = ["v4"] }` for `local-<uuid>` ids (§4.2) |

Total: 8 modifications (one is the `Cargo.toml` dep-add). No new files, no SQL migrations, no wire variants, no module bump. Tauri command surface grows from 39 → 41.

---

## §9 Anti-scope (per ARCH §11)

| Anti-scope | Why |
|---|---|
| **Skills/attributes editing** (#7) | Live in `canonical.raw.system.*`; deferred to Phase 2.5 alongside Roll20 mappings (§2.1) |
| **Blood Potency +/-** (#7) | UX policy: BP changes are book-keeping events, not mid-session resource flow (§2.4) |
| **Roll20 live advantage editing** (#8) | No defined wire shape for Roll20 repeating-section advantages; Phase 2.5 follow-up (§2.8) |
| **Roll20 live canonical editing** (#7) | Same Phase 2.5 follow-up; router stub already in place (router spec §2.8) |
| **Item-level field edits other than create/delete** (#8) | E.g., editing the description of an existing merit. Out of scope; deferred to a "merit detail editor" feature if it's ever wanted |
| **Bulk operations** (#7+#8) | E.g., "drop all hunger to 0", "remove all merits". v1 ships single-field / single-item primitives |
| **Confirmation modals** beyond simple-window-confirm (#8) | Modal infrastructure exists for Compare; advantage delete uses `window.confirm` for v1 simplicity. Custom modal can land later if churn demands |
| **Item.create wire helper for non-feature types** (#8) | E.g., adding a new specialty or weapon. `actor.create_feature` is feature-specific; specialties/weapons need different umbrella shapes |
| **Optimistic UI updates** (#7+#8) | Foundry round-trip is fast enough; bridge store + `aria-busy` is sufficient (§2.9) |
| **New Plan-0 wire variants** | Both commands use already-shipped helpers (`actor.update_field`, `actor.create_feature`, `actor.delete_item_by_id`) |
| **`vtmtools-bridge` module changes** | No new JS executors, no module bump |
| **Compare modal restructure** | Plan 2's modal automatically renders new `diffAdvantages` entries via the existing list (§4.5) |

---

## §10 Plan structure

This spec produces **two implementation plans** that ship sequentially:

### Plan A — `2026-05-NN-stat-editor-ui.md` (#7)

Pure-frontend. ~150 LoC across `Campaign.svelte` plus a few helpers. Tests not required (§7). Tasks (rough sketch):
1. Add `liveEditAllowed`, range constants, `tweakField`, `+ -` button helper component
2. Wire hunger +/-
3. Wire humanity / stains +/-
4. Wire health superficial / aggravated +/-
5. Wire willpower superficial / aggravated +/-
6. Apply same controls to offline-saved-only saved cards
7. Disable controls on Roll20 live cards with tooltip
8. Manual smoke + `verify.sh` + commit (footer: `Closes #7`)

Plan A depends only on #6's router (shipped). Can ship today.

### Plan B — `2026-05-NN-advantage-editor.md` (#8)

Backend + frontend + diff layer. ~400 LoC across the seven modified files in §8.

Suggested task partition (independent enough for one-implementer-per-task per CLAUDE.md's lean plan execution rule):

1. Rust: add `uuid` dep to `Cargo.toml`
2. Rust: add `db_add_advantage` + `db_remove_advantage` + tests (§4.2 + §7)
3. Rust: add `character_add_advantage` + `character_remove_advantage` commands + inner helpers + tests (§4.1 + §7)
4. Rust: register both commands in `lib.rs::invoke_handler!`
5. TS: add `FeatureType` to `types.ts` and `characterAddAdvantage` / `characterRemoveAdvantage` to `api.ts` (§4.3)
6. TS: extend `diff.ts` with `diffAdvantages` and compose into `diffCharacter` (§4.5; uses existing `cap()`)
7. Svelte: chip-X buttons on existing feature chips (§4.4)
8. Svelte: per-category Add form + submit handler (§4.4)
9. Svelte: same controls on offline-saved-only saved cards
10. Svelte: hide controls on Roll20 live cards
11. Manual smoke + `verify.sh` + commit (footer: `Closes #8`)

Plan B depends on Plan A only for visual coherence (the chip-X buttons live in the same Campaign.svelte file). Functional dependency: none — Plan B can ship before Plan A if convenient.

### Why two plans, not one

Per CLAUDE.md "Lean plan execution": tasks within a plan share scene-setting and serialize through a single implementer. Plan A is small enough to fit one session; Plan B is large and touches a different layer (Rust + diff). Splitting keeps each plan's scope coherent and lets `verify.sh` run twice with smaller diffs to inspect.

The two plans ship independently to master. No cross-plan rebasing; no shared task IDs.

### Dependencies

```
#6 (router) [SHIPPED] ─────► Plan A (#7)
                       └───► Plan B (#8)
```

Both plans can run in parallel worktrees if desired (per `superpowers:using-git-worktrees`). The shared edit point is `Campaign.svelte` — handle by anti-scope: Plan A only touches the per-field stat-row UI; Plan B only touches the feats section.

---

## §11 How #7 and #8 compose existing primitives

(Made explicit per the router spec §10 precedent — so future plans don't redesign these on the fly.)

### #7 composes #6's router

Each editable canonical stat on a **live** card:

```ts
characterSetField('live', char.source, char.source_id, '<canonical-name>', newValue);
```

For an **offline-saved-only** card:

```ts
characterSetField('saved', saved.source, saved.sourceId, '<canonical-name>', newValue);
```

No new Tauri commands. No new error-handling pathways. Pure composition of #6.

### #8 composes FHL Phase 1 helpers

Live add: `bridge::foundry::actions::actor::build_create_feature` (already shipped) → `send_to_source_inner` (already shipped).

Live remove: `bridge::foundry::actions::actor::build_delete_item_by_id` (already shipped) → `send_to_source_inner` (already shipped).

Saved add/remove: new JSON-walking helpers `db_add_advantage` / `db_remove_advantage` (§4.2). These mirror the design style of `db_patch_field` from #6 but operate on `canonical.raw.items[]` instead of typed fields.

Diff layer: new `diffAdvantages` (§4.5) follows the exact pattern of Plan 2's `diffSpecialties` — same `collect → set-merge → diff entries` shape.

### Pattern catalog (for future Phase 2.5+ work)

The two helpers in #8 establish a pattern that future item-level features can mirror:

- **Live composer:** call the Foundry action builder directly, dispatch via `send_to_source_inner`.
- **Saved composer:** walk `canonical.raw.<key>[]` with serde_json, mutate, write back.
- **Both target:** saved-first, partial-success error.
- **Roll20:** fast-fail at the router with module-prefixed message.
- **Diff:** add a `diffXxx` list comparator alongside the existing ones; compose into `diffCharacter`.

When Phase 2.5 adds skills/attributes editing, it inherits #6's router pattern. When future phases add discipline / specialty / weapon CRUD, they inherit #8's pattern from this spec.

---

## §12 Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

| Stage | What `verify.sh` proves |
|---|---|
| `cargo test` | New `db/saved_character.rs` and `tools/character.rs` tests for #8 — happy path, validation, partial-success error formatting, Roll20 deferral |
| `npm run check` | `FeatureType` literal type matches Rust enum; `WriteTarget` parameters typed correctly; `diffAdvantages` returns `DiffEntry[]` |
| `npm run build` | Svelte compiles; no TS regressions |

Manual gate (done once per plan, not per-task): the 8-item smoke list in §7.

---

## §13 Open questions

### Resolved during this spec

- ✅ **#7 v1 scope** — canonical fields only; skills/attributes deferred to Phase 2.5 (§2.1; resolves the router spec's deferral against #7's title)
- ✅ **Which canonical fields get +/- buttons?** — seven of eight; Blood Potency stays read-only (§2.4)
- ✅ **#8 command shape** — two separate commands (`add_advantage`, `remove_advantage`), not a single `advantage_set` (§2.2)
- ✅ **#8 saved-side `_id` strategy** — `local-<uuid>` prefix (§2.3)
- ✅ **#8 advantage UI shape** — chip-X + per-category Add form, inline (§2.5)
- ✅ **#8 Both atomicity** — saved-first, partial-success error (§2.6; mirrors router §2.3)
- ✅ **#8 diff layer** — `diffAdvantages` mirrors `diffSpecialties` (§2.7)
- ✅ **#8 Roll20** — live editing fast-fails; Phase 2.5 follow-up (§2.8)
- ✅ **Optimistic vs wait** — wait-for-bridge with `aria-busy` is sufficient (§2.9)
- ✅ **One spec, two plans** — packaging matches FHL Phase 2 precedent (§10)

### Outstanding (deferred to later phases)

- **Phase 2.5:** skills/attributes inline editing (composes the same #6 router with extended `ALLOWED_NAMES`); Roll20 canonical-name attr mappings; Roll20 advantage-creation wire shape.
- **Phase 3:** "edit-this-merit" detail editor (description / points changes on existing items via `actor.update_item_field`, already shipped). Not on any roadmap milestone yet — file when GM workflow demands it.

---

## §14 Pointers

- `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 2 — original sketch this design refines
- `docs/superpowers/specs/2026-05-02-character-set-field-router-design.md` — sibling spec for #6; #7 directly composes its router
- `docs/superpowers/specs/2026-04-26-foundry-helper-library-roadmap.md` — `actor.*` umbrella; FHL Phase 1 shipped `actor.create_feature` + `actor.delete_item_by_id`
- `ARCHITECTURE.md` §4 (Tauri IPC + bridge protocol), §7 (error handling), §9 (Add a Tauri command)
- `docs/reference/foundry-vtm5e-paths.md` — WoD5e actor schema, including item-document shapes
- `docs/reference/foundry-vtm5e-actor-sample.json` — live actor wire blob; ground truth for `raw.items[]` shape
