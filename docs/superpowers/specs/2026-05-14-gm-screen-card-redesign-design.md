# GM Screen — Modifier Card Redesign

**Date:** 2026-05-14
**Status:** Approved (brainstorm) — ready for plan
**Scope:** UI-only refactor of `ModifierCard.svelte` and adjacent components in the GM Screen. Data model, IPC surface, and DnD infrastructure are unchanged.

Related specs (do NOT re-litigate decisions in these):
- [`2026-05-03-gm-screen-design.md`](./2026-05-03-gm-screen-design.md) — original GM screen design; the carousel z-stack visual model is preserved as-is.
- [`2026-05-14-gm-screen-modifier-zones-and-dnd-design.md`](./2026-05-14-gm-screen-modifier-zones-and-dnd-design.md) — zone/DnD model. Drop targets and `dndStore` continue to apply.
- [`2026-05-13-gm-screen-live-data-priority-design.md`](./2026-05-13-gm-screen-live-data-priority-design.md) — read-through layering. Cards remain dumb renderers of the merged data.

---

## §1 What this is

`ModifierCard.svelte` has grown organically: name + cog + optional template subtitle + bonus list + conditionals badge + effect list + tag list, and a foot row that can hold up to six buttons (toggle, save-override, push, reset, delete, hide). The fixed `8rem` card height + `overflow: hidden` safety net trades two failure modes against each other: when content is small the foot is fine, when content overflows either the foot is clipped (recent bug) or the body is clipped (current state after partial fix).

This spec replaces the card's **anatomy and interaction model** with a content-aware design grounded in current desktop-card patterns (Material 3, Linear, Notion):

- Drag-and-drop pickup is restricted to a **handle bar** at the top.
- The **ON/OFF button is removed**; left-clicking the card body toggles `is_active`.
- All other foot buttons (save-override, push, reset, delete, hide) **move into a right-click context menu**, portal-rendered to escape the carousel's `overflow: hidden`.
- Content overflow is communicated by a **CSS-mask fade-out** + a small **`+N ⤢` overflow pill** anchored bottom-right.
- The pill click (or a context-menu **Open** action) opens a **native `<dialog>` overlay** showing the full card contents. The existing `ModifierEffectEditor` fields become the body of that overlay.
- Card width is **fluid via `clamp(9rem, 14cqi, 14rem)`** with a `container-type: inline-size` declaration on the row, so cards adapt to the vtmtools window size.

The carousel z-stack, neighbor-shift cascade, materialize-on-engagement, live data priority, and zone/DnD wiring are unchanged.

## §2 Composition — existing pieces this builds on

| Piece | Purpose | How the redesign uses it |
|---|---|---|
| `src/lib/components/gm-screen/ModifierCard.svelte` | Card renderer | Anatomy + interactions rewritten; data props unchanged. |
| `src/lib/components/gm-screen/CharacterRow.svelte` | Per-character row; renders cards inside `.modifier-row` | Adds `container-type: inline-size`; sets `--card-width` to a `clamp()` expression. |
| `src/lib/components/gm-screen/ModifierEffectEditor.svelte` | Effect/tag editor; currently rendered inside a custom popover anchored to the cog button | Moved inside the new `CardOverlay`; the editor fields stay the same. |
| `src/lib/components/dnd/DragSource.svelte` | Pointerdown → `dndStore.pickup` | Wraps the **handle bar only**, not the whole card. |
| `src/lib/components/dnd/DropMenu.svelte` | Portal-style menu at viewport coords, used by DnD actions | **Template** for the new `CardContextMenu` — mirror its position-fixed pattern and outside-click/Esc cleanup. |
| `src/tools/GmScreen.svelte` | Hosts the global pickup listeners | Existing `contextmenu` handler **cancels DnD pickup**. The new right-click flow only opens the menu when `dndStore.held === null`. |

## §3 Card anatomy

```
┌──────────────────────────┐  ← .modifier-card
│ ⠿            ●           │  ← .drag-handle  (h: 1.1rem, cursor: grab, owns pointerdown for DnD)
│                          │     - ⠿ at opacity 0.4 always (1.0 on hover) [research: NN/g — fast-paced tools need persistent grab affordances]
│                          │     - ●: active dot — color matches zone accent, glows when data-active="true"
│ Beautiful                │  ← .card-name (single-line, ellipsis on overflow)
│ ───                      │
│ +1 Stamina vs poison     │  ← .bonuses / .effects (one line each, ellipsis on overflow)
│ +2 Social pool networking│
│ #Social #Court #Polit…   │  ← .tags (single-line, ellipsis mid-string)
│ ░ mask fade ░░░░░░░░░░░  │  ← bottom 20% mask-image fade on .card-body
│              [+3 ⤢]      │  ← .overflow-pill (absolute, bottom-right; click → CardOverlay)
└──────────────────────────┘
```

Three vertical regions:

1. **Drag handle** — 1.1rem strip. Owns pointerdown via `<DragSource>`. Contains the grip icon (`⠿`) and the active-state dot. No other content.
2. **Card body** — flex:1, min-height:0, overflow hidden, with a CSS `mask-image: linear-gradient(180deg, black 80%, transparent)`. Holds the name, bonuses, effects, and tags. Owns the click-to-toggle gesture.
3. **Overflow pill** (conditional) — only rendered when content would exceed the card height. Anchored bottom-right of the card with `position: absolute; z-index: 2`. Click opens the overlay.

The foot row is **deleted entirely**. With ON/OFF gone, there is no remaining persistent affordance for the foot to hold.

## §4 Interaction model

| Gesture | Target | Result |
|---|---|---|
| `pointerdown` left button | `.drag-handle` | `<DragSource>` invokes `dndStore.pickup(...)`; existing DnD lifecycle takes over. |
| `pointerup` (click without drag) | `.card-body` (not pill, not handle) | Toggle `is_active` via `modifiers.setActive(id, !isActive)`. If the card is virtual, materialize first (existing `materialize()` helper). |
| `click` | `.overflow-pill` | `event.stopPropagation()`; open `CardOverlay` for this card. |
| `contextmenu` (right-click) | `.modifier-card`, only when `dndStore.held === null` | `event.preventDefault()`; open `CardContextMenu` at `event.clientX/Y`. |
| `Shift+F10` or `ContextMenu` key | focused card | Open `CardContextMenu` anchored to the card's top-right (a11y keyboard equivalent — per ARIA APG menu pattern). |
| `Escape` | active overlay or menu | Close. Focus returns to the card. |
| Backdrop click / `×` / Finish / Cancel | open overlay | Close. |

The handle being physically separate from the body **eliminates the click-vs-drag disambiguation problem** — no movement threshold, no `wasDragging` flag. The handle gets pointerdown, the body gets click; they cannot fire on the same gesture.

The existing `GmScreen.svelte` `contextmenu` handler (`l. 91-94`) which calls `dndStore.cancel()` continues to apply — right-click during a held pickup cancels the pickup; right-click idle opens the menu. The discriminator is `dndStore.held !== null`.

## §5 Responsive sizing

`.modifier-row` becomes a CSS container:

```css
.modifier-row {
  container-type: inline-size;
  container-name: modrow;
  --card-width: clamp(9rem, 14cqi, 14rem);
  /* …existing carousel transition tokens unchanged */
}
```

The existing carousel math (`--base-x`, `sibling-index()`, `sibling-count()`, neighbor-shift cascade) reads `--card-width` and continues to work — the variable is just no longer a fixed `9rem`.

Inside the card, container queries trim secondary content at narrow widths:

```css
@container modrow (max-width: 10rem) {
  .bonus-source { display: none; }  /* italic source label hides */
}
```

**Browser support:** Tauri 2 requires WebKitGTK 2.40+, where `container-type` (shipped 2.40, early 2023) and the `cqi` unit are supported. Same applies to macOS WebKit and Windows WebView2 on supported OS versions.

**Card height becomes `9.5rem`** (from `8rem`) — modest growth for breathing room. The character row's `.row` has no fixed height, so it auto-grows.

## §6 Overflow handling

### §6.1 The mask fade

`.card-body` carries:

```css
.card-body {
  flex: 1;
  min-height: 0;          /* respect flex allocation, never bleed */
  overflow: hidden;
  -webkit-mask-image: linear-gradient(180deg, black 80%, transparent);
          mask-image: linear-gradient(180deg, black 80%, transparent);
}
```

The mask fades the actual content (any color, any theme), not an overlay element. This is more elegant than gradient overlays and stays correct on theme switches (though the project is dark-only per ADR 0004).

### §6.2 The overflow pill

When does it render? **When body content would clip.** Two viable detection approaches:

- **Static heuristic** (simple): pre-compute a line count covering every element that consumes a body row, and render the pill when that count exceeds the threshold. Pre-compute in `cardEntries`.
- **Dynamic measurement** (precise): use `ResizeObserver` on the body, compare `scrollHeight > clientHeight`, set a `data-overflow` flag.

**Decision: static heuristic for v1.** The formula must count **every** body element that renders a line, not just `bonuses + effects + tags` — a card with origin subtitle + 2 bonuses + conditionals badge + 2 effects + tags occupies 8 lines and a too-narrow formula will silently miss real overflows:

```ts
function bodyLineCount(m: CharacterModifier, bonuses: FoundryItemBonus[], conditionalsSkipped: FoundryItemBonus[]): number {
  let n = 1; // name (always)
  if (m.originTemplateId != null) n += 1;       // origin subtitle
  n += bonuses.length;                          // bonus rows
  if (conditionalsSkipped.length > 0) n += 1;   // conditionals badge
  n += Math.max(m.effects.length, 1);           // effects (or "(no effect)" placeholder)
  if (m.tags.length > 0) n += 1;                // tag row
  return n;
}
// Render pill when bodyLineCount > 5 at the clamp() minimum 9rem width.
```

The `> 5` threshold reflects a 9.5rem card at the `clamp()` minimum — roughly five short single-line entries fit before the mask fades the last. Tune the constant if cards routinely under- or over-pill in practice. Move to dynamic `ResizeObserver`-based measurement only if the heuristic proves insufficient — overflow accuracy is a polish concern, not a correctness one (worst case: a card with hidden content has no pill, but the right-click → Open path still works).

The pill is a small `<button>` styled as a rounded chip:

```html
<button class="overflow-pill" onclick={openOverlay}>
  +{hiddenCount} <span class="glyph">⤢</span>
</button>
```

Click stops propagation so the underlying card body's click-to-toggle doesn't fire.

## §7 Active-state visual without ON/OFF

The removed ON/OFF button took away a fast-glance cue. Three-axis replacement:

1. **Border-color** flips to `--accent-bright` (existing behavior; unchanged).
2. **Background** flips to `--bg-active` (existing; unchanged).
3. **Glowing dot in the handle** — a `0.5rem × 0.5rem` circle that:
   - is transparent when inactive
   - fills with the zone accent (`--accent-bright` for character, `--accent-situational-bright` for situational) when active
   - has a `box-shadow: 0 0 6px <accent>` glow for motion-on-toggle

Together, these provide unambiguous visual state without occupying body real estate.

## §8 Component split

Three new components extracted from the current `ModifierCard.svelte`. **No new generic `<Card>` primitive** — per §11, the convention is documented in ARCHITECTURE.md so other card kinds (`CharacterCard`, `DyscrasiaCard`, palette templates) follow the same pattern without coupling.

### §8.1 `CardDragHandle.svelte`

```ts
interface Props {
  /** Active state — drives the dot's color/glow. */
  isActive: boolean;
  /** Zone — selects the dot's accent token. */
  zone: 'character' | 'situational';
}
```

Pure presentational. Renders the `⠿` grip + active dot inside a 1.1rem strip. Caller wraps it with `<DragSource>` and supplies the DnD source.

### §8.2 `CardContextMenu.svelte`

Portal-rendered, position-fixed at viewport coords. Mirrors the pattern in `DropMenu.svelte`.

```ts
export type CardAction =
  | {
      kind: 'item';
      /** Menu label */
      label: string;
      /** Optional shortcut hint shown right-aligned ("H", "Del", "Enter"). */
      shortcut?: string;
      /** Marks destructive actions for amber styling. */
      destructive?: boolean;
      /** Invoked on activation. */
      onActivate: () => void;
    }
  | { kind: 'divider' };

interface Props {
  open: boolean;
  /** Pointer coords for first render; adjusted in onMount if off-screen. */
  anchor: { x: number; y: number };
  actions: CardAction[];
  onClose: () => void;
}
```

ARIA: `role="menu"`, items use `role="menuitem"`. Keyboard: arrow keys cycle focus, Enter activates, Escape closes (returns focus to opener), Tab also closes.

Cleanup edges (mirroring `GmScreen.svelte` DnD cleanup):

- `pointerdown` outside the menu → close (use `pointerdown`, not `click`, to win the race with the opener)
- `Escape` keydown → close
- Window blur → close

Off-screen adjustment: after render, if `getBoundingClientRect().right > innerWidth`, reposition left; same for bottom edge.

### §8.3 `CardOverlay.svelte`

Native `<dialog>` element used with `.showModal()`. Gives focus trap, `Escape` handling, and **top-layer rendering that escapes the carousel's `overflow: hidden`** for free.

```ts
interface Props {
  /** Two-way binding controls open state. Caller calls bind:open. */
  open: boolean;
  /** Header title — typically the card name. */
  title: string;
  /** Snippet rendering the overlay body (bonus list, effects, editor fields, etc.). */
  children: Snippet;
  /** Snippet rendering the foot — buttons supplied by caller. */
  foot?: Snippet;
  /** Close hook called on backdrop click / × / Esc. */
  onClose: () => void;
}
```

ModifierEffectEditor's existing form-field markup becomes the `children` snippet content. The current popover-anchored-to-cog opening pattern is removed entirely — opening always goes through the overlay.

Backdrop styling: `::backdrop { background: rgba(0,0,0,0.55); backdrop-filter: blur(2px); }` per the mockup.

### §8.4 What `ModifierCard.svelte` becomes

```svelte
<div class="modifier-card" data-active={...} data-hidden={...} data-zone={...}>
  <DragSource source={dragSource} disabled={dragDisabled}>
    <CardDragHandle isActive={modifier.isActive} zone={modifier.zone} />
  </DragSource>

  <div class="card-body" onclick={handleBodyClick}>
    <p class="card-name">{modifier.name}{#if isVirtual}<span class="virtual-mark">*</span>{/if}</p>
    {#if originTemplateName}<p class="origin">from "{originTemplateName}"</p>{/if}
    {#if bonuses.length > 0}<div class="bonuses">…</div>{/if}
    {#if conditionalsSkipped.length > 0}<p class="conditionals-badge">…</p>{/if}
    <div class="effects">…</div>
    {#if modifier.tags.length > 0}<div class="tags" title={…}>…</div>{/if}
  </div>

  {#if hasOverflow}
    <button class="overflow-pill" onclick={openOverlay}>+{hiddenCount} ⤢</button>
  {/if}
</div>

<!-- Portaled out of the card so overflow:hidden doesn't clip them: -->
<CardContextMenu open={ctxOpen} anchor={ctxAnchor} actions={cardActions} onClose={…} />
<CardOverlay bind:open={overlayOpen} title={modifier.name} onClose={…}>
  <ModifierEffectEditor … />
</CardOverlay>
```

The `cardActions` list is built per-card:

```ts
let cardActions = $derived<CardAction[]>(([
  { kind: 'item', label: 'Open',                                        shortcut: 'Enter', onActivate: openOverlay },
  { kind: 'item', label: modifier.isActive ? 'Deactivate' : 'Activate', shortcut: 'Click', onActivate: onToggleActive },
  { kind: 'divider' },
  { kind: 'item', label: modifier.isHidden ? 'Unhide' : 'Hide',         shortcut: 'H',     onActivate: onHide },
  canPush          ? { kind: 'item', label: 'Push to Foundry',          shortcut: '↑',     onActivate: onPush }          : null,
  onSaveAsOverride ? { kind: 'item', label: 'Save as local override',                      onActivate: onSaveAsOverride } : null,
  { kind: 'divider' },
  canReset ? { kind: 'item', label: 'Reset card', shortcut: '↺',   destructive: true, onActivate: onReset }  : null,
  onDelete ? { kind: 'item', label: 'Delete',     shortcut: 'Del', destructive: true, onActivate: onDelete } : null,
] satisfies (CardAction | null)[]).filter((a): a is CardAction => a !== null));
```

## §9 Accessibility

### §9.1 Keyboard

- The card itself is focusable (`tabindex="0"`).
- `Space` / `Enter` on a focused card toggles active (matches click).
- `Shift+F10` and the dedicated `ContextMenu` key on a focused card open the context menu (ARIA APG menubar pattern).
- Within the menu: arrow keys cycle, Enter activates, Escape closes and returns focus to the card. Tab also closes.
- Within the overlay: native `<dialog>.showModal()` provides focus trap. Escape closes (browser default). **Manually save `document.activeElement` at open and call `.focus()` on it at close** — WebKitGTK's built-in focus restoration on `<dialog>.close()` has been historically inconsistent across versions, so do not rely on it. Save into a local variable in the component, restore on the `close` event listener and on backdrop-click close. This is a one-liner and removes a class of "focus disappears after closing the overlay" bugs.

### §9.2 Drag-and-drop keyboard equivalent (deferred)

WCAG 2.2 SC 2.5.7 requires a non-drag alternative for any drag-driven interaction. For a single-user offline GM tool there is **no compliance obligation**, so v1 ships without keyboard DnD. The context menu offers a partial fallback (a future "Move to character → …" item can open a picker; not in v1 scope).

**Spec deferral note:** if WCAG compliance is ever pursued, the Adobe React Spectrum pattern — Enter to pick up, arrow keys to navigate among drop targets, Enter to drop, Escape to cancel — is the canonical implementation.

## §10 Styling tokens

All tokens already exist in `src/routes/+layout.svelte`. No new tokens added:

| Element | Token |
|---|---|
| Card background (inactive) | `--bg-card` |
| Card background (active) | `--bg-active` |
| Card background (situational) | `--bg-situational-card` |
| Card border (inactive) | `--border-card` |
| Card border (active) | `--accent-bright` (character) / `--accent-situational-bright` (situational) |
| Drag handle text | `--text-muted` |
| Card name | `--text-primary` |
| Bonus value | `--accent-bright` / `--accent-situational-bright` |
| Effect / bonus text | `--text-secondary` |
| Tag text | `--text-muted` |
| Overflow pill bg | `--bg-raised`; hover `--accent` |
| Overlay bg | `--bg-card` |
| Overlay border | `--border-surface` |
| Context menu bg | `--bg-raised` |
| Context menu destructive | `--accent-amber` |
| Active dot glow | `--accent-bright` / `--accent-situational-bright` with `box-shadow` |

## §11 ARCHITECTURE.md additions

Add a new bullet to `ARCHITECTURE.md` §6 *Invariants* immediately after the existing styling bullets:

> - **Card pattern.** Card-shaped UI surfaces (modifier cards, status palette
>   templates, character cards, dyscrasia cards) follow a shared anatomy:
>   *drag handle (top, persistent grab affordance) → name → body content →
>   optional overflow pill (bottom-right)*. Overflow content opens a native
>   `<dialog>` overlay. Context menus and overlays must render outside the
>   card's stacking context (portal pattern in `DropMenu.svelte`, or native
>   `<dialog>`/Popover API) to escape `overflow: hidden` and `transform`
>   parents. CSS container queries (`container-type: inline-size`) drive
>   fluid card sizing via `clamp(min, Ncqi, max)` on the row.

A new entry to `ARCHITECTURE.md` §9 *Extensibility seams*:

> - **Add a card-shaped surface.** Follow the card pattern in §6: handle +
>   name + body + optional overflow pill, with menus/overlays portal-rendered.
>   Reuse `CardContextMenu` for right-click actions and `CardOverlay` for
>   the "open full" view; both are zero-dep wrappers (Svelte 5 runes,
>   `<dialog>` native, position-fixed portal). Per-domain content goes
>   inside the overlay body snippet.

No ADR needed — this is a refinement of existing styling conventions, not a new architectural decision.

## §12 Implementation gotchas

These flow directly from the research phase and must be documented in the plan so subagent implementers don't trip on them:

1. **`container-type` silently no-ops without declaration.** A `@container` rule does nothing unless an *ancestor* has `container-type: inline-size`. Put it on `.modifier-row`. **Do not** use `container-type: size` (would force the row to ignore intrinsic height and collapse). The card itself cannot read its own container size — `@container` rules target descendants of the container.

2. **`transform` parents break `position: fixed` for descendants.** Cards in the carousel use `transform: translateX(...)` for positioning. A `position: fixed` element rendered as a card descendant positions relative to the transformed card, **not the viewport**. This is why:
   - `CardContextMenu` must portal to a root-level container OR use the Popover API / `<dialog>`.
   - `CardOverlay` uses native `<dialog>.showModal()` which renders in the top layer outside all stacking contexts.

3. **Do NOT rely on `<dialog>` for automatic focus restoration.** The spec says the browser saves activeElement at `showModal()` and restores at `close()`; WebKitGTK has shipped inconsistent behavior here across versions, and we cannot control the user's GTK build. Always save manually at open and `.focus()` manually at close. See §9.1 — this is a one-liner per component, but skipping it produces "focus disappears" bugs that are fiddly to diagnose later.

4. **`<dialog>` backdrop click is detected by event-target check.** `e.target === dialogEl` when the click landed on the backdrop rather than on dialog content. Standard pattern: `onclick={e => e.target === overlayEl && overlayEl.close()}`.

5. **The existing `DragSource.svelte` uses `display: contents`** — the wrapper is layout-transparent. Wrapping `<CardDragHandle>` with `<DragSource>` does not introduce a new flex level. Good as-is.

6. **`pointerdown` for outside-menu detection, not `click`.** Click fires after pointerup; if the user pointerdowns on the menu opener while the menu is open, you want the close-on-outside to fire on pointerdown (so the menu closes before the new opener fires). Existing `GmScreen.svelte` DnD cleanup uses this pattern at `l. 113`.

7. **Sibling-index math is unchanged.** The carousel's `--base-x = (sibling-index() - 1) * card-width * (1 - overlap)` still works when `--card-width` is a `clamp()` expression — `calc()` flattens it. No layout-recompute trick is needed.

## §13 Out of scope

- **No data-model changes.** `CharacterModifier` shape, `ModifierEffect` kinds, `FoundryItemBonus` schema all unchanged.
- **No new IPC commands.** Command count stays where it is.
- **No carousel geometry change.** Z-stack centering, neighbor-shift cascade, sibling-index math preserved.
- **No new tools.** GM Screen is the only consumer of these components in v1. Other card-kind components are flagged for *future* convergence (per §11) but not refactored here.
- **No keyboard DnD.** Documented deferral (§9.2).
- **No drag-handle keyboard pickup.** Same deferral.
- **No dynamic overflow measurement.** Static heuristic only (§6.2); revisit if needed.

## §14 Migration notes for the plan

For the writing-plans phase, the natural task split is:

1. **Extract `CardDragHandle.svelte`** — pure markup move + active-dot styling. No behavior change yet.
2. **Add `CardOverlay.svelte`** as a `<dialog>` wrapper. Initially: wrap the existing `ModifierEffectEditor` and open via the existing cog button (validates the overlay works before removing the popover).
3. **Carousel smoke test (no commit).** Before going further, run `npm run tauri dev` against a real Foundry character with 3-5 advantage items. Verify the new handle + overlay coexist with the carousel z-stack, hover lift (`translateY(-0.75rem) translateZ(20px)`), neighbor-shift cascade, and the mask gradient under a `:hover` box-shadow. The brainstorm preview used `display: flex` not absolute positioning — what looked good on the flat grid may interact unexpectedly with the lifted/shifted cards. Adjust the styling before locking in subsequent tasks if the smoke test surfaces issues. No commit produced from this step.
4. **Switch the cog-opens-popover flow to open the overlay.** Delete the inline popover wrap from `CharacterRow.svelte`. Cog button still triggers — for one task only — so we can verify the overlay UX before the cog goes away.
5. **Add `CardContextMenu.svelte`** mirroring `DropMenu.svelte`. Initially unused.
6. **Wire right-click on `.modifier-card`** → open `CardContextMenu`. Build the `cardActions` list. Ensure `dndStore.held === null` is the discriminator.
7. **Move all foot-row actions into the context menu.** Delete the foot row entirely from `ModifierCard.svelte`. Delete the cog button. Tests/smoke: every action still works via right-click.
8. **Add left-click body → toggle active.** Use existing `handleToggleActive`. Wire `Space`/`Enter` keyboard equivalent on the focusable card root.
9. **Add the overflow pill + mask-image fade.** Compute `hasOverflow` via the full-formula static heuristic (§6.2). Pill button stops propagation.
10. **Switch `--card-width` to `clamp(9rem, 14cqi, 14rem)` + add `container-type: inline-size`** to `.modifier-row`. Bump card height `8rem → 9.5rem`.
11. **Add the `@container` rule** to hide `.bonus-source` at narrow widths.
12. **ARCHITECTURE.md** edits per §11.
13. **Verify and commit.** `./scripts/verify.sh` after each task before commit (per project CLAUDE.md rule).

**Clustering note.** Per project memory `feedback_atomic_cluster_commits`, tasks 1-2 (and possibly 1-4) leave the runtime in a visually hybrid state — new handle + old foot row + new overlay + still-active cog. None of this breaks the runtime (the hybrid is functional), so verify.sh stays green. If reviewing intermediate commits in isolation looks confusing, cluster tasks 1-4 into a single "scaffold new components + bridge cog→overlay" commit and tasks 7-9 into a single "remove foot row + wire new interactions" commit. The plan-writing phase makes the final call based on independent-vs-coupled judgment.

Plan tasks 1–10 are mostly mechanical UI refactors. No new tests required (per project CLAUDE.md "TDD on demand" override — `verify.sh` is the gate for refactor work). The single exception is task 5's context-menu wiring, which may warrant a Svelte-level smoke test if any disambiguation logic creeps in; default is no test.

## §15 References

- CSS container queries: [MDN container-type](https://developer.mozilla.org/en-US/docs/Web/CSS/container-type), [caniuse](https://caniuse.com/css-container-queries) (Safari 16 / WebKitGTK 2.40+).
- ARIA menu pattern: [W3C menubar APG](https://www.w3.org/WAI/ARIA/apg/patterns/menubar/).
- WCAG 2.2 SC 2.5.7 Dragging Movements: [w3.org](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html).
- Native `<dialog>` top-layer rendering: [MDN dialog](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog).
- Mask-image fade pattern: [PQINA](https://pqina.nl/blog/fade-out-overflow-using-css-mask-image/).
- Drag-and-drop click conflict resolution: [Atlassian Pragmatic DnD](https://atlassian.design/components/pragmatic-drag-and-drop/design-guidelines).

---

**Approved 2026-05-14.** Next step: invoke `superpowers:writing-plans` against this spec.
