# Item Builder Shell — Visual Compliance Remediation
_2026-05-22 · supersedes nothing; remediates the 2026-05-17 spec's execution_

## Context

The 2026-05-17 spec (`docs/superpowers/specs/2026-05-17-item-builder-shell-design.md`) and its derived plan (`docs/superpowers/plans/2026-05-19-item-builder-shell.md`) were executed across 23 tasks via subagent-driven development on the `templates-impl` branch. Final `./scripts/verify.sh` passed green; 325 cargo tests + 20 frontend tests passed; the branch merged into `master` (commit `00146ee`).

**Despite the green gate, the implementation has 10 visual / behavioral deviations from the brainstorm mockups + spec.** Post-merge testing surfaced the gaps. The root cause is that the lean-execution workflow override (one implementer per task, no per-task spec-compliance reviewer, only `verify.sh`) verified _compilation_ and _logic_ but never _visual conformance_. The brainstorm mockup HTMLs in `.superpowers/brainstorm/156015-1779148305/content/` are the ground truth — every deviation below is a regression against a specific mockup line.

This is a **bring-up-to-spec** remediation, not a feature extension. The original 2026-05-17 spec is correct and unchanged; this spec catalogs the gaps and prescribes the path back to compliance plus workflow safeguards to prevent recurrence.

## Goals

1. **Implementation matches brainstorm mockup 07-card-shape-spotlight.html** byte-for-byte where the mockup expresses a concrete visual choice (slot order, slot states, cursor indicator, queue terminator, WIP strip, step indicator, magnitude glow).
2. **Spec-only behavioral guarantees are honored** even where the mockup didn't visualize them (spotlight cross-fade, right-click=back, click-empty-slot-to-focus).
3. **Workflow safeguards baked into the new plan** so the next execution cycle catches drift before it lands.

## Non-goals

- **No new features.** Discipline/Item bodies, Library home page entry, and Foundry-roller `auto_success` consumption remain v1.x scope per the original spec.
- **No data-model changes.** The 0010 + 0011 migrations stay as-is. No schema work.
- **No new bodies, no new primitives.** The three existing bodies + three primitives are correct in shape; only the AdvantageBody spatial layout needs reordering.
- **No re-architecture of the stage machine.** The reactivity fix (rename to `.svelte.ts`, `$state` + `SvelteSet`) from in-flight uncommitted changes is the right shape — finalize and commit it.
- **No StatusTemplateBody or ModifierEffectBody row-order changes.** Audit confirms those bodies match their spec layouts. Only AdvantageBody is broken.

---

## Audit findings — concrete deviations

### Pending uncommitted (in-flight on `master`, blocking baseline)

These three were the focus of the prior session. They're applied to the working tree via Vite HMR but not yet committed. They unblock the rest of the audit: until height + reactivity + buttons are in, the user can't navigate far enough to *see* the row-order bug.

| # | Item | Files | State |
|---|------|-------|-------|
| **F1** | Dialog body collapses to 0 height (`<dialog>` UA default `height: fit-content` + `flex: 1` on body resolves to basis:0). User sees only header + bottom border. | `src/lib/components/builder/ItemBuilderShell.svelte` `.shell` rule | Pending commit — `height: min(40rem, calc(100vh - 4rem))` added |
| **F2** | Stage machine non-reactive (plain `let stage` + `new Set()` in `.ts` file). Tab/Enter calls `machine.advance()` but template never re-renders. | `src/lib/builder/stage-machine.svelte.ts` (renamed from `.ts`); 4 importers updated | Pending commit — uses `$state` + `SvelteSet` from `svelte/reactivity` |
| **F3** | No mouse-only path through `name` / `description` / `bonus-name` sub-stage (no clickable "Next" buttons; users were keyboard-locked) | `AdvantageBody.svelte` × 3 spots, `StatusTemplateBody.svelte` × 2 spots | Pending commit — "Next →" buttons added |

### Critical — direct spec violation, visible immediately (P0)

| # | Item | Mockup ref | Spec ref | Current state | Target state |
|---|------|-----------|----------|---------------|--------------|
| **D1** | **AdvantageBody spatial order: Description and Bonus rows are SWAPPED.** | 07 lines 358-378 | 2026-05-17 spec line 165-170, 286-290 | DOM order: row 3 = Bonus, row 4 = Description, row 5 = Tags | DOM order: row 3 = Description, row 4 = Bonus, row 5 = Tags. Tab order remains `name, type, level, bonus, description, tags` — but spatially, Description sits *above* Bonus on the card. This is the whole point of "tab order ≠ spatial order." |

### Significant — agreed-on UI elements absent (P1)

| # | Item | Mockup ref | Current state | Target state |
|---|------|-----------|---------------|--------------|
| **D2** | **No step indicator in title bar.** Mockup shows `step 2 of 2 · new merit` as small uppercase tag in the composer-bar. | 07 lines 318-321, 425 | Header just shows title + close. | `ItemBuilderShell` gains optional `stepIndicator?: string` prop (e.g., `"step 2 of 2 · new merit"`). Rendered to the left of the title; small, muted, uppercase. |
| **D3** | **No WIP strip in footer.** Mockup shows an amber pill `· description still empty · saveable as draft` next to Cancel/Save when invalid. | 07 lines 277-284, 407-411 | Footer has just buttons. | When `valid === false`, render a `<div class="wip-strip">` with `<span class="wip-dot">` + concise list of missing required fields. Color: `--wip` (gold-warm, new token to add). |
| **D4** | **No cursor indicator on active slot.** Mockup shows a blinking `▾` glyph next to the active field's value. | 07 lines 143-151, 363 | Active slot pulses border only. | `CardSlot.svelte`: when `state === 'active'`, render `<span class="cursor-indicator">▾</span>` after the value. Blink animation per mockup. |
| **D5** | **Queue doesn't terminate with "Save"** highlighted. Mockup queue ends with `→ Save` in `--accent-bright` weight 600. | 07 lines 398-405 | Queue lists pending stages only. | `CardComposer` queue appends a final `<span class="queue-item save-terminator">Save</span>` after the last pending stage. |
| **D6** | **Bonus magnitude lacks text-shadow glow.** Mockup styles the `+5` with `text-shadow: 0 0 5px rgba(224, 86, 58, 0.4)` for incandescent feel. | 07 lines 165-167 | `.bonus-line strong` has color only. | Add `text-shadow` to `.bonus-line strong` (or hoist `.bonus-magnitude` class from mockup). Same treatment in `AdvantageCard.svelte` preview rendering if applicable. |

### Behavioral — spec'd interactions not wired (P2)

| # | Item | Spec ref | Current state | Target state |
|---|------|----------|---------------|--------------|
| **D7** | **No spotlight cross-fade.** Spec line 220: "spotlight's content swaps via a 140ms cross-fade." | 2026-05-17 line 218-220 | Instant swap on stage change. | `CardComposer` spotlight wraps the snippet render in a keyed transition: `{#key machine.stage}<div in:fade={{ duration: 140 }}>{@render spotlight()}</div>{/key}`. Eye stays at fixed screen coordinate; content fades. |
| **D8** | **Right-click = back not wired.** Spec line 349-369 spells out the carve-out for editable text and the behavior table. Shell accepts `onBack` prop but no caller passes it. | 2026-05-17 line 22, 349-369 | `oncontextmenu` suppresses native menu (with carve-out). Calls `onBack?.()` — but `onBack` is always undefined. | Each body's machine is the source of truth for "back." Two design options (decide in plan): (a) body exposes `onBack` via `$bindable` so the call site can wire it through to the shell; (b) body attaches its own `oncontextmenu` to the CardComposer root, calls `machine.retreat()` directly, and the shell drops its own contextmenu handler. **Option (b) is cleaner architecturally** (back-behavior owner = state-machine owner) but requires the shell to give up the global suppression. Plan picks one. |
| **D9** | **Empty / active slots aren't clickable to focus.** Mockup describes click-row-label = focus first field as Speed Pattern #7. | 2026-05-17 line 343 (pattern #7), 264 | `CardSlot.svelte:20` wires `onclick` only when `state === 'confirmed'`. Empty slots ignore clicks. | `CardSlot.svelte`: wire `onclick` for all three states. The `onClick` callback is already passed from the body. Confirmed = re-edit (unchanged); active = no-op (already active); empty = jump-to-stage (this is the new behavior). All three states clickable resolves D9 *and* enables "click row label = focus first field" for empty slots. |
| **D10** | **No `<kbd>`-style key labels in spotlight hints.** Mockup styles `Tab` / `Enter` / `Shift+Tab` etc. as pill-style key-cap glyphs inline. | 07 lines 224-229, 393-394, 498 | Plain inline text. | Add `.kbd` global class in `+layout.svelte` (or scoped where reused). Update spotlight hints to use `<span class="kbd">Tab</span>` for each key reference. |

---

## Workflow safeguards (to prevent recurrence)

The original plan's failure mode: implementer subagents executed tasks against the spec _text_, never against the mockup _images_. Lean-execution intentionally drops per-task spec-compliance review, so visual drift compounds silently. The remediation plan must close this gap.

1. **Every plan task lists the mockup file + line numbers it must match.** Not "see the mockup" — actual `.superpowers/brainstorm/156015-1779148305/content/07-card-shape-spotlight.html:358-378` style references in the task spec.
2. **Visual smoke-test in `npm run tauri dev` is the gate.** Each plan task includes an explicit step: "Open the create-custom dialog in a running tauri dev; navigate to the affected stage; visually confirm the change matches the mockup reference; capture a screenshot or describe the rendered state." This step runs _before_ `./scripts/verify.sh` and _before_ commit.
3. **Re-enable per-task spec-compliance reviewer for THIS remediation.** The standing lean-execution override in CLAUDE.md trades QA for speed. This remediation is precisely the cost of that trade — for one branch, restore the per-task reviewer to validate visual conformance. The reviewer's task: open the same mockup line range, compare to the implementation diff, confirm the change applies.
4. **End-of-branch holistic visual audit.** Before merge, walk every flow (Advantage / StatusTemplate / ModifierEffect) in `npm run tauri dev` against the mockups. Document any remaining gaps as v1.x backlog items rather than punting them silently.

These safeguards apply to the remediation plan only. Whether to make them the new default for vtmtools workflow overrides is a separate question for the user.

---

## Phasing (suggested)

Atomic per-deviation commits. Each commit has a clear scope and a corresponding mockup reference.

1. **F1 + F2 + F3 — Commit in-flight bugfixes.** Three sequential commits (height, reactivity, mouse-buttons). Keep separate for clean revert points. `./scripts/verify.sh` between each.
2. **D1 — Swap Description and Bonus rows in AdvantageBody.cardRows.** One-file change. Mockup ref: 07 lines 358-378. Verify by opening dialog → confirming name+type+level → checking that Description slot appears _above_ Bonus slot in the card preview.
3. **D9 — Click-empty-slot-to-focus.** `CardSlot.svelte` wires `onclick` for all three states. Verify by clicking the empty Description slot in card preview while editing Name → spotlight jumps to Description.
4. **D8 — Right-click = back.** Pick architecture (option a vs b per audit table). Implement. Verify by right-clicking inside spotlight zone → retreats one stage; right-clicking inside a text input → native Cut/Copy/Paste menu still appears.
5. **D2 — Step indicator in title bar.** `ItemBuilderShell` gets `stepIndicator?: string` prop. AdvantagesManager / StatusPaletteDock / call sites pass `"step 2 of 2 · new merit"` style strings.
6. **D3 — WIP strip in footer.** Add `--wip` token. Render `wip-strip` when `!valid`. Build the missing-fields list from `requiredFields - confirmedFields` (each body exports its required-fields set; this connects to the existing validity derivation).
7. **D4 — Cursor indicator on active slot.** `CardSlot.svelte` renders `▾` glyph when active. Blink animation. Verify: active slot shows pulsing border AND blinking glyph.
8. **D5 — Queue terminator "Save".** `CardComposer` appends final queue item. Color: `--accent-bright`.
9. **D6 — Magnitude text-shadow glow.** `AdvantageBody` bonus-line styling + `AdvantageCard` preview rendering both get the `text-shadow` rule.
10. **D7 — Spotlight cross-fade.** Wrap spotlight render in `{#key machine.stage}<div in:fade out:fade>...</div>{/key}`. 140ms in, 90ms out per existing transition timing.
11. **D10 — `.kbd` styling for spotlight hints.** Add the class once; wrap key references throughout `AdvantageBody.spotlight` and `StatusTemplateBody.spotlight`.
12. **End-of-branch holistic visual audit + single `code-review:code-review`.** Walk all three flows (Advantage / StatusTemplate / ModifierEffect) in `npm run tauri dev`. Document anything still off as v1.x backlog. Then run the standard code review against the remediation branch diff.

**Gates per task:**
- `./scripts/verify.sh` green before every commit (existing rule).
- Visual smoke-test in `npm run tauri dev` matching the cited mockup reference, before commit (new for this remediation).
- Per-task spec-compliance reviewer subagent runs after implementer commits (new for this remediation; reviewer reads mockup + diff, confirms match).

---

## Open questions

1. **Branch policy.** Work on `master` directly (we're solo, no CI), or create `templates-remediation` for the audit run? Recommendation: new branch for clean diff visibility against `master`.
2. **Step indicator text content.** Mockup uses `step 2 of 2 · new merit`. For AdvantagesManager (caller knows type, picker skipped), should it still say "step 2 of 2"? Or "new merit" alone? Spec line 119 says picker is "Conditional. Shown when caller does not provide `initialType`. Skipped when caller provides one" — so when picker is skipped, there is no step 1. Suggested text: just `"new merit"` / `"new template"` / `"edit advantage"` etc., dropping the step counter. Plan picks.
3. **Right-click design choice (option a vs b).** Audit lists both; one must be picked when writing the plan.
4. **`--wip` color token.** Mockup uses `#c9a064` (gold-warm). Add as standalone token in `+layout.svelte` next to existing `--field-*` and `--blood-*` tokens. Plan confirms.
5. **Whether to update CLAUDE.md `Workflow overrides` section** with the new per-remediation safeguards as a permanent or one-off pattern. User decision.

---

## What is intentionally NOT in scope

Items previously identified during the original spec/plan execution as v1.x deferrals stay deferred:
- DisciplineBody and ItemBody (preview-disabled stubs in TypePicker are unchanged)
- Library home page (no production caller for TypePicker in v1)
- Foundry-roller `auto_success` consumption (data is stored, helper code lands later)
- Drag-to-reorder, command-palette quick-add, mass-edit (none were in v1 spec)
- Latin pool expansion from 12 → ~50 entries (target stated in original spec; non-blocking)
- Brand-new `#fff` hex literal hardening (code review flagged 6 instances at low confidence)

These are documented in the original spec's "Out of scope" section and remain so.
