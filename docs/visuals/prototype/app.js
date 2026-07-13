const main = document.querySelector("#main-content");
const inspector = document.querySelector("#inspector-content");
const toast = document.querySelector("#toast");

const dashboard = `
  <section>
    <header class="page-header">
      <div>
        <div class="caption">SYS / Jun 17, 2026, 3:00 AM</div>
        <h1>Dashboard</h1>
        <p>Distributed runtime status &amp; current workload.</p>
      </div>
    </header>
    <section class="overview">
      <div>
        <div class="caption">System Overview</div>
        <h2><span aria-hidden="true">△</span> Review required</h2>
        <p>3 objects need review across nodes, workspaces, or runtimes.</p>
      </div>
      <dl>
        <div><dt>Primary Cause</dt><dd>Node is stale</dd></div>
        <div><dt>Affected Scope</dt><dd>3</dd></div>
        <div><dt>Next Action</dt><dd><button class="text-link" data-inspect="node">Review Local Node →</button></dd></div>
      </dl>
    </section>
    <section class="metrics">
      <div class="metric"><div class="caption">Core API</div><div class="value">ok</div><div class="detail">controlled dev</div></div>
      <div class="metric"><div class="caption">Reachable Nodes</div><div class="value risk">0/1</div><div class="detail">1 unavailable</div></div>
      <div class="metric"><div class="caption">Active Runtimes</div><div class="value">1</div><div class="detail">1 open sessions</div></div>
      <div class="metric"><div class="caption">Attention</div><div class="value risk">3</div><div class="detail">current deviations</div></div>
    </section>
    <section class="section-rule">
      <div class="section-heading"><div><div class="caption">Runtime Topology</div><h2>Control-to-Work Pipeline</h2></div><span class="badge risk">review</span></div>
      <div class="pipeline">
        <div class="pipeline-node"><strong>▤<br />Core</strong><span>ok</span></div><span class="pipeline-arrow">→</span>
        <button class="pipeline-node risk" data-inspect="node"><strong>▤<br />Nodes</strong><span>0/1</span></button><span class="pipeline-arrow">→</span>
        <button class="pipeline-node" data-route="workspace"><strong>▤<br />Workspaces</strong><span>1 valid</span></button><span class="pipeline-arrow">→</span>
        <button class="pipeline-node" data-route="session"><strong>▤<br />Sessions</strong><span>1 active</span></button>
      </div>
      <p class="caption">fig. Current placement &amp; runtime path. Deviations use a crossed risk mark.</p>
    </section>
    <section class="section-rule lower-grid">
      <div>
        <div class="list-heading"><h3>Recent Sessions</h3><span class="caption">Updated Jun 17, 2026, 3:00 AM</span></div>
        <div class="list-row"><button data-route="session"><span class="list-title">Fix issue</span><span class="list-detail">codex / 2 messages / Jun 17, 2026, 3:00 AM</span></button><span class="badges"><span class="badge">active</span><span class="badge dashed">blocked</span></span></div>
      </div>
      <div>
        <div class="list-heading"><div><h3>System Attention</h3><span class="caption">Cause &amp; next review target</span></div></div>
        <div class="list-row"><span><span class="list-title">Local Node</span><span class="list-detail">Node is stale</span></span><button class="badge dashed" data-inspect="node">review</button></div>
        <div class="list-row"><span><span class="list-title">Uprava</span><span class="list-detail">Dirty workspace</span></span><button class="badge dashed" data-route="workspace">review</button></div>
        <div class="list-row"><span><span class="list-title">Fix issue</span><span class="list-detail">Runtime is blocked</span></span><button class="badge dashed" data-route="session">review</button></div>
      </div>
    </section>
  </section>`;

const workspace = `
  <section>
    <header class="workspace-head">
      <div><h1>Uprava</h1><div class="workspace-path">/workspace/uprava</div></div>
      <div class="button-row"><button class="button" data-action="refresh">↻ Refresh</button><button class="button danger" data-action="delete">▱ Delete</button><button class="button primary" data-action="start">▷ Start Codex</button></div>
    </header>
    <label class="workspace-options"><input type="checkbox" />Force start if the provider reports 5% or less quota remaining</label>
    <div class="workspace-meta"><span class="badge">validated</span><span class="badge dashed">Dirty workspace</span><button class="text-link" data-inspect="node">Open node</button></div>
    <div class="inspector-title"><div><h2>Workspace Inspector</h2><div class="workspace-path">/workspace/uprava</div></div><button class="button" data-action="refresh">↻ Refresh</button></div>
    <section class="panel"><div class="panel-head">Files</div><button class="file-row" data-action="open-file">▱ README.md</button></section>
    <section class="panel"><div class="panel-head"><span id="file-title">Editor</span><div class="button-row"><button class="button" data-action="discard" hidden>Discard</button><button class="button primary" data-action="save" hidden>Save</button></div></div><div id="editor-slot" class="editor-empty">No text file selected</div></section>
    <section class="panel"><div class="panel-head"><span>▣ Terminal</span><button class="button" data-action="terminal">＋ New</button></div><div id="terminal-slot" class="empty-line" style="min-height:168px;display:grid;place-items:center">No terminal open</div></section>
    <section class="panel"><div class="panel-head"><span>▣ Command</span><div class="button-row"><button class="button" data-command="make l">◉ make l</button><button class="button" data-command="make c">◉ make c</button></div></div><div class="command-line"><input id="command-input" class="input" value="make l" aria-label="Command" /><button class="button primary" data-action="run">▷ Run</button></div><div id="command-output"></div></section>
    <section class="panel"><div class="panel-head"><span>⑂ Diff</span><button class="button" data-action="diff">↻ Refresh</button></div><div id="diff-slot" class="empty-line">No diff loaded</div></section>
    <section class="panel"><div class="panel-head"><span>◷ History</span></div><div id="history-slot" class="empty-line">No commands recorded</div></section>
  </section>`;

const session = `
  <section>
    <header class="page-header session-head"><div><div class="caption">SESSION / codex / blocked</div><h1>Fix issue</h1><div class="session-path">Uprava / /workspace/uprava</div><div class="button-row"><button class="button" data-route="workspace">▱ Workspace</button><button class="button" data-inspect="session">＋</button><button class="button" data-inspect="session">⊞</button><span class="badge">active</span><span class="badge dashed">blocked</span></div></div></header>
    <div class="session-layout">
      <div>
        <section class="runtime-context"><div class="caption">Runtime Context</div><strong>Phase blocked · Session active · Resume supported</strong><p class="caption">Stop and interrupt can end active work. Detach preserves the managed runtime.</p><div class="button-row"><button class="button" disabled>⇥ Attach</button><button class="button" disabled>⇥ Detach</button><button class="button" disabled>Ⅱ Cancel</button><button class="button" disabled>□ Stop</button><button class="button" disabled>↶ Resume</button></div></section>
        <div class="timeline">
          <article class="timeline-block"><h3>♙ Operator Input</h3><p>Please continue</p><div class="timeline-actions"><button class="text-link" data-inspect="operator">＋</button><button class="text-link" data-inspect="operator">⊞</button></div></article>
          <article class="timeline-block"><h3>♙ Agent Output</h3><p>Assistant reply</p><p class="caption">Evidence &amp; source are available through the + reference layer.</p><div class="timeline-actions"><button class="text-link" data-inspect="assistant">＋</button><button class="text-link" data-inspect="assistant">⊞</button></div></article>
          <article class="timeline-block"><h3><span class="badge dashed">◉ Approval</span> <span class="caption">approval-1</span></h3><p>Allow command?</p><p class="caption">Affected Scope</p><p>Current runtime</p><p class="caption">Risk</p><p>Command-dependent</p><p class="caption">Reversibility</p><p>Review before approval</p><div class="timeline-actions"><button class="text-link" data-inspect="approval">＋</button><button class="text-link" data-inspect="approval">⊞</button><button class="button primary" data-action="approve">Approve</button><button class="button danger" data-action="deny">Deny</button></div></article>
          <article class="timeline-block error"><h3>△ RUNTIME.ERROR</h3><p>Provider failed safely</p><p class="caption" style="color:inherit">Affected scope: current runtime. Next safe step: inspect the source event, then retry or stop.</p><div class="timeline-actions"><button class="text-link" data-inspect="error">＋</button><button class="text-link" data-inspect="error">⊞</button></div></article>
        </div>
        <section class="composer"><div class="composer-head"><strong>Next Agent Turn</strong><span class="caption">Draft stays until the turn is accepted.</span></div><textarea id="composer" class="input" placeholder="Send a turn" aria-label="Next Agent Turn"></textarea><div class="composer-foot"><span id="composer-state">Ready</span><button id="send-turn" class="button" disabled data-action="send">⌁ Send Turn</button></div></section>
      </div>
      <aside>
        <section class="projection"><div class="caption">Evidence</div><h2>Evidence Projection</h2><div class="projection-row">Session-local index</div><button class="projection-row text-link" data-inspect="assistant">Assistant reply ⊞</button></section>
        <section class="projection"><div class="caption">Agent Context</div><h2>Agent Projection</h2><p class="caption">Known event source refs are preserved</p><div class="button-row"><span class="badge dashed">Dirty workspace</span><button class="button" data-action="acknowledge">Acknowledge</button></div><div class="button-row" style="margin-top:12px"><span class="badge">session.sendTurn</span><span class="badge">approval.resolve</span><span class="badge">warning.acknowledge</span></div></section>
      </aside>
    </div>
  </section>`;

const jobs = `
  <section><header class="page-header"><div><div class="caption">AUTOMATION / DURABLE WORK</div><h1>Background Jobs</h1><p>Each run gets its own managed session in the selected workspace. Jobs start paused so you can test them manually before enabling a schedule.</p></div></header>
  <form class="form-card" data-action="job-form"><strong>＋ New paused Job</strong><div class="form-grid" style="margin-top:16px"><label class="field">Name<input class="input" placeholder="Dependency audit" /></label><label class="field">Workspace<select class="input"><option>Uprava</option></select></label><label class="field full">Prompt / task contract<textarea class="input" placeholder="Inspect dependencies and report actionable risks."></textarea></label><label class="field">Schedule<select class="input"><option>Interval</option><option>Daily</option><option>Weekly</option><option>Manual only</option></select></label><label class="field">Every, minutes<input class="input" type="number" value="60" /></label><label class="field">IANA timezone<input class="input" value="Europe/Moscow" /></label></div><button class="button primary" type="submit" style="margin-top:16px">Create paused Job</button></form>
  <div class="list-row"><span><span class="list-title">Nightly workspace review</span><span class="list-detail">Uprava · every day at 02:00 Europe/Moscow</span></span><span class="badges"><span class="badge">paused</span><span class="badge">latest: completed</span></span></div></section>`;

const nodes = `
  <section><header class="page-header"><div><h1>Nodes</h1><p>Registered runtime environments and current heartbeat state.</p></div></header>
  <article class="node-card"><div><h2>Local Node</h2><p>heartbeat 24 minutes ago</p><div class="button-row"><span class="badge">1 active sessions</span><span class="badge">codex</span><span class="badge">workspace.fs</span></div><div class="button-row" style="margin-top:16px"><button class="button" data-route="workspace">＋ Workspace</button><button class="button danger" data-action="delete">▱ Delete</button></div></div><span class="badge risk">stale</span></article>
  <article class="node-card"><div><h2>Zarya Server</h2><p>heartbeat 4 seconds ago</p><div class="button-row"><span class="badge">0 active sessions</span><span class="badge">codex</span></div></div><span class="badge">online</span></article></section>`;

const settings = `
  <section><header class="page-header"><div><div class="caption">SETTINGS / RUNTIME</div><h1>Runtime Settings</h1><p>Provider defaults and local execution limits.</p></div></header><div class="form-card"><div class="form-grid"><label class="field">Default provider<select class="input"><option>Codex</option></select></label><label class="field">Concurrent runtimes<input class="input" type="number" value="2" /></label><label class="field full">Provider executable<input class="input" value="codex" /></label></div><button class="button primary" data-action="save-settings" style="margin-top:16px">Save settings</button></div></section>`;

const views = { dashboard, workspace, session, jobs, nodes, settings };

const inspectorCards = {
  node: [
    "cause",
    "Local Node",
    ["Presence", "stale"],
    ["Last heartbeat", "24 minutes ago"],
    ["Next action", "Check daemon connectivity"],
  ],
  session: [
    "session",
    "Fix issue",
    ["Provider", "codex"],
    ["State", "blocked"],
    ["Workspace", "/workspace/uprava"],
  ],
  operator: [
    "operator message",
    "Please continue",
    ["Sequence", "38"],
    ["Source", "session.sendTurn"],
  ],
  assistant: [
    "assistant message",
    "Assistant reply",
    ["Sequence", "39"],
    ["Evidence", "Session-local index"],
  ],
  approval: [
    "approval",
    "approval-1",
    ["Risk", "Command-dependent"],
    ["Reversibility", "Review before approval"],
  ],
  error: [
    "runtime event",
    "runtime.error",
    ["Scope", "current runtime"],
    ["Recovery", "Inspect, retry, or stop"],
  ],
};

function render(route, updateHash = true) {
  const next = views[route] ? route : "dashboard";
  main.innerHTML = views[next];
  document
    .querySelectorAll("[data-route]")
    .forEach((item) =>
      item.classList.toggle("active", item.dataset.route === next),
    );
  inspector.innerHTML =
    "<p>Select a source, cause, or evidence reference to inspect it here.</p>";
  if (updateHash) history.replaceState(null, "", `#${next}`);
  wireView();
  main.focus({ preventScroll: true });
}

function inspect(key) {
  const card = inspectorCards[key];
  if (!card) return;
  const fields = card
    .slice(2)
    .map(([label, value]) => `<div><dt>${label}</dt><dd>${value}</dd></div>`)
    .join("");
  inspector.innerHTML = `<article class="inspector-card"><div class="eyebrow">${card[0]}</div><h2>${card[1]}</h2><dl>${fields}</dl></article>`;
  if (window.innerWidth <= 1200) showToast(`Inspector: ${card[1]}`);
}

function showToast(message) {
  toast.textContent = message;
  toast.classList.add("visible");
  clearTimeout(showToast.timer);
  showToast.timer = setTimeout(() => toast.classList.remove("visible"), 1800);
}

function wireView() {
  main
    .querySelectorAll("[data-inspect]")
    .forEach((item) =>
      item.addEventListener("click", () => inspect(item.dataset.inspect)),
    );
  main
    .querySelectorAll("[data-route]")
    .forEach((item) =>
      item.addEventListener("click", () => render(item.dataset.route)),
    );
  main.querySelectorAll("[data-command]").forEach((item) =>
    item.addEventListener("click", () => {
      document.querySelector("#command-input").value = item.dataset.command;
    }),
  );
  const composer = document.querySelector("#composer");
  composer?.addEventListener("input", () => {
    document.querySelector("#send-turn").disabled = !composer.value.trim();
    document.querySelector("#composer-state").textContent =
      composer.value.trim() ? "Draft not sent" : "Ready";
  });
  const form = main.querySelector("form[data-action='job-form']");
  form?.addEventListener("submit", (event) => {
    event.preventDefault();
    showToast("Prototype: paused Job created");
  });
  main
    .querySelectorAll("[data-action]")
    .forEach((item) => item.addEventListener("click", handleAction));
}

function handleAction(event) {
  const action = event.currentTarget.dataset.action;
  if (action === "open-file") {
    document.querySelector("#file-title").textContent = "README.md";
    document.querySelector("#editor-slot").outerHTML =
      `<textarea id="editor-slot" class="editor" aria-label="File editor README.md"># Uprava\n\nDistributed agent control plane and work surface.\n\n## Local development\n\n    make core-r\n    make node-r\n    make web-r\n</textarea>`;
    document
      .querySelectorAll("[data-action='discard'], [data-action='save']")
      .forEach((button) => {
        button.hidden = false;
      });
    return;
  }
  if (action === "terminal") {
    document.querySelector("#terminal-slot").outerHTML =
      `<div id="terminal-slot" class="terminal-body">uprava@local:/workspace/uprava$ <span aria-hidden="true">▋</span></div>`;
    return;
  }
  if (action === "run") {
    const command = document.querySelector("#command-input").value;
    document.querySelector("#command-output").innerHTML =
      `<div class="empty-line"><strong>${command}</strong><br />Checks passed in 1.8s · exit 0</div>`;
    document.querySelector("#history-slot").innerHTML =
      `<div class="list-row"><span><span class="list-title">${command}</span><span class="list-detail">just now · exit 0 · 1.8s</span></span><span class="badge">Success</span></div>`;
    showToast("Mock command completed");
    return;
  }
  if (action === "diff") {
    document.querySelector("#diff-slot").innerHTML =
      `<pre style="margin:0;padding:14px;color:var(--muted)"> README.md | 2 ++\n 1 file changed</pre>`;
    return;
  }
  if (action === "send") {
    showToast("Prototype: turn accepted");
    document.querySelector("#composer").value = "";
    event.currentTarget.disabled = true;
    document.querySelector("#composer-state").textContent = "Ready";
    return;
  }
  const messages = {
    refresh: "Mock data refreshed",
    delete: "Destructive action is disabled in the prototype",
    start: "Prototype: Codex session started",
    save: "README.md saved locally",
    discard: "Local draft discarded",
    approve: "Approval resolved: approved",
    deny: "Approval resolved: denied",
    acknowledge: "Warning acknowledged",
    "save-settings": "Runtime settings saved",
  };
  showToast(messages[action] || "Prototype action completed");
}

document.querySelectorAll("body > .app-shell [data-route]").forEach((item) =>
  item.addEventListener("click", (event) => {
    event.preventDefault();
    render(item.dataset.route);
  }),
);
window.addEventListener("hashchange", () =>
  render(location.hash.slice(1), false),
);
render(location.hash.slice(1) || "dashboard", false);
