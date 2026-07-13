import { expect, test } from "@playwright/test";

test("renders the control panel shell", async ({ page }) => {
  await mockCoreApi(page);
  await page.goto("/");

  await expect(
    page.getByRole("link", { name: "Uprava", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Primary navigation" }),
  ).toBeVisible();
  await expect(
    page.getByRole("navigation", { name: "Inventory tree" }),
  ).toBeVisible();
  const primaryNavigation = page.getByRole("navigation", {
    name: "Primary navigation",
  });
  await expect(primaryNavigation.getByRole("link")).toHaveCount(1);
  await expect(
    primaryNavigation.getByRole("link", { name: "Dashboard" }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: "Add Node" })).toHaveAttribute(
    "href",
    "/nodes/pair",
  );
  await expect(
    page.getByRole("complementary", { name: "Context Inspector" }),
  ).toHaveCount(0);

  const hideNavigation = page.getByRole("button", {
    name: "Hide navigation",
  });
  await expect(hideNavigation).toHaveAttribute("aria-expanded", "true");
  await hideNavigation.click();
  await expect(
    page.getByRole("button", { name: "Show navigation" }),
  ).toHaveAttribute("aria-expanded", "false");
  await expect(
    page.getByRole("complementary", {
      name: "Node and workspace navigation",
    }),
  ).toHaveCount(0);
  await page.reload();
  await expect(
    page.getByRole("button", { name: "Show navigation" }),
  ).toHaveAttribute("aria-expanded", "false");
});

test("renders warning badges and structured session blocks from snapshots", async ({
  page,
}) => {
  const core = await mockCoreApi(page);

  await page.goto("/nodes");
  await expect(page.getByText("Local Node").first()).toBeVisible();
  await expect(page.getByText("stale", { exact: true }).first()).toBeVisible();

  await page.goto("/workspaces/placement-1");
  await expect(
    page.getByRole("img", { name: "Workspace: Dirty workspace" }),
  ).toBeVisible();
  await expect(page.getByRole("heading", { name: "Agent" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Start Codex" })).toBeEnabled();
  await expect(page.getByRole("link", { name: /Fix issue/ })).toHaveAttribute(
    "aria-current",
    "page",
  );

  await page.goto("/nodes/node-1/placements/new");
  await expect(page.getByRole("button", { name: "Validate" })).toBeEnabled();
  await page
    .getByRole("combobox", { name: "Workspace path" })
    .fill("/missing/workspace");
  await page.getByRole("button", { name: "Validate" }).click();
  await expect(page.getByText("Workspace validation failed")).toBeVisible();
  await expect(page.getByText("placement.invalid")).toBeVisible();
  await page
    .getByRole("combobox", { name: "Workspace path" })
    .fill("/workspace/uprava");
  await page.getByRole("button", { name: "Validate" }).click();
  await expect(page).toHaveURL(/\/workspaces\/placement-1\/agent\/session-1$/);
  await expect.poll(() => core.validationAttempts).toBe(2);

  await page.goto("/sessions/session-1");
  const main = page.getByRole("main");
  await expect(
    main.getByText("blocked", { exact: true }).first(),
  ).toBeVisible();
  await expect(main.getByText("Assistant reply").first()).toBeVisible();
  await expect(main.getByText("Allow command?")).toBeVisible();
  await expect(main.getByRole("button", { name: "Approve" })).toBeVisible();
  await expect(main.getByText("runtime.error")).toBeVisible();
  await page.getByText("Session details").click();
  await expect(main.getByText("Session-local index").first()).toBeVisible();
  await page.getByRole("button", { name: "Inspect Assistant reply" }).click();
  await expect(
    page.getByRole("heading", { name: "assistant message" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Open approval approval-1 in inspector" })
    .click();
  await expect(page.getByRole("heading", { name: "approval-1" })).toBeVisible();
  const inspector = page.getByRole("complementary", {
    name: "Context Inspector",
  });
  await expect(inspector).toBeVisible();
  expect(
    await inspector.evaluate((element) => getComputedStyle(element).position),
  ).toBe("fixed");
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("heading", { name: "assistant message" }),
  ).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(inspector).toHaveCount(0);
  await expect(page.getByText("session.sendTurn")).toBeVisible();
  await page.getByRole("button", { name: "Acknowledge" }).click();
  await expect.poll(() => core.warningAcknowledged).toBe(true);
});

test("loads Workbench chunks on demand and refits after shell changes", async ({
  page,
}) => {
  test.setTimeout(60_000);
  const resources: string[] = [];
  page.on("response", (response) => resources.push(response.url()));
  const core = await mockCoreApi(page);
  await page.setViewportSize({ width: 1440, height: 1000 });

  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  expect(resources.some(isMonacoResource)).toBe(false);
  expect(resources.some(isXtermResource)).toBe(false);

  await page.goto("/workspaces/placement-1/workbench");
  await expect(page.getByRole("button", { name: "Start Codex" })).toHaveCount(
    0,
  );
  await expect(
    page.getByRole("heading", { name: "Workbench", level: 2 }),
  ).toBeVisible();
  await expect(page.getByText("No terminal open")).toBeVisible();
  expect(core.diffRequests).toBe(0);
  await page.getByRole("treeitem", { name: "README.md" }).click();
  await expect(
    page.getByRole("region", { name: "File editor README.md" }),
  ).toBeVisible({ timeout: 15_000 });
  await expect.poll(() => resources.some(isMonacoResource)).toBe(true);
  expect(resources.some(isXtermResource)).toBe(false);

  await page.getByRole("tab", { name: "Diff" }).click();
  await expect(
    page.getByRole("region", { name: "Workspace diff viewer" }),
  ).toBeVisible({ timeout: 15_000 });
  expect(core.diffRequests).toBe(1);
  await page.getByRole("tab", { name: "Source" }).click();

  await page.getByRole("button", { name: "New" }).click();
  const terminal = page.getByRole("region", { name: "Terminal /bin/zsh" });
  await expect(terminal).toBeVisible({ timeout: 15_000 });
  await expect.poll(() => resources.some(isXtermResource)).toBe(true);
  const initialResizeFrames = await countResizeFrames(page);
  expect(initialResizeFrames).toBeGreaterThan(0);

  await page.getByRole("button", { name: "Hide navigation" }).click();
  await expect
    .poll(() => countResizeFrames(page))
    .toBeGreaterThan(initialResizeFrames);
  const collapsedResizeFrames = await countResizeFrames(page);
  await page
    .getByRole("button", { name: "Open workspace placement-1 in inspector" })
    .first()
    .click();
  await expect(
    page.getByRole("complementary", { name: "Context Inspector" }),
  ).toBeVisible();
  await expect
    .poll(() => countResizeFrames(page))
    .toBeGreaterThan(collapsedResizeFrames);
  await expect(terminal).toBeVisible();
  expect(await horizontalOverflow(page)).toBeLessThanOrEqual(1);
});

test("keeps Jobs scoped to the workspace through create, detail, and run", async ({
  page,
}) => {
  const core = await mockCoreApi(page);

  await page.goto("/workspaces/placement-1/jobs");
  await expect(
    page.getByRole("heading", { name: "Background Jobs" }),
  ).toBeVisible();
  await expect(page.getByRole("link", { name: /Nightly check/ })).toBeVisible();
  await expect(page.getByText("Other workspace Job")).toHaveCount(0);
  expect(core.jobsRequests).toBe(1);

  await page.getByRole("link", { name: "Create Job" }).first().click();
  await expect(
    page.getByRole("heading", { name: "New paused Job" }),
  ).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Workspace" })).toHaveCount(
    0,
  );
  await page.getByRole("textbox", { name: "Name" }).fill("Created check");
  await page
    .getByRole("textbox", { name: "Prompt / task contract" })
    .fill("Inspect workspace");
  await page.getByRole("button", { name: "Create paused Job" }).click();
  await expect(
    page.getByRole("heading", { name: "Created check" }),
  ).toBeVisible();
  expect(core.createdJobRequest).toMatchObject({
    name: "Created check",
    project_placement_id: "placement-1",
    prompt: "Inspect workspace",
    provider: "codex",
    schedule: { kind: "interval", minutes: 60 },
    continue_after_error: false,
  });

  await page.goto("/workspaces/placement-1/jobs/job-1");
  await expect(
    page.getByRole("heading", { name: "Nightly check" }),
  ).toBeVisible();
  await page.getByRole("link", { name: /Completed/ }).click();
  await expect(page.getByRole("heading", { name: "Run run-1" })).toBeVisible();
  await expect(
    page.getByRole("link", { name: "Open session output and evidence" }),
  ).toHaveAttribute("href", "/workspaces/placement-1/agent/session-1");

  await page.goto("/job-runs/run-1?inspect=bad-ref");
  await expect(page).toHaveURL(
    /\/workspaces\/placement-1\/jobs\/job-1\/runs\/run-1\?inspect=bad-ref$/,
  );

  await page.setViewportSize({ width: 1024, height: 900 });
  expect(await horizontalOverflow(page)).toBeLessThanOrEqual(1);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/workspaces/placement-1/jobs/job-1");
  await expect(
    page.getByRole("heading", { name: "Nightly check" }),
  ).toBeVisible();
  expect(await horizontalOverflow(page)).toBeLessThanOrEqual(1);
});

test("matches stable Zarya sheets and keeps the mobile session usable", async ({
  page,
}) => {
  await mockCoreApi(page);
  await page.clock.setFixedTime(new Date("2026-07-13T13:45:00Z"));
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto("/dashboard");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  await expect(page).toHaveScreenshot("dashboard-zarya-desktop.png", {
    animations: "disabled",
    fullPage: true,
  });

  await page.goto("/workspaces/placement-1/agent/session-1");
  await expect(page.getByRole("heading", { name: "Fix issue" })).toBeVisible();
  await expect(page).toHaveScreenshot("workspace-agent-desktop.png", {
    animations: "disabled",
    fullPage: true,
  });

  await page.goto("/workspaces/placement-1/jobs");
  await expect(page.getByRole("link", { name: /Nightly check/ })).toBeVisible();
  await expect(page).toHaveScreenshot("workspace-jobs-desktop.png", {
    animations: "disabled",
    fullPage: true,
  });
  await page.getByRole("link", { name: /Nightly check/ }).click();
  await expect(
    page.getByRole("heading", { name: "Nightly check" }),
  ).toBeVisible();
  await expect(page).toHaveScreenshot("workspace-job-detail-desktop.png", {
    animations: "disabled",
    fullPage: true,
  });
  await page.getByRole("link", { name: /Completed/ }).click();
  await expect(page.getByRole("heading", { name: "Run run-1" })).toBeVisible();
  await page.getByRole("main").focus();
  await expect(page).toHaveScreenshot("workspace-job-run-desktop.png", {
    animations: "disabled",
    fullPage: false,
  });

  await page.goto("/workspaces/placement-1/workbench");
  await page.getByRole("treeitem", { name: "README.md" }).click();
  await expect(
    page.getByRole("region", { name: "File editor README.md" }),
  ).toBeVisible({ timeout: 15_000 });
  await page.getByRole("button", { name: "New" }).click();
  await expect(
    page.getByRole("region", { name: "Terminal /bin/zsh" }),
  ).toBeVisible({ timeout: 15_000 });
  await expect(page).toHaveScreenshot("workspace-workbench-desktop.png", {
    animations: "disabled",
    fullPage: true,
  });
  await page.getByRole("button", { name: "Hide navigation" }).click();
  await expect.poll(() => workbenchWidth(page)).toBeGreaterThan(1_200);
  await expect(page).toHaveScreenshot(
    "workspace-workbench-collapsed-desktop.png",
    {
      animations: "disabled",
      fullPage: false,
    },
  );
  await page.getByRole("button", { name: "Show navigation" }).click();
  await expect.poll(() => workbenchWidth(page)).toBeLessThan(1_200);

  await page.setViewportSize({ width: 1024, height: 900 });
  await expect(
    page.getByRole("heading", { name: "Workbench", level: 2 }),
  ).toBeVisible();
  await expect(page).toHaveScreenshot("workspace-zarya-narrow.png", {
    animations: "disabled",
    fullPage: true,
  });

  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/sessions/session-1");
  await expect(page.getByRole("heading", { name: "Fix issue" })).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Delayed messages" }),
  ).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: "Delayed turn content" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Schedule" })).toBeDisabled();
  expect(await horizontalOverflow(page)).toBeLessThanOrEqual(1);
});

test("supports the shell and composer keyboard path", async ({ page }) => {
  await mockCoreApi(page);
  await page.goto("/dashboard");
  await expect(page.getByRole("heading", { name: "Dashboard" })).toBeVisible();
  await page.keyboard.press("Tab");
  await expect(
    page.getByRole("link", { name: "Skip to Main Content" }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("main")).toBeFocused();

  await page.goto("/sessions/session-1");
  const composer = page.getByRole("textbox", { name: "Next Agent Turn" });
  await expect(composer).toBeVisible();
  await composer.fill("Inspect the failed runtime event");
  await expect(page.getByText("Draft not sent")).toBeVisible();
});

async function mockCoreApi(page: import("@playwright/test").Page) {
  const state = {
    validationAttempts: 0,
    warningAcknowledged: false,
    diffRequests: 0,
    terminalOpen: false,
    jobsRequests: 0,
    createdJobRequest: null as Record<string, unknown> | null,
  };
  await installMockWebSocket(page);
  await mockPublicShellApi(page);
  await page.route("**/api/v1/inventory", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(inventory),
    });
  });
  await page.route("**/api/v1/node-enrollments", async (route) => {
    await route.fulfill({ contentType: "application/json", body: json([]) });
  });
  await page.route("**/api/v1/jobs", async (route) => {
    if (route.request().method() === "POST") {
      state.createdJobRequest = route.request().postDataJSON() as Record<
        string,
        unknown
      >;
      await route.fulfill({
        contentType: "application/json",
        body: json(createdJobDetail),
      });
      return;
    }
    state.jobsRequests += 1;
    await route.fulfill({
      contentType: "application/json",
      body: json([jobSummary, otherWorkspaceJob]),
    });
  });
  await page.route("**/api/v1/jobs/job-1", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(jobDetail),
    });
  });
  await page.route("**/api/v1/jobs/job-created", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(createdJobDetail),
    });
  });
  await page.route("**/api/v1/job-runs/run-1", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(jobRun),
    });
  });
  await page.route("**/api/v1/provider-quota/codex", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json({
        provider: "codex",
        state: "available",
        five_hour_remaining_percent: 80,
        weekly_remaining_percent: 75,
        observed_at: "2026-06-17T00:00:00Z",
        unavailable_reason: null,
      }),
    });
  });
  await page.route("**/api/v1/placements/placement-1", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: json(placement),
    });
  });
  await page.route(
    "**/api/v1/placements/placement-1/workspace/tree**",
    async (route) => {
      await route.fulfill({
        contentType: "application/json",
        body: json(workspaceTree),
      });
    },
  );
  await page.route(
    "**/api/v1/placements/placement-1/workspace/diff",
    async (route) => {
      state.diffRequests += 1;
      await route.fulfill({
        contentType: "application/json",
        body: json(workspaceDiff),
      });
    },
  );
  await page.route(
    "**/api/v1/placements/placement-1/workspace/file**",
    async (route) => {
      await route.fulfill({
        contentType: "application/json",
        body: json(workspaceFile),
      });
    },
  );
  await page.route(
    "**/api/v1/placements/placement-1/workspace/commands?**",
    async (route) => {
      await route.fulfill({
        contentType: "application/json",
        body: json({
          placement_id: "placement-1",
          commands: [],
          generated_at: "2026-06-17T00:00:00Z",
        }),
      });
    },
  );
  await page.route(
    "**/api/v1/placements/placement-1/workspace/terminals",
    async (route) => {
      if (route.request().method() === "POST") {
        state.terminalOpen = true;
        await route.fulfill({
          contentType: "application/json",
          body: json({
            placement_id: "placement-1",
            terminal: workspaceTerminal,
            replay: [],
          }),
        });
        return;
      }
      await route.fulfill({
        contentType: "application/json",
        body: json({
          placement_id: "placement-1",
          terminals: state.terminalOpen ? [workspaceTerminal] : [],
          generated_at: "2026-06-17T00:00:00Z",
        }),
      });
    },
  );
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
    "**/api/v1/sessions/session-1/evidence-projection",
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

async function installMockWebSocket(page: import("@playwright/test").Page) {
  await page.addInitScript(() => {
    const target = window as Window & { __upravaSocketFrames?: string[] };
    target.__upravaSocketFrames = [];
    class MockWebSocket extends EventTarget {
      static readonly CONNECTING = 0;
      static readonly OPEN = 1;
      static readonly CLOSING = 2;
      static readonly CLOSED = 3;
      readonly url: string;
      readyState = MockWebSocket.CONNECTING;

      constructor(url: string | URL) {
        super();
        this.url = String(url);
        queueMicrotask(() => {
          this.readyState = MockWebSocket.OPEN;
          this.dispatchEvent(new Event("open"));
          this.dispatchEvent(
            new MessageEvent("message", {
              data: JSON.stringify({
                kind: "output",
                terminal_id: "terminal-1",
                seq: 1,
                data: "uprava@local:/workspace/uprava$ ",
                sent_at: "2026-06-17T00:00:00Z",
              }),
            }),
          );
        });
      }

      send(data: string | ArrayBufferLike | Blob | ArrayBufferView) {
        if (typeof data === "string") target.__upravaSocketFrames?.push(data);
      }

      close() {
        this.readyState = MockWebSocket.CLOSED;
        this.dispatchEvent(new Event("close"));
      }
    }
    Object.defineProperty(window, "WebSocket", {
      configurable: true,
      value: MockWebSocket,
    });
  });
}

async function countResizeFrames(page: import("@playwright/test").Page) {
  return page.evaluate(() => {
    const frames = (window as Window & { __upravaSocketFrames?: string[] })
      .__upravaSocketFrames;
    return (frames ?? []).filter((frame) => {
      try {
        return JSON.parse(frame).kind === "resize";
      } catch {
        return false;
      }
    }).length;
  });
}

async function horizontalOverflow(page: import("@playwright/test").Page) {
  return page.evaluate(
    () =>
      document.documentElement.scrollWidth -
      document.documentElement.clientWidth,
  );
}

async function workbenchWidth(page: import("@playwright/test").Page) {
  return page
    .locator(".uprava-workbench-grid")
    .evaluate((element) => element.getBoundingClientRect().width);
}

async function mockPublicShellApi(page: import("@playwright/test").Page) {
  await page.route("**/api/v1/auth/status", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({
        auth_required: false,
        setup_required: false,
        authenticated: true,
        profile: "controlled_dev",
        security: {
          mode: "controlled_dev",
          web_auth_required: false,
          web_auth_configured: false,
          cookie_secure: false,
        },
      }),
    });
  });
  await page.route("**/api/v1/health", async (route) => {
    await route.fulfill({
      contentType: "application/json",
      body: JSON.stringify({ status: "ok", profile: "controlled_dev" }),
    });
  });
}

function json(value: unknown) {
  return JSON.stringify(value);
}

function isMonacoResource(url: string) {
  return url.includes("MonacoViews") || url.includes("monaco-editor");
}

function isXtermResource(url: string) {
  return url.includes("XtermTerminal") || url.includes("@xterm");
}

const runtime = {
  runtime_session_id: "runtime-1",
  provider: "codex",
  state: "blocked",
  resume_supported: true,
  degraded_reason: null,
  last_runtime_step_at: "2026-06-17T00:00:00Z",
};

const placement = {
  project_placement_id: "placement-1",
  project_id: "project-1",
  node_id: "node-1",
  display_name: "Uprava",
  workspace_path: "/workspace/uprava",
  state: "validated",
  resource_badges: [
    { kind: "dirty_workspace", severity: "warning", label: "Dirty workspace" },
  ],
  last_validated_at: "2026-06-17T00:00:00Z",
};

const readmeEntry = {
  name: "README.md",
  path: "README.md",
  kind: "file",
  status: "readable",
  byte_len: 16,
  modified_at: "2026-06-17T00:00:00Z",
  children: [],
};

const workspaceTree = {
  placement_id: "placement-1",
  root: {
    name: "uprava",
    path: ".",
    kind: "directory",
    status: "directory",
    byte_len: null,
    modified_at: "2026-06-17T00:00:00Z",
    children: [readmeEntry],
  },
  generated_at: "2026-06-17T00:00:00Z",
};

const workspaceFile = {
  placement_id: "placement-1",
  path: "README.md",
  metadata: readmeEntry,
  content: "# Uprava\n",
  truncated: false,
  generated_at: "2026-06-17T00:00:00Z",
};

const workspaceDiff = {
  placement_id: "placement-1",
  diff_id: "diff-1",
  summary: "README.md | 1 +",
  diff: "diff --git a/README.md b/README.md\n+# Uprava",
  summary_truncated: false,
  diff_truncated: false,
  generated_at: "2026-06-17T00:00:00Z",
};

const workspaceTerminal = {
  placement_id: "placement-1",
  terminal_id: "terminal-1",
  title: "Terminal 1",
  cwd: "/workspace/uprava",
  shell: "/bin/zsh",
  cols: 120,
  rows: 24,
  state: "running",
  exit_code: null,
  created_at: "2026-06-17T00:00:00Z",
  updated_at: "2026-06-17T00:00:00Z",
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
      capabilities: [
        {
          key: "provider.codex",
          value: {
            kind: "provider",
            available: true,
            configured: true,
            mode: "local",
            timeout_seconds: 120,
            unavailable_reason: null,
          },
        },
      ],
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
  payload: { type: "provider_message_completed", content: "Assistant reply" },
};

const approvalEvent = {
  ...messageEvent,
  event_id: "event-approval",
  seq: 2,
  kind: "approval.requested",
  payload: {
    type: "approval_requested",
    approval_id: "approval-1",
    prompt: "Allow command?",
    provider: "codex",
    provider_event_type: null,
    source: null,
  },
};

const runtimeErrorEvent = {
  ...messageEvent,
  event_id: "event-runtime-error",
  seq: 3,
  kind: "runtime.error",
  payload: {
    type: "runtime_error",
    code: "provider.failed",
    message: "Provider failed safely",
  },
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
  available_commands: [
    "session.sendTurn",
    "approval.resolve",
    "warning.acknowledge",
  ],
  visible_refs: [{ kind: "session", session_thread_id: "session-1" }],
  source_cause_summary: "Known event source refs are preserved",
  resume_context: "Runtime blocked on approval",
  generated_at: "2026-06-17T00:00:00Z",
};

const jobRun = {
  job_run_id: "run-1",
  job_id: "job-1",
  trigger: "manual",
  state: "succeeded",
  scheduled_for: null,
  queued_at: "2026-06-17T00:00:00Z",
  started_at: "2026-06-17T00:00:01Z",
  finished_at: "2026-06-17T00:00:02Z",
  session_thread_id: "session-1",
  runtime_session_id: "runtime-1",
  summary: "Completed",
  terminal_reason: null,
  config_snapshot: { provider: "codex", prompt: "Run checks" },
  force: false,
};

const jobSummary = {
  job_id: "job-1",
  name: "Nightly check",
  project_placement_id: "placement-1",
  placement_name: "Uprava",
  provider: "codex",
  enabled: true,
  schedule: { kind: "daily", hour: 2, minute: 0 },
  timezone: "UTC",
  overlap_policy: "skip",
  continue_after_error: false,
  next_run_at: "2026-06-18T02:00:00Z",
  paused_reason: null,
  latest_run: jobRun,
  created_at: "2026-06-16T00:00:00Z",
  updated_at: "2026-06-17T00:00:02Z",
};

const jobDetail = {
  job: jobSummary,
  prompt: "Run checks",
  runs: [jobRun],
};

const otherWorkspaceJob = {
  ...jobSummary,
  job_id: "job-other",
  name: "Other workspace Job",
  project_placement_id: "placement-2",
  placement_name: "Other workspace",
  latest_run: null,
};

const createdJobDetail = {
  job: {
    ...jobSummary,
    job_id: "job-created",
    name: "Created check",
    enabled: false,
    latest_run: null,
    next_run_at: null,
  },
  prompt: "Inspect workspace",
  runs: [],
};
