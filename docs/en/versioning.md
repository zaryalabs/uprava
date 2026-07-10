# Uprava Versioning

Status: `active`

Uprava uses Semantic Versioning for implementation and release baselines:

```text
MAJOR.MINOR.PATCH
```

During the pre-`1.0.0` period, `MINOR` identifies a coherent product baseline
and `PATCH` identifies completed feature or fix slices on top of that baseline.
The public compatibility contract is still allowed to change before `1.0.0`,
but each version bump must make the repository state easier to reason about.

## Naming Rules

- Use `0.x.y` for pre-production development releases.
- Use `0.1.0` for the first shipped working baseline.
- Increment `PATCH` for completed implementation slices that do not redefine
  the product baseline.
- Increment `MINOR` when the product shape or architecture baseline changes
  enough that downstream docs and runbooks need a new baseline.
- Reserve `1.0.0` for the first production-ready compatibility and security
  contract.

## Product Cuts And Release Versions

`V01` is a product-cut name, not a SemVer version. It describes the first
coherent product scope that was shipped as `0.1.0`.

Release versions describe repository and implementation state. Current planning
documents should refer to the current release baseline, not to `V01`, when they
discuss features delivered after `0.1.0`.

Current baseline: `0.1.8`.

## 0.2.0 Release Candidates

- Every candidate build containing source changes uses a unique SemVer
  pre-release version `0.2.0-rc.N`; `N` increases monotonically and a candidate
  version is never rebuilt with different contents.
- Every candidate also has an immutable Git-SHA-based release id. Core, Web and
  Node artifacts for that candidate share the same version, Git SHA and release
  id.
- Release candidates are recorded in the temporary RC checklist and build
  manifest, not as the current shipped baseline in [`releases.md`](releases.md).
- The final `0.2.0` version is assigned only after the final RC passes the full
  clean-state release gate. The final build must pass that gate again. If it
  fails, discard it, return to the next `0.2.0-rc.N`, fix and repeat; never
  publish two different `0.2.0` artifacts.
- `0.2.0` is a coordinated breaking protocol-v2 release. Compatibility with
  0.1.x APIs, schemas or state and an in-place state migration are not release
  requirements.

## Update Rules

When a feature queue item or another large work block is completed:

1. Update [`releases.md`](releases.md) with the version and completed slice.
2. Update package metadata if the current implementation version changes.
3. Update any temporary plan that refers to outdated scope.
4. Promote durable product, architecture or process decisions into synchronized
   docs under `docs/en` and `docs/ru`.

Temporary plans may keep historical references to `V01`, but they must not use
`V01` as shorthand for the current implementation after post-`0.1.0` slices
have shipped.
