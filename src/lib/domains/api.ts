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
