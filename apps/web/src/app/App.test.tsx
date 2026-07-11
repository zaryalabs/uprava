import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it, vi } from "vitest";

import { App } from "./App";

describe("App routes", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders nodes, placement, and session routes from mocked Core snapshots", async () => {
    renderApp("/");

    expect(
      await screen.findByRole("heading", { name: "Dashboard" }),
    ).toBeVisible();
    expect(await screen.findByText("Core API")).toBeVisible();
    expect(screen.getByRole("link", { name: "Nodes" })).toBeVisible();

    renderApp("/nodes");

    expect(await screen.findByRole("heading", { name: "Nodes" })).toBeVisible();
    expect((await screen.findAllByText("Local Node")).length).toBeGreaterThan(
      0,
    );
    expect(screen.getByText("Pair Node")).toBeVisible();
    expect(screen.getByText("not production-secure")).toBeVisible();

    renderApp("/workspaces/placement-1");

    expect(
      await screen.findByRole(
        "heading",
        { name: "Uprava" },
        { timeout: 5_000 },
      ),
    ).toBeVisible();
    expect(screen.getAllByText("Dirty workspace").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Start Codex" })).toBeEnabled();

    renderApp("/workspaces/placement-1");

    expect(
      await screen.findByRole("heading", { name: "Uprava" }),
    ).toBeVisible();
    expect(await screen.findByText("Workspace Inspector")).toBeVisible();
    expect((await screen.findAllByText("README.md")).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("treeitem", { name: "README.md" }));
    expect(
      await screen.findByRole(
        "region",
        { name: "File editor README.md" },
        { timeout: 5_000 },
      ),
    ).toBeVisible();

    renderApp("/projects/project-1");

    expect(
      await screen.findByRole("heading", { name: "Uprava" }),
    ).toBeVisible();
    expect(screen.getByText("/workspace/uprava")).toBeVisible();

    renderApp("/sessions/session-1");

    expect(
      await screen.findByRole("heading", { name: "Fix issue" }),
    ).toBeVisible();
    expect(screen.getAllByText("Assistant reply").length).toBeGreaterThan(0);
    expect(screen.getByRole("link", { name: "Workspace" })).toHaveAttribute(
      "href",
      "/workspaces/placement-1",
    );
    expect(
      (await screen.findAllByText("Session evidence projection"))[0],
    ).toBeVisible();
    expect(await screen.findByText("session.sendTurn")).toBeVisible();
    expect(MockEventSource.created).toBe(1);
    MockEventSource.latest?.emit("uprava.event", {
      ...messageEvent,
      event_id: "event-streamed",
      seq: 2,
      session_projection_seq: 2,
      payload: {
        type: "provider_message_completed",
        content: "Streamed reply",
      },
    });
    expect(await screen.findByText("Streamed reply")).toBeVisible();
    await waitFor(() => expect(MockEventSource.created).toBe(1));

    renderApp("/settings/runtime");

    expect(
      await screen.findByRole("heading", { name: "Runtime Settings" }),
    ).toBeVisible();
    expect(await screen.findByText("uprava-core 0.1.8")).toBeVisible();
    expect(screen.getByText("v2")).toBeVisible();
    expect(screen.getByText("1")).toBeVisible();
  }, 15_000);
});

function renderApp(path: string) {
  cleanup();
  vi.unstubAllGlobals();
  vi.stubGlobal("fetch", vi.fn(mockFetch));
  vi.stubGlobal("EventSource", MockEventSource);
  MockEventSource.reset();

  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[path]}>
        <App />
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

class MockEventSource {
  static created = 0;
  static latest: MockEventSource | null = null;
  onerror: (() => void) | null = null;
  private listeners = new Map<string, (event: MessageEvent) => void>();

  constructor(readonly url: string) {
    MockEventSource.created += 1;
    MockEventSource.latest = this;
  }

  addEventListener(type: string, listener: (event: MessageEvent) => void) {
    this.listeners.set(type, listener);
  }

  emit(type: string, payload: unknown) {
    this.listeners.get(type)?.({
      data: JSON.stringify(payload),
    } as MessageEvent);
  }

  close() {}

  static reset() {
    MockEventSource.created = 0;
    MockEventSource.latest = null;
  }
}

async function mockFetch(input: RequestInfo | URL) {
  const url = new URL(input.toString());
  const payload = responseForPath(url.pathname);
  return new Response(JSON.stringify(payload), {
    status: 200,
    headers: { "content-type": "application/json" },
  });
}

function responseForPath(pathname: string) {
  switch (pathname) {
    case "/api/v1/auth/status":
      return {
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
      };
    case "/api/v1/health":
      return { status: "ok", profile: "controlled_dev" };
    case "/api/v1/version":
      return {
        name: "uprava-core",
        version: "0.1.8",
        api_version: "v2",
        schema_version: 1,
        profile: "controlled_dev",
      };
    case "/api/v1/inventory":
      return inventory;
    case "/api/v1/node-enrollments":
      return [];
    case "/api/v1/placements/placement-1":
      return placement;
    case "/api/v1/placements/placement-1/workspace/tree":
      return workspaceTree;
    case "/api/v1/placements/placement-1/workspace/file":
      return workspaceFile;
    case "/api/v1/placements/placement-1/workspace/terminals":
      return workspaceTerminals;
    case "/api/v1/sessions/session-1":
      return sessionDetail;
    case "/api/v1/sessions/session-1/evidence-projection":
      return evidenceProjection;
    case "/api/v1/sessions/session-1/agent-projection":
      return agentProjection;
    default:
      throw new Error(`Unhandled mocked Core path: ${pathname}`);
  }
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
      presence: "reachable",
      sleep_hint: "unknown",
      heartbeat_age_seconds: 4,
      active_runtime_count: 1,
      capabilities: [
        {
          key: "provider.codex",
          value: {
            kind: "provider",
            available: true,
            configured: true,
            mode: "exec",
            timeout_seconds: 120,
            unavailable_reason: null,
          },
        },
      ],
      diagnostics: "ok",
    },
  ],
  placements: [placement],
  sessions: [session],
  generated_at: "2026-06-17T00:00:00Z",
};

const workspaceTree = {
  placement_id: "placement-1",
  truncated: false,
  total_entries: 1,
  generated_at: "2026-06-17T00:00:00Z",
  root: {
    name: ".",
    path: ".",
    kind: "directory",
    status: "directory",
    classification: "normal",
    expandable: true,
    byte_len: null,
    modified_at: null,
    children: [
      {
        name: "README.md",
        path: "README.md",
        kind: "file",
        status: "readable",
        classification: "normal",
        expandable: false,
        byte_len: 12,
        modified_at: "2026-06-17T00:00:00Z",
        children: [],
      },
    ],
  },
};

const workspaceFile = {
  placement_id: "placement-1",
  path: "README.md",
  metadata: {
    name: "README.md",
    path: "README.md",
    kind: "file",
    status: "readable",
    classification: "normal",
    expandable: false,
    byte_len: 12,
    modified_at: "2026-06-17T00:00:00Z",
    children: [],
  },
  content: "# Uprava",
  truncated: false,
  generated_at: "2026-06-17T00:00:00Z",
};

const workspaceTerminals = {
  placement_id: "placement-1",
  terminals: [],
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
  payload: {
    type: "provider_message_completed",
    content: "Assistant reply",
  },
};

const sessionDetail = {
  session,
  placement,
  messages: [
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
  events: [messageEvent],
};

const evidenceProjection = {
  session_thread_id: "session-1",
  generated_at: "2026-06-17T00:00:00Z",
  root: {
    evidence_id: "session:session-1",
    label: "Session evidence projection",
    primary_ref: { kind: "session", session_thread_id: "session-1" },
    source_refs: [],
    evidence_refs: [],
    cause_refs: [],
    children: [],
  },
};

const agentProjection = {
  session_thread_id: "session-1",
  project_placement: placement,
  runtime_summary: runtime,
  current_turn: "turn-1",
  pending_approvals: [],
  active_warnings: [],
  recent_turn_summaries: ["turn-1 running"],
  recent_message_refs: [{ kind: "message", message_id: "message-assistant" }],
  evidence_projection_summary: "Session evidence projection",
  available_block_types: ["core.assistant-message"],
  available_commands: ["session.sendTurn"],
  visible_refs: [{ kind: "session", session_thread_id: "session-1" }],
  source_cause_summary: "Known event source refs are preserved",
  resume_context: "Runtime blocked on approval",
  generated_at: "2026-06-17T00:00:00Z",
};
