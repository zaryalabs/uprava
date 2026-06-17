import { expect, test } from "@playwright/test";

test("renders the control panel shell", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByRole("link", { name: "Cortex" })).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Inventory tree" }),
  ).toBeVisible();
});

test("renders warning badges and structured session blocks from snapshots", async ({
  page,
}) => {
  const core = await mockCoreApi(page);

  await page.goto("/nodes");
  await expect(page.getByText("Local Node").first()).toBeVisible();
  await expect(page.getByText("stale", { exact: true }).first()).toBeVisible();

  await page.goto("/placements/placement-1");
  await expect(
    page.getByRole("main").getByText("Dirty workspace"),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: /Start/i })).toBeEnabled();

  await page.goto("/nodes/node-1/placements/new");
  await expect(page.getByRole("button", { name: "Validate" })).toBeDisabled();
  await page.getByLabel("Workspace path").fill("/missing/workspace");
  await page.getByRole("button", { name: "Validate" }).click();
  await expect(page.getByText("Workspace validation failed")).toBeVisible();
  await expect(page.getByText("placement.invalid")).toBeVisible();
  await page.getByLabel("Workspace path").fill("/workspace/cortex");
  await page.getByRole("button", { name: "Validate" }).click();
  await expect(page).toHaveURL(/\/placements\/placement-1$/);
  await expect.poll(() => core.validationAttempts).toBe(2);

  await page.goto("/sessions/session-1");
  const main = page.getByRole("main");
  await expect(main.getByText("blocked", { exact: true })).toBeVisible();
  await expect(main.getByText("Assistant reply").first()).toBeVisible();
  await expect(main.getByText("Allow command?")).toBeVisible();
  await expect(main.getByRole("button", { name: "Approve" })).toBeVisible();
  await expect(main.getByText("runtime.error")).toBeVisible();
  await expect(main.getByText("Session-local index").first()).toBeVisible();
  await page.getByRole("button", { name: "Inspect Assistant reply" }).click();
  await expect(
    page.getByRole("heading", { name: "assistant message" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Open approval approval-1 in inspector" })
    .click();
  await expect(page.getByRole("heading", { name: "approval-1" })).toBeVisible();
  await expect(page.getByText("session.sendTurn")).toBeVisible();
  await page.getByRole("button", { name: "Acknowledge" }).click();
  await expect.poll(() => core.warningAcknowledged).toBe(true);
});

async function mockCoreApi(page: import("@playwright/test").Page) {
  const state = { validationAttempts: 0, warningAcknowledged: false };
  await page.route("**/api/v1/health", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ status: "ok", profile: "local_trusted" }),
    });
  });
  await page.route("**/api/v1/inventory", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(inventory),
    });
  });
  await page.route("**/api/v1/node-enrollments", async (route) => {
    await route.fulfill({ contentType: "application/json", body: json([]) });
  });
  await page.route("**/api/v1/placements/placement-1", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(placement),
    });
  });
  await page.route("**/api/v1/project-placements/validate", async (route) => {
    state.validationAttempts += 1;
    if (state.validationAttempts === 1) {
      await route.fulfill({
        status: 400,
        contentType: "application/json",
        body: json({
          error_code: "placement.invalid",
          message: "Workspace path is not available",
          retryable: false,
          correlation_id: "corr-placement",
        }),
      });
      return;
    }
    await route.fulfill({
      contentType: "application/json",
      body: json(placement),
    });
  });
  await page.route(
    "**/api/v1/sessions/session-1/artifact-tree",
    async (route) => {
      await route.fulfill({
        contentType: "application/json",
        body: json(artifactTree),
      });
    },
  );
  await page.route(
    "**/api/v1/sessions/session-1/agent-projection",
    async (route) => {
      await route.fulfill({
        contentType: "application/json",
        body: json(agentProjection),
      });
    },
  );
  await page.route("**/api/v1/sessions/session-1/stream?**", async (route) => {
    await route.fulfill({
      contentType: "text/event-stream",
      body: "",
    });
  });
  await page.route(
    "**/api/v1/sessions/session-1/warnings/dirty_workspace/acknowledge",
    async (route) => {
      state.warningAcknowledged = true;
      await route.fulfill({
        contentType: "application/json",
        body: json({
          event_id: "event-warning-acknowledged",
          session: sessionDetail,
        }),
      });
    },
  );
  await page.route("**/api/v1/sessions/session-1", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(sessionDetail),
    });
  });
  return state;
}

function json(value: unknown) {
  return JSON.stringify(value);
}

const runtime = {
  runtime_session_id: "runtime-1",
  provider: "fake",
  state: "blocked",
  resume_supported: true,
  degraded_reason: null,
  last_runtime_step_at: "2026-06-17T00:00:00Z",
};

const placement = {
  project_placement_id: "placement-1",
  project_id: "project-1",
  node_id: "node-1",
  display_name: "Cortex",
  workspace_path: "/workspace/cortex",
  state: "validated",
  resource_badges: [
    { kind: "dirty_workspace", severity: "warning", label: "Dirty workspace" },
  ],
  last_validated_at: "2026-06-17T00:00:00Z",
};

const session = {
  session_thread_id: "session-1",
  project_placement_id: "placement-1",
  runtime_session_id: "runtime-1",
  title: "Fix issue",
  state: "active",
  runtime,
  message_count: 2,
  updated_at: "2026-06-17T00:00:00Z",
};

const inventory = {
  nodes: [
    {
      node_id: "node-1",
      display_name: "Local Node",
      presence: "stale",
      sleep_hint: "unknown",
      heartbeat_age_seconds: 75,
      active_runtime_count: 1,
      capabilities: [{ key: "provider.fake", value: { configured: true } }],
      diagnostics: "last heartbeat stale",
    },
  ],
  placements: [placement],
  sessions: [session],
  generated_at: "2026-06-17T00:00:00Z",
};

const messageEvent = {
  event_id: "event-message",
  command_id: "command-1",
  actor_ref: { kind: "provider" },
  scope_ref: { kind: "runtime", runtime_session_id: "runtime-1" },
  node_id: "node-1",
  runtime_session_id: "runtime-1",
  session_thread_id: "session-1",
  turn_id: "turn-1",
  seq: 1,
  kind: "provider.message.completed",
  happened_at: "2026-06-17T00:00:01Z",
  source_refs: [],
  evidence_refs: [],
  cause_refs: [],
  result_refs: [],
  payload: { content: "Assistant reply" },
};

const approvalEvent = {
  ...messageEvent,
  event_id: "event-approval",
  seq: 2,
  kind: "approval.requested",
  payload: { approval_id: "approval-1", prompt: "Allow command?" },
};

const runtimeErrorEvent = {
  ...messageEvent,
  event_id: "event-runtime-error",
  seq: 3,
  kind: "runtime.error",
  payload: { code: "provider.failed", message: "Provider failed safely" },
};

const sessionDetail = {
  session,
  placement,
  messages: [
    {
      message_id: "message-user",
      session_thread_id: "session-1",
      turn_id: "turn-1",
      role: "user",
      content: "Please continue",
      created_at: "2026-06-17T00:00:00Z",
      completed_at: "2026-06-17T00:00:00Z",
      source_event_id: null,
    },
    {
      message_id: "message-assistant",
      session_thread_id: "session-1",
      turn_id: "turn-1",
      role: "assistant",
      content: "Assistant reply",
      created_at: "2026-06-17T00:00:01Z",
      completed_at: "2026-06-17T00:00:01Z",
      source_event_id: "event-message",
    },
  ],
  events: [messageEvent, approvalEvent, runtimeErrorEvent],
};

const artifactTree = {
  session_thread_id: "session-1",
  generated_at: "2026-06-17T00:00:00Z",
  root: {
    artifact_id: "artifact-root",
    label: "Session-local index",
    primary_ref: { kind: "session", session_thread_id: "session-1" },
    source_refs: [],
    evidence_refs: [],
    cause_refs: [],
    children: [
      {
        artifact_id: "artifact-message",
        label: "Assistant reply",
        primary_ref: { kind: "message", message_id: "message-assistant" },
        source_refs: [],
        evidence_refs: [],
        cause_refs: [],
        children: [],
      },
    ],
  },
};

const agentProjection = {
  session_thread_id: "session-1",
  project_placement: placement,
  runtime_summary: runtime,
  current_turn: "turn-1",
  pending_approvals: ["approval-1"],
  active_warnings: [
    { kind: "dirty_workspace", severity: "warning", label: "Dirty workspace" },
  ],
  recent_turn_summaries: ["turn-1 running"],
  recent_message_refs: [{ kind: "message", message_id: "message-assistant" }],
  artifact_tree_summary:
    "Session-local index: 2 messages, 3 events, 1 pending approvals",
  available_block_types: ["core.assistant-message", "core.approval-request"],
  available_commands: ["session.sendTurn", "approval.resolve"],
  visible_refs: [{ kind: "session", session_thread_id: "session-1" }],
  source_cause_summary: "Known event source refs are preserved",
  resume_context: "Runtime blocked on approval",
  generated_at: "2026-06-17T00:00:00Z",
};
