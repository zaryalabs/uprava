import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type {
  IntegrationConnectionSummary,
  McpDependencyStatus,
  NodeSummary,
  ToolCallSummary,
} from "../../shared/protocol/types";
import {
  availabilityReasonLabel,
  IntegrationCard,
  integrationDiagnostic,
  toolCallTraceRoute,
} from "./ToolingRoute";

describe("integration management", () => {
  it("explains missing ToolHive without exposing credential detail", () => {
    const detail = integrationDiagnostic(
      connection(),
      dependency({ actual_state: "toolhive_missing" }),
      node(),
    );

    expect(detail).toContain("ToolHive is not installed");
    expect(detail).not.toMatch(/token|secret|credential_ref/u);
  });

  it("requires explicit confirmation before disconnect", () => {
    const onRequestDisconnect = vi.fn();
    const onDisconnect = vi.fn();
    render(
      <IntegrationCard
        connection={connection()}
        dependency={dependency()}
        node={node()}
        pendingDisconnect={false}
        disconnecting={false}
        connecting={false}
        authorization={null}
        error={null}
        onConnect={vi.fn()}
        onRequestDisconnect={onRequestDisconnect}
        onCancelDisconnect={vi.fn()}
        onDisconnect={onDisconnect}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));

    expect(onRequestDisconnect).toHaveBeenCalledOnce();
    expect(onDisconnect).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Reconnect" })).toBeEnabled();
  });

  it("shows immediate availability closure in the confirmation state", () => {
    render(
      <IntegrationCard
        connection={connection()}
        dependency={dependency()}
        node={node()}
        pendingDisconnect
        disconnecting={false}
        connecting={false}
        authorization={null}
        error={null}
        onConnect={vi.fn()}
        onRequestDisconnect={vi.fn()}
        onCancelDisconnect={vi.fn()}
        onDisconnect={vi.fn()}
      />,
    );

    expect(
      screen.getByText(/immediately disables effective availability/u),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Confirm disconnect" }),
    ).toBeEnabled();
  });

  it("starts reconnect and exposes only the one-time authorization action", () => {
    const onConnect = vi.fn();
    render(
      <IntegrationCard
        connection={connection()}
        dependency={dependency()}
        node={node()}
        pendingDisconnect={false}
        disconnecting={false}
        connecting={false}
        authorization={{
          integrationId: "integration-linear-1",
          url: "https://linear.app/oauth/authorize?state=opaque",
          expiresAt: "2026-07-19T10:05:00Z",
        }}
        error={null}
        onConnect={onConnect}
        onRequestDisconnect={vi.fn()}
        onCancelDisconnect={vi.fn()}
        onDisconnect={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Reconnect" }));

    expect(onConnect).toHaveBeenCalledOnce();
    expect(
      screen.getByRole("link", { name: "Continue authorization in Linear" }),
    ).toHaveAttribute(
      "href",
      "https://linear.app/oauth/authorize?state=opaque",
    );
  });
});

describe("capability and trace presentation", () => {
  it("uses explicit policy and runtime availability explanations", () => {
    expect(availabilityReasonLabel("policy_blocked")).toBe(
      "Current policy blocks execution",
    );
    expect(availabilityReasonLabel("node_offline")).toBe("Node offline");
    expect(availabilityReasonLabel("toolhive_missing")).toBe(
      "ToolHive missing on Node",
    );
  });

  it("opens a scoped tool call in the canonical session trace", () => {
    expect(toolCallTraceRoute(toolCall())).toBe(
      "/workspaces/placement-1/agent/session-1?agentView=trace",
    );
  });
});

function connection(
  overrides: Partial<IntegrationConnectionSummary> = {},
): IntegrationConnectionSummary {
  return {
    integration_id: "integration-linear-1",
    source_id: "linear-remote-mcp",
    provider: "linear",
    display_name: "Linear",
    desired_state: "enabled",
    auth_state: "connected",
    node_id: "node-1",
    authenticated_actor_label: "workspace member",
    connected_at: "2026-07-19T10:00:00Z",
    updated_at: "2026-07-19T10:00:00Z",
    error_code: null,
    ...overrides,
  };
}

function dependency(
  overrides: Partial<McpDependencyStatus> = {},
): McpDependencyStatus {
  return {
    dependency_instance_id: "dependency-linear-1",
    integration_id: "integration-linear-1",
    node_id: "node-1",
    desired_state: "enabled",
    actual_state: "running",
    runtime_name: "toolhive",
    runtime_version: "0.40.0",
    upstream_identity: "linear-remote-mcp",
    schema_set_hash: "sha256:fixture",
    error_code: null,
    observed_at: "2026-07-19T10:00:00Z",
    ...overrides,
  };
}

function node(): NodeSummary {
  return {
    node_id: "node-1",
    display_name: "Local Node",
    presence: "reachable",
    sleep_hint: "awake",
    heartbeat_age_seconds: 1,
    active_runtime_count: 1,
    capabilities: [],
    diagnostics: "healthy",
  };
}

function toolCall(): ToolCallSummary {
  return {
    tool_call_id: "tool-call-1",
    tool_id: "uprava.session.inspect",
    schema_hash: "sha256:fixture",
    actor_ref: { kind: "provider", provider: "codex" },
    scope: {
      actor_ref: { kind: "provider", provider: "codex" },
      node_id: "node-1",
      project_id: "project-1",
      project_placement_id: "placement-1",
      session_thread_id: "session-1",
    },
    source_kind: "uprava_native",
    state: "completed",
    policy_decision: "allow",
    route: "core_native",
    requested_at: "2026-07-19T10:00:00Z",
    started_at: "2026-07-19T10:00:01Z",
    completed_at: "2026-07-19T10:00:02Z",
    correlation_id: "correlation-1",
  };
}
