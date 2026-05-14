# Domains Manager UI Design

**Date:** 2026-04-19
**Status:** Draft
**Scope:** Frontend (SvelteKit) v1 of the Domains Manager tool. Consumes the backend contract locked in `2026-04-18-domains-manager-design.md` — no schema or command changes are in scope.

---

## Overview

Ship the first user-facing surface for the Domains Manager. The tool lets the Storyteller browse, create, edit, and link nodes in a chronicle graph using a three-column layout: a sidebar tree driven by `contains` edges, a detail pane for the focused node, and an edges panel for non-`contains` relationships. Serves both between-session worldbuilding (heavy CRUD) and live-session reference (fast find, clean read-mode) without privileging one mode.

Twenty Tauri commands and 44 unit tests are already in place. This phase wires them to a Svelte UI, adds a typed-property editor, introduces a chronicle-scoped UI state store, and registers the tool in the sidebar.

---

## Scope

### In Scope (v1)

- New tool entry in `src/tools.ts` (`domains`, label "Domains", component lazy-loaded).
- Top-level `src/tools/DomainsManager.svelte` — thin shell that composes zone components.
- Zone components in `src/lib/components/domains/`:
  - `ChronicleHeader.svelte` — chronicle selector, "+ New chronicle", Kumu export button (disabled in v1).
  - `DomainTree.svelte` — hierarchical tree driven by `contains` edges; search input; "+ Add root node" action.
  - `NodeDetail.svelte` — breadcrumb, header (label + type chip + edit toggle), tags, description, properties grid. Read by default; flips to edit mode.
  - `NodeForm.svelte` — form for create/edit (label, type autocomplete, tags chips, description textarea, property list).
  - `EdgesPanel.svelte` — incoming/outgoing list of non-`contains` edges; "+ Add relationship" opens picker.
  - `EdgePicker.svelte` — modal/inline form: target node (searchable), edge type autocomplete, description.
  - `PropertyEditor.svelte` — registry-dispatched input-widget for a single `Field`; supports add/remove/reorder; ships v1 widgets for `string`, `number`, `text`, `bool`.
- `src/store/domains.svelte.ts` — Svelte 5 runes store holding UI state. (Must use the `.svelte.ts` extension — plain `.ts` files cannot use runes.) Contents:
  - `currentChronicleId: number | null`
  - `currentNodeId: number | null`
  - `nodes: ChronicleNode[]` and `edges: ChronicleEdge[]` — chronicle-scoped cache
  - `loading`, `error` surface flags
  - Reload helpers: `refreshNodes()`, `refreshEdges()`, `setChronicle(id)`, `selectNode(id)`.
- Typed Tauri invoke wrappers in `src/lib/domains/api.ts` — one function per backend command, correctly typed, so zone components don't hand-build `invoke('create_node', …)` calls.
- Empty states: no chronicles yet, empty chronicle, no node selected, node has no children, node has no edges.
- Delete guards surfaced in UI: deleting a node that has children prompts a confirmation; cycle-rejection errors from the backend surface as toasts/inline errors; contains-parent uniqueness is enforced by the picker (existing parent replaced on confirm).
- Breadcrumb follows `get_path_to_root`; entries are clickable.
- Tree search filters nodes by label, tag, or description (chronicle-scoped, client-side over the cached `nodes` array — no new backend command needed).
- Node-type and edge-type autocomplete derived from the cached `nodes[].type` and `edges[].edge_type` distinct values.

### Deferred (explicitly out of v1)

- **Kumu export.** The button renders disabled with a "coming soon" tooltip; the exporter itself is a follow-up PR. Rationale: simple formatter, orthogonal to the UI, can ship independently once the UI is stable.
- **"All nodes" sidebar tab.** v1 sidebar shows only the `contains` tree. A second tab for a flat/filterable list of all nodes is a natural follow-up.
- **Reference / date / url / email / multi-value property types.** `PropertyEditor` is built as a registry so each can be added as a one-file contribution without touching existing widgets.
- **Graph canvas / mindmap view.** Deferred per approved navigation choice; the composed architecture accommodates a future sibling pane.
- **Icon-per-type.** Visual callout in `NodeDetail` and the tree uses the type chip only.
- **Drag-to-reparent, bulk operations, undo/redo.** "Move under…" action in the detail pane handles reparenting.
- **Cross-tool chronicle awareness.** `currentChronicleId` is defined in the store but no other tool subscribes to it yet. This is the extensibility hook — future tools (Resonance, Campaign) can adopt it without store changes.

---

## Navigation Model

Three-column layout under a header bar. Decided over graph-canvas and wiki-style alternatives because it maps directly to the `contains` edge type, doubles as the live-session find-fast surface, and keeps the door open for alternate views as future sibling panes.

### Header Bar

- Tool title on the left.
- Right-aligned: chronicle selector dropdown, "+ New" chronicle button, Kumu export button (disabled in v1).
- Chronicle selector is tool-scoped (not app-global). Other tools are unaffected.

### Sidebar (Tree)

- Single-tab in v1 — the `contains` tree. "All nodes" tab is an in-UI placeholder for the deferred second tab but is not rendered.
- Top: search input (client-side filter over cached nodes).
- Body: recursive collapsible tree. Each node row shows its label and an optional type-chip suffix.
- Bottom: "+ Add root node" action.

### Detail Pane (Center)

- Breadcrumb derived from `get_path_to_root` — hidden for root nodes.
- Header row: label (inline-editable in edit mode), type chip, "✎ Edit" toggle.
- Tags row: chip list.
- Description: rendered as plain text with preserved newlines in v1 (markdown rendering deferred; the spec for backend-phase already flags this).
- Properties: typed-registry grid, one row per `Field`. Read-mode renders values; edit-mode renders widgets with add/remove/reorder.
- Empty state ("no node selected"): neutral copy directing the user to pick a node or add one.

### Edges Panel (Right)

- Header: "Relationships" label.
- Two grouped lists: **Outgoing** (this node as `from_node_id`) and **Incoming** (this node as `to_node_id`).
- `contains` edges are filtered out — they're represented by the tree and breadcrumb.
- Each row: `edge_type` + linked target node label (clickable → becomes `currentNodeId`).
- Bottom: "+ Add relationship" action.

---

## Architecture

### Approach Z: Composed + dedicated store

- Tool is split into ~8 zone components plus `PropertyEditor` and a thin shell (`DomainsManager.svelte`).
- UI state lives in `src/store/domains.ts` as Svelte 5 runes (`$state` exports). Zone components import and read/write directly — no prop-drilling across the main zones, though leaf widgets (`PropertyEditor`, `EdgePicker`) still accept props for their scoped concerns.
- Backend-facing I/O is wrapped in `src/lib/domains/api.ts` — a single file exporting typed async functions per Tauri command. Zone components import from here; they never call `invoke` directly.
- Loading strategy: when `currentChronicleId` changes, `refreshNodes()` and `refreshEdges()` run once, populating the cache. All downstream views render from cache. Mutations go through `api.ts`, then trigger a targeted cache refresh (re-fetch the affected list(s), not invalidate everything).

### Why this over a monolith

Given the stated extensibility preference: the store is ~40 lines, zero runtime cost, and enables any future tool to participate in chronicle-scoped state by importing it. Retrofitting it later after 7+ zone components have their own prop interfaces is meaningfully more expensive.

### Component graph

`NodeDetail` hosts an internal read/edit mode toggle. In read mode it renders its own read-only markup; in edit mode it mounts `NodeForm`. `NodeForm` is therefore *inside* `NodeDetail`, not a sibling.

```
routes/+layout.svelte
└── DomainsManager.svelte (tool shell)
    ├── ChronicleHeader.svelte          [selector, + New, export]
    ├── DomainTree.svelte                [tree + search + add root]
    ├── NodeDetail.svelte                [breadcrumb, header, tags, desc, props]
    │   ├── (read mode)  PropertyEditor.svelte  ×N   [one per Field, readonly]
    │   └── (edit mode)  NodeForm.svelte
    │                     └── PropertyEditor.svelte  ×N   [editable]
    ├── EdgesPanel.svelte                [in/out groups]
    └── EdgePicker.svelte                [opens from EdgesPanel "+ Add relationship"]
```

### State module — `src/store/domains.svelte.ts`

```ts
// File must end in .svelte.ts for runes to work outside a component.
import type { Chronicle, ChronicleNode, ChronicleEdge } from '../types';

// Rune-reactive values cannot be reassigned by importers, so we wrap them in
// objects whose properties are mutated. Consumers read e.g. `session.chronicleId`.
export const session = $state<{
  chronicleId: number | null;
  nodeId: number | null;
}>({ chronicleId: null, nodeId: null });

export const cache = $state<{
  chronicles: Chronicle[];
  nodes: ChronicleNode[];
  edges: ChronicleEdge[];
}>({ chronicles: [], nodes: [], edges: [] });

export const status = $state<{ loading: boolean; error: string | null }>({
  loading: false, error: null,
});

export async function setChronicle(id: number | null): Promise<void> { /* … */ }
export async function refreshNodes(): Promise<void>                  { /* … */ }
export async function refreshEdges(): Promise<void>                  { /* … */ }
export function selectNode(id: number | null): void                  { /* … */ }
```

The wrapper-object pattern is required: Svelte 5 runes can't be reassigned across module boundaries, but property mutations on a `$state`-wrapped object propagate reactively.

### API wrapper — `src/lib/domains/api.ts`

One typed function per Tauri command. Example signatures:

```ts
import { invoke } from '@tauri-apps/api/core';
import type { Chronicle, ChronicleNode, ChronicleEdge, Field } from '../../types';

export const listChronicles  = () =>
  invoke<Chronicle[]>('list_chronicles');

export const createChronicle = (name: string, description: string) =>
  invoke<Chronicle>('create_chronicle', { name, description });

export const listNodes = (chronicleId: number, typeFilter?: string) =>
  invoke<ChronicleNode[]>('list_nodes', { chronicleId, typeFilter });

export const createNode = (
  chronicleId: number, nodeType: string, label: string, description: string,
  tags: string[], properties: Field[],
) => invoke<ChronicleNode>('create_node', {
  chronicleId, nodeType, label, description, tags, properties,
});

// …etc for update/delete/get and all edge commands and tree-derived queries
```

Zone components depend only on this file for backend access.

### Property editor registry

`PropertyEditor.svelte` takes a `Field` and a read/edit mode, looks up a widget from a registry keyed on `FieldValue['type']`, and renders it. v1 ships widgets for `string`, `number`, `text`, `bool`.

```ts
// src/lib/components/domains/property-widgets/index.ts
const WIDGETS: Record<FieldValue['type'], Widget> = {
  string: StringWidget,
  text:   TextWidget,
  number: NumberWidget,
  bool:   BoolWidget,
  // date / url / email / reference added here in follow-ups
};
```

Adding a deferred type post-v1 is a single-file PR: new widget, one registry entry. No changes in `NodeDetail`, `NodeForm`, or `PropertyEditor`.

---

## Data Flow (Key Interactions)

- **Open tool.** `DomainsManager` mounts. `ChronicleHeader` calls `listChronicles()` via `api.ts`, populates `chronicles.list`. If any exist, `setChronicle(first.id)` triggers `refreshNodes()` + `refreshEdges()`. Otherwise the empty state prompts "create your first chronicle."
- **Select a node.** `DomainTree` calls `selectNode(id)`. `NodeDetail` reactively renders from the cached node. `EdgesPanel` reactively re-groups `edges.list` by `from_node_id === id` / `to_node_id === id`, excluding `contains`. Breadcrumb is computed via `getPathToRoot(id)` on-demand (cached in local reactive `$derived` inside `NodeDetail`).
- **Create node.** `NodeForm` submits → `api.createNode(…)` → on success, append to `nodes.list` (no full refetch needed). If the user also chose a parent, also `api.createEdge(parent, new, 'contains')` → append to `edges.list`.
- **Edit node.** Edit toggle flips `NodeDetail` into `NodeForm`. Submit → `api.updateNode(…)` → replace the entry in `nodes.list` by id.
- **Delete node.** If the node has children (check `edges.list` for outgoing `contains`), show a confirmation mentioning cascade. On confirm → `api.deleteNode(id)`. Cache-level: remove node from `nodes.list` and all edges referencing it from `edges.list` (the backend cascades the DB; the frontend mirrors that).
- **Add relationship.** `EdgePicker` submits → `api.createEdge(…)` → append to `edges.list`. Cycle errors bubble as an inline error ("This would create a loop under contains").
- **Move under (reparent).** An action in `NodeDetail`'s edit mode; represented as: find the current `contains` edge targeting this node, delete it; create a new one from the chosen parent. Single backend-atomic sequence is out of v1 scope — two separate calls with UI-level error handling if the second fails.

---

## Error Handling

- All `api.ts` functions throw on error. Callers catch and surface via `status.error` (banner) or local inline error (form validation).
- Backend cycle rejection from `create_edge` is a user-actionable error, not a crash. Show inline in the picker.
- SQLite UNIQUE constraint on `(from, to, edge_type)` surfaces as "This relationship already exists."
- The `idx_edges_contains_single_parent` partial index means trying to add a second `contains` parent fails with a UNIQUE violation. UI intercepts this: if user is setting a `contains` parent and one exists, prompt "Replace existing parent?" and delete-then-insert.

---

## Testing Strategy

- **Rust tests**: already complete (44 tests). No new backend code.
- **Frontend**: no test framework currently in the project. Per the existing project convention, correctness is verified via:
  - `npm run check` — TypeScript + Svelte checks.
  - `npm run build` — production build proves no broken imports / syntax.
  - `./scripts/verify.sh` — aggregate gate.
  - Manual smoke test: open app, create chronicle, create nodes with `contains` relationships, verify tree renders, verify breadcrumb, verify edit flow, verify edge creation and cycle rejection.
- Adding a frontend test framework is out of v1 scope; would be a separate decision.

---

## File List

**New files:**

- `src/tools/DomainsManager.svelte`
- `src/lib/components/domains/ChronicleHeader.svelte`
- `src/lib/components/domains/DomainTree.svelte`
- `src/lib/components/domains/NodeDetail.svelte`
- `src/lib/components/domains/NodeForm.svelte`
- `src/lib/components/domains/EdgesPanel.svelte`
- `src/lib/components/domains/EdgePicker.svelte`
- `src/lib/components/domains/PropertyEditor.svelte`
- `src/lib/components/domains/property-widgets/StringWidget.svelte`
- `src/lib/components/domains/property-widgets/TextWidget.svelte`
- `src/lib/components/domains/property-widgets/NumberWidget.svelte`
- `src/lib/components/domains/property-widgets/BoolWidget.svelte`
- `src/lib/components/domains/property-widgets/index.ts` (registry)
- `src/lib/domains/api.ts` (typed invoke wrappers)
- `src/store/domains.svelte.ts` (UI state store — `.svelte.ts` extension is required for runes)

**Modified files:**

- `src/tools.ts` — one new entry registering the `domains` tool.
- `docs/design/data-sources.md` and `docs/design/data-sources-kumu.json` — light edits to mark the Chronicle Store as "wired to UI."
- `CLAUDE.md` — remove the "types not yet wired into Tauri commands" note for `Chronicle`/`Node`/`Edge` (still valid for unused Rust-side field types until property widgets for them exist; update the list).

---

## Risks / Open Questions

- **Svelte 5 rune state pattern in a module.** Runes at module-top-level need the wrapper-object workaround. If any zone component reads `currentChronicleId.v` without tracking, we'll miss reactivity — need to be disciplined about using `.v` access inside `$derived`/`$effect`. Standard pattern, low risk.
- **Client-side search scale.** A chronicle with >5,000 nodes would make client-side search sluggish. Not a real-world concern for v1 but noted. If it becomes one, add a backend `search_nodes` command and swap the tree's filter source.
- **Move-under atomicity.** Reparenting is two calls (delete old contains, create new). If the second fails, the node ends up rootless. Acceptable for v1; a future backend command `reparent_node(node_id, new_parent_id)` could do it in a single transaction.

---

## Implementation Order (for the plan)

The writing-plans skill will expand this into tasks. Rough order:

1. Store module + API wrapper (pure plumbing, no UI).
2. Minimal `DomainsManager.svelte` shell (renders a placeholder) *and* matching entry in `src/tools.ts` added together in the same step — registering the tool before the component file exists would break the lazy import.
3. `ChronicleHeader` + chronicle CRUD.
4. `DomainTree` rendering (read-only).
5. `NodeDetail` (read mode).
6. `NodeForm` + `PropertyEditor` + v1 property widgets.
7. Create/edit/delete node wiring.
8. `EdgesPanel` + `EdgePicker`.
9. Search, breadcrumb, empty states, error handling polish.
10. Delete guards, cycle-error surfacing, move-under action.
