# TMP Plans

Status: `active`

`TMP Plans` is a temporary documentation section for intermediate
implementation plans. These documents help turn an already designed product
slice into concrete implementation work, but they are not the canonical source
of product or architecture decisions.

Use this directory when a slice is already designed enough, but development
still needs a staged plan, task breakdown, release gate or temporary
coordination document.

## Rules

- Every temporary plan must link to the product or architecture documents it
  implements.
- Every temporary plan must have a status, scope and completion criteria.
- Temporary plans can be written in the working language that is most useful to
  the team at that stage.
- If a temporary plan records a durable product, architecture or process
  decision, that decision must be promoted into the canonical Russian
  documentation under `docs/`.
- Do not use this section as the final roadmap. Long-lived sequencing should
  stay in `docs/product/feature-queue.md`.
- After a slice ships or is superseded, archive, replace or delete its
  temporary plan.

## Current Plans

- [`web-design-system-alignment.md`](web-design-system-alignment.md)
  - implemented alignment of the production Web Control Panel with Zarya
    Design System 0.1, including tokens, shell, Dashboard, Agent Chat,
    workspace surfaces, accessibility and visual regression gates.

- [`0.2.0-completion-from-current.md`](0.2.0-completion-from-current.md)
  - sequential execution plan from the current `0.2.0` implementation to the
    automated RC handoff, user-owned live E2E gate and final release.
- [`0.2.0-quality-foundation.md`](0.2.0-quality-foundation.md)
  - authoritative requirements, finding coverage and release criteria for the
    0.2.0 quality foundation based on the 2026-07-09 audit.
