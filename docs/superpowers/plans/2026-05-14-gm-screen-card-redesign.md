# GM Screen Modifier Card Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Project workflow overrides** (CLAUDE.md, take precedence over skill defaults):
> - **One implementer subagent per task; no per-task reviewer subagents.** A single `code-review:code-review` against the full branch diff runs after the last task.
> - **No new tests unless the task text says so.** This entire plan is UI refactor / component extraction / CSS rework — `verify.sh` (`npm run check` + `cargo check` + `cargo test` + frontend build) is the correctness gate, not new unit tests.
> - **Every commit task MUST run `./scripts/verify.sh` immediately before the commit.** No exceptions.

**Goal:** Refactor `ModifierCard.svelte` from a cog-popover-and-foot-row design into a handle/body/overlay design where left-click toggles, right-click opens a portal context menu, and overflow opens a native `<dialog>` overlay. Carousel z-stack, DnD infra, and data model are unchanged.

**Architecture:** Three new presentational components (`CardDragHandle`, `CardContextMenu`, `CardOverlay`) extract dedicated responsibilities. The existing `ModifierEffectEditor`'s fields move inside `CardOverlay`. CSS container queries make the card width fluid. Implementation is sequential — each task leaves the runtime functional (`verify.sh` green); some intermediate commits are visual hybrids (handle present + foot row still there), which is acceptable per `feedback_atomic_cluster_commits` since runtime is not broken.

**Tech Stack:** Svelte 5 (runes mode) + plain CSS. No new dependencies. Uses native `<dialog>`, CSS container queries (`container-type: inline-size`, `cqi` unit), and CSS `mask-image`. Tauri 2 (WebKitGTK 2.40+) supports all three. Existing pointer-event-based DnD infrastructure (`DragSource`, `dndStore`) is reused.

**Spec:** `docs/superpowers/specs/2026-05-14-gm-screen-card-redesign-design.md` (commit `3ebc10d`). Refer to it for the design rationale; the plan implements its §14 task split.

---

## File structure

### New files
- `src/lib/components/gm-screen/CardDragHandle.svelte` — top strip with `⠿` grip + active-state dot. Pure presentational.
- `src/lib/components/gm-screen/CardContextMenu.svelte` — portal-rendered right-click menu. Takes `CardAction[]` + viewport anchor coords.
- `src/lib/components/gm-screen/CardOverlay.svelte` — native `<dialog>` wrapper providing top-layer rendering, focus trap, Escape handling, backdrop close, manual focus-restoration.

### Modified files
- `src/lib/components/gm-screen/ModifierCard.svelte` — rewritten template (no foot row, no cog button, no inline popover anchor); CSS updated for handle + mask + pill.
- `src/lib/components/gm-screen/CharacterRow.svelte` — removes the `.popover-wrap` block; switches `--card-width` to `clamp(...)`; adds `container-type: inline-size`; bumps card height 8rem → 9.5rem.
- `ARCHITECTURE.md` — adds a card-pattern invariant to §6 and an extensibility seam to §9.

### Untouched (referenced but not modified)
- `src/lib/components/gm-screen/ModifierEffectEditor.svelte` — renders inside `CardOverlay`; its own markup unchanged.
- `src/lib/components/dnd/DragSource.svelte` — wraps `CardDragHandle` instead of `.card-body` after Task 7.
- `src/lib/components/dnd/DropMenu.svelte` — pattern template for `CardContextMenu`. Not modified.
- `src/tools/GmScreen.svelte` — existing `contextmenu` handler at lines 91-94 cancels DnD pickups; coexistence with new right-click flow is via `dndStore.held === null` discriminator inside `ModifierCard`.

---

## Tasks

### Task 1: Extract `CardDragHandle.svelte` (presentational only)

**Goal:** Create the new handle component and render it inside the existing `ModifierCard` ABOVE `.card-body`. Still wrapped by the existing `<DragSource>`, so pickup behavior is unchanged.

**Files:**
- Create: `src/lib/components/gm-screen/CardDragHandle.svelte`
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte:121-125`

- [ ] **Step 1: Create `CardDragHandle.svelte`**

Use the Write tool to create `src/lib/components/gm-screen/CardDragHandle.svelte` with this exact content:

```svelte
<script lang="ts">
  import type { ModifierZone } from '../../../types';

  interface Props {
    /** Drives the active-dot color/glow. */
    isActive: boolean;
    /** Zone — selects which accent token the dot uses when active. */
    zone: ModifierZone;
  }
  let { isActive, zone }: Props = $props();
</script>

<div class="drag-handle" data-zone={zone} aria-hidden="true">
  <span class="grip">⠿</span>
  <span class="active-dot" class:on={isActive}></span>
</div>

<style>
  .drag-handle {
    height: 1.1rem;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 0.55rem;
    cursor: grab;
    user-select: none;
    background: linear-gradient(180deg, rgba(255,255,255,0.025), transparent);
    border-bottom: 1px solid rgba(255,255,255,0.04);
  }
  .drag-handle:active { cursor: grabbing; }

  .grip {
    color: var(--text-muted);
    opacity: 0.4;
    font-size: 0.7rem;
    letter-spacing: -1px;
    line-height: 1;
    transition: opacity 120ms ease;
  }
  /* Reveal fully on card hover OR handle hover. The :global() escapes
     Svelte's per-component scoping for the ancestor selector. */
  :global(.modifier-card:hover) .grip,
  .drag-handle:hover .grip { opacity: 1; }

  .active-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: transparent;
    transition: background 200ms ease, box-shadow 200ms ease;
  }
  .active-dot.on {
    background: var(--accent-bright);
    box-shadow: 0 0 6px var(--accent-bright);
  }
  .drag-handle[data-zone="situational"] .active-dot.on {
    background: var(--accent-situational-bright);
    box-shadow: 0 0 6px var(--accent-situational-bright);
  }
</style>
```

- [ ] **Step 2: Render the handle inside `ModifierCard.svelte`**

Add the import alongside existing imports near the top of the `<script>` block. After the existing imports (`DragSource`, `DragSource as DragSourceType` from `../../dnd/types`), add:

```ts
import CardDragHandle from './CardDragHandle.svelte';
```

Then use the Edit tool to insert the handle inside the `<DragSource>` block. The current markup at `ModifierCard.svelte:121-125`:

```svelte
  <DragSource source={dragSource} disabled={dragDisabled}>
    <div class="card-body">
      {#if modifier.zone === 'situational'}
        <span class="zone-chip" aria-label="Situational modifier">Situational</span>
      {/if}
```

becomes:

```svelte
  <DragSource source={dragSource} disabled={dragDisabled}>
    <CardDragHandle isActive={modifier.isActive} zone={modifier.zone} />
    <div class="card-body">
      {#if modifier.zone === 'situational'}
        <span class="zone-chip" aria-label="Situational modifier">Situational</span>
      {/if}
```

- [ ] **Step 3: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. The handle appears at the top of every modifier card; pointerdown on it OR the body both still pick up (existing DragSource scope unchanged); active state shows the dot lit.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/gm-screen/CardDragHandle.svelte src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): extract CardDragHandle component

Pure presentational strip with grip glyph + active-state dot. Rendered
inside the existing DragSource wrap on ModifierCard, so DnD pickup
behavior is unchanged. Wired only as a visual affordance for now;
subsequent tasks will (a) move the DragSource wrap onto this component
specifically and (b) add the body click-to-toggle gesture.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Add `CardOverlay.svelte` (`<dialog>` wrapper)

**Goal:** Create the overlay component using native `<dialog>.showModal()`. Not yet wired — the cog still opens the existing popover. This task validates the overlay renders, traps focus, restores focus, and handles Escape / backdrop / × correctly.

**Files:**
- Create: `src/lib/components/gm-screen/CardOverlay.svelte`

- [ ] **Step 1: Create `CardOverlay.svelte`**

```svelte
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** Two-way binding controls open state. Caller does `bind:open={...}`. */
    open?: boolean;
    /** Header title — usually the card name. */
    title: string;
    /** Body content. */
    children: Snippet;
    /** Optional foot — buttons supplied by caller. If omitted, no foot rendered. */
    foot?: Snippet;
    /** Called after the dialog actually closes (post-close cleanup, e.g. clearing parent state). */
    onClose?: () => void;
  }

  let { open = $bindable(false), title, children, foot, onClose }: Props = $props();

  let dialogEl: HTMLDialogElement | undefined = $state();
  /** activeElement at open — restored manually on close because WebKitGTK's
   *  built-in <dialog>.close() focus restoration has shipped inconsistent
   *  behavior. See spec §9.1 / §12. */
  let openerFocus: HTMLElement | null = null;

  function close() {
    if (dialogEl?.open) dialogEl.close();
  }

  function handleClose() {
    open = false;
    onClose?.();
    if (openerFocus && openerFocus.isConnected) {
      openerFocus.focus();
    }
    openerFocus = null;
  }

  // Open/close the dialog when `open` prop changes.
  $effect(() => {
    if (!dialogEl) return;
    if (open && !dialogEl.open) {
      openerFocus = (document.activeElement as HTMLElement | null) ?? null;
      dialogEl.showModal();
    } else if (!open && dialogEl.open) {
      dialogEl.close();
    }
  });

  /** Backdrop click — event.target equals the dialog element only when the
   *  click landed on the backdrop, not on dialog content. Standard pattern. */
  function handleBackdropClick(e: MouseEvent) {
    if (e.target === dialogEl) close();
  }
</script>

<dialog
  bind:this={dialogEl}
  class="card-overlay"
  onclose={handleClose}
  onclick={handleBackdropClick}
>
  <header class="overlay-head">
    <h3>{title}</h3>
    <button
      type="button"
      class="overlay-close"
      aria-label="Close"
      onclick={close}
    >×</button>
  </header>
  <div class="overlay-body">
    {@render children()}
  </div>
  {#if foot}
    <footer class="overlay-foot">
      {@render foot()}
    </footer>
  {/if}
</dialog>

<style>
  .card-overlay {
    border: 1px solid var(--border-surface);
    background: var(--bg-card);
    color: var(--text-primary);
    border-radius: 0.75rem;
    padding: 0;
    max-width: 30rem;
    width: 90vw;
    box-shadow: 0 1rem 3rem rgba(0, 0, 0, 0.7);
  }
  .card-overlay::backdrop {
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
  }

  .overlay-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1.1rem;
    border-bottom: 1px solid var(--border-faint);
  }
  .overlay-head h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 1rem;
    font-weight: 500;
  }

  .overlay-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 1rem;
    cursor: pointer;
    padding: 0.2rem 0.5rem;
    border-radius: 4px;
  }
  .overlay-close:hover {
    color: var(--text-primary);
    background: var(--bg-raised);
  }

  .overlay-body {
    padding: 1rem 1.1rem 1.1rem;
  }

  .overlay-foot {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.75rem 1.1rem;
    border-top: 1px solid var(--border-faint);
  }
</style>
```

- [ ] **Step 2: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. No runtime change — the component exists but is unused.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/CardOverlay.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): add CardOverlay component (native <dialog>)

Wraps native <dialog>.showModal() with: title header + × close button,
default body slot, optional foot slot, backdrop-click close, Escape
close (browser default), and manual openerFocus save/restore (WebKitGTK
focus restoration is unreliable; spec §9.1, §12).

Unused in this commit. Task 3 wires the existing cog → popover flow to
open this overlay instead.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Replace cog-popover with `CardOverlay`

**Goal:** Repoint the cog button to open the new `<dialog>` overlay (with `ModifierEffectEditor` inside) instead of the inline anchored popover. The cog and editor still function; only their wrapping changes.

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte:525-535` (popover-wrap block)
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (script — `popoverPos` state, `openEditor` function)
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (style — `.popover-wrap` rule)

- [ ] **Step 1: Import `CardOverlay` and adjust editor state**

Add the import at the top of `CharacterRow.svelte`'s `<script>` block, next to the other component imports:

```ts
import CardOverlay from './CardOverlay.svelte';
```

Then replace the `popoverPos` state declaration. The current declaration (around line 38):

```ts
  let popoverPos = $state<{ left: number; top: number } | null>(null);
```

becomes (we no longer need anchor coords because the overlay centers itself):

```ts
  // popoverPos removed — CardOverlay is centered by the browser, no anchor needed.
```

Then simplify `openEditor` and `closeEditor`. Find the existing functions (around lines 336-351) and replace them with:

```ts
  function openEditor(e: CardEntry, _anchor: HTMLElement): void {
    editorTarget = e.kind === 'materialized'
      ? { kind: 'materialized', mod: e.mod }
      : { kind: 'virtual', virt: e.virt };
    editorOpen = true;
  }

  function closeEditor(): void {
    editorOpen = false;
    editorTarget = null;
  }
```

The `_anchor` parameter stays in the signature because `ModifierCard.onOpenEditor` still passes the cog element — until Task 8 removes the cog entirely. The `_` prefix marks it as intentionally unused.

- [ ] **Step 2: Replace the popover markup with `CardOverlay`**

Replace the `{#if editorOpen && editorTarget && popoverPos}` block (lines 525-535):

```svelte
  {#if editorOpen && editorTarget && popoverPos}
    <div class="popover-wrap" style="left: {popoverPos.left}px; top: {popoverPos.top}px;">
      <ModifierEffectEditor
        initialEffects={editorTarget.kind === 'materialized' ? editorTarget.mod.effects : []}
        initialTags={editorTarget.kind === 'materialized' ? editorTarget.mod.tags : []}
        onSave={saveEditor}
        onCancel={closeEditor}
        {character}
      />
    </div>
  {/if}
```

with:

```svelte
  {#if editorTarget}
    {@const target = editorTarget}
    <CardOverlay
      bind:open={editorOpen}
      title={target.kind === 'materialized' ? target.mod.name : target.virt.name}
      onClose={closeEditor}
    >
      <ModifierEffectEditor
        initialEffects={target.kind === 'materialized' ? target.mod.effects : []}
        initialTags={target.kind === 'materialized' ? target.mod.tags : []}
        onSave={saveEditor}
        onCancel={closeEditor}
        {character}
      />
    </CardOverlay>
  {/if}
```

- [ ] **Step 3: Remove the `.popover-wrap` CSS rule**

Delete the `.popover-wrap` rule from the `<style>` block (search for `.popover-wrap` — it's the small rule that sets `position: fixed; z-index: 1000;`):

```css
  .popover-wrap {
    /* Anchored to the cog via getBoundingClientRect() — viewport coords. */
    position: fixed;
    z-index: 1000;
  }
```

Delete those four lines entirely (rule plus comment).

- [ ] **Step 4: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Clicking the cog button on any card now opens a centered `<dialog>` overlay containing the existing `ModifierEffectEditor` form. Escape, ×, backdrop-click, and Cancel all close it. Save still calls `update_character_modifier`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
refactor(gm-screen): route cog-popover through CardOverlay

The cog button now opens the new <dialog>-based CardOverlay with
ModifierEffectEditor as its body content, replacing the previous
position:fixed popover anchored to the cog via getBoundingClientRect().

Benefits: native focus trap, top-layer rendering escapes the carousel's
overflow:hidden, no manual anchor math. The cog itself still exists
(removed in Task 8) so the editor remains reachable while subsequent
tasks add right-click and click-to-toggle.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Carousel smoke test (no commit, no code change)

**Goal:** Per advisor flag and spec §14 task 3, verify the new handle + overlay interact correctly with the actual carousel z-stack and hover lift (not the flat grid from the brainstorm preview).

**This task is USER-DRIVEN.** A subagent cannot run an interactive Tauri desktop window. The implementer asks the user to perform the smoke test and report back.

- [ ] **Step 1: Start the Tauri dev shell**

```bash
npm run tauri dev
```

Wait for the window to open.

- [ ] **Step 2: Open the GM Screen**

In the running app, click "🛡 GM Screen" in the sidebar. Connect a Foundry character with 3+ advantage items (or use a saved character with materialized modifiers).

- [ ] **Step 3: Inspect**

Verify the following behaviors look correct in the carousel context:

| Check | Expected |
|---|---|
| Handle is visible at the top of every modifier card | ⠿ glyph centered-left, active dot right side |
| Hovering a card lifts it (`translateY(-0.75rem) translateZ(20px)`) | Whole card lifts; handle lifts with it; neighbors slide laterally per the cascade |
| Active card | Border accent-bright, bg shifts to `--bg-active`, dot in handle glows |
| Situational-zone card | Green theme; dot glows green when active |
| Cog button still works | Click cog → centered overlay opens with editor fields |
| Overlay close | Esc / × / backdrop / Cancel all close cleanly; focus returns to the cog button |
| Reduced motion preference | Animations disabled per `@media (prefers-reduced-motion: reduce)` |

- [ ] **Step 4: Report**

If everything looks right, proceed to Task 5.

If anything looks off (handle z-fighting with neighbor-shift, hover lift clipping the handle, dot positioning weird at narrow zoom levels), pause the plan, document the issue, and either fix in Task 1-3 follow-up commits or amend the spec before continuing.

**No commit is produced from this task.**

---

### Task 5: Add `CardContextMenu.svelte` (portal-rendered)

**Goal:** Create the right-click menu component. Mirrors `DropMenu.svelte`'s position-fixed + outside-pointerdown pattern, but takes a `CardAction[]` list from the caller instead of reading `dndStore`.

**Files:**
- Create: `src/lib/components/gm-screen/CardContextMenu.svelte`

- [ ] **Step 1: Create `CardContextMenu.svelte`**

```svelte
<script lang="ts" module>
  /**
   * Discriminated union — either a clickable item or a visual divider.
   * Using `kind` instead of optional fields prevents "divider with onActivate"
   * type errors at call sites.
   */
  export type CardAction =
    | {
        kind: 'item';
        label: string;
        /** Optional shortcut hint shown right-aligned (e.g. "H", "Del", "Enter"). */
        shortcut?: string;
        /** Marks destructive actions for amber styling. */
        destructive?: boolean;
        onActivate: () => void;
      }
    | { kind: 'divider' };
</script>

<script lang="ts">
  interface Props {
    /** Whether the menu is visible. */
    open: boolean;
    /** Viewport coords for the menu's top-left. Adjusted in $effect if off-screen. */
    anchor: { x: number; y: number };
    actions: CardAction[];
    /** Called on outside-pointerdown, Escape, or item activation. */
    onClose: () => void;
  }

  let { open, anchor, actions, onClose }: Props = $props();

  let menuEl: HTMLDivElement | undefined = $state();
  let position = $state<{ left: number; top: number }>({ left: 0, top: 0 });

  function handleActivate(action: CardAction) {
    if (action.kind !== 'item') return;
    action.onActivate();
    onClose();
  }

  function handleOutsidePointerDown(e: PointerEvent) {
    if (!menuEl) return;
    if (!menuEl.contains(e.target as Node)) onClose();
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') onClose();
  }

  // Position the menu when it opens; adjust if it'd fall off the right/bottom.
  $effect(() => {
    if (!open) return;
    // Start at the requested coords, then measure and clamp after layout.
    position = { left: anchor.x, top: anchor.y };
    requestAnimationFrame(() => {
      if (!menuEl) return;
      const r = menuEl.getBoundingClientRect();
      const vw = window.innerWidth;
      const vh = window.innerHeight;
      let { left, top } = position;
      if (r.right > vw) left = Math.max(4, vw - r.width - 4);
      if (r.bottom > vh) top = Math.max(4, vh - r.height - 4);
      position = { left, top };
    });
  });

  // Outside-pointerdown + Esc listeners while open. Capture phase on pointerdown
  // to win the race with the next opener element's own pointerdown handler.
  $effect(() => {
    if (!open) return;
    document.addEventListener('pointerdown', handleOutsidePointerDown, true);
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('pointerdown', handleOutsidePointerDown, true);
      document.removeEventListener('keydown', handleKeyDown);
    };
  });
</script>

{#if open}
  <div
    bind:this={menuEl}
    class="card-context-menu"
    style="left: {position.left}px; top: {position.top}px;"
    role="menu"
  >
    {#each actions as action}
      {#if action.kind === 'divider'}
        <div class="divider" aria-hidden="true"></div>
      {:else}
        <button
          type="button"
          class="item"
          class:destructive={action.destructive}
          role="menuitem"
          onclick={() => handleActivate(action)}
        >
          <span class="label">{action.label}</span>
          {#if action.shortcut}
            <span class="shortcut">{action.shortcut}</span>
          {/if}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .card-context-menu {
    position: fixed;
    z-index: 2000;
    background: var(--bg-raised);
    border: 1px solid var(--border-faint);
    border-radius: 5px;
    box-shadow: 0 0.5rem 1.5rem var(--shadow-strong);
    padding: 0.25rem 0;
    min-width: 12rem;
    font-size: 0.8rem;
  }
  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    box-sizing: border-box;
    background: transparent;
    color: var(--text-primary);
    border: none;
    text-align: left;
    padding: 0.4rem 0.85rem;
    font-size: inherit;
    font-family: inherit;
    cursor: pointer;
    gap: 1rem;
  }
  .item:hover { background: var(--accent); color: var(--text-primary); }
  .item.destructive { color: var(--accent-amber); }
  .item.destructive:hover { background: rgba(204, 153, 34, 0.15); color: var(--accent-amber); }
  .shortcut {
    color: var(--text-muted);
    font-size: 0.7rem;
    font-family: monospace;
  }
  .item.destructive:hover .shortcut { color: var(--accent-amber); }
  .divider {
    height: 1px;
    background: var(--border-faint);
    margin: 0.25rem 0;
  }
</style>
```

- [ ] **Step 2: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Component compiles; unused.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/CardContextMenu.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): add CardContextMenu component

Portal-style (position:fixed) right-click menu mirroring the DropMenu.svelte
pattern. Discriminated CardAction union (item | divider) prevents
divider-with-onActivate type errors at call sites. Outside-pointerdown
in capture phase + Escape close. Off-screen clamping in a post-render
rAF so first paint can't overflow the viewport.

ARIA: role="menu" on container, role="menuitem" on items. Keyboard arrow
navigation deferred (single-user offline tool, see spec §9.1) — Escape
already works, mouse drives v1.

Unused in this commit. Task 6 wires right-click on .modifier-card to
open it.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Wire right-click → `CardContextMenu` (with single Open action)

**Goal:** Right-click on any modifier card opens the context menu, anchored at the cursor. The menu's only action for now is "Open" (which opens the overlay — same as the cog). Other actions are added in Task 8.

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (script + template)

- [ ] **Step 1: Import the menu component and its action type**

Near the top of `ModifierCard.svelte`'s `<script>` block, add:

```ts
import CardContextMenu, { type CardAction } from './CardContextMenu.svelte';
import { dndStore } from '../../dnd/store.svelte';
```

- [ ] **Step 2: Add context menu state and handlers**

After the existing `let cogEl: HTMLButtonElement | undefined = $state();` declaration, add:

```ts
  let ctxOpen = $state(false);
  let ctxAnchor = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  function handleContextMenu(e: MouseEvent) {
    // Right-click during a held DnD pickup is reserved for cancellation by
    // GmScreen.svelte's global listener. Do nothing here in that case.
    if (dndStore.held !== null) return;
    e.preventDefault();
    ctxAnchor = { x: e.clientX, y: e.clientY };
    ctxOpen = true;
  }

  function closeCtx() { ctxOpen = false; }

  let cardEl: HTMLDivElement | undefined = $state();

  let cardActions = $derived<CardAction[]>([
    {
      kind: 'item',
      label: 'Open',
      shortcut: 'Enter',
      onActivate: () => {
        // Reuse the existing onOpenEditor — caller passes the card element
        // as anchor (the overlay ignores it post-Task-3, but the signature
        // stays for compatibility).
        if (cardEl) onOpenEditor(cardEl);
      },
    },
  ]);
```

- [ ] **Step 3: Wire `oncontextmenu` and the menu to the card root**

Edit the existing `.modifier-card` root (around line 115). The current markup:

```svelte
<div
  class="modifier-card"
  data-active={modifier.isActive ? 'true' : 'false'}
  data-hidden={modifier.isHidden ? 'true' : 'false'}
  data-zone={modifier.zone}
>
```

becomes:

```svelte
<div
  bind:this={cardEl}
  class="modifier-card"
  data-active={modifier.isActive ? 'true' : 'false'}
  data-hidden={modifier.isHidden ? 'true' : 'false'}
  data-zone={modifier.zone}
  oncontextmenu={handleContextMenu}
>
```

Then at the end of the file's markup (after the closing `</div>` of `.foot`'s wrapper, but BEFORE the closing `</div>` of `.modifier-card`), add the menu instance. The current closing pattern is:

```svelte
    <button
      class="hide"
      title={modifier.isHidden ? 'Show card again' : 'Hide card'}
      aria-label={modifier.isHidden ? 'Show card again' : 'Hide card'}
      onclick={onHide}
    >{modifier.isHidden ? '+' : '×'}</button>
  </div>
</div>
```

becomes:

```svelte
    <button
      class="hide"
      title={modifier.isHidden ? 'Show card again' : 'Hide card'}
      aria-label={modifier.isHidden ? 'Show card again' : 'Hide card'}
      onclick={onHide}
    >{modifier.isHidden ? '+' : '×'}</button>
  </div>
  <CardContextMenu open={ctxOpen} anchor={ctxAnchor} actions={cardActions} onClose={closeCtx} />
</div>
```

- [ ] **Step 4: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Right-clicking any card opens the menu at the cursor; clicking "Open" opens the overlay; clicking outside the menu or pressing Escape closes it. Right-click during a DnD pickup still cancels the pickup (existing `GmScreen.svelte` handler at lines 91-94 wins because the card's `oncontextmenu` early-returns when `dndStore.held !== null`).

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): right-click modifier cards opens context menu

oncontextmenu on .modifier-card opens CardContextMenu at the cursor when
no DnD pickup is held. Single action for now ("Open") which routes
through the existing onOpenEditor → CardOverlay flow. Other foot-row
actions move into this menu in Task 8.

DnD-cancel-on-right-click in GmScreen.svelte is preserved: the card's
oncontextmenu early-returns when dndStore.held !== null, so the
global handler runs to cancel.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Move `<DragSource>` to handle + add body click-to-toggle

**Goal:** Restrict DnD pickup to the handle strip; left-clicking the body toggles `is_active`. Add Space/Enter keyboard equivalent on the focused card. The cog and foot row still exist (removed in Task 8) — only the click/drag attachment moves.

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (template + script)

- [ ] **Step 1: Restructure the DragSource wrap**

The current template around lines 121-176:

```svelte
  <DragSource source={dragSource} disabled={dragDisabled}>
    <CardDragHandle isActive={modifier.isActive} zone={modifier.zone} />
    <div class="card-body">
      {#if modifier.zone === 'situational'}
        <span class="zone-chip" aria-label="Situational modifier">Situational</span>
      {/if}
      <div class="head">
        <!-- ...name + cog... -->
      </div>
      <!-- ...origin / bonuses / conditionals / effects / tags... -->
    </div>
  </DragSource>
```

The `<DragSource>` now wraps ONLY the handle. The body sits outside it. Replace that block with:

```svelte
  <DragSource source={dragSource} disabled={dragDisabled}>
    <CardDragHandle isActive={modifier.isActive} zone={modifier.zone} />
  </DragSource>
  <div
    class="card-body"
    role="button"
    tabindex="0"
    onclick={handleBodyClick}
    onkeydown={handleBodyKey}
  >
    {#if modifier.zone === 'situational'}
      <span class="zone-chip" aria-label="Situational modifier">Situational</span>
    {/if}
    <div class="head">
      <span class="name" title={modifier.name}>
        {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}{#if showOverride}<span class="override-mark" title="Saved local override — this card's data comes from your saved copy, which supersedes the live Foundry read-through">*</span>{/if}
      </span>
      <button
        bind:this={cogEl}
        class="cog"
        title="Edit effects"
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => { e.stopPropagation(); cogEl && onOpenEditor(cogEl); }}
      >⚙</button>
    </div>
    {#if originTemplateName}
      <p class="origin">from "{originTemplateName}"</p>
    {/if}
    {#if bonuses.length > 0}
      <div class="bonuses">
        {#each bonuses as b}
          <p class="bonus" title={`${summarizeBonus(b)}${b.source ? ' — ' + b.source : ''}`}>
            <span class="bonus-value">{summarizeBonus(b)}</span>
            {#if b.source}<span class="bonus-source">{b.source}</span>{/if}
          </p>
        {/each}
      </div>
    {/if}
    {#if conditionalsSkipped.length > 0}
      <p
        class="conditionals-badge"
        title={conditionalsSkipped
          .map(b => `${b.source ?? '(unnamed)'} — ${b.activeWhen?.check ?? '?'}`)
          .join('\n')}
      >
        ({conditionalsSkipped.length} conditional{conditionalsSkipped.length === 1 ? '' : 's'})
      </p>
    {/if}
    <div class="effects">
      {#if modifier.effects.length === 0}
        <p class="no-effect">(no effect)</p>
      {:else}
        {#each modifier.effects as e}
          <p class="effect" title={summarize(e)}>{summarize(e)}</p>
        {/each}
      {/if}
    </div>
    {#if modifier.tags.length > 0}
      <div class="tags" title={modifier.tags.map(t => `#${t}`).join(' ')}>
        {#each modifier.tags as t, i}{#if i > 0}{' '}{/if}<span class="tag">#{t}</span>{/each}
      </div>
    {/if}
  </div>
```

Note: the **cog button's `onclick` now calls `e.stopPropagation()`** so clicking the cog doesn't bubble to the body toggle handler.

- [ ] **Step 2: Add the click and keyboard handlers**

In the `<script>` block, after the `closeCtx` function, add:

```ts
  function handleBodyClick(e: MouseEvent) {
    // Cog click stop-propagation guards against bubble in Task 7; the
    // overflow pill (Task 9) does the same. This handler runs only for
    // bare body clicks, which always toggle active.
    onToggleActive();
  }

  function handleBodyKey(e: KeyboardEvent) {
    if (e.key === ' ' || e.key === 'Enter') {
      e.preventDefault();
      onToggleActive();
    }
  }
```

- [ ] **Step 3: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Behavioral changes:
- Pointerdown on the handle → DnD pickup. Pointerdown on the body → no pickup.
- Click anywhere on the body (excluding cog) → toggles active.
- Click cog → opens overlay (`stopPropagation` blocks the toggle).
- Tab to a card, press Space/Enter → toggles active.
- Right-click anywhere → context menu still works.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): handle owns DnD, body owns click-to-toggle

DragSource now wraps CardDragHandle only — pointerdown on the body no
longer initiates pickup. Body becomes a role=button with tabindex=0;
left-click and Space/Enter toggle is_active. Cog's onclick gains
stopPropagation so editing doesn't double-fire as a toggle.

The foot row (ON/OFF, push, reset, save-override, delete, hide) is
still present — removed in Task 8 once the context menu hosts those
actions.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Migrate foot-row + cog actions into the context menu, delete the foot row and cog

**Goal:** All affordances that were in the foot row become menu items in `CardContextMenu`. The foot row's `<div class="foot">...</div>` block and its CSS are deleted. The cog button and `.head` wrapper are deleted; the card name moves into the body directly.

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (template + script + CSS)

- [ ] **Step 1: Expand `cardActions`**

Replace the existing minimal `cardActions` derived from Task 6:

```ts
  let cardActions = $derived<CardAction[]>([
    {
      kind: 'item',
      label: 'Open',
      shortcut: 'Enter',
      onActivate: () => {
        if (cardEl) onOpenEditor(cardEl);
      },
    },
  ]);
```

with the full action list:

```ts
  let cardActions = $derived<CardAction[]>(([
    {
      kind: 'item',
      label: 'Open',
      shortcut: 'Enter',
      onActivate: () => { if (cardEl) onOpenEditor(cardEl); },
    },
    {
      kind: 'item',
      label: modifier.isActive ? 'Deactivate' : 'Activate',
      shortcut: 'Click',
      onActivate: onToggleActive,
    },
    { kind: 'divider' },
    {
      kind: 'item',
      label: modifier.isHidden ? 'Unhide' : 'Hide',
      onActivate: onHide,
    },
    canPush ? {
      kind: 'item' as const,
      label: 'Push to Foundry',
      onActivate: () => onPush?.(),
    } : null,
    onSaveAsOverride ? {
      kind: 'item' as const,
      label: 'Save as local override',
      onActivate: () => onSaveAsOverride?.(),
    } : null,
    { kind: 'divider' as const },
    canReset ? {
      kind: 'item' as const,
      label: 'Reset card',
      destructive: true,
      onActivate: () => onReset?.(),
    } : null,
    onDelete ? {
      kind: 'item' as const,
      label: 'Delete',
      destructive: true,
      onActivate: () => onDelete?.(),
    } : null,
  ] satisfies (CardAction | null)[]).filter((a): a is CardAction => a !== null));
```

- [ ] **Step 2: Delete the foot row from the template**

The current foot block (lines 177-221 in the original — by now lines may have shifted, search for `<div class="foot">`):

```svelte
  <div class="foot">
    <button
      class="toggle"
      class:on={modifier.isActive}
      onclick={onToggleActive}
    >{modifier.isActive ? 'ON' : 'OFF'}</button>
    {#if onSaveAsOverride}
      <button class="save-override" ...>💾</button>
    {/if}
    {#if canPush}
      <button class="push" ...>↑</button>
    {/if}
    {#if canReset}
      <button class="reset" ...>↺</button>
    {/if}
    {#if onDelete}
      <button class="delete" ...>🗑</button>
    {/if}
    <button class="hide" ...>{modifier.isHidden ? '+' : '×'}</button>
  </div>
```

Delete the entire `<div class="foot">...</div>` block.

- [ ] **Step 3: Remove the cog and `.head` wrapper; inline the name into the body**

The current `.head` block inside the card body:

```svelte
    <div class="head">
      <span class="name" title={modifier.name}>
        {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}{#if showOverride}<span class="override-mark" title="Saved local override — this card's data comes from your saved copy, which supersedes the live Foundry read-through">*</span>{/if}
      </span>
      <button
        bind:this={cogEl}
        class="cog"
        title="Edit effects"
        onpointerdown={(e) => e.stopPropagation()}
        onclick={(e) => { e.stopPropagation(); cogEl && onOpenEditor(cogEl); }}
      >⚙</button>
    </div>
```

becomes (cog gone; name promoted to a direct child paragraph):

```svelte
    <p class="card-name" title={modifier.name}>
      {modifier.name}{#if isVirtual}<span class="virtual-mark" title="Not yet customized">*</span>{/if}{#if showOverride}<span class="override-mark" title="Saved local override — this card's data comes from your saved copy, which supersedes the live Foundry read-through">*</span>{/if}
    </p>
```

- [ ] **Step 4: Remove the now-unused `cogEl` state and clean up `onOpenEditor` usage**

In the `<script>` block, remove this line:

```ts
  let cogEl: HTMLButtonElement | undefined = $state();
```

The `Open` action in `cardActions` (Step 1) uses `cardEl` (added in Task 6), not `cogEl`, so the cog state is now dead.

Then update the `cardActions` "Open" item to pass `cardEl` as the anchor — already correct from Step 1. Confirm.

- [ ] **Step 5: Delete the foot-row CSS rules**

In `<style>`, delete every rule scoped to the now-removed elements. Search the file and delete:

- `.head { ... }`
- `.cog { ... }`
- `.modifier-card:hover .cog, .cog:focus { ... }`
- `.foot { ... }`
- `.toggle { ... }`, `.toggle.on { ... }`
- `.push { ... }`, hover/focus rules for `.push`
- `.save-override { ... }`, hover/focus rules
- `.reset { ... }`, hover/focus rules
- `.hide { ... }`, hover/focus rules
- `.modifier-card[data-hidden="true"] .hide { opacity: 1; }`
- `.delete { ... }`, hover/focus rules

Then add a new rule for `.card-name`:

```css
  .card-name {
    margin: 0 0 0.3rem;
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
```

- [ ] **Step 6: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. The card now shows: drag handle (top), name (below handle), bonuses/effects/tags (body). No foot row. No cog button. All actions accessible via right-click. The Hide affordance is renamed in-menu to "Unhide" when the card is currently hidden — the previous `+`/`×` toggle button is gone.

If a card is currently hidden, the GM unhides it via right-click → Unhide. Test this path explicitly: the hide affordance was previously the only persistent way out of hidden state; verify right-click is discoverable enough.

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): migrate foot-row + cog actions into context menu

The .foot row and its six buttons (ON/OFF, push, save-override, reset,
delete, hide), plus the cog button and its containing .head wrapper,
are deleted. Their actions live in CardContextMenu now: Open, Activate/
Deactivate, Hide/Unhide, Push to Foundry, Save as local override, Reset
card, Delete.

Card name promoted to a direct .card-name <p> at the top of the body
(no more name/cog flex pair). Roughly 90 lines of CSS removed.

Behavioral change: all card-management gestures now go through left-click
(toggle) or right-click (everything else). Drag from handle. The cog as
an idiom is retired in favor of the same right-click menu.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Add overflow pill + CSS mask-image fade

**Goal:** Cards whose content exceeds the card height get a small `+N ⤢` pill at the bottom-right; the card body fades out via `mask-image` to signal there's more. Clicking the pill opens `CardOverlay`.

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (script + template + CSS)

- [ ] **Step 1: Compute `hasOverflow` and `hiddenCount` in `<script>`**

Add this derived helper near the other derived values (after `dragSource` / `dragDisabled`):

```ts
  // Static overflow heuristic (spec §6.2) — count every body element that
  // renders a line, including the conditional ones. Tune the threshold if
  // cards routinely under- or over-pill in practice.
  let bodyLineCount = $derived.by(() => {
    let n = 1; // .card-name
    if (originTemplateName) n += 1;
    n += bonuses.length;
    if (conditionalsSkipped.length > 0) n += 1;
    n += Math.max(modifier.effects.length, 1); // effects (or "(no effect)" placeholder)
    if (modifier.tags.length > 0) n += 1;
    return n;
  });
  const OVERFLOW_THRESHOLD = 5;
  let hasOverflow = $derived(bodyLineCount > OVERFLOW_THRESHOLD);
  let hiddenCount = $derived(Math.max(0, bodyLineCount - OVERFLOW_THRESHOLD));
```

- [ ] **Step 2: Add the pill markup**

After the `.card-body` closing `</div>` and before the `<CardContextMenu />` instance, add:

```svelte
  {#if hasOverflow}
    <button
      type="button"
      class="overflow-pill"
      title="Open full card"
      aria-label="Open full card"
      onclick={(e) => { e.stopPropagation(); if (cardEl) onOpenEditor(cardEl); }}
    >+{hiddenCount} <span class="glyph">⤢</span></button>
  {/if}
```

The `stopPropagation` prevents the underlying `.card-body` click-to-toggle.

- [ ] **Step 3: Add CSS for pill + mask fade**

Inside `<style>`, locate the existing `.card-body` rule (the one with `min-height: 0; overflow: hidden`). Add `mask-image` to it:

```css
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    -webkit-mask-image: linear-gradient(180deg, black 80%, transparent);
            mask-image: linear-gradient(180deg, black 80%, transparent);
    cursor: pointer;
  }
```

Then add the pill rule (near the bottom of the style block, before the `@media (prefers-reduced-motion)`):

```css
  .overflow-pill {
    position: absolute;
    bottom: 0.3rem;
    right: 0.5rem;
    background: var(--bg-raised);
    border: 1px solid var(--border-faint);
    color: var(--text-primary);
    font-size: 0.6rem;
    padding: 0.05rem 0.45rem;
    border-radius: 999px;
    cursor: pointer;
    z-index: 2;
    transition: background 120ms ease, border-color 120ms ease;
    font-family: inherit;
  }
  .overflow-pill:hover {
    background: var(--accent);
    border-color: var(--accent-bright);
  }
  .overflow-pill .glyph {
    margin-left: 0.2rem;
    opacity: 0.7;
  }
```

- [ ] **Step 4: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Cards with > 5 body lines render the pill bottom-right; the body content fades at the bottom (mask gradient). Clicking the pill opens the overlay. Light-content cards (≤ 5 lines) render no pill and no visible fade (the mask only affects content that reaches the fade band).

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): overflow pill + mask-image fade

Static heuristic (spec §6.2): bodyLineCount counts name + origin +
bonuses + conditionals badge + effects + tags; threshold > 5 at the
clamp() minimum width. A +N ⤢ pill at bottom-right opens the overlay
when content overflows; the .card-body fades via mask-image so the
clipped lines visibly trail off instead of cutting hard.

The pill's onclick stops propagation so the underlying body's click-
to-toggle doesn't fire.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Fluid card width via `clamp()` + container query + height bump

**Goal:** `--card-width` scales fluidly between 9rem and 14rem with row width via container queries. Card height grows from 8rem to 9.5rem for breathing room.

**Files:**
- Modify: `src/lib/components/gm-screen/CharacterRow.svelte` (CSS — `.modifier-row` rule)

- [ ] **Step 1: Replace the fixed `--card-width` and add `container-type`**

Find the existing `.modifier-row` rule (around line 611). The current rule (relevant lines only):

```css
  .modifier-row {
    --card-trans-duration: 600ms;
    --card-trans-easing: linear( /* ...long easing string... */ );
    --card-width: 9rem;
    --card-overlap: 0.55;
    --card-shift-delta: 0.5rem;
    /* --cards is set inline via the style prop above to drive the z-stack centering math. */
    position: relative;
    height: 8rem;
    perspective: 800px;
  }
```

becomes:

```css
  .modifier-row {
    container-type: inline-size;
    container-name: modrow;
    --card-trans-duration: 600ms;
    --card-trans-easing: linear( /* ...long easing string, leave verbatim... */ );
    --card-width: clamp(9rem, 14cqi, 14rem);
    --card-overlap: 0.55;
    --card-shift-delta: 0.5rem;
    /* --cards is set inline via the style prop above to drive the z-stack centering math. */
    position: relative;
    height: 9.5rem;
    perspective: 800px;
  }
```

**Important:** do not delete the `--card-trans-easing: linear(...)` value. Keep the existing long curve verbatim — only the surrounding rules change.

- [ ] **Step 2: Verify the carousel math still works**

The carousel uses `--base-x = (sibling-index() - 1) * var(--card-width) * (1 - var(--card-overlap))` in `ModifierCard.svelte:229`. `calc()` flattens a `clamp()` expression to its resolved value, so the math works without changes. No edits needed in `ModifierCard.svelte` for this step.

- [ ] **Step 3: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Cards now scale with the GM Screen's row width. At small window widths, cards stay at 9rem minimum. At wider windows, they grow toward 14rem max.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/gm-screen/CharacterRow.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): fluid card width via container query + height bump

.modifier-row declares container-type: inline-size; --card-width becomes
clamp(9rem, 14cqi, 14rem) so cards fluidly resize with the GM Screen
row width. Carousel sibling-index math is unaffected — calc() flattens
the clamp() at render time.

Card height bumps from 8rem to 9.5rem for breathing room around the
new handle + mask-fade.

WebKitGTK 2.40+ (Tauri 2 minimum) supports container queries and the
cqi unit; no fallback needed.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: Add narrow-width `@container` rule

**Goal:** At very narrow card widths, hide the italic bonus-source label so the value remains legible.

**Files:**
- Modify: `src/lib/components/gm-screen/ModifierCard.svelte` (CSS only)

- [ ] **Step 1: Add the `@container` rule**

In `ModifierCard.svelte`'s `<style>` block, after the existing `.bonus-source` rule, add:

```css
  /* Narrow-width adaptation. The .modifier-row declares container-type
     in CharacterRow.svelte; this rule fires when the card's container
     (the row) is ≤ 60rem wide, which is when card-width is at or near
     the clamp() floor. */
  @container modrow (max-width: 60rem) {
    .bonus-source { display: none; }
  }
```

Note: container queries on the `.bonus-source` selector apply when the **ancestor** with `container-type` (the `.modifier-row`) matches the query, NOT when the card itself matches. The `60rem` threshold corresponds to roughly 4 cards at the clamp() floor (`4 × 9rem ≈ 36rem` + gaps + paddings); tune if the row width breakpoint feels off in practice.

- [ ] **Step 2: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Resize the app narrow enough that the GM Screen row width drops below 60rem — bonus source italics disappear; bonus value stays visible.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/gm-screen/ModifierCard.svelte
git commit -m "$(cat <<'EOF'
feat(gm-screen): hide bonus-source at narrow row widths

@container query keyed to the .modifier-row (which declared
container-type: inline-size in Task 10) hides .bonus-source italics
when the row is ≤ 60rem wide. At that width the cards are at or near
the clamp() floor; preserving the bonus value matters more than the
source label.

The 60rem threshold is tunable; it corresponds to roughly 4 cards at
9rem + gaps. Revisit if cards feel cramped at typical playthrough
widths.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: Update `ARCHITECTURE.md`

**Goal:** Document the card pattern as an invariant (§6) and as an extensibility seam (§9) per spec §11.

**Files:**
- Modify: `ARCHITECTURE.md`

- [ ] **Step 1: Add card-pattern invariant under §6**

Open `ARCHITECTURE.md` and find the §6 *Invariants* section. After the existing CSS-tokens bullet ("CSS colors use tokens from `:global(:root)`..."), add a new bullet:

```markdown
- **Card pattern.** Card-shaped UI surfaces (modifier cards, status
  palette templates, character cards, dyscrasia cards) follow a shared
  anatomy: *drag handle (top, persistent grab affordance) → name →
  body content → optional overflow pill (bottom-right)*. Overflow
  content opens a native `<dialog>` overlay. Context menus and
  overlays must render outside the card's stacking context (portal
  pattern in `DropMenu.svelte`, or native `<dialog>`/Popover API) to
  escape `overflow: hidden` and `transform` parents. CSS container
  queries (`container-type: inline-size`) drive fluid card sizing
  via `clamp(min, Ncqi, max)` on the row. See
  [`docs/superpowers/specs/2026-05-14-gm-screen-card-redesign-design.md`](docs/superpowers/specs/2026-05-14-gm-screen-card-redesign-design.md).
```

- [ ] **Step 2: Add the extensibility seam under §9**

In `ARCHITECTURE.md` §9 *Extensibility seams*, after the last existing bullet (the bridge source one ending with `[ADR 0006](docs/adr/0006-bridge-source-generalization.md)`), add:

```markdown
- **Add a card-shaped surface.** Follow the card pattern in §6:
  handle + name + body + optional overflow pill, with menus and
  overlays portal-rendered. Reuse `CardContextMenu` for right-click
  actions and `CardOverlay` for the "open full" view; both are
  zero-dep wrappers (Svelte 5 runes, `<dialog>` native, position-
  fixed portal). Per-domain content goes inside the overlay body
  snippet.
```

- [ ] **Step 3: Run verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`. Docs change only.

- [ ] **Step 4: Commit**

```bash
git add ARCHITECTURE.md
git commit -m "$(cat <<'EOF'
docs(architecture): card pattern invariant + extensibility seam

§6: card-shaped UI surfaces follow a shared anatomy (handle → name →
body → optional overflow pill); menus and overlays must portal-render
out of the card's stacking context to escape overflow:hidden and
transform parents; container queries drive fluid sizing.

§9: add a "card-shaped surface" extensibility seam pointing at
CardContextMenu + CardOverlay for reuse.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 13: Final verification + branch review handoff

**Goal:** Run the aggregate gate once more from a clean tree, optionally smoke-test in Tauri dev, and queue the single end-of-branch code review per CLAUDE.md's lean-plan-execution override.

- [ ] **Step 1: Confirm clean tree**

```bash
git status --short
```

Expected: empty output (no uncommitted changes). If the in-progress CSS from earlier (the ones described in the spec amendments commit message under "still parked in working tree") is somehow staged, unstage them — they are subsumed by Tasks 7-11 and committing them now would create a redundant diff.

- [ ] **Step 2: Final aggregate verify**

```bash
./scripts/verify.sh
```

Expected: `verify: all checks passed`.

- [ ] **Step 3 (optional but recommended): Tauri-dev smoke test**

```bash
npm run tauri dev
```

In the running window:

- Open GM Screen with a Foundry character having 3+ advantage merits (or a saved character with materialized modifiers).
- For each of the 10 visual properties listed in Task 4 Step 3, plus the new ones from Tasks 8-11, confirm:
  - Drag handle visible always at 0.4 opacity, full opacity on card hover
  - Active state shows: accent border, bg shift, glowing dot in handle
  - Hidden state shows: 0.45 opacity, desaturated
  - Right-click opens menu at cursor; menu closes on outside click / Esc
  - Menu actions all work: Open / Activate-Deactivate / Hide-Unhide / Push / Save as override / Reset / Delete
  - Left-click body toggles; cog is GONE (no cog wheel anywhere)
  - Cards with many bonuses+effects show the `+N ⤢` pill and the mask fade
  - Pill click opens overlay; Esc / × / backdrop / Finish close it
  - Resize window: cards scale between 9rem and 14rem
  - Narrow window: bonus-source italic labels disappear

If anything fails, fix in a follow-up commit on the same branch; do not amend prior commits.

- [ ] **Step 4: Run a full-branch code review**

Per the project CLAUDE.md "Lean plan execution" override, a single review at end-of-branch replaces per-task reviewers. From an interactive session (NOT from a subagent — `/ultrareview` and `code-review:code-review` are user-billed), the user invokes:

```
/ultrareview
```

or for an existing PR:

```
/ultrareview <PR-number>
```

Subagent implementers stop here — the user runs the review.

- [ ] **Step 5: (User decision) — merge, PR, or further iterate**

After the review feedback resolves, the user merges to master (or whichever branch this work lives on) and optionally invokes `superpowers:finishing-a-development-branch` for the handoff workflow.

---

## Self-review

Run through the spec section-by-section against the plan:

| Spec section | Plan task(s) | Notes |
|---|---|---|
| §3 Card anatomy | 1 (handle), 7 (DragSource → handle), 8 (.card-name promoted), 9 (mask + pill), 10 (height bump) | All anatomy elements covered. |
| §4 Interaction model | 6 (right-click), 7 (left-click body, Space/Enter), 9 (pill click) | `Shift+F10` / ContextMenu key keyboard equivalent: NOT covered in v1. Documented as deferred in spec §9.1. Not adding a task. |
| §5 Responsive sizing | 10 (clamp + container-type), 11 (@container rule) | Covered. |
| §6 Overflow handling | 9 (mask + pill + heuristic) | Full formula used. |
| §7 Active-state visual | 1 (active dot in handle); 8 (foot ON/OFF removed) | Border + bg shift were already in existing code; unchanged. |
| §8.1 CardDragHandle | 1 | Full file. |
| §8.2 CardContextMenu | 5 | Full file with discriminated CardAction. |
| §8.3 CardOverlay | 2 (component) + 3 (wired to editor) | Full file. |
| §9 Accessibility | 7 (keyboard Space/Enter on focused body) | Shift+F10 / context-menu key + keyboard DnD deferred per spec §9.2. |
| §10 Styling tokens | All tasks use tokens; no hex. | Verified token names against `+layout.svelte`. |
| §11 ARCHITECTURE updates | 12 | Both §6 and §9 bullets added. |
| §12 Implementation gotchas | Task 2 (focus restoration manual), Task 5 (portal + outside-pointerdown capture phase), Task 6 (dndStore.held discriminator) | Each gotcha addressed in the relevant task. |
| §13 Out of scope | n/a | Plan respects scope — no data-model, IPC, carousel-math, or new-tool changes. |
| §14 Migration notes | Tasks 1-13 mirror the migration ordering (with smoke test inserted as Task 4 per spec amendment). | Verified 1:1. |

**Placeholder scan:**
- No TBD / TODO / "implement later" tokens.
- No "Similar to Task N" references — each task's code is shown in full or as a complete edit.
- No "add appropriate error handling" — error handling for this refactor is "let it bubble" since none of these are user-facing failure cases.
- Every commit task ends with `./scripts/verify.sh` followed by `git commit` per CLAUDE.md hard rule.

**Type consistency:**
- `CardAction` declared as discriminated union in Task 5; all consumers (Task 6 minimal list, Task 8 full list) use `kind: 'item'` / `kind: 'divider'` consistently.
- `ModifierZone` import in Task 1 matches the type exported from `src/types.ts` (verified via existing code in `ModifierCard.svelte`).
- `Snippet` import in Task 2 uses Svelte 5's standard `import type { Snippet } from 'svelte'`.

**Cross-task contract check:**
- Task 1 introduces `CardDragHandle` rendered inside `<DragSource>`; Task 7 moves the `<DragSource>` wrap to surround only `<CardDragHandle>`. No name mismatch.
- Task 2 creates `CardOverlay` with `bind:open`; Task 3 uses `bind:open={editorOpen}`. Matches.
- Task 5 exports `CardAction` from `CardContextMenu.svelte` module-scope; Task 6 imports `type CardAction` from same path. Matches.
- Task 6 introduces `cardEl` via `bind:this`; Task 8's expanded `cardActions` Open action uses `cardEl`. Matches.
- Task 9's `hasOverflow` and `hiddenCount` use `bonuses` (prop) and `conditionalsSkipped` (prop) — these are passed from `CharacterRow` and exist in `ModifierCard`'s `Props` interface. Verified.

Plan is complete.

---

## Execution choice

Two execution options:

**1. Subagent-Driven (recommended for this plan)** — Dispatch one fresh implementer subagent per task with the task's full text + scene-setting context. After the implementer commits, run `./scripts/verify.sh` (already in each task), then move to the next task. No per-task reviewer subagents (CLAUDE.md override). A single `/ultrareview` or `code-review:code-review` runs against the full branch diff after Task 12, before merge.

**2. Inline Execution** — Execute all 13 tasks in this conversation using `superpowers:executing-plans`. Same task-by-task verification. Use this if the tasks are tightly coupled enough that you want continuous human-in-the-loop visibility rather than batch handoffs.

The CLAUDE.md "Lean plan execution" override leans option 1 for plans with independent tasks. These tasks are largely independent (each one's verify gate is green at commit), so option 1 is the recommended choice — though the smoke-test gates in Tasks 4 and 13 Step 3 are user-driven and pause the loop regardless.
