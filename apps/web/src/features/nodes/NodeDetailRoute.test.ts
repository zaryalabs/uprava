import { describe, expect, it } from "vitest";

import type {
  JobSummary,
  NodeSummary,
  ProjectPlacementSummary,
} from "../../shared/protocol/types";
import { buildNodeOverview, nextNodeRouteAfterDelete } from "./NodeDetailRoute";

describe("Node Overview model", () => {
  it("counts running Jobs only through placements on the Node", () => {
    const stats = buildNodeOverview(
      nodes[0],
      [placement("placement-1")],
      [
        job("job-local", "placement-1", "running"),
        job("job-foreign", "placement-2", "running"),
        job("job-finished", "placement-1", "succeeded"),
      ],
    );

    expect(stats).toEqual({
      activeRuntimeCount: 3,
      runningJobCount: 1,
      workspaceCount: 1,
    });
  });

  it("chooses a neighboring Node or Dashboard after delete", () => {
    expect(nextNodeRouteAfterDelete(nodes, "node-1")).toBe("/nodes/node-2");
    expect(nextNodeRouteAfterDelete(nodes, "node-2")).toBe("/nodes/node-1");
    expect(nextNodeRouteAfterDelete([nodes[0]], "node-1")).toBe("/dashboard");
  });
});

const nodes: NodeSummary[] = [
  {
    node_id: "node-1",
    display_name: "One",
    presence: "reachable",
    sleep_hint: "unknown",
    heartbeat_age_seconds: 1,
    active_runtime_count: 3,
    capabilities: [],
    diagnostics: "ok",
  },
  {
    node_id: "node-2",
    display_name: "Two",
    presence: "reachable",
    sleep_hint: "unknown",
    heartbeat_age_seconds: 2,
    active_runtime_count: 0,
    capabilities: [],
    diagnostics: "ok",
  },
];

function placement(projectPlacementId: string): ProjectPlacementSummary {
  return {
    project_placement_id: projectPlacementId,
    project_id: null,
    node_id: "node-1",
    display_name: projectPlacementId,
    workspace_path: `/workspace/${projectPlacementId}`,
    state: "validated",
    resource_badges: [],
    last_validated_at: null,
  };
}

function job(
  jobId: string,
  projectPlacementId: string,
  state: NonNullable<JobSummary["latest_run"]>["state"],
): JobSummary {
  return {
    job_id: jobId,
    name: jobId,
    project_placement_id: projectPlacementId,
    placement_name: projectPlacementId,
    provider: "codex",
    enabled: true,
    schedule: null,
    timezone: "UTC",
    overlap_policy: "skip",
    continue_after_error: false,
    next_run_at: null,
    paused_reason: null,
    latest_run: {
      job_run_id: `run-${jobId}`,
      job_id: jobId,
      trigger: "manual",
      state,
      scheduled_for: null,
      queued_at: "2026-07-13T10:00:00Z",
      started_at: null,
      finished_at: null,
      session_thread_id: null,
      runtime_session_id: null,
      summary: null,
      terminal_reason: null,
      config_snapshot: {},
      force: false,
    },
    created_at: "2026-07-13T10:00:00Z",
    updated_at: "2026-07-13T10:00:00Z",
  };
}
