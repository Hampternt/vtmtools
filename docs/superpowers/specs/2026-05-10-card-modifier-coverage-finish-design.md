# Card Modifier Coverage Finish — View 1 + View 3 deltas + Banner Navigation

> **Status:** designed; ready for plan-writing.
> **Roadmap fit:** Phase 2 polish — closes out four §12 future seams of `2026-05-10-character-card-redesign-design.md` (Hunger/Health/WP deltas on View 1; discipline-level deltas on View 3; active-modifiers banner click navigation; tooltip / accessibility polish). Sibling track of `2026-05-10-foundry-roll-mirroring-design.md` (independent files, parallelizable).
> **Audience:** anyone extending `CharacterCard.svelte`'s modifier-projection system.
> **Source spec:** `docs/superpowers/specs/2026-05-10-character-card-redesign-design.md` §12 (future seams 1, 3, 4, 5).

---

## §1 What this is

The character-card redesign shipped View-2 stat-delta projection (attributes / skills) but left three §12 future seams open: View 1 vital deltas, View 3 discipline deltas, and the active-modifiers banner click navigation. This spec closes those seams in one focused frontend pass.

The load-bearing change is a **path-resolver generalization** in `src/lib/character/active-deltas.ts` — replacing the hardcoded `<head>.<tail>.value` reader with a generic dot-path walker. That single change unlocks every existing canonical path on `raw.system` and is naturally forward-compatible with future paths.

Per-power deltas on View 3 (e.g., `+1 to Heightened Senses`) are explicitly **deferred to Track 1.5**: powers live in `raw.items[]` (not `raw.system`), so they need a different resolver branch and arguably a different visual treatment. That decision belongs in a future spec.

## §2 Composition — what this builds on

| Piece | What it provides | How this spec uses it |
|---|---|---|
| `2026-05-10-character-card-redesign-design.md` | Card body, four views, stat-delta visualisation pattern on View 2, modifier subscription wiring | This spec extends the same projection vocabulary to View 1 and View 3 — no new components. |
| `src/lib/character/active-deltas.ts` | `computeActiveDeltas()` returning `Map<path, DeltaEntry>` | Resolver internal (`readPath`) is replaced; the public API stays identical. |
| `src/store/toolEvents.ts` | Cross-tool pub/sub channel | New `navigate-to-character` event for banner click. |
| `src-tauri/src/bridge/foundry/translate.rs` | Authoritative path constants for vitals (`hunger.value`, `health.max`, `willpower.max`, `humanity.value`, `humanity.stains`, `blood.potency`) | Source of truth for the canonical-path strings the new resolver must support. |
| `ModifierEffectEditor.svelte` path-input autocomplete (commit `d280178`) | Currently lists `FOUNDRY_ATTRIBUTE_NAMES + FOUNDRY_SKILL_NAMES` | Extended with vital paths and per-actor discipline names. |
| `GmScreen.svelte` `CharacterRow` markup | Existing rows, scrollable container, source/source_id keys | Subscribes to `navigate-to-character`, matches row, scrolls into view. |

## §3 Path resolver generalization

### §3.1 Current shape (active-deltas.ts:44–55)

```ts
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
```

Hard requirements: exactly two segments separated by one dot; leaf must be an object with `.value`. Fails for `hunger`, fails for `health.max`, fails for `humanity.stains`.

### §3.2 New shape — generic dot-walker

```ts
function readPath(char: BridgeCharacter, path: string): number {
  if (char.source !== 'foundry') return 0;
  const raw = char.raw as { system?: unknown } | null;
  if (!raw?.system) return 0;

  let cur: unknown = raw.system;
  for (const seg of path.split('.')) {
    if (cur === null || typeof cur !== 'object') return 0;
    cur = (cur as Record<string, unknown>)[seg];
    if (cur === undefined) return 0;
  }

  // Leaf is a number directly (e.g. health.max).
  if (typeof cur === 'number') return cur;

  // Leaf is an object with .value (e.g. attributes.charisma → { value: 3 }).
  if (cur !== null && typeof cur === 'object') {
    const v = (cur as { value?: unknown }).value;
    if (typeof v === 'number') return v;
  }

  return 0;
}
```

### §3.3 Path coverage table

After §3.2 lands, all following paths resolve correctly:

| Canonical path | Foundry actual | Leaf shape | Notes |
|---|---|---|---|
| `attributes.charisma` | `system.attributes.charisma` | `{ value: number }` | existing |
| `skills.brawl` | `system.skills.brawl` | `{ value: number }` | existing |
| `hunger` | `system.hunger.value` | object → `.value` | **new — View 1** |
| `humanity` | `system.humanity.value` | object → `.value` | **new — View 1** |
| `health.max` | `system.health.max` | number directly | **new — View 1** |
| `health.superficial` | `system.health.superficial` | number directly | **new — View 1** |
| `health.aggravated` | `system.health.aggravated` | number directly | **new — View 1** |
| `willpower.max` | `system.willpower.max` | number directly | **new — View 1** |
| `willpower.superficial` | `system.willpower.superficial` | number directly | **new — View 1** |
| `willpower.aggravated` | `system.willpower.aggravated` | number directly | **new — View 1** |
| `humanity.stains` | `system.humanity.stains` | number directly | **new — View 1** |
| `blood.potency` | `system.blood.potency` | number directly | **new — View 1** |
| `disciplines.auspex` | `system.disciplines.auspex` | object → `.value` (or per-discipline schema) | **new — View 3** |

For `disciplines.<name>`, `translate.rs` doesn't currently extract these — they live on `raw.system.disciplines`. View 3 already reads them today via `canonical.raw.items[]` filter for power tier (existing convention from the original card-redesign spec §5.3); the path-resolver reads the discipline-level dot count directly from `raw.system.disciplines.<name>.value`. **Plan A must verify** that `system.disciplines.<name>` exists in the live Foundry sample — see §10 verification.

## §4 View 1 — Vital deltas

### §4.1 Visual treatment per element

Same projection pattern as View 2 (strikethrough baseline + red modified value + delta badge + tooltip from `entry.sources`). Applied per-element:

- **Hunger drops row**: when `activeDeltas.has('hunger')`, render the drops based on `entry.modified` (red filled drops). Show baseline as muted ghost-drops alongside (struck-through visual): e.g., baseline 2 → modified 4 displays 4 active drops with a small `+2` badge to the right of the row. Hunger ranges 0–5 — clamp render to 5 drops max regardless of delta.
- **BP pill**: when `activeDeltas.has('blood.potency')`, the pill text shows `2 → 4` with a small delta badge. No struck-through visual on the pill itself (it's a single value, not a track).
- **Conscience block**: when `activeDeltas.has('humanity')`, the 10-letter row recomputes its filled count from `entry.modified`. When `activeDeltas.has('humanity.stains')`, the strikethrough/stained letter count recomputes. Both annotations co-exist independently. Delta badge appears on the track-label row (right of `Conscience`).
- **Health block**: when `activeDeltas.has('health.max')`, the box-row count recomputes (extra empty boxes appear). Delta badge on the `Health` track-label row. `health.superficial` / `health.aggravated` paths get a **badge-only** annotation on the track-label row (no fill-position recompute) — these are uncommon modifier targets and a recompute would mean re-laying out filled boxes mid-row, which is visually disruptive for marginal payoff. The resolver supports them (§3.2) so a future spec can promote them if the use-case emerges.
- **Willpower block**: same as Health.

### §4.2 Steppers under modifiers

Steppers (the ± circles next to track-labels) write to the *baseline* via `character::set_field` — they are unchanged by this spec. The delta badge sits between the track-label and the stepper, visually identifying that the displayed value is modified.

### §4.3 Edge cases

- **Negative modified value**: clamped at zero in render only (the delta entry preserves the negative arithmetic). E.g., `health.max` baseline 5, delta -8 → renders 0 boxes with a `−8` badge tooltip "modifier reduced max below zero".
- **Modified value exceeds known maximum**: not clamped — render the full count. Hunger and humanity are bounded 0–5 and 0–10 by V5 rules, but a +3 hunger modifier on a baseline-3 character displays 6 drops (the rule overflow is the GM's concern, not the renderer's).
- **Multiple paths target the same track block** (e.g., both `health.max` and `health.superficial` modifiers active): each path is annotated independently. The track-label row carries the higher-priority badge (`max` deltas dominate `superficial` / `aggravated` deltas visually).

## §5 View 3 — Discipline-level deltas

### §5.1 Visual treatment

Per discipline section (existing structure from card-redesign §5.3):

- The `disc-name` row's dot indicator (`●●●`) gains a delta annotation when `activeDeltas.has('disciplines.<name>')`. Pattern matches View 2: struck-through baseline dots + modified dots in red + delta badge + tooltip.
- The powers list below the discipline header is **unchanged** in v1 — per-power annotation requires the §6 deferred resolver branch.

### §5.2 Discipline-name normalization

Discipline names in canonical paths follow the Foundry system slug (lowercase, no spaces): `auspex`, `dominate`, `presence`, `bloodsorcery` (note: no underscore for compound names per WoD5e convention — verify against `system.disciplines` keys in the live sample). The autocomplete in §7 derives these from the actor's actual disciplines, not a hardcoded list, so the slug spelling is taken from the live data and not invented spec-side.

## §6 Out of scope — Track 1.5 future seam

Per-power deltas (e.g., `+1 dot to Heightened Senses`) are deferred. The shape of that future spec:

- New canonical path form: `power.<itemId>` or `power.<systemSlug>` — TBD. `itemId` is stable across renames but opaque; `systemSlug` is human-readable but fragile.
- Resolver branch: walks `raw.items[]`, filters `type === 'power'`, matches by id/slug, reads the power's dot value. Lives in a new `readItemPath()` helper next to `readPath()`.
- Visual treatment: probably a per-power-line badge in the View 3 powers list, but the per-power lines are tight on space at S density — the design needs a proper visual round.
- ModifierEffectEditor: needs an item-picker UI variant for power paths since the path string is not human-readable.

The path-resolver generalization in §3 of this spec does NOT fit per-power paths — that's correct. They need their own walker, and bundling them in this spec would inflate scope without payoff.

## §7 ModifierEffectEditor path autocomplete extension

The path-input autocomplete (commit `d280178`) currently lists `FOUNDRY_ATTRIBUTE_NAMES + FOUNDRY_SKILL_NAMES`. Two additions:

```ts
// src/lib/foundry/canonical-names.ts (extended)
export const FOUNDRY_VITAL_PATHS = [
  'hunger',
  'humanity',
  'humanity.stains',
  'health.max',
  'willpower.max',
  'blood.potency',
] as const;

// Discipline names are derived per-character from the actor's current
// disciplines, like skills are. Add a helper:
export function foundryDisciplineNames(char: BridgeCharacter): string[] {
  if (char.source !== 'foundry') return [];
  const raw = char.raw as { system?: { disciplines?: Record<string, unknown> } } | null;
  return Object.keys(raw?.system?.disciplines ?? {})
    .map(name => `disciplines.${name}`);
}
```

The editor's autocomplete list becomes `FOUNDRY_ATTRIBUTE_NAMES + FOUNDRY_SKILL_NAMES + FOUNDRY_VITAL_PATHS + foundryDisciplineNames(char)`. The character context flows in from the existing prop chain (editor receives `char` already for the path-context menu).

`health.superficial`, `health.aggravated`, `willpower.superficial`, `willpower.aggravated` are deliberately omitted from autocomplete — they're rare modifier targets and the user can type them by hand. The resolver supports them; the autocomplete lists the common ones.

## §8 Banner click → GM-screen navigation

### §8.1 Event shape

`src/store/toolEvents.ts` gains a new event variant (per ARCH §4 cross-tool pub/sub):

```ts
export type ToolEvent =
  | { type: 'navigate-to-character'; source: SourceKind; sourceId: string }
  | /* existing variants */;
```

### §8.2 Banner publisher

Banner becomes interactive: cursor:pointer, hover state (subtle red glow on the existing red-tinted background). Click handler:

```ts
function onBannerClick() {
  publishEvent({
    type: 'navigate-to-character',
    source: character.source,
    sourceId: character.source_id,
  });
}
```

Right-click and keyboard activation (Enter / Space) route through the same handler. Banner gets `role="button"` + `tabindex="0"` for accessibility.

### §8.3 GmScreen subscriber

`GmScreen.svelte` subscribes on mount. Plan A's first step is to **identify the active-tool state seam** — `Sidebar.svelte` is the canonical reader; it routes clicks somewhere (a writable store, a URL hash, or an `App.svelte`-level state). The plan task names that seam concretely before writing the publish/switch helper. With the seam identified, the subscriber sketch is:

```ts
toolEvents.subscribe(ev => {
  if (ev?.type !== 'navigate-to-character') return;
  // 1. If GM Screen tool not currently active, switch to it via the located seam.
  // 2. After render settles (next tick), find row by source/sourceId and scrollIntoView.
  setActiveTool('gm-screen'); // exact API depends on seam
  tick().then(() => {
    const row = document.querySelector(
      `[data-character-source="${ev.source}"][data-character-source-id="${CSS.escape(ev.sourceId)}"]`
    );
    row?.scrollIntoView({ behavior: 'smooth', block: 'center' });
    row?.classList.add('flash-target');
    setTimeout(() => row?.classList.remove('flash-target'), 1500);
  });
});
```

`CharacterRow.svelte` markup gains `data-character-source={...}` + `data-character-source-id={...}` attributes for the selector to land. `.flash-target` CSS animates a brief red outline pulse.

### §8.4 Tool-active edge cases

- **GM Screen already active**: skip the tool-switch step, jump straight to scroll.
- **Character not in GM Screen list** (filtered out, or not a Foundry character): scroll-target query returns null; banner click is a no-op visually (no toast). Future seam: surface "not found" if the empty result is surprising.
- **Tool registry doesn't expose a programmatic switch helper**: the plan's Step 1 verifies and adds one if missing — `Sidebar.svelte` already routes clicks to a writable store, so the helper is `currentTool.set('gm-screen')`. Trivial.

## §9 Plan packaging — single plan, three commit clusters

**Plan A — vital + discipline + banner nav** (single plan; three logical commits):

1. **Path resolver + tests** — Replace `readPath` with the §3.2 generalization. Add Rust-side serde test? No — it's TS-side. Without a frontend test runner (per ARCH §10), verification is via TypeScript signature consistency + manual validation of each path-coverage table row. Plan adds inline JSDoc tabular doc-comment showing the coverage matrix. Single commit.

2. **View 1 + View 3 annotations + ModifierEffectEditor autocomplete** — `CharacterCard.svelte` View 1 / View 3 markup gains delta annotations per §4 / §5; `canonical-names.ts` gains `FOUNDRY_VITAL_PATHS` + `foundryDisciplineNames`; `ModifierEffectEditor.svelte` includes them in autocomplete. Single combined commit (per `feedback_atomic_cluster_commits` — partial state shows half the deltas in some places not others, which is jarring; combined keeps render coherent). Manual verification: add a Stat effect with path `health.max` and see annotation on View 1.

3. **Banner navigation** — `toolEvents.ts` event variant; banner becomes interactive in `CharacterCard.svelte`; `GmScreen.svelte` subscribes; `CharacterRow.svelte` gains data attrs. Single commit.

Per CLAUDE.md hard rule: every commit ends `./scripts/verify.sh` green.

## §10 Verification

Per ARCH §10:

- Path-resolver coverage: each row of §3.3 verified manually with a constructed Stat or Pool effect at that path. No frontend test framework.
- View 1: with a `health.max +2` modifier, cycle through S/M/L densities and confirm box-row recompute scales correctly.
- View 3: confirm `system.disciplines.<name>` exists on the live Foundry sample (`docs/reference/foundry-vtm5e-actor-sample.json`); if the schema is `system.disciplines.<name>.value`, the resolver picks up the `.value` automatically. **Plan Step 1 verifies this against the sample file before §3.2 lands** — if disciplines schema is non-conforming, this spec amends.
- Banner navigation: click banner from card in Campaign tool → Tool switches → matching row scrolls into view → row gets a brief flash. Negative case: banner click on a card whose character isn't in GM Screen list → no-op (no error toast).
- `./scripts/verify.sh` green for each commit.

## §11 Anti-scope

- **No View 4 changes** (chips, advantages — frozen by card-redesign Plan B).
- **No new card tokens** (frozen by card-redesign Plan A).
- **No bridge / IPC / Rust changes** — pure frontend spec.
- **No per-power View 3 deltas** — Track 1.5 future seam.
- **No mock-data fixtures** — verification is against live Foundry actor.
- **No animation spec for the banner click flash** beyond a brief CSS transition.

## §12 Future seams

| Future feature | How this spec accommodates it |
|---|---|
| **Per-power View 3 deltas (Track 1.5)** | New `readItemPath()` helper added to `active-deltas.ts`; `computeActiveDeltas` extended to call it for `power.*` paths. Visual treatment new; no card-body restructure required. |
| **Roll20 vital deltas** | `readPath()` returns 0 for `char.source !== 'foundry'` today. A future spec wiring Roll20 attribute paths into the resolver fits cleanly — extend the `if (char.source === 'roll20')` branch to walk Roll20's `attributes[]` array shape. |
| **Cross-tool nav from elsewhere** | `navigate-to-character` is generic — any other tool gains the ability to dispatch this event with no GmScreen changes. |
| **Per-row badge on GM Screen** showing modifier-source character | The `data-character-source-id` attrs added in §8.3 make any character lookup trivial. |
| **Healing animation** when modifiers are toggled | Element keys are stable (path-keyed), so a `transition:slide` on the delta badge is a one-line addition. Out of scope here. |

## §13 Open questions

1. **Discipline schema verification**: §3.3 assumes `system.disciplines.<name>` exists with either `{ value: number }` or just `number`. Plan Step 1 must verify against `docs/reference/foundry-vtm5e-actor-sample.json` — if the schema differs (e.g., disciplines are an array, not a map), §3 amends. **No action this spec.**
2. **Negative-modified-value visual policy**: §4.3 says clamp render to zero with a tooltip explanation. The plan may want to clamp earlier (in `computeActiveDeltas` itself) to avoid renderers each implementing the clamp — but that conflates math with display. Default: renderer-side clamp.
3. **Discipline autocomplete vs. canonical name lookup**: The autocomplete derives from per-character data. If a Stat effect targets `disciplines.fortitude` on a character that doesn't have Fortitude, the resolver returns 0 (correct — `0 → +X`). The autocomplete won't suggest it because the actor lacks it. That's a real friction case — the GM may want to author "if this character had Fortitude…" pre-emptively. Default: live with it; raise as a future autocomplete-fallback if friction shows up.

## §14 Phase placement

Phase 2 (during-play tooling polish, per `2026-04-30-character-tooling-roadmap.md` §5). Goes on the GitHub Project board as **one feature-level parent issue**: "Card modifier coverage finish — View 1 + View 3 + banner navigation". Plan A is its only plan. No subtask issues unless a clear ~30-min checkpoint emerges (per `feedback_issue_granularity`).
