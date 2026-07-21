import { beforeEach, describe, expect, it } from "vitest";

import {
  lastNodeId,
  lastWorkspaceId,
  preferredWorkspaceRoute,
  rememberNodeRoute,
  rememberWorkspaceRoute,
  routeWithSearch,
  workspaceAgentRoute,
  workspaceAgentSessionRoute,
  workspaceJobNewRoute,
  workspaceJobRoute,
  workspaceJobRunRoute,
  workspaceJobsRoute,
  workspaceRoute,
  workspaceSurfaceFromPathname,
  workspaceTaskRoute,
  workspaceTasksRoute,
  workspaceWorkbenchRoute,
} from "./routes";

describe("workspace routes", () => {
  beforeEach(() => window.localStorage.clear());

  it("builds canonical workspace-aware routes", () => {
    expect(workspaceRoute("placement 1")).toBe("/workspaces/placement%201");
    expect(workspaceAgentRoute("placement 1")).toBe(
      "/workspaces/placement%201/agent",
    );
    expect(workspaceAgentSessionRoute("placement 1", "session/1")).toBe(
      "/workspaces/placement%201/agent/session%2F1",
    );
    expect(workspaceWorkbenchRoute("placement 1")).toBe(
      "/workspaces/placement%201/workbench",
    );
    expect(workspaceTasksRoute("placement 1")).toBe(
      "/workspaces/placement%201/tasks",
    );
    expect(workspaceTaskRoute("placement 1", "task/1")).toBe(
      "/workspaces/placement%201/tasks/task%2F1",
    );
    expect(workspaceJobsRoute("placement 1")).toBe(
      "/workspaces/placement%201/jobs",
    );
    expect(workspaceJobNewRoute("placement 1")).toBe(
      "/workspaces/placement%201/jobs/new",
    );
    expect(workspaceJobRoute("placement 1", "job/1")).toBe(
      "/workspaces/placement%201/jobs/job%2F1",
    );
    expect(workspaceJobRunRoute("placement 1", "job/1", "run/1")).toBe(
      "/workspaces/placement%201/jobs/job%2F1/runs/run%2F1",
    );
  });

  it("recognizes surfaces and preserves query parameters", () => {
    expect(
      workspaceSurfaceFromPathname(
        "placement-1",
        "/workspaces/placement-1/agent/session-1",
      ),
    ).toBe("agent");
    expect(
      workspaceSurfaceFromPathname(
        "placement-1",
        "/workspaces/placement-1/tasks/task-1",
      ),
    ).toBe("tasks");
    expect(
      workspaceSurfaceFromPathname(
        "placement-1",
        "/workspaces/placement-1/unknown",
      ),
    ).toBeNull();
    expect(
      routeWithSearch("/workspaces/placement-1/jobs", "?inspect=ref&tab=runs"),
    ).toBe("/workspaces/placement-1/jobs?inspect=ref&tab=runs");
  });

  it("stores a versioned surface preference with safe defaults", () => {
    expect(preferredWorkspaceRoute("placement-1")).toBe(
      "/workspaces/placement-1/agent",
    );

    rememberWorkspaceRoute("placement-1", "node-1", "workbench");
    rememberNodeRoute("node-2");

    expect(preferredWorkspaceRoute("placement-1")).toBe(
      "/workspaces/placement-1/workbench",
    );
    expect(lastWorkspaceId()).toBe("placement-1");
    expect(lastNodeId()).toBe("node-2");

    window.localStorage.setItem("uprava.workspace-routes.v1", "not-json");
    expect(preferredWorkspaceRoute("placement-1")).toBe(
      "/workspaces/placement-1/agent",
    );
  });
});
