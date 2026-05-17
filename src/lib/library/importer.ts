import type { ImportOutcome } from '../../types';

export interface ImportSummary {
  inserted: number;
  updated: number;
  skipped: number;
  details: ImportOutcome[];
}

/**
 * Summarize a list of import outcomes for the UI toast.
 * Pure; no IPC; no side effects.
 */
export function summarizeImport(outcomes: ImportOutcome[]): ImportSummary {
  let inserted = 0;
  let updated = 0;
  let skipped = 0;
  for (const o of outcomes) {
    if (o.action === 'inserted') inserted++;
    else if (o.action === 'updated') updated++;
    else skipped++;
  }
  return { inserted, updated, skipped, details: outcomes };
}

/**
 * Format the summary as a single-line toast message.
 * Example: "Imported 4 new (2 updated, 1 skipped) from Chronicles of Chicago"
 */
export function summaryAsToast(summary: ImportSummary, worldTitle: string): string {
  const parts: string[] = [];
  parts.push(`Imported ${summary.inserted} new`);
  const suffixes: string[] = [];
  if (summary.updated > 0) suffixes.push(`${summary.updated} updated`);
  if (summary.skipped > 0) suffixes.push(`${summary.skipped} skipped`);
  if (suffixes.length > 0) parts.push(`(${suffixes.join(', ')})`);
  parts.push(`from ${worldTitle}`);
  return parts.join(' ');
}
