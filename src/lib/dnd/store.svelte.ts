/**
 * DnD state-machine store. Singleton runes store. Owns the pickup-and-place
 * lifecycle plus the cursor position. UI components subscribe to derive
 * highlight / overlay rendering.
 *
 * State machine: idle → held → dropped|cancelled → idle.
 *
 * Lifecycle methods (called by DragSource/DropZone/DropMenu and the
 * global cleanup listeners installed by GmScreen.svelte):
 *   - pickup(source, originRect) — left-click on a card body
 *   - setTarget(target | null) — pointermove over a DropZone or off it
 *   - moveCursor(x, y) — pointermove updates the overlay
 *   - drop() — left-click on a DropZone; resolves actions and routes
 *   - cancel() — right-click, Esc, blur, click-outside, pointercancel
 *   - executeAction(action) — DropMenu picks one
 *
 * The store knows about `modifiers.setZone` because the only v1 action
 * is move-zone — keeps the wiring trivial. v2/v3 will inject more action
 * handlers; the dispatch table can grow inside `executeAction`.
 *
 * See spec §"DnD primitive" for the full state machine and cleanup edges.
 */
import { modifiers } from '../../store/modifiers.svelte';
import { getActionsFor } from './actions';
import type { Action, DragSource, DropTarget } from './types';

type HeldState = {
  source: DragSource;
  originRect: DOMRect;
  cursorX: number;
  cursorY: number;
  target: DropTarget | null;
  /** Computed action list for current (source, target). Refreshes on setTarget. */
  actions: Action[];
  /** When the held → menu transition has fired and the user is choosing. */
  menuOpenAt: { x: number; y: number } | null;
};

let _held = $state<HeldState | null>(null);

function refreshActions(): void {
  if (!_held) return;
  _held.actions = _held.target ? getActionsFor(_held.source, _held.target) : [];
}

async function applyAction(action: Action, source: DragSource): Promise<void> {
  // v1: only move-zone is in the matrix. v2/v3 will branch here.
  if (action.id === 'move-zone' && source.kind === 'free-mod') {
    await modifiers.setZone(source.mod.id, action.newZone);
    return;
  }
  // Defensive: unknown action / unsupported source — log and no-op.
  console.warn('[dnd] unhandled action', action.id, 'for source', source.kind);
}

export const dndStore = {
  get held() { return _held; },

  pickup(source: DragSource, originRect: DOMRect, startX: number, startY: number): void {
    _held = {
      source,
      originRect,
      cursorX: startX,
      cursorY: startY,
      target: null,
      actions: [],
      menuOpenAt: null,
    };
  },

  setTarget(target: DropTarget | null): void {
    if (!_held) return;
    _held.target = target;
    refreshActions();
  },

  moveCursor(x: number, y: number): void {
    if (!_held) return;
    _held.cursorX = x;
    _held.cursorY = y;
  },

  async drop(): Promise<void> {
    if (!_held) return;
    const { actions, source } = _held;
    if (actions.length === 0) {
      this.cancel();
      return;
    }
    if (actions.length === 1) {
      const snapshotSource = source;
      _held = null;
      try {
        await applyAction(actions[0], snapshotSource);
      } catch (err) {
        console.error('[dnd] action failed:', err);
      }
      return;
    }
    // ≥2 actions: open the menu at the current cursor location.
    _held.menuOpenAt = { x: _held.cursorX, y: _held.cursorY };
  },

  async executeAction(action: Action): Promise<void> {
    if (!_held) return;
    const snapshotSource = _held.source;
    _held = null;
    try {
      await applyAction(action, snapshotSource);
    } catch (err) {
      console.error('[dnd] action failed:', err);
    }
  },

  cancel(): void {
    _held = null;
  },
};
