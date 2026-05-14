# Domains Manager UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Domains Manager frontend v1 — a three-column Svelte UI (sidebar tree + node detail + edges panel) that consumes the 20 Tauri commands already shipped for chronicle / node / edge CRUD, plus a typed-property editor and a chronicle-scoped Svelte runes store.

**Architecture:** Thin tool shell (`DomainsManager.svelte`) composes zone components that read/write shared UI state from `src/store/domains.svelte.ts`. All backend I/O goes through a single typed wrapper module (`src/lib/domains/api.ts`); no component calls `invoke` directly. Property editor uses a widget registry so deferred field types (date/url/email/reference/multi-value) are one-file additions.

**Tech Stack:** SvelteKit, Svelte 5 runes (`$state`, `$derived`, `$effect`), TypeScript, Tauri 2 `@tauri-apps/api/core` `invoke`. Backend already in place — no Rust changes.

**Verification without a test framework:** This project has no frontend test framework (see `CLAUDE.md`). Every task verifies correctness via `npm run check` (type-check + Svelte-check) and, where the task changes runtime behavior, a manual smoke test in `npm run tauri dev`. After the last task, `./scripts/verify.sh` must pass (it runs `npm run check` + `cargo test` + `npm run build`).

**Spec:** `docs/superpowers/specs/2026-04-19-domains-manager-ui-design.md`.

---

## File Structure

### New files

| Path | Responsibility |
|---|---|
| `src/store/domains.svelte.ts` | Svelte 5 runes store: current chronicle/node, cached nodes+edges+chronicles, status, mutators. |
| `src/lib/domains/api.ts` | One typed async function per Tauri command. Zone components never call `invoke` directly. |
| `src/tools/DomainsManager.svelte` | Tool shell. Composes header + tree + detail + edges panel. Triggers initial load. |
| `src/lib/components/domains/ChronicleHeader.svelte` | Chronicle dropdown, "+ New chronicle" dialog, Kumu export button (disabled v1). |
| `src/lib/components/domains/DomainTree.svelte` | Recursive sidebar tree over `contains` edges. Search input. "+ Add root node". |
| `src/lib/components/domains/NodeDetail.svelte` | Read-mode detail view with breadcrumb, header, tags, description, property list. Edit toggle mounts `NodeForm`. |
| `src/lib/components/domains/NodeForm.svelte` | Create/edit form for a node (label, type, tags, description, properties). |
| `src/lib/components/domains/EdgesPanel.svelte` | Non-`contains` relationships split into outgoing / incoming. "+ Add relationship". |
| `src/lib/components/domains/EdgePicker.svelte` | Inline form to create an edge (target node + edge type + optional description). |
| `src/lib/components/domains/PropertyEditor.svelte` | Renders one `Field` via the widget registry in read or edit mode. Emits updates. |
| `src/lib/components/domains/property-widgets/index.ts` | The registry — maps `FieldValue['type']` to a widget component. |
| `src/lib/components/domains/property-widgets/StringWidget.svelte` | Single-line text input. |
| `src/lib/components/domains/property-widgets/TextWidget.svelte` | Multi-line textarea. |
| `src/lib/components/domains/property-widgets/NumberWidget.svelte` | Number input. |
| `src/lib/components/domains/property-widgets/BoolWidget.svelte` | Checkbox. |

### Modified files

| Path | Change |
|---|---|
| `src/tools.ts` | Add `domains` tool entry (lazy-loaded `DomainsManager.svelte`). |
| `CLAUDE.md` | Remove the "not yet wired into Tauri commands" warning about `Chronicle/Node/Edge` once the UI is consuming them. Update the note about `shared/types.rs` to list only the genuinely-unused types that remain (reference-field-related types until those widgets exist). |
| `docs/design/data-sources.md` | Mark Chronicle Store as "wired to UI: Domains Manager". |
| `docs/design/data-sources-kumu.json` | Update the Chronicle Store / Domains Manager connection status. |

### Constraints discovered during exploration

- **Tauri argument naming.** Rust Tauri commands declare parameters in `snake_case`; invocation from TS uses `camelCase` (Tauri auto-converts at the IPC boundary). Confirmed by existing usage in `DyscrasiaForm.svelte` (`invoke('update_dyscrasia', { id, name, description, bonus })`). Nested objects keep their serde-defined shape.
- **Struct response field names.** `Chronicle`, `Node`, `Edge` Rust structs have no `#[serde(rename_all = "camelCase")]` attribute, so response JSON uses `snake_case` (`chronicle_id`, `from_node_id`, `created_at`). This matches the existing TS type definitions in `src/types.ts`.
- **Tagged-union `FieldValue`.** `#[serde(tag = "type")]` with `rename_all = "snake_case"` means the JSON discriminator is `{"type": "string"|"text"|"number"|"bool"|…, "value": …}`. `Field` uses `#[serde(flatten)]`, so a Field JSON is `{"name": "x", "type": "number", "value": 3}`.
- **Store rune pattern.** Module-level `$state` values cannot be reassigned by importers, but mutations on properties of a `$state`-wrapped object propagate. Use wrapper objects (`session.chronicleId`, `cache.nodes`, etc.).
- **Runes in `.ts` files.** Svelte 5 only enables runes in `.svelte` and `.svelte.ts`/`.svelte.js` files. The store module **must** be named `domains.svelte.ts`.

---

## Task Index

1. Create the API wrapper module
2. Create the runes store module
3. Scaffold the tool shell and register in `tools.ts`
4. Build `ChronicleHeader`
5. Build `DomainTree` (read-only render)
6. Build `NodeDetail` in read mode (no properties yet)
7. Build the property widgets, registry, and `PropertyEditor`
8. Wire `PropertyEditor` into `NodeDetail` read mode
9. Build `NodeForm` and wire create/edit flows
10. Wire node delete (with cascade confirmation) and root-node creation
11. Build `EdgesPanel` and `EdgePicker`
12. Add tree search, breadcrumb, empty states
13. Error surfacing and contains-parent replacement flow
14. Update docs (`CLAUDE.md`, `data-sources.md`, Kumu JSON)
15. Final verification

---

## Task 1: Create the API wrapper module

**Files:**
- Create: `src/lib/domains/api.ts`

**Purpose:** One typed async function per Tauri command. No calls to `invoke` happen outside this file.

- [ ] **Step 1: Create the file with all 20 wrappers**

Write `src/lib/domains/api.ts`:

```ts
import { invoke } from '@tauri-apps/api/core';
import type {
  Chronicle,
  ChronicleNode,
  ChronicleEdge,
  EdgeDirection,
  Field,
} from '../../types';

// ---------- Chronicles ----------

export const listChronicles = () =>
  invoke<Chronicle[]>('list_chronicles');

export const getChronicle = (id: number) =>
  invoke<Chronicle>('get_chronicle', { id });

export const createChronicle = (name: string, description: string) =>
  invoke<Chronicle>('create_chronicle', { name, description });

export const updateChronicle = (id: number, name: string, description: string) =>
  invoke<Chronicle>('update_chronicle', { id, name, description });

export const deleteChronicle = (id: number) =>
  invoke<void>('delete_chronicle', { id });

// ---------- Nodes ----------

export const listNodes = (chronicleId: number, typeFilter?: string) =>
  invoke<ChronicleNode[]>('list_nodes', { chronicleId, typeFilter });

export const getNode = (id: number) =>
  invoke<ChronicleNode>('get_node', { id });

export const createNode = (
  chronicleId: number,
  nodeType: string,
  label: string,
  description: string,
  tags: string[],
  properties: Field[],
) =>
  invoke<ChronicleNode>('create_node', {
    chronicleId, nodeType, label, description, tags, properties,
  });

export const updateNode = (
  id: number,
  nodeType: string,
  label: string,
  description: string,
  tags: string[],
  properties: Field[],
) =>
  invoke<ChronicleNode>('update_node', {
    id, nodeType, label, description, tags, properties,
  });

export const deleteNode = (id: number) =>
  invoke<void>('delete_node', { id });

// ---------- Derived tree queries ----------

export const getParentOf = (nodeId: number) =>
  invoke<ChronicleNode | null>('get_parent_of', { nodeId });

export const getChildrenOf = (nodeId: number) =>
  invoke<ChronicleNode[]>('get_children_of', { nodeId });

export const getSiblingsOf = (nodeId: number) =>
  invoke<ChronicleNode[]>('get_siblings_of', { nodeId });

export const getPathToRoot = (nodeId: number) =>
  invoke<ChronicleNode[]>('get_path_to_root', { nodeId });

export const getSubtree = (nodeId: number, maxDepth?: number) =>
  invoke<ChronicleNode[]>('get_subtree', { nodeId, maxDepth });

// ---------- Edges ----------

export const listEdges = (chronicleId: number, edgeTypeFilter?: string) =>
  invoke<ChronicleEdge[]>('list_edges', { chronicleId, edgeTypeFilter });

export const listEdgesForNode = (
  nodeId: number,
  direction: EdgeDirection,
  edgeTypeFilter?: string,
) =>
  invoke<ChronicleEdge[]>('list_edges_for_node', { nodeId, direction, edgeTypeFilter });

export const createEdge = (
  chronicleId: number,
  fromNodeId: number,
  toNodeId: number,
  edgeType: string,
  description: string,
  properties: Field[],
) =>
  invoke<ChronicleEdge>('create_edge', {
    chronicleId, fromNodeId, toNodeId, edgeType, description, properties,
  });

export const updateEdge = (
  id: number,
  edgeType: string,
  description: string,
  properties: Field[],
) =>
  invoke<ChronicleEdge>('update_edge', { id, edgeType, description, properties });

export const deleteEdge = (id: number) =>
  invoke<void>('delete_edge', { id });
```

- [ ] **Step 2: Type-check**

Run: `npm run check`
Expected: 0 errors. The file is imported nowhere yet, but type-check parses it.

- [ ] **Step 3: Commit**

```bash
git add src/lib/domains/api.ts
git commit -m "feat(domains): add typed Tauri API wrapper for domains manager"
```

---

## Task 2: Create the runes store module

**Files:**
- Create: `src/store/domains.svelte.ts`

- [ ] **Step 1: Create the store**

Write `src/store/domains.svelte.ts`:

```ts
import type { Chronicle, ChronicleNode, ChronicleEdge } from '../types';
import * as api from '../lib/domains/api';

// -------- Reactive state ---------------------------------------------------
//
// Svelte 5 $state values cannot be reassigned across module boundaries, so
// each group is wrapped in an object whose properties are mutated. Readers
// access e.g. `session.chronicleId`, `cache.nodes`.

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
  loading: false,
  error: null,
});

// -------- Mutators ---------------------------------------------------------

export async function refreshChronicles(): Promise<void> {
  status.error = null;
  try {
    cache.chronicles = await api.listChronicles();
  } catch (e) {
    status.error = String(e);
  }
}

export async function refreshNodes(): Promise<void> {
  if (session.chronicleId == null) {
    cache.nodes = [];
    return;
  }
  status.error = null;
  try {
    cache.nodes = await api.listNodes(session.chronicleId);
  } catch (e) {
    status.error = String(e);
  }
}

export async function refreshEdges(): Promise<void> {
  if (session.chronicleId == null) {
    cache.edges = [];
    return;
  }
  status.error = null;
  try {
    cache.edges = await api.listEdges(session.chronicleId);
  } catch (e) {
    status.error = String(e);
  }
}

export async function setChronicle(id: number | null): Promise<void> {
  session.chronicleId = id;
  session.nodeId = null;
  cache.nodes = [];
  cache.edges = [];
  if (id == null) return;
  status.loading = true;
  try {
    await Promise.all([refreshNodes(), refreshEdges()]);
  } finally {
    status.loading = false;
  }
}

export function selectNode(id: number | null): void {
  session.nodeId = id;
}

export function clearError(): void {
  status.error = null;
}
```

- [ ] **Step 2: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/store/domains.svelte.ts
git commit -m "feat(domains): add runes store for chronicle-scoped UI state"
```

---

## Task 3: Scaffold the tool shell and register in `tools.ts`

**Files:**
- Create: `src/tools/DomainsManager.svelte`
- Modify: `src/tools.ts`

- [ ] **Step 1: Create a minimal shell**

Write `src/tools/DomainsManager.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { session, cache, status, refreshChronicles, setChronicle } from '../store/domains.svelte';

  onMount(async () => {
    await refreshChronicles();
    if (session.chronicleId == null && cache.chronicles.length > 0) {
      await setChronicle(cache.chronicles[0].id);
    }
  });
</script>

<div class="page">
  <h1 class="title">Domains</h1>

  {#if status.loading}
    <p class="loading-text">Loading…</p>
  {:else if status.error}
    <p class="error-text">{status.error}</p>
  {:else if cache.chronicles.length === 0}
    <p class="empty">No chronicles yet. Create one to get started.</p>
  {:else}
    <p class="empty">
      Chronicle selected: {cache.chronicles.find(c => c.id === session.chronicleId)?.name ?? '(none)'}
      — nodes: {cache.nodes.length}, edges: {cache.edges.length}.
    </p>
  {/if}
</div>

<style>
  .page { padding: 1rem 1.25rem; }
  .title { color: var(--accent); font-size: 1.4rem; margin-bottom: 1rem; }
  .loading-text, .empty { color: var(--text-ghost); font-size: 0.8rem; }
  .error-text { color: var(--accent); font-size: 0.8rem; padding: 1rem 0; }
</style>
```

- [ ] **Step 2: Register the tool in `src/tools.ts`**

Replace the entire contents of `src/tools.ts` with:

```ts
import type { Component } from 'svelte';

export interface Tool {
  id: string;
  label: string;
  icon: string; // emoji or SVG string
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  component: () => Promise<{ default: Component<any> }>;
}

// Add new tools here — the sidebar renders from this list automatically.
export const tools: Tool[] = [
  {
    id: 'resonance',
    label: 'Resonance Roller',
    icon: '🩸',
    component: () => import('./tools/Resonance.svelte'),
  },
  {
    id: 'dyscrasias',
    label: 'Dyscrasias',
    icon: '📋',
    component: () => import('./tools/DyscrasiaManager.svelte'),
  },
  {
    id: 'campaign',
    label: 'Campaign',
    icon: '🗺️',
    component: () => import('./tools/Campaign.svelte'),
  },
  {
    id: 'domains',
    label: 'Domains',
    icon: '🏰',
    component: () => import('./tools/DomainsManager.svelte'),
  },
];
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test**

Run: `npm run tauri dev`
Expected:
- Sidebar shows four entries: Resonance Roller, Dyscrasias, Campaign, Domains.
- Clicking Domains loads the placeholder page.
- Page shows either "No chronicles yet. Create one to get started." (fresh DB) or a summary line showing the first chronicle.
- No errors in devtools.

Close the app (or leave it running for later steps).

- [ ] **Step 5: Commit**

```bash
git add src/tools/DomainsManager.svelte src/tools.ts
git commit -m "feat(domains): scaffold DomainsManager tool shell and register in sidebar"
```

---

## Task 4: Build `ChronicleHeader`

**Files:**
- Create: `src/lib/components/domains/ChronicleHeader.svelte`
- Modify: `src/tools/DomainsManager.svelte`

**Purpose:** Chronicle dropdown, "+ New", rename/delete actions, and the disabled "Kumu" export button.

- [ ] **Step 1: Create `ChronicleHeader.svelte`**

Write `src/lib/components/domains/ChronicleHeader.svelte`:

```svelte
<script lang="ts">
  import * as api from '../../domains/api';
  import { session, cache, setChronicle, refreshChronicles } from '../../../store/domains.svelte';

  let creating = $state(false);
  let renaming = $state(false);
  let draftName = $state('');
  let draftDesc = $state('');
  let localError = $state('');

  const current = $derived(cache.chronicles.find(c => c.id === session.chronicleId) ?? null);

  function startCreate() {
    creating = true;
    renaming = false;
    draftName = '';
    draftDesc = '';
    localError = '';
  }

  function startRename() {
    if (!current) return;
    renaming = true;
    creating = false;
    draftName = current.name;
    draftDesc = current.description;
    localError = '';
  }

  function cancel() {
    creating = false;
    renaming = false;
    localError = '';
  }

  async function saveCreate() {
    const name = draftName.trim();
    if (!name) { localError = 'Name is required.'; return; }
    try {
      const c = await api.createChronicle(name, draftDesc.trim());
      await refreshChronicles();
      await setChronicle(c.id);
      creating = false;
    } catch (e) {
      localError = String(e);
    }
  }

  async function saveRename() {
    if (!current) return;
    const name = draftName.trim();
    if (!name) { localError = 'Name is required.'; return; }
    try {
      await api.updateChronicle(current.id, name, draftDesc.trim());
      await refreshChronicles();
      renaming = false;
    } catch (e) {
      localError = String(e);
    }
  }

  async function deleteCurrent() {
    if (!current) return;
    if (!confirm(`Delete chronicle "${current.name}"? All nodes and edges will be removed. This cannot be undone.`)) return;
    try {
      await api.deleteChronicle(current.id);
      await refreshChronicles();
      const next = cache.chronicles[0]?.id ?? null;
      await setChronicle(next);
    } catch (e) {
      localError = String(e);
    }
  }

  async function onSelect(e: Event) {
    const id = Number((e.target as HTMLSelectElement).value);
    await setChronicle(id);
  }
</script>

<header class="header">
  <div class="title">🏰 Domains Manager</div>

  <div class="spacer"></div>

  {#if !creating && !renaming}
    <label class="field">
      <span class="label">Chronicle</span>
      <select value={session.chronicleId ?? ''} onchange={onSelect} disabled={cache.chronicles.length === 0}>
        {#if cache.chronicles.length === 0}
          <option value="">(none)</option>
        {/if}
        {#each cache.chronicles as c (c.id)}
          <option value={c.id}>{c.name}</option>
        {/each}
      </select>
    </label>

    <button class="btn" onclick={startCreate}>+ New</button>
    {#if current}
      <button class="btn" onclick={startRename}>✎ Rename</button>
      <button class="btn danger" onclick={deleteCurrent}>✕ Delete</button>
    {/if}
    <button class="btn" disabled title="Coming soon">⇩ Kumu</button>
  {:else}
    <div class="inline-form">
      <input class="input" placeholder="Chronicle name" bind:value={draftName} />
      <input class="input" placeholder="Description (optional)" bind:value={draftDesc} />
      <button class="btn primary" onclick={creating ? saveCreate : saveRename}>Save</button>
      <button class="btn" onclick={cancel}>Cancel</button>
    </div>
  {/if}
</header>

{#if localError}
  <div class="error">{localError}</div>
{/if}

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background: var(--bg-card);
    border-bottom: 1px solid var(--border-surface);
  }
  .title { font-weight: 600; color: var(--text-primary); font-size: 0.92rem; }
  .spacer { flex: 1; }
  .field { display: inline-flex; align-items: center; gap: 0.4rem; }
  .label {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  select, .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.78rem;
    font-family: inherit;
    outline: none;
  }
  select:focus, .input:focus { border-color: var(--accent); }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.65rem;
    font-size: 0.74rem;
    cursor: pointer;
    transition: box-shadow 0.15s, transform 0.1s;
  }
  .btn:hover:not(:disabled) { color: var(--text-primary); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn.primary { color: var(--accent); }
  .btn.primary:hover { box-shadow: 0 0 8px #cc222255; }
  .btn.danger { color: var(--accent); }
  .btn.danger:hover { box-shadow: 0 0 8px #cc222255; }
  .inline-form { display: flex; gap: 0.4rem; align-items: center; }
  .error {
    color: var(--accent-bright);
    font-size: 0.72rem;
    padding: 0.3rem 0.75rem;
    background: var(--bg-sunken);
    border-bottom: 1px solid var(--border-surface);
  }
</style>
```

- [ ] **Step 2: Wire `ChronicleHeader` into `DomainsManager.svelte`**

Replace the contents of `src/tools/DomainsManager.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import ChronicleHeader from '$lib/components/domains/ChronicleHeader.svelte';
  import { session, cache, status, refreshChronicles, setChronicle } from '../store/domains.svelte';

  onMount(async () => {
    await refreshChronicles();
    if (session.chronicleId == null && cache.chronicles.length > 0) {
      await setChronicle(cache.chronicles[0].id);
    }
  });
</script>

<div class="tool">
  <ChronicleHeader />

  <div class="body">
    {#if status.loading}
      <p class="muted">Loading…</p>
    {:else if cache.chronicles.length === 0}
      <p class="muted">No chronicles yet. Click "+ New" above to create one.</p>
    {:else if session.chronicleId == null}
      <p class="muted">Select a chronicle to start.</p>
    {:else}
      <p class="muted">
        Chronicle loaded: {cache.nodes.length} nodes, {cache.edges.length} edges.
      </p>
    {/if}
  </div>
</div>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100%;
  }
  .body { flex: 1; padding: 1rem 1.25rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.82rem; }
</style>
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test**

Run: `npm run tauri dev`
Expected:
- Open Domains tool.
- Click "+ New", type a name like "Test Chronicle", save. Header re-renders with dropdown showing the new chronicle.
- Click "✎ Rename", change name, save. Dropdown updates.
- Create a second chronicle, use dropdown to switch. Body text updates ("0 nodes, 0 edges").
- Click "✕ Delete" (confirm the dialog). Chronicle disappears; dropdown selects the other.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/domains/ChronicleHeader.svelte src/tools/DomainsManager.svelte
git commit -m "feat(domains): add ChronicleHeader with CRUD + disabled Kumu export"
```

---

## Task 5: Build `DomainTree` (read-only render)

**Files:**
- Create: `src/lib/components/domains/DomainTree.svelte`
- Modify: `src/tools/DomainsManager.svelte`

**Purpose:** Recursive tree over `contains` edges in the cached state. Click selects the node. No editing or search yet — those come in later tasks.

- [ ] **Step 1: Create `DomainTree.svelte`**

Write `src/lib/components/domains/DomainTree.svelte`:

```svelte
<script lang="ts">
  import type { ChronicleNode } from '../../../types';
  import { session, cache, selectNode } from '../../../store/domains.svelte';

  // Nodes that appear in the tree root: those with no incoming 'contains' edge.
  const rootNodes = $derived(computeRoots());
  // Map parentId -> children[], so recursive rendering is O(n).
  const childrenByParent = $derived(computeChildrenMap());
  const expanded: Set<number> = $state(new Set());

  function computeRoots(): ChronicleNode[] {
    const containsTargets = new Set(
      cache.edges.filter(e => e.edge_type === 'contains').map(e => e.to_node_id)
    );
    return cache.nodes
      .filter(n => !containsTargets.has(n.id))
      .sort((a, b) => a.label.localeCompare(b.label));
  }

  function computeChildrenMap(): Map<number, ChronicleNode[]> {
    const byId = new Map<number, ChronicleNode>();
    for (const n of cache.nodes) byId.set(n.id, n);
    const m = new Map<number, ChronicleNode[]>();
    for (const e of cache.edges) {
      if (e.edge_type !== 'contains') continue;
      const child = byId.get(e.to_node_id);
      if (!child) continue;
      if (!m.has(e.from_node_id)) m.set(e.from_node_id, []);
      m.get(e.from_node_id)!.push(child);
    }
    for (const arr of m.values()) arr.sort((a, b) => a.label.localeCompare(b.label));
    return m;
  }

  function toggle(id: number) {
    // Svelte 5 proxies Set mutations, so .add/.delete trigger reactivity.
    if (expanded.has(id)) expanded.delete(id);
    else expanded.add(id);
  }
</script>

<aside class="tree">
  <div class="tree-header">Domains</div>

  {#if session.chronicleId == null}
    <p class="empty">No chronicle selected.</p>
  {:else if cache.nodes.length === 0}
    <p class="empty">This chronicle is empty. Create a node to begin.</p>
  {:else}
    <ul class="list">
      {#each rootNodes as node (node.id)}
        {@render treeRow(node, 0)}
      {/each}
    </ul>
  {/if}
</aside>

{#snippet treeRow(node: ChronicleNode, depth: number)}
  {@const kids = childrenByParent.get(node.id) ?? []}
  {@const open = expanded.has(node.id)}
  <li>
    <button
      class="row"
      class:active={session.nodeId === node.id}
      style="padding-left: calc(0.5rem + {depth * 0.8}rem)"
      onclick={() => selectNode(node.id)}
    >
      {#if kids.length > 0}
        <span
          class="caret"
          role="button"
          tabindex="0"
          onclick={(e) => { e.stopPropagation(); toggle(node.id); }}
          onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); toggle(node.id); } }}
        >{open ? '▾' : '▸'}</span>
      {:else}
        <span class="caret ghost">•</span>
      {/if}
      <span class="label">{node.label}</span>
      <span class="type">{node.type}</span>
    </button>
    {#if open && kids.length > 0}
      <ul class="list">
        {#each kids as child (child.id)}
          {@render treeRow(child, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
{/snippet}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: auto;
    border-right: 1px solid var(--border-surface);
    background: var(--bg-sunken);
  }
  .tree-header {
    padding: 0.45rem 0.65rem;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-faint);
  }
  .empty { color: var(--text-ghost); font-size: 0.74rem; padding: 0.6rem 0.8rem; }
  .list { list-style: none; margin: 0; padding: 0; }
  .row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    width: 100%;
    background: transparent;
    border: 0;
    border-left: 2px solid transparent;
    padding: 0.28rem 0.6rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
  }
  .row:hover { background: var(--bg-raised); color: var(--text-primary); }
  .row.active {
    background: var(--bg-active);
    color: var(--text-primary);
    border-left-color: var(--accent);
  }
  .caret {
    width: 0.85rem;
    color: var(--text-ghost);
    font-size: 0.65rem;
    user-select: none;
  }
  .caret.ghost { color: var(--border-faint); }
  .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .type {
    color: var(--text-ghost);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.7;
  }
</style>
```

- [ ] **Step 2: Slot the tree into `DomainsManager.svelte`**

Replace the contents of `src/tools/DomainsManager.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import ChronicleHeader from '$lib/components/domains/ChronicleHeader.svelte';
  import DomainTree from '$lib/components/domains/DomainTree.svelte';
  import { session, cache, status, refreshChronicles, setChronicle } from '../store/domains.svelte';

  onMount(async () => {
    await refreshChronicles();
    if (session.chronicleId == null && cache.chronicles.length > 0) {
      await setChronicle(cache.chronicles[0].id);
    }
  });
</script>

<div class="tool">
  <ChronicleHeader />

  {#if status.loading}
    <p class="muted">Loading…</p>
  {:else if cache.chronicles.length === 0}
    <p class="muted">No chronicles yet. Click "+ New" above to create one.</p>
  {:else if session.chronicleId == null}
    <p class="muted">Select a chronicle to start.</p>
  {:else}
    <div class="grid">
      <DomainTree />
      <main class="detail-placeholder">
        {#if session.nodeId == null}
          <p class="muted">Select a node from the tree to view its details.</p>
        {:else}
          <p class="muted">
            Selected node id: {session.nodeId}
            — (detail pane not implemented yet; see Task 6)
          </p>
        {/if}
      </main>
      <aside class="edges-placeholder">
        <p class="muted">Relationships panel (Task 11).</p>
      </aside>
    </div>
  {/if}
</div>

<style>
  .tool {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100%;
  }
  .grid {
    display: grid;
    grid-template-columns: 18rem 1fr 17rem;
    flex: 1;
    min-height: 0;
  }
  .detail-placeholder, .edges-placeholder { padding: 1rem; overflow: auto; }
  .edges-placeholder { border-left: 1px solid var(--border-surface); background: var(--bg-sunken); }
  .muted { color: var(--text-ghost); font-size: 0.82rem; }
</style>
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test (needs seed data)**

Because node/edge CRUD from the UI doesn't exist yet, seed test data via the app's devtools console (Ctrl+Shift+I). After `npm run tauri dev` is running and a chronicle is selected:

```js
const { invoke } = window.__TAURI__.core;
const cid = /* session.chronicleId value — copy it from the body text */;
const a = await invoke('create_node', { chronicleId: cid, nodeType: 'area',      label: 'New York',  description: '', tags: [], properties: [] });
const b = await invoke('create_node', { chronicleId: cid, nodeType: 'area',      label: 'Manhattan', description: '', tags: [], properties: [] });
const c = await invoke('create_node', { chronicleId: cid, nodeType: 'business',  label: 'Succubus Club', description: '', tags: [], properties: [] });
await invoke('create_edge', { chronicleId: cid, fromNodeId: a.id, toNodeId: b.id, edgeType: 'contains', description: '', properties: [] });
await invoke('create_edge', { chronicleId: cid, fromNodeId: b.id, toNodeId: c.id, edgeType: 'contains', description: '', properties: [] });
```

Then reload the app (Ctrl+R inside the devtools or restart). Expected:
- Sidebar tree shows "New York" with a ▸ caret.
- Clicking the caret opens it, revealing "Manhattan", which in turn reveals "Succubus Club".
- Clicking a row highlights it (active state) and updates the center pane to show its id.
- Type suffix appears in muted text (e.g. "area", "business").

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/domains/DomainTree.svelte src/tools/DomainsManager.svelte
git commit -m "feat(domains): add read-only DomainTree driven by contains edges"
```

---

## Task 6: Build `NodeDetail` in read mode (without properties yet)

**Files:**
- Create: `src/lib/components/domains/NodeDetail.svelte`
- Modify: `src/tools/DomainsManager.svelte`

**Purpose:** Center pane showing the currently-selected node's label, type chip, tags, and description. Breadcrumb is deferred to Task 12; properties to Task 8.

- [ ] **Step 1: Create `NodeDetail.svelte`**

Write `src/lib/components/domains/NodeDetail.svelte`:

```svelte
<script lang="ts">
  import { session, cache } from '../../../store/domains.svelte';

  const node = $derived(cache.nodes.find(n => n.id === session.nodeId) ?? null);
</script>

<section class="detail">
  {#if !node}
    <p class="muted">No node selected. Click one in the tree to view its details.</p>
  {:else}
    <header class="head">
      <span class="title">{node.label}</span>
      <span class="type-chip">{node.type}</span>
      <span class="spacer"></span>
      <button class="btn" disabled title="Edit mode lands in Task 9">✎ Edit</button>
    </header>

    {#if node.tags.length > 0}
      <div class="tags">
        {#each node.tags as tag (tag)}
          <span class="tag">{tag}</span>
        {/each}
      </div>
    {/if}

    {#if node.description}
      <p class="desc">{node.description}</p>
    {:else}
      <p class="desc muted">(no description)</p>
    {/if}

    <!-- Properties panel lands in Task 8. -->
  {/if}
</section>

<style>
  .detail { padding: 0.9rem 1rem; display: flex; flex-direction: column; gap: 0.55rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.8rem; }
  .head { display: flex; align-items: center; gap: 0.5rem; }
  .title { font-size: 1.1rem; font-weight: 600; color: var(--text-primary); }
  .type-chip {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
    background: var(--bg-raised);
    border: 1px solid var(--border-card);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
  }
  .spacer { flex: 1; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-muted);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .tags { display: flex; gap: 0.3rem; flex-wrap: wrap; }
  .tag {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    font-size: 0.6rem;
    padding: 0.1rem 0.4rem;
    color: var(--text-label);
  }
  .desc {
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.55;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
    white-space: pre-wrap;
  }
</style>
```

- [ ] **Step 2: Slot `NodeDetail` into `DomainsManager.svelte`**

Replace the `<main class="detail-placeholder">…</main>` block in `src/tools/DomainsManager.svelte` with:

```svelte
      <NodeDetail />
```

And add the import near the top of the `<script>`:

```ts
  import NodeDetail from '$lib/components/domains/NodeDetail.svelte';
```

The `.detail-placeholder` CSS class becomes unused — remove it from the `<style>` block. The surrounding `<main>` tag and the "not implemented yet" placeholder copy disappear with the replacement.

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test**

Run: `npm run tauri dev`
Expected:
- Click a tree node → center pane shows its label (big), type chip (small uppercase), tags (if any), and description.
- Select the Edit button is visible but disabled with a tooltip.
- Click a different node → detail updates.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/domains/NodeDetail.svelte src/tools/DomainsManager.svelte
git commit -m "feat(domains): add read-mode NodeDetail (label/type/tags/description)"
```

---

## Task 7: Build property widgets, registry, and `PropertyEditor`

**Files:**
- Create: `src/lib/components/domains/property-widgets/StringWidget.svelte`
- Create: `src/lib/components/domains/property-widgets/TextWidget.svelte`
- Create: `src/lib/components/domains/property-widgets/NumberWidget.svelte`
- Create: `src/lib/components/domains/property-widgets/BoolWidget.svelte`
- Create: `src/lib/components/domains/property-widgets/index.ts`
- Create: `src/lib/components/domains/PropertyEditor.svelte`

**Widget contract:**
```ts
{
  field: Field;           // The current field (name + value).
  readonly: boolean;      // Read mode → display the value; edit mode → render an input.
  onchange?: (updated: Field) => void;  // Edit mode only — emit the new Field shape.
}
```

Widgets must ignore `multi` variants (not in v1): `StringWidget` and `NumberWidget` handle only the `Single` branch. If a `Multi` value sneaks in, fall back to reading/writing its first element.

- [ ] **Step 1: Create `StringWidget.svelte`**

Write `src/lib/components/domains/property-widgets/StringWidget.svelte`:

```svelte
<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  function currentString(): string {
    if (field.type !== 'string') return '';
    return Array.isArray(field.value) ? (field.value[0] ?? '') : field.value;
  }

  function emit(next: string) {
    onchange?.({ name: field.name, type: 'string', value: next });
  }
</script>

{#if readonly}
  <span class="value">{currentString()}</span>
{:else}
  <input
    class="input"
    type="text"
    value={currentString()}
    oninput={(e) => emit((e.target as HTMLInputElement).value)}
  />
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; }
  .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.28rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    width: 100%;
    outline: none;
    font-family: inherit;
  }
  .input:focus { border-color: var(--accent); }
</style>
```

- [ ] **Step 2: Create `TextWidget.svelte`**

Write `src/lib/components/domains/property-widgets/TextWidget.svelte`:

```svelte
<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  const text = $derived(field.type === 'text' ? field.value : '');
</script>

{#if readonly}
  <span class="value">{text}</span>
{:else}
  <textarea
    class="input"
    rows={3}
    value={text}
    oninput={(e) => onchange?.({ name: field.name, type: 'text', value: (e.target as HTMLTextAreaElement).value })}
  ></textarea>
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; white-space: pre-wrap; }
  .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.28rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    width: 100%;
    resize: vertical;
    outline: none;
    font-family: inherit;
  }
  .input:focus { border-color: var(--accent); }
</style>
```

- [ ] **Step 3: Create `NumberWidget.svelte`**

Write `src/lib/components/domains/property-widgets/NumberWidget.svelte`:

```svelte
<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  function currentNumber(): number {
    if (field.type !== 'number') return 0;
    return Array.isArray(field.value) ? (field.value[0] ?? 0) : field.value;
  }
</script>

{#if readonly}
  <span class="value">{currentNumber()}</span>
{:else}
  <input
    class="input"
    type="number"
    value={currentNumber()}
    oninput={(e) => {
      const v = Number((e.target as HTMLInputElement).value);
      onchange?.({ name: field.name, type: 'number', value: Number.isFinite(v) ? v : 0 });
    }}
  />
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; }
  .input {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.28rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    width: 6rem;
    outline: none;
    font-family: inherit;
  }
  .input:focus { border-color: var(--accent); }
</style>
```

- [ ] **Step 4: Create `BoolWidget.svelte`**

Write `src/lib/components/domains/property-widgets/BoolWidget.svelte`:

```svelte
<script lang="ts">
  import type { Field } from '../../../../types';

  const { field, readonly, onchange }: {
    field: Field;
    readonly: boolean;
    onchange?: (f: Field) => void;
  } = $props();

  const bool = $derived(field.type === 'bool' ? field.value : false);
</script>

{#if readonly}
  <span class="value">{bool ? 'true' : 'false'}</span>
{:else}
  <input
    type="checkbox"
    checked={bool}
    onchange={(e) => onchange?.({ name: field.name, type: 'bool', value: (e.target as HTMLInputElement).checked })}
  />
{/if}

<style>
  .value { color: var(--text-primary); font-size: 0.76rem; }
</style>
```

- [ ] **Step 5: Create the widget registry `index.ts`**

Write `src/lib/components/domains/property-widgets/index.ts`:

```ts
import type { Component } from 'svelte';
import type { FieldValue } from '../../../../types';
import StringWidget from './StringWidget.svelte';
import TextWidget from './TextWidget.svelte';
import NumberWidget from './NumberWidget.svelte';
import BoolWidget from './BoolWidget.svelte';

// Widget for a given FieldValue discriminator. Adding a new supported type
// (date / url / email / reference) is a two-line change: import the widget,
// add an entry here.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const WIDGETS: Partial<Record<FieldValue['type'], Component<any>>> = {
  string: StringWidget,
  text:   TextWidget,
  number: NumberWidget,
  bool:   BoolWidget,
};

export const SUPPORTED_TYPES = Object.keys(WIDGETS) as Array<FieldValue['type']>;
```

- [ ] **Step 6: Create `PropertyEditor.svelte`**

Write `src/lib/components/domains/PropertyEditor.svelte`:

```svelte
<script lang="ts">
  import type { Field } from '../../../types';
  import { WIDGETS } from './property-widgets';

  const { field, readonly, onchange, onremove }: {
    field: Field;
    readonly: boolean;
    onchange?: (updated: Field) => void;
    onremove?: () => void;
  } = $props();

  const Widget = $derived(WIDGETS[field.type] ?? null);
</script>

<div class="row">
  <span class="name">{field.name}</span>
  <div class="widget">
    {#if Widget}
      <Widget {field} {readonly} {onchange} />
    {:else}
      <span class="unsupported">Unsupported type: {field.type}</span>
    {/if}
  </div>
  {#if !readonly && onremove}
    <button class="remove" onclick={onremove} title="Remove field">✕</button>
  {/if}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 8rem 1fr auto;
    align-items: center;
    gap: 0.5rem;
    padding: 0.2rem 0;
  }
  .name { color: var(--text-muted); font-size: 0.68rem; }
  .widget { min-width: 0; }
  .unsupported { color: var(--text-ghost); font-size: 0.7rem; font-style: italic; }
  .remove {
    background: none;
    border: 1px solid var(--border-faint);
    color: var(--text-ghost);
    border-radius: 3px;
    padding: 0.05rem 0.25rem;
    font-size: 0.6rem;
    cursor: pointer;
  }
  .remove:hover { color: var(--accent); border-color: var(--accent); }
</style>
```

- [ ] **Step 7: Type-check**

Run: `npm run check`
Expected: 0 errors. (The widgets and editor aren't rendered yet — they're plumbed into `NodeDetail` in Task 8.)

- [ ] **Step 8: Commit**

```bash
git add src/lib/components/domains/property-widgets src/lib/components/domains/PropertyEditor.svelte
git commit -m "feat(domains): add property widgets and PropertyEditor registry"
```

---

## Task 8: Wire `PropertyEditor` into `NodeDetail` read mode

**Files:**
- Modify: `src/lib/components/domains/NodeDetail.svelte`

- [ ] **Step 1: Add a properties section to `NodeDetail.svelte`**

Open `src/lib/components/domains/NodeDetail.svelte`. Replace the whole file with:

```svelte
<script lang="ts">
  import PropertyEditor from './PropertyEditor.svelte';
  import { session, cache } from '../../../store/domains.svelte';

  const node = $derived(cache.nodes.find(n => n.id === session.nodeId) ?? null);
</script>

<section class="detail">
  {#if !node}
    <p class="muted">No node selected. Click one in the tree to view its details.</p>
  {:else}
    <header class="head">
      <span class="title">{node.label}</span>
      <span class="type-chip">{node.type}</span>
      <span class="spacer"></span>
      <button class="btn" disabled title="Edit mode lands in Task 9">✎ Edit</button>
    </header>

    {#if node.tags.length > 0}
      <div class="tags">
        {#each node.tags as tag (tag)}
          <span class="tag">{tag}</span>
        {/each}
      </div>
    {/if}

    {#if node.description}
      <p class="desc">{node.description}</p>
    {:else}
      <p class="desc muted">(no description)</p>
    {/if}

    <div class="props">
      <div class="props-label">Properties</div>
      {#if node.properties.length === 0}
        <p class="muted small">(none)</p>
      {:else}
        {#each node.properties as f (f.name)}
          <PropertyEditor field={f} readonly={true} />
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .detail { padding: 0.9rem 1rem; display: flex; flex-direction: column; gap: 0.55rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.8rem; }
  .muted.small { font-size: 0.7rem; }
  .head { display: flex; align-items: center; gap: 0.5rem; }
  .title { font-size: 1.1rem; font-weight: 600; color: var(--text-primary); }
  .type-chip {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
    background: var(--bg-raised);
    border: 1px solid var(--border-card);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
  }
  .spacer { flex: 1; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-muted);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .tags { display: flex; gap: 0.3rem; flex-wrap: wrap; }
  .tag {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    font-size: 0.6rem;
    padding: 0.1rem 0.4rem;
    color: var(--text-label);
  }
  .desc {
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.55;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
    white-space: pre-wrap;
  }
  .props { border-top: 1px solid var(--border-faint); padding-top: 0.5rem; }
  .props-label {
    font-size: 0.55rem;
    color: #7c9;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.25rem;
  }
</style>
```

- [ ] **Step 2: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 3: Smoke-test (seed a node with properties via devtools)**

Run: `npm run tauri dev`. In devtools console:

```js
const { invoke } = window.__TAURI__.core;
const cid = /* chronicle id */;
await invoke('create_node', {
  chronicleId: cid, nodeType: 'business', label: 'Test Club',
  description: 'A test venue.',
  tags: ['nightclub', 'feeding-ground'],
  properties: [
    { name: 'capacity',  type: 'number', value: 300 },
    { name: 'public',    type: 'bool',   value: true },
    { name: 'tagline',   type: 'string', value: 'Where the blood flows' },
    { name: 'notes',     type: 'text',   value: 'Multi-line\nnotes here.' },
  ],
});
```

Reload the app. Click the new node. Expected:
- Properties section renders four rows: capacity (300), public (true), tagline (with string), notes (multi-line text, preserving newlines).
- All rows are read-only (no inputs).

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/domains/NodeDetail.svelte
git commit -m "feat(domains): render properties in NodeDetail read mode via PropertyEditor"
```

---

## Task 9: Build `NodeForm` and wire create/edit flows

**Files:**
- Create: `src/lib/components/domains/NodeForm.svelte`
- Modify: `src/lib/components/domains/NodeDetail.svelte`
- Modify: `src/lib/components/domains/DomainTree.svelte`

**Purpose:** A single form component used in two cases:
1. Edit — fills from an existing node, patches it via `updateNode`.
2. Create — runs with a `parentId` (root = `null`), produces a new node and optionally a `contains` edge.

- [ ] **Step 1: Create `NodeForm.svelte`**

Write `src/lib/components/domains/NodeForm.svelte`:

```svelte
<script lang="ts">
  import { untrack } from 'svelte';
  import type { ChronicleNode, Field, FieldValue } from '../../../types';
  import * as api from '../../domains/api';
  import { session, cache, refreshNodes, refreshEdges, selectNode } from '../../../store/domains.svelte';
  import PropertyEditor from './PropertyEditor.svelte';
  import { SUPPORTED_TYPES } from './property-widgets';

  const { node = null, parentId = null, oncancel, onsave }: {
    node?: ChronicleNode | null;
    parentId?: number | null;
    oncancel: () => void;
    onsave: (saved: ChronicleNode) => void;
  } = $props();

  let label       = $state(untrack(() => node?.label ?? ''));
  let nodeType    = $state(untrack(() => node?.type ?? 'area'));
  let description = $state(untrack(() => node?.description ?? ''));
  let tagsText    = $state(untrack(() => (node?.tags ?? []).join(', ')));
  let properties  = $state<Field[]>(untrack(() => structuredClone(node?.properties ?? [])));

  let newPropName = $state('');
  let newPropType = $state<FieldValue['type']>('string');

  let saving = $state(false);
  let localError = $state('');

  // Known types suggested from existing cache (for the autocomplete list).
  const knownTypes = $derived(
    Array.from(new Set(cache.nodes.map(n => n.type))).sort()
  );

  function defaultValueFor(type: FieldValue['type']): Field {
    switch (type) {
      case 'string': return { name: newPropName.trim(), type: 'string', value: '' };
      case 'text':   return { name: newPropName.trim(), type: 'text',   value: '' };
      case 'number': return { name: newPropName.trim(), type: 'number', value: 0 };
      case 'bool':   return { name: newPropName.trim(), type: 'bool',   value: false };
      default:       return { name: newPropName.trim(), type: 'string', value: '' };
    }
  }

  function addProperty() {
    const name = newPropName.trim();
    if (!name) { localError = 'Property name is required.'; return; }
    if (properties.some(p => p.name === name)) { localError = 'Property name already used.'; return; }
    properties = [...properties, defaultValueFor(newPropType)];
    newPropName = '';
    localError = '';
  }

  function updateProperty(index: number, updated: Field) {
    properties = properties.map((p, i) => (i === index ? updated : p));
  }

  function removeProperty(index: number) {
    properties = properties.filter((_, i) => i !== index);
  }

  async function save() {
    if (!label.trim()) { localError = 'Label is required.'; return; }
    if (!nodeType.trim()) { localError = 'Type is required.'; return; }
    if (session.chronicleId == null) { localError = 'No chronicle selected.'; return; }

    const tags = tagsText.split(',').map(t => t.trim()).filter(Boolean);

    saving = true;
    localError = '';
    try {
      let saved: ChronicleNode;
      if (node) {
        saved = await api.updateNode(
          node.id, nodeType.trim(), label.trim(), description, tags, properties,
        );
      } else {
        saved = await api.createNode(
          session.chronicleId, nodeType.trim(), label.trim(), description, tags, properties,
        );
        if (parentId != null) {
          try {
            await api.createEdge(
              session.chronicleId, parentId, saved.id, 'contains', '', [],
            );
          } catch (e) {
            localError = `Node created, but linking to parent failed: ${e}`;
          }
        }
      }
      await refreshNodes();
      await refreshEdges();
      selectNode(saved.id);
      onsave(saved);
    } catch (e) {
      localError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form">
  <div class="form-title">{node ? 'Edit node' : (parentId != null ? 'Add child node' : 'Add root node')}</div>

  <div class="field">
    <label for="nf-label">Label</label>
    <input id="nf-label" bind:value={label} placeholder="e.g. Manhattan" />
  </div>

  <div class="field">
    <label for="nf-type">Type</label>
    <input id="nf-type" list="nf-types" bind:value={nodeType} placeholder="area, character, business…" />
    <datalist id="nf-types">
      {#each knownTypes as t (t)}
        <option value={t}></option>
      {/each}
    </datalist>
  </div>

  <div class="field">
    <label for="nf-tags">Tags (comma-separated)</label>
    <input id="nf-tags" bind:value={tagsText} placeholder="nightclub, feeding-ground" />
  </div>

  <div class="field">
    <label for="nf-desc">Description</label>
    <textarea id="nf-desc" bind:value={description} rows={4}></textarea>
  </div>

  <div class="props">
    <div class="props-label">Properties</div>
    {#each properties as p, i (p.name)}
      <PropertyEditor
        field={p}
        readonly={false}
        onchange={(updated) => updateProperty(i, updated)}
        onremove={() => removeProperty(i)}
      />
    {/each}

    <div class="add-prop">
      <input class="small" bind:value={newPropName} placeholder="new property name" />
      <select class="small" bind:value={newPropType}>
        {#each SUPPORTED_TYPES as t (t)}
          <option value={t}>{t}</option>
        {/each}
      </select>
      <button class="btn" onclick={addProperty}>+ Add property</button>
    </div>
  </div>

  {#if localError}
    <p class="error">{localError}</p>
  {/if}

  <div class="actions">
    <button class="btn" onclick={oncancel} disabled={saving}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>

<style>
  .form {
    border: 1px solid var(--border-active);
    background: var(--bg-card);
    border-radius: 6px;
    padding: 0.8rem;
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .form-title {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-label);
  }
  .field { display: flex; flex-direction: column; gap: 0.2rem; }
  label {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  input, select, textarea {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    color: var(--text-primary);
    font-size: 0.76rem;
    font-family: inherit;
    outline: none;
  }
  input:focus, select:focus, textarea:focus { border-color: var(--accent); }
  textarea { resize: vertical; min-height: 4rem; }
  .small { font-size: 0.7rem; padding: 0.25rem 0.4rem; }
  .props {
    border-top: 1px solid var(--border-faint);
    padding-top: 0.45rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .props-label {
    font-size: 0.55rem;
    color: #7c9;
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }
  .add-prop {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.35rem;
    align-items: center;
    padding-top: 0.3rem;
  }
  .actions { display: flex; justify-content: flex-end; gap: 0.4rem; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.3rem 0.75rem;
    font-size: 0.74rem;
    cursor: pointer;
  }
  .btn:hover:not(:disabled) { color: var(--text-primary); }
  .btn.primary { color: var(--accent); }
  .btn.primary:hover { box-shadow: 0 0 8px #cc222255; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .error { font-size: 0.7rem; color: var(--accent-bright); }
</style>
```

- [ ] **Step 2: Wire edit mode in `NodeDetail.svelte`**

Replace the whole file with:

```svelte
<script lang="ts">
  import PropertyEditor from './PropertyEditor.svelte';
  import NodeForm from './NodeForm.svelte';
  import { session, cache } from '../../../store/domains.svelte';

  const node = $derived(cache.nodes.find(n => n.id === session.nodeId) ?? null);

  let editing = $state(false);

  // Exit edit mode when the selected node changes.
  $effect(() => {
    session.nodeId;
    editing = false;
  });
</script>

<section class="detail">
  {#if !node}
    <p class="muted">No node selected. Click one in the tree to view its details.</p>
  {:else if editing}
    <NodeForm
      node={node}
      oncancel={() => editing = false}
      onsave={() => editing = false}
    />
  {:else}
    <header class="head">
      <span class="title">{node.label}</span>
      <span class="type-chip">{node.type}</span>
      <span class="spacer"></span>
      <button class="btn" onclick={() => editing = true}>✎ Edit</button>
    </header>

    {#if node.tags.length > 0}
      <div class="tags">
        {#each node.tags as tag (tag)}
          <span class="tag">{tag}</span>
        {/each}
      </div>
    {/if}

    {#if node.description}
      <p class="desc">{node.description}</p>
    {:else}
      <p class="desc muted">(no description)</p>
    {/if}

    <div class="props">
      <div class="props-label">Properties</div>
      {#if node.properties.length === 0}
        <p class="muted small">(none)</p>
      {:else}
        {#each node.properties as f (f.name)}
          <PropertyEditor field={f} readonly={true} />
        {/each}
      {/if}
    </div>
  {/if}
</section>

<style>
  .detail { padding: 0.9rem 1rem; display: flex; flex-direction: column; gap: 0.55rem; overflow: auto; }
  .muted { color: var(--text-ghost); font-size: 0.8rem; }
  .muted.small { font-size: 0.7rem; }
  .head { display: flex; align-items: center; gap: 0.5rem; }
  .title { font-size: 1.1rem; font-weight: 600; color: var(--text-primary); }
  .type-chip {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-label);
    background: var(--bg-raised);
    border: 1px solid var(--border-card);
    border-radius: 3px;
    padding: 0.1rem 0.35rem;
  }
  .spacer { flex: 1; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--accent-amber);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.72rem;
    cursor: pointer;
  }
  .btn:hover { box-shadow: 0 0 8px #cc992255; }
  .tags { display: flex; gap: 0.3rem; flex-wrap: wrap; }
  .tag {
    background: var(--bg-card);
    border: 1px solid var(--border-card);
    border-radius: 8px;
    font-size: 0.6rem;
    padding: 0.1rem 0.4rem;
    color: var(--text-label);
  }
  .desc {
    color: var(--text-secondary);
    font-size: 0.78rem;
    line-height: 1.55;
    border-top: 1px solid var(--border-faint);
    padding-top: 0.5rem;
    white-space: pre-wrap;
  }
  .props { border-top: 1px solid var(--border-faint); padding-top: 0.5rem; }
  .props-label {
    font-size: 0.55rem;
    color: #7c9;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    margin-bottom: 0.25rem;
  }
</style>
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test edit flow**

Run: `npm run tauri dev`. Click a node → click ✎ Edit. Expected:
- Form appears with label/type/tags/description/properties populated from the node.
- Change the label, click Save. Detail view returns, showing the new label. Sidebar tree also updates.
- Re-open edit → click Cancel. Detail restores without any change.
- In edit mode, click "+ Add property" after filling a name and choosing a type. New row appears. Save. Re-select the node; property persists.
- Remove a property via ✕. Save. Property is gone.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/domains/NodeForm.svelte src/lib/components/domains/NodeDetail.svelte
git commit -m "feat(domains): add NodeForm and edit mode for NodeDetail"
```

---

## Task 10: Wire node delete and root-node creation

**Files:**
- Modify: `src/lib/components/domains/NodeDetail.svelte`
- Modify: `src/lib/components/domains/DomainTree.svelte`

**Purpose:**
- Add a "✕ Delete" button to `NodeDetail`. If the node has children (outgoing `contains` edges), confirm with a cascade warning.
- Wire the tree's "+ Add root node" action.
- Wire a per-row "+ child" action in `DomainTree` (hover-surfaced) to create children.

- [ ] **Step 1: Add delete action to `NodeDetail.svelte`**

In `src/lib/components/domains/NodeDetail.svelte`, replace the `<script lang="ts">` block with:

```svelte
<script lang="ts">
  import PropertyEditor from './PropertyEditor.svelte';
  import NodeForm from './NodeForm.svelte';
  import * as api from '../../domains/api';
  import { session, cache, refreshNodes, refreshEdges, selectNode } from '../../../store/domains.svelte';

  const node = $derived(cache.nodes.find(n => n.id === session.nodeId) ?? null);

  const childCount = $derived(
    node == null
      ? 0
      : cache.edges.filter(e => e.edge_type === 'contains' && e.from_node_id === node.id).length
  );

  let editing = $state(false);
  let deleting = $state(false);

  $effect(() => {
    session.nodeId;
    editing = false;
  });

  async function handleDelete() {
    if (!node) return;
    const msg = childCount > 0
      ? `Delete "${node.label}"? ${childCount} child node${childCount === 1 ? '' : 's'} will also be removed (cascade). This cannot be undone.`
      : `Delete "${node.label}"? This cannot be undone.`;
    if (!confirm(msg)) return;
    deleting = true;
    try {
      await api.deleteNode(node.id);
      selectNode(null);
      await refreshNodes();
      await refreshEdges();
    } catch (e) {
      alert(`Delete failed: ${e}`);
    } finally {
      deleting = false;
    }
  }
</script>
```

Then in the same file, replace the header's action buttons with:

```svelte
    <header class="head">
      <span class="title">{node.label}</span>
      <span class="type-chip">{node.type}</span>
      <span class="spacer"></span>
      <button class="btn" onclick={() => editing = true}>✎ Edit</button>
      <button class="btn danger" onclick={handleDelete} disabled={deleting}>
        {deleting ? 'Deleting…' : '✕ Delete'}
      </button>
    </header>
```

And add a `.btn.danger` style in the `<style>` block, next to the existing `.btn`:

```css
  .btn.danger { color: var(--accent); }
  .btn.danger:hover { box-shadow: 0 0 8px #cc222255; }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
```

- [ ] **Step 2: Wire "+ Add root node" and per-row "+ child" in `DomainTree.svelte`**

Replace the entire file with:

```svelte
<script lang="ts">
  import type { ChronicleNode } from '../../../types';
  import { session, cache, selectNode } from '../../../store/domains.svelte';
  import NodeForm from './NodeForm.svelte';

  // Nodes that appear in the tree root: those with no incoming 'contains' edge.
  const rootNodes = $derived(computeRoots());
  const childrenByParent = $derived(computeChildrenMap());
  const expanded: Set<number> = $state(new Set());

  let adding = $state<'root' | number | null>(null); // 'root' or parent node id

  function computeRoots(): ChronicleNode[] {
    const containsTargets = new Set(
      cache.edges.filter(e => e.edge_type === 'contains').map(e => e.to_node_id)
    );
    return cache.nodes
      .filter(n => !containsTargets.has(n.id))
      .sort((a, b) => a.label.localeCompare(b.label));
  }

  function computeChildrenMap(): Map<number, ChronicleNode[]> {
    const byId = new Map<number, ChronicleNode>();
    for (const n of cache.nodes) byId.set(n.id, n);
    const m = new Map<number, ChronicleNode[]>();
    for (const e of cache.edges) {
      if (e.edge_type !== 'contains') continue;
      const child = byId.get(e.to_node_id);
      if (!child) continue;
      if (!m.has(e.from_node_id)) m.set(e.from_node_id, []);
      m.get(e.from_node_id)!.push(child);
    }
    for (const arr of m.values()) arr.sort((a, b) => a.label.localeCompare(b.label));
    return m;
  }

  function toggle(id: number) {
    if (expanded.has(id)) expanded.delete(id);
    else expanded.add(id);
  }

  function onFormDone() {
    adding = null;
  }
</script>

<aside class="tree">
  <div class="tree-header">Domains</div>

  {#if session.chronicleId == null}
    <p class="empty">No chronicle selected.</p>
  {:else}
    {#if adding === 'root'}
      <div class="inline-form">
        <NodeForm parentId={null} oncancel={onFormDone} onsave={onFormDone} />
      </div>
    {/if}

    {#if cache.nodes.length === 0 && adding !== 'root'}
      <p class="empty">This chronicle is empty. Create a node to begin.</p>
    {:else}
      <ul class="list">
        {#each rootNodes as node (node.id)}
          {@render treeRow(node, 0)}
        {/each}
      </ul>
    {/if}

    <div class="tree-footer">
      <button class="add-btn" onclick={() => adding = 'root'} disabled={adding !== null}>
        + Add root node
      </button>
    </div>
  {/if}
</aside>

{#snippet treeRow(node: ChronicleNode, depth: number)}
  {@const kids = childrenByParent.get(node.id) ?? []}
  {@const open = expanded.has(node.id)}
  <li>
    <div
      class="row-wrap"
      class:active={session.nodeId === node.id}
    >
      <button
        class="row"
        style="padding-left: calc(0.5rem + {depth * 0.8}rem)"
        onclick={() => selectNode(node.id)}
      >
        {#if kids.length > 0}
          <span
            class="caret"
            role="button"
            tabindex="0"
            onclick={(e) => { e.stopPropagation(); toggle(node.id); }}
            onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); e.stopPropagation(); toggle(node.id); } }}
          >{open ? '▾' : '▸'}</span>
        {:else}
          <span class="caret ghost">•</span>
        {/if}
        <span class="label">{node.label}</span>
        <span class="type">{node.type}</span>
      </button>
      <button
        class="add-child"
        title="Add child"
        onclick={(e) => { e.stopPropagation(); adding = node.id; }}
      >+</button>
    </div>

    {#if adding === node.id}
      <div class="inline-form nested" style="padding-left: calc(0.5rem + {(depth + 1) * 0.8}rem)">
        <NodeForm parentId={node.id} oncancel={onFormDone} onsave={onFormDone} />
      </div>
    {/if}

    {#if open && kids.length > 0}
      <ul class="list">
        {#each kids as child (child.id)}
          {@render treeRow(child, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
{/snippet}

<style>
  .tree {
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: auto;
    border-right: 1px solid var(--border-surface);
    background: var(--bg-sunken);
  }
  .tree-header {
    padding: 0.45rem 0.65rem;
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border-faint);
  }
  .empty { color: var(--text-ghost); font-size: 0.74rem; padding: 0.6rem 0.8rem; }
  .list { list-style: none; margin: 0; padding: 0; }
  .row-wrap {
    display: flex;
    align-items: center;
    gap: 0;
    border-left: 2px solid transparent;
  }
  .row-wrap.active {
    background: var(--bg-active);
    border-left-color: var(--accent);
  }
  .row-wrap:hover .add-child { opacity: 1; }
  .row {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 0;
    padding: 0.28rem 0.6rem;
    color: var(--text-secondary);
    font-size: 0.78rem;
    text-align: left;
    cursor: pointer;
    transition: color 0.12s;
  }
  .row-wrap:hover .row { color: var(--text-primary); }
  .row-wrap.active .row { color: var(--text-primary); }
  .caret {
    width: 0.85rem;
    color: var(--text-ghost);
    font-size: 0.65rem;
    user-select: none;
  }
  .caret.ghost { color: var(--border-faint); }
  .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .type {
    color: var(--text-ghost);
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.7;
  }
  .add-child {
    background: transparent;
    border: 0;
    color: var(--text-ghost);
    font-size: 0.9rem;
    padding: 0 0.5rem;
    opacity: 0;
    cursor: pointer;
    transition: opacity 0.12s, color 0.12s;
  }
  .add-child:hover { color: var(--accent); }
  .inline-form {
    padding: 0.5rem 0.65rem;
  }
  .inline-form.nested { padding-right: 0.5rem; }
  .tree-footer {
    margin-top: auto;
    padding: 0.4rem 0.65rem;
    border-top: 1px solid var(--border-faint);
  }
  .add-btn {
    background: none;
    border: 0;
    color: var(--text-ghost);
    font-size: 0.72rem;
    padding: 0;
    cursor: pointer;
    transition: color 0.12s;
  }
  .add-btn:hover:not(:disabled) { color: var(--accent); }
  .add-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test**

Run: `npm run tauri dev`. Expected:
- Hover a tree row → a dim "+" appears on the right. Click it → inline `NodeForm` appears nested under the row. Fill and save → new child node appears under its parent in the tree, and a `contains` edge is created.
- Click "+ Add root node" at the bottom → inline form appears at the top of the tree. Save → new root-level node appears.
- Click a node, then click "✕ Delete" in the detail pane. If it has children, the confirm mentions cascade. After confirming, the node and its subtree vanish from the tree. Detail pane returns to "No node selected."

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/domains/NodeDetail.svelte src/lib/components/domains/DomainTree.svelte
git commit -m "feat(domains): wire node create (root + child) and delete with cascade warning"
```

---

## Task 11: Build `EdgesPanel` and `EdgePicker`

**Files:**
- Create: `src/lib/components/domains/EdgesPanel.svelte`
- Create: `src/lib/components/domains/EdgePicker.svelte`
- Modify: `src/tools/DomainsManager.svelte`

- [ ] **Step 1: Create `EdgePicker.svelte`**

Write `src/lib/components/domains/EdgePicker.svelte`:

```svelte
<script lang="ts">
  import * as api from '../../domains/api';
  import { session, cache, refreshEdges } from '../../../store/domains.svelte';

  const { oncancel, onsave }: {
    oncancel: () => void;
    onsave: () => void;
  } = $props();

  let targetId = $state<number | ''>('');
  let edgeType = $state('');
  let description = $state('');
  let saving = $state(false);
  let localError = $state('');

  // Suggest edge-type strings from existing edges in the chronicle, excluding
  // 'contains' (contains is managed via the tree).
  const knownEdgeTypes = $derived(
    Array.from(new Set(
      cache.edges.map(e => e.edge_type).filter(t => t !== 'contains')
    )).sort()
  );

  // Candidate target nodes: every node except the current one.
  const candidates = $derived(
    cache.nodes
      .filter(n => n.id !== session.nodeId)
      .sort((a, b) => a.label.localeCompare(b.label))
  );

  async function save() {
    if (session.chronicleId == null || session.nodeId == null) { localError = 'No node selected.'; return; }
    if (targetId === '') { localError = 'Target node is required.'; return; }
    const et = edgeType.trim();
    if (!et) { localError = 'Edge type is required.'; return; }
    if (et === 'contains') { localError = 'Use the tree to manage contains relationships.'; return; }

    saving = true;
    localError = '';
    try {
      await api.createEdge(
        session.chronicleId, session.nodeId, Number(targetId), et, description, [],
      );
      await refreshEdges();
      onsave();
    } catch (e) {
      localError = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="form">
  <div class="form-title">Add relationship</div>

  <div class="field">
    <label for="ep-target">Target</label>
    <select id="ep-target" bind:value={targetId}>
      <option value="" disabled>Pick a node…</option>
      {#each candidates as c (c.id)}
        <option value={c.id}>{c.label} ({c.type})</option>
      {/each}
    </select>
  </div>

  <div class="field">
    <label for="ep-type">Type</label>
    <input id="ep-type" list="ep-types" bind:value={edgeType} placeholder="controls, allied-with…" />
    <datalist id="ep-types">
      {#each knownEdgeTypes as t (t)}
        <option value={t}></option>
      {/each}
    </datalist>
  </div>

  <div class="field">
    <label for="ep-desc">Description (optional)</label>
    <input id="ep-desc" bind:value={description} />
  </div>

  {#if localError}
    <p class="error">{localError}</p>
  {/if}

  <div class="actions">
    <button class="btn" onclick={oncancel} disabled={saving}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>
      {saving ? 'Saving…' : 'Save'}
    </button>
  </div>
</div>

<style>
  .form {
    border: 1px solid var(--border-active);
    background: var(--bg-card);
    border-radius: 6px;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .form-title {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-label);
  }
  .field { display: flex; flex-direction: column; gap: 0.15rem; }
  label {
    font-size: 0.55rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
  }
  input, select {
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    color: var(--text-primary);
    font-size: 0.72rem;
    outline: none;
    font-family: inherit;
  }
  input:focus, select:focus { border-color: var(--accent); }
  .actions { display: flex; gap: 0.35rem; justify-content: flex-end; }
  .btn {
    background: var(--bg-active);
    border: 1px solid var(--border-active);
    color: var(--text-label);
    border-radius: 4px;
    padding: 0.25rem 0.55rem;
    font-size: 0.68rem;
    cursor: pointer;
  }
  .btn.primary { color: var(--accent); }
  .btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .error { font-size: 0.65rem; color: var(--accent-bright); }
</style>
```

- [ ] **Step 2: Create `EdgesPanel.svelte`**

Write `src/lib/components/domains/EdgesPanel.svelte`:

```svelte
<script lang="ts">
  import type { ChronicleEdge } from '../../../types';
  import * as api from '../../domains/api';
  import { session, cache, selectNode, refreshEdges } from '../../../store/domains.svelte';
  import EdgePicker from './EdgePicker.svelte';

  let picking = $state(false);
  let localError = $state('');

  const outgoing = $derived(
    session.nodeId == null
      ? []
      : cache.edges.filter(e => e.from_node_id === session.nodeId && e.edge_type !== 'contains')
  );
  const incoming = $derived(
    session.nodeId == null
      ? []
      : cache.edges.filter(e => e.to_node_id === session.nodeId && e.edge_type !== 'contains')
  );

  function labelOf(id: number): string {
    return cache.nodes.find(n => n.id === id)?.label ?? `#${id}`;
  }

  async function removeEdge(e: ChronicleEdge) {
    if (!confirm(`Remove relationship "${e.edge_type}"?`)) return;
    try {
      await api.deleteEdge(e.id);
      await refreshEdges();
    } catch (err) {
      localError = String(err);
    }
  }
</script>

<aside class="panel">
  <div class="panel-header">Relationships</div>

  {#if session.nodeId == null}
    <p class="empty">Select a node to see its relationships.</p>
  {:else}
    {#if localError}<p class="error">{localError}</p>{/if}

    <div class="group">
      <div class="group-label">→ Outgoing</div>
      {#if outgoing.length === 0}
        <p class="empty small">(none)</p>
      {:else}
        {#each outgoing as e (e.id)}
          <div class="edge">
            <span class="edge-type">{e.edge_type}</span>
            <button class="linked" onclick={() => selectNode(e.to_node_id)}>{labelOf(e.to_node_id)}</button>
            <button class="remove" title="Remove" onclick={() => removeEdge(e)}>✕</button>
          </div>
        {/each}
      {/if}
    </div>

    <div class="group">
      <div class="group-label">← Incoming</div>
      {#if incoming.length === 0}
        <p class="empty small">(none)</p>
      {:else}
        {#each incoming as e (e.id)}
          <div class="edge">
            <span class="edge-type">{e.edge_type}</span>
            <button class="linked" onclick={() => selectNode(e.from_node_id)}>{labelOf(e.from_node_id)}</button>
            <button class="remove" title="Remove" onclick={() => removeEdge(e)}>✕</button>
          </div>
        {/each}
      {/if}
    </div>

    {#if picking}
      <EdgePicker
        oncancel={() => picking = false}
        onsave={() => picking = false}
      />
    {:else}
      <button class="add-btn" onclick={() => picking = true}>+ Add relationship</button>
    {/if}
  {/if}
</aside>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.65rem 0.7rem;
    border-left: 1px solid var(--border-surface);
    background: var(--bg-sunken);
    overflow: auto;
  }
  .panel-header {
    font-size: 0.6rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #7c9;
  }
  .group { display: flex; flex-direction: column; gap: 0.15rem; }
  .group-label {
    font-size: 0.55rem;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .empty { color: var(--text-ghost); font-size: 0.72rem; padding: 0.2rem 0; }
  .empty.small { font-size: 0.65rem; }
  .edge {
    display: grid;
    grid-template-columns: auto 1fr auto;
    gap: 0.4rem;
    align-items: center;
    padding: 0.15rem 0;
    font-size: 0.72rem;
    color: var(--text-secondary);
  }
  .edge-type { color: var(--text-muted); font-style: italic; }
  .linked {
    background: none;
    border: 0;
    padding: 0;
    color: #9cf;
    font-size: inherit;
    cursor: pointer;
    text-align: left;
  }
  .linked:hover { text-decoration: underline; }
  .remove {
    background: none;
    border: 1px solid var(--border-faint);
    color: var(--text-ghost);
    border-radius: 3px;
    padding: 0 0.25rem;
    font-size: 0.6rem;
    cursor: pointer;
  }
  .remove:hover { color: var(--accent); border-color: var(--accent); }
  .add-btn {
    background: none;
    border: 0;
    color: var(--text-ghost);
    font-size: 0.7rem;
    padding: 0.3rem 0;
    cursor: pointer;
    text-align: left;
    margin-top: auto;
  }
  .add-btn:hover { color: var(--accent); }
  .error { font-size: 0.65rem; color: var(--accent-bright); }
</style>
```

- [ ] **Step 3: Slot `EdgesPanel` into `DomainsManager.svelte`**

In `src/tools/DomainsManager.svelte`, add this import near the others:

```ts
  import EdgesPanel from '$lib/components/domains/EdgesPanel.svelte';
```

Replace the `<aside class="edges-placeholder">…</aside>` block with:

```svelte
      <EdgesPanel />
```

Remove the now-unused `.edges-placeholder` CSS class.

- [ ] **Step 4: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 5: Smoke-test**

Run: `npm run tauri dev`. Expected:
- Select a node with no non-contains edges: right panel shows "Outgoing: (none)", "Incoming: (none)".
- Click "+ Add relationship" → picker appears. Pick a target node, type "controls" as the edge type, save → appears in Outgoing.
- Select the target node → same edge appears in its Incoming list.
- Click the blue linked name → selection jumps to that node.
- Click ✕ on an edge, confirm → edge disappears.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/domains/EdgesPanel.svelte src/lib/components/domains/EdgePicker.svelte src/tools/DomainsManager.svelte
git commit -m "feat(domains): add EdgesPanel and EdgePicker for non-contains relationships"
```

---

## Task 12: Add tree search, breadcrumb, empty states

**Files:**
- Modify: `src/lib/components/domains/DomainTree.svelte`
- Modify: `src/lib/components/domains/NodeDetail.svelte`

- [ ] **Step 1: Add a search input at the top of `DomainTree.svelte`**

In the `<script>` block of `DomainTree.svelte`, add:

```ts
  let rawSearch = $state('');
  let searchQuery = $state('');
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  function onSearchInput(e: Event) {
    rawSearch = (e.target as HTMLInputElement).value;
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => { searchQuery = rawSearch; }, 110);
  }

  $effect(() => () => { if (searchTimer) clearTimeout(searchTimer); });

  // Filter: a node is visible if it matches or has a matching descendant.
  const visibleIds = $derived(computeVisibleIds());

  function computeVisibleIds(): Set<number> | null {
    const q = searchQuery.trim().toLowerCase();
    if (!q) return null; // null = show all

    const matches = new Set<number>();
    for (const n of cache.nodes) {
      const hit =
        n.label.toLowerCase().includes(q) ||
        n.description.toLowerCase().includes(q) ||
        n.tags.some(t => t.toLowerCase().includes(q));
      if (hit) matches.add(n.id);
    }
    // Include all ancestors of matches so they stay visible.
    const keep = new Set<number>(matches);
    const byChild = new Map<number, number>();
    for (const e of cache.edges) {
      if (e.edge_type === 'contains') byChild.set(e.to_node_id, e.from_node_id);
    }
    for (const id of matches) {
      let cur: number | undefined = byChild.get(id);
      while (cur != null) {
        keep.add(cur);
        cur = byChild.get(cur);
      }
    }
    return keep;
  }

  function isVisible(id: number): boolean {
    return visibleIds == null || visibleIds.has(id);
  }
```

Just below the `<div class="tree-header">Domains</div>` line, add:

```svelte
  <div class="search-wrap">
    <input
      class="search"
      type="text"
      value={rawSearch}
      oninput={onSearchInput}
      placeholder="🔍 Search this chronicle…"
    />
  </div>
```

Apply `isVisible(node.id)` as a guard inside the `#snippet treeRow`. Replace the `<li>` body start with:

```svelte
  {#if isVisible(node.id)}
  <li>
```

And the end of the snippet body (matching `{/if}` at the end):

```svelte
    {#if open && kids.length > 0}
      <ul class="list">
        {#each kids as child (child.id)}
          {@render treeRow(child, depth + 1)}
        {/each}
      </ul>
    {/if}
  </li>
  {/if}
{/snippet}
```

Add styles to the `<style>` block:

```css
  .search-wrap { padding: 0.35rem 0.5rem; border-bottom: 1px solid var(--border-faint); }
  .search {
    width: 100%;
    box-sizing: border-box;
    background: var(--bg-input);
    border: 1px solid var(--border-surface);
    border-radius: 4px;
    padding: 0.3rem 0.45rem;
    color: var(--text-primary);
    font-size: 0.74rem;
    outline: none;
  }
  .search:focus { border-color: var(--accent); }
  .search::placeholder { color: var(--text-ghost); }
```

- [ ] **Step 2: Add a breadcrumb to `NodeDetail.svelte`**

In the `<script>` of `NodeDetail.svelte`, add:

```ts
  import type { ChronicleNode } from '../../../types';

  // Walk the contains edges to build an ancestor list (top-down).
  const pathToRoot = $derived(computePath());

  function computePath(): ChronicleNode[] {
    if (!node) return [];
    const byId = new Map(cache.nodes.map(n => [n.id, n]));
    const byChild = new Map<number, number>();
    for (const e of cache.edges) {
      if (e.edge_type === 'contains') byChild.set(e.to_node_id, e.from_node_id);
    }
    const path: ChronicleNode[] = [];
    let cur: number | undefined = byChild.get(node.id);
    let guard = 0;
    while (cur != null && guard++ < 64) {
      const p = byId.get(cur);
      if (!p) break;
      path.unshift(p);
      cur = byChild.get(cur);
    }
    return path;
  }
```

Just above the `<header class="head">` line in the `!editing` branch, add:

```svelte
      {#if pathToRoot.length > 0}
        <nav class="crumbs" aria-label="breadcrumb">
          {#each pathToRoot as p, i (p.id)}
            <button class="crumb" onclick={() => selectNode(p.id)}>{p.label}</button>
            {#if i < pathToRoot.length}<span class="sep">▸</span>{/if}
          {/each}
        </nav>
      {/if}
```

And add a style:

```css
  .crumbs {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.65rem;
    color: var(--text-ghost);
  }
  .crumb {
    background: none;
    border: 0;
    color: var(--text-muted);
    font-size: inherit;
    padding: 0;
    cursor: pointer;
  }
  .crumb:hover { color: var(--text-primary); text-decoration: underline; }
  .sep { color: var(--text-ghost); }
```

- [ ] **Step 3: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 4: Smoke-test**

Run: `npm run tauri dev`. Expected:
- Tree has a search input at the top. Typing filters visible rows; ancestors stay visible so matches render in place. Empty query restores all rows.
- Selecting a deep child node shows a breadcrumb like "New York ▸ Manhattan ▸" in the detail pane. Clicking a crumb jumps to that ancestor.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/domains/DomainTree.svelte src/lib/components/domains/NodeDetail.svelte
git commit -m "feat(domains): add tree search and detail breadcrumb"
```

---

## Task 13: Error surfacing and contains-parent replacement flow

**Files:**
- Modify: `src/lib/components/domains/NodeForm.svelte`

**Purpose:** When creating a child node under a parent that already has a `contains` parent, the second-parent UNIQUE constraint fires. The user should see an actionable message rather than a cryptic SQLite error. Also: cycle-rejection errors should render cleanly.

The single-parent UNIQUE index on `(to_node_id) WHERE edge_type = 'contains'` means this scenario only happens when moving an existing node — not when creating a new one (new nodes have no prior parent). For v1, *creating* a child is always safe under the UNIQUE index; only *move-under* would trigger replacement. Move-under is deferred (spec). What we still need is clean cycle messaging and graceful UNIQUE errors from accidental repeated actions.

- [ ] **Step 1: Friendly error mapping in `NodeForm.svelte`**

In `NodeForm.svelte`, replace the `catch (e)` in `save()` with:

```ts
    } catch (e) {
      localError = friendlyError(String(e));
    } finally {
```

And add this helper above `save()`:

```ts
  function friendlyError(raw: string): string {
    if (raw.includes('cycle')) return 'Cannot link: this would create a loop under contains.';
    if (raw.includes('UNIQUE constraint failed')) {
      if (raw.includes('idx_edges_contains_single_parent')) {
        return 'That node already has a parent. Move-under is not supported in v1 — delete the existing contains edge first.';
      }
      return 'That relationship already exists.';
    }
    return raw;
  }
```

Also in `EdgePicker.svelte`, replace the `catch (e)` in `save()` with the same approach — add the same helper (keep the file self-contained; do not create a shared util in v1). Change `save()`'s catch to:

```ts
    } catch (e) {
      localError = friendlyError(String(e));
    } finally {
```

And add:

```ts
  function friendlyError(raw: string): string {
    if (raw.includes('cycle')) return 'Cannot link: this would create a loop under contains.';
    if (raw.includes('UNIQUE constraint failed')) return 'That relationship already exists.';
    return raw;
  }
```

- [ ] **Step 2: Type-check**

Run: `npm run check`
Expected: 0 errors.

- [ ] **Step 3: Smoke-test**

- Pick a node that already has a child. Add the same child again via the devtools console:
  ```js
  const { invoke } = window.__TAURI__.core;
  // parent and child ids — find them via cache
  await invoke('create_edge', { chronicleId: cid, fromNodeId: parent, toNodeId: child, edgeType: 'controls', description: '', properties: [] });
  ```
  Now try the same via the UI (EdgePicker) — it should show "That relationship already exists."
- Try to create a contains loop: from a great-grandchild back to a great-grandparent. Devtools console is fine here since EdgePicker blocks `contains`. Expected: friendly "Cannot link: this would create a loop…" if such a creation is driven through NodeForm with a `parentId` that is a descendant. For v1, the UI paths do not expose this, but the mapping is present for future flows.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/domains/NodeForm.svelte src/lib/components/domains/EdgePicker.svelte
git commit -m "feat(domains): friendlier cycle/UNIQUE error messages in forms"
```

---

## Task 14: Update docs

**Files:**
- Modify: `CLAUDE.md`
- Modify: `docs/design/data-sources.md`
- Modify: `docs/design/data-sources-kumu.json`

- [ ] **Step 1: Update `CLAUDE.md`**

Find the paragraph that begins:

```
Expected `verify.sh` warnings (not regressions): `shared/types.rs` types for the Domains Manager (`Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`) trigger "never constructed / never used" — they back migration `0002_chronicle_graph.sql` but aren't yet wired into Tauri commands.
```

Replace with:

```
Expected `verify.sh` warnings (not regressions): `shared/types.rs` types for the Domains Manager (`Chronicle`, `Node`, `Edge`, `Field`, `FieldValue`, `StringFieldValue`, `NumberFieldValue`, `EdgeDirection`) are now wired into both Tauri commands and the Svelte UI (Domains tool), so these specific types no longer trigger the warning. The v1 UI uses only `string`, `text`, `number`, and `bool` FieldValue variants — `date`, `url`, `email`, and `reference` may still surface "never constructed" until property widgets for them ship.
```

Near the end of the "Frontend (`src/`)" section, just before the `src/types.ts` bullet, add a bullet:

```
- **`src/store/domains.svelte.ts`** — runes-based UI state for the Domains Manager: current chronicle/node selection and cached `nodes`/`edges` lists. Intended to be importable from other tools for future cross-tool chronicle awareness.
```

Also just before the `src/store/toolEvents.ts` bullet, add a bullet:

```
- **`src/lib/domains/api.ts`** — typed wrappers around the 20 Tauri commands backing the Domains Manager. Every call from a Domains component goes through here; components never call `invoke` directly.
```

And in the "Frontend (`src/`)" section, in the bullet about `src/tools/*.svelte`, update the sentence that lists current tools:

```
- **`src/tools/*.svelte`** — one file per tool (e.g. `Resonance.svelte`). Each tool is an independent page-level component. `DyscrasiaManager.svelte` handles dyscrasia CRUD and random rolling. `Campaign.svelte` is the Roll20 viewer: it listens to `roll20://` events and uses a hardcoded `ATTR` constants map to resolve sheet attribute names from Roll20 Jumpgate character sheets. `DomainsManager.svelte` is the shell for the Domains Manager; it composes zone components from `src/lib/components/domains/`.
```

- [ ] **Step 2: Update `docs/design/data-sources.md`**

Find the Chronicle Store entry and update its "Consumers" / "Status" line to mention the Domains Manager UI. If the doc lists consumer status fields, change the Chronicle Store consumer from something like "none yet" to "Domains Manager (src/tools/DomainsManager.svelte and src/lib/components/domains/)". Preserve the existing structure — this is a one-line edit within the existing entry.

- [ ] **Step 3: Update `docs/design/data-sources-kumu.json`**

Find the Chronicle Store node and the Domains Manager node. For each connection between them currently marked as unwired / pending, change the connection's `status` property (or equivalent — look for the convention already used in this JSON) to reflect that the Domains Manager UI is now wired to the Chronicle Store. Preserve formatting.

- [ ] **Step 4: Smoke-check**

Run `./scripts/verify.sh` and confirm it passes without new warnings beyond the expected ones.

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md docs/design/data-sources.md docs/design/data-sources-kumu.json
git commit -m "docs: reflect Domains Manager UI v1 wiring"
```

---

## Task 15: Final verification

- [ ] **Step 1: Clean working tree**

Run: `git status`
Expected: no uncommitted changes.

- [ ] **Step 2: Aggregate gate**

Run: `./scripts/verify.sh`
Expected: exits 0.
- `npm run check` passes (0 Svelte/TS errors, warnings may exist for the two known `listen` imports per `CLAUDE.md`).
- `cargo test` passes all 44 tests.
- `npm run build` produces a build with no errors. Warnings for still-unused `FieldValue::Date/Url/Email/Reference`-related types are expected.

- [ ] **Step 3: End-to-end manual smoke**

Run: `npm run tauri dev`. Perform each of the following in sequence without reloading:

1. Sidebar shows four tools including "Domains".
2. Open Domains. "+ New" chronicle, name "Smoke Test", save. Dropdown shows it.
3. "+ Add root node", type label "City A", type "area", save. Appears in the tree.
4. Hover "City A" → click "+" → add child "District 1" type "area", save. Tree expands to show it.
5. Click "District 1" → "+ Add relationship" → pick "City A" as target, type "adjacent-to", save. (Expected failure? No — City A is a valid target; adjacent-to is not contains. Works.)
6. Edit "District 1" → add a property `capacity` type `number` value `42`, add `public` type `bool` checked, save.
7. Re-select "District 1" → detail pane shows both properties with their values.
8. Type "city" in tree search → only ancestors and matches remain visible.
9. Breadcrumb: "City A ▸" on "District 1".
10. Delete "City A" → confirm with cascade warning (1 child) → tree empties; detail returns to "No node selected".
11. Rename chronicle to "Smoke Test 2" → dropdown updates.
12. Delete chronicle → confirm → dropdown empties.

If every step behaves as described, the implementation is complete.

- [ ] **Step 4: Final commit (only if docs or polish edits were needed during smoke)**

If any files changed during smoke-testing (typo fixes, CSS tweaks, forgotten imports), commit them with a focused message. Otherwise skip this step.

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Task |
|---|---|
| Tool entry in `tools.ts` | 3 |
| `DomainsManager.svelte` shell | 3 (extended in 4, 5, 6, 8, 11) |
| `ChronicleHeader.svelte` with CRUD | 4 |
| Kumu export button disabled v1 | 4 |
| `DomainTree.svelte` with contains-only tree | 5 |
| `NodeDetail.svelte` read mode | 6, 8, 10, 12 |
| `NodeForm.svelte` | 9 |
| `EdgesPanel.svelte` + `EdgePicker.svelte` | 11 |
| `PropertyEditor.svelte` + widgets + registry | 7, 8 |
| `src/store/domains.svelte.ts` runes store | 2 |
| `src/lib/domains/api.ts` typed wrappers | 1 |
| Empty states (no chronicles, no nodes, no selection) | 3, 4, 5, 6, 11 |
| Delete guards (cascade confirm) | 10 |
| Cycle-error surfacing | 13 |
| Breadcrumb via path_to_root | 12 |
| Tree search, chronicle-scoped, client-side | 12 |
| Node-type autocomplete from cache | 9 |
| Edge-type autocomplete from cache | 11 |
| Widget registry extensibility | 7 |
| Docs updates | 14 |
| Kumu export (full feature) | **Deferred** per spec |
| "All nodes" sidebar tab | **Deferred** per spec |
| Reference/date/url/email/multi-value widgets | **Deferred** per spec |
| Icon-per-type | **Deferred** per spec |
| Drag-to-reparent, bulk ops, undo/redo | **Deferred** per spec |
| Move-under (atomic) | **Deferred** per spec |

All in-scope requirements are covered. No gaps.

**Type-consistency check:**
- `session.chronicleId`, `session.nodeId`, `cache.nodes`, `cache.edges`, `cache.chronicles` are used consistently across tasks 2–13.
- `WIDGETS` registry key type is `FieldValue['type']` in Task 7 and referenced the same way in Task 9's `SUPPORTED_TYPES`.
- API function names (`createNode`, `updateNode`, `deleteNode`, `createEdge`, `deleteEdge`, …) match between Task 1 and all later tasks.
- Prop interface for widgets (`field`, `readonly`, `onchange`) matches between Tasks 7, 8, and 9.
- `ChronicleNode`, `ChronicleEdge` TypeScript types (already in `src/types.ts`) used consistently.

No inconsistencies.

**Placeholder scan:** No "TBD"/"TODO"/"implement later"/"similar to"/"write tests for the above" — all code is concrete. Error handling is specific per task (friendlyError map in Task 13, cascade confirm in Task 10, inline errors in forms).
