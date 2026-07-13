const main = document.querySelector("#main-content");
const appShell = document.querySelector(".app-shell");
const shellGrid = document.querySelector("#shell-grid");
const inspector = document.querySelector("#inspector-content");
const toast = document.querySelector("#toast");
const topbarLocation = document.querySelector("#topbar-location");
const sidebarToggle = document.querySelector("#toggle-sidebar");

const workspaceRoutes = new Set(["agent", "workbench", "jobs"]);

function workspaceHeader(active) {
  const tabs = [
    ["agent", "Agent", "Agent sessions and runtime activity"],
    ["workbench", "Workbench", "Files, editor, diff and terminal"],
    ["jobs", "Jobs", "Scheduled and manual background work"],
  ];

  return `
    <header class="workspace-header">
      <div class="workspace-heading-row">
        <div>
          <div class="breadcrumb">
            <a href="#node" data-route="node">Local Node</a>
            <span aria-hidden="true">/</span>
            <span>Uprava</span>
          </div>
          <h1>Uprava</h1>
          <div class="workspace-path">/workspace/uprava</div>
        </div>
        <div class="status-groups" aria-label="Workspace status">
          <span class="status presence"><span>Presence</span> stale</span>
          <span class="status workspace"><span>Workspace</span> dirty</span>
          <span class="status lifecycle"><span>Runtime</span> active</span>
        </div>
      </div>
      <nav class="workspace-tabs" aria-label="Workspace surfaces">
        ${tabs
          .map(
            ([route, label, description]) => `
              <a
                href="#${route}"
                data-route="${route}"
                class="${route === active ? "active" : ""}"
                ${route === active ? 'aria-current="page"' : ""}
              >
                <strong>${label}</strong>
                <span>${description}</span>
              </a>`,
          )
          .join("")}
      </nav>
    </header>`;
}

const dashboard = `
  <section class="page dashboard-page">
    <header class="page-header">
      <div>
        <div class="eyebrow">System Overview</div>
        <h1>Dashboard</h1>
        <p>Current control plane workload and recent operational activity.</p>
      </div>
      <div class="updated-at">Updated just now</div>
    </header>

    <section class="dashboard-metrics" aria-label="System metrics">
      <article class="dashboard-metric">
        <div class="metric-head"><span>Core API</span><i class="status-dot online"></i></div>
        <strong>Operational</strong>
        <p>controlled_dev profile</p>
      </article>
      <article class="dashboard-metric">
        <div class="metric-head"><span>Reachable Nodes</span><i class="status-dot warning"></i></div>
        <strong>1 / 2</strong>
        <p>Local Node is stale</p>
      </article>
      <article class="dashboard-metric">
        <div class="metric-head"><span>Active Runtimes</span><i class="status-dot online"></i></div>
        <strong>2</strong>
        <p>Across 2 workspaces</p>
      </article>
      <article class="dashboard-metric">
        <div class="metric-head"><span>Running Jobs</span><i class="status-dot online"></i></div>
        <strong>1</strong>
        <p>3 scheduled definitions</p>
      </article>
    </section>

    <section class="activity-section">
      <header class="section-header">
        <div>
          <div class="eyebrow">Event Projection</div>
          <h2>Recent Activity</h2>
        </div>
        <button class="text-button" type="button" data-action="prototype-only">Open Event Log →</button>
      </header>
      <div class="activity-list">
        <button class="activity-row" type="button" data-route="agent">
          <time>12:41</time>
          <span class="activity-object">Fix issue</span>
          <span class="activity-message">Session blocked</span>
          <span class="status attention">Approval required</span>
          <span aria-hidden="true">→</span>
        </button>
        <button class="activity-row" type="button" data-inspect="check">
          <time>12:38</time>
          <span class="activity-object">Uprava</span>
          <span class="activity-message">Check completed · make c</span>
          <span class="status lifecycle">exit 0</span>
          <span aria-hidden="true">→</span>
        </button>
        <button class="activity-row" type="button" data-inspect="node">
          <time>12:36</time>
          <span class="activity-object">Local Node</span>
          <span class="activity-message">Heartbeat became stale</span>
          <span class="status presence">stale</span>
          <span aria-hidden="true">→</span>
        </button>
        <button class="activity-row" type="button" data-route="jobs">
          <time>12:31</time>
          <span class="activity-object">Dependency audit</span>
          <span class="activity-message">Job run started</span>
          <span class="status lifecycle">running</span>
          <span aria-hidden="true">→</span>
        </button>
        <button class="activity-row" type="button" data-inspect="diff">
          <time>12:24</time>
          <span class="activity-object">Uprava</span>
          <span class="activity-message">Workspace changed · 3 files</span>
          <span class="status workspace">dirty</span>
          <span aria-hidden="true">→</span>
        </button>
      </div>
    </section>
  </section>`;

const node = `
  <section class="page node-page">
    <header class="page-header node-title">
      <div>
        <div class="eyebrow">Node Overview</div>
        <h1>Local Node</h1>
        <p>Runtime environment, workspace roots and current workload.</p>
      </div>
      <div class="header-actions">
        <span class="status presence"><span>Presence</span> stale</span>
        <button class="button" type="button" data-action="reconnect">Check Connection</button>
        <button class="icon-button bordered" type="button" data-route="settings" aria-label="Node settings">⚙</button>
      </div>
    </header>

    <section class="node-summary-grid">
      <article><span>Last heartbeat</span><strong>24 min ago</strong><small>Expected every 10 sec</small></article>
      <article><span>Workspaces</span><strong>2</strong><small>2 validated roots</small></article>
      <article><span>Active runtimes</span><strong>1</strong><small>1 session blocked</small></article>
      <article><span>Running jobs</span><strong>1</strong><small>2 scheduled definitions</small></article>
    </section>

    <div class="node-content-grid">
      <section>
        <header class="section-header compact">
          <div><div class="eyebrow">Workspace Roots</div><h2>Workspaces</h2></div>
          <button class="button" type="button" data-action="add-workspace">＋ Add Workspace</button>
        </header>
        <div class="workspace-cards">
          <a class="workspace-card" href="#agent" data-route="agent">
            <div>
              <strong>Uprava</strong>
              <span>/workspace/uprava</span>
            </div>
            <div class="workspace-card-state">
              <span class="status workspace">dirty</span>
              <span class="status attention">1 blocked</span>
              <span aria-hidden="true">→</span>
            </div>
          </a>
          <button class="workspace-card muted-card" type="button" data-action="prototype-only">
            <div><strong>Research Lab</strong><span>/workspace/research</span></div>
            <div class="workspace-card-state"><span class="status workspace">clean</span><span aria-hidden="true">→</span></div>
          </button>
        </div>
      </section>

      <aside class="node-details">
        <section>
          <div class="eyebrow">Capabilities</div>
          <div class="capability-list">
            <span>codex</span><span>workspace.fs</span><span>workspace.pty</span><span>workspace.diff</span>
          </div>
        </section>
        <section>
          <div class="eyebrow">Connection</div>
          <dl class="detail-list">
            <div><dt>Node ID</dt><dd>node_local_01</dd></div>
            <div><dt>Version</dt><dd>0.2.5</dd></div>
            <div><dt>Platform</dt><dd>darwin / arm64</dd></div>
          </dl>
        </section>
      </aside>
    </div>
  </section>`;

const zaryaNode = `
  <section class="page node-page">
    <header class="page-header node-title">
      <div><div class="eyebrow">Node Overview</div><h1>Zarya Server</h1><p>Remote runtime environment and available workspace roots.</p></div>
      <div class="header-actions"><span class="status presence online"><span>Presence</span> online</span><button class="icon-button bordered" type="button" data-route="settings" aria-label="Node settings">⚙</button></div>
    </header>
    <section class="node-summary-grid">
      <article><span>Last heartbeat</span><strong>4 sec ago</strong><small>Connection healthy</small></article>
      <article><span>Workspaces</span><strong>1</strong><small>1 validated root</small></article>
      <article><span>Active runtimes</span><strong>1</strong><small>Agent is running</small></article>
      <article><span>Running jobs</span><strong>0</strong><small>No current runs</small></article>
    </section>
    <section class="empty-surface"><div class="eyebrow">Workspace Roots</div><h2>Workspace navigation is collapsed</h2><p>This secondary node is included to demonstrate the node-first information architecture.</p><button class="button" type="button" data-action="prototype-only">Expand in Sidebar</button></section>
  </section>`;

const agent = `
  <section class="workspace-page agent-page">
    ${workspaceHeader("agent")}
    <div class="agent-layout">
      <aside class="session-list" aria-label="Workspace sessions">
        <header>
          <div><div class="eyebrow">Sessions</div><strong>Agent Work</strong></div>
          <button class="icon-button bordered" type="button" data-action="start-session" aria-label="Start Agent Session">＋</button>
        </header>
        <button class="session-item active" type="button">
          <span class="session-item-title">Fix issue</span>
          <span class="session-item-meta">Codex · 12:41</span>
          <span class="status attention">blocked</span>
        </button>
        <button class="session-item" type="button" data-action="prototype-only">
          <span class="session-item-title">Review architecture</span>
          <span class="session-item-meta">Codex · yesterday</span>
          <span class="status lifecycle">completed</span>
        </button>
        <button class="start-session-card" type="button" data-action="start-session">
          <span aria-hidden="true">＋</span>
          <span><strong>Start Session</strong><small>Provider: Codex</small></span>
        </button>
      </aside>

      <section class="session-surface">
        <header class="session-header">
          <div>
            <div class="eyebrow">Agent Session · Codex</div>
            <h2>Fix issue</h2>
          </div>
          <div class="header-actions">
            <span class="status lifecycle">active</span>
            <span class="status attention">blocked</span>
            <button class="icon-button bordered" type="button" data-inspect="session" aria-label="Inspect session">⊞</button>
          </div>
        </header>

        <section class="blocker-card" id="current-blocker">
          <div class="blocker-symbol" aria-hidden="true">!</div>
          <div class="blocker-copy">
            <div class="eyebrow">Current Blocker</div>
            <h3>Approval required to continue</h3>
            <p><code>make c</code> needs permission to run in the current workspace.</p>
            <button class="text-button" type="button" data-inspect="approval">Review scope and evidence →</button>
          </div>
          <div class="blocker-actions">
            <button class="button" type="button" data-action="deny">Deny</button>
            <button class="button primary" type="button" data-action="approve">Approve & Continue</button>
          </div>
        </section>

        <div class="timeline" aria-label="Session timeline">
          <article class="turn operator-turn">
            <header><span>Operator</span><time>12:34</time></header>
            <p>Please inspect the failing checks and fix the issue.</p>
          </article>
          <article class="turn agent-turn">
            <header><span>Agent</span><time>12:36</time></header>
            <p>I found an inconsistent route state and updated the workspace navigation.</p>
            <div class="turn-evidence">
              <button type="button" data-inspect="diff"><span aria-hidden="true">⑂</span> 3 files changed</button>
              <button type="button" data-inspect="check"><span aria-hidden="true">✓</span> make l passed</button>
            </div>
          </article>
          <article class="turn system-turn">
            <header><span>Runtime</span><time>12:41</time></header>
            <p>Agent requested approval before running <code>make c</code>.</p>
            <button class="text-button" type="button" data-inspect="approval">Open approval event →</button>
          </article>
        </div>

        <section class="composer">
          <div class="composer-meta"><strong>Next Agent Turn</strong><span id="composer-state">Ready</span></div>
          <textarea id="composer" placeholder="Send instructions to the agent…" aria-label="Next Agent Turn"></textarea>
          <div class="composer-actions">
            <span>Draft is preserved until accepted.</span>
            <button class="button primary" id="send-turn" type="button" data-action="send" disabled>Send Turn</button>
          </div>
        </section>
      </section>
    </div>
  </section>`;

const workbench = `
  <section class="workspace-page workbench-page">
    ${workspaceHeader("workbench")}
    <div class="workbench">
      <aside class="file-tree" aria-label="Workspace files">
        <header><div class="eyebrow">Files</div><button class="bare-button" type="button" data-action="refresh">↻</button></header>
        <div class="tree-root"><span aria-hidden="true">⌄</span><strong>uprava</strong></div>
        <button class="file-item active" type="button" data-action="select-file" data-file="README.md"><span aria-hidden="true">▱</span> README.md</button>
        <button class="file-item" type="button" data-action="select-file" data-file="Cargo.toml"><span aria-hidden="true">▱</span> Cargo.toml</button>
        <div class="tree-folder"><span aria-hidden="true">⌄</span> docs</div>
        <button class="file-item nested" type="button" data-action="select-file" data-file="docs/vision.md"><span aria-hidden="true">▱</span> vision.md</button>
        <div class="tree-folder"><span aria-hidden="true">›</span> crates</div>
        <div class="tree-folder"><span aria-hidden="true">›</span> apps</div>
      </aside>

      <section class="editor-pane">
        <header class="pane-tabs">
          <button class="pane-tab active" type="button" data-action="editor-mode" data-mode="source"><span id="editor-tab-title">README.md</span><span class="dirty-indicator">●</span></button>
          <button class="pane-tab" type="button" data-action="editor-mode" data-mode="diff">Diff <span class="count">3</span></button>
          <div class="pane-tab-spacer"></div>
          <button class="button small" type="button" data-action="discard">Discard</button>
          <button class="button primary small" type="button" data-action="save">Save</button>
        </header>
        <div id="editor-body" class="editor-body">
          <div class="editor-gutter">1<br />2<br />3<br />4<br />5<br />6<br />7<br />8<br />9</div>
          <textarea id="file-editor" spellcheck="false" aria-label="File editor README.md"># Uprava

Distributed agent control plane and work surface.

## Local development

    make core-r
    make node-r
    make web-r</textarea>
        </div>
      </section>

      <section class="terminal-pane">
        <header class="pane-tabs terminal-tabs">
          <button class="pane-tab active" type="button">Terminal 1</button>
          <button class="pane-tab" type="button" data-action="prototype-only">Terminal 2</button>
          <button class="icon-button" type="button" data-action="new-terminal" aria-label="New Terminal">＋</button>
          <div class="pane-tab-spacer"></div>
          <span class="terminal-context">uprava · zsh</span>
          <button class="icon-button" type="button" data-action="prototype-only" aria-label="Close Terminal">×</button>
        </header>
        <div class="terminal-body" id="terminal-body">
          <div><span class="terminal-prompt">uprava@local</span>:<span class="terminal-path">/workspace/uprava</span>$ make l</div>
          <div class="terminal-muted">docs lint · rust lint · web lint</div>
          <div class="terminal-success">Checks passed in 1.8s</div>
          <div><span class="terminal-prompt">uprava@local</span>:<span class="terminal-path">/workspace/uprava</span>$ <span class="terminal-cursor" aria-hidden="true">▋</span></div>
        </div>
      </section>
    </div>
  </section>`;

const jobs = `
  <section class="workspace-page jobs-page">
    ${workspaceHeader("jobs")}
    <div class="jobs-layout">
      <aside class="job-list" aria-label="Workspace jobs">
        <header>
          <div><div class="eyebrow">Workspace Jobs</div><strong>3 definitions</strong></div>
          <button class="icon-button bordered" type="button" data-action="new-job" aria-label="Create Job">＋</button>
        </header>
        <button class="job-item active" type="button" data-inspect="job">
          <span class="job-title">Dependency audit</span>
          <span class="job-schedule">Every day · 02:00</span>
          <span class="status lifecycle">running</span>
        </button>
        <button class="job-item" type="button" data-action="prototype-only">
          <span class="job-title">Workspace review</span>
          <span class="job-schedule">Every Monday · 09:00</span>
          <span class="status lifecycle neutral">paused</span>
        </button>
        <button class="job-item" type="button" data-action="prototype-only">
          <span class="job-title">Release notes</span>
          <span class="job-schedule">Manual only</span>
          <span class="status lifecycle neutral">paused</span>
        </button>
      </aside>

      <section class="job-detail">
        <header class="job-detail-head">
          <div><div class="eyebrow">Background Job</div><h2>Dependency audit</h2><p>Inspect dependencies and report actionable risks.</p></div>
          <div class="header-actions"><span class="status lifecycle">running</span><button class="button" type="button" data-action="prototype-only">Pause Schedule</button><button class="button primary" type="button" data-action="run-job">Run Now</button></div>
        </header>

        <section class="job-config-grid">
          <div><span>Schedule</span><strong>Daily at 02:00</strong><small>Europe/Moscow</small></div>
          <div><span>Workspace</span><strong>Uprava</strong><small>/workspace/uprava</small></div>
          <div><span>Overlap policy</span><strong>Skip</strong><small>Maximum 1 active run</small></div>
          <div><span>Error policy</span><strong>Pause</strong><small>Manual review required</small></div>
        </section>

        <section class="run-history">
          <header class="section-header compact"><div><div class="eyebrow">Evidence</div><h3>Run History</h3></div><button class="text-button" type="button" data-action="prototype-only">View all →</button></header>
          <button class="run-row" type="button" data-inspect="job-run">
            <span class="run-symbol running">◌</span><span><strong>Run #18</strong><small>Started 4 min ago · Session Fix issue</small></span><span class="status lifecycle">running</span><span aria-hidden="true">→</span>
          </button>
          <button class="run-row" type="button" data-inspect="job-run-complete">
            <span class="run-symbol">✓</span><span><strong>Run #17</strong><small>Yesterday, 02:00 · 3 findings</small></span><span class="status lifecycle neutral">completed</span><span aria-hidden="true">→</span>
          </button>
          <button class="run-row" type="button" data-action="prototype-only">
            <span class="run-symbol">✓</span><span><strong>Run #16</strong><small>Jul 11, 02:00 · No findings</small></span><span class="status lifecycle neutral">completed</span><span aria-hidden="true">→</span>
          </button>
        </section>
      </section>
    </div>
  </section>`;

const settings = `
  <section class="page settings-page">
    <header class="page-header"><div><div class="eyebrow">Control Plane</div><h1>Settings</h1><p>Provider defaults and local execution limits.</p></div></header>
    <section class="settings-card">
      <label>Default provider<select><option>Codex</option></select></label>
      <label>Concurrent runtimes<input type="number" value="2" /></label>
      <label class="full-field">Provider executable<input value="codex" /></label>
      <div class="settings-actions"><button class="button primary" type="button" data-action="save-settings">Save Settings</button></div>
    </section>
  </section>`;

const views = {
  dashboard,
  node,
  "node-zarya": zaryaNode,
  agent,
  workbench,
  jobs,
  settings,
};

const routeLabels = {
  dashboard: "Dashboard",
  node: "Local Node",
  "node-zarya": "Zarya Server",
  agent: "Local Node / Uprava / Agent",
  workbench: "Local Node / Uprava / Workbench",
  jobs: "Local Node / Uprava / Jobs",
  settings: "Settings",
};

const inspectorCards = {
  node: {
    kind: "Node Presence",
    title: "Local Node is stale",
    summary: "The last heartbeat was received 24 minutes ago.",
    fields: [
      ["Source", "node.heartbeat"],
      ["Evidence", "Last seen at 12:36"],
      ["Cause", "Unknown"],
      ["Next action", "Check daemon connectivity"],
    ],
  },
  session: {
    kind: "Agent Session",
    title: "Fix issue",
    summary: "Persistent Codex session in the Uprava workspace.",
    fields: [
      ["Lifecycle", "active"],
      ["Attention", "blocked"],
      ["Provider", "Codex"],
      ["Workspace", "/workspace/uprava"],
    ],
  },
  approval: {
    kind: "Approval Request",
    title: "Run make c",
    summary: "The agent is waiting for permission before running a workspace check.",
    fields: [
      ["Scope", "Current workspace"],
      ["Command", "make c"],
      ["Requested by", "Codex session Fix issue"],
      ["Reversibility", "Read/check operation"],
    ],
  },
  check: {
    kind: "Check Result",
    title: "make l passed",
    summary: "Lightweight repository checks completed successfully.",
    fields: [
      ["Exit code", "0"],
      ["Duration", "1.8s"],
      ["Source", "Terminal 1"],
      ["Evidence", "Bounded output snapshot"],
    ],
  },
  diff: {
    kind: "Workspace Diff",
    title: "3 files changed",
    summary: "Current workspace snapshot, not an exact per-turn edit attribution.",
    fields: [
      ["Precision", "snapshot"],
      ["Added", "+84"],
      ["Removed", "−31"],
      ["Cause", "Agent turn 39"],
    ],
  },
  job: {
    kind: "Job Definition",
    title: "Dependency audit",
    summary: "A scheduled workspace-scoped background job.",
    fields: [
      ["Schedule", "Daily at 02:00"],
      ["Timezone", "Europe/Moscow"],
      ["Workspace", "Uprava"],
      ["State", "running"],
    ],
  },
  "job-run": {
    kind: "Job Run",
    title: "Dependency audit · Run #18",
    summary: "The current run has its own managed agent session.",
    fields: [
      ["State", "running"],
      ["Started", "4 minutes ago"],
      ["Session", "Fix issue"],
      ["Evidence", "Session trace available"],
    ],
  },
  "job-run-complete": {
    kind: "Job Run",
    title: "Dependency audit · Run #17",
    summary: "Completed with three reviewable findings.",
    fields: [
      ["State", "completed"],
      ["Result", "3 findings"],
      ["Duration", "8m 14s"],
      ["Evidence", "Session and workspace refs"],
    ],
  },
};

const fileContents = {
  "README.md": `# Uprava

Distributed agent control plane and work surface.

## Local development

    make core-r
    make node-r
    make web-r`,
  "Cargo.toml": `[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.2.5"
edition = "2024"`,
  "docs/vision.md": `# Vision

Uprava is a distributed agent operating system for large-scale work with AI agents.

The product is organized around nodes, workspaces, durable sessions and traceable evidence.`,
};

let currentRoute = "dashboard";
let selectedFile = "README.md";
let editorMode = "source";

function render(route, updateHash = true) {
  const normalized = route === "workspace" ? "agent" : route;
  currentRoute = views[normalized] ? normalized : "dashboard";
  main.innerHTML = views[currentRoute];
  topbarLocation.textContent = routeLabels[currentRoute];
  closeInspector();
  updateNavigation();
  if (updateHash) history.replaceState(null, "", `#${currentRoute}`);
  main.focus({ preventScroll: true });
}

function updateNavigation() {
  document.querySelectorAll("[data-route]").forEach((item) => {
    const itemRoute = item.dataset.route;
    const isActive =
      itemRoute === currentRoute ||
      (item.dataset.navContext === "local-node" &&
        (currentRoute === "node" || workspaceRoutes.has(currentRoute))) ||
      (item.dataset.navContext === "uprava" && workspaceRoutes.has(currentRoute));
    item.classList.toggle("active", isActive);
    if (isActive) item.setAttribute("aria-current", "page");
    else item.removeAttribute("aria-current");
  });
}

function openInspector(key) {
  const card = inspectorCards[key];
  if (!card) return;
  const fields = card.fields
    .map(([label, value]) => `<div><dt>${label}</dt><dd>${value}</dd></div>`)
    .join("");
  inspector.innerHTML = `
    <article class="inspector-card">
      <div class="eyebrow">${card.kind}</div>
      <h2>${card.title}</h2>
      <p>${card.summary}</p>
      <dl>${fields}</dl>
      <div class="inspector-actions">
        <button class="button" type="button" data-action="copy-reference">Copy Reference</button>
        <button class="text-button" type="button" data-action="prototype-only">Open Raw Event →</button>
      </div>
    </article>`;
  shellGrid.classList.add("inspector-open");
}

function closeInspector() {
  shellGrid.classList.remove("inspector-open");
  inspector.innerHTML = "";
}

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("visible");
  clearTimeout(showToast.timer);
  showToast.timer = setTimeout(() => toast.classList.remove("visible"), 1800);
}

function renderEditor() {
  const body = document.querySelector("#editor-body");
  if (!body) return;
  document.querySelectorAll("[data-action='editor-mode']").forEach((button) => {
    button.classList.toggle("active", button.dataset.mode === editorMode);
  });

  if (editorMode === "diff") {
    body.innerHTML = `<pre class="diff-view"><span class="diff-file">docs/visuals/prototype/app.js</span>
<span class="diff-hunk">@@ workspace navigation @@</span>
<span class="diff-remove">- global Nodes and Jobs routes</span>
<span class="diff-add">+ Node -> Workspace -> Agent / Workbench / Jobs</span>
<span class="diff-add">+ contextual source and evidence Inspector</span>

<span class="diff-file">docs/visuals/vdr/001-workspace-centered-navigation.md</span>
<span class="diff-add">+ visual decision record</span></pre>`;
    return;
  }

  const lines = fileContents[selectedFile].split("\n").length;
  body.innerHTML = `
    <div class="editor-gutter">${Array.from({ length: lines }, (_, index) => index + 1).join("<br />")}</div>
    <textarea id="file-editor" spellcheck="false" aria-label="File editor ${selectedFile}">${fileContents[selectedFile]}</textarea>`;
}

function handleAction(target) {
  const action = target.dataset.action;

  if (action === "toggle-sidebar") {
    const collapsed = appShell.classList.toggle("sidebar-collapsed");
    sidebarToggle.setAttribute("aria-expanded", String(!collapsed));
    sidebarToggle.setAttribute(
      "aria-label",
      collapsed ? "Show navigation" : "Hide navigation",
    );
    return;
  }

  if (action === "select-file") {
    selectedFile = target.dataset.file;
    editorMode = "source";
    document.querySelectorAll(".file-item").forEach((item) => {
      item.classList.toggle("active", item.dataset.file === selectedFile);
    });
    document.querySelector("#editor-tab-title").textContent = selectedFile;
    renderEditor();
    return;
  }

  if (action === "editor-mode") {
    editorMode = target.dataset.mode;
    renderEditor();
    return;
  }

  if (action === "approve" || action === "deny") {
    const blocker = document.querySelector("#current-blocker");
    if (blocker) {
      blocker.classList.add("resolved");
      blocker.innerHTML = `<div class="blocker-symbol" aria-hidden="true">✓</div><div class="blocker-copy"><div class="eyebrow">Approval Resolved</div><h3>${action === "approve" ? "Approved · agent may continue" : "Denied · agent remains paused"}</h3><p>The decision is preserved in the session trace.</p></div>`;
    }
    showToast(action === "approve" ? "Approval resolved" : "Approval denied");
    return;
  }

  if (action === "send") {
    const composer = document.querySelector("#composer");
    if (composer) composer.value = "";
    target.disabled = true;
    document.querySelector("#composer-state").textContent = "Ready";
    showToast("Prototype: turn accepted");
    return;
  }

  if (action === "new-terminal") {
    document.querySelector("#terminal-body").insertAdjacentHTML(
      "beforeend",
      '<div class="terminal-muted">New terminal would open in /workspace/uprava</div>',
    );
    showToast("Prototype: terminal created");
    return;
  }

  const messages = {
    "add-node": "Prototype: add Node flow",
    "add-workspace": "Prototype: add Workspace flow",
    "prototype-only": "This interaction is a visual placeholder",
    reconnect: "Connection check requested",
    refresh: "Workspace tree refreshed",
    discard: "Editor changes discarded",
    save: `${selectedFile} saved locally`,
    "start-session": "Prototype: Agent session started",
    "new-job": "Prototype: new workspace Job",
    "run-job": "Prototype: Job run started",
    "save-settings": "Settings saved",
    "copy-reference": "Reference copied",
  };
  showToast(messages[action] || "Prototype action completed");
}

document.addEventListener("click", (event) => {
  const routeTarget = event.target.closest("[data-route]");
  if (routeTarget) {
    event.preventDefault();
    render(routeTarget.dataset.route);
    return;
  }

  const inspectTarget = event.target.closest("[data-inspect]");
  if (inspectTarget) {
    openInspector(inspectTarget.dataset.inspect);
    return;
  }

  const actionTarget = event.target.closest("[data-action]");
  if (actionTarget) handleAction(actionTarget);
});

main.addEventListener("input", (event) => {
  if (event.target.id !== "composer") return;
  const hasDraft = Boolean(event.target.value.trim());
  document.querySelector("#send-turn").disabled = !hasDraft;
  document.querySelector("#composer-state").textContent = hasDraft
    ? "Draft not sent"
    : "Ready";
});

document.querySelector("#close-inspector").addEventListener("click", closeInspector);
window.addEventListener("hashchange", () => render(location.hash.slice(1), false));
render(location.hash.slice(1) || "dashboard", false);
