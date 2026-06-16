# Cortex Feature Inventory

Status: `draft`

Purpose: record feature ideas that were already proposed in `README.md` and
`docs/ru/cortex-notes.md`, without prioritizing them and without turning them
into a roadmap. This is a raw but grouped inventory for later selection into
vision, architecture, and backlog.

Sources:

- `README.md`
- `docs/ru/cortex-notes.md`

## How to Read

- This is not a list of commitments.
- Repeated ideas are merged into one item.
- `Source` shows where the idea came from or where it appears repeatedly.
- If an idea currently looks like a research direction, that is stated
  explicitly.

## 1. Platform / Distributed Agent OS

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-001 | General-purpose WorkOS for agents | Cortex as a work operating environment for agent work, first for development, then for analytics, research, finance, and other tasks. | `README.md:3`, `docs/ru/cortex-notes.md:24` |
| F-002 | Distributed Agent OS | The system is not limited to one machine or one chat; it manages agents, nodes, environments, and workflows. | `docs/ru/cortex-notes.md:24` |
| F-003 | Core / control plane | Central layer for managing agents, projects, nodes, tasks, artifacts, and workflows. | `README.md:28`, `docs/ru/cortex-notes.md:23`, `docs/ru/cortex-notes.md:54` |
| F-004 | Node Daemon | System daemon running on a node, registering it in Core, starting agents, and providing access to files and system state. | `README.md:28`, `docs/ru/cortex-notes.md:23`, User clarification, 2026-06-15 |
| F-005 | Multi-node execution | Ability to run agents on different nodes: local computer, server, cloud workspace, sandbox. | `README.md:17`, `docs/ru/cortex-notes.md:23`, `docs/ru/cortex-notes.md:54` |
| F-006 | Connect a personal computer as a node | A user can connect a personal computer and see what agents are doing there. | `docs/ru/cortex-notes.md:23` |
| F-007 | Lightweight runtime for agents | Environment where agents can be started quickly for concrete tasks. | `docs/ru/cortex-notes.md:28` |
| F-008 | Stable isolated environment | The agent works in a stable environment with sandbox, files, code, bash, and UI visibility. | `docs/ru/cortex-notes.md:54` |
| F-009 | microVM for agents | All agents can run in a microVM. | `docs/ru/cortex-notes.md:15` |
| F-010 | Separate git branch per agent/run | Agent work is isolated in a separate branch. | `docs/ru/cortex-notes.md:15` |
| F-011 | Stateless agent + sandbox | The agent does not have to be a long-lived process; work can happen through a sandbox/workspace. | `docs/ru/cortex-notes.md:112-126` |
| F-012 | Durable workflow state | Workflow state is long-lived, not a container or a specific agent session. | `docs/ru/cortex-notes.md:121-126` |
| F-013 | Event-driven state machine for agent work | Agent work is modeled as an event-driven state machine. | `docs/ru/cortex-notes.md:109-110` |
| F-014 | State store + event log | Store state and event history through Postgres/Redis/S3/Vector DB or a similar layer. | `docs/ru/cortex-notes.md:102-103` |
| F-015 | Deployment repo / compose generator | Repository or CLI for starting infrastructure: choose core/node configuration and generate compose/deploy config. | `docs/ru/cortex-notes.md:73` |
| F-016 | Cloud product with accounts/projects | Commercial cloud variant with accounts and projects. | `docs/ru/cortex-notes.md:55` |
| F-017 | Multiplatform support | The product should work across desktop and mobile. | `README.md:7`, `README.md:16`, `docs/ru/cortex-notes.md:9`, `docs/ru/cortex-notes.md:69` |
| F-018 | Execution-mode neutral core | Core should not be tied only to task-based cloud-agent flow; persistent, task-based, and hybrid modes should be different execution modes of one system. | User clarification, 2026-06-15 |
| F-019 | Hybrid managed session | A live session or orchestration agent can spawn bounded task runs for subtasks and return the result into shared workflow/session state. | User clarification, 2026-06-15 |

## 2. Agents

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-020 | Codex as default agent | Initial default agent. | `README.md:47` |
| F-021 | Agent orchestrator | Orchestrator that manages multiple agents and distributes tasks. | `docs/ru/cortex-notes.md:4`, `docs/ru/cortex-notes.md:14` |
| F-022 | Internal Cortex agent | Agent inside the UI as a first-class citizen for working on Cortex and helping the user. | `README.md:29` |
| F-023 | Internal agent orchestrates node agents | The internal agent can orchestrate agents running on nodes. | `README.md:29` |
| F-024 | Multi-chat with agents | Ability to communicate with several agents at once. | `docs/ru/cortex-notes.md:13` |
| F-025 | Pluggable agent providers | Unified interface over different agent providers, like a meta-tool for agents. | `docs/ru/cortex-notes.md:76` |
| F-026 | Agent CLI connectors | Support CLI connectors for different agent tools. | `docs/ru/cortex-notes.md:32` |
| F-027 | Agent server | Own or ready-made agent server capable of performing code-action level tasks. | `docs/ru/cortex-notes.md:32` |
| F-028 | Specialized agents | Different agent classes: coding, support, retrieval, browser, finance. | `docs/ru/cortex-notes.md:95-100` |
| F-029 | Agents as commit executors | Use agents for commits as much as possible. | `docs/ru/cortex-notes.md:27` |
| F-030 | Self-creation as eval | Self-creation of the first Cortex version as an evaluation scenario. | `docs/ru/cortex-notes.md:8` |
| F-031 | Agent with available UI context | The interface is designed so the agent understands what the user sees. | `docs/ru/cortex-notes.md:83` |
| F-032 | Human/agent co-working model | Possible hierarchy: people, managers, workers, agents; or a simplified human/agent model. | `docs/ru/cortex-notes.md:10` |
| F-033 | Agents in tool environments | Direction: give agents environments as tools, not only as a process/machine. | `docs/ru/cortex-notes.md:29` |
| F-034 | Persistent agent session | The agent starts as a live process or connects as an external interactive agent; the user can continue dialogue, inspect state, and control the process. | User clarification, 2026-06-15 |
| F-035 | Attach/detach to live agent | Ability to attach to an already live agent process and detach without losing state. | User clarification, 2026-06-15 |
| F-036 | Task-based agent server mode | The agent is invoked as a server/task executor: it receives tools, context package, and sandbox, then returns a bounded result. | User clarification, 2026-06-15 |
| F-037 | Agent CP-like connection | Ability to connect to an agent as a managed process/control plane, not only start a separate task. | User clarification, 2026-06-15 |

## 3. Tasks, Workflows, Harness

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-040 | Long agent tasks | Ability to give an agent tasks for hours and return to the result. | `README.md:8`, `README.md:20`, `README.md:39` |
| F-041 | Harness for long tasks | Layer of practices and mechanisms that helps agents perform long tasks in a controlled way. | `README.md:39` |
| F-042 | Semi-deterministic pipelines | Workflow such as implementation -> review -> fix, where some stages can be deterministic. | `README.md:42` |
| F-043 | Agent self-review / tests inside pipeline | During implementation, the agent can self-review and run tests, while a separate review block remains explicit. | `README.md:42` |
| F-044 | Schedules / n8n-like automation | Pipelines and schedules in the style of n8n, but for agent work. | `docs/ru/cortex-notes.md:11` |
| F-045 | Guides library | Guides the agent can apply, for example setup Python project. | `README.md:40` |
| F-046 | Guidelines library | Rules for code style, architecture, tests, review, and common agent mistakes. | `README.md:41` |
| F-047 | Skills, tools, pipelines out of the box | Basic ready-made sets so each workflow does not need to be configured from scratch. | `README.md:43` |
| F-048 | Task -> MR/PR flow | Scenario where agent work ends with a merge request / pull request. | `README.md:9`, `docs/ru/cortex-notes.md:20`, `docs/ru/cortex-notes.md:120` |
| F-049 | Git webhook wakes workflow | Workflow can sleep and wake from a GitHub webhook, for example after CI. | `docs/ru/cortex-notes.md:121-124` |
| F-050 | CI follow-up loop | The agent checks CI and updates the external task after a webhook. | `docs/ru/cortex-notes.md:121-124` |
| F-051 | One-shot vs dialogue mode experiment | Research when "1 task = 1 request" is better and when dialogue is better. | `docs/ru/cortex-notes.md:57` |
| F-052 | Hierarchical planning approach | Move from scale to detail to reduce cognitive load during design. | `docs/ru/cortex-notes.md:78-81` |
| F-053 | C4 + Activity Diagram + UML State Machine | Use C4, activity diagrams, and UML state machines to design Cortex and agent processes. | `docs/ru/cortex-notes.md:81` |

## 4. Developer Workflow

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-060 | Project/file browser | Ability to view the whole project and files, not only changed files. | `README.md:19`, `docs/ru/cortex-notes.md:31`, `docs/ru/cortex-notes.md:59` |
| F-061 | Terminal view / agent output screen | Screen with agent output, preferably as terminals. | `docs/ru/cortex-notes.md:12`, `docs/ru/cortex-notes.md:54` |
| F-062 | Bash/tool call visibility | See which commands and tool calls ran in the environment. | `docs/ru/cortex-notes.md:54` |
| F-063 | Diff viewer | Convenient work with diff as a required early layer. | `README.md:19`, `docs/ru/cortex-notes.md:22` |
| F-064 | Git integration | Integration with git providers and working branches. | `README.md:36`, `docs/ru/cortex-notes.md:20`, `docs/ru/cortex-notes.md:120` |
| F-065 | PR/MR comments import | Load comments from PR/MR into Cortex. | `docs/ru/cortex-notes.md:20` |
| F-066 | Fix PR comments with agents | Ability to send PR/MR comments to an agent for fixing. | `docs/ru/cortex-notes.md:20` |
| F-067 | Mobile review | Convenient review from a phone. | `README.md:10`, `docs/ru/cortex-notes.md:69` |
| F-068 | Test/check reports | The agent runs tests through bash tool, and the system shows the result as part of the run. | `docs/ru/cortex-notes.md:119` |
| F-069 | API-level regression for UI/system evals | UI is hard to benchmark, but API can be covered with a large regression set. | `docs/ru/cortex-notes.md:41` |
| F-070 | Project state view | See project/task/agent state, not only chat transcript. | `README.md:19`, `docs/ru/cortex-notes.md:31`, `docs/ru/cortex-notes.md:59` |

## 5. UI, Visual Artifacts, Interaction

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-080 | Notion-like block UI | Interface with blocks where different data and actions can be embedded. | `README.md:30`, `docs/ru/cortex-notes.md:34` |
| F-081 | Obsidian-like knowledge/navigation model | Inspiration from Obsidian: connectivity, tree, docs, links, knowledge base. | `docs/ru/cortex-notes.md:34`, `docs/ru/cortex-notes.md:72` |
| F-082 | Dynamic block | The agent can output a form, graph, or full dashboard. | `README.md:31` |
| F-083 | Dynamic UI in chats | In chats, the agent can show Grafana, other tools, and interactive UI blocks. | `docs/ru/cortex-notes.md:21`, `docs/ru/cortex-notes.md:54` |
| F-084 | Visual plugins | Plugins that display actions and results in the interface. | `README.md:38` |
| F-085 | Forms instead of text | Sometimes the agent should show a form rather than ask/answer with text. | `README.md:18`, `README.md:31` |
| F-086 | Graphs / charts | Data visualizations, graphs, and other chart artifacts. | `README.md:18`, `README.md:31` |
| F-087 | Dashboards | An agent or plugin can create full dashboards. | `README.md:31`, `docs/ru/cortex-notes.md:54` |
| F-088 | Embedded external views | Embedded views and links to tools such as Grafana, services, and external systems. | `docs/ru/cortex-notes.md:31`, `docs/ru/cortex-notes.md:54` |
| F-089 | UML visualization | Minimum UML viewing. | `README.md:11` |
| F-090 | UML editor | Extension from UML visualization to editor. | `README.md:11` |
| F-091 | Canvas / dynamic interfaces | Canvas and dynamic interfaces as an important direction. | `docs/ru/cortex-notes.md:33` |
| F-092 | @mentions | Ability to mention a file, tool, agent, and other entities through `@`. | `README.md:35` |
| F-093 | Dual UI | Each element has a visual representation for humans and a machine-readable representation for agents. | `README.md:32-34` |
| F-094 | Long press to internal agent chat | Long press opens chat with the internal agent for flexible interaction with UI. | `README.md:34`, `docs/ru/cortex-notes.md:83` |
| F-095 | UI available to agent | The agent understands what the user sees and can act with interface context. | `docs/ru/cortex-notes.md:83` |
| F-096 | Visual stack integration | Integrations should appear visually, not only as text such as "I did X" or a link. | `README.md:12`, `docs/ru/cortex-notes.md:31` |

## 6. Integrations and Plugins

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-100 | Plugin system | Extension system for tools, visualizations, agents, integrations, and workflows. | `README.md:21`, `README.md:36-38`, `docs/ru/cortex-notes.md:63` |
| F-101 | Notion integration | Connect Notion through a plugin/integration. | `README.md:36` |
| F-102 | GitLab integration | Connect GitLab. | `README.md:36` |
| F-103 | Linear integration | Linear as the first main task tracker. | `README.md:36`, `README.md:50` |
| F-104 | Grafana integration | Embedded views, dashboards, and monitoring through Grafana. | `README.md:36`, `docs/ru/cortex-notes.md:21`, `docs/ru/cortex-notes.md:30-31` |
| F-105 | Docker integration | Integration with Docker/deployment/runtime layer. | `README.md:36`, `docs/ru/cortex-notes.md:73` |
| F-106 | MLflow integration | Connect MLflow as a possible plugin. | `README.md:36` |
| F-107 | MCP integration | Connect MCP, but output results into visual UI, not only into text. | `README.md:12`, `docs/ru/cortex-notes.md:100` |
| F-108 | CLI access to connected tools | CLI with access to tools connected at the system level. | `README.md:37` |
| F-109 | External task trackers instead of own tracker first | Do not build an own task tracker at the start; use existing ones. | `README.md:48`, `README.md:50` |
| F-110 | Task tracker provider abstraction | Later, build a meta-tool over different task trackers. | `docs/ru/cortex-notes.md:76` |
| F-111 | External memory instead of own memory first | Do not build an own memory system at the start. | `README.md:48` |
| F-112 | Memory provider abstraction | Meta-tool over different memory providers. | `docs/ru/cortex-notes.md:5`, `docs/ru/cortex-notes.md:19`, `docs/ru/cortex-notes.md:76` |
| F-113 | Git provider integration | Git integration for PR/MR, comments, branches, checks. | `docs/ru/cortex-notes.md:20`, `docs/ru/cortex-notes.md:120-124` |
| F-114 | Observability provider integration | LangSmith, Langfuse, OpenTelemetry, Phoenix, or similar tools. | `docs/ru/cortex-notes.md:105-106` |
| F-115 | Sandbox/devbox providers | Possible external sandbox providers from useful links: Daytona, E2B, Sandcastle, and equivalents. | `docs/ru/cortex-notes.md:132`, `docs/ru/cortex-notes.md:138-139` |
| F-116 | Core Tool Registry | Registry of tools/capabilities in Core: metadata, schemas, permissions, routing, UI contracts, and audit policy. Tool execution can happen on Node, in an external provider, or later in Core for lightweight tools. | User clarification, 2026-06-15 |
| F-117 | Plugin Registry | Registry of installed plugins in Core: versions, configuration, exposed tools, visual blocks, artifact types, workflow templates, permissions, and compatibility. | User clarification, 2026-06-15 |
| F-118 | Integration adapter model | Integrations connect through MCP, native API, Node-local adapters, external provider adapters, or hybrid adapters. MCP matters, but is not the only option. | User clarification, 2026-06-15 |
| F-119 | First-class integration UX | Integrations should provide UI blocks, artifacts, workflow hooks, trace, and permissions, not only a hidden tool call inside the agent's text answer. | User clarification, 2026-06-15 |

## 7. Traceability, Monitoring, Metrics

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-120 | Traceability / process visibility | See what the agent did, which files it looked at, which changes it made, which commands it ran, and what it checked. | `README.md:19`, `docs/ru/cortex-notes.md:31`, `docs/ru/cortex-notes.md:54` |
| F-121 | Review-friendly trace | The system should reduce review cost, not merely collect logs. | `README.md:19`, `docs/ru/cortex-notes.md:31`, `docs/ru/cortex-notes.md:61` |
| F-122 | Monitoring layer | Monitoring of agent work and system state. | `docs/ru/cortex-notes.md:30`, `docs/ru/cortex-notes.md:61`, `docs/ru/cortex-notes.md:105-106` |
| F-123 | Explainable AI direction | Research explainability/interpretability of agent decisions as part of the future system. | `docs/ru/cortex-notes.md:61` |
| F-124 | Stats screen | Statistics screen for agents/tasks/system. | `docs/ru/cortex-notes.md:16` |
| F-125 | LLM proxy | Possible proxy for LLM to collect statistics, control costs, and observe calls. | `docs/ru/cortex-notes.md:16` |
| F-126 | Hard metrics | Hard metrics instead of a feeling of progress. | `docs/ru/cortex-notes.md:17`, `docs/ru/cortex-notes.md:67` |
| F-127 | Edits per iteration | Measure the number of edits in one iteration. | `docs/ru/cortex-notes.md:25` |
| F-128 | Iterations to merge | Measure the number of iterations until merge. | `docs/ru/cortex-notes.md:25` |
| F-129 | Scalability/support metrics | Diff sizes, number of changed lines per function, number of changed modules per feature, number of fix commits. | `docs/ru/cortex-notes.md:26` |
| F-130 | Attention budgeting / token economics | Metrics and optimization of attention/token costs: retrieval cost, context entropy, cache stability, semantic locality. | `docs/ru/cortex-notes.md:65` |
| F-131 | Workflow provenance / audit trail | Preserve provenance of actions, events, checks, and decisions. | `docs/ru/cortex-notes.md:102-106`, `docs/ru/cortex-notes.md:126` |
| F-132 | Review debt visibility | Visibility into accumulated review/integration debt so agent speed does not turn into hidden burden. | `README.md:20`, `docs/ru/cortex-notes.md:25-26`, `docs/ru/cortex-notes.md:67` |

## 8. Mobile and Collaboration

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-140 | Desktop/mobile continuity | A user can start work on a computer and continue from a phone. | `README.md:7`, `README.md:16`, `docs/ru/cortex-notes.md:9`, `docs/ru/cortex-notes.md:69` |
| F-141 | Mobile task monitoring | From a phone, the user can see task state, agents, and their results. | `README.md:7`, `docs/ru/cortex-notes.md:69` |
| F-142 | Mobile review | From a phone, the user can review changes, read trace, inspect diff, and make decisions. | `README.md:10` |
| F-143 | Multi-user control / co-working | Research collaborative scenarios for agent control by analogy with Figma/Zed. | `docs/ru/cortex-notes.md:10` |
| F-144 | Agent work surface for teams | Shared work surface where agent runs, tasks, results, and statuses are visible. | `docs/ru/cortex-notes.md:10`, `docs/ru/cortex-notes.md:54` |

## 9. Knowledge Base, Documentation, Research

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-160 | Knowledge base mode | Cortex can be not only a runtime, but also a layer for a knowledge base. | `docs/ru/cortex-notes.md:72` |
| F-161 | Git repo + Obsidian model | Knowledge base as a git repo with docs, indexes, links, and tree navigation. | `docs/ru/cortex-notes.md:72` |
| F-162 | Docs as code | Documents evolve like code: created when needed, split as they grow, not spawned in advance. | `docs/ru/cortex-notes.md:44-51` |
| F-163 | README as project/key feature source | Project description and key features should live in the main README. | `docs/ru/cortex-notes.md:45-46` |
| F-164 | Architecture tree | Extended diagram/tree with a short description of each large module. | `docs/ru/cortex-notes.md:51` |
| F-165 | Zotero-inspired research features | Take useful ideas from Zotero for research/document workflows without turning Cortex into Zotero. | `docs/ru/cortex-notes.md:74` |
| F-166 | Research/article workflows | Do not limit Cortex to development: articles and research should be supported too. | `docs/ru/cortex-notes.md:24` |

## 10. Benchmarks and Evals

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-180 | Self-build benchmark | Evaluate Cortex through an attempt to create the system again from scratch. | `docs/ru/cortex-notes.md:8`, `docs/ru/cortex-notes.md:36-42` |
| F-181 | Detailed spec input benchmark | Benchmark with a detailed specification as input. | `docs/ru/cortex-notes.md:38` |
| F-182 | Business case coverage | Broad business-case coverage in the eval/regression set. | `docs/ru/cortex-notes.md:39` |
| F-183 | Autonomous progress metric | Measure how far the system can progress without human intervention. | `docs/ru/cortex-notes.md:40` |
| F-184 | API regression benchmark | Build a large regression at API level because UI is harder to test. | `docs/ru/cortex-notes.md:41` |
| F-185 | Agent mode benchmark | Compare single-shot, dialogue, hierarchical, and pipeline modes of agent work. | `docs/ru/cortex-notes.md:57`, `docs/ru/cortex-notes.md:78-81` |
| F-186 | Execution mode comparison | Compare persistent session, task-based sandbox run, and hybrid managed session by review cost, autonomy, latency, trace quality, and user control. | User clarification, 2026-06-15 |

## 11. Domains and Expansion

| ID | Feature / Direction | Meaning | Source |
| --- | --- | --- | --- |
| F-200 | Software development first | First focus is development, because it has project, files, git, tests, diff, review, MR/PR. | `README.md:3`, `docs/ru/cortex-notes.md:22`, `docs/ru/cortex-notes.md:59` |
| F-201 | Analytics workflows | Later expansion into analytics. | `README.md:3` |
| F-202 | Research workflows | Later expansion into research, articles, and studies. | `README.md:3`, `docs/ru/cortex-notes.md:24` |
| F-203 | Finance workflows | Later expansion into finance. | `README.md:3`, `docs/ru/cortex-notes.md:98` |
| F-204 | Personal tasks branch | Possible separate branch for personal tasks. | `README.md:3` |
| F-205 | Site/email generators | A lightweight agent runtime can simplify generation of sites and emails. | `docs/ru/cortex-notes.md:28` |
| F-206 | Knowledge workflows | Knowledge-base processes, docs, indexes, research library, and handoff through repo/tree. | `docs/ru/cortex-notes.md:72`, `docs/ru/cortex-notes.md:74` |

## 12. Candidate Foundation Cut

If this inventory is reduced to the first practical layer, the most coherent
minimal version looks like:

- Core/control plane.
- Node Daemon.
- Project + workspace.
- Codex as default agent.
- Persistent agent session.
- Chat plus non-chat views.
- File browser.
- Terminal/output view.
- Diff view.
- Basic session event log and trace.
- Basic git/diff awareness.
- Mobile-readable run/review state.
- Minimal dynamic artifact/block API.
- Minimal plugin boundary.
- Minimal Tool Registry and Plugin Registry shape.

Task-based sandbox runs, durable workflows, and full MR/PR flow are
intentionally outside the first foundation cut. The first product should prove
the persistent Node-based developer workbench and leave architectural space for
task-based mode later.

This version validates the main thesis: Cortex gives more control,
visualization, and traceability than a regular agent chat.
