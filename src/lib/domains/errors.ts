export function friendlyEdgeError(raw: string): string {
  if (raw.includes('cycle')) return 'Cannot link: this would create a loop under contains.';
  if (raw.includes('UNIQUE constraint failed')) {
    if (raw.includes('idx_edges_contains_single_parent')) {
      return 'That node already has a parent. Move-under is not supported in v1 — delete the existing contains edge first.';
    }
    return 'That relationship already exists.';
  }
  return raw;
}
