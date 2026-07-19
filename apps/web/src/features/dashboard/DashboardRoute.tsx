import { useQuery } from "@tanstack/react-query";
import { BriefcaseBusiness, MessageSquareText } from "lucide-react";
import { Link } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  InventorySnapshot,
  JobSummary,
  NodePresence,
  SessionSummary,
} from "../../shared/protocol/types";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  StatusIndicator,
  type StatusDimension,
} from "../../shared/ui/status-indicator";
import { EmptyState, LoadingState, PageHeader } from "../../shared/ui/system";
import { useHealth, useInventory } from "../inventory/api";
import { isJobRunActive } from "../jobs/status";
import { sessionAttention } from "../sessions/session-attention";
import {
  workspaceAgentSessionRoute,
  workspaceJobRunRoute,
} from "../workspaces/routes";

export type RecentActivityItem = {
  key: string;
  kind: "session" | "job";
  title: string;
  detail: string;
  occurredAt: string;
  to: string;
  lifecycle: string;
  attention: string | null;
};

export function DashboardRoute() {
  const inventory = useInventory();
  const health = useHealth();
  const jobs = useQuery({
    queryKey: queryKeys.jobs,
    queryFn: coreApi.jobs,
  });

  if (inventory.isLoading) {
    return (
      <section>
        <DashboardHeader />
        <LoadingState stage="Loading system state" />
      </section>
    );
  }

  if (inventory.isError || !inventory.data) {
    return (
      <section>
        <DashboardHeader />
        <ErrorNotice error={inventory.error} title="Core API unavailable" />
      </section>
    );
  }

  const stats = buildDashboardStats(inventory.data, jobs.data ?? []);
  const activity = buildRecentActivity(inventory.data, jobs.data ?? []);
  const coreStatus =
    health.data?.status ?? (health.isError ? "error" : "pending");

  return (
    <section>
      <DashboardHeader generatedAt={inventory.data.generated_at} />

      <section
        className="grid grid-cols-2 gap-x-6 border-b border-[var(--color-border)] py-6 md:grid-cols-4"
        aria-label="System metrics"
      >
        <Metric
          label="Core API"
          value={health.data?.status ?? (health.isError ? "error" : "—")}
          detail={health.data?.profile.replaceAll("_", " ") ?? "health check"}
          status={{ dimension: "presence", value: coreStatus }}
        />
        <Metric
          label="Reachable Nodes"
          value={`${stats.reachableNodes}/${stats.totalNodes}`}
          detail={`${stats.totalNodes - stats.reachableNodes} unavailable`}
          status={{
            dimension: "presence",
            value:
              stats.reachableNodes === stats.totalNodes
                ? "reachable"
                : "offline",
            label:
              stats.reachableNodes === stats.totalNodes
                ? "All reachable"
                : "Reduced reachability",
          }}
        />
        <Metric
          label="Active Runtimes"
          value={stats.activeRuntimeCount}
          detail="managed runtimes"
          status={{
            dimension: "lifecycle",
            value: stats.activeRuntimeCount > 0 ? "active" : "idle",
          }}
        />
        <Metric
          label="Running Jobs"
          value={jobs.isError ? "—" : stats.runningJobCount}
          detail={jobs.isError ? "Jobs unavailable" : "queued or executing"}
          status={
            jobs.isError
              ? { dimension: "attention", value: "error" }
              : {
                  dimension: "lifecycle",
                  value: stats.runningJobCount > 0 ? "running" : "idle",
                }
          }
        />
      </section>

      <section className="py-7" aria-labelledby="recent-activity-title">
        <div className="mb-4 flex flex-wrap items-end justify-between gap-2">
          <div>
            <div className="zarya-caption">SYSTEM ACTIVITY</div>
            <h2 id="recent-activity-title" className="mt-1 text-lg font-bold">
              Recent Activity
            </h2>
          </div>
          <span className="zarya-caption">
            Updated {formatDateTime(inventory.data.generated_at)}
          </span>
        </div>

        {jobs.isError ? (
          <div className="mb-4 max-w-2xl">
            <ErrorNotice error={jobs.error} title="Job activity unavailable" />
          </div>
        ) : null}

        {activity.length === 0 ? (
          <EmptyState
            title="No Recent Activity"
            detail="Session and Job activity will appear here."
          />
        ) : (
          <ol className="divide-y divide-[var(--color-border)]">
            {activity.map((item) => (
              <li key={item.key}>
                <Link
                  to={item.to}
                  className="grid min-w-0 gap-3 py-3 hover:bg-[var(--color-bg-muted)] sm:grid-cols-[auto_minmax(0,1fr)_auto] sm:items-center sm:px-2"
                >
                  <ActivityKindIcon kind={item.kind} />
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">
                      {item.title}
                    </span>
                    <span className="block truncate text-xs text-[var(--color-muted)]">
                      {item.detail} · {formatDateTime(item.occurredAt)}
                    </span>
                  </span>
                  <span className="flex flex-wrap gap-1">
                    <StatusIndicator
                      dimension="lifecycle"
                      value={item.lifecycle}
                    />
                    {item.attention ? (
                      <StatusIndicator
                        dimension="attention"
                        value={item.attention}
                      />
                    ) : null}
                  </span>
                </Link>
              </li>
            ))}
          </ol>
        )}
      </section>
    </section>
  );
}

function DashboardHeader({ generatedAt }: { generatedAt?: string }) {
  return (
    <PageHeader
      title="Dashboard"
      description="Distributed runtime status & current workload."
      meta={generatedAt ? `SYS / ${formatDateTime(generatedAt)}` : undefined}
    />
  );
}

function Metric({
  detail,
  label,
  status,
  value,
}: {
  detail: string;
  label: string;
  status: { dimension: StatusDimension; value: string; label?: string };
  value: number | string;
}) {
  return (
    <article className="min-w-0 py-4">
      <div className="zarya-label">{label}</div>
      <div className="mt-2 break-words text-2xl font-bold tabular-nums">
        {value}
      </div>
      <div className="mt-1 text-xs text-[var(--color-muted)]">{detail}</div>
      <div className="mt-3">
        <StatusIndicator {...status} />
      </div>
    </article>
  );
}

function ActivityKindIcon({ kind }: { kind: RecentActivityItem["kind"] }) {
  const Icon = kind === "session" ? MessageSquareText : BriefcaseBusiness;
  return (
    <span className="hidden h-8 w-8 items-center justify-center border border-[var(--color-border-strong)] sm:inline-flex">
      <Icon size={15} aria-hidden="true" />
    </span>
  );
}

export function buildDashboardStats(
  inventory: InventorySnapshot,
  jobs: JobSummary[],
) {
  const presence = inventory.nodes.reduce<Record<NodePresence, number>>(
    (counts, node) => {
      counts[node.presence] += 1;
      return counts;
    },
    { reachable: 0, stale: 0, offline: 0, revoked: 0 },
  );

  return {
    activeRuntimeCount: inventory.nodes.reduce(
      (total, node) => total + node.active_runtime_count,
      0,
    ),
    reachableNodes: presence.reachable,
    runningJobCount: jobs.filter(
      (job) => job.latest_run && isJobRunActive(job.latest_run.state),
    ).length,
    totalNodes: inventory.nodes.length,
  };
}

export function buildRecentActivity(
  inventory: InventorySnapshot,
  jobs: JobSummary[],
): RecentActivityItem[] {
  const placementNames = new Map(
    inventory.placements.map((placement) => [
      placement.project_placement_id,
      placement.display_name,
    ]),
  );
  const sessionActivity = inventory.sessions.map((session) =>
    activityForSession(session, placementNames),
  );
  const jobActivity = jobs.flatMap<RecentActivityItem>((job) => {
    const run = job.latest_run;
    if (!run) return [];
    return [
      {
        key: `job-${run.job_run_id}`,
        kind: "job",
        title: job.name,
        detail: `${job.placement_name} · ${run.trigger} Job run`,
        occurredAt: run.finished_at ?? run.started_at ?? run.queued_at,
        to: workspaceJobRunRoute(
          job.project_placement_id,
          job.job_id,
          run.job_run_id,
        ),
        lifecycle: run.state,
        attention: null,
      },
    ];
  });

  return [...sessionActivity, ...jobActivity]
    .sort(
      (left, right) =>
        Date.parse(right.occurredAt) - Date.parse(left.occurredAt) ||
        left.key.localeCompare(right.key),
    )
    .slice(0, 8);
}

function activityForSession(
  session: SessionSummary,
  placementNames: Map<string, string>,
): RecentActivityItem {
  return {
    key: `session-${session.session_thread_id}`,
    kind: "session",
    title: session.title,
    detail: `${placementNames.get(session.project_placement_id) ?? "Workspace"} · ${session.runtime.provider} session`,
    occurredAt: session.updated_at,
    to: workspaceAgentSessionRoute(
      session.project_placement_id,
      session.session_thread_id,
    ),
    lifecycle: session.state,
    attention: sessionAttention(
      session.state,
      session.runtime.state,
      session.runtime.degraded_reason,
      session.runtime.last_runtime_step_at,
    ),
  };
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) return value;
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}
