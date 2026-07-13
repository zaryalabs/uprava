import { describe, expect, it } from "vitest";

import type {
  InventorySnapshot,
  JobSummary,
} from "../../shared/protocol/types";
import { buildDashboardStats, buildRecentActivity } from "./DashboardRoute";

describe("Dashboard model", () => {
  it("builds exactly the operational metrics from inventory and Jobs", () => {
    expect(buildDashboardStats(inventory, jobs)).toEqual({
      activeRuntimeCount: 2,
      reachableNodes: 1,
      runningJobCount: 1,
      totalNodes: 2,
    });
  });

  it("combines session and Job activity in descending order with workspace routes", () => {
    const activity = buildRecentActivity(inventory, jobs);

    expect(activity.map((item) => item.key)).toEqual([
      "job-run-running",
      "session-session-1",
      "job-run-finished",
    ]);
    expect(activity[0]?.to).toBe(
      "/workspaces/placement-1/jobs/job-running/runs/run-running",
    );
    expect(activity[1]).toMatchObject({
      lifecycle: "active",
      attention: "blocked",
      to: "/workspaces/placement-1/agent/session-1",
    });
  });
});

const inventory = {
  nodes: [
    {
      node_id: "node-1",
      display_name: "Local Node",
      presence: "reachable",
      sleep_hint: "unknown",
      heartbeat_age_seconds: 4,
      active_runtime_count: 2,
      capabilities: [],
      diagnostics: "ok",
    },
    {
      node_id: "node-2",
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
      last_validated_at: "2026-07-13T10:00:00Z",
    },
  ],
  sessions: [
    {
      session_thread_id: "session-1",
      project_placement_id: "placement-1",
      runtime_session_id: "runtime-1",
      title: "Fix issue",
      state: "active",
      runtime: {
        runtime_session_id: "runtime-1",
        provider: "codex",
        state: "blocked",
        resume_supported: true,
        degraded_reason: null,
        last_runtime_step_at: "2026-07-13T10:04:00Z",
      },
      message_count: 2,
      updated_at: "2026-07-13T10:04:00Z",
    },
  ],
  generated_at: "2026-07-13T10:06:00Z",
} satisfies InventorySnapshot;

const jobBase = {
  name: "Workspace check",
  project_placement_id: "placement-1",
  placement_name: "Uprava",
  provider: "codex",
  enabled: true,
  schedule: null,
  timezone: "UTC",
  overlap_policy: "skip",
  continue_after_error: false,
  next_run_at: null,
  paused_reason: null,
  created_at: "2026-07-13T09:00:00Z",
  updated_at: "2026-07-13T10:05:00Z",
} satisfies Omit<JobSummary, "job_id" | "latest_run">;

const jobs: JobSummary[] = [
  {
    ...jobBase,
    job_id: "job-running",
    latest_run: {
      job_run_id: "run-running",
      job_id: "job-running",
      trigger: "scheduled",
      state: "running",
      scheduled_for: "2026-07-13T10:05:00Z",
      queued_at: "2026-07-13T10:05:00Z",
      started_at: "2026-07-13T10:05:30Z",
      finished_at: null,
      session_thread_id: null,
      runtime_session_id: null,
      summary: null,
      terminal_reason: null,
      config_snapshot: {},
      force: false,
    },
  },
  {
    ...jobBase,
    job_id: "job-finished",
    latest_run: {
      job_run_id: "run-finished",
      job_id: "job-finished",
      trigger: "manual",
      state: "succeeded",
      scheduled_for: null,
      queued_at: "2026-07-13T10:01:00Z",
      started_at: "2026-07-13T10:01:10Z",
      finished_at: "2026-07-13T10:02:00Z",
      session_thread_id: "session-1",
      runtime_session_id: "runtime-1",
      summary: "Done",
      terminal_reason: null,
      config_snapshot: {},
      force: false,
    },
  },
];
