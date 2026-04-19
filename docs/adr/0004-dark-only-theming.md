# 0004: Dark-only theming

**Status:** accepted
**Date:** 2026-04-19

## Context

vtmtools targets Storytellers running V5 chronicles — a domain with a
strong aesthetic lean toward low-light, subdued UI. Supporting both light
and dark themes doubles the design and regression-test surface for every
component. The question is whether to invest in theming or commit to a
single palette.

## Decision

Dark-only. No theme toggle exists or will be added. All colors are CSS
custom properties defined once in `:global(:root)` inside
`src/routes/+layout.svelte`. Components reference tokens
(`--bg-base`, `--text-primary`, `--accent`, etc.) — hardcoded hex is
permitted only for transient states with no semantic equivalent (hover
intermediates, glow shadows).

## Consequences

- One palette to tune; design effort concentrates on a single
  well-calibrated look.
- Components are cheaper to author and review — no dual-theme reasoning.
- Users who prefer light UI are not served. Acceptable given the audience
  and use context (GMs running chronicles, often in dim settings).
- Future reversal would require retrofitting light-mode token variants
  and a runtime theme switch; cost is proportional to the component
  surface at that time.

## Alternatives considered

- **Dual-mode toggle.** Rejected — 2× design/test surface, misaligned
  with product posture.
- **System-followed theme (`prefers-color-scheme`).** Rejected — same
  2× surface cost; saves only the toggle UI.
- **User-configurable palette.** Rejected — scope creep; users can't
  tune four colors well, and "just make it dark" is the stated need.
