# GM Screen — modifier dashboard (stage 1) — design spec

> **Status:** designed; ready for plan-writing.
> **Roadmap fit:** Phase 3 (during-play tooling), sub-feature 3A. Sibling of 3B (roll mirroring).
> **Audience:** anyone implementing the new tool, or extending it later with rooms/bundles, V5 auto-fold, or Foundry effect mirroring.
> **Source roadmap:** `docs/superpowers/specs/2026-04-30-character-tooling-roadmap.md` §5 Phase 3 (recast as during-play tooling).

---

## §1 What this is

A new tool — `🛡 GM Screen` — that gives the storyteller a live, per-character view of modifiers (buffs / debuffs / situational effects / merit-derived bonuses) currently hanging on each character. **Stage 1** of a future full GM-screen tool; later stages may add rooms/bundles, scene tracking, initiative, etc.

The tool's posture is **observability + curation, not enforcement**. The GM eyeballs the modifier stack when calling for a roll; the tool does not auto-fold modifiers into V5 dice pools, and never automatically applies effects to the sheet. The GM may explicitly mirror a card's effects to the bound merit's `system.bonuses[]` on Foundry via a per-card push button (Phase 2.5, §11A) — but this is always a manual, opt-in, single-button-press action, never automatic. This sidesteps the "TTRPG situations are infinitely variable" problem — the tool remembers and presents, the GM decides.

The headline interactions:

- **Vertical character list** down the page, each character a row.
- **Horizontal row of modifier cards** to the right of each character header.
- **Per-card on/off toggle** — sticky state. GM clicks to enable when the in-fiction effect becomes true; click again to disable.
- **Cog-wheel quick-edit** on each card — attach a structured effect (pool delta, difficulty delta, free-text note) or compose multiple effects.
- **Status palette dock** on the side with reusable templates (`Slippery`, `Blind`, `Loud environment`); one-click applies an independent **copy** to a character.
- **Tag-chip filter bar** at the top — derive freeform tags from cards, filter visible cards by tag. **Active cards are pinned past the filter** (never hidden by it).
- **Hide / show hidden** — GM hides cards they don't want cluttering the row; a toolbar toggle reveals them.

## §2 Composition — existing pieces this builds on

| Phase / piece | What it provides | How GM Screen uses it |
|---|---|---|
| Phase 1 — `saved_characters` table | Per-character persistence anchor | Modifier records share the same `(source, source_id)` composite key. Cascade-friendly join. |
| Phase 1 — bridge cache (`BridgeState.characters`) | Live `CanonicalCharacter` map | The GM Screen reads from the existing `bridgeStore` for live character data. |
| Phase 2 — advantage editor (#8) | Per-character merit/flaw/background/boon items in `canonical.raw.items` | Auto-derived advantage cards in the modifier row read from the same `_id`. |
| Library tools (`AdvantagesManager`, `AdvantageForm`) | Tag chip-editor + tag-derived filter chips UI | Reused for modifier-card tag UI and the GM Screen filter bar. |
| Phase 3 (future, 3B) — V5 auto-fold | `shared/v5/SkillCheckInput` will gain `Vec<ModifierEffect>` | Future "Roll …" button per character feeds active non-hidden modifiers into the helper. Strictly additive; no shape change here. |
| Phase 4 (future) — Foundry effect mirror | Future `actor.apply_active_effect` helper on the FHL roadmap | A future `ModifierBinding::FoundryEffect { effect_id }` variant carries the round-trip key. Strictly additive. |

## §3 Modifier sources

Three sources, each with a different binding shape:

| Source | Binding | Card creation flow |
|---|---|---|
| **Advantage-derived** | `binding: { kind: "advantage", item_id: <raw.items[]._id> }` | Auto-derived at render time from the character's `canonical.raw.items` filtered to `type === 'feature'`. DB row materialized only when GM engages the card (attaches an effect, toggles active, hides). |
| **Free-floating** | `binding: { kind: "free" }` | GM clicks `+ Add modifier` on a character row. DB row materialized immediately. |
| **Status template instance** | `binding: { kind: "free" }`, `origin_template_id: <template.id>` | GM applies a palette template to a character. DB row materialized immediately as an independent **copy** (palette edits do NOT propagate to instances). |

**Roll20 advantage cards** ship behind a flag in stage 1. Reading Roll20's `repeating_meritsflaws_*` rows is new code that lines up better with Phase 2.5's broader Roll20 advantage-editing wave. Stage 1 ships free-floating + palette modifiers for Roll20 characters, no auto-spawned advantage cards. (This is symmetric with the existing Phase 2 §2.8 deferral of Roll20 live advantage editing.)

## §4 Domain model & schema

### Rust types

```rust
// src-tauri/src/db/modifier.rs (new module)

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct CharacterModifier {
    pub id: i64,
    pub source: SourceKind,                 // 'roll20' | 'foundry'
    pub source_id: String,                  // matches CanonicalCharacter.source_id
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,       // 0..N effects (alpha shape)
    pub binding: ModifierBinding,
    pub tags: Vec<String>,                  // freeform card tags ("Social", "Combat", …)
    pub is_active: bool,
    pub is_hidden: bool,
    pub origin_template_id: Option<i64>,    // provenance only, no FK
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "snake_case", tag = "kind")]
pub enum ModifierBinding {
    Free,
    Advantage { item_id: String },
    // Future variants (NOT in stage 1):
    //   Room { room_id: i64 }              — rooms/bundles future
    //   FoundryEffect { effect_id: String } — Phase 4 mirror
}

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct ModifierEffect {
    pub kind: ModifierKind,
    pub scope: Option<String>,              // freeform: "Social", "balance tests", "vs supernatural"
    pub delta: Option<i32>,                 // None for note kind
    pub note: Option<String>,               // None for pool/difficulty kinds
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind { Pool, Difficulty, Note }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct StatusTemplate {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

// Argument-shape helpers (validated at IPC boundary)

#[derive(Debug, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct NewCharacterModifier {
    pub source: SourceKind,
    pub source_id: String,
    pub name: String,
    pub description: String,
    pub effects: Vec<ModifierEffect>,
    pub binding: ModifierBinding,
    pub tags: Vec<String>,
    pub origin_template_id: Option<i64>,
}

#[derive(Debug, Deserialize)] #[serde(rename_all = "camelCase")]
pub struct ModifierPatch {
    pub name: Option<String>,
    pub description: Option<String>,
    pub effects: Option<Vec<ModifierEffect>>,
    pub tags: Option<Vec<String>>,
}
```

### Schema

```sql
-- src-tauri/migrations/0005_modifiers.sql

CREATE TABLE IF NOT EXISTS character_modifiers (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    source               TEXT    NOT NULL CHECK(source IN ('roll20','foundry')),
    source_id            TEXT    NOT NULL,
    name                 TEXT    NOT NULL,
    description          TEXT    NOT NULL DEFAULT '',
    effects_json         TEXT    NOT NULL DEFAULT '[]',
    binding_json         TEXT    NOT NULL DEFAULT '{"kind":"free"}',
    tags_json            TEXT    NOT NULL DEFAULT '[]',
    is_active            INTEGER NOT NULL DEFAULT 0,
    is_hidden            INTEGER NOT NULL DEFAULT 0,
    origin_template_id   INTEGER,
    created_at           TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at           TEXT    NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_modifiers_char
    ON character_modifiers(source, source_id);

CREATE TABLE IF NOT EXISTS status_templates (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    description     TEXT    NOT NULL DEFAULT '',
    effects_json    TEXT    NOT NULL DEFAULT '[]',
    tags_json       TEXT    NOT NULL DEFAULT '[]',
    created_at      TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now'))
);
```

### Rationale

- **No FK to `saved_characters`.** Modifiers anchor to `(source, source_id)` directly, joining the live cache OR a saved row OR neither (orphan). Symmetric with how `saved_characters` joins the live cache. A character that's only ever live can carry modifiers; a character that's later saved automatically inherits them by key match; a saved character that's never live still has its modifier list available.
- **`binding_json` as a tagged JSON enum.** Easy to extend (rooms / Foundry effect mirror) without a migration — just deserialize a new variant.
- **`origin_template_id` is provenance only**, no FK. Survives template deletion; lets the UI render a "from template Slippery" subtitle on instances. Not a behavioral link.
- **No unique constraint** on `(source, source_id, binding.item_id)`. A character could plausibly have two modifier cards bound to the same merit (compound merit with two different effects expressed as two cards).

## §5 Tauri command surface

Twelve new commands across two modules. Total command count grows from 31 → 43.

### `src-tauri/src/db/modifier.rs`

```rust
list_character_modifiers(source: SourceKind, source_id: String) -> Vec<CharacterModifier>
list_all_character_modifiers() -> Vec<CharacterModifier>
add_character_modifier(input: NewCharacterModifier) -> CharacterModifier
update_character_modifier(id: i64, patch: ModifierPatch) -> CharacterModifier
delete_character_modifier(id: i64) -> ()
set_modifier_active(id: i64, is_active: bool) -> ()
set_modifier_hidden(id: i64, is_hidden: bool) -> ()
materialize_advantage_modifier(
    source: SourceKind,
    source_id: String,
    item_id: String,
    name: String,
    description: String,
) -> CharacterModifier
// Idempotent upsert. Called the first time GM engages a derived advantage card.
// If a row already exists for (source, source_id, binding.item_id), returns it unchanged.
// Otherwise inserts a new row with empty effects, is_active=false, is_hidden=false.
```

### `src-tauri/src/db/status_template.rs`

```rust
list_status_templates() -> Vec<StatusTemplate>
add_status_template(input: NewStatusTemplate) -> StatusTemplate
update_status_template(id: i64, patch: StatusTemplatePatch) -> StatusTemplate
delete_status_template(id: i64) -> ()
```

No separate `apply_template_to_character` command — the frontend reads the template, builds a `NewCharacterModifier` (carrying the template's effects + tags + name + description and `origin_template_id: Some(template.id)`), and calls `add_character_modifier`. Provenance flows through the input shape.

All twelve commands register in `src-tauri/src/lib.rs` via `invoke_handler(tauri::generate_handler![...])` (ARCH §9). Each gets a typed wrapper in `src/lib/modifiers/api.ts`; components never call `invoke(...)` directly (ARCH §4 / §5).

Errors prefix with `db/modifier.<op>:` and `db/status_template.<op>:` per ARCH §7. Partial-failure cases:

- `update_character_modifier` on missing id → `Err("db/modifier.update: not found")`
- `delete_character_modifier` on missing id → `Err("db/modifier.delete: not found")`
- `materialize_advantage_modifier` is upsert-by-design — never errors on idempotent-call.
- `add_character_modifier` with empty `name` → `Err("db/modifier.add: empty name")`.

Capability impact: none. The existing `core:default + opener:default` grant in `src-tauri/capabilities/default.json` covers them (ARCH §8).

## §6 Frontend file inventory

| New | Modified |
|---|---|
| `src/lib/modifiers/api.ts` (typed Tauri wrappers, 12 functions) | `src/tools.ts` (one entry) |
| `src/store/modifiers.svelte.ts` (load + cache modifier rows; UI prefs: `activeFilterTags`, `showHidden`, `showOrphans`) | `src/types.ts` (mirror `CharacterModifier`, `ModifierBinding`, `ModifierEffect`, `ModifierKind`, `StatusTemplate`, plus argument shapes) |
| `src/store/statusTemplates.svelte.ts` (CRUD-backed template list) | `src-tauri/src/lib.rs` (register 12 commands) |
| `src/tools/GmScreen.svelte` (top-level tool) | `src-tauri/src/shared/types.rs` (or new `shared/modifier.rs` alongside) — type defs + serde derives |
| `src/components/gm-screen/CharacterRow.svelte` |  |
| `src/components/gm-screen/ModifierCard.svelte` |  |
| `src/components/gm-screen/ModifierEffectEditor.svelte` (cog-wheel popover; reuses pattern from `AdvantageForm`'s field editor) |  |
| `src/components/gm-screen/StatusPaletteDock.svelte` |  |
| `src/components/gm-screen/StatusTemplateEditor.svelte` |  |
| `src/components/gm-screen/TagFilterBar.svelte` |  |

### Tool registration

```ts
// src/tools.ts (one new entry)
{ id: 'gm-screen', label: 'GM Screen', icon: '🛡', component: () => import('./tools/GmScreen.svelte') }
```

### Cross-tool events

The GM Screen does NOT publish or subscribe to `toolEvents` in stage 1 (no cross-tool coordination yet). Future: a `gmScreen:roll-with-modifiers` event could fan into a roll log when 3B lands.

## §7 UI — layout and card-row visual model

### §7.1 Page layout

```
┌────────────────────────────────────────────────────────────────────────────────┐
│ 🛡 GM Screen                                                                   │
├────────────────────────────────────────────────────────────────────────────────┤
│ Filter: [Social] [Combat] [Physical] [+]    [Show hidden ◯] [Show orphans ◯] │
├──────────────────────────────────┬─────────────────────────────────────────────┤
│ Characters                       │ Status palette                              │
│                                  │                                             │
│ ┌─ Charlotte (Foundry) ───────┐  │  ┌─ Templates ──────────────────────────┐ │
│ │ Hunger 2 · WP 7/8 · Dmg ─   │  │  │ [+ New template]                     │ │
│ │                              │  │  │                                       │ │
│ │ [card-row: Beautiful*│Iron   │  │  │ [Slippery]  [Blind]  [Loud env]      │ │
│ │  Will*│Burning Wrath ON +2   │  │  │ [Heavy winds] [Pinned] [Restrained]  │ │
│ │  Str│ + Add modifier]        │  │  └───────────────────────────────────────┘ │
│ └──────────────────────────────┘  │                                             │
│                                  │   Click a template card while a character   │
│ ┌─ Marcus (Foundry) ──────────┐  │   row is focused (or drag onto a row) to    │
│ │ Hunger 4 · WP 5/6 · Dmg 2s  │  │   apply an independent copy. Click-to-apply │
│ │ [card-row: Linguist│Bad      │  │   ships first; drag-and-drop is a polish    │
│ │  Sight│ + Add modifier]      │  │   pass.                                     │
│ └──────────────────────────────┘  │                                             │
│                                  │                                             │
│ ┌─ Orphans (1) ──────────────┐   │                                             │
│ │ [card-row: Old Burning      │   │                                             │
│ │  Wrath × delete-from-orphan]│   │                                             │
│ └──────────────────────────────┘  │                                             │
└──────────────────────────────────┴─────────────────────────────────────────────┘

Asterisk * = derived-from-advantage card with no effect attached yet.
"ON" annotation = active-state indicator (in real UI: accent border + saturated fill).
The cog-wheel reveals on hover/focus per card.
```

The character header line shows hunger / willpower-track / damage-track in compact read-only form so the GM doesn't have to bounce to Campaign for context. Editing those values stays in Campaign (Phase 2 set_field UI).

### §7.2 Card-row visual model — straight stacked carousel

The modifier cards in each character's row use the **stacked overlapping carousel** treatment, adapting the radial-fan technique from the user-supplied reference (`offset-path: circle(...)` with neighbor-shift on hover) into a **horizontal straight layout**. The original technique's load-bearing pieces:

- Cards are absolutely positioned by a calculated index (`sibling-index()`).
- Cards overlap each other (z-index layered with center-on-top).
- Hover a card → it lifts forward, neighbors slide laterally to give it room.
- The neighbor-shift uses `:has(+*:hover)` chains to cascade in both directions.
- Smooth bouncy transition driven by a custom `linear()` easing curve.

Translated to a horizontal row:

```css
/* GM Screen card row — straight stacked carousel */
.modifier-row {
  --card-trans-duration: 600ms;
  --card-trans-easing: linear(
    0, 0.01 0.8%, 0.038 1.6%, 0.154 3.4%, 0.781 9.7%, 1.01 12.5%,
    1.089 13.8%, 1.153 15.2%, 1.195 16.6%, 1.219 18%, 1.224 19.7%,
    1.208 21.6%, 1.172 23.6%, 1.057 28.6%, 1.007 31.2%, 0.969 34.1%,
    0.951 37.1%, 0.953 40.9%, 0.998 50.4%, 1.011 56%, 0.998 74.7%, 1
  );
  --card-width: 9rem;
  --card-overlap: 0.55;            /* fraction of card width that overlaps the next */
  --card-shift-delta: 0.5rem;
  --cards: sibling-count();        /* CSS sibling-count() for layout math */

  position: relative;
  height: 8rem;
  perspective: 800px;
}

.modifier-card {
  --card-i: sibling-index();
  --base-x: calc((var(--card-i) - 1) * var(--card-width) * (1 - var(--card-overlap)));
  --shift-x: 0rem;

  position: absolute;
  left: 0;
  top: 0;
  width: var(--card-width);
  height: 100%;
  box-sizing: border-box;          /* per ARCH §6 — no global reset */
  background: var(--bg-card);
  border: 1px solid var(--border-card);
  border-radius: 0.625rem;

  transform: translateX(calc(var(--base-x) + var(--shift-x)));
  transition: transform var(--card-trans-duration) var(--card-trans-easing),
              box-shadow var(--card-trans-duration) var(--card-trans-easing),
              border-color 200ms ease;
}

/* z-stack: cards near the centre of the row sit highest (matches reference's centre-on-top pattern).
   distance-from-centre = abs(card-i - (cards + 1) / 2); higher cards-i-distance → lower z. */
.modifier-card {
  --centre: calc((var(--cards) + 1) / 2);
  --distance: max(calc(var(--card-i) - var(--centre)), calc(var(--centre) - var(--card-i)));
  z-index: calc(100 - var(--distance));
}

/* hover lift */
.modifier-card:hover {
  z-index: 100;
  transform: translateX(calc(var(--base-x) + var(--shift-x))) translateY(-0.75rem) translateZ(20px);
  box-shadow: 0 1.25rem 2rem -0.5rem var(--accent);
}

/* neighbor-shift cascade — cards AFTER hovered slide right */
.modifier-card:hover + .modifier-card                 { --shift-x: calc(var(--card-shift-delta) * 3); }
.modifier-card:hover + .modifier-card + .modifier-card { --shift-x: calc(var(--card-shift-delta) * 2); }
.modifier-card:hover + .modifier-card + .modifier-card + .modifier-card {
  --shift-x: calc(var(--card-shift-delta) * 1);
}

/* neighbor-shift cascade — cards BEFORE hovered slide left, via :has() */
.modifier-card:has(+ .modifier-card:hover)                                        { --shift-x: calc(var(--card-shift-delta) * -3); }
.modifier-card:has(+ .modifier-card + .modifier-card:hover)                       { --shift-x: calc(var(--card-shift-delta) * -2); }
.modifier-card:has(+ .modifier-card + .modifier-card + .modifier-card:hover)      { --shift-x: calc(var(--card-shift-delta) * -1); }

/* active-state visual axis (independent of focus) */
.modifier-card[data-active="true"] {
  border-color: var(--accent-bright);
  background: var(--bg-active);
}

/* hidden state (only rendered when showHidden === true) */
.modifier-card[data-hidden="true"] {
  opacity: 0.45;
  filter: saturate(0.6);
}

/* reduced-motion fallback */
@media (prefers-reduced-motion: reduce) {
  .modifier-row,
  .modifier-card {
    --card-overlap: 0;
    --card-shift-delta: 0;
    transition: none;
  }
}
```

Key adaptations from the reference:

1. **Path → straight transform.** Reference uses `offset-path: circle(... at 50% 100%)` to lay cards on a circular arc. Stage 1 uses `transform: translateX(...)` directly — same overlap-and-stack visual model, different geometry.
2. **Z-index pattern preserved.** Center cards forward; outer cards behind. Implementation can either hardcode (as in reference's `:nth-child(2,6)` etc.) or derive from `--card-i` against `--cards / 2`.
3. **Bouncy easing preserved.** The `linear(...)` easing curve from the reference is copied verbatim — it's the spring-like character that makes the motion feel alive.
4. **Neighbor-shift cascade preserved.** Same `:has(+*:hover)` chains; same three-step shift-delta pattern (3 / 2 / 1). The deltas are smaller in pixels here (0.5rem) than in the radial version because cards don't have to travel along an arc.
5. **Hover lift adds Z.** The radial version uses `offset-anchor` to push the hovered card forward along its path. The straight version uses `translateY` + `translateZ` for the lift.
6. **Independent state axis.** `data-active`, `data-hidden`, and hover are three independent visual axes. Active and hidden are state attributes (driven by the modifier record); hover is transient. They compose cleanly.
7. **Reduced-motion fallback.** Removes overlap and disables the spring transition; the row degrades to a flat horizontal list.

**Browser-support note.** `sibling-index()` and `sibling-count()` are CSS Values L4/L5 tree-counting functions, supported by recent Chromium / WebKit / Gecko (2025+). Tauri 2's bundled WebView (system WebView2 on Windows, system WebKit on macOS, WebKitGTK on Linux) gets these on any reasonably-current OS. The implementation should still ship a defensive fallback that hardcodes the layout for up to 12 cards via `&:nth-child(N)` rules — matching the reference CSS's `@supports not (order:sibling-index())` block. Twelve is well above the typical modifier-row size (1–7) and gives a generous safety margin.

### §7.3 Card content layout

```
┌─────────────────────┐
│ Beautiful        ⚙ │   ← name + cog (cog reveals on hover/focus)
│ ────                │
│ Social +1 dice      │   ← effect summary (or "(no effect)" if empty)
│ Difficulty −1       │   ←   second effect, if present
│                     │
│ #Social  #Looks     │   ← tag chips
│                     │
│              [ ON ] │   ← toggle pill bottom-right
└─────────────────────┘
```

Cog opens an inline popover (anchored to the cog itself, not a modal) containing the effect editor: list of `ModifierEffect` rows with kind/scope/delta/note fields, plus `+ Add effect` and `+ Add tag` chip rows. Save commits a `ModifierPatch` via `update_character_modifier` (or, if the card is still derived, calls `materialize_advantage_modifier` first).

### §7.4 Status palette dock

Templates render as compact cards (smaller than character-row modifier cards — palette uses ~6rem width, no carousel treatment, just a wrap-grid). Each shows name, tag chips, and a small effect summary. Click a template:

- **If a character row is focused** (last clicked): apply to that character.
- **Otherwise**: apply to the topmost character (or surface a "pick character" hint). Drag-to-character is a polish-pass enhancement.

`+ New template` opens `StatusTemplateEditor.svelte` in a side-pane modal — same shape as the modifier effect editor, but persisted to `status_templates` instead of `character_modifiers`.

### §7.5 Tag filter bar

Identical UX pattern to `AdvantagesManager`'s filter chips:

- Chip set derived from `[...new Set(allCards.flatMap(c => c.tags))].sort()`.
- Clicking a chip toggles it active. Multi-select with OR semantics (a card matches if it has any active tag).
- Empty active set = no filtering.
- **Active modifier cards always render past the filter** (per the user's pin rule).
- Hidden cards are excluded entirely unless `showHidden === true`.
- Orphan cards excluded entirely unless `showOrphans === true`.

### §7.6 Color tokens

All colors use the `:root` tokens from `src/routes/+layout.svelte` per ARCH §6. The card visual model uses:

- `--bg-card` (resting card background)
- `--bg-active` (active-state card background — accent-tinted)
- `--border-card` / `--border-active` (resting / focused borders)
- `--accent` / `--accent-bright` (hover glow, active border, toggle pill)
- `--text-primary` / `--text-secondary` / `--text-muted` (name / effect summary / tag chips)

No hex literals in card-row CSS. Hover-glow shadow may use a translucent overlay derived from `--accent` if the existing tokens don't quite fit a glow shape — falling back to the ARCH §6 carve-out for "transient states with no semantic token."

## §8 Data flow

### §8.1 Read flow — rendering one character row

1. Take live `BridgeCharacter` from `bridgeStore` + saved match (if any) from `savedCharactersStore` + materialized `CharacterModifier[]` for `(source, source_id)` from `modifiersStore`.
2. Walk `canonical.raw.items` (Foundry source). For each item with `type === 'feature'` and a `system.featuretype` of `merit` / `flaw` / `background` / `boon`:
   - Look for a materialized modifier with `binding.kind === 'advantage' && binding.item_id === item._id`.
   - **If yes** → render the materialized record (effects, active state, hidden state from DB).
   - **If no** → render a *derived virtual card* with `name` from the item, `description` from the item, empty effects, `is_active: false`, `is_hidden: false`. This card has no DB row yet.
3. Append free-floating modifiers (those with `binding.kind === 'free'`).
4. Apply the active filter chips. Active modifier cards remain visible past the filter (pin rule). Hidden cards excluded unless `showHidden === true`.
5. Sort the final card list by `is_active DESC, created_at ASC` (active cards bubble to the left of the row).

Stage 1 reads only Foundry advantages. Roll20 advantage cards land in Phase 2.5.

### §8.2 Materialize-on-engagement

A derived card has no DB row. The first time the GM does any of:

- **Cog-edit attach effect** — frontend calls `materialize_advantage_modifier(source, source_id, item_id, name, description)` (idempotent), then `update_character_modifier(id, { effects: [...] })`.
- **Toggle active** — same materialize call, then `set_modifier_active(id, true)`.
- **Hide** — same materialize call, then `set_modifier_hidden(id, true)`.

Subsequent engagements skip the materialize step (the row exists). The `materialize_advantage_modifier` command is idempotent by design — calling it twice with the same `(source, source_id, item_id)` returns the existing row without modification.

Materialized rows survive the merit being deleted from the character. The render flow detects this case (no matching `_id` in `raw.items`) and shows a "stale" badge on the card. The GM can dismiss it (delete) or wait for the merit to come back (e.g. it was temporarily un-applied).

### §8.3 Hide / show

`set_modifier_hidden(id, true)` flips `is_hidden = 1` on the row. With `showHidden === false` (default), hidden cards are filtered out completely from the row. With `showHidden === true`, hidden cards render with a muted style and a "show again" affordance (clicking it calls `set_modifier_hidden(id, false)`).

`showHidden` is a per-session UI preference held in the Svelte store, not persisted to DB.

### §8.4 Status template apply

GM clicks a palette card while a character row is focused. Frontend:

1. Read the template (`name`, `description`, `effects`, `tags`).
2. Build a `NewCharacterModifier`:
   ```ts
   {
     source: focused.source,
     source_id: focused.source_id,
     name: template.name,
     description: template.description,
     effects: structuredClone(template.effects),
     binding: { kind: "free" },
     tags: [...template.tags],
     origin_template_id: template.id,
   }
   ```
3. Call `add_character_modifier(input)`.
4. New card appears at the end of the character's row (sorted to its place by §8.1's sort order).

Independent copy: subsequent template edits do not propagate. The `origin_template_id` is a provenance breadcrumb only.

### §8.5 Orphans

A modifier row whose `(source, source_id)` matches no live character AND no saved character is an orphan. Orphans render in a separate section at the bottom of the page, hidden by default behind `showOrphans` toggle. GM can delete them outright. If the live character reappears (e.g. Foundry world reopened), the key match auto-rejoins the modifier to the character.

### §8.6 Bridge invalidation

The `modifiersStore` does not auto-refetch on every `bridge://characters-updated` event — modifier records don't change with bridge state. Refetch only on:

- Tool mount (`onMount` → `list_all_character_modifiers`).
- Successful CRUD response (the response carries the updated row; merge into store).

This avoids the per-event refetch storm during a Foundry actor-update flurry.

## §9 Error handling

Per ARCH §7. All Tauri commands return `Result<T, String>` with module-stable prefixes:

| Failure | Surfaces as |
|---|---|
| `add_character_modifier` empty name | `Err("db/modifier.add: empty name")` |
| `update_character_modifier` missing id | `Err("db/modifier.update: not found")` |
| `delete_character_modifier` missing id | `Err("db/modifier.delete: not found")` |
| `set_modifier_active` missing id | `Err("db/modifier.set_active: not found")` |
| `set_modifier_hidden` missing id | `Err("db/modifier.set_hidden: not found")` |
| `materialize_advantage_modifier` empty name | `Err("db/modifier.materialize: empty name")` (idempotent path returns Ok) |
| `add_status_template` empty name | `Err("db/status_template.add: empty name")` |
| `update_status_template` missing id | `Err("db/status_template.update: not found")` |
| `delete_status_template` missing id | `Err("db/status_template.delete: not found")` |
| `serde` deserialize failure at IPC | Tauri auto-rejects with serde error |

Frontend API wrapper catches rejected promises at the call site and surfaces user-visible errors via the existing toast pattern. Raw errors logged to console.

## §10 Plan packaging

This spec naturally decomposes into **two plans** (per ARCH §11 + lean-execution override):

### Plan A — Modifier core

- Migration `0005_modifiers.sql` ships **both** the `character_modifiers` and `status_templates` tables (one migration is cheaper than two; the empty templates table is inert until Plan B wires its commands).
- Rust types in `shared/types.rs` (or new `shared/modifier.rs`).
- `db/modifier.rs` with the 8 modifier commands + inline tests.
- TS mirror in `src/types.ts`.
- `src/lib/modifiers/api.ts` typed wrapper.
- `src/store/modifiers.svelte.ts`.
- `src/tools/GmScreen.svelte`, `CharacterRow.svelte`, `ModifierCard.svelte`, `ModifierEffectEditor.svelte`, `TagFilterBar.svelte`.
- Tool registry entry.
- `verify.sh` green; manual: free-floating modifier add → toggle on/off → cog-edit pool effect → see card summary; advantage card materializes on first engagement; hide/show works.

### Plan B — Status palette

- `db/status_template.rs` with the 4 template commands + inline tests.
- `src/lib/modifiers/api.ts` extended with template wrappers.
- `src/store/statusTemplates.svelte.ts`.
- `StatusPaletteDock.svelte`, `StatusTemplateEditor.svelte`.
- Wire into `GmScreen.svelte` layout.
- `origin_template_id` provenance display on instance cards.
- `verify.sh` green; manual: create template → drop on character → instance is independently editable → palette edit doesn't propagate.

Both plans run in single-session execution (lean-plan-execution override). Plan A first; Plan B parallelizable in a worktree once Plan A's IPC is committed.

**Implementation hint — visual-heavy components.** The plan-writer should mark the following components as candidates for `frontend-design:frontend-design` dispatch during implementation: `ModifierCard.svelte` (carousel + state axes), `ModifierEffectEditor.svelte` (cog popover with multi-effect editor), `StatusPaletteDock.svelte` (palette grid + apply interaction), `StatusTemplateEditor.svelte` (template authoring side-pane). Pass the §7 spec excerpt + the surrounding component conventions (`AdvantagesManager.svelte`, `DyscrasiaManager.svelte`, `+layout.svelte` token set) as context. The non-visual components (`GmScreen.svelte`, `CharacterRow.svelte`, `TagFilterBar.svelte`) follow existing patterns directly and don't need the dispatch.

### Anti-scope (per ARCH §11)

| Plan | MUST NOT touch |
|---|---|
| A | `db/status_template.rs`, `StatusPaletteDock.svelte`, `StatusTemplateEditor.svelte`, `Campaign.svelte` (no edits) |
| B | `db/modifier.rs` (frozen by Plan A), `migrations/` (no new migration in Plan B), `tools/Campaign.svelte` |

### Invariants cited

- Plan A: §3 (storage strategy — SQLite, `tokio::fs`), §4 (Tauri IPC + frontend API wrappers), §5 (only `db/*` talks to SQLite), §6 (`PRAGMA foreign_keys = ON`, `:root` color tokens, `box-sizing: border-box`, no multi-column).
- Plan B: same as Plan A minus migrations.

### Verification gate

Per CLAUDE.md hard rule: every plan task ending in a commit runs `./scripts/verify.sh` first.

## §11 Testing

- Rust unit tests inline in `db/modifier.rs` and `db/status_template.rs` (ARCH §10):
  - Round-trip insert/list/delete preserves `effects_json`, `binding_json`, `tags_json`.
  - `materialize_advantage_modifier` idempotency: two calls with same `(source, source_id, item_id)` return the same row.
  - `set_modifier_active` and `set_modifier_hidden` flip the right column.
  - `update_character_modifier` rejects missing id; preserves untouched fields when patch is partial.
  - Status template CRUD round-trip.
- No frontend tests (ARCH §10 stands).
- `./scripts/verify.sh` green before any commit.
- Manual verification per Plan A / B as listed in §10.

## 11A. Phase 2.5 — Explicit Foundry write-back (push to merit bonuses)

A per-card "↑ Push" button on advantage-bound cards (Foundry sources only)
that mirrors the card's effects to the bound merit's `system.bonuses[]`.

**Visibility:** the button is rendered only when ALL of the following hold:

- character source is `foundry`
- card is materialized (has DB row, not a virtual advantage card)
- binding is `advantage` (not `free`)
- card is not stale (the source merit still exists on the actor)
- card has at least one `pool` effect

**Translation rule.** Each `ModifierEffect` translates as follows:

| `kind`       | Behavior                                                                          |
|--------------|-----------------------------------------------------------------------------------|
| `pool`       | Emit one bonus: `value = delta`, `paths = e.paths` (`[""]` if empty per Foundry sample). All conditional fields default: `activeWhen = { check: "always", path: "", value: "" }`, `displayWhenInactive = true`, `unless = ""`. |
| `difficulty` | Skipped — Foundry's `system.bonuses[]` has no difficulty mechanism. Surfaced in `PushReport.skipped` so the GM understands the asymmetry. |
| `note`       | Skipped — descriptive only.                                                       |

**Idempotency.** Each pushed bonus is tagged `source: "GM Screen #<modifier_id>: <name>"`.
Re-pushing first removes any bonus whose `source` starts with `"GM Screen #<id>"`
(matching exactly, so id 5 doesn't match id 50), then appends the freshly translated
ones. Player-added bonuses and bonuses pushed for other modifiers on the same item
are preserved.

**TOCTOU caveat.** The push reads `system.bonuses[]` from the cached actor in
`BridgeState`, then writes the merged array. If the player edits the same item's
bonuses in Foundry between the read and the write, edits to bonuses tagged as ours
can be lost (player-added bonuses are not at risk — they're filtered through `is_ours`
and pass through unchanged). Acceptable for v1; documented in
`src-tauri/src/tools/gm_screen.rs::do_push_to_foundry`.

**No Roll20 equivalent.** Roll20 sheets don't expose a `system.bonuses[]`
analogue; the push button is hidden for Roll20 characters by the visibility
predicate above.

### Reset semantics

A per-card "↺ Reset" button on materialized advantage cards (Foundry sources only)
deletes the local DB row and reverts the card to its virtual baseline (just the
item name + whatever bonuses Foundry currently has on the merit).

**Local-only delete.** Reset does NOT touch Foundry's `system.bonuses[]`. The
two data stores are separated by design: local effects are *intent*, Foundry
bonuses are *durable state*. Reset drops the intent; durable state is left to
the GM (or the player) to manage on the Foundry side.

**Orphan implication.** If the card was previously pushed, the
`GM Screen #<old_id>: <name>` tagged bonuses persist on the merit until
manually removed in Foundry. They remain visibly labeled, so the GM can
spot them in the merit's bonus list. Over many reset→re-push cycles
orphan bonuses with stale ids can accumulate; a future "clean up GM Screen
orphans" action (out of scope for this phase) would batch-remove them.

**Confirmation.** Reset is destructive (deletes all local effects, paths,
tags, isActive, isHidden in one step). The button triggers a `confirm()`
dialog before delete.

**Free modifiers.** Not eligible — there's no live baseline to revert to.
Reset is hidden on free-binding cards.

## §12 Future seams

| Future feature | How this spec accommodates it |
|---|---|
| **Rooms / bundles** (deferred — own future spec) | Add `rooms` + `room_members` tables; extend `ModifierBinding` with `Room { room_id }` variant; cards bound to rooms render on every member character. Strictly additive — no refactor of stage 1. |
| **V5 auto-fold into dice pools** (Phase 3B / future) | `shared/v5/SkillCheckInput` grows `pub modifiers: Vec<ModifierEffect>`; `pool` and `difficulty` builders fold the deltas. The GM Screen exposes a "Roll …" button per character feeding *active, non-hidden* modifiers. The `ModifierEffect` shape was designed to slot in directly. |
| **Foundry effect mirror** (Phase 4 / future) | Add `actor.apply_active_effect` to the FHL roadmap; new `ModifierBinding::FoundryEffect { effect_id }` variant carries the round-trip key. Activate-on-toggle could send to Foundry. |
| **Roll20 advantage auto-spawn** (Phase 2.5) | Reuse the same `binding.item_id` shape, sourced from `repeating_meritsflaws_*` row-ids. Adds a Roll20 read-path; no schema change. |
| **`actor.apply_dyscrasia` integration** | Already shipped; dyscrasia surfaces as a feature item on the actor and falls into the §8.1 advantage-derived card path automatically. |

## §13 Open questions

These are flagged for plan-time resolution; none block writing the plans.

1. **Roll20 advantage auto-spawn in stage 1?** Spec defaults to no (Phase 2.5 follow-up). If user wants stage 1 to include it, add a Roll20-read sub-task to Plan A.
2. **Effect editor `delta` widget — stepper or dot-strip?** Spec defaults to a small `[− 0 +]` numeric stepper bounded -10..+10 with freeform `scope` text input below. Dot-strip is a swap if symmetry with merit dot-strip is preferred.
3. **Drag-and-drop for status template apply?** Spec defaults to click-to-apply (with focused-character convention) for v1; drag is a polish pass.
4. **Templates seed?** Should the database ship a few canned templates (`Slippery`, `Blind`, `Loud environment`, `Heavy winds`, `Restrained`, `Pinned`, `Drugged`, `Wounded`)? Spec defaults to no — empty palette by default; user populates via `+ New template`. If yes, add a destructive-reseed pass following the dyscrasia/advantage pattern (ADR 0002), with `is_custom = 0` on canned templates.

## §14 Phase placement

Stage 1 sits in **Phase 3 — During-play tooling**. Sub-features:

- **3A — GM Screen** (this spec)
- **3B — Roll mirroring + roll log** (existing roadmap §5 sketch)

3A and 3B are independent; either can ship first. Both go on the GitHub Project board as feature-level parents (per the project's roadmap-tracking rule); subtasks linked from each parent body via `- [ ] #N` task lists.

The future "full GM screen" tool (rooms/bundles, scene tracking, initiative) is a separate phase or Phase 3+ continuation — not in scope here.
