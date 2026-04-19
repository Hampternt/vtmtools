# 0002: Destructive reseed of non-custom dyscrasias on startup

**Status:** accepted
**Date:** 2026-04-19

## Context

Built-in dyscrasias (canonical V5 content) ship baked into the binary via
`src-tauri/src/db/seed.rs`. Users can also create custom dyscrasias
(flagged `is_custom = 1` in the `dyscrasias` table). The canonical seed
changes over time as descriptions are refined or entries are added. The
question: how do we reconcile seed-file changes with the live SQLite DB
on the user's machine without stomping their custom work?

## Decision

On every app start, `seed.rs` deletes all rows where `is_custom = 0` and
reinserts the full canonical set from the seed source. Rows with
`is_custom = 1` are never touched.

## Consequences

- Seed changes land automatically on next launch; users always see the
  current canonical dyscrasia data shipped with the build.
- Any edit a user makes to a built-in entry is discarded on next
  launch. This is intentional: edits should be made by forking the entry
  into a custom copy (new row with `is_custom = 1`), not by mutating the
  built-in.
- No migration is required when the seed changes — the next boot
  reconciles automatically.
- Row `id` values are AUTOINCREMENT and renumber on every startup when
  built-ins are deleted and reinserted. Code that references a built-in
  dyscrasia must use the stable natural key (`resonance_type` + `name`),
  not `id`. No tables currently declare a foreign key against
  `dyscrasias.id`, and any new such table should avoid doing so or rely
  on a stable natural-key column instead.

## Alternatives considered

- **Count-check guard (seed only when `dyscrasias` is empty).** Rejected:
  stale data after first run; every future seed change becomes invisible
  without manual DB surgery.
- **Per-seed-change migration.** Rejected: high engineering cost, easy to
  forget, brittle across app versions.
- **Merge-by-name with conflict resolution.** Rejected: "same name"
  semantics are ambiguous; makes the system harder to reason about for
  negligible user-facing benefit.
