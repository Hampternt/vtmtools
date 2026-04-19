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
