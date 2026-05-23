# Item Builder Shell Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to execute this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **PROJECT-SPECIFIC WORKFLOW OVERRIDES** (in addition to / overriding CLAUDE.md's standing lean-execution rules — see spec §"Workflow safeguards"):
>
> 1. **Per-task spec-compliance reviewer subagent is RE-ENABLED for this remediation only.** After each implementer commits, dispatch a spec-compliance reviewer subagent. Hand it the spec section citations from the task plus the implementer's diff. The reviewer's job is to confirm the change matches the design intent in the cited spec sections. (The per-task code-quality reviewer stays dropped — the final end-of-branch `code-review:code-review` handles that.)
> 2. **Visual smoke-test in `npm run tauri dev` is a gate before `./scripts/verify.sh`.** Every task that touches visible UI has a smoke-test step BEFORE the verify step BEFORE the commit step. If the implementer can't visually confirm the change matches the spec, the task is BLOCKED — they must escalate, not commit on faith.
> 3. **`./scripts/verify.sh` must run green before every commit.** Hard CLAUDE.md rule.
> 4. **Atomic combined commits where intermediate state breaks runtime.** A handful of tasks below intentionally combine multiple related changes into one commit for this reason.
> 5. **Spec section refs** in each task point at `docs/superpowers/specs/2026-05-23-item-builder-shell-redesign.md` by section name (stable across edits).

---

**Goal:** Replace the existing single-form ItemBuilderShell with the two-form architecture from the 2026-05-23 spec — Form #1 inline-edit cards for fast tweaks, Form #2 as a SvelteKit route for creation and deep restructuring.

**Architecture:** Front-end only. No backend changes — every field both forms produce already round-trips through the existing `addAdvantage` / `updateAdvantage` / `addStatusTemplate` / etc. IPC commands. The shell dialog component is deleted; its callers move to either inline-edit (Form #1, Advantage only) or a route navigation (Form #2, all entity types). A new `InlineEditCard` wraps `AdvantageCard` with an `editing: boolean` prop so visual changes propagate to both the library list AND the Form #2 card preview from one display component.

**Tech Stack:** Svelte 5 runes (`$state`, `$bindable`, `$effect`, `SvelteSet`), SvelteKit routes (`+page.svelte`), Tauri IPC via existing typed wrappers in `src/lib/**/api.ts`, vitest for the Latin-placeholder commit-semantics tests, W3C ARIA APG patterns for custom-widget keyboard maps.

---

## File structure

**New files:**

```
src/lib/components/
  InlineEditCard.svelte             — wraps AdvantageCard with edit-mode transformation; read↔edit toggle

src/routes/builder/
  advantage/
    new/+page.svelte                — Form #2 entry: create advantage
    [id]/+page.svelte               — Form #2 entry: edit advantage (optional ?stage= query)
  template/
    new/+page.svelte                — Form #2 entry: create status template
    [id]/+page.svelte               — Form #2 entry: edit status template
  modifier/
    [id]/+page.svelte               — Form #2 entry: edit per-character modifier
```

**Modified files:**

```
src/routes/+layout.svelte                              — add new CSS tokens (§"CSS tokens")
src/lib/components/AdvantageCard.svelte                — three-zone Witcher refactor, hover-reveal ✎ + 🗑
src/lib/components/gm-screen/StatusTemplateCard.svelte — three-zone refactor + hover-reveal 🗑 only
src/lib/components/builder/bodies/AdvantageBody.svelte — required:['name'], row order fix, Latin commit semantics, Delete button
src/lib/components/builder/CardComposer.svelte         — Form #2 footer Delete button + confirm dialog plumbing
src/lib/components/builder/ValueSelector.svelte        — APG slider keyboard map + ARIA
src/lib/components/builder/TagChipInput.svelte         — APG combobox+token-list keyboard map + ARIA
src/lib/components/builder/AppliesToPicker.svelte      — APG toolbar+listbox keyboard map + ARIA
src/lib/components/builder/TypePicker.svelte           — APG radio group keyboard map + ARIA
src/tools/AdvantagesManager.svelte                     — render via InlineEditCard; "+ Add Custom" routes to /builder/advantage/new
src/lib/components/gm-screen/StatusPaletteDock.svelte  — "+ New template" routes to /builder/template/new
src/lib/components/builder/bodies/ModifierEffectBody.svelte — edit affordance routes to /builder/modifier/[id]
src/lib/components/CharacterCard.svelte                — drop ItemBuilderShell import once Modifier route exists
src/lib/components/gm-screen/CharacterRow.svelte       — drop ItemBuilderShell import
src/routes/builder-test/+page.svelte                   — drop ItemBuilderShell test mount; replace with route-link smoke test
src/lib/builder/latin-placeholders.ts                  — (if needed) extend with commit/cancel semantics helpers
src/lib/builder/latin-placeholders.test.ts             — add Tab/Enter/Esc commit semantics tests
```

**Deleted files:**

```
src/lib/components/builder/ItemBuilderShell.svelte     — replaced by route + InlineEditCard
```

---

## Phase 0 — Foundation

### Task 1: CSS tokens

**Spec refs:** §"CSS tokens (added to `+layout.svelte`)"

**Files:**
- Modify: `src/routes/+layout.svelte` — add new tokens in `:root`

**Tests:** not required (token addition only).

- [ ] **Step 1: Read existing `:root` token block**

Run: `grep -n ':root\|--outline-color\|--bg-detail\|--validation-warn' src/routes/+layout.svelte`
Expected: identify the existing `:root { ... }` block so the new tokens land alongside it.

- [ ] **Step 2: Add the new tokens inside `:root`**

Insert (at the end of the existing `:root` block, before its closing brace):

```css
/* Card outline (Witcher-inspired) */
--outline-color: #83868a;
--shadow-color: 41deg 3% 15%;
--shadow-medium:
  0.3px 0.5px 0.5px hsl(var(--shadow-color) / 0.68),
  0.6px 1.2px 1.2px -1.3px hsl(var(--shadow-color) / 0.55),
  1.9px 3.9px 3.9px -2.7px hsl(var(--shadow-color) / 0.42),
  5.4px 10.8px 10.8px -4.1px hsl(var(--shadow-color) / 0.29);

/* Detail box (footer three-box) */
--bg-detail: #302f30;
--border-detail: #403d3d;
--hairline: rgba(255, 255, 255, 0.06);

/* WIP strip */
--wip: #c9a064;

/* Inline blur validation */
--validation-warn: #d9a560;
--validation-error: #c46050;
```

Do NOT re-declare any token that already exists. Use `grep -n` first to check each one.

- [ ] **Step 3: Visual smoke-test**

Run: `npm run tauri dev`
In a Tauri window: confirm the existing app loads with no visual regression (the new tokens are unused at this point, so nothing changes — the smoke test is "the app still boots and renders the library list").

- [ ] **Step 4: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: `verify: all checks passed`.

- [ ] **Step 5: Commit**

```bash
git add src/routes/+layout.svelte
git commit -m "$(cat <<'EOF'
feat(tokens): add Witcher-card CSS tokens for Item Builder redesign

Adds --outline-color, --shadow-color, --shadow-medium, --bg-detail,
--border-detail, --hairline, --wip, --validation-warn,
--validation-error to :root. Tokens are unused at this commit;
subsequent tasks consume them.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 1 — Card visuals (Form #1 read mode)

### Task 2: AdvantageCard three-zone Witcher refactor

**Spec refs:** §"Card visual design" (zones 1-3, typography stack), §"What this spec does NOT change" (kind text only, no hero / no per-kind background)

**Files:**
- Modify: `src/lib/components/AdvantageCard.svelte`

**Tests:** not required (visual refactor; `verify.sh`'s svelte-check + frontend build is the gate).

- [ ] **Step 1: Read the current AdvantageCard implementation**

Run: `wc -l src/lib/components/AdvantageCard.svelte` then read the file in full.
Expected: understand the existing props, slot structure, and `discriminated saved | preview` input shape from the original execution.

- [ ] **Step 2: Replace the card markup with the three-zone outlined-frame structure**

The container becomes a single `<article class="vamp-card">` with an outlined inner frame. No hero zone. Three child regions in order:

```svelte
<article class="vamp-card" data-kind={kind}>
  <div class="frame">
    <header class="zone-title">
      <h3>{name}</h3>
    </header>
    <section class="zone-content">
      <p class="description">{description}</p>
      <div class="tag-row">
        {#each tags as t}
          <span class="tag">{t}</span>
        {/each}
      </div>
      <div class="hairline" aria-hidden="true"></div>
    </section>
    <footer class="zone-footer">
      <div class="detail" data-field="type">
        <h6>TYPE</h6>
        <span class="value">{kindLabel(kind)}</span>
      </div>
      <div class="detail" data-field="level">
        <h6>LEVEL</h6>
        <span class="value tier-dots">
          {#each Array(5) as _, i}
            <span class="dot" class:filled={i < level} aria-hidden="true"></span>
          {/each}
          <span class="sr-only">{level} of 5</span>
        </span>
      </div>
      <div class="detail" data-field="bonus">
        <h6>BONUS</h6>
        <span class="value bonus">{bonusLabel(effects?.[0])}</span>
      </div>
    </footer>
  </div>
</article>
```

Add a `kindLabel(k)` helper inside the `<script>` block:

```ts
function kindLabel(k: AdvantageKind): string {
  return ({ merit: 'Merit', flaw: 'Flaw', background: 'Background', boon: 'Boon' } as const)[k] ?? 'Merit';
}
```

Add a `bonusLabel(e?: ModifierEffect): string` helper that returns `'—'` when no effect, otherwise renders e.g. `'+5 Social'` (delta + scope). Scope falls back to `'all'` if `paths` is empty.

- [ ] **Step 3: Replace the stylesheet with the spec's locked typography + layout**

```css
.vamp-card {
  position: relative;
  width: 100%;
  max-width: 22rem;
  min-width: 14rem;
  font-family: 'Iowan Old Style', Garamond, serif;
  color: var(--text-primary);
  box-shadow: var(--shadow-medium);
}

.frame {
  border: 1px solid var(--outline-color);
  position: relative;
  background: var(--bg-card, #1d1c1d);
  /* Outline+offset double-frame: the inner border is the offset */
}
.frame::before {
  content: "";
  position: absolute;
  inset: 4px;
  border: 1px solid var(--outline-color);
  pointer-events: none;
  opacity: 0.35;
}

.zone-title {
  padding: 0.85rem 1rem 0.5rem;
  text-align: center;
}
.zone-title h3 {
  margin: 0;
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 1.05rem;
  letter-spacing: 1.6px;
  line-height: 1.25;
  text-transform: uppercase;
}

.zone-content {
  padding: 0 1rem 0.55rem;
}
.zone-content .description {
  margin: 0 0 0.5rem;
  font-family: 'Oswald', sans-serif;
  font-weight: 200;
  font-size: 0.82rem;
  letter-spacing: 0.4px;
  line-height: 1.55;
  text-align: left;
}
.zone-content .tag-row {
  display: flex;
  justify-content: center;
  gap: 0.25rem;
  font-family: 'Spartan', sans-serif;
  font-size: 0.6rem;
  letter-spacing: 0.4px;
  color: var(--text-muted);
  text-transform: lowercase;
}
.zone-content .tag-row .tag::before { content: "· "; opacity: 0.5; }
.zone-content .tag-row .tag:first-child::before { content: ""; }
.zone-content .hairline {
  height: 1px;
  margin: 0.45rem 1rem;
  background: linear-gradient(to right, transparent, var(--hairline), transparent);
}

.zone-footer {
  display: flex;
  gap: 0.35rem;
  margin: 0 0.55rem 0.6rem;
  padding: 0 0.4rem;
}
.zone-footer .detail {
  flex: 1;
  background: var(--bg-detail);
  border: 1px solid var(--border-detail);
  padding: 0.45rem 0.4rem 0.5rem;
  text-align: center;
}
.zone-footer .detail h6 {
  margin: 0 0 0.25rem;
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 0.58rem;
  letter-spacing: 1.1px;
  text-transform: uppercase;
  color: var(--text-muted);
}
.zone-footer .detail .value {
  font-family: 'Oswald', sans-serif;
  font-weight: 200;
  font-size: 0.85rem;
  letter-spacing: 0.7px;
}
.zone-footer .detail[data-field="bonus"] .value {
  /* Spec §"Bonus magnitude treatment" */
  color: var(--blood-10);
  text-shadow: 0 0 4px rgba(224, 86, 58, 0.4);
  font-weight: 600;
  font-size: 0.95rem;
}
.zone-footer .detail[data-field="bonus"] .value:empty,
.zone-footer .detail[data-field="bonus"] .value.empty {
  color: var(--text-muted);
  opacity: 0.45;
  font-size: 0.85rem;
  text-shadow: none;
  font-weight: 200;
}

.tier-dots {
  display: inline-flex;
  gap: 0.18rem;
  align-items: center;
  justify-content: center;
}
.tier-dots .dot {
  width: 0.48rem;
  height: 0.48rem;
  border-radius: 50%;
  background: transparent;
  border: 1px solid var(--accent-amber);
  display: inline-block;
}
.tier-dots .dot.filled {
  background: var(--accent-amber);
  box-shadow: 0 0 3px rgba(217, 165, 96, 0.55);
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
```

Make sure no hex values are added outside the token block in `+layout.svelte`. The hex values above are all references to existing tokens (`var(--bg-detail)` etc.) except `rgba(217,165,96,0.55)` and `rgba(224,86,58,0.4)` which are decorative `rgba()` overlays explicitly carved out by CLAUDE.md.

- [ ] **Step 4: Verify svelte-check**

Run: `npm run check`
Expected: no new type errors. Pre-existing warnings (e.g. unused CSS selector hints) tolerated per ARCHITECTURE.md §10.

- [ ] **Step 5: Visual smoke-test in `npm run tauri dev`**

Run: `npm run tauri dev`
In the app: navigate to the Advantages tool. Confirm:
- Each advantage card renders with the outlined+offset frame (no hero/banner above the title).
- Title is uppercase, Spartan-light, letter-spaced.
- Description is Oswald-light.
- Footer shows three boxes: TYPE | LEVEL (tier dots) | BONUS.
- Bonus cell shows `'—'` muted when no effects.

- [ ] **Step 6: Run `./scripts/verify.sh`**

Run: `./scripts/verify.sh`
Expected: green.

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
refactor(card): Witcher three-zone AdvantageCard, no hero

Adopts the spec §"Card visual design" three-zone layout: title /
description+tags / three-box footer. Outline+offset double frame
via .frame::before. Spartan-light titles, Oswald-light description
and values, tier dots in the LEVEL detail, bonus magnitude in
glowing crimson when present.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: AdvantageCard hover-reveal ✎ + 🗑 affordances

**Spec refs:** §"Hover-reveal edit affordance (read mode)", §"Hover-reveal delete affordance (read mode)"

**Files:**
- Modify: `src/lib/components/AdvantageCard.svelte`

**Tests:** not required.

- [ ] **Step 1: Add the two icon elements inside `<article class="vamp-card">` (alongside the existing `<div class="frame">`)**

```svelte
<button
  type="button"
  class="affordance edit-icon"
  aria-label={`Edit ${name}`}
  onclick={() => onEditClick?.()}
>✎</button>
<button
  type="button"
  class="affordance delete-icon"
  aria-label={`Delete ${name}`}
  onclick={onDeleteClick}
>🗑</button>
```

Top-of-script, add props:

```ts
interface Props {
  // existing props ...
  onEditClick?: () => void;       // Form #1 entry — controller decides what "edit" means
  onDelete?: (id: number) => void;
}
let { /* existing */ onEditClick, onDelete }: Props = $props();

function onDeleteClick(e: Event) {
  e.stopPropagation();  // never let the click bubble up to a card-body click target
  const ok = confirm(`Delete ${name}?`);
  if (ok && id != null) onDelete?.(id);
}
```

The `confirm()` dialog is the v1 confirmation shape — the spec says `"Delete <name>?"` with Cancel/Delete buttons; `window.confirm` matches that semantics without needing a custom dialog. (A custom amber-themed dialog is v1.x polish.)

- [ ] **Step 2: Style the affordance buttons**

Append to the existing `<style>`:

```css
.affordance {
  position: absolute;
  background: transparent;
  border: 0;
  padding: 0;
  cursor: pointer;
  color: var(--text-muted);
  font-size: 0.85rem;
  line-height: 1;
  opacity: 0;
  transition: opacity 120ms ease-out;
  z-index: 2;
}
.vamp-card:hover .edit-icon { opacity: 0.7; }
.vamp-card:hover .delete-icon { opacity: 0.6; }
.affordance:focus-visible {
  opacity: 1;
  outline: 1px solid var(--accent-amber);
  outline-offset: 2px;
}
.edit-icon   { top: 0.5rem;    right: 0.5rem; }
.delete-icon { bottom: 0.5rem; right: 0.5rem; }
.delete-icon:hover { color: var(--accent-red); }
```

Note: ✎ at top-right; 🗑 at bottom-right. Different corners by design (spec § "Hover-reveal delete affordance" — Fitts's Law mis-click defense).

- [ ] **Step 3: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 4: Visual smoke-test**

Run: `npm run tauri dev`
In the Advantages tool, hover an advantage card. Confirm:
- ✎ icon fades in at top-right (~0.7 opacity).
- 🗑 icon fades in at bottom-right (~0.6 opacity).
- Both disappear on mouse-leave.
- Clicking 🗑 opens a native confirm dialog: `"Delete <name>?"`. Cancel keeps the card.
- Tab-key focus on either icon shows a visible amber outline ring (focus-visible).

Do NOT yet test "Delete" confirms — `onDelete` callers aren't wired until Task 8. For now, clicking 🗑 + confirming is a no-op (or you can wire a temporary `console.log` for the smoke test; remove before commit).

- [ ] **Step 5: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/AdvantageCard.svelte
git commit -m "$(cat <<'EOF'
feat(card): hover-reveal edit + delete affordances on AdvantageCard

✎ icon (top-right, opacity 0→0.7) and 🗑 icon (bottom-right,
0→0.6) fade in on .vamp-card:hover. Different corners by design
(Fitts's Law mis-click defense). Delete opens a native confirm
dialog; the actual deletion is wired in Task 8 when callers
migrate to InlineEditCard.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: StatusTemplateCard three-zone refactor + hover-🗑

**Spec refs:** §"Card visual design" (three-zone layout applies to all card types), §"Hover-reveal delete affordance" (applies to all card types; ✎ does NOT — StatusTemplateCard has no inline edit in v1)

**Files:**
- Modify: `src/lib/components/gm-screen/StatusTemplateCard.svelte`

**Tests:** not required.

- [ ] **Step 1: Read the current implementation**

Run: `wc -l src/lib/components/gm-screen/StatusTemplateCard.svelte` and read.
Expected: catalog the existing props and which sections need to remain.

- [ ] **Step 2: Restructure to the three-zone outlined frame**

Mirror the AdvantageCard structure from Task 2, but with StatusTemplate-specific footer fields. StatusTemplate has no `kind` or `level`; the footer surfaces:

```svelte
<footer class="zone-footer">
  <div class="detail" data-field="effects">
    <h6>EFFECTS</h6>
    <span class="value">{effectCount} effect{effectCount === 1 ? '' : 's'}</span>
  </div>
  <div class="detail" data-field="tags">
    <h6>TAGS</h6>
    <span class="value">{tags.length || '—'}</span>
  </div>
</footer>
```

Two boxes (not three), `flex: 1` each. Outline + offset and typography rules identical to AdvantageCard (copy the relevant CSS, no shared file unless one already exists).

- [ ] **Step 3: Add ONLY the hover-🗑 affordance (no ✎)**

Inside `<article>`:

```svelte
<button
  type="button"
  class="affordance delete-icon"
  aria-label={`Delete ${name}`}
  onclick={onDeleteClick}
>🗑</button>
```

```ts
interface Props {
  // existing props ...
  onDelete?: (id: number) => void;
}
function onDeleteClick(e: Event) {
  e.stopPropagation();
  const ok = confirm(`Delete ${name}?`);
  if (ok && id != null) onDelete?.(id);
}
```

Style block (same as AdvantageCard's `.affordance`, `.delete-icon`, hover/focus rules — copy them).

- [ ] **Step 4: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 5: Visual smoke-test**

Run: `npm run tauri dev`
Open the GM Screen. Confirm:
- Status template cards in the palette render with outlined+offset three-zone (no hero, two-box footer).
- Hover reveals 🗑 at bottom-right; no ✎.
- Delete confirm dialog opens; callers will wire actual delete in Task 8.

- [ ] **Step 6: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/gm-screen/StatusTemplateCard.svelte
git commit -m "$(cat <<'EOF'
refactor(card): Witcher three-zone StatusTemplateCard + hover-🗑

Three-zone outlined-frame layout matching AdvantageCard, with a
two-box footer (EFFECTS · TAGS) instead of three. Hover-reveal 🗑
at bottom-right; no ✎ — inline edit for StatusTemplate is deferred
to v2 per spec §"Form #1 — Inline-edit card · Scope (v1)".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2 — Body refactor + Form #2 routes

### Task 5: AdvantageBody — required:['name'], row order fix, Latin placeholder commit semantics

**Spec refs:** §"Data model · Advantage" (required:['name']; level default 0), §"Card preview (zone 2) · Spatial layout" (Description row 3, Bonus row 4), §"Custom widget keyboard support · Latin placeholder commit semantics (name stage)"

**Files:**
- Modify: `src/lib/components/builder/bodies/AdvantageBody.svelte`
- Modify: `src/lib/builder/latin-placeholders.ts` — add `commitPlaceholder()` helper (if not already idiomatic in component-side code)
- Modify: `src/lib/builder/latin-placeholders.test.ts` — add commit-semantics tests

**Tests:** REQUIRED — new behavior (Tab/Enter commits placeholder; Esc clears; typing displaces) is logic, not visual.

- [ ] **Step 1: Read current AdvantageBody and tests**

Run: `wc -l src/lib/components/builder/bodies/AdvantageBody.svelte src/lib/builder/latin-placeholders.ts src/lib/builder/latin-placeholders.test.ts` then read each.
Expected: locate the existing `createStageMachine({ ..., required: ['name', 'bonus'] })` call (line ~91 today) and the current `emptyState()` level default (currently `1`).

- [ ] **Step 2: Write the failing Latin commit semantics tests first**

Append to `src/lib/builder/latin-placeholders.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { commitPlaceholder } from './latin-placeholders';

describe('commitPlaceholder', () => {
  it('returns the placeholder when the typed value is empty', () => {
    expect(commitPlaceholder('', 'Sanguine Tenacity')).toBe('Sanguine Tenacity');
  });

  it('returns the typed value when non-empty (placeholder displaced)', () => {
    expect(commitPlaceholder('User Name', 'Sanguine Tenacity')).toBe('User Name');
  });

  it('treats whitespace-only typed value as empty (commit placeholder)', () => {
    expect(commitPlaceholder('   ', 'Sanguine Tenacity')).toBe('Sanguine Tenacity');
  });

  it('returns empty string when the user explicitly clears (Esc semantics handled elsewhere; this is just the commit helper)', () => {
    // commitPlaceholder is not responsible for Esc — see component handler
    expect(commitPlaceholder('', '')).toBe('');
  });
});
```

- [ ] **Step 3: Run tests to confirm they fail**

Run: `npm test -- src/lib/builder/latin-placeholders.test.ts`
Expected: FAIL — `commitPlaceholder` not exported.

- [ ] **Step 4: Implement `commitPlaceholder` in `latin-placeholders.ts`**

Append:

```ts
/**
 * Latin placeholder commit semantics — see spec §"Latin placeholder commit
 * semantics (name stage)". Returns the typed value when non-empty; returns
 * the placeholder when the user hasn't typed anything (placeholder behaves
 * as accepted suggestion). Whitespace-only typed value is treated as empty.
 */
export function commitPlaceholder(typed: string, placeholder: string): string {
  return typed.trim() === '' ? placeholder : typed;
}
```

- [ ] **Step 5: Run tests to confirm they pass**

Run: `npm test -- src/lib/builder/latin-placeholders.test.ts`
Expected: PASS — all 4 new tests.

- [ ] **Step 6: Update `AdvantageBody.svelte` — required-set, level default, row order, name input semantics**

In the file:

(a) Change the `createStageMachine` call:

```ts
const machine = createStageMachine({
  tabOrder: ['name', 'type', 'level', 'bonus', 'description', 'tags'],
  required: ['name'],
  initialConfirmed: initial?.confirmedFields ?? undefined,
});
```

(b) In `emptyState()`, change the level default. The current expression resolves to `1` when no value is found; change it to `0`:

```ts
level: Number(
  (init?.properties.find(p => p.name === 'dot_cost_per_level' && p.type === 'number')?.value) ?? 0
),
```

(c) In the `<svelte:fragment>` for the name stage spotlight (or wherever the name input is rendered — look for `bodyState.name`), wire the Latin commit semantics:

```svelte
<script lang="ts">
  import { pickPlaceholder, commitPlaceholder } from '$lib/builder/latin-placeholders';
  // ...
  const placeholder = pickPlaceholder(existingCount);

  function onNameKeydown(e: KeyboardEvent) {
    if (e.key === 'Tab' || e.key === 'Enter') {
      // Commit semantics: empty typed → placeholder; otherwise → typed
      if (bodyState && bodyState.name.trim() === '') {
        bodyState.name = commitPlaceholder(bodyState.name, placeholder);
      }
      // Allow default Tab handling (focus shift); Enter advances stage
      if (e.key === 'Enter') {
        e.preventDefault();
        machine.advance();
      }
    } else if (e.key === 'Escape') {
      // Esc rejects the placeholder and clears the field
      if (bodyState) bodyState.name = '';
      e.preventDefault();
    }
  }
</script>

<input
  type="text"
  bind:value={bodyState.name}
  placeholder={placeholder}
  onkeydown={onNameKeydown}
  aria-label="Advantage name"
/>
```

The visual treatment of "placeholder as suggestion" relies on the native `placeholder` attribute styling (italic, opacity ~0.55 from base theme) plus the typed-value being empty. No special styling needed if the project's existing placeholder CSS matches the spec; if not, add:

```css
input::placeholder { font-style: italic; opacity: 0.55; }
```

(d) Fix the card-preview row order so Description = row 3 and Bonus = row 4. Locate the existing `<CardComposer>` slot grid or `cardRows` array. The current order may have Bonus before Description (the D1 deviation). The correct order per spec §"Card preview · Spatial layout":

```
Row 1 (col-2):  name      | type
Row 2 (col-2):  level     | category
Row 3 (col-1):  description
Row 4 (col-1):  bonus
Row 5 (col-1):  tags
```

Adjust the array literal to match. (Read the existing array first; the change is just reordering two entries.)

- [ ] **Step 7: Verify svelte-check + run all tests**

Run: `npm run check` then `npm test`
Expected: no new type errors; all tests pass (existing + 4 new).

- [ ] **Step 8: Visual smoke-test**

Run: `npm run tauri dev`
In the Advantages tool, click `+ Add Custom` (it still opens the old shell — the route migration happens in Task 8; in the meantime the shell embeds AdvantageBody, so the changes are testable):
- Walk to the `name` stage. The Latin placeholder shows in the input.
- Press `Tab` with the input empty: name becomes the placeholder; stage advances.
- Press `Esc` while a typed value is present: field clears (placeholder re-shows).
- Press `Tab` after typing "Hello": commits "Hello", stage advances.
- Walk forward to confirm the card preview rows order: Description above Bonus.
- Save with just `name` filled (defaults for kind=merit, level=0). The save should succeed (validity ⇒ `isComplete()` true now that required=['name']).
- Open an existing advantage from the library and confirm level shows 0 dots when previously unset.

- [ ] **Step 9: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 10: Commit**

```bash
git add src/lib/components/builder/bodies/AdvantageBody.svelte src/lib/builder/latin-placeholders.ts src/lib/builder/latin-placeholders.test.ts
git commit -m "$(cat <<'EOF'
feat(builder): AdvantageBody required:['name'], row order, Latin Tab semantics

- Stage machine required-set is now ['name'] only. Type defaults
  to 'merit', level defaults to 0; both pass through without user
  interaction needed.
- Card-preview row order matches spec §"Card preview · Spatial
  layout": Description on row 3, Bonus on row 4.
- Latin placeholder commit semantics on the name input — Tab/Enter
  with empty typed value commits the placeholder, Esc clears, any
  typed character displaces the placeholder.
- New commitPlaceholder() helper in latin-placeholders.ts with
  vitest coverage for the commit/displace/whitespace edge cases.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: `/builder/advantage/{new,[id]}` routes

**Spec refs:** §"Two-form structure · Form #2 · Entry points", §"Form #2 zone specifications" (page layout — composer column centered, ~36 rem cap)

**Files:**
- Create: `src/routes/builder/advantage/new/+page.svelte`
- Create: `src/routes/builder/advantage/[id]/+page.svelte`

**Tests:** not required (route plumbing; smoke-test gate).

- [ ] **Step 1: Create `src/routes/builder/advantage/new/+page.svelte`**

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { addAdvantage, listAdvantages, type AdvantageInput } from '$lib/advantages/api';
  import CardComposer from '$lib/components/builder/CardComposer.svelte';
  import AdvantageBody from '$lib/components/builder/bodies/AdvantageBody.svelte';
  import type { AdvantageBodyState } from '$lib/components/builder/bodies/AdvantageBody.svelte';
  import type { Advantage } from '../../../../types';

  let bodyState = $state<AdvantageBodyState | undefined>(undefined);
  let valid = $state(false);
  let existingCount = $state(0);
  let busy = $state(false);

  // Load existing count for placeholder rotation
  $effect(() => {
    void (async () => {
      try { existingCount = (await listAdvantages()).length; } catch { existingCount = 0; }
    })();
  });

  async function save() {
    if (!bodyState || busy) return;
    busy = true;
    try {
      const input = toAdvantageInput(bodyState, valid);
      const created = await addAdvantage(input);
      await goto('/');  // return to library list — wired-up entry sets the actual landing
    } finally {
      busy = false;
    }
  }

  function toAdvantageInput(s: AdvantageBodyState, isValid: boolean): AdvantageInput {
    return {
      name: s.name,
      kind: s.kind,
      description: s.description,
      tags: [...s.tags],
      properties: [{ name: 'dot_cost_per_level', type: 'number', value: s.level }],
      effects: [
        { kind: s.bonus.kind, bonusName: s.bonus.bonusName, delta: s.bonus.value, paths: [...s.bonus.paths] },
        ...s.extraEffects,
      ],
      wip: !isValid,
      confirmedFields: s.confirmedFields,
    } as AdvantageInput;
  }

  function cancel() { void goto('/'); }
</script>

<svelte:head><title>New Advantage · vtmtools</title></svelte:head>

<main class="builder-page">
  <header class="title-bar">
    <span class="step-indicator">STEP {currentStepNum()} OF {totalSteps()}</span>
    <span class="subtitle">· new merit</span>
    <button type="button" class="close" aria-label="Close" onclick={cancel}>×</button>
  </header>
  <CardComposer let:slots>
    <AdvantageBody bind:bodyState bind:valid {existingCount} />
  </CardComposer>
  <footer class="composer-footer">
    <button type="button" onclick={cancel}>Cancel</button>
    <button type="button" class="save" onclick={save} disabled={busy}>
      {valid ? 'Save' : 'Save WIP'}
    </button>
  </footer>
</main>
```

Two helper functions `currentStepNum()` / `totalSteps()` can be stub'd to `1` / `6` for now — the live step indicator is driven by the body's stage machine but reading it from outside the body needs a `bind:` exposure that's covered later. For Task 6, a static placeholder is fine; the indicator becomes reactive when the body exposes `machine` via `$bindable` in a follow-up.

If `AdvantageBodyState` is not exported by AdvantageBody, expose it via `<script context="module">` in AdvantageBody — but check first whether it's already importable as a type (likely yes from the existing implementation).

- [ ] **Step 2: Create `src/routes/builder/advantage/[id]/+page.svelte`**

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { listAdvantages, updateAdvantage, deleteAdvantage, type AdvantageInput } from '$lib/advantages/api';
  import CardComposer from '$lib/components/builder/CardComposer.svelte';
  import AdvantageBody from '$lib/components/builder/bodies/AdvantageBody.svelte';
  import type { AdvantageBodyState } from '$lib/components/builder/bodies/AdvantageBody.svelte';
  import type { Advantage } from '../../../../types';

  const id = $derived(Number(page.params.id));
  const stage = $derived(page.url.searchParams.get('stage') ?? null);

  let initial = $state<Advantage | undefined>(undefined);
  let bodyState = $state<AdvantageBodyState | undefined>(undefined);
  let valid = $state(false);
  let busy = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const all = await listAdvantages();
        initial = all.find(a => a.id === id);
        if (!initial) loadError = `Advantage ${id} not found`;
      } catch (e) { loadError = String(e); }
    })();
  });

  async function save() {
    if (!bodyState || !initial || busy) return;
    busy = true;
    try {
      await updateAdvantage(id, toAdvantageInput(bodyState, valid));
      await goto('/');
    } finally { busy = false; }
  }

  async function onDelete() {
    if (!initial || busy) return;
    if (!confirm(`Delete ${initial.name}?`)) return;
    busy = true;
    try {
      await deleteAdvantage(id);
      await goto('/');
    } finally { busy = false; }
  }

  function cancel() { void goto('/'); }

  function toAdvantageInput(s: AdvantageBodyState, isValid: boolean): AdvantageInput {
    return {
      name: s.name,
      kind: s.kind,
      description: s.description,
      tags: [...s.tags],
      properties: [{ name: 'dot_cost_per_level', type: 'number', value: s.level }],
      effects: [
        { kind: s.bonus.kind, bonusName: s.bonus.bonusName, delta: s.bonus.value, paths: [...s.bonus.paths] },
        ...s.extraEffects,
      ],
      wip: !isValid,
      confirmedFields: s.confirmedFields,
    } as AdvantageInput;
  }
</script>

<svelte:head><title>Edit Advantage · vtmtools</title></svelte:head>

{#if loadError}
  <main class="builder-page error">
    <p>{loadError}</p>
    <button type="button" onclick={cancel}>Back</button>
  </main>
{:else if initial}
  <main class="builder-page">
    <header class="title-bar">
      <span class="step-indicator">EDIT</span>
      <span class="subtitle">· {initial.name}</span>
      <button type="button" class="close" aria-label="Close" onclick={cancel}>×</button>
    </header>
    <CardComposer let:slots>
      <AdvantageBody {initial} bind:bodyState bind:valid />
    </CardComposer>
    <footer class="composer-footer">
      <button type="button" class="delete" onclick={onDelete}>Delete</button>
      <div class="spacer"></div>
      <button type="button" onclick={cancel}>Cancel</button>
      <button type="button" class="save" onclick={save} disabled={busy}>
        {valid ? 'Save' : 'Save WIP'}
      </button>
    </footer>
  </main>
{:else}
  <main class="builder-page loading"><p>Loading…</p></main>
{/if}
```

The `?stage=` query param is read but not yet consumed; Task 12 (Form #1 hand-off) sets it. For Task 6, just accept it without acting on it — leave a `// TODO Task 12: seed machine.stage from query` comment so the next implementer knows. Use a one-line comment, not a multi-line block, per CLAUDE.md.

Edit: per CLAUDE.md "default to writing no comments" — drop the TODO. The reader will discover the gap when they implement Task 12 against the spec.

- [ ] **Step 3: Shared composer-page styles**

Both files share visual chrome. Co-locate styles in each file (DRY-violating in two route files is fine — the rules are stable; abstracting prematurely is YAGNI). Each file's `<style>` block:

```css
.builder-page {
  width: min(36rem, 100% - 2rem);
  margin: 0 auto;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 1rem 0;
}
.title-bar {
  display: flex;
  align-items: center;
  padding: 0.45rem 0.75rem;
  background: rgba(0, 0, 0, 0.3);
  border-bottom: 1px solid var(--hairline);
}
.step-indicator {
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 0.56rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  color: var(--accent-amber);
}
.subtitle {
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 0.56rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-left: 0.25rem;
}
.close {
  margin-left: auto;
  background: transparent;
  border: 0;
  color: var(--text-muted);
  font-size: 1.2rem;
  cursor: pointer;
  padding: 0 0.25rem;
}
.composer-footer {
  display: flex;
  gap: 0.4rem;
  align-items: center;
  padding: 0.5rem 0.75rem;
  background: rgba(0, 0, 0, 0.25);
}
.composer-footer .spacer { flex: 1; }
.composer-footer .delete { /* muted */ color: var(--text-muted); }
.composer-footer .save { background: var(--accent-red); color: var(--text-primary); }
```

- [ ] **Step 4: Verify svelte-check**

Run: `npm run check`
Expected: no new type errors. SvelteKit will pick up the new routes automatically.

- [ ] **Step 5: Visual smoke-test**

Run: `npm run tauri dev`
Manually navigate to `/builder/advantage/new` in the address bar (or `Ctrl+L` / dev tools console: `window.location = '/builder/advantage/new'`). Confirm:
- The page renders with title bar, card composer, queue, and footer.
- Footer shows `Cancel` and `Save WIP` (no `Delete` — this is the new route, no `id`).
- `Cancel` returns to `/`.
- Save creates a new advantage (verify in the library list).

Then navigate to `/builder/advantage/<some-existing-id>`. Confirm:
- The page loads the existing advantage's data into the composer.
- Footer shows `Delete` (left), `Cancel`, `Save`.
- `Delete` opens confirm dialog; cancelling is a no-op; confirming removes the advantage.

- [ ] **Step 6: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 7: Commit**

```bash
git add src/routes/builder/advantage/
git commit -m "$(cat <<'EOF'
feat(routes): /builder/advantage/{new,[id]} for Form #2

Form #2 as a SvelteKit route, NOT a dialog modal — per spec
§"Two-form structure · Form #2". The /new route creates a new
advantage; /[id] loads an existing one. Reads ?stage= query param
for stage deep-link from Form #1 hand-off (consumed in Task 12).
Footer includes Delete (edit mode only) per spec §"Footer (zone 4
bottom half)".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: `/builder/template/{new,[id]}` + `/builder/modifier/[id]` routes

**Spec refs:** §"Two-form structure · Form #2 · Entry points" (entries 2 & 4)

**Files:**
- Create: `src/routes/builder/template/new/+page.svelte`
- Create: `src/routes/builder/template/[id]/+page.svelte`
- Create: `src/routes/builder/modifier/[id]/+page.svelte`

**Tests:** not required.

- [ ] **Step 1: Create `src/routes/builder/template/new/+page.svelte`**

Mirror the structure of `src/routes/builder/advantage/new/+page.svelte` from Task 6, swapping:
- `addAdvantage` → `addStatusTemplate` (from `$lib/modifiers/api`)
- `AdvantageBody` → `StatusTemplateBody`
- Title `'new merit'` → `'new template'`
- `toAdvantageInput` → `toStatusTemplateInput` (using the StatusTemplate input shape)

Implementer: read `StatusTemplateBody.svelte` to learn its `bodyState` shape and props. Mirror them in the route's `toStatusTemplateInput()` helper. The relevant `NewStatusTemplateInput` type lives in `src/lib/modifiers/api.ts`.

- [ ] **Step 2: Create `src/routes/builder/template/[id]/+page.svelte`**

Mirror `src/routes/builder/advantage/[id]/+page.svelte` from Task 6, swapping:
- `listAdvantages` → `listStatusTemplates`
- `updateAdvantage` → `updateStatusTemplate`
- `deleteAdvantage` → `deleteStatusTemplate`
- `AdvantageBody` → `StatusTemplateBody`

- [ ] **Step 3: Create `src/routes/builder/modifier/[id]/+page.svelte`**

The Modifier route is structurally similar but the body is `ModifierEffectBody` and the data path is per-character (modifier is bound to a character). Read `ModifierEffectBody.svelte` first to learn its bodyState shape and how it gets the character context — likely a `characterId` prop or store.

Skeleton:

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { listAllCharacterModifiers, updateCharacterModifier, deleteCharacterModifier } from '$lib/modifiers/api';
  import CardComposer from '$lib/components/builder/CardComposer.svelte';
  import ModifierEffectBody from '$lib/components/builder/bodies/ModifierEffectBody.svelte';
  import type { CharacterModifier } from '../../../../types';

  const id = $derived(Number(page.params.id));
  let initial = $state<CharacterModifier | undefined>(undefined);
  let bodyState = $state<unknown>(undefined);  // ModifierEffectBody's state shape; refine after reading the body
  let valid = $state(false);
  let busy = $state(false);
  let loadError = $state<string | null>(null);

  $effect(() => {
    void (async () => {
      try {
        const all = await listAllCharacterModifiers();
        initial = all.find(m => m.id === id);
        if (!initial) loadError = `Modifier ${id} not found`;
      } catch (e) { loadError = String(e); }
    })();
  });

  async function save() { /* mirror Advantage [id] save */ }
  async function onDelete() {
    if (!initial || busy) return;
    if (!confirm(`Delete this modifier?`)) return;
    busy = true;
    try { await deleteCharacterModifier(id); await goto('/'); } finally { busy = false; }
  }
  function cancel() { void goto('/'); }
</script>

<!-- markup mirrors Advantage [id] page; substitute ModifierEffectBody for AdvantageBody -->
```

The exact `bodyState` shape and the `updateCharacterModifier` input shape need to match what ModifierEffectBody emits — read both before completing the skeleton.

- [ ] **Step 4: Shared styles**

Each new route file co-locates the same `.builder-page` / `.title-bar` / `.composer-footer` styles from Task 6. (Three more file-local copies. DRY can be revisited in v1.x; for v1 the duplication keeps each file standalone and reviewable.)

- [ ] **Step 5: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 6: Visual smoke-test**

Run: `npm run tauri dev`
- Navigate to `/builder/template/new`. Confirm a status template can be created and saved.
- Navigate to `/builder/template/<existing-id>`. Confirm load + save + delete.
- Navigate to `/builder/modifier/<existing-modifier-id>`. Confirm the modifier loads. (If no per-character modifier exists yet, create one via the existing UI first, then test.)

- [ ] **Step 7: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 8: Commit**

```bash
git add src/routes/builder/template/ src/routes/builder/modifier/
git commit -m "$(cat <<'EOF'
feat(routes): /builder/template/* and /builder/modifier/* for Form #2

Status template and per-character modifier routes. Both follow the
same shape as /builder/advantage — title bar, card composer,
queue, footer with Delete (edit mode only). Modifier route mounts
ModifierEffectBody; Template mounts StatusTemplateBody.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Migrate callers — AdvantagesManager + GM Screen + ModifierEffectBody — to routes; wire delete on cards

**Spec refs:** §"Two-form structure · Form #2 · Entry points" (entries 1, 2, 4), §"Hover-reveal delete affordance" (callers must wire `onDelete`)

**Files:**
- Modify: `src/tools/AdvantagesManager.svelte`
- Modify: `src/lib/components/gm-screen/StatusPaletteDock.svelte`
- Modify: `src/lib/components/builder/bodies/ModifierEffectBody.svelte` (its edit affordance for managed cards)

**Tests:** not required.

- [ ] **Step 1: Read current AdvantagesManager**

Run: `grep -n 'ItemBuilderShell\|+ Add Custom\|AdvantageCard' src/tools/AdvantagesManager.svelte` then read the relevant sections.

- [ ] **Step 2: Replace "+ Add Custom" with route navigation**

Find the "+ Add Custom" button. Replace its `onclick={() => openShell()}` (or similar) with a SvelteKit `<a href="/builder/advantage/new" class="add-custom">+ Add Custom</a>` — OR keep the button but make `onclick` call `goto('/builder/advantage/new')`.

Either form is fine — match what the rest of the file uses.

Wire `onDelete` on every `<AdvantageCard>` instance:

```svelte
{#each advantages as a}
  <AdvantageCard
    saved={a}
    onDelete={async (id) => { await deleteAdvantage(id); advantages = advantages.filter(x => x.id !== id); }}
  />
{/each}
```

Note that the `confirm()` dialog already lives in the card; the caller only needs to wire the actual delete-and-refresh.

- [ ] **Step 3: Remove the ItemBuilderShell import + render block (if it's at this level)**

If `AdvantagesManager` directly imports and mounts `<ItemBuilderShell>`, delete the import line, the `bind:open` state, and the `<ItemBuilderShell …>` mount. The shell is replaced by the route; nothing in this component needs it any more. (Task 18 will physically delete the file; for now just stop referencing it.)

- [ ] **Step 4: Repeat for StatusPaletteDock**

In `src/lib/components/gm-screen/StatusPaletteDock.svelte`:
- "+ New template" button → `goto('/builder/template/new')`.
- Each `<StatusTemplateCard>` gets an `onDelete={async (id) => { await deleteStatusTemplate(id); … }}` prop.
- Drop any `ItemBuilderShell` import + mount.
- Each existing template card's "edit" affordance (if any) should route to `/builder/template/[id]`. Read the current code to find the affordance.

- [ ] **Step 5: Repeat for ModifierEffectBody callers**

ModifierEffectBody is itself a body (renders inside the Modifier builder route). It's NOT a caller of the shell in the same way; but if `ModifierEffectBody` is used elsewhere as part of an in-place editor (e.g. `CharacterRow` or `CharacterCard`), those callers should:
- For "edit", route to `/builder/modifier/[id]`.
- For "delete", call `deleteCharacterModifier`.

Read `CharacterRow.svelte` and `CharacterCard.svelte` to identify where ModifierEffectBody / ItemBuilderShell are mounted today, and migrate each call site.

- [ ] **Step 6: Verify svelte-check**

Run: `npm run check`
Expected: no new errors. If you see "Cannot find module ItemBuilderShell", that's the success signal — we want to confirm no caller still references it.

Run: `grep -rn 'ItemBuilderShell' src/` — expected: only `src/lib/components/builder/ItemBuilderShell.svelte` itself remains (and possibly `src/routes/builder-test/+page.svelte`, which Task 18 will handle). If any other source file still references it, fix that file in this task before committing.

- [ ] **Step 7: Visual smoke-test**

Run: `npm run tauri dev`
- Advantages tool: `+ Add Custom` navigates to `/builder/advantage/new`. Clicking 🗑 on any card prompts to delete; confirming removes it from the list.
- GM Screen status palette: `+ New template` navigates to `/builder/template/new`. 🗑 on a template card deletes it.
- Character card / Character row: any edit-modifier affordance navigates to `/builder/modifier/[id]`.

- [ ] **Step 8: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 9: Commit**

```bash
git add src/tools/AdvantagesManager.svelte src/lib/components/gm-screen/StatusPaletteDock.svelte src/lib/components/builder/bodies/ModifierEffectBody.svelte src/lib/components/CharacterCard.svelte src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
refactor(callers): route to /builder/* and wire onDelete on cards

AdvantagesManager: "+ Add Custom" → /builder/advantage/new;
AdvantageCard onDelete wired to deleteAdvantage + list refresh.
StatusPaletteDock: "+ New template" → /builder/template/new;
StatusTemplateCard onDelete wired to deleteStatusTemplate.
ModifierEffectBody / CharacterRow / CharacterCard: edit affordance
routes to /builder/modifier/[id]. All ItemBuilderShell mounts at
these call sites are removed.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Form #2 footer Delete button — plumb through `CardComposer`

**Spec refs:** §"Footer (zone 4 bottom half)" (Delete on the left, edit mode only)

**Files:**
- Modify: `src/lib/components/builder/CardComposer.svelte` — accept an optional `onDelete` prop and surface it in the footer slot
- (Routes already render the Delete button outside CardComposer per Task 6; this task verifies the prop flow is consistent and adds any missing markup if CardComposer was rendering its own footer)

**Tests:** not required.

- [ ] **Step 1: Audit CardComposer**

Run: `wc -l src/lib/components/builder/CardComposer.svelte` then read.
Expected: 84 lines. Decide whether the footer markup lives in CardComposer (in which case the Delete button must be added inside it and the routes simplify) OR in each route (which is what Task 6 did). If both render footers, that's a bug — pick one.

Preferred: the **route** owns the footer markup (Task 6's pattern). CardComposer just lays out zones 2-3 (preview + spotlight + queue). If CardComposer currently renders zone 4 too, factor it out so the routes own the footer.

- [ ] **Step 2: Move zone 4 ownership to the route (if needed)**

If CardComposer currently includes a `<footer>` block, remove it from CardComposer; the route templates from Task 6 already include the correct footer. The composer becomes responsible for zones 2-3 only.

- [ ] **Step 3: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 4: Visual smoke-test**

Run: `npm run tauri dev`
- Open `/builder/advantage/new` — confirm exactly ONE footer is visible (no duplicate buttons).
- Open `/builder/advantage/[id]` — confirm Delete button is visible on the left and works.
- Open `/builder/advantage/new` — confirm Delete button is NOT visible (no id to delete).

- [ ] **Step 5: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/builder/CardComposer.svelte
git commit -m "$(cat <<'EOF'
refactor(builder): CardComposer owns zones 2-3, routes own zone 4

Footer (zone 4) markup belongs to the route — the route knows
whether we're creating or editing and shows/hides Delete
accordingly. CardComposer is now exclusively responsible for the
card preview + spotlight + queue (zones 2-3).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3 — Form #1 (InlineEditCard)

### Task 10: `InlineEditCard.svelte` wrapper + edit-mode field controls

**Spec refs:** §"Two-form structure · Form #1 — Inline-edit card", §"Component map · InlineEditCard ↔ AdvantageCard"

**Files:**
- Create: `src/lib/components/InlineEditCard.svelte`

**Tests:** not required.

- [ ] **Step 1: Skeleton the wrapper**

```svelte
<script lang="ts">
  import AdvantageCard from './AdvantageCard.svelte';
  import TagChipInput from './builder/TagChipInput.svelte';
  import { goto } from '$app/navigation';
  import { updateAdvantage, type AdvantageInput } from '$lib/advantages/api';
  import type { Advantage, AdvantageKind, ModifierEffect } from '../../types';

  interface Props {
    advantage: Advantage;
    editing?: boolean;
    onSaved?: (saved: Advantage) => void;
    onCancelled?: () => void;
    onDelete?: (id: number) => void;
  }

  let { advantage, editing = $bindable(false), onSaved, onCancelled, onDelete }: Props = $props();

  // Local edit-mode draft state — only populated when entering edit mode
  let draftName = $state('');
  let draftDescription = $state('');
  let draftTags = $state<string[]>([]);
  let draftKind = $state<AdvantageKind>('merit');
  let draftLevel = $state(0);

  let dirty = $derived(
    editing && (
      draftName !== advantage.name ||
      draftDescription !== (advantage.description ?? '') ||
      JSON.stringify(draftTags) !== JSON.stringify(advantage.tags) ||
      draftKind !== advantage.kind ||
      draftLevel !== (Number(advantage.properties.find(p => p.name === 'dot_cost_per_level')?.value) ?? 0)
    )
  );

  let validationWarn = $state<string | null>(null);
  let busy = $state(false);

  function enterEdit() {
    draftName = advantage.name;
    draftDescription = advantage.description ?? '';
    draftTags = [...advantage.tags];
    draftKind = advantage.kind;
    draftLevel = Number(advantage.properties.find(p => p.name === 'dot_cost_per_level')?.value) ?? 0;
    validationWarn = null;
    editing = true;
  }

  function buildInput(asWip: boolean): AdvantageInput {
    return {
      name: draftName,
      kind: draftKind,
      description: draftDescription,
      tags: [...draftTags],
      properties: [{ name: 'dot_cost_per_level', type: 'number', value: draftLevel }],
      effects: [...advantage.effects.map(e => ({ ...e, paths: [...e.paths] }))],
      wip: asWip,
      confirmedFields: advantage.confirmedFields ?? null,
    } as AdvantageInput;
  }

  async function save() {
    if (busy) return;
    busy = true;
    try {
      const wip = draftName.trim() === '';
      await updateAdvantage(advantage.id, buildInput(wip));
      onSaved?.({ ...advantage, name: draftName, description: draftDescription, tags: [...draftTags], kind: draftKind });
      editing = false;
    } finally { busy = false; }
  }

  function cancel() {
    editing = false;
    onCancelled?.();
  }

  async function navigateToBonus() {
    // Spec §"Hand-off to Form #2" — Gap 1 resolution (c): auto-save WIP, then route.
    if (busy) return;
    busy = true;
    try {
      const wip = draftName.trim() === '';
      await updateAdvantage(advantage.id, buildInput(wip));
      editing = false;
      await goto(`/builder/advantage/${advantage.id}?stage=bonus`);
    } finally { busy = false; }
  }

  function onNameBlur() {
    if (draftName.trim() === '') {
      validationWarn = '⚠ name is empty — won\'t save as complete';
    } else if (validationWarn?.includes('name')) {
      validationWarn = null;
    }
  }
</script>

{#if !editing}
  <AdvantageCard saved={advantage} onEditClick={enterEdit} {onDelete} />
{:else}
  <article class="vamp-card editing">
    <div class="frame">
      <div class="hint-strip">
        click any field · <span class="kbd">Tab</span> walks · <span class="kbd">Esc</span> cancels · <span class="kbd">Ctrl+S</span> saves
      </div>
      <header class="zone-title">
        <input
          type="text"
          bind:value={draftName}
          onblur={onNameBlur}
          class="name-input"
          aria-label="Advantage name"
        />
      </header>
      <section class="zone-content">
        <textarea
          bind:value={draftDescription}
          class="description-input"
          aria-label="Description"
          rows="3"
        ></textarea>
        <TagChipInput bind:tags={draftTags} />
        {#if validationWarn}
          <p class="validation-warn" role="status">{validationWarn}</p>
        {/if}
        <div class="hairline" aria-hidden="true"></div>
      </section>
      <footer class="zone-footer">
        <div class="detail" data-field="type">
          <h6>TYPE</h6>
          <select bind:value={draftKind} class="kind-select" aria-label="Kind">
            <option value="merit">Merit</option>
            <option value="flaw">Flaw</option>
            <option value="background">Background</option>
            <option value="boon">Boon</option>
          </select>
        </div>
        <div class="detail" data-field="level">
          <h6>LEVEL</h6>
          <span class="value tier-dots" role="slider" aria-label="Level" aria-valuemin="0" aria-valuemax="5" aria-valuenow={draftLevel} tabindex="0" onkeydown={onLevelKeydown}>
            {#each Array(5) as _, i}
              <button type="button" class="dot" class:filled={i < draftLevel} aria-label={`Set to ${i + 1}`} onclick={() => draftLevel = i + 1}></button>
            {/each}
          </span>
        </div>
        <div class="detail" data-field="bonus">
          <h6>BONUS</h6>
          <button type="button" class="bonus-edit" onclick={navigateToBonus} disabled={busy}>
            edit →
          </button>
        </div>
      </footer>
    </div>
  </article>
  <div class="edit-footer">
    <span class="kbd-hint">
      <span class="kbd">Enter</span> saves · <span class="kbd">Esc</span> cancels
    </span>
    <div class="actions">
      <button type="button" onclick={cancel} disabled={busy}>Cancel</button>
      <button type="button" class="save" onclick={save} disabled={busy}>Save</button>
    </div>
  </div>
{/if}
```

Add the level keyboard handler:

```ts
function onLevelKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
    draftLevel = Math.max(0, draftLevel - 1);
    e.preventDefault();
  } else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
    draftLevel = Math.min(5, draftLevel + 1);
    e.preventDefault();
  } else if (e.key === 'Home') { draftLevel = 0; e.preventDefault(); }
  else if (e.key === 'End') { draftLevel = 5; e.preventDefault(); }
  else if (/^[0-5]$/.test(e.key)) { draftLevel = Number(e.key); e.preventDefault(); }
}
```

- [ ] **Step 2: Style the edit mode**

```css
.vamp-card.editing .frame {
  border-style: dashed;
  border-color: var(--accent-amber);
}
.hint-strip {
  font-size: 0.55rem;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  text-align: center;
  padding: 0.3rem 0.5rem 0;
  font-family: 'Spartan', sans-serif;
  text-transform: uppercase;
}
.name-input {
  width: 100%;
  background: transparent;
  border: 1px solid var(--outline-color);
  font-family: 'Spartan', sans-serif;
  font-weight: 200;
  font-size: 1.05rem;
  letter-spacing: 1.6px;
  text-transform: uppercase;
  text-align: center;
  color: inherit;
  padding: 0.2rem 0.5rem;
}
.description-input {
  width: 100%;
  min-height: 3.5rem;
  background: transparent;
  border: 1px solid var(--outline-color);
  font-family: 'Oswald', sans-serif;
  font-weight: 200;
  font-size: 0.82rem;
  letter-spacing: 0.4px;
  color: inherit;
  resize: vertical;
}
.validation-warn {
  color: var(--validation-warn);
  font-size: 0.7rem;
  margin: 0.25rem 0 0;
}
.kind-select {
  width: 100%;
  background: transparent;
  border: 1px solid var(--outline-color);
  color: inherit;
  font-family: 'Oswald', sans-serif;
  font-size: 0.8rem;
}
.bonus-edit {
  background: transparent;
  border: 1px solid var(--outline-color);
  color: var(--accent-amber);
  font-family: 'Oswald', sans-serif;
  font-size: 0.78rem;
  cursor: pointer;
  padding: 0.15rem 0.4rem;
}
.bonus-edit:hover { background: rgba(217, 165, 96, 0.08); }
.edit-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.4rem 0.5rem;
  background: rgba(0, 0, 0, 0.2);
  margin-top: 0.3rem;
}
.kbd-hint { font-size: 0.6rem; color: var(--text-muted); font-family: 'Oswald', sans-serif; }
.kbd {
  display: inline-block;
  background: rgba(0, 0, 0, 0.4);
  border: 1px solid var(--outline-color);
  border-radius: 2px;
  padding: 0 0.25rem;
  font-family: ui-monospace, monospace;
  font-size: 0.65rem;
}
.actions { display: flex; gap: 0.3rem; }
.actions .save { background: var(--accent-red); color: var(--text-primary); }
.tier-dots .dot {
  background: transparent;
  border: 1px solid var(--accent-amber);
  width: 0.55rem; height: 0.55rem; border-radius: 50%; padding: 0; cursor: pointer;
}
.tier-dots .dot.filled { background: var(--accent-amber); box-shadow: 0 0 3px rgba(217, 165, 96, 0.55); }
.tier-dots:focus-visible { outline: 1px solid var(--accent-amber); outline-offset: 2px; }
```

- [ ] **Step 3: Verify svelte-check**

Run: `npm run check`
Expected: no new errors. If `AdvantageCard` doesn't yet accept `onEditClick`, that prop was already added in Task 3; verify it.

- [ ] **Step 4: Visual smoke-test**

Run: `npm run tauri dev`
Temporarily mount InlineEditCard somewhere visible (e.g. `src/routes/builder-test/+page.svelte`):

```svelte
<InlineEditCard advantage={someAdvantage} />
```

Confirm:
- Read mode shows the standard AdvantageCard.
- Hovering reveals ✎; clicking enters edit mode.
- Frame outline changes to dashed amber; hint strip appears.
- Tab walks: name → description → tag input → type select → tier pip group → bonus "edit →" → Cancel → Save.
- Esc cancels.
- Save commits (verify against the IPC by reloading the library).
- "edit →" auto-saves and routes to `/builder/advantage/[id]?stage=bonus`.
- Inline blur validation: empty the name and tab away — orange warning appears.

- [ ] **Step 5: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/InlineEditCard.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
feat(card): InlineEditCard wraps AdvantageCard with edit mode

Form #1 — Inline-edit card per spec §"Two-form structure · Form
#1". InlineEditCard takes editing:boolean and renders either the
read-only AdvantageCard or the inline form (input/textarea/chip-
input/select/pip-group inside the same outlined frame). Save
commits via updateAdvantage; Cancel discards; "edit →" auto-saves
as WIP and routes to /builder/advantage/[id]?stage=bonus per spec
Gap 1 resolution.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: Tab order, Save/Cancel, inline blur validation, Ctrl+S

**Spec refs:** §"Two-form structure · Form #1 — Tab order inside edit mode", §"Best-practice rules · 1, 3, 5", §"Validation: inline on blur"

**Files:**
- Modify: `src/lib/components/InlineEditCard.svelte`

**Tests:** not required.

- [ ] **Step 1: Verify the Tab order matches the spec**

Expected order per spec line: `name → description → tags chip-input → type select → tier pip group → bonus "edit →" button → Cancel → Save`.

The natural DOM order produced by Task 10's markup already matches this if Cancel/Save sit AFTER the card in the source. Move the `<div class="edit-footer">` block such that Cancel/Save are last in the tab order (they should be, given they're below `<article>`).

Add `tabindex="0"` to the `<span class="tier-dots">` to ensure it joins the tab order (the spec says one tab stop for the pip group). The inner `.dot` buttons should be `tabindex="-1"` (focus stays on the group; arrow keys walk).

```svelte
<span
  class="value tier-dots"
  role="slider"
  aria-label="Level"
  aria-valuemin="0"
  aria-valuemax="5"
  aria-valuenow={draftLevel}
  tabindex="0"
  onkeydown={onLevelKeydown}
>
  {#each Array(5) as _, i}
    <button type="button" tabindex="-1" class="dot" class:filled={i < draftLevel} aria-label={`Set to ${i + 1}`} onclick={() => draftLevel = i + 1}></button>
  {/each}
</span>
```

- [ ] **Step 2: Wire Ctrl+S to Save**

Add a global keydown listener while editing:

```ts
import { onMount } from 'svelte';

function onWindowKeydown(e: KeyboardEvent) {
  if (!editing) return;
  if (e.key === 'Escape') { cancel(); e.preventDefault(); }
  else if ((e.ctrlKey || e.metaKey) && e.key === 's') { void save(); e.preventDefault(); }
}

onMount(() => {
  window.addEventListener('keydown', onWindowKeydown);
  return () => window.removeEventListener('keydown', onWindowKeydown);
});
```

`Esc` while editing exits edit mode (alongside the form-level Cancel button).

- [ ] **Step 3: Inline blur validation — extend `onNameBlur` and add `onDescriptionBlur`**

Currently Task 10 added `onNameBlur`. Add:

```ts
function onDescriptionBlur() {
  // Description is optional per the spec — no validation message.
}
```

(That's a documented intentional no-op; if you genuinely don't need a function, just omit `onblur` from the textarea. The above is illustrative of why no validation exists for description.)

- [ ] **Step 4: Visual focus rings on all interactive elements**

Confirm `:focus-visible` styling is present on `.name-input`, `.description-input`, `.kind-select`, `.bonus-edit`, `.tier-dots`, `.actions button`, `.affordance`. If any are missing, add:

```css
.name-input:focus-visible,
.description-input:focus-visible,
.kind-select:focus-visible,
.bonus-edit:focus-visible,
.actions button:focus-visible {
  outline: 1px solid var(--accent-amber);
  outline-offset: 2px;
}
```

- [ ] **Step 5: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 6: Visual smoke-test**

Run: `npm run tauri dev`
- Open the test mount. Press Tab repeatedly from the name input. Confirm the order matches: name → description → tag-input → type-select → pip group → bonus button → Cancel → Save.
- Press `Esc` while editing — exits edit mode.
- Press `Ctrl+S` while editing — saves and exits edit mode.
- Empty the name field and tab away — orange `⚠` warning appears between description and footer.
- Focus rings (amber) are visible on every interactive element when keyboard-focused.

- [ ] **Step 7: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/InlineEditCard.svelte
git commit -m "$(cat <<'EOF'
feat(card): Form #1 keyboard polish — Tab order, Ctrl+S, focus rings

Tab order matches spec §"Tab order inside edit mode": name →
description → tags → type → pips → bonus → Cancel → Save. Pip group
is a single tab stop with arrow-key navigation (W3C APG Slider
pattern). Ctrl+S saves, Esc cancels via a window-level keydown
listener gated by editing. :focus-visible rings on every
interactive element.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: AdvantagesManager renders InlineEditCard (replace AdvantageCard at the call site)

**Spec refs:** §"Two-form structure · Form #1 · Entry point" (library list click → inline edit)

**Files:**
- Modify: `src/tools/AdvantagesManager.svelte`

**Tests:** not required.

- [ ] **Step 1: Replace `<AdvantageCard>` in the library iteration with `<InlineEditCard>`**

Locate the iteration block (typically `{#each advantages as a (a.id)}`). Replace:

```svelte
<AdvantageCard
  saved={a}
  onDelete={async (id) => { await deleteAdvantage(id); advantages = advantages.filter(x => x.id !== id); }}
/>
```

with:

```svelte
<InlineEditCard
  advantage={a}
  onSaved={(saved) => { advantages = advantages.map(x => x.id === saved.id ? saved : x); }}
  onDelete={async (id) => { await deleteAdvantage(id); advantages = advantages.filter(x => x.id !== id); }}
/>
```

Update the import:

```ts
import InlineEditCard from '$lib/components/InlineEditCard.svelte';
// Remove the import of AdvantageCard if it's no longer referenced directly in this file.
```

- [ ] **Step 2: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 3: Visual smoke-test**

Run: `npm run tauri dev`
Open the Advantages tool. Confirm:
- Each card now responds to click → inline edit mode (✎ icon reveals on hover).
- "edit →" on the bonus footer box auto-saves and routes to `/builder/advantage/[id]?stage=bonus`.
- The new `?stage=bonus` query param doesn't yet seed the stage machine (Task 13 wires that); the route loads on the default first-unconfirmed stage. The hand-off still works — bonus is just not pre-selected. Note this as the gap to close in Task 13.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/tools/AdvantagesManager.svelte
git commit -m "$(cat <<'EOF'
refactor(advantages): use InlineEditCard in library list

Replaces direct <AdvantageCard> mounts with <InlineEditCard>, so
each advantage card supports click-to-edit (Form #1) inline.
Delete affordance flows through unchanged. The "edit →" hand-off
auto-saves and routes to /builder/advantage/[id]?stage=bonus per
spec §"Hand-off to Form #2".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 13: Seed Form #2 stage machine from `?stage=` query param

**Spec refs:** §"Two-form structure · Form #2 · Entry points" (entry #3: "edit →" handoff opens at `/builder/advantage/{id}/bonus` or equivalent stage-deep-link)

**Files:**
- Modify: `src/routes/builder/advantage/[id]/+page.svelte` — consume `?stage=` param
- Modify: `src/lib/components/builder/bodies/AdvantageBody.svelte` — accept an `initialStage` prop that seeds `machine.stage` after mount

**Tests:** not required.

- [ ] **Step 1: Expose `initialStage` from AdvantageBody**

In `src/lib/components/builder/bodies/AdvantageBody.svelte`, extend the `Props` interface:

```ts
interface Props {
  initial?: Advantage;
  existingCount?: number;
  valid?: boolean;
  bodyState?: AdvantageBodyState;
  initialStage?: 'name' | 'type' | 'level' | 'bonus' | 'description' | 'tags';
}
```

Destructure with default:

```ts
let { initial, existingCount = 0, valid = $bindable(false), bodyState = $bindable<AdvantageBodyState>(), initialStage }: Props = $props();
```

After the `createStageMachine` call, if `initialStage` is provided, set the stage:

```ts
if (initialStage) {
  machine.setStage(initialStage);
  // setStage un-confirms the target stage; that's the intended re-edit behavior
}
```

- [ ] **Step 2: Pass `initialStage` from the route**

In `src/routes/builder/advantage/[id]/+page.svelte`:

```ts
const stage = $derived(page.url.searchParams.get('stage') ?? null);
// type-narrow the stage param to allowed values
const initialStage = $derived(
  (['name', 'type', 'level', 'bonus', 'description', 'tags'] as const).includes(stage as any)
    ? (stage as 'name' | 'type' | 'level' | 'bonus' | 'description' | 'tags')
    : undefined
);
```

Then pass into the body:

```svelte
<AdvantageBody {initial} bind:bodyState bind:valid {initialStage} />
```

- [ ] **Step 3: Verify svelte-check**

Run: `npm run check`
Expected: no new errors.

- [ ] **Step 4: Visual smoke-test**

Run: `npm run tauri dev`
- Open the Advantages tool, click any card to enter Form #1 edit mode, click "edit →" on the bonus footer.
- The route opens with the bonus stage active in the spotlight (not the first unconfirmed stage).
- Save returns to the library list.

- [ ] **Step 5: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/builder/bodies/AdvantageBody.svelte src/routes/builder/advantage/\[id\]/+page.svelte
git commit -m "$(cat <<'EOF'
feat(builder): seed Form #2 stage from ?stage= query param

Closes the Form #1 → Form #2 hand-off loop: edit→ on Form #1's
bonus row routes to /builder/advantage/[id]?stage=bonus, and the
route now reads ?stage= and passes it as initialStage to
AdvantageBody, which seeds machine.stage on mount.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 4 — APG keyboard widgets

> Each task below applies the W3C ARIA Authoring Practices Guide pattern named in §"Custom widget keyboard support". The spec table is authoritative for which keys each widget accepts; this plan references the spec rather than reproducing the full table per task.

### Task 14: ValueSelector — APG Slider keyboard map

**Spec refs:** §"Custom widget keyboard support · ValueSelector" (Slider pattern)

**Files:**
- Modify: `src/lib/components/builder/ValueSelector.svelte`

**Tests:** not required.

- [ ] **Step 1: Audit current widget**

Read the file in full. Identify the focusable element (probably the blood-dot row container), the value-binding pattern, and the existing event handlers.

- [ ] **Step 2: Apply ARIA + keyboard map**

The container element acquires:

```svelte
<div
  class="value-row"
  role="slider"
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={value}
  aria-label={label ?? 'Value'}
  tabindex="0"
  onkeydown={onKeydown}
>
  <!-- dot buttons (tabindex=-1) -->
</div>
```

```ts
function onKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
    value = Math.max(min, value - 1);
    e.preventDefault();
  } else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
    value = Math.min(max, value + 1);
    e.preventDefault();
  } else if (e.key === 'PageDown') {
    value = Math.max(min, value - 10);
    e.preventDefault();
  } else if (e.key === 'PageUp') {
    value = Math.min(max, value + 10);
    e.preventDefault();
  } else if (e.key === 'Home') { value = min; e.preventDefault(); }
  else if (e.key === 'End') { value = max; e.preventDefault(); }
  else if (/^[0-9]$/.test(e.key)) {
    // Numeric direct typing (single-digit at a time; for two-digit see APG note)
    const n = Number(e.key);
    if (n >= min && n <= max) { value = n; e.preventDefault(); }
  }
}

function onWheel(e: WheelEvent) {
  if (document.activeElement !== e.currentTarget) return;  // only when focused
  if (e.deltaY < 0) value = Math.min(max, value + 1);
  else value = Math.max(min, value - 1);
  e.preventDefault();
}
```

Wire `onwheel={onWheel}` on the container.

- [ ] **Step 3: Verify svelte-check + visual smoke-test**

Run: `npm run check`
Expected: no new errors.

Run: `npm run tauri dev`
Open `/builder/advantage/new`, walk to the bonus value sub-stage. Confirm:
- Tab lands on the value row once.
- Arrow keys ±1.
- PageUp/PageDown ±10 (clamped).
- Home/End jump to min/max.
- Numeric typing sets the value directly.
- Mouse wheel adjusts when focused.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/ValueSelector.svelte
git commit -m "$(cat <<'EOF'
feat(a11y): ValueSelector — APG Slider keyboard pattern

role=slider with aria-valuemin/max/now. One tab stop on the row.
Arrows ±1, PageUp/Down ±10, Home/End jump to bounds, numeric
direct typing, mouse wheel when focused. Per spec §"Custom widget
keyboard support".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 15: TagChipInput — APG Combobox + Token List

**Spec refs:** §"Custom widget keyboard support · TagChipInput"

**Files:**
- Modify: `src/lib/components/builder/TagChipInput.svelte`

**Tests:** not required.

- [ ] **Step 1: Read current widget**

Run: `wc -l src/lib/components/builder/TagChipInput.svelte` then read.

- [ ] **Step 2: Apply ARIA + keyboard map**

Container:

```svelte
<div role="combobox" aria-haspopup="listbox" aria-expanded="false" class="tag-chip-input">
  <ul role="list" class="chips">
    {#each tags as t, i}
      <li role="listitem">
        <span class="chip">
          {t}
          <button type="button" tabindex="-1" class="chip-remove" aria-label={`Remove ${t}`} onclick={() => removeChipAt(i)}>×</button>
        </span>
      </li>
    {/each}
  </ul>
  <input
    type="text"
    bind:value={draft}
    onkeydown={onInputKeydown}
    aria-label="Add tag"
    placeholder="+ add tag"
  />
</div>
```

Keyboard map:

```ts
function onInputKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' || e.key === ',') {
    if (draft.trim().length > 0) {
      tags = [...tags, draft.trim()];
      draft = '';
    }
    e.preventDefault();
  } else if (e.key === 'Backspace' && draft === '' && tags.length > 0) {
    tags = tags.slice(0, -1);
    e.preventDefault();
  } else if (e.key === 'Escape') {
    draft = '';
    e.preventDefault();
  }
}
```

Chips are NOT in the natural tab order (per spec: "chips are NOT in tab order by default"); their `×` buttons are `tabindex="-1"`. To make a chip removable via keyboard, the user can `Shift+Tab` from the input back to the last chip's × button — but that requires the chip × buttons to be tabbable. The spec says "chip × button focusable via Shift+Tab from input." That means the × buttons ARE in the tab order, just AFTER the input in DOM order? Re-read the spec carefully.

Spec: "1 tab stop on the input; chips are NOT in tab order by default ... chip × button focusable via Shift+Tab from input."

The chips ARE before the input in DOM order. Tab walks DOM. Shift+Tab from input goes to the previous tab stop. If the chip × buttons are tabbable, Shift+Tab would walk back through them. Implementation: set `tabindex="0"` on the chip × buttons. The phrase "chips are NOT in tab order by default" likely refers to the chip body (the text); only the × button is tabbable.

Revised:

```svelte
<button type="button" tabindex="0" class="chip-remove" aria-label={`Remove ${t}`} onclick={() => removeChipAt(i)}>×</button>
```

- [ ] **Step 3: Verify svelte-check + visual smoke-test**

Run: `npm run check`
Expected: no new errors.

Run: `npm run tauri dev`
Open `/builder/advantage/new`, walk to the tags stage. Confirm:
- Tab lands on the chip-input field (after walking past existing chip × buttons via Shift+Tab from input).
- Type a word + `Enter` → adds chip, clears input.
- Type a word + `,` → adds chip, clears input.
- Empty input + Backspace → removes last chip.
- Esc → clears draft.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/TagChipInput.svelte
git commit -m "$(cat <<'EOF'
feat(a11y): TagChipInput — APG combobox + token list

role=combobox with role=list for chips. Enter/, commits a chip;
Backspace on empty input removes the last chip; Esc clears the
draft. Chip × buttons are tabbable (Shift+Tab from the input
walks back through them). Per spec §"Custom widget keyboard
support".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 16: AppliesToPicker — APG Toolbar + Listbox

**Spec refs:** §"Custom widget keyboard support · AppliesToPicker"

**Files:**
- Modify: `src/lib/components/builder/AppliesToPicker.svelte`

**Tests:** not required.

- [ ] **Step 1: Read current widget**

Identify the chip group and the disclosure dropdown.

- [ ] **Step 2: Apply ARIA + keyboard map**

Group container:

```svelte
<div role="toolbar" aria-label="Applies to" class="applies-to" onkeydown={onGroupKeydown}>
  {#each chips as c, i}
    <button
      type="button"
      class="chip"
      class:selected={selected.has(c)}
      tabindex={i === activeChipIndex ? 0 : -1}
      role="button"
      aria-pressed={selected.has(c)}
      onclick={() => toggle(c)}
      onfocus={() => activeChipIndex = i}
    >{c}</button>
  {/each}
  <!-- disclosure to listbox -->
</div>
```

```ts
let activeChipIndex = $state(0);

function onGroupKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowLeft') {
    activeChipIndex = (activeChipIndex - 1 + chips.length) % chips.length;
    focusChip(activeChipIndex);
    e.preventDefault();
  } else if (e.key === 'ArrowRight') {
    activeChipIndex = (activeChipIndex + 1) % chips.length;
    focusChip(activeChipIndex);
    e.preventDefault();
  } else if (e.key === ' ' || e.key === 'Spacebar') {
    toggle(chips[activeChipIndex]);
    e.preventDefault();
  } else if (e.key === 'Enter') {
    // Confirm and advance — emit a custom event the body listens to
    onConfirm?.();
    e.preventDefault();
  }
}

function focusChip(i: number) {
  const el = (document.activeElement?.parentElement?.querySelectorAll('.chip') ?? [])[i] as HTMLElement | undefined;
  el?.focus();
}
```

Disclosure "A skill…" opens a listbox. The listbox itself follows the APG Listbox pattern (arrow keys walk, Enter selects, Esc closes). Implementer reads the APG Listbox pattern in detail at <https://www.w3.org/WAI/ARIA/apg/patterns/listbox/> for completeness.

- [ ] **Step 3: Verify svelte-check + visual smoke-test**

Run: `npm run check`
Expected: no new errors.

Run: `npm run tauri dev`
Walk to the bonus applies-to sub-stage. Confirm:
- Tab lands on the chip group once.
- Arrow keys move focus between chips.
- Space toggles the focused chip.
- Enter confirms and advances.
- The "A skill…" disclosure opens a listbox dropdown.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/AppliesToPicker.svelte
git commit -m "$(cat <<'EOF'
feat(a11y): AppliesToPicker — APG toolbar + listbox

role=toolbar for the chip group with roving tabindex. Arrows move
focus, Space toggles selection, Enter confirms. Disclosure to a
Listbox-pattern dropdown for skill selection. Per spec §"Custom
widget keyboard support".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 17: TypePicker — APG Radio Group

**Spec refs:** §"Custom widget keyboard support · TypePicker"

**Files:**
- Modify: `src/lib/components/builder/TypePicker.svelte`

**Tests:** not required.

- [ ] **Step 1: Read current widget**

- [ ] **Step 2: Apply ARIA + keyboard map**

```svelte
<div role="radiogroup" aria-label="Kind" class="type-picker" onkeydown={onKeydown}>
  {#each kinds as k, i}
    <button
      type="button"
      class="kind-card"
      role="radio"
      aria-checked={selected === k}
      tabindex={selected === k ? 0 : (i === 0 && !kinds.some(kk => kk === selected) ? 0 : -1)}
      onclick={() => selected = k}
    >{kindLabel(k)}</button>
  {/each}
</div>
```

```ts
function onKeydown(e: KeyboardEvent) {
  if (['ArrowUp', 'ArrowLeft'].includes(e.key)) {
    const i = kinds.indexOf(selected);
    selected = kinds[(i - 1 + kinds.length) % kinds.length];
    focusSelected();
    e.preventDefault();
  } else if (['ArrowDown', 'ArrowRight'].includes(e.key)) {
    const i = kinds.indexOf(selected);
    selected = kinds[(i + 1) % kinds.length];
    focusSelected();
    e.preventDefault();
  } else if (e.key === ' ' || e.key === 'Enter') {
    // Confirm — selected is already set on focus; advance the stage
    onConfirm?.();
    e.preventDefault();
  }
}
```

- [ ] **Step 3: Verify svelte-check + visual smoke-test**

Run: `npm run check`. Smoke test if TypePicker is reachable (per spec it's not exposed in v1's production callers; if so, mount briefly in `src/routes/builder-test/+page.svelte` to confirm behavior, then revert the temp mount).

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/TypePicker.svelte
git commit -m "$(cat <<'EOF'
feat(a11y): TypePicker — APG radio group

role=radiogroup with role=radio cards. Roving tabindex tracks the
selected kind. Arrow keys move + select; Space/Enter confirm.
Smoke-only callers in v1 per spec §"Non-goals" (no Library home
page exposed yet).

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 18: Tier dot pips — APG discrete Slider

**Spec refs:** §"Custom widget keyboard support · Tier dot pips"

**Files:**
- This is the pip group used in Form #1 InlineEditCard (already wired in Task 11 with the partial slider keyboard map) AND in Form #2's level-stage spotlight inside AdvantageBody.
- Modify: `src/lib/components/builder/bodies/AdvantageBody.svelte` — the level stage's pip group acquires the same ARIA + keyboard map as Task 11's pip group in InlineEditCard
- Modify: `src/lib/components/InlineEditCard.svelte` — extend Task 11's `onLevelKeydown` to handle `PageUp`/`PageDown` (currently missing) to keep behavior identical to APG Slider

**Tests:** not required.

- [ ] **Step 1: Locate the level pip group in AdvantageBody**

Run: `grep -n 'level\|dot\|tier\|pip' src/lib/components/builder/bodies/AdvantageBody.svelte | head -30`
Expected: identify the existing level pip rendering.

- [ ] **Step 2: Apply ARIA + keyboard map to AdvantageBody's level pips**

Mirror the InlineEditCard pattern from Task 11. Container becomes:

```svelte
<span
  class="level-pips"
  role="slider"
  aria-label="Level"
  aria-valuemin="0"
  aria-valuemax="5"
  aria-valuenow={bodyState.level}
  tabindex="0"
  onkeydown={onLevelKeydown}
>
  {#each Array(5) as _, i}
    <button type="button" tabindex="-1" class="dot" class:filled={i < bodyState.level} aria-label={`Set to ${i + 1}`} onclick={() => bodyState.level = i + 1}></button>
  {/each}
</span>
```

```ts
function onLevelKeydown(e: KeyboardEvent) {
  if (!bodyState) return;
  if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
    bodyState.level = Math.max(0, bodyState.level - 1);
    e.preventDefault();
  } else if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
    bodyState.level = Math.min(5, bodyState.level + 1);
    e.preventDefault();
  } else if (e.key === 'Home') { bodyState.level = 0; e.preventDefault(); }
  else if (e.key === 'End') { bodyState.level = 5; e.preventDefault(); }
  else if (/^[0-5]$/.test(e.key)) { bodyState.level = Number(e.key); e.preventDefault(); }
  else if (e.key === 'Enter') { machine.advance(); e.preventDefault(); }
}
```

- [ ] **Step 3: Verify svelte-check + visual smoke-test**

Run: `npm run check`. Smoke test the level stage in `/builder/advantage/new`:
- Tab lands on the pip group.
- Arrow keys ±1, Home/End to bounds, 0–5 direct typing.
- Enter advances the stage.

- [ ] **Step 4: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/builder/bodies/AdvantageBody.svelte src/lib/components/InlineEditCard.svelte
git commit -m "$(cat <<'EOF'
feat(a11y): tier dot pips — APG discrete Slider

The level pip group in both Form #1 (InlineEditCard) and Form #2
(AdvantageBody level spotlight) gains role=slider with arrows,
Home/End, 0-5 direct typing, and Enter-to-advance. Per spec
§"Custom widget keyboard support · Tier dot pips".

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5 — Cleanup

### Task 19: Delete `ItemBuilderShell.svelte` + drop callers

**Spec refs:** §"Non-goals · No backwards-compatibility shims" (shell is replaced; no shim needed), §"Component map · ItemBuilderShell.svelte — DELETED"

**Files:**
- Delete: `src/lib/components/builder/ItemBuilderShell.svelte`
- Modify: `src/routes/builder-test/+page.svelte` — remove the shell test mount; replace with route-link smoke test

**Tests:** not required.

- [ ] **Step 1: Verify no remaining imports**

Run: `grep -rn 'ItemBuilderShell' src/`
Expected: only `src/lib/components/builder/ItemBuilderShell.svelte` (the file itself) and possibly `src/routes/builder-test/+page.svelte`.

If any other file still imports the shell, fix it BEFORE proceeding — that's a forgotten caller from Task 8.

- [ ] **Step 2: Delete the shell file**

```bash
rm src/lib/components/builder/ItemBuilderShell.svelte
```

- [ ] **Step 3: Update `src/routes/builder-test/+page.svelte`**

Read the file. Replace the ItemBuilderShell mount with a simple route-link smoke page:

```svelte
<script lang="ts">
  // No imports needed — just links
</script>

<svelte:head><title>Builder routes — smoke test</title></svelte:head>

<main>
  <h1>Builder routes</h1>
  <ul>
    <li><a href="/builder/advantage/new">New advantage</a></li>
    <li><a href="/builder/template/new">New status template</a></li>
  </ul>
</main>
```

- [ ] **Step 4: Verify svelte-check**

Run: `npm run check`
Expected: no new errors. If you see "Cannot find module ItemBuilderShell.svelte", that's a residual import — fix it.

- [ ] **Step 5: Visual smoke-test**

Run: `npm run tauri dev`
Navigate to `/builder-test`. Confirm the page renders the route-links list and each link navigates to the expected page.

- [ ] **Step 6: Run `./scripts/verify.sh`**

Expected: green.

- [ ] **Step 7: Commit**

```bash
git add -A src/lib/components/builder/ItemBuilderShell.svelte src/routes/builder-test/+page.svelte
git commit -m "$(cat <<'EOF'
chore(builder): delete ItemBuilderShell, replace test mount with links

The dialog-based shell is fully replaced by the route + InlineEditCard
architecture per spec §"Non-goals · No backwards-compatibility shims".
builder-test page becomes a simple smoke-link list pointing at the new
routes.

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>
EOF
)"
```

---

### Task 20: End-of-branch holistic visual audit + final code review

**Spec refs:** §"Workflow safeguards · 4. End-of-branch holistic visual audit"

**Files:** none directly modified in this task; any drift found becomes a fix commit (or a v1.x backlog note).

**Tests:** not required.

- [ ] **Step 1: Run the full verify suite**

Run: `./scripts/verify.sh`
Expected: green.

Run: `npm test`
Expected: all vitest tests pass (existing + the 4 new Latin commit semantics tests from Task 5).

- [ ] **Step 2: Holistic visual audit in `npm run tauri dev`**

Walk all three flows and check against the spec section noted:

**Advantage flow:**
- Open the Advantages tool — card list renders with Witcher three-zone outline+offset frame (no hero). §"Card visual design"
- Hover a card — ✎ at top-right, 🗑 at bottom-right; different corners. §"Hover-reveal edit/delete affordance"
- Click a card — enters Form #1 inline edit mode; dashed amber outline; hint strip visible. §"Two-form structure · Form #1"
- Edit name, description, tags, type, level; tab order matches spec. §"Tab order inside edit mode"
- Empty name → orange `⚠` warning on blur. §"Validation"
- Ctrl+S saves; Esc cancels. §"Best-practice rules · 6"
- Click "edit →" on bonus → auto-saves WIP, routes to /builder/advantage/[id]?stage=bonus. §"Hand-off to Form #2"
- Form #2 opens directly on the bonus stage. §"Two-form structure · Form #2 · Entry points #3"
- Footer shows `Delete` (left), `Cancel`, `Save`. §"Footer (zone 4 bottom half)"
- Delete confirms; deletion removes the card.
- "+ Add Custom" navigates to /builder/advantage/new (not a dialog). §"Form factor: SvelteKit route, NOT a <dialog> modal"
- Save WIP succeeds with just `name` filled. §"Save WIP always succeeds"

**StatusTemplate flow:**
- GM Screen palette — template cards render with three-zone outline (two-box footer). §"Card visual design (StatusTemplate variant)"
- Hover a card — 🗑 at bottom-right; NO ✎. §"Hover-reveal edit affordance · Applies to AdvantageCard only"
- "+ New template" navigates to /builder/template/new.
- Edit existing template via /builder/template/[id]; Delete works.

**Modifier flow:**
- Character row / character card → edit modifier navigates to /builder/modifier/[id].
- Modifier route loads + saves + deletes.

**Custom widget keyboards:**
- ValueSelector — arrows ±1, PageUp/Down ±10, Home/End, 0–9 direct.
- TagChipInput — Enter/, commits, Backspace removes last, Esc clears.
- AppliesToPicker — arrows walk, Space toggle, Enter confirm.
- TypePicker — arrows walk + select, Space/Enter confirm.
- Tier dot pips — arrows, Home/End, 0–5 direct, Enter advance.

Document any drift in a final commit message OR open a follow-up issue per the user's standing roadmap policy (subtask issues do NOT go on the project board; v1.x polish notes go in the parent issue's body).

- [ ] **Step 3: Final code review (lean execution gate)**

Per project CLAUDE.md and the spec's workflow safeguard #3:
- Per-task spec-compliance reviewer subagents were dispatched throughout this plan (one after each task).
- Per-task code-quality reviewers were dropped (lean execution default).
- A SINGLE `code-review:code-review` runs HERE against the full branch diff.

Invoke `code-review:code-review` against the `visual-remediation` branch.

The reviewer's job at this scope:
- Bugs / logic errors that survived task-level review.
- CLAUDE.md adherence (hex tokens, async fs, typed wrappers, IPC inventory updates).
- Cross-task consistency (the same Stage type used in routes + bodies + InlineEditCard, etc.).

- [ ] **Step 4: Final commit (only if drift was caught and fixed during the audit)**

If the audit found drift and you committed fixes, no additional task commit needed — those fixes ARE the audit work. Otherwise, no commit for Task 20; the task is signed off by the reviewer's approval.

If no drift was found and the reviewer approved, mark the branch ready for merge per `superpowers:finishing-a-development-branch`.

---

## Self-review

Before handing off to subagent-driven-development, the plan author should walk this checklist (this is for me, not the implementers):

### 1. Spec coverage

| Spec section | Tasks |
|---|---|
| §"Context" / "Goals" (narrative) | n/a — design rationale |
| §"Data model" — required:['name'], level default 0 | T5 |
| §"Two-form structure · Form #1 — Inline-edit card" | T10, T11, T12 |
| §"Two-form structure · Form #2 — Builder page" | T6, T7, T8, T9, T13 |
| §"Card visual design" — three-zone, no hero, typography | T2, T4 |
| §"Hover-reveal edit affordance" | T3 |
| §"Hover-reveal delete affordance" | T3, T4, T8 |
| §"CSS tokens (added to +layout.svelte)" | T1 |
| §"Form #2 zone specifications · title bar, card preview, spotlight, queue, footer" | T6, T7 (composer + route templates), T9 (footer Delete) |
| §"Stage state machine" | T5 (required-set change), T13 (initialStage seed) |
| §"Best-practice rules" | T1 (tokens), T2/T4 (visual), T11 (Tab order, focus, blur, Ctrl+S), T14–T18 (keyboard) |
| §"Custom widget keyboard support · ValueSelector" | T14 |
| §"Custom widget keyboard support · TagChipInput" | T15 |
| §"Custom widget keyboard support · AppliesToPicker" | T16 |
| §"Custom widget keyboard support · TypePicker" | T17 |
| §"Custom widget keyboard support · Tier dot pips" | T11 (Form #1), T18 (Form #2) |
| §"Custom widget keyboard support · CardSlot" | (already implemented in prior execution; not modified here) |
| §"Custom widget keyboard support · Latin placeholder commit semantics" | T5 |
| §"Responsive design" | T2, T6 (composer width strategy) |
| §"Save pipeline" | n/a — no changes |
| §"Component map" | T1–T19 collectively realize it; T19 finalizes the deletion |
| §"Workflow safeguards" | Plan header (per-task spec reviewer re-enabled), T20 (holistic audit) |
| §"Open questions" | All non-blocking calibration — no task needed unless user resolves them later |

No gaps detected.

### 2. Placeholder scan

Grepped the plan for the No-Placeholders red flags (`TBD`, `TODO`, `implement later`, `Similar to Task`, etc.). One intentional comment-out about a TODO in Task 6 was removed per CLAUDE.md (no future-context comments). No outstanding placeholders.

### 3. Type consistency

- `AdvantageBodyState` is exported by `AdvantageBody.svelte` and imported by Tasks 6, 10, 11, 12. Single source of truth.
- `Stage` type (from `stage-machine.svelte.ts`) and the explicit string union in `initialStage` (Task 13) are kept in sync — the union `'name' | 'type' | 'level' | 'bonus' | 'description' | 'tags'` matches the `tabOrder` for AdvantageBody.
- `AdvantageInput` shape is reused identically across all route save handlers — no drift.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-23-item-builder-shell-redesign.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — Dispatch one implementer subagent per task. Per the project's spec-specific override, the per-task spec-compliance reviewer is RE-ENABLED for this remediation (see plan header workflow override #1). Per-task code-quality reviewer stays dropped; a single `code-review:code-review` runs at Task 20.

2. **Inline Execution** — Execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints for review.

Which approach?
