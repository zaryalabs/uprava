import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

import type { InventorySnapshot } from "../../shared/protocol/types";
import { InventoryTreeContent } from "./InventoryTree";

describe("InventoryTreeContent", () => {
  it("renders node, placement, session, and runtime states in the tree", () => {
    render(
      <MemoryRouter>
        <InventoryTreeContent snapshot={snapshot} pathname="/nodes/node-1" />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("navigation", { name: "Inventory tree" }),
    ).toBeVisible();
    expect(screen.getByText("reachable")).toBeVisible();
    expect(screen.getByText("stale")).toBeVisible();
    expect(screen.getByText("offline")).toBeVisible();
    expect(screen.getAllByText("active").length).toBeGreaterThan(0);
    expect(screen.getAllByText("idle").length).toBeGreaterThan(0);
    expect(screen.getByText("sleep sleeping")).toBeVisible();
    expect(screen.getByText("validated")).toBeVisible();
    expect(screen.getByText("Missing workspace")).toBeVisible();
    expect(screen.getByText("degraded")).toBeVisible();
    expect(screen.getByText("error")).toBeVisible();
    expect(screen.getByRole("link", { name: /Uprava/ })).toHaveAttribute(
      "href",
      "/workspaces/placement-1",
    );
    expect(
      screen.getByRole("link", { name: /Active session/ }),
    ).toHaveAttribute("href", "/workspaces/placement-1/agent/session-1");
  });

  it("keeps stale inventory links visible when refresh fails", () => {
    render(
      <MemoryRouter>
        <InventoryTreeContent
          snapshot={snapshot}
          pathname="/dashboard"
          refreshError={new Error("network unavailable")}
        />
      </MemoryRouter>,
    );

    expect(screen.getByText("Inventory refresh failed")).toBeVisible();
    expect(screen.getByRole("link", { name: /Uprava/ })).toHaveAttribute(
      "href",
      "/workspaces/placement-1",
    );
  });
});

const snapshot: InventorySnapshot = {
  nodes: [
    {
      node_id: "node-1",
      display_name: "Local Node",
      presence: "reachable",
      sleep_hint: "unknown",
      heartbeat_age_seconds: 4,
      active_runtime_count: 1,
      capabilities: [],
      diagnostics: "ok",
    },
    {
      node_id: "node-2",
      display_name: "Stale Node",
      presence: "stale",
      sleep_hint: "sleeping",
      heartbeat_age_seconds: 120,
      active_runtime_count: 0,
      capabilities: [],
      diagnostics: "stale heartbeat",
    },
    {
      node_id: "node-3",
      display_name: "Offline Node",
      presence: "offline",
      sleep_hint: "unknown",
      heartbeat_age_seconds: null,
      active_runtime_count: 0,
      capabilities: [],
      diagnostics: "offline",
    },
  ],
  placements: [
    {
      project_placement_id: "placement-1",
      project_id: "project-1",
      node_id: "node-1",
      display_name: "Uprava",
      workspace_path: "/workspace/uprava",
      state: "validated",
      resource_badges: [],
      last_validated_at: "2026-06-17T00:00:00Z",
    },
    {
      project_placement_id: "placement-2",
      project_id: "project-2",
      node_id: "node-2",
      display_name: "Missing",
      workspace_path: "/missing",
      state: "missing",
      resource_badges: [
        {
          kind: "missing_workspace",
          severity: "hard_block",
          label: "Missing workspace",
        },
      ],
      last_validated_at: null,
    },
  ],
  sessions: [
    {
      session_thread_id: "session-1",
      project_placement_id: "placement-1",
      runtime_session_id: "runtime-1",
      title: "Active session",
      state: "active",
      runtime: {
        runtime_session_id: "runtime-1",
        provider: "codex",
        state: "ready",
        resume_supported: true,
        degraded_reason: null,
        last_runtime_step_at: null,
      },
      message_count: 0,
      updated_at: "2026-06-17T00:00:00Z",
    },
    {
      session_thread_id: "session-2",
      project_placement_id: "placement-2",
      runtime_session_id: "runtime-2",
      title: "Degraded session",
      state: "degraded",
      runtime: {
        runtime_session_id: "runtime-2",
        provider: "codex",
        state: "error",
        resume_supported: true,
        degraded_reason: "provider.failed",
        last_runtime_step_at: "2026-06-17T00:00:00Z",
      },
      message_count: 1,
      updated_at: "2026-06-17T00:00:00Z",
    },
  ],
  generated_at: "2026-06-17T00:00:00Z",
};
