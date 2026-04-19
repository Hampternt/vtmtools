# 0003: Freeform strings for nodes.type and edges.edge_type

**Status:** accepted
**Date:** 2026-04-19

## Context

The Domains Manager stores a chronicle graph of nodes (areas, characters,
institutions, businesses, merits, and anything else a Storyteller wants
to track) and edges (contains, owns, knows, member-of, leads, and so on).
The schema question is whether `nodes.type` and `edges.edge_type` should
be enumerated (CHECK constraint or Rust enum) or left as freeform user-
authored strings.

## Decision

Both fields are freeform `TEXT` columns with no enumeration. The UI
derives autocomplete suggestions from the distinct values already present
in the current chronicle.

The sole exception is the `"contains"` edge type, which the UI uses as
its hierarchy/drilldown convention. A partial unique index
(`ON edges(chronicle_id, to_node_id) WHERE edge_type = 'contains'`)
enforces at most one `contains` parent per node. All other edge types
are unconstrained.

## Consequences

- Users can invent new node and edge types without a code change, a
  migration, or a release.
- The domain grammar of the Storyteller's world is authored by the
  Storyteller, not pre-committed by the tool.
- Typos (`"carachter"` vs `"character"`) can create phantom types —
  mitigated by the autocomplete UI, which surfaces existing types first.
- Pattern is consistent with the project's extensibility preference
  (pluggable over locked-in).

## Alternatives considered

- **Fixed Rust enum + CHECK constraint.** Rejected: every new type
  requires a migration and a code change; undermines the "Storyteller-
  authored world" posture.
- **Enum with an `Other(String)` fallback.** Rejected: adds complexity
  without meaningful benefit. Autocomplete over freeform strings gives
  the ergonomic wins of an enum without the schema rigidity.
- **JSON-schema-validated type taxonomy.** Rejected for v1 scope;
  potentially revisitable later as user-defined schemas per chronicle.
