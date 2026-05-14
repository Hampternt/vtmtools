# Character Card Redesign — Plan B — Modifier Integration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Prerequisite:** Plan A (`2026-05-10-character-card-redesign-plan-a-card-body.md`) MUST be merged. This plan extends `CharacterCard.svelte` and `ModifierEffectEditor.svelte` and adds a new pure projection function — none of those targets exist before Plan A.

**Goal:** Wire the existing GM-Screen modifier system into the new `CharacterCard.svelte`. Toggling a merit chip on View 4 flips its `is_active` in the modifier store; the card subscribes to that store and projects active stat-kind modifiers as visible attribute/skill deltas on View 2 (`CHA 4 → 5` with `+1` badge). Adds a fourth `Stat` variant to `ModifierKind` (additive — no schema migration).

**Architecture:** One new Rust enum variant. One new TS pure function (`computeActiveDeltas`). One small extension to `ModifierEffectEditor.svelte` (just adds `'stat'` to its `KINDS` array — the existing `paths[]` chip-input is reused as-is). Behavior changes to `CharacterCard.svelte` (subscription, click handlers, delta annotations, banner). Zero data-flow changes to the existing modifier store / IPC — Plan B uses what's already there.

**Tech Stack:** Rust (serde tagged enum extension + cargo test), Svelte 5 (runes), TypeScript pure function (no test framework — verified via manual UI checklist per ARCH §10).

---

## Required Reading

1. **CLAUDE.md** — verify.sh gate, snake_case vs camelCase asymmetry (`BridgeCharacter` is snake_case wire, but `CharacterModifier` is camelCase via Rust serde rename).
2. **ARCHITECTURE.md** §3, §4, §6, §7, §10 — storage / IPC / `:root` tokens / error handling / no-frontend-test-framework.
3. **`docs/superpowers/specs/2026-05-10-character-card-redesign-design.md`** §6 (full modifier integration spec — render-time vs roll-time, `computeActiveDeltas`, chip click flow, banner). §11 (testing — Rust serde round-trip + manual UI checklist).
4. **`docs/superpowers/specs/2026-05-03-gm-screen-design.md`** §3, §4, §5 — modifier table, binding shapes, existing IPC commands. (Plan B does NOT add new commands — it consumes what's there.)
5. **`src/lib/saved-characters/diff.ts`** — read it. The new `computeActiveDeltas()` mirrors its path-projection vocabulary; understanding `DIFFABLE_PATHS` is required to write the path-resolver helper.

The implementer is **not** to introduce a frontend test framework (ARCH §10). Plan B's only new test is a Rust serde test (`cargo test`-runnable).

The implementer is **not** to commit without first running `./scripts/verify.sh` and confirming green output.

---

## File Structure

| Action | Path | Responsibility |
|---|---|---|
| Modify | `src-tauri/src/shared/modifier.rs` | Add `Stat` variant to `ModifierKind` enum + inline `#[cfg(test)] mod tests` covering serde round-trip of all four kinds. |
| Modify | `src/types.ts` | Add `'stat'` to the `ModifierKind` TS union (mirrors Rust). |
| Modify | `src/lib/components/gm-screen/ModifierEffectEditor.svelte` | Add `'stat'` entry to `KINDS` array; helper text under dropdown explaining §6.2 asymmetry; tweak `setKind` so 'stat' kind sets `note: null` (matches pool/difficulty). |
| Create | `src/lib/character/active-deltas.ts` | Pure function `computeActiveDeltas()` returning `Map<path, DeltaEntry>` from a character + modifier list. Inline path-resolver helper (factor only on second use per spec §14.1 default). |
| Modify | `src/lib/components/CharacterCard.svelte` | Subscribe to `modifiers.forCharacter()`; compute `activeDeltas` and `activeChipIds`; render banner; annotate View 2 entries; wire chip `onclick` (materialize + toggle) and `oncontextmenu` (popover); flip `data-active` on chips. **No structural layout change** (anti-scope). |

Plan B adds ~150 lines net across these files.

---

## Task 1 — Add `Stat` variant to Rust `ModifierKind` + serde test

**Files:**
- Modify: `src-tauri/src/shared/modifier.rs`

- [ ] **Step 1:** Open `src-tauri/src/shared/modifier.rs`. Find the `ModifierKind` enum declaration (around line 30, after `ModifierEffect`'s definition). Add the new variant:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModifierKind {
    Pool,
    Difficulty,
    Note,
    Stat,  // NEW — render-time visual stat delta on the character card.
}
```

  No other change to the file is required for the variant itself — `effects_json` is a TEXT column with no CHECK constraint, and serde tagged-enum derives handle the round-trip automatically.

- [ ] **Step 2:** At the bottom of `src-tauri/src/shared/modifier.rs` (after the last `pub` item), add a `#[cfg(test)] mod tests` block. If a test module already exists, **add the new tests below the existing ones** rather than creating a second module. Test code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_effect_stat_round_trips_through_json() {
        let original = ModifierEffect {
            kind: ModifierKind::Stat,
            scope: Some("vs social rolls".to_string()),
            delta: Some(1),
            note: None,
            paths: vec!["attributes.charisma".to_string(), "attributes.manipulation".to_string()],
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let round_trip: ModifierEffect = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip.kind, ModifierKind::Stat);
        assert_eq!(round_trip.scope.as_deref(), Some("vs social rolls"));
        assert_eq!(round_trip.delta, Some(1));
        assert_eq!(round_trip.note, None);
        assert_eq!(
            round_trip.paths,
            vec!["attributes.charisma".to_string(), "attributes.manipulation".to_string()],
        );
    }

    #[test]
    fn modifier_kind_serializes_as_snake_case() {
        // Regression — guards against accidentally renaming the variant.
        let stat_json = serde_json::to_string(&ModifierKind::Stat).expect("ser stat");
        assert_eq!(stat_json, r#""stat""#);
        let pool_json = serde_json::to_string(&ModifierKind::Pool).expect("ser pool");
        assert_eq!(pool_json, r#""pool""#);
    }

    #[test]
    fn effects_json_blob_with_all_four_kinds_round_trips() {
        // Mirrors the actual `effects_json` TEXT column shape.
        let blob = vec![
            ModifierEffect { kind: ModifierKind::Pool,       scope: Some("Social".into()),  delta: Some(1),  note: None,                paths: vec![] },
            ModifierEffect { kind: ModifierKind::Difficulty, scope: None,                   delta: Some(-1), note: None,                paths: vec![] },
            ModifierEffect { kind: ModifierKind::Note,       scope: None,                   delta: None,     note: Some("blinded".into()), paths: vec![] },
            ModifierEffect { kind: ModifierKind::Stat,       scope: Some("Beautiful".into()), delta: Some(1),  note: None,                paths: vec!["attributes.charisma".into()] },
        ];
        let json = serde_json::to_string(&blob).expect("serialize");
        let round_trip: Vec<ModifierEffect> = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_trip.len(), 4);
        assert_eq!(round_trip[3].kind, ModifierKind::Stat);
        assert_eq!(round_trip[3].paths, vec!["attributes.charisma".to_string()]);
    }
}
```

- [ ] **Step 3:** Run `cargo test --manifest-path src-tauri/Cargo.toml -- modifier::tests`. Expected: three new tests pass; pre-existing tests in the file (if any) still pass.

- [ ] **Step 4:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 5:** Commit.

```bash
git add src-tauri/src/shared/modifier.rs
git commit -m "$(cat <<'EOF'
feat(modifier): add Stat variant to ModifierKind for render-time card deltas

Stat is the fourth ModifierKind, used for visible attribute/skill deltas on
the character card (CHA 4 → 5). Render-time only — never consumed by the V5
dice helper at roll dispatch (see spec §6.2). Reuses the existing
ModifierEffect.paths field for target paths; no struct shape change.

No schema migration required: effects_json is a TEXT column with no CHECK
constraint, so the new variant deserializes through serde's tagged-enum
derives. Existing rows continue to deserialize.

Tests: serde round-trip for the new variant, snake_case wire-format
regression, mixed-kind blob round-trip.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6.1
EOF
)"
```

---

## Task 2 — Mirror `'stat'` to TS + extend `ModifierEffectEditor`

**Files:**
- Modify: `src/types.ts`
- Modify: `src/lib/components/gm-screen/ModifierEffectEditor.svelte`

- [ ] **Step 1:** Open `src/types.ts`. Find the `ModifierKind` type declaration (line 201). Replace:

```ts
export type ModifierKind = 'pool' | 'difficulty' | 'note';
```

with:

```ts
export type ModifierKind = 'pool' | 'difficulty' | 'note' | 'stat';
```

- [ ] **Step 2:** Open `src/lib/components/gm-screen/ModifierEffectEditor.svelte`. Find the `KINDS` array (around line 19). Replace:

```ts
  const KINDS: { value: ModifierKind; label: string }[] = [
    { value: 'pool',       label: 'Pool' },
    { value: 'difficulty', label: 'Difficulty' },
    { value: 'note',       label: 'Note' },
  ];
```

with:

```ts
  const KINDS: { value: ModifierKind; label: string }[] = [
    { value: 'pool',       label: 'Pool' },
    { value: 'difficulty', label: 'Difficulty' },
    { value: 'note',       label: 'Note' },
    { value: 'stat',       label: 'Stat' },
  ];
```

- [ ] **Step 3:** Find the `setKind` function (around line 39). Currently it special-cases `'note'` (clearing `delta`) and clears `note` for everything else. The 'stat' kind needs the same treatment as pool/difficulty (delta is meaningful, note is unused), so the existing `else` branch already handles it. **No change needed** here — verify the function reads:

```ts
  function setKind(i: number, kind: ModifierKind) {
    if (kind === 'note') {
      effects[i] = { ...effects[i], kind, delta: null };
    } else {
      effects[i] = { ...effects[i], kind, note: null };
    }
  }
```

- [ ] **Step 4:** Add helper text below the kind dropdown when the selected kind is `'stat'`. Find the `<select value={effect.kind} ...>` element (around line 96). Wrap the existing `<select>` and add a sibling helper-text node beneath it inside the same `effect-row`. The block becomes:

```svelte
        <div class="kind-cluster">
          <select value={effect.kind} onchange={(e) => setKind(i, (e.currentTarget as HTMLSelectElement).value as ModifierKind)}>
            {#each KINDS as k}<option value={k.value}>{k.label}</option>{/each}
          </select>
          {#if effect.kind === 'stat'}
            <span class="kind-help" title="Stat effects show on the card as attribute deltas. They don't auto-affect rolls — use a Pool effect for that.">render-time only ⓘ</span>
          {/if}
        </div>
```

  Add to the existing `<style>` block:

```css
  .kind-cluster { display: flex; flex-direction: column; gap: 0.15rem; }
  .kind-help {
    font-size: 0.65rem;
    color: var(--text-secondary);
    font-style: italic;
    cursor: help;
    letter-spacing: 0.04em;
  }
```

  The mouseover tooltip via `title=` carries the full asymmetry explanation; the inline italic chip is the visible affordance.

- [ ] **Step 5:** Run `./scripts/verify.sh`. Expected: green. (Type-check now sees `'stat'` flowing through the editor; cargo test still passes from Task 1.)

- [ ] **Step 6:** Commit.

```bash
git add src/types.ts src/lib/components/gm-screen/ModifierEffectEditor.svelte
git commit -m "$(cat <<'EOF'
feat(modifier-editor): add Stat kind option with render-time-only helper text

Adds 'stat' to the ModifierKind TS union (mirrors Rust enum from previous
commit). ModifierEffectEditor's KINDS array gains the Stat option; an inline
italic chip with a help tooltip surfaces the §6.2 render-time vs roll-time
asymmetry — Stat effects show as card deltas but don't auto-fold into rolls.

The existing paths[] chip-input is reused as-is — no new path-picker UI.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6.1, §6.2
EOF
)"
```

---

## Task 3 — Create `computeActiveDeltas` projection

**Files:**
- Create: `src/lib/character/active-deltas.ts`

- [ ] **Step 1:** Create the file `src/lib/character/active-deltas.ts` with this content:

```ts
// computeActiveDeltas — projects active stat-kind modifiers onto a character.
//
// Pure function. No IPC, no side effects. Mirrors the path-projection
// vocabulary used by src/lib/saved-characters/diff.ts (canonical attribute /
// skill paths like 'attributes.charisma' or 'skills.brawl').
//
// Render-time consumer: src/lib/components/CharacterCard.svelte uses the
// returned map to annotate View 2 entries with strikethrough baseline + red
// modified value + delta badge.
//
// See: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6.3.

import type {
  BridgeCharacter,
  CharacterModifier,
  SourceKind,
} from '../../types';

/** Per-path projection result. */
export interface DeltaEntry {
  /** Canonical path, e.g. 'attributes.charisma'. */
  path: string;
  /** Value read from the character at this path. 0 for non-existent paths. */
  baseline: number;
  /** Sum of all active stat-kind effects targeting this path. Never zero — zero entries are omitted from the returned map. */
  delta: number;
  /** baseline + delta. */
  modified: number;
  /** Modifier names contributing to this delta, for hover-tooltip display. */
  sources: { modifierId: number; modifierName: string; scope: string | null }[];
}

/**
 * Read the integer value at a canonical path on a character.
 * Returns 0 for non-existent paths or non-numeric values.
 *
 * Supported path shapes:
 *   - 'attributes.<name>' — Foundry: raw.system.attributes.<name>.value
 *   - 'skills.<name>'     — Foundry: raw.system.skills.<name>.value
 *
 * Roll20: returns 0 (Roll20 sources don't expose attribute/skill data via
 * canonical paths; modifier projections on Roll20 chars render as `0 → delta`).
 */
function readPath(char: BridgeCharacter, path: string): number {
  if (char.source !== 'foundry') return 0;
  const raw = char.raw as { system?: Record<string, Record<string, { value?: unknown }>> } | null;
  if (!raw?.system) return 0;
  const dot = path.indexOf('.');
  if (dot < 0) return 0;
  const head = path.slice(0, dot);
  const tail = path.slice(dot + 1);
  const node = raw.system[head]?.[tail];
  const v = node?.value;
  return typeof v === 'number' ? v : 0;
}

/**
 * Match a modifier to a character by `(source, source_id)` key.
 * Mirrors §3 of `2026-05-03-gm-screen-design.md` — modifiers anchor to live
 * characters via this composite key, not via FK.
 */
function modifierMatchesChar(
  m: CharacterModifier,
  source: SourceKind,
  sourceId: string,
): boolean {
  return m.source === source && m.sourceId === sourceId;
}

/**
 * Project active stat-kind modifiers onto a character's canonical paths.
 *
 * Returns a Map keyed by path. Entries with `delta === 0` after summation
 * (two opposing modifiers that cancel) are omitted from the map — the View 2
 * renderer uses `map.has(path)` to decide whether to apply the annotation.
 *
 * Inactive modifiers and modifiers belonging to other characters are filtered
 * out; the caller does NOT need to pre-filter `modifiers`.
 */
export function computeActiveDeltas(
  char: BridgeCharacter,
  modifiers: CharacterModifier[],
): Map<string, DeltaEntry> {
  const acc = new Map<string, DeltaEntry>();

  for (const m of modifiers) {
    if (!m.isActive) continue;
    if (!modifierMatchesChar(m, char.source, char.source_id)) continue;
    for (const e of m.effects) {
      if (e.kind !== 'stat') continue;
      const delta = e.delta ?? 0;
      if (delta === 0) continue;
      for (const path of e.paths) {
        const existing = acc.get(path);
        if (existing) {
          existing.delta += delta;
          existing.modified = existing.baseline + existing.delta;
          existing.sources.push({ modifierId: m.id, modifierName: m.name, scope: e.scope });
        } else {
          const baseline = readPath(char, path);
          acc.set(path, {
            path,
            baseline,
            delta,
            modified: baseline + delta,
            sources: [{ modifierId: m.id, modifierName: m.name, scope: e.scope }],
          });
        }
      }
    }
  }

  // Drop entries where opposing modifiers summed to zero.
  for (const [path, entry] of acc) {
    if (entry.delta === 0) acc.delete(path);
  }

  return acc;
}

/**
 * Convenience: collect the set of advantage `item_id`s belonging to active
 * modifiers on this character. View 4 chip-rendering uses this to flip a
 * chip's data-active attribute when its `_id` matches.
 */
export function activeAdvantageItemIds(
  char: BridgeCharacter,
  modifiers: CharacterModifier[],
): Set<string> {
  const out = new Set<string>();
  for (const m of modifiers) {
    if (!m.isActive) continue;
    if (!modifierMatchesChar(m, char.source, char.source_id)) continue;
    if (m.binding.kind === 'advantage') out.add(m.binding.item_id);
  }
  return out;
}
```

- [ ] **Step 2:** Run `./scripts/verify.sh`. Expected: green. (`npm run check` validates the new file in isolation; no consumer yet.)

- [ ] **Step 3:** Commit.

```bash
git add src/lib/character/active-deltas.ts
git commit -m "$(cat <<'EOF'
feat(character): add computeActiveDeltas projection

Pure function projecting active stat-kind modifiers onto a character's
canonical paths. Returns a Map<path, DeltaEntry> keyed by canonical path.
Inactive modifiers and modifiers for other characters are filtered out;
opposing modifiers that sum to zero are omitted from the map.

Path resolver inlined (per spec §14.1 — factor on second use). Supports
Foundry attributes/skills today; Roll20 reads return 0 (no canonical path
exposure on Roll20 sources).

Companion helper activeAdvantageItemIds() returns the Set of advantage
item_ids belonging to active modifiers, used by the card to flip
data-active on View 4 chips.

No frontend tests — ARCH §10. Manual verification per spec §11 follows
in the next commit.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6.3
EOF
)"
```

---

## Task 4 — Wire subscription into `CharacterCard.svelte`

This task touches `CharacterCard.svelte` from Plan A — the only file Plan A's anti-scope explicitly *unfreezes* for Plan B (anti-scope reads "view layout frozen", not "the file frozen"). Behavior changes only — markup structure and styles stay as Plan A shipped.

**Files:**
- Modify: `src/lib/components/CharacterCard.svelte`

- [ ] **Step 1:** Add imports at the top of the script block. After the existing imports:

```ts
  import { modifiers as modifiersStore } from '../../store/modifiers.svelte';
  import { computeActiveDeltas, activeAdvantageItemIds } from '$lib/character/active-deltas';
  import ModifierEffectEditor from './gm-screen/ModifierEffectEditor.svelte';
  import type { ModifierEffect } from '../../types';
  import { onMount } from 'svelte';
```

- [ ] **Step 2:** Add subscription state inside the script block. After the `viewIndex` derived expression and before the `VIEW_LABELS` constant:

```ts
  // ── Modifier subscription ─────────────────────────────────────────────
  onMount(() => { void modifiersStore.ensureLoaded(); });

  const characterModifiers = $derived(
    modifiersStore.list.filter(
      m => m.source === character.source && m.sourceId === character.source_id,
    ),
  );
  const activeDeltas = $derived(
    computeActiveDeltas(character, modifiersStore.list),
  );
  const activeChipIds = $derived(
    activeAdvantageItemIds(character, modifiersStore.list),
  );
  const hasActiveModifiers = $derived(
    characterModifiers.some(m => m.isActive),
  );

  function deltaTooltip(path: string): string {
    const entry = activeDeltas.get(path);
    if (!entry) return '';
    return entry.sources
      .map(s => `${s.modifierName}${s.scope ? ' — ' + s.scope : ''} (${entry.delta >= 0 ? '+' : ''}${entry.delta})`)
      .join('\n');
  }

  // ── Chip click — toggle activation ────────────────────────────────────
  let chipBusy = $state<string | null>(null);

  async function toggleChip(itemId: string, name: string, description: string) {
    if (character.source !== 'foundry') return; // Roll20 chip toggle deferred to Phase 2.5.
    chipBusy = itemId;
    try {
      const existing = characterModifiers.find(
        m => m.binding.kind === 'advantage' && m.binding.item_id === itemId,
      );
      if (existing) {
        await modifiersStore.setActive(existing.id, !existing.isActive);
      } else {
        const created = await modifiersStore.materializeAdvantage({
          source: character.source,
          sourceId: character.source_id,
          itemId,
          name,
          description,
        });
        await modifiersStore.setActive(created.id, true);
      }
    } catch (e) {
      console.error('[CharacterCard] toggleChip failed:', e);
      window.alert(String(e));
    } finally {
      if (chipBusy === itemId) chipBusy = null;
    }
  }

  // ── Chip right-click — open editor popover ────────────────────────────
  let editorTarget = $state<{ itemId: string; name: string; description: string; effects: ModifierEffect[]; tags: string[] } | null>(null);

  async function openChipEditor(itemId: string, name: string, description: string, ev: Event) {
    ev.preventDefault(); // suppress browser context menu
    if (character.source !== 'foundry') return;
    let modifier = characterModifiers.find(
      m => m.binding.kind === 'advantage' && m.binding.item_id === itemId,
    );
    if (!modifier) {
      modifier = await modifiersStore.materializeAdvantage({
        source: character.source,
        sourceId: character.source_id,
        itemId,
        name,
        description,
      });
    }
    editorTarget = {
      itemId,
      name: modifier.name,
      description: modifier.description,
      effects: modifier.effects.map(e => ({ ...e })),
      tags: [...modifier.tags],
    };
  }

  async function saveChipEditor(effects: ModifierEffect[], tags: string[]) {
    if (!editorTarget) return;
    const modifier = characterModifiers.find(
      m => m.binding.kind === 'advantage' && m.binding.item_id === editorTarget!.itemId,
    );
    if (!modifier) return;
    await modifiersStore.update(modifier.id, { effects, tags });
    editorTarget = null;
  }

  function closeChipEditor() { editorTarget = null; }
```

- [ ] **Step 3:** Add the active-modifiers banner. Find the `<div class="panel">` opener in the template (added in Plan A Step 3). Insert the banner element **inside** the `.panel` div but BEFORE the `{#if viewIndex === 1}` block:

```svelte
  <div class="panel">
    {#if hasActiveModifiers}
      <div class="modifier-banner" title="Active modifiers on this character">
        <span class="banner-label">Active modifiers</span>
        <span class="banner-count">{characterModifiers.filter(m => m.isActive).length}</span>
      </div>
    {/if}
    {#if viewIndex === 1}
      ...
    {/if}
  </div>
```

  Add the corresponding styles to the existing `<style>` block:

```css
  .modifier-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: color-mix(in srgb, var(--alert-card-dossier) 10%, transparent);
    border: 1px solid var(--alert-card-dossier);
    color: var(--alert-card-dossier);
    padding: calc(0.25rem * var(--card-scale, 1)) calc(0.5rem * var(--card-scale, 1));
    font-size: calc(0.65rem * var(--card-scale, 1));
    letter-spacing: 0.12em;
    text-transform: uppercase;
    border-radius: calc(0.2rem * var(--card-scale, 1));
    margin-bottom: calc(0.4rem * var(--card-scale, 1));
  }
  .modifier-banner .banner-count {
    background: var(--alert-card-dossier);
    color: var(--bg-card-dossier);
    font-weight: 700;
    padding: 0 0.5em;
    border-radius: 999px;
  }
```

- [ ] **Step 4:** Annotate View 2 attribute and skill entries with delta visualization. Find the View 2 `<div class="attr-grid">` block and modify the `<div class="attr-cell">` template to read from `activeDeltas`. Replace the existing per-cell:

```svelte
      <div class="attr-cell" data-path={`attributes.${n}`}>
        <span class="attr-name">{abbr}</span>
        <span class="attr-val">{attrInt(character, n)}</span>
      </div>
```

with:

```svelte
      {@const path = `attributes.${n}`}
      {@const delta = activeDeltas.get(path)}
      <div class="attr-cell" data-path={path} class:modified={!!delta} title={deltaTooltip(path)}>
        <span class="attr-name">{abbr}</span>
        {#if delta}
          <span class="attr-val">
            <span class="baseline">{delta.baseline}</span>{delta.modified}
            <span class="delta-badge">{delta.delta > 0 ? '+' : ''}{delta.delta}</span>
          </span>
        {:else}
          <span class="attr-val">{attrInt(character, n)}</span>
        {/if}
      </div>
```

  Apply the analogous pattern to the skill row block. Replace:

```svelte
        <div class="skill-row" data-path={`skills.${s.name}`}>
          <span class="skill-name">{s.name}</span>
          <span class="skill-val">{s.value}</span>
        </div>
```

with:

```svelte
        {@const sPath = `skills.${s.name}`}
        {@const sDelta = activeDeltas.get(sPath)}
        <div class="skill-row" data-path={sPath} class:modified={!!sDelta} title={deltaTooltip(sPath)}>
          <span class="skill-name">{s.name}</span>
          {#if sDelta}
            <span class="skill-val">
              <span class="baseline">{sDelta.baseline}</span>{sDelta.modified}
              <span class="delta-badge">{sDelta.delta > 0 ? '+' : ''}{sDelta.delta}</span>
            </span>
          {:else}
            <span class="skill-val">{s.value}</span>
          {/if}
        </div>
```

  Add styles for the delta visualization:

```css
  .attr-cell.modified .attr-val,
  .skill-row.modified .skill-val {
    color: var(--alert-card-dossier);
    font-weight: 700;
  }
  .attr-cell.modified .baseline,
  .skill-row.modified .baseline {
    color: color-mix(in srgb, var(--text-card-dossier) 40%, transparent);
    text-decoration: line-through;
    font-weight: 400;
    margin-right: 0.25em;
  }
  .delta-badge {
    background: var(--alert-card-dossier);
    color: var(--bg-card-dossier);
    font-size: calc(0.55rem * var(--card-scale, 1));
    font-weight: 700;
    padding: 0 0.3em;
    border-radius: calc(0.15rem * var(--card-scale, 1));
    margin-left: 0.3em;
    letter-spacing: 0.04em;
  }
```

- [ ] **Step 5:** Wire chip click handlers on View 4. Find each `<span class="chip ...">` in the four advantage sections (merits, flaws, backgrounds, boons). Replace the `data-active="false"` static attribute with a dynamic value driven by `activeChipIds`, and add `onclick` + `oncontextmenu`. The merit chip changes from:

```svelte
          <span class="chip merit" data-active="false" data-item-id={m._id}>
            <span class="feat-name">{m.name}</span>
            {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
            {@render chipRemoveBtn(character, 'merit', m)}
          </span>
```

to:

```svelte
          <span
            class="chip merit"
            data-active={activeChipIds.has(m._id)}
            data-item-id={m._id}
            data-busy={chipBusy === m._id}
            role="button" tabindex="0"
            onclick={() => toggleChip(m._id, m.name, '')}
            oncontextmenu={(ev) => openChipEditor(m._id, m.name, '', ev)}
            onkeydown={(ev) => { if (ev.key === 'Enter' || ev.key === ' ') { ev.preventDefault(); toggleChip(m._id, m.name, ''); } }}
          >
            <span class="feat-name">{m.name}</span>
            {#if points > 0}<span class="dots">{'●'.repeat(Math.min(points, 5))}</span>{/if}
            {@render chipRemoveBtn(character, 'merit', m)}
          </span>
```

  Apply the same transformation to the flaw, background, and boon chip blocks (replacing `m._id` with `f._id`, `b._id`, `bn._id` respectively). The `actorFx` chips stay unchanged — they are Foundry-managed and not toggleable.

  Critical: the existing chip-remove button is INSIDE the chip span. The remove button's onclick must NOT bubble up and trigger the toggle. Update the `chipRemoveBtn` snippet to call `ev.stopPropagation()` on its onclick:

```svelte
{#snippet chipRemoveBtn(c: BridgeCharacter, ft: FeatureType, item: FoundryItem)}
  {@const allowed = advantageEditAllowed(c)}
  {@const busy    = busyAdvantageKey === advantageBusyKey(c, item._id)}
  {#if allowed}
    <button type="button" class="chip-remove-btn"
      onclick={(ev) => { ev.stopPropagation(); removeAdvantage(c, ft, item); }}
      disabled={busy} aria-busy={busy}
      title={`Remove ${ft}`} aria-label={`Remove ${ft} ${item.name}`}>×</button>
  {/if}
{/snippet}
```

- [ ] **Step 6:** Render the editor popover when `editorTarget` is set. At the bottom of the template (after the closing `</div>` of `.dossier`), add:

```svelte
{#if editorTarget}
  <div class="editor-overlay" onclick={closeChipEditor} role="presentation">
    <div class="editor-anchor" onclick={(ev) => ev.stopPropagation()} role="presentation">
      <ModifierEffectEditor
        initialEffects={editorTarget.effects}
        initialTags={editorTarget.tags}
        onSave={saveChipEditor}
        onCancel={closeChipEditor}
      />
    </div>
  </div>
{/if}
```

  Add styles:

```css
  .editor-overlay {
    position: fixed; inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: grid; place-items: center;
    z-index: 100;
  }
```

  (The popover anchors to the viewport center for v1 rather than to the chip itself — anchored-positioning is a polish pass; the spec §6.5 says "anchored to the chip" but a centered overlay is acceptable for v1 and easier to implement reliably across density levels. If the implementer is comfortable with anchored-positioning APIs and the dev WebView supports them, anchor instead.)

- [ ] **Step 7:** Update the active-state CSS that Plan A pre-shipped. Plan A's CSS for `.chip[data-active="true"]` should already be in place — verify it. The selector should look something like:

```css
  .chip { /* existing baseline styles */ cursor: pointer; }
  .chip[data-active="true"] {
    background: var(--alert-card-dossier);
    color: var(--bg-card-dossier);
    border-color: var(--alert-card-dossier);
    box-shadow: 0 0 calc(0.6rem * var(--card-scale, 1)) color-mix(in srgb, var(--alert-card-dossier) 40%, transparent);
    font-weight: 700;
    position: relative;
  }
  .chip[data-active="true"] .dots { color: var(--bg-card-dossier); }
  .chip[data-active="true"]::after {
    content: '◉';
    position: absolute;
    top: calc(-0.18rem * var(--card-scale, 1));
    right: calc(-0.18rem * var(--card-scale, 1));
    background: var(--bg-card-dossier);
    color: var(--alert-card-dossier);
    border-radius: 50%;
    width:  calc(0.6rem * var(--card-scale, 1));
    height: calc(0.6rem * var(--card-scale, 1));
    font-size: calc(0.5rem * var(--card-scale, 1));
    display: grid; place-items: center;
  }
  .chip[data-busy="true"] { opacity: 0.6; pointer-events: none; }
```

  If Plan A didn't ship this, add it. If it did, leave alone.

- [ ] **Step 8:** Run `./scripts/verify.sh`. Expected: green.

- [ ] **Step 9:** Manual smoke verification (per spec §11). Run `npm run tauri dev`. With a Foundry actor connected:

  1. Open Campaign view → cards render with View 1 (Basics) active. No active-modifier banner (no modifiers yet).
  2. Flip to View 4 (Advantages). Click a merit chip — chip turns red with the ◉ corner marker; an `Active modifiers · 1` banner appears at the top of the panel.
  3. Flip to View 2 (Stats). The banner is still visible, but no attribute or skill is annotated yet (the modifier has no Stat effects — just `is_active = true`).
  4. Flip back to View 4. Right-click the same merit chip. The `ModifierEffectEditor` overlay opens. Click `+ Add effect` → in the new row, change kind to `Stat` (the new option). The italic "render-time only ⓘ" helper appears next to the kind dropdown. Tooltip-hover confirms the asymmetry message.
  5. Set scope (optional, e.g., "vs social"), set delta to `+1`, add path `attributes.charisma`. Save.
  6. Flip to View 2 — Charisma row should now show `[baseline-strikethrough] [modified-red] [+1 badge]`. Tooltip-hover on the Charisma row shows the modifier name + scope.
  7. Click the merit chip again — chip de-activates; banner disappears; Charisma annotation disappears.
  8. Click chip again to re-activate — Charisma annotation re-appears (modifier persists, `is_active` round-trips).
  9. Add a second Stat effect on a *different* modifier targeting the same path with delta `-1`, both active. Charisma annotation should disappear (deltas cancel; entry omitted).

  If any step fails, fix and re-verify before committing.

- [ ] **Step 10:** Commit.

```bash
git add src/lib/components/CharacterCard.svelte
git commit -m "$(cat <<'EOF'
feat(character-card): wire modifier subscription, stat deltas, chip toggle

CharacterCard subscribes to the existing modifiers store via the new
computeActiveDeltas projection and activeAdvantageItemIds helper. View 4
chips become clickable: click toggles is_active (materializing the modifier
row first if needed), right-click opens ModifierEffectEditor anchored as a
centered overlay. View 2 attribute and skill entries whose canonical paths
appear in activeDeltas render with strikethrough baseline + red modified
value + delta badge. An "Active modifiers · N" banner surfaces above the
panel whenever the character has ≥1 active modifier.

Manual verification per spec §11 — Foundry round-trip exercised; no Roll20
cases (Phase 2.5).

Closes the Character Card Redesign feature: visual refactor (Plan A) +
modifier integration (Plan B) ship together.

Refs: docs/superpowers/specs/2026-05-10-character-card-redesign-design.md §6
EOF
)"
```

---

## Self-Review Checklist (run after Task 4 commit)

- [ ] **Spec coverage:** Every §6 requirement of the spec is implemented. Specifically: `Stat` ModifierKind variant; `paths[]` reuse; `computeActiveDeltas` returning `Map<path, DeltaEntry>`; opposing-modifiers cancel and omit; non-existent path treated as baseline-0; chip click materializes-then-toggles; right-click chip opens editor; banner appears when ≥1 active modifier; View 2 annotations with baseline / modified / badge / tooltip. §6.6 banner click is informational only (out of v1).
- [ ] **Anti-scope respected:** No changes to `CharacterCardShell.svelte`, `Campaign.svelte` shell-rail markup, `:root` token additions (all frozen by Plan A). No new Tauri commands. No schema migration. No new database tables.
- [ ] **Token discipline:** No hex literals introduced (`grep -E '#[0-9a-fA-F]{3,6}' src/lib/components/CharacterCard.svelte src/lib/character/active-deltas.ts src/lib/components/gm-screen/ModifierEffectEditor.svelte`). Every color via `var(--*)`.
- [ ] **Render-time vs roll-time isolation:** `computeActiveDeltas` ignores `pool` / `difficulty` / `note` kinds — only `stat`. Verify by reading the function: the kind filter on line `if (e.kind !== 'stat') continue;` is the load-bearing isolation.
- [ ] **No frontend test framework introduced.** ARCH §10 invariant. Plan B's only new test is the Rust serde test from Task 1.
- [ ] **`./scripts/verify.sh`** green for the final commit.

---

## Open questions (deferred from spec §14)

These remain unresolved at plan-completion time. Address in follow-up specs / commits as the need arises:

1. **Path resolver factoring.** Plan B inlines `readPath()` in `active-deltas.ts`. If a third consumer needs canonical path reads (likely candidates: a future "differential roll dispatch" or a stat-editor UI), factor into `src/lib/character/paths.ts` and re-export from both `diff.ts` and `active-deltas.ts`. Defer until that second use lands.
2. **Editor anchoring to chip.** Plan B uses a centered overlay; spec §6.5 prefers chip-anchored. Anchored-positioning (CSS `anchor-name` / `position-anchor`) is supported in Tauri 2's WebViews on most modern OSes but stability varies. A polish pass after the feature ships in production is the right time to upgrade.
3. **Active-modifiers banner navigation.** v1 banner is informational. Future cross-tool nav (banner click → GM Screen scrolled to row) needs a navigation helper that doesn't exist today; out of v1 scope.
