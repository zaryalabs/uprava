import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { cleanup, render, screen } from "@testing-library/react";
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

    renderApp("/placements/placement-1");

    expect(
      await screen.findByRole("heading", { name: "Cortex" }),
    ).toBeVisible();
    expect(screen.getAllByText("Dirty workspace").length).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: /Start/i })).toBeEnabled();

    renderApp("/sessions/session-1");

    expect(
      await screen.findByRole("heading", { name: "Fix issue" }),
    ).toBeVisible();
    expect(screen.getAllByText("Assistant reply").length).toBeGreaterThan(0);
    expect(await screen.findByText("Session-local index")).toBeVisible();
    expect(await screen.findByText("session.sendTurn")).toBeVisible();

    renderApp("/settings/runtime");

    expect(
      await screen.findByRole("heading", { name: "Runtime Settings" }),
    ).toBeVisible();
    expect(await screen.findByText("cortex-core 0.1.0")).toBeVisible();
    expect(screen.getByText("v1")).toBeVisible();
    expect(screen.getByText("1")).toBeVisible();
  });
});

function renderApp(path: string) {
  cleanup();
  vi.unstubAllGlobals();
  vi.stubGlobal("fetch", vi.fn(mockFetch));
  vi.stubGlobal("EventSource", MockEventSource);

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
  onerror: (() => void) | null = null;

  constructor(readonly url: string) {}

  addEventListener() {}

  close() {}
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
        name: "cortex-core",
        version: "0.1.0",
        api_version: "v1",
        schema_version: 1,
        profile: "controlled_dev",
      };
    case "/api/v1/inventory":
      return inventory;
    case "/api/v1/node-enrollments":
      return [];
    case "/api/v1/placements/placement-1":
      return placement;
    case "/api/v1/sessions/session-1":
      return sessionDetail;
    case "/api/v1/sessions/session-1/artifact-tree":
      return artifactTree;
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
      presence: "reachable",
      sleep_hint: "unknown",
      heartbeat_age_seconds: 4,
      active_runtime_count: 1,
      capabilities: [{ key: "provider.codex", value: { configured: true } }],
      diagnostics: "ok",
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
  artifact_tree_summary: "Session-local index",
  available_block_types: ["core.assistant-message"],
  available_commands: ["session.sendTurn"],
  visible_refs: [{ kind: "session", session_thread_id: "session-1" }],
  source_cause_summary: "Known event source refs are preserved",
  resume_context: "Runtime blocked on approval",
  generated_at: "2026-06-17T00:00:00Z",
};
