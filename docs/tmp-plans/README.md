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
  decision, that decision must be promoted into canonical documentation under
  `docs/en` and `docs/ru`.
- Do not use this section as the final roadmap. Long-lived sequencing should
  stay in `docs/en/feature-queue.md` and `docs/ru/feature-queue.md`.
- After a slice ships or is superseded, archive, replace or delete its
  temporary plan.

## Current Plans

- [`v0.1-implementation-plan.md`](v0.1-implementation-plan.md) - detailed
  implementation plan for the first `0.1` version based on the V01 product
  slice.
