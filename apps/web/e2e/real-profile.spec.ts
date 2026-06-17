import { expect, test, type APIRequestContext } from "@playwright/test";

const realApiEnabled = process.env.CORTEX_E2E_REAL_API === "1";
const coreUrl = process.env.CORTEX_E2E_CORE_URL ?? "http://127.0.0.1:8080";
const expectedNode = process.env.CORTEX_E2E_EXPECTED_NODE ?? "Compose Node";
const workspacePath = process.env.CORTEX_E2E_WORKSPACE_PATH ?? "/workspace";
const provider = process.env.CORTEX_E2E_PROVIDER ?? "fake";
const sessionTitle =
  process.env.CORTEX_E2E_SESSION_TITLE ?? "Playwright real profile";
const turnContent =
  process.env.CORTEX_E2E_TURN_CONTENT ?? "playwright real profile";
const expectedAssistantContent =
  process.env.CORTEX_E2E_EXPECTED_ASSISTANT_CONTENT ??
  `Fake provider accepted: ${turnContent}`;
const turnTimeoutMs = Number(process.env.CORTEX_E2E_TURN_TIMEOUT_MS ?? "30000");
const lifecycleEnabled =
  process.env.CORTEX_E2E_LIFECYCLE === "1" ||
  (process.env.CORTEX_E2E_LIFECYCLE !== "0" && provider === "fake");
const testTimeoutMs = Number(
  process.env.CORTEX_E2E_TEST_TIMEOUT_MS ??
    String(
      Math.max(30_000, turnTimeoutMs + (lifecycleEnabled ? 45_000 : 15_000)),
    ),
);

test.describe("real local profile", () => {
  test.skip(
    !realApiEnabled,
    "set CORTEX_E2E_REAL_API=1 to run against a live Core/Web/Node profile",
  );

  test("creates a provider session through Core and renders it in Web", async ({
    page,
    request,
  }) => {
    test.setTimeout(testTimeoutMs);
    const placementId = await waitForPlacementId(request);
    const session = await createProviderSession(request, placementId);
    const sessionId = isRecord(session.session)
      ? stringField(session.session, "session_thread_id")
      : "";
    expect(sessionId).not.toBe("");
    const sessionUrl = `${coreUrl}/api/v1/sessions/${encodeURIComponent(sessionId)}`;

    await waitForRuntimeState(request, sessionUrl, "ready");
    await postJson(request, `${sessionUrl}/turns`, { content: turnContent });
    await waitForAssistantContent(
      request,
      sessionUrl,
      expectedAssistantContent,
    );
    await waitForRuntimeState(request, sessionUrl, "ready");

    await page.goto(`/sessions/${sessionId}`);
    await expect(
      page.getByRole("heading", { name: sessionTitle }),
    ).toBeVisible();
    const main = page.getByRole("main");
    await expect(
      main.getByText(expectedAssistantContent, { exact: true }),
    ).toBeVisible();
    await expect(main.getByText("Session-local index")).toBeVisible();
    await expect(main.getByText(provider, { exact: true })).toBeVisible();
    await expect(main.getByText("ready", { exact: true })).toBeVisible();

    if (lifecycleEnabled) {
      await page.getByRole("button", { name: "Detach" }).click();
      await waitForSessionState(request, sessionUrl, "detached");
      await expect(page.getByRole("button", { name: "Attach" })).toBeEnabled();
      await expect(page.getByRole("button", { name: "Send" })).toBeDisabled();

      await page.getByRole("button", { name: "Attach" }).click();
      await waitForSessionState(request, sessionUrl, "active");
      await expect(page.getByRole("button", { name: "Detach" })).toBeEnabled();

      await page.getByRole("button", { name: "Stop" }).click();
      await waitForRuntimeState(request, sessionUrl, "stopped");
      await waitForSessionState(request, sessionUrl, "stopped");
      await expect(page.getByRole("button", { name: "Resume" })).toBeEnabled();

      await page.getByRole("button", { name: "Resume" }).click();
      await waitForRuntimeState(request, sessionUrl, "ready");
      await waitForSessionState(request, sessionUrl, "active");
      await expect(page.getByRole("button", { name: "Detach" })).toBeEnabled();

      const postResumeTurn = `${turnContent} after browser resume`;
      const postResumeAssistant = `Fake provider accepted: ${postResumeTurn}`;
      await page.getByPlaceholder("Send a turn").fill(postResumeTurn);
      await page.getByRole("button", { name: "Send" }).click();
      await waitForAssistantContent(request, sessionUrl, postResumeAssistant);
      await waitForRuntimeState(request, sessionUrl, "ready");
      await expect(
        main.getByText(postResumeAssistant, { exact: true }),
      ).toBeVisible();

      await page.reload();
      await expect(
        page.getByRole("heading", { name: sessionTitle }),
      ).toBeVisible();
      await expect(
        page.getByRole("main").getByText(postResumeAssistant, { exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("main").getByText("Session-local index"),
      ).toBeVisible();
    }
  });
});

async function waitForPlacementId(request: APIRequestContext) {
  let lastInventory: unknown = null;
  await expect
    .poll(
      async () => {
        const response = await request.get(`${coreUrl}/api/v1/inventory`);
        if (!response.ok()) return "";
        lastInventory = await response.json();
        return placementIdFromInventory(lastInventory);
      },
      {
        timeout: 60_000,
        intervals: [500, 1_000, 2_000],
        message: `waiting for ${expectedNode} validated placement at ${workspacePath}`,
      },
    )
    .not.toBe("");

  return placementIdFromInventory(lastInventory);
}

function placementIdFromInventory(value: unknown) {
  if (
    !isRecord(value) ||
    !Array.isArray(value.nodes) ||
    !Array.isArray(value.placements)
  ) {
    return "";
  }
  const node = value.nodes.find(
    (candidate) =>
      isRecord(candidate) &&
      stringField(candidate, "display_name") === expectedNode,
  );
  if (!isRecord(node)) return "";
  const nodeId = stringField(node, "node_id");
  const placement = value.placements.find(
    (candidate) =>
      isRecord(candidate) &&
      stringField(candidate, "node_id") === nodeId &&
      stringField(candidate, "workspace_path") === workspacePath &&
      stringField(candidate, "state") === "validated",
  );
  return isRecord(placement)
    ? stringField(placement, "project_placement_id")
    : "";
}

async function createProviderSession(
  request: APIRequestContext,
  placementId: string,
) {
  return postJson(request, `${coreUrl}/api/v1/sessions`, {
    project_placement_id: placementId,
    title: sessionTitle,
    provider,
  });
}

async function waitForRuntimeState(
  request: APIRequestContext,
  sessionUrl: string,
  expectedState: string,
) {
  await expect
    .poll(
      async () => {
        const detail = await getJson(request, sessionUrl);
        return runtimeState(detail);
      },
      { timeout: 30_000, intervals: [500, 1_000, 2_000] },
    )
    .toBe(expectedState);
}

async function waitForSessionState(
  request: APIRequestContext,
  sessionUrl: string,
  expectedState: string,
) {
  await expect
    .poll(
      async () => {
        const detail = await getJson(request, sessionUrl);
        return sessionState(detail);
      },
      { timeout: 30_000, intervals: [500, 1_000, 2_000] },
    )
    .toBe(expectedState);
}

async function waitForAssistantContent(
  request: APIRequestContext,
  sessionUrl: string,
  expectedContent: string,
) {
  await expect
    .poll(
      async () => {
        const detail = await getJson(request, sessionUrl);
        return hasAssistantContent(detail, expectedContent);
      },
      { timeout: turnTimeoutMs, intervals: [500, 1_000, 2_000] },
    )
    .toBe(true);
}

async function getJson(request: APIRequestContext, url: string) {
  const response = await request.get(url);
  expect(response.ok()).toBe(true);
  return response.json() as Promise<unknown>;
}

async function postJson(
  request: APIRequestContext,
  url: string,
  body: unknown,
) {
  const response = await request.post(url, { data: body });
  expect(response.ok()).toBe(true);
  return response.json() as Promise<Record<string, unknown>>;
}

function runtimeState(value: unknown) {
  if (
    !isRecord(value) ||
    !isRecord(value.session) ||
    !isRecord(value.session.runtime)
  ) {
    return "";
  }
  return stringField(value.session.runtime, "state");
}

function sessionState(value: unknown) {
  if (!isRecord(value) || !isRecord(value.session)) {
    return "";
  }
  return stringField(value.session, "state");
}

function hasAssistantContent(value: unknown, expectedContent: string) {
  if (!isRecord(value) || !Array.isArray(value.messages)) return false;
  return value.messages.some(
    (message) =>
      isRecord(message) &&
      stringField(message, "role") === "assistant" &&
      stringField(message, "content").includes(expectedContent),
  );
}

function stringField(value: Record<string, unknown>, field: string) {
  const fieldValue = value[field];
  return typeof fieldValue === "string" ? fieldValue : "";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
