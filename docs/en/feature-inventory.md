# Cortex Feature Inventory

Status: `draft`

Purpose: capture the product features and directions already discussed in `README.md`, `docs/ru/cortex-notes.md`, and follow-up design discussion. This is not a roadmap or commitment list. The staged roadmap lives in [product-stages.md](product-stages.md).

## How to Read

- This is an inventory, not a delivery promise.
- Repeated ideas are deduplicated.
- Stage 1 intentionally focuses on persistent developer sessions, not task-based sandbox runs.
- Task-based runtime, hybrid workflows, and broader domains are preserved as later product directions.

## 1. Platform / Distributed Agent OS

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-001 | General WorkOS for agents | Cortex as a work operating system for agentic work, starting with software development and later expanding to analytics, research, finance, and other domains. |
| F-002 | Distributed Agent OS | The system manages agents, nodes, environments, artifacts, and workflows instead of being tied to one machine or one chat. |
| F-003 | Core / control plane | Central layer for agents, projects, nodes, tasks, artifacts, registries, permissions, and workflow state. |
| F-004 | Node Daemon | System daemon running on a node; registers with Core, launches agents, and exposes files, terminal, processes, logs, and state. |
| F-005 | Multi-node execution | Agents can run on local machine, server, cloud workspace, sandbox, or future managed node. |
| F-006 | Personal computer as node | A personal computer can be connected as a node and inspected through Cortex. |
| F-007 | Lightweight agent runtime | Runtime for launching agents under task/session-specific environments. |
| F-008 | Stable isolated environment | Agents can work in stable environments with sandbox, files, code, bash, and UI visibility. |
| F-009 | microVM support | Future support for running agent work in microVM-style isolation. |
| F-010 | Branch/workspace isolation | Agent work can be isolated by branch or workspace. |
| F-011 | Stateless agent + sandbox | Task-based mode can treat the agent as a stateless executor with sandbox/tool environment. |
| F-012 | Durable workflow state | Long-lived state belongs to workflow, not necessarily to a container or agent process. |
| F-013 | Event-driven agent work | Agent work can be modeled as an event-driven state machine. |
| F-014 | State store + event log | Persistent event/state layer for sessions, runs, traces, artifacts, and workflows. |
| F-015 | Deployment/compose generator | Future CLI/repo for starting Core/Node configurations. |
| F-016 | Cloud product with accounts/projects | Future commercial cloud mode with accounts, projects, and managed infrastructure. |
| F-017 | Cross-platform use | Work should move between desktop and mobile. |
| F-018 | Execution-mode neutral core | Persistent, task-based, and hybrid modes are execution modes of one system, not separate products. |
| F-019 | Hybrid managed session | A live session or orchestrator can spawn bounded task runs and merge results back into session/workflow state. |

## 2. Agents

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-020 | Codex as default provider adapter | Default AI-agent provider adapter for the first developer-focused product. |
| F-021 | Agent orchestrator | Agent that coordinates multiple agents or delegated work. |
| F-022 | Internal Cortex agent | First-class assistant inside Cortex UI for working with Cortex and helping the user. |
| F-023 | Internal agent orchestrates node agents | Internal Cortex agent can coordinate agents running on nodes. |
| F-024 | Multi-chat with agents | User can work with several agents at once. |
| F-025 | Pluggable agent providers | Unified interface over different agent providers. |
| F-026 | CLI connectors for agents | Support agent tools that expose CLI interfaces. |
| F-027 | Agent server | Own or external agent server capable of code-action style tasks. |
| F-028 | Specialized agents | Coding, support, retrieval, browser, finance, and other domain agents. |
| F-029 | Agent-authored commits | Agents can eventually create commits under controlled workflows. |
| F-030 | Self-creation as eval | Use building Cortex itself as a benchmark/evaluation scenario. |
| F-031 | UI context available to agents | Agents can understand what the user sees in the interface. |
| F-032 | Human/agent co-working model | Work hierarchy can include humans, operators, workers, and agents; initially simplified to human/agent. |
| F-033 | Agents in tool environments | Give agents tool environments, not only raw processes. |
| F-034 | Persistent agent session | Live agent process or interactive external agent with continued dialogue and state. |
| F-035 | Attach/detach to live agent | User can connect to and disconnect from a live agent without losing state. |
| F-036 | Task-based agent server mode | Agent called as bounded executor with tools, context package, sandbox, and result contract. |
| F-037 | Agent control-plane-like connection | Ability to connect to an agent as a manageable process/control surface, not only launch a task. |
| F-038 | Agent Provider Adapter | Minimal provider boundary for launching, resuming, streaming, interrupting, stopping, and normalizing provider-specific agent runtimes. |
| F-039 | Provider resume reference | Opaque provider session id or resume cursor that allows a runtime to be restored without making provider internals part of the Core contract. |

## 3. Tasks, Workflows, Harness

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-040 | Long agent tasks | Support assigning work that can run for hours. |
| F-041 | Harness for long work | Practices and mechanisms that make long agent work manageable. |
| F-042 | Semi-deterministic pipelines | Workflows like implement -> review -> fix. |
| F-043 | Agent self-review/checks | Agents can run self-review and tests inside controlled pipelines. |
| F-044 | Schedules / n8n-like automation | Future agentic schedules and automation flows. |
| F-045 | Guide library | Reusable guides agents can apply, e.g. Python project setup. |
| F-046 | Guidelines library | Code style, architecture, tests, review, and agent failure-mode guidelines. |
| F-047 | Built-in skills/tools/pipelines | Out-of-the-box primitives to avoid configuring every workflow from scratch. |
| F-048 | Task -> MR/PR flow | Agent work can eventually end in merge request / pull request. |
| F-049 | Git webhook wakes workflow | Workflow can sleep and resume when GitHub/GitLab/CI sends webhook. |
| F-050 | CI follow-up loop | Agent checks CI and updates external task after webhook. |
| F-051 | One-shot vs dialogue experiments | Compare single-shot, dialogue, and hybrid task framing. |
| F-052 | Hierarchical planning | Move from broad structure to details to reduce design/review overload. |
| F-053 | C4 + activity + UML state diagrams | Use these diagram forms for architecture and agent process design. |

## 4. Developer Workflow

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-060 | Project/file browser | Inspect full project and files, not only changed files. |
| F-061 | Terminal/output screen | See agent output, ideally as terminal sessions. |
| F-062 | Bash/tool call visibility | Inspect commands and tool calls performed by the agent. |
| F-063 | Diff viewer | First-class diff inspection. |
| F-064 | Git integration | Branches, changes, checks, PR/MR later. |
| F-065 | PR/MR comments import | Import review comments from git providers. |
| F-066 | Fix PR/MR comments with agents | Delegate review comment fixes to agents. |
| F-067 | Mobile review | Review work from phone. |
| F-068 | Test/check reports | Show tests/checks as run/session artifacts. |
| F-069 | API-level regression evals | Use API regression where UI evals are expensive. |
| F-070 | Project state view | See project/task/agent state, not only chat transcript. |
| F-071 | Project Workspace Inspector | Non-chat workbench surface for project tree, file viewing, lightweight editing, terminal sessions, command/output history, diffs, checks, and trace-linked evidence. |
| F-072 | File viewer/editor | Open workspace files and ranges safely, with lightweight text editing for direct human intervention. |
| F-073 | Workspace terminal/PTY sessions | Start, attach, detach, resize, and close project-scoped consoles through Node Daemon. |
| F-074 | Command/output history | Preserve terminal and agent command output as navigable evidence tied to session events and trace. |
| F-075 | Addressable workspace references | Refer to files, ranges, edits, terminal sessions, commands, output ranges, diff hunks, checks, and artifacts from chat, trace, and UI actions. |
| F-076 | Inspect-first intervention model | Start with safe observation and narrow interventions before full IDE replacement. |
| F-077 | Basic file editing | Edit text files inside a workspace with explicit save/apply, conflict detection, diff visibility, and trace/audit events. |
| F-078 | IDE sidecar escape hatch | Later optional action to open the same workspace in code-server, OpenVSCode Server, Theia, or another full browser IDE provider. |

## 5. UI, Visual Artifacts, Interaction

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-080 | Notion-like block UI | Blocks for different data/action types. |
| F-081 | Obsidian-like navigation | Links, docs, tree, and knowledge-base style navigation. |
| F-082 | Dynamic block | Agent can return a form, graph, or dashboard. |
| F-083 | Dynamic UI in chats | Chats can contain Grafana, tools, and interactive UI blocks. |
| F-084 | Visual plugins | Plugins render actions/results visually in UI. |
| F-085 | Forms instead of text | Use forms when structured input is better than text. |
| F-086 | Graphs/charts | Data visualizations and chart artifacts. |
| F-087 | Dashboards | Agents/plugins can produce dashboards. |
| F-088 | Embedded external views | Embed external systems and service views. |
| F-089 | UML visualization | View UML diagrams. |
| F-090 | UML editor | Edit UML diagrams later. |
| F-091 | Canvas/dynamic interfaces | Canvas and dynamic interfaces as an important direction. |
| F-092 | @mentions | Mention files, tools, agents, and entities via `@`. |
| F-093 | Dual UI | Human-readable visual representation plus machine-readable agent representation. |
| F-094 | Long press to internal agent chat | Gesture to open internal agent chat for UI element. |
| F-095 | UI available to agent | Agent understands current user-visible UI context. |
| F-096 | Visual stack integration | Integrations must show up visually, not only as "I did X" text. |

## 6. Integrations and Plugins

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-100 | Plugin system | Extensions for tools, visualizations, agents, integrations, workflows. |
| F-101 | Notion integration | Connect Notion through plugin/integration. |
| F-102 | GitLab integration | Connect GitLab. |
| F-103 | Linear integration | Linear as the initial task tracker. |
| F-104 | Grafana integration | Embedded views, dashboards, monitoring. |
| F-105 | Docker integration | Runtime/deployment integration. |
| F-106 | MLflow integration | Possible experiment/ML integration. |
| F-107 | MCP integration | MCP support with visual output, not only text. |
| F-108 | CLI access to connected tools | CLI access to system-level connected tools. |
| F-109 | External task trackers first | Do not build own task tracker initially. |
| F-110 | Task tracker provider abstraction | Later meta-interface over different task trackers. |
| F-111 | External memory first | Do not build own memory system initially. |
| F-112 | Memory provider abstraction | Later meta-interface over memory providers. |
| F-113 | Git provider integration | PR/MR, comments, branches, checks. |
| F-114 | Observability provider integration | LangSmith, Langfuse, OpenTelemetry, Phoenix, etc. |
| F-115 | Sandbox/devbox providers | Daytona, E2B, Sandcastle, and similar providers later. |
| F-116 | Core Tool Registry | Tool/capability registry in Core with metadata, schemas, permissions, routing, UI contracts, audit policy. |
| F-117 | Plugin Registry | Installed plugins, versions, config, exposed tools, visual blocks, artifact types, workflow templates, permissions, compatibility. |
| F-118 | Integration adapter model | Integrations can use MCP, native API, Node-local, external provider, or hybrid adapters. |
| F-119 | First-class integration UX | Integrations provide UI blocks, artifacts, workflow hooks, trace, and permissions, not hidden tool calls. |

## 7. Traceability, Monitoring, Metrics

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-120 | Process visibility | See what the agent did, which files it touched, which commands it ran, and what it checked. |
| F-121 | Review-friendly trace | Reduce review cost instead of merely collecting logs. |
| F-122 | Monitoring layer | Monitor agent work and system state. |
| F-123 | Explainability direction | Investigate explainable agent decisions as future system capability. |
| F-124 | Stats screen | Statistics for agents/tasks/system. |
| F-125 | LLM proxy | Possible proxy to track usage, costs, and calls. |
| F-126 | Hard metrics | Measured progress instead of intuition. |
| F-127 | Edits per iteration | Measure number of edits in one iteration. |
| F-128 | Iterations to merge | Measure iterations until merge/acceptance. |
| F-129 | Scalability/support metrics | Diff size, changed lines/modules per function, fix commits. |
| F-130 | Attention budgeting / token economics | Track retrieval cost, context entropy, cache stability, semantic locality. |
| F-131 | Workflow provenance / audit trail | Record action/event/check/decision provenance. |
| F-132 | Review debt visibility | Surface accumulated review/integration debt. |

## 8. Mobile and Collaboration

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-140 | Desktop/mobile continuity | Start on desktop and continue from phone. |
| F-141 | Mobile task/session monitoring | See tasks, sessions, agents, and results from phone. |
| F-142 | Mobile review | Review trace, diff, and decisions from phone. |
| F-143 | Multi-user control / co-working | Explore collaborative agent control like Figma/Zed-style co-working. |
| F-144 | Agent work surface for teams | Shared surface for runs, tasks, results, and statuses. |

## 9. Knowledge Base, Documentation, Research

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-160 | Knowledge base mode | Cortex can also act as knowledge-base layer. |
| F-161 | Git repo + Obsidian model | Knowledge base as git repo with docs, indexes, links, tree navigation. |
| F-162 | Docs as code | Docs evolve as needed, split when too large, and are not overproduced early. |
| F-163 | README as project/key feature source | Project description and key features should be easy to find from README. |
| F-164 | Architecture tree | Tree/schema with short descriptions for major modules. |
| F-165 | Zotero-inspired research features | Borrow useful research workflow ideas without becoming Zotero. |
| F-166 | Research/article workflows | Support research and writing beyond software development. |

## 10. Benchmarks and Evals

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-180 | Self-build benchmark | Evaluate Cortex by trying to build itself from scratch. |
| F-181 | Detailed spec input benchmark | Benchmark with detailed spec input. |
| F-182 | Business case coverage | Broad business-case coverage in eval/regression set. |
| F-183 | Autonomous progress metric | Measure how far system progresses without human intervention. |
| F-184 | API regression benchmark | Use API regression where UI testing is expensive. |
| F-185 | Agent mode benchmark | Compare single-shot, dialogue, hierarchical, and pipeline modes. |
| F-186 | Execution mode comparison | Compare persistent, task-based, and hybrid modes by review cost, autonomy, latency, trace quality, and user control. |

## 11. Domains and Expansion

| ID | Feature / Direction | Meaning |
| --- | --- | --- |
| F-200 | Software development first | First focus because projects, files, git, tests, diff, review, and PR/MR are clear. |
| F-201 | Analytics workflows | Future expansion to analytics. |
| F-202 | Research workflows | Future expansion to research, articles, and investigations. |
| F-203 | Finance workflows | Future expansion to finance. |
| F-204 | Personal tasks branch | Possible separate branch for personal tasks. |
| F-205 | Site/email generators | Lightweight agent runtime may support site/email generation. |
| F-206 | Knowledge workflows | Docs, indexes, research library, and handoff via repo/tree. |

## 12. Candidate Foundation Cut

Most coherent first layer:

- Core/control plane.
- Node Daemon.
- Project + workspace.
- Minimal Agent Provider Adapter boundary.
- Codex as default provider adapter.
- Persistent agent session.
- Chat plus non-chat views.
- Project Workspace Inspector.
- File tree and file viewer/editor.
- Basic text file editing.
- Workspace terminal/PTY sessions.
- Command/output history.
- Diff/check entry points.
- Basic session event log and trace.
- Addressable workspace references.
- Basic git/diff awareness.
- Mobile-readable session/review state.
- Minimal dynamic artifact/block API.
- Minimal plugin boundary.
- Minimal Tool Registry and Plugin Registry shape.

Task-based sandbox runs, durable workflows, and full MR/PR flow are intentionally outside the first foundation cut. The first product should prove the persistent Node-based developer workbench and leave architectural space for task-based mode later.

This version tests the main thesis: Cortex gives more control, visualization, and traceability than a regular agent chat.
