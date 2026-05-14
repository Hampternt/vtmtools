# Foundry skills/attrs — Plan A: Skills display in Campaign manager (and fix Foundry attr display)

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.
>
> **Project lean-execution override (CLAUDE.md):** dispatch ONE implementer subagent per task with full task text + scene-setting context, run `./scripts/verify.sh` after the implementer commits, then move on. After ALL Plan A tasks are committed, run a SINGLE `code-review:code-review` against the full Plan A branch diff.
>
> **TDD-on-demand override (CLAUDE.md):** subagents do NOT auto-invoke `superpowers:test-driven-development`. Each task below explicitly states whether tests are required.

**Goal:** (1) Lay the foundation `canonical-names.ts` module that exports `AttributeName` / `SkillName` literal types and `FOUNDRY_ATTRIBUTE_NAMES` / `FOUNDRY_SKILL_NAMES` runtime arrays — Plans B and C both depend on it. (2) Fix the existing bug where `Campaign.svelte`'s "Core attributes" section reads via `r20AttrInt`, causing all 9 attribute values to display as `0` for any Foundry character. (3) Add a new "Skills" collapsible section showing all 27 WoD5e skills for Foundry characters.

**Architecture:** Pure-frontend change. New TS module under `src/lib/foundry/` mirrors the existing pattern (`raw.ts` lives there). New source-aware `attrInt` / `skillInt` helpers inside `Campaign.svelte` dispatch to the existing `r20AttrInt` / `foundryAttrInt` / `foundrySkillInt` primitives by `char.source`. Skills section guard-rendered with `{#if char.source === 'foundry'}`.

**Tech Stack:** Svelte 5 runes mode, TypeScript. No backend changes, no IPC, no SQL migrations.

**Spec:** `docs/superpowers/specs/2026-05-09-foundry-skills-attrs-and-roll-dispatcher-design.md` §4 (Plan A).
**Architecture reference:** `ARCHITECTURE.md` §6 (CSS / token invariants).

**Spec defaults adopted:**
- Show all 27 skills, sorted alphabetically (spec §4.2).
- 3-column grid layout (spec §4.2).
- Roll20 chars: skills toggle button hidden (spec §4.2).

**Depends on:** nothing. Plan A is the foundation for Plans B and C.

---

## File structure

### Files created

| Path | Responsibility |
|---|---|
| `src/lib/foundry/canonical-names.ts` | Single source of truth for `AttributeName` / `SkillName` literal types and the matching runtime arrays. Mirrors `src-tauri/src/shared/canonical_fields.rs::ATTRIBUTE_NAMES` / `SKILL_NAMES` (Plan B). |

### Files modified

| Path | Change |
|---|---|
| `src/tools/Campaign.svelte` | (a) Add `attrInt` / `skillInt` source-aware helpers; (b) replace 9 inline `r20AttrInt(char, ...)` calls with `attrInt(char, ...)`; (c) add `expandedSkills` state + toggle function; (d) add Skills collapsible section + `skills` footer button; (e) add `.skill-grid` CSS rule. |

### Files NOT touched in Plan A

- Anything under `src-tauri/` — no backend changes.
- `src/types.ts` — Plan B owns the `CanonicalFieldName` extension.
- `src/lib/foundry/raw.ts` — `foundryAttrInt` / `foundrySkillInt` already exist.
- `src/lib/components/gm-screen/**` — Plan C territory.

---

## Task A1: Create `canonical-names.ts` foundation module

**Goal:** Land the literal types and runtime arrays in one self-contained module. Plans B and C will both import from this file.

**Files:**
- Create: `src/lib/foundry/canonical-names.ts`

**Anti-scope:** Do not modify `src/types.ts` (Plan B's territory). Do not import from this file yet (the consumer wiring is Tasks A2 and Plan B Task B5).

**Depends on:** nothing.

**Invariants cited:** ARCH §6 (no hex literals — N/A here, no styling).

**Tests required:** NO. This file is a pure-data module; verification is `npm run check` proving the literal types compile cleanly.

- [x] **Step 1: Create the file with the full content**

```typescript
// src/lib/foundry/canonical-names.ts
//
// Canonical names for WoD5e attributes and skills. Foundation module for:
//   - Plan A: Campaign.svelte's Skills section iterates FOUNDRY_SKILL_NAMES.
//   - Plan B: src/types.ts's CanonicalFieldName template-literal type
//             extends `attribute.${AttributeName}` and `skill.${SkillName}`.
//   - Plan C: src/lib/gm-screen/path-providers.ts's ATTRIBUTE_PROVIDER /
//             SKILL_PROVIDER iterate the runtime arrays for dropdown options.
//
// Mirrors src-tauri/src/shared/canonical_fields.rs::ATTRIBUTE_NAMES and
// SKILL_NAMES. When changing this list, update the Rust arrays in the same
// commit (manual checklist convention; matches the BridgeCharacter mirror
// pattern in src/types.ts).

/** WoD5e v5.3.17 attribute keys (system.attributes.<key>.value). */
export type AttributeName =
  | 'charisma'
  | 'composure'
  | 'dexterity'
  | 'intelligence'
  | 'manipulation'
  | 'resolve'
  | 'stamina'
  | 'strength'
  | 'wits';

/** WoD5e v5.3.17 skill keys (system.skills.<key>.value). */
export type SkillName =
  | 'academics'
  | 'animalken'
  | 'athletics'
  | 'awareness'
  | 'brawl'
  | 'craft'
  | 'drive'
  | 'etiquette'
  | 'finance'
  | 'firearms'
  | 'insight'
  | 'intimidation'
  | 'investigation'
  | 'larceny'
  | 'leadership'
  | 'medicine'
  | 'melee'
  | 'occult'
  | 'performance'
  | 'persuasion'
  | 'politics'
  | 'science'
  | 'stealth'
  | 'streetwise'
  | 'subterfuge'
  | 'survival'
  | 'technology';

/**
 * Runtime array of attribute keys, sorted alphabetically (matches the WoD5e
 * sheet's display order). Use this to iterate when rendering dropdowns or
 * grids — never hardcode the list elsewhere.
 *
 * Mirrors src-tauri/src/shared/canonical_fields.rs::ATTRIBUTE_NAMES.
 */
export const FOUNDRY_ATTRIBUTE_NAMES: readonly AttributeName[] = [
  'charisma',
  'composure',
  'dexterity',
  'intelligence',
  'manipulation',
  'resolve',
  'stamina',
  'strength',
  'wits',
] as const;

/**
 * Runtime array of skill keys, sorted alphabetically.
 *
 * Mirrors src-tauri/src/shared/canonical_fields.rs::SKILL_NAMES.
 */
export const FOUNDRY_SKILL_NAMES: readonly SkillName[] = [
  'academics',
  'animalken',
  'athletics',
  'awareness',
  'brawl',
  'craft',
  'drive',
  'etiquette',
  'finance',
  'firearms',
  'insight',
  'intimidation',
  'investigation',
  'larceny',
  'leadership',
  'medicine',
  'melee',
  'occult',
  'performance',
  'persuasion',
  'politics',
  'science',
  'stealth',
  'streetwise',
  'subterfuge',
  'survival',
  'technology',
] as const;
```

- [x] **Step 2: Run `npm run check`**

Run: `npm run check`
Expected: PASS — no new TS errors. The file isn't imported anywhere yet, so the only check is that it compiles in isolation.

- [x] **Step 3: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS — full repo gate (cargo + npm check + frontend build).

- [x] **Step 4: Commit**

```bash
git add src/lib/foundry/canonical-names.ts
git commit -m "feat(foundry): add canonical-names.ts with attribute/skill types

Foundation module for Plan B (CanonicalFieldName template literal) and
Plan C (PathProvider registry). Exports AttributeName/SkillName literal
unions and FOUNDRY_ATTRIBUTE_NAMES/FOUNDRY_SKILL_NAMES runtime arrays.
Mirrors the Rust ATTRIBUTE_NAMES/SKILL_NAMES arrays that Plan B will
add to canonical_fields.rs (manual-checklist drift policy)."
```

---

## Task A2: Add source-aware helpers + fix attribute display + add Skills section in Campaign.svelte

**Goal:** Single coherent commit that (a) adds `attrInt` / `skillInt` helpers, (b) replaces the 9 inline `r20AttrInt` calls with `attrInt`, (c) adds `expandedSkills` state and `toggleSkills`, (d) adds the Skills collapsible section in the markup, (e) adds the `skills` footer toggle button (Foundry-only), (f) adds the `.skill-grid` CSS rule.

**Files:**
- Modify: `src/tools/Campaign.svelte`

**Anti-scope:** Do not change `r20AttrInt` / `r20AttrText` / `r20Attrs` / `parseDisciplines` (Roll20-specific helpers stay as-is). Do not touch the existing `expandedAttrs` toggle behavior. Do not touch `src/lib/foundry/raw.ts`.

**Depends on:** Task A1 (imports from `canonical-names.ts`).

**Invariants cited:** ARCH §6 (use `var(--*)` tokens; no hex literals — the new `.skill-grid` reuses `var(--bg-sunken)`, `var(--text-ghost)`, `var(--text-primary)` from the existing `.attr-grid`). CLAUDE.md: never call `invoke()` directly from a component (N/A — no IPC here).

**Tests required:** NO. Pure UI rendering changes; the helpers are 2-line dispatch functions. Verification is `./scripts/verify.sh` + the manual smoke at the end of the plan.

- [x] **Step 1: Add imports at the top of the `<script>` block**

Locate the existing imports at the top of `src/tools/Campaign.svelte` (around lines 1-50). Add:

```typescript
import {
  FOUNDRY_ATTRIBUTE_NAMES,
  FOUNDRY_SKILL_NAMES,
} from '../lib/foundry/canonical-names';
import { foundryAttrInt, foundrySkillInt } from '../lib/foundry/raw';
```

(`BridgeCharacter` and `Roll20Raw` types are already imported from `'../types'`. The Roll20-specific helpers `r20Attrs`, `r20AttrInt`, `r20AttrText`, `parseDisciplines` are local functions defined later in the script — no import change needed for those.)

- [x] **Step 2: Add the `attrInt` and `skillInt` source-aware helpers**

Find the `r20AttrText` function (around `Campaign.svelte:174`). Immediately after it, add:

```typescript
  /// Source-aware attribute reader. Foundry: walks system.attributes.<name>.value
  /// via the shared foundryAttrInt helper. Roll20: walks the flat attribute
  /// list via r20AttrInt. Returns 0 for unknown names or unsupported sources.
  function attrInt(char: BridgeCharacter, name: string): number {
    if (char.source === 'foundry') return foundryAttrInt(char, name);
    return r20AttrInt(char, name);
  }

  /// Source-aware skill reader. Foundry only — Roll20 sheets don't expose
  /// skills via this codepath. Returns 0 for non-Foundry chars (the markup
  /// guards the Skills section behind {#if char.source === 'foundry'} so this
  /// fallback is defensive).
  function skillInt(char: BridgeCharacter, name: string): number {
    if (char.source === 'foundry') return foundrySkillInt(char, name);
    return 0;
  }
```

- [x] **Step 3: Add the `expandedSkills` state next to `expandedAttrs`**

Find the `expandedAttrs` declaration (around `Campaign.svelte:63`). Immediately after it, add:

```typescript
  let expandedSkills = $state<Set<string>>(new Set());
```

- [x] **Step 4: Add the `toggleSkills` function next to `toggleAttrs`**

Find the `toggleAttrs` function (around `Campaign.svelte:210`). Immediately after it, add:

```typescript
  function toggleSkills(id: string) { expandedSkills = toggleSet(expandedSkills, id); }
```

(`toggleSet` is the existing helper used by `toggleAttrs`, `toggleInfo`, `toggleFeats`, `toggleRaw` — no new utility needed.)

- [x] **Step 5: Replace the 9 inline `r20AttrInt` calls with `attrInt`**

Find the `{@const}` block around `Campaign.svelte:569-577`. Replace exactly these 9 lines:

```svelte
        {@const strAttr      = r20AttrInt(char, 'strength')}
        {@const dexAttr      = r20AttrInt(char, 'dexterity')}
        {@const staAttr      = r20AttrInt(char, 'stamina')}
        {@const chaAttr      = r20AttrInt(char, 'charisma')}
        {@const manAttr      = r20AttrInt(char, 'manipulation')}
        {@const comAttr      = r20AttrInt(char, 'composure')}
        {@const intAttr      = r20AttrInt(char, 'intelligence')}
        {@const witAttr      = r20AttrInt(char, 'wits')}
        {@const resAttr      = r20AttrInt(char, 'resolve')}
```

with:

```svelte
        {@const strAttr      = attrInt(char, 'strength')}
        {@const dexAttr      = attrInt(char, 'dexterity')}
        {@const staAttr      = attrInt(char, 'stamina')}
        {@const chaAttr      = attrInt(char, 'charisma')}
        {@const manAttr      = attrInt(char, 'manipulation')}
        {@const comAttr      = attrInt(char, 'composure')}
        {@const intAttr      = attrInt(char, 'intelligence')}
        {@const witAttr      = attrInt(char, 'wits')}
        {@const resAttr      = attrInt(char, 'resolve')}
```

(Only the function name changes; the variable names `strAttr` etc. stay identical so downstream consumers — the markup at `Campaign.svelte:728-736` — keep working unchanged.)

- [x] **Step 6: Add the Skills collapsible section in the markup**

Find the existing `{#if expandedAttrs.has(charKey)}` block (around `Campaign.svelte:725`). After its closing `{/if}` (the one that ends the attributes collapsible — locate the line just before the next sibling collapsible, which is `{#if expandedInfo.has(charKey)}`), add:

```svelte
          <!-- ── Collapsible: skills (Foundry only) ──────────────────────── -->
          {#if char.source === 'foundry' && expandedSkills.has(charKey)}
            <div class="card-section">
              <div class="skill-grid">
                {#each FOUNDRY_SKILL_NAMES as skill (skill)}
                  <div class="attr-cell">
                    <span class="attr-name">{skill}</span>
                    <span class="attr-val">{skillInt(char, skill)}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
```

(Reuses the `.attr-cell` class for consistent cell styling. The new `.skill-grid` rule lives in CSS — Step 8.)

- [x] **Step 7: Add the `skills` footer toggle button**

Find the footer's existing `attrs` button (around `Campaign.svelte:940-942`):

```svelte
            <button class="section-toggle" onclick={() => toggleAttrs(charKey)}>
              attrs {expandedAttrs.has(charKey) ? '▴' : '▾'}
            </button>
```

Immediately after that button (still inside the `<div class="card-footer">`), add:

```svelte
            {#if char.source === 'foundry'}
              <button class="section-toggle" onclick={() => toggleSkills(charKey)}>
                skills {expandedSkills.has(charKey) ? '▴' : '▾'}
              </button>
            {/if}
```

(The `{#if}` guard ensures the button only renders for Foundry characters. Roll20 character footers stay unchanged.)

- [x] **Step 8: Add the `.skill-grid` CSS rule**

Find the existing `.attr-grid` CSS rule (around `Campaign.svelte:1608`). Immediately after the `.attr-grid { ... }` block (before `.attr-cell { ... }`), add:

```css
  /* Skills grid — same 3-col shape as attrs but denser (3×9 = 27 cells) */
  .skill-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.4rem 0.25rem;
    text-align: center;
  }
```

(`.attr-cell`, `.attr-name`, `.attr-val` already provide cell-level styling — no new cell rules needed because Step 6 reuses `.attr-cell`.)

- [x] **Step 9: Run `npm run check`**

Run: `npm run check`
Expected: PASS — Svelte type-check passes; the new helpers compile; the iterated `FOUNDRY_SKILL_NAMES` is correctly typed.

- [x] **Step 10: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: PASS — full repo gate (cargo + npm check + frontend build).

- [x] **Step 11: Commit**

```bash
git add src/tools/Campaign.svelte
git commit -m "feat(campaign): source-aware attrs + Skills section for Foundry chars

Adds attrInt(char, name) and skillInt(char, name) helpers that dispatch
by char.source. Replaces 9 r20AttrInt() call sites in the Core Attributes
section so Foundry characters now render their actual sheet values
instead of zeros (latent bug since Foundry source landed).

Adds a new collapsible 'Skills' section showing all 27 WoD5e skills in
a 3x9 grid, gated to Foundry characters only via {#if char.source}.
The skills toggle button in the card footer is correspondingly hidden
on Roll20 rows.

Iterates FOUNDRY_SKILL_NAMES from the new canonical-names.ts module so
the skill list lives in one place; no inline duplication."
```

---

## Final smoke test (manual, after both tasks committed)

Per CLAUDE.md, verify.sh runs after each commit. The end-of-plan smoke verifies the user-facing behavior end-to-end.

**Setup:** `npm run tauri dev` with a connected Foundry world that has at least one vampire actor, AND a connected Roll20 game with at least one character (or a saved character with `source: 'roll20'`).

- [x] **Foundry attribute fix verification** — open the Campaign tool, expand a Foundry character's "Core attributes" section. **Expected:** all 9 attributes (Str/Dex/Sta/Cha/Man/Com/Int/Wit/Res) display the actor's actual sheet values, NOT zeros. Cross-reference one or two values against the actor's sheet in Foundry directly.

- [x] **Skills section appears for Foundry chars** — in the same Foundry character's footer, verify a `skills ▾` toggle button is present (next to `attrs`, `info`, `feats`). Click it. **Expected:** a 3-column grid of 27 skills appears with their values.

- [x] **Skills values are correct** — cross-reference a couple of the skill values (e.g., `brawl`, `subterfuge`) against the actor's sheet in Foundry. Empty/zero skills should display as `0`.

- [x] **Skills section absent on Roll20** — find a Roll20 character row's footer. **Expected:** NO `skills` toggle button — only `attrs`, `info`, `feats`, raw-toggle.

- [x] **Roll20 attributes regression check** — expand a Roll20 character's "Core attributes". **Expected:** Roll20 attribute values still display correctly (the new `attrInt` helper dispatches to `r20AttrInt` for `source === 'roll20'`).

---

## Self-review checklist

- [x] Did the new `attrInt` / `skillInt` helpers preserve the exact return type (`number`) of the originals they replace? — Yes; `r20AttrInt`, `foundryAttrInt`, `foundrySkillInt` all return `number`.
- [x] Are all 9 `r20AttrInt(char, '<attr>')` call sites in the attributes section converted? — Yes; Step 5 lists the exact 9 lines.
- [x] Does the Skills section's `{#if}` guard cover BOTH the markup section AND the footer toggle button? — Yes; Step 6 guards the section, Step 7 guards the button.
- [x] Does `canonical-names.ts` mirror match the WoD5e v5.3.17 actor sample? — Yes; the 9 attrs and 27 skills are derived from `docs/reference/foundry-vtm5e-actor-sample.json`.
- [x] Does the spec's §10 dependency graph hold? Plan A creates the foundation module that B and C consume. — Yes; this plan's commits land before B and C start.

---

## Plan dependencies

- **Depends on:** nothing.
- **Blocks:** Plan B (imports `AttributeName` / `SkillName` from `canonical-names.ts` for `CanonicalFieldName` template literal). Plan C (imports `FOUNDRY_*_NAMES` from `canonical-names.ts` for `PathProvider` registries).

---

## Execution handoff

Plan A is two tasks; both are mostly mechanical. Recommended:
- **Subagent-driven:** one subagent per task. Task A1 is ~5 min; Task A2 is ~20 min. Two-stage review per CLAUDE.md (lean override: no per-task spec/quality reviewer; final code-review at the end of all three plans, not per-plan).
- **Inline:** also fine. The plan is short enough that batched execution within a single session works.
