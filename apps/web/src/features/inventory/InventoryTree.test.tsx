import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, describe, expect, it } from "vitest";

import type { InventorySnapshot } from "../../shared/protocol/types";
import { rememberWorkspaceRoute } from "../workspaces/routes";
import { InventoryTreeContent } from "./InventoryTree";

describe("InventoryTreeContent", () => {
  afterEach(() => window.localStorage.clear());

  it("separates node disclosure from navigation and only renders workspaces", () => {
    render(
      <MemoryRouter>
        <InventoryTreeContent snapshot={snapshot} pathname="/nodes/node-1" />
      </MemoryRouter>,
    );

    expect(
      screen.getByRole("navigation", { name: "Inventory tree" }),
    ).toBeVisible();
    const tree = screen.getByRole("navigation", { name: "Inventory tree" });
    expect(
      within(tree).getByRole("link", { name: "Add Node" }),
    ).toHaveAttribute("href", "/nodes/pair");
    expect(
      within(tree).getByRole("img", { name: "Presence: reachable" }),
    ).toBeVisible();
    expect(
      within(tree).getByRole("img", { name: "Presence: stale" }),
    ).toBeVisible();
    expect(
      within(tree).getByRole("img", { name: "Presence: offline" }),
    ).toBeVisible();
    expect(within(tree).queryByText("Active session")).not.toBeInTheDocument();
    expect(
      within(tree).queryByText("Degraded session"),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: /Uprava/ })).toHaveAttribute(
      "href",
      "/workspaces/placement-1/agent",
    );
    const disclosure = within(tree).getByRole("button", {
      name: "Collapse Local Node workspaces",
    });
    expect(disclosure).toHaveAttribute("aria-expanded", "true");
    expect(
      within(tree).getByRole("link", { name: /Local Node/ }),
    ).toHaveAttribute("href", "/nodes/node-1");

    fireEvent.click(disclosure);
    expect(
      within(tree).getByRole("button", {
        name: "Expand Local Node workspaces",
      }),
    ).toHaveAttribute("aria-expanded", "false");
    expect(
      within(tree).queryByRole("link", { name: /Uprava/ }),
    ).not.toBeInTheDocument();
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
      "/workspaces/placement-1/agent",
    );
  });

  it("opens a workspace on its preferred surface", () => {
    rememberWorkspaceRoute("placement-1", "node-1", "workbench");

    render(
      <MemoryRouter>
        <InventoryTreeContent snapshot={snapshot} pathname="/dashboard" />
      </MemoryRouter>,
    );

    expect(screen.getByRole("link", { name: /Uprava/ })).toHaveAttribute(
      "href",
      "/workspaces/placement-1/workbench",
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
