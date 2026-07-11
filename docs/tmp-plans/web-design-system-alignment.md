# Web UI/UX Alignment With Zarya Design System 0.1

Status: `implemented`

Implemented: 2026-07-11

Release baseline: `0.2.1`

Scope: привести production Web Control Panel к локальной Zarya Design System
0.1 без изменения Core/Node protocol contracts и без функциональной
перестройки существующих workbench-сценариев.

## Sources

- Local source of truth: [Zarya Design System 0.1](../../.local/context/design/DESIGN.md).
- Local visual reference: `../../.local/context/design/prototype/` (`Dashboard`
  and `Agent Chat`).
- [A-004 Modular UI and Work Surface](../en/design/004-modular-ui-work-surface.md).
- [A-008 Go to Source and Causality UX](../en/design/008-go-to-source-and-causality-ux.md).
- [A-009 Human-Agent Dual Interface](../en/design/009-human-agent-dual-interface.md).
- [A-010 Project Workspace Surface](../en/design/010-project-workspace-surface.md).
- [Feature Inventory](../en/feature-inventory.md), especially `F-067`,
  `F-080`-`F-096`, `F-120`-`F-121`, and `F-140`-`F-142`.

The private design-system repository remains the authority for visual DNA,
tokens and accepted component patterns. If implementation needs a visual
decision not covered by Zarya 0.1, validate and record it there first instead
of silently inventing an Uprava-only variant.

## Current Baseline

The application already has the right structural foundation:

- stable three-area workbench shell;
- typed timeline blocks and safe fallback renderers;
- URL-addressable inspector stack;
- references for source, evidence and cause navigation;
- responsive Tailwind layout;
- reusable primitives for buttons, badges, textarea and error notices;
- Playwright golden-path tests for Dashboard, Agent Chat and Workspace.

The visual and interaction layer does not yet follow Zarya 0.1:

- `apps/web/src` contains 76 direct hex colors instead of the six base tokens
  and their documented derivatives;
- the product uses a green-tinted canvas and green primary action, while the
  design system requires a white/near-black monochrome base, black action,
  red risk and restrained violet notice;
- status is encoded through separate green, yellow, red and blue filled badge
  palettes, while normal, idle, running, review and warning should remain
  monochrome and differ primarily by text and glyph/form;
- 27 production files use rounded containers or shadows; Zarya 0.1 has square
  corners, flat surfaces and no nested card depth;
- Dashboard is composed as a grid of SaaS cards instead of a system overview,
  stable process map, annotated metrics and progressive disclosure;
- Agent Chat renders messages, approvals and errors as colored cards rather
  than phase/evidence/action layers with `+`/`-` nested disclosure;
- typography uses Inter/system defaults instead of the Zarya type grid and
  GOST Type A target;
- mobile currently reflows the shell, but dense tables, inspector depth and
  critical actions do not yet have an explicit mobile review model;
- loading, disabled, stale and error copy is inconsistent and often omits the
  reason or next safe action;
- accessibility foundations exist, but there is no skip link, async status
  announcement coverage is incomplete, some form metadata is missing, and
  motion does not yet honor `prefers-reduced-motion`.

## Target Experience

Uprava should read as an engineering work sheet, not a dashboard theme:

```text
stable shell
-> system state / affected scope / cause / next action
-> typed work objects and quiet normal state
-> explicit risk and review decisions
-> source / evidence / cause drill-down in the same surface
-> raw workspace or trace evidence only on demand
```

The migration must preserve current information architecture, command
authorization, deep links, safe fallbacks and Core-owned capabilities. This is
a UI-system migration, not a protocol redesign.

## Implementation Record

The production Web Control Panel now uses the Zarya 0.1 visual and interaction
vocabulary across all routes:

- the six base tokens, derived backgrounds and semantic lines live in
  `apps/web/src/styles.css`;
- shared `Button`, `Badge`, `Textarea`, `ErrorNotice`, page header, surface,
  empty/loading, caption and `+`/`-` disclosure primitives own common states;
- App Shell, inventory and URL-backed inspector use a flat three-area sheet,
  stable landmarks, a skip link and a maximum visible inspector depth of three;
- Dashboard is a system overview with cause, affected scope, next action,
  annotated metrics, runtime pipeline and dense attention/activity rows;
- Session uses explicit runtime context, work-phase timeline blocks, decision
  metadata for approvals, source/evidence disclosure and a labelled composer
  that preserves drafts and warns before navigation;
- Nodes, Projects, Placements, Runtime Settings, auth and Workspace Inspector
  consume the same monochrome tokens; Monaco and xterm retain documented dark
  domain palettes inside Zarya-aligned outer chrome;
- `npm run lint` rejects raw production colors, radii and shadows outside the
  token file and the two documented renderer exceptions;
- component tests cover the state/disclosure vocabulary, while Playwright owns
  desktop Dashboard, narrow Workspace and mobile Session baselines plus the
  shell/composer keyboard path.

GOST Type A remains a licensed local design asset and was not copied into the
application. Production uses the documented font name with the system fallback
stack until redistribution terms are confirmed. Product copy remains English;
localization is outside this slice.

## Execution Plan

### 0. Lock the migration contract

Deliverables:

- Create a screen/component matrix covering shell, Dashboard, Nodes,
  Placements, Projects, Session, Inspector and Workspace Inspector.
- For every component record role, anatomy, tokens, applicable states,
  keyboard behavior, responsive behavior and its prototype reference.
- Record deliberate production exceptions for Monaco and xterm surfaces;
  their dark rendering palettes may stay domain-specific, while their outer
  chrome follows Zarya tokens.
- Decide how the GOST Type A asset can be redistributed. The current font file
  is local-only and the design repository explicitly forbids committing it
  without a compatible license. Until this gate is resolved, implement the
  documented fallback stack without copying the private asset.
- Define the supported UI language for this slice. Keep English product copy
  initially unless localization is separately scoped, but enforce the Zarya
  voice: short, concrete, active and non-promotional.

Exit criteria:

- every current screen and shared primitive has a target pattern;
- font licensing/distribution has an explicit decision;
- exceptions are documented instead of encoded as stray styles.

### 1. Build the production token foundation

Primary files:

- `apps/web/src/styles.css`;
- `apps/web/src/shared/ui/`;
- `apps/web/index.html` and a future `apps/web/public/fonts/` only if the font
  gate allows it.

Deliverables:

- Define the six Zarya base color variables and documented derived color and
  line roles in `styles.css`.
- Define typography, leading and spacing variables exactly from Zarya 0.1;
  expose them to Tailwind v4 through semantic theme aliases.
- Set square geometry, stable control heights, tabular numerals, visible
  `:focus-visible`, intentional touch feedback and reduced-motion defaults.
- Add semantic primitives instead of page-local class strings: `Button`,
  `IconButton`, `Field`, `Textarea`, `StatusMark`, `StatusLabel`, `Surface`,
  `FigureCaption`, `SheetAnnotation`, `DisclosureControl`, `EmptyState` and
  `LoadingState`.
- Replace `Badge` tone colors with a state model aligned to `Normal`, `Idle`,
  `Running`, `Review`, `Warning`, `Risk`, `Blocked`, `Unknown`, `Disabled` and
  `Loading`. Risk may use red; notice may use violet; ordinary states remain
  monochrome.
- Add a static check that rejects new raw hex colors, radii and shadows outside
  the token/exception files.

Exit criteria:

- feature components consume semantic tokens, not palette literals;
- every interactive primitive has hover, active, focus-visible, disabled and
  pending states without layout shift;
- status is never communicated by color alone.

### 2. Recompose the app shell

Primary files:

- `apps/web/src/app/shell/AppShell.tsx`;
- `apps/web/src/features/inventory/InventoryTree.tsx`;
- `apps/web/src/workbench/inspector/InspectorPresentation.tsx`.

Deliverables:

- Convert shell chrome to one flat work surface with a 248px-class navigation
  rail, coordinate-like top row and optional inspector area.
- Remove tinted panel backgrounds, visible ordinary dividers, rounded nav
  fills and card-like inspector framing.
- Introduce the Zarya brand glyph treatment without turning the shell into a
  marketing header.
- Keep inventory hierarchy stable, but replace badge clusters with concise
  glyph + text state and reveal secondary runtime/resource details on demand.
- Add a skip link and stable landmarks. Preserve URL-backed inspector depth;
  render it as the production form of 2.5D disclosure with a visible layer
  title, `-` return control and 2-3 level depth limit.
- On narrow desktop collapse the inspector predictably; on mobile use a
  full-width navigation stack while preserving the same reference URL state.

Exit criteria:

- shell geography remains recognizable across desktop, narrow desktop and
  mobile;
- inventory and inspector are fully keyboard reachable;
- opening detail never loses the originating object or return path.

### 3. Rebuild Dashboard as a system sheet

Primary file: `apps/web/src/features/dashboard/DashboardRoute.tsx`.

Deliverables:

- Replace the four-card first row with one System Overview showing system
  state, affected scope, primary cause/risk and next action.
- Convert metrics to annotated metric cells with value, unit, period and
  comparison; use tabular numerals.
- Replace repeated status cards with a stable node -> workspace -> runtime ->
  session figure/pipeline map. Normal state stays quiet; deviations remain
  visible without reading every row.
- Consolidate recent activity and attention into a dense table/list with a
  left-side state glyph and nearby next action.
- Add `+`/`-` disclosure for causes, evidence, history and parameters while
  keeping each object in stable screen geography.
- Specify loading, empty, stale, partial-data and API-error variants with
  actionable copy.

Exit criteria:

- the first viewport answers what is happening, what is affected, why and what
  to do next;
- no ordinary content is represented as nested cards;
- critical deviation is identifiable without relying on color.

### 4. Rebuild Session / Agent Chat around work phases

Primary files:

- `apps/web/src/features/sessions/SessionRoute.tsx`;
- `apps/web/src/features/sessions/SessionTimeline.tsx`;
- `apps/web/src/features/sessions/ChatComposer.tsx`;
- `apps/web/src/workbench/blocks/TimelineBlockRenderer.tsx`;
- `apps/web/src/features/artifacts/EvidenceProjection.tsx`;
- `apps/web/src/features/agent-projection/AgentProjectionPanel.tsx`.

Deliverables:

- Make the session header a context line: workspace, runtime phase, state,
  reversibility and available lifecycle actions.
- Rework message anatomy to phase/glyph, concise conclusion, evidence/source,
  proposed action and risk/reversibility. Do not anthropomorphize the agent.
- Use the existing typed refs and inspector stack for inline source/evidence/
  cause disclosure. Replace chevron accordion semantics with the documented
  thin `+` entry and `-` return where the action changes semantic depth.
- Make approvals a decision surface: exact requested action, affected scope,
  source/evidence, risk, reversibility and explicit Approve/Deny labels.
- Keep errors red but include cause, affected scope and next safe step.
- Integrate Evidence Projection and Agent Projection into the same disclosure
  model instead of permanent nested side cards.
- Give the composer a persistent label and command context; add draft
  preservation/navigation warning, pending announcement and disabled reason.

Exit criteria:

- every agent action exposes phase and a path to evidence;
- destructive or privileged decisions show consequence and reversibility;
- long raw trace and payload data are available but not always visible;
- the composer works with keyboard alone and preserves unsent work.

### 5. Migrate operational screens and workspace chrome

Primary areas:

- Nodes, Projects, Placements and Runtime Settings;
- Workspace tree, file viewer/editor, command history, diff and terminal tabs;
- Auth and trusted-profile surfaces.

Deliverables:

- Move all screens onto shared page header, section, field, status and empty
  state patterns.
- Replace colored success/warning panels with form + text state; reserve red
  for risk and violet for non-dangerous notices.
- Keep file tree, Monaco, diff and xterm functionally unchanged while aligning
  headers, tabs, actions, selection, focus and evidence links.
- Turn command/check history into dense evidence rows with command, timestamp,
  result, affected files and source/cause links.
- Define responsive stacked-row labels for dense data instead of horizontal
  scrolling for the primary task.
- Ensure disabled lifecycle and workspace actions explain why they are
  unavailable.

Exit criteria:

- no legacy green SaaS palette, card radius or shadow remains outside approved
  editor/terminal exceptions;
- all current routes use the same component and state vocabulary;
- mobile can monitor state and perform review decisions without losing labels.

### 6. Accessibility, regression and quality gate

Deliverables:

- Add component tests for every state in the design-system vocabulary.
- Add Playwright screenshot baselines for Dashboard, Agent Chat and Workspace
  at desktop, narrow desktop and mobile viewports, including loading, empty,
  stale, warning/risk, blocked and disclosure-depth states.
- Add automated accessibility checks to Playwright and a manual keyboard pass
  for shell, inventory, disclosure, approvals, composer, file tree and
  inspector.
- Verify WCAG AA contrast for base, muted, risk and notice tokens; verify 200%
  zoom, long identifiers, long agent messages and reduced motion.
- Verify URL restoration for inspector/disclosure state and browser back/forward
  behavior.
- Extend `make l` for fast frontend token/type/test checks and keep `make c` as
  the final handoff gate.
- Update canonical English/Russian docs only for durable product or interaction
  decisions; keep this file tactical.

Exit criteria:

- `make c` passes;
- visual baselines pass in all required viewports;
- no critical automated accessibility finding remains;
- manual keyboard and mobile review checklists pass;
- the final implementation is visually traceable to Zarya 0.1 patterns.

## Recommended Work Slices

Implementation should land in independently reviewable slices:

1. token foundation and primitives;
2. shell, navigation and inspector disclosure;
3. Dashboard system sheet;
4. Session/Agent Chat and approvals;
5. operational routes and Workspace Inspector;
6. accessibility, visual regression, documentation and release gate.

Do not migrate page by page before primitives exist: that would duplicate
styles and create a long mixed-state period. After slice 1, each later slice
must remove legacy literals from every file it touches.

## Non-Goals

- no Core/Node API or protocol-v2 redesign;
- no new plugin architecture or dynamic block schema;
- no new charting library unless a validated product scenario requires it;
- no full localization system in this slice;
- no redesign of Monaco or xterm internals;
- no Figma library or extraction of a standalone npm UI package;
- no change to Zarya visual DNA without a separate design-system experiment.

## Completion Criteria

This plan is complete when:

1. all production routes use the Zarya token and component vocabulary;
2. Dashboard and Agent Chat match the design-system structure, not merely its
   colors;
3. source/evidence/cause navigation is expressed through consistent 2.5D
   disclosure;
4. state uses text + form/glyph, with color only as a supporting signal;
5. desktop and mobile primary tasks pass visual, keyboard and accessibility
   checks;
6. raw style literals are blocked by tooling except documented domain-specific
   editor/terminal palettes;
7. `make c` passes and release/version impact has been reviewed under
   [Versioning](../en/versioning.md).
