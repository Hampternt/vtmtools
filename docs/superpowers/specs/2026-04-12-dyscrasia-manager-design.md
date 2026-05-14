# Dyscrasia Manager — Design Spec
_2026-04-12_

## Context

Dyscrasias are blood-memory bonuses a vampire gains when feeding on a mortal with Acute resonance. The SQLite database and all five backend Tauri commands (`list_dyscrasias`, `add_dyscrasia`, `update_dyscrasia`, `delete_dyscrasia`, `roll_random_dyscrasia`) already exist and are fully functional. Eight built-in dyscrasias are seeded (two per resonance type). This feature is entirely frontend work — wiring those commands into UI.

Two surfaces are needed:
1. **Dyscrasia Manager** — a standalone sidebar tool for browsing, adding, and editing dyscrasias.
2. **Acute Panel** — an inline section that appears inside the Resonance Roller when a roll lands on Acute, showing the relevant type's dyscrasias for GM selection.

---

## Surface 1: Dyscrasia Manager Tool

### Registration
Add one entry to `src/tools.ts`:
```ts
{ id: 'dyscrasias', label: 'Dyscrasias', icon: '🩸', component: () => import('./tools/DyscrasiaManager.svelte') }
```
The sidebar and lazy-loading are automatic.

### Layout
- **Search bar** at the top (full width), placeholder "Search by name or description…"
- **Filter chips** below the search bar, one per resonance type: All / Phlegmatic / Melancholy / Choleric / Sanguine. Default: All active. Multi-select: clicking two chips shows both types. Clicking All resets. Each chip has a distinct color when active (blue / purple / red / gold).
- **Results count** label ("Showing N dyscrasias") below chips.
- **Card grid** using CSS `column-width: ~200px` masonry layout. Cards size naturally to their content with no fixed height cap — only cards whose description exceeds ~15 lines (≈260px) show a "show more ▾" button with a gradient fade at the cut point. Clicking "show more" expands just that card via `max-height` transition.

### Card anatomy
```
[Resonance type — coloured label]
[Name — bold]
[Description — full text, gradient-clipped only if very long]
[show more ▾]  ← only if overflowing
────────────────────────────────
[Bonus text]         [built-in] or [✎ Edit] [✕]
```
- Built-in cards (`is_custom = false`): read-only, "built-in" badge in footer.
- Custom cards (`is_custom = true`): Edit and Delete buttons in footer.

### Add custom dyscrasia
"+ Add Custom" button in top-right of search row. Clicking it inserts an inline form card at the top of the grid with fields for: Resonance Type (select), Name, Description, Bonus. Save / Cancel buttons. On save, calls `add_dyscrasia` and the new card animates into the grid.

### Edit custom dyscrasia
Clicking "✎ Edit" on a custom card transforms that card in-place into an edit form (same fields, pre-filled). Save calls `update_dyscrasia`. Cancel reverts the card. Built-in cards cannot be edited (backend enforces this too).

### Delete custom dyscrasia
Clicking "✕" on a custom card removes it immediately (optimistic) and calls `delete_dyscrasia`. No confirmation dialog — the list is easily re-addable.

### Search behaviour
Client-side filter across name, description, and bonus fields. Debounced 110ms. Filters the already-loaded card set; no additional Tauri calls needed while typing.

### Filter + search animations
- **Exiting cards**: a fixed-position ghost clone fades out and scales down (`scale(0.74)`) at the card's original position while the real card is instantly removed from flow, triggering grid reflow.
- **Entering cards**: staggered fade-in with `scale(0.82) → scale(1)`, `cubic-bezier(0.22, 1, 0.36, 1)` easing (fast start, smooth deceleration). 40ms stagger between cards.
- **Chip press**: `transform: scale(0.87)` on `:active`.

### Data loading
On component mount, call `list_dyscrasias` for all four resonance types in parallel (four concurrent `invoke` calls). Cache results in component state. Re-fetch only after a successful add/update/delete.

---

## Surface 2: Acute Panel (Resonance Roller)

### Trigger
The panel appears below the result card in `Resonance.svelte` when `result.dyscrasia !== null` (i.e. the roll was Acute). It is not shown for non-Acute results.

### Result card change
When the roll is Acute, the Temperament row displays **"Acute"** (bright red, glowing) instead of "Intense". No separate "Acute: Yes" row — the dyscrasia panel appearing below is sufficient indication.

### Acute panel layout
```
┌─ Dyscrasias — [Resonance Type] ──────────── [🔍 filter…] [⟳ Re-roll] ─┐
│                                                                          │
│  [card]  [card — rolled highlight]  [card]                              │
│  [card]                                                                  │
│                                                                          │
├──────────────────────────────────────────────────────────────────────────┤
│  Auto-rolled: Hair-Trigger                          [Confirm]            │
└──────────────────────────────────────────────────────────────────────────┘
```

### Card states
- **Default**: standard card style.
- **Rolled** (auto-selected): red border glow, `background: #1a0808`, small "rolled" badge top-right.
- **Selected** (GM override): gold border glow, `background: #1a1206`, "selected ✓" badge top-right.
- Only one card can be rolled, only one can be selected. Selecting a card updates the summary line.

### Controls
- **Filter input**: client-side search within the displayed type's cards. Same debounce as Manager.
- **Re-roll button**: calls `roll_random_dyscrasia(resonance_type)`, updates the rolled card highlight and resets selection to the new roll.
- **Confirm button**: locks the selection. The Acute Panel hides entirely (conditional render removed). The confirmed `DyscrasiaEntry` is stored in the roll result state and displayed in the result card's existing dyscrasia name row.

### Data loading
When the Acute panel mounts (result becomes Acute), call `list_dyscrasias(resonance_type)` to fetch all cards for that type. The initial rolled dyscrasia is already in `result.dyscrasia` from the `roll_resonance` response — no extra call needed for the initial highlight.

---

## Components

| File | Purpose |
|------|---------|
| `src/tools/DyscrasiaManager.svelte` | New top-level tool component |
| `src/lib/components/DyscrasiaCard.svelte` | Single card — used in both Manager and Acute panel. Props: `entry: DyscrasiaEntry`, `mode: 'manager' \| 'acute'`, `state?: 'rolled' \| 'selected' \| null`. In `manager` mode: shows edit/delete buttons for custom entries, no rolled/selected states. In `acute` mode: hides edit/delete, shows rolled/selected highlight states, emits `select` event on click. |
| `src/lib/components/DyscrasiaForm.svelte` | Inline add/edit form card |
| `src/lib/components/AcutePanel.svelte` | Acute dyscrasia picker — imported by `Resonance.svelte` |

---

## Types

`DyscrasiaEntry` in `src/types.ts` is already correct:
```ts
export interface DyscrasiaEntry {
  id: number;
  resonanceType: string;
  name: string;
  description: string;
  bonus: string;
  isCustom: boolean;
}
```
No type changes needed.

---

## Tauri commands used

| Command | Called from | When |
|---------|-------------|------|
| `list_dyscrasias(resonance_type)` | Manager (×4 on mount), AcutePanel (×1 on mount) | Initial load |
| `add_dyscrasia(resonance_type, name, description, bonus)` | DyscrasiaForm | Save new |
| `update_dyscrasia(id, name, description, bonus)` | DyscrasiaForm | Save edit |
| `delete_dyscrasia(id)` | DyscrasiaCard | Delete button |
| `roll_random_dyscrasia(resonance_type)` | AcutePanel | Re-roll button |

---

## Verification

1. Run `npm run tauri dev` — open the app.
2. Click "Dyscrasias" in the sidebar — Manager loads, 8 built-in cards appear in masonry layout.
3. Click a type chip — irrelevant cards animate out, relevant ones animate in.
4. Type in search — cards filter in real time.
5. Click "+ Add Custom" — inline form appears; fill fields, save → new card appears in grid with correct resonance type.
6. Click "✎ Edit" on a custom card → form appears pre-filled; edit and save → card updates.
7. Click "✕" on a custom card → card disappears.
8. Switch to Resonance Roller, run a roll until Acute fires → Acute Panel appears below result card.
9. Temperament row reads "Acute" in bright red.
10. Dyscrasia cards for the correct type appear; rolled card has red glow.
11. Click a different card → it turns gold, summary updates.
12. Click Re-roll → rolled highlight moves to a new card.
13. Click Confirm → panel collapses, result shows confirmed dyscrasia.
14. Run `npm run check` — no type errors.
