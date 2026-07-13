import { AlertTriangle, ArrowRight, Circle, Server } from "lucide-react";
import { Link } from "react-router-dom";

import { useHealth, useInventory } from "../inventory/api";
import type {
  InventorySnapshot,
  NodePresence,
  RuntimeSessionState,
  SessionThreadState,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  EmptyState,
  FigureCaption,
  LoadingState,
  PageHeader,
} from "../../shared/ui/system";
import {
  workspaceAgentSessionRoute,
  workspaceRoute,
} from "../workspaces/routes";

type Tone = "neutral" | "good" | "warn" | "bad" | "info";

type AttentionItem = {
  key: string;
  title: string;
  detail: string;
  to: string;
  tone: Tone;
};

export function DashboardRoute() {
  const inventory = useInventory();
  const health = useHealth();

  if (inventory.isLoading) {
    return (
      <section>
        <PageHeader
          title="Dashboard"
          description="Distributed runtime status & current workload."
        />
        <LoadingState stage="Loading system state" />
      </section>
    );
  }

  if (inventory.isError || !inventory.data) {
    return (
      <section>
        <PageHeader
          title="Dashboard"
          description="Distributed runtime status & current workload."
        />
        <ErrorNotice error={inventory.error} title="Core API unavailable" />
      </section>
    );
  }

  const stats = buildDashboardStats(inventory.data);
  const attentionItems = buildAttentionItems(inventory.data);
  const recentSessions = inventory.data.sessions
    .slice()
    .sort(
      (left, right) =>
        Date.parse(right.updated_at) - Date.parse(left.updated_at),
    )
    .slice(0, 5);
  const coreStatus =
    health.data?.status ?? (health.isError ? "error" : "pending");
  const coreTone =
    health.data?.status === "ok" ? "good" : health.isError ? "bad" : "warn";
  const hasRisk = stats.attentionCount > 0 || health.isError;
  const overviewState = hasRisk
    ? "Review required"
    : "System operating normally";
  const overviewCause = health.isError
    ? "Core health request failed"
    : (attentionItems[0]?.detail ?? "No active deviations");
  const overviewAction = attentionItems[0]
    ? { label: `Review ${attentionItems[0].title}`, to: attentionItems[0].to }
    : { label: "Inspect nodes", to: "/nodes" };

  return (
    <section>
      <PageHeader
        title="Dashboard"
        description="Distributed runtime status & current workload."
        meta={`SYS / ${formatDateTime(inventory.data.generated_at)}`}
      />

      <section className="grid gap-5 border-l-2 border-[var(--color-ink)] py-4 pl-4 lg:grid-cols-[minmax(0,1.2fr)_minmax(240px,0.8fr)]">
        <div>
          <div className="zarya-label">System Overview</div>
          <div
            className={`mt-2 flex items-center gap-3 ${hasRisk ? "text-[var(--color-risk)]" : "text-[var(--color-ink)]"}`}
          >
            {hasRisk ? (
              <AlertTriangle size={19} aria-hidden="true" />
            ) : (
              <Circle size={16} fill="currentColor" aria-hidden="true" />
            )}
            <h2 className="text-2xl font-bold leading-[30px]">
              {overviewState}
            </h2>
          </div>
          <p className="mt-3 max-w-2xl text-sm text-[var(--color-muted)]">
            {stats.attentionCount > 0
              ? `${stats.attentionCount} objects need review across nodes, workspaces, or runtimes.`
              : "Core, nodes, workspaces, and runtimes report no active deviation."}
          </p>
        </div>
        <dl className="grid gap-3 text-sm">
          <div>
            <dt className="zarya-label">Primary Cause</dt>
            <dd className="mt-1">{overviewCause}</dd>
          </div>
          <div>
            <dt className="zarya-label">Affected Scope</dt>
            <dd className="mt-1">{stats.attentionCount || "None"}</dd>
          </div>
          <div>
            <dt className="zarya-label">Next Action</dt>
            <dd className="mt-1">
              <Link
                to={overviewAction.to}
                className="inline-flex items-center gap-2 underline underline-offset-4 hover:no-underline"
              >
                {overviewAction.label}
                <ArrowRight size={14} aria-hidden="true" />
              </Link>
            </dd>
          </div>
        </dl>
      </section>

      <section className="grid grid-cols-2 gap-x-6 py-8 md:grid-cols-4">
        <Metric
          label="Core API"
          value={coreStatus}
          detail={health.data?.profile.replace("_", " ") ?? "health pending"}
          tone={coreTone}
        />
        <Metric
          label="Reachable Nodes"
          value={`${stats.reachableNodes}/${stats.totalNodes}`}
          detail={`${stats.unhealthyNodes} unavailable`}
          tone={stats.unhealthyNodes > 0 ? "bad" : "good"}
        />
        <Metric
          label="Active Runtimes"
          value={stats.activeRuntimeCount}
          detail={`${stats.openSessions} open sessions`}
          tone="neutral"
        />
        <Metric
          label="Attention"
          value={stats.attentionCount}
          detail="current deviations"
          tone={stats.attentionCount > 0 ? "bad" : "good"}
        />
      </section>

      <section className="zarya-rule py-8">
        <div className="mb-6 flex items-end justify-between gap-4">
          <div>
            <div className="zarya-label">Runtime Topology</div>
            <h2 className="mt-1 text-lg font-bold">Control-to-Work Pipeline</h2>
          </div>
          <Badge tone={stats.attentionCount > 0 ? "bad" : "good"}>
            {stats.attentionCount > 0 ? "review" : "normal"}
          </Badge>
        </div>
        <div className="grid items-center gap-2 text-center text-xs md:grid-cols-[1fr_auto_1fr_auto_1fr_auto_1fr]">
          <PipelineNode label="Core" value={coreStatus} risk={health.isError} />
          <ArrowRight
            className="mx-auto rotate-90 text-[var(--color-muted)] md:rotate-0"
            size={16}
            aria-hidden="true"
          />
          <PipelineNode
            label="Nodes"
            value={`${stats.reachableNodes}/${stats.totalNodes}`}
            risk={stats.unhealthyNodes > 0}
          />
          <ArrowRight
            className="mx-auto rotate-90 text-[var(--color-muted)] md:rotate-0"
            size={16}
            aria-hidden="true"
          />
          <PipelineNode
            label="Workspaces"
            value={`${stats.validatedPlacements} valid`}
            risk={stats.problemPlacements > 0}
          />
          <ArrowRight
            className="mx-auto rotate-90 text-[var(--color-muted)] md:rotate-0"
            size={16}
            aria-hidden="true"
          />
          <PipelineNode
            label="Sessions"
            value={`${stats.activeRuntimeCount} active`}
            risk={stats.errorRuntimeCount > 0}
          />
        </div>
        <FigureCaption>
          Current placement & runtime path. Deviations use a crossed risk mark.
        </FigureCaption>
      </section>

      <div className="grid gap-8 border-t border-black/10 py-8 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,0.8fr)]">
        <section>
          <div className="mb-3 flex items-baseline justify-between">
            <h2 className="text-base font-bold">Recent Sessions</h2>
            <span className="zarya-caption">
              Updated {formatDateTime(inventory.data.generated_at)}
            </span>
          </div>
          <div className="divide-y divide-black/10">
            {recentSessions.length === 0 ? (
              <EmptyState
                title="No Session Activity"
                detail="New sessions will appear here after their first event."
              />
            ) : (
              recentSessions.map((session) => (
                <Link
                  key={session.session_thread_id}
                  to={workspaceAgentSessionRoute(
                    session.project_placement_id,
                    session.session_thread_id,
                  )}
                  className="grid min-h-14 min-w-0 gap-2 py-3 hover:bg-[var(--color-bg-muted)] md:grid-cols-[minmax(0,1fr)_auto] md:items-center md:px-2"
                >
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">
                      {session.title}
                    </span>
                    <span className="block truncate text-xs text-[var(--color-muted)]">
                      {session.runtime.provider} / {session.message_count}{" "}
                      messages / {formatDateTime(session.updated_at)}
                    </span>
                  </span>
                  <span className="flex shrink-0 items-center gap-1">
                    <Badge tone={sessionTone(session.state)}>
                      {session.state}
                    </Badge>
                    <Badge tone={runtimeTone(session.runtime.state)}>
                      {session.runtime.state}
                    </Badge>
                  </span>
                </Link>
              ))
            )}
          </div>
        </section>

        <section>
          <div className="mb-3">
            <h2 className="text-base font-bold">System Attention</h2>
            <p className="zarya-caption mt-1">Cause & next review target</p>
          </div>
          <div className="divide-y divide-black/10">
            {attentionItems.length === 0 ? (
              <EmptyState
                title="No Current Attention Items"
                detail="The system is quiet. Continue monitoring runtime state."
              />
            ) : (
              attentionItems.map((item) => (
                <Link
                  key={item.key}
                  to={item.to}
                  className="block py-3 hover:bg-[var(--color-bg-muted)] md:px-2"
                >
                  <span className="flex items-center justify-between gap-3">
                    <span className="min-w-0">
                      <span className="block truncate text-sm font-medium">
                        {item.title}
                      </span>
                      <span className="block truncate text-xs text-[var(--color-muted)]">
                        {item.detail}
                      </span>
                    </span>
                    <Badge tone={item.tone}>review</Badge>
                  </span>
                </Link>
              ))
            )}
          </div>
        </section>
      </div>
    </section>
  );
}

function Metric({
  label,
  value,
  detail,
  tone = "neutral",
}: {
  label: string;
  value: string | number;
  detail: string;
  tone?: Tone;
}) {
  return (
    <article className="min-w-0 py-4">
      <div className="zarya-label">{label}</div>
      <div
        className={`mt-2 break-words text-2xl font-bold tabular-nums ${tone === "bad" ? "text-[var(--color-risk)]" : ""}`}
      >
        {value}
      </div>
      <div className="mt-1 text-xs text-[var(--color-muted)]">{detail}</div>
    </article>
  );
}

function PipelineNode({
  label,
  value,
  risk,
}: {
  label: string;
  value: string;
  risk: boolean;
}) {
  return (
    <div
      className={`min-w-0 border px-3 py-4 ${risk ? "border-[var(--color-risk)] text-[var(--color-risk)]" : "border-[var(--color-ink)]"}`}
    >
      <Server size={15} className="mx-auto mb-2" aria-hidden="true" />
      <div className="font-bold">{label}</div>
      <div className="mt-1 truncate text-[var(--color-muted)]">{value}</div>
    </div>
  );
}

function buildDashboardStats(snapshot: InventorySnapshot) {
  const nodePresence = countNodePresence(snapshot);
  const activeRuntimeCount = snapshot.nodes.reduce(
    (total, node) => total + node.active_runtime_count,
    0,
  );
  const openSessions = snapshot.sessions.filter(
    (session) => session.state !== "stopped",
  ).length;
  const readyRuntimeCount = countRuntimeStates(snapshot, ["ready", "running"]);
  const blockedRuntimeCount = countRuntimeStates(snapshot, ["blocked"]);
  const errorRuntimeCount = countRuntimeStates(snapshot, [
    "error",
    "expired",
    "stale",
  ]);
  const stoppedRuntimeCount = countRuntimeStates(snapshot, [
    "stopped",
    "stopping",
    "interrupted",
  ]);
  const validatedPlacements = snapshot.placements.filter(
    (placement) => placement.state === "validated",
  ).length;
  const pendingPlacements = snapshot.placements.filter(
    (placement) => placement.state === "pending",
  ).length;
  const problemPlacements = snapshot.placements.filter(
    (placement) =>
      placement.state === "missing" ||
      placement.state === "read_only" ||
      placement.state === "error",
  ).length;
  const resourceWarningCount = snapshot.placements.reduce(
    (total, placement) =>
      total +
      placement.resource_badges.filter((badge) => badge.severity !== "info")
        .length,
    0,
  );
  const unhealthyNodes =
    nodePresence.stale + nodePresence.offline + nodePresence.revoked;
  const attentionCount =
    unhealthyNodes +
    problemPlacements +
    resourceWarningCount +
    blockedRuntimeCount +
    errorRuntimeCount;

  return {
    activeRuntimeCount,
    attentionCount,
    blockedRuntimeCount,
    errorRuntimeCount,
    nodePresence,
    openSessions,
    pendingPlacements,
    problemPlacements,
    readyRuntimeCount,
    reachableNodes: nodePresence.reachable,
    resourceWarningCount,
    stoppedRuntimeCount,
    totalNodes: snapshot.nodes.length,
    unhealthyNodes,
    validatedPlacements,
  };
}

function countNodePresence(
  snapshot: InventorySnapshot,
): Record<NodePresence, number> {
  return snapshot.nodes.reduce<Record<NodePresence, number>>(
    (counts, node) => {
      counts[node.presence] += 1;
      return counts;
    },
    { reachable: 0, stale: 0, offline: 0, revoked: 0 },
  );
}

function countRuntimeStates(
  snapshot: InventorySnapshot,
  states: RuntimeSessionState[],
) {
  return snapshot.sessions.filter((session) =>
    states.includes(session.runtime.state),
  ).length;
}

function buildAttentionItems(snapshot: InventorySnapshot): AttentionItem[] {
  const nodeItems = snapshot.nodes
    .filter((node) => node.presence !== "reachable")
    .map<AttentionItem>((node) => ({
      key: `node-${node.node_id}`,
      title: node.display_name,
      detail: `Node is ${node.presence.replace("_", " ")}`,
      to: `/nodes/${node.node_id}`,
      tone: node.presence === "offline" ? "bad" : "warn",
    }));

  const placementItems = snapshot.placements
    .filter(
      (placement) =>
        placement.state !== "validated" ||
        placement.resource_badges.some((badge) => badge.severity !== "info"),
    )
    .map<AttentionItem>((placement) => ({
      key: `placement-${placement.project_placement_id}`,
      title: placement.display_name,
      detail:
        placement.resource_badges.find((badge) => badge.severity !== "info")
          ?.label ?? `Workspace is ${placement.state}`,
      to: workspaceRoute(placement.project_placement_id),
      tone:
        placement.state === "error" || placement.state === "missing"
          ? "bad"
          : "warn",
    }));

  const sessionItems = snapshot.sessions
    .filter((session) =>
      ["blocked", "error", "expired", "stale"].includes(session.runtime.state),
    )
    .map<AttentionItem>((session) => ({
      key: `session-${session.session_thread_id}`,
      title: session.title,
      detail: `Runtime is ${session.runtime.state}`,
      to: workspaceAgentSessionRoute(
        session.project_placement_id,
        session.session_thread_id,
      ),
      tone: session.runtime.state === "error" ? "bad" : "warn",
    }));

  return [...nodeItems, ...placementItems, ...sessionItems].slice(0, 6);
}

function sessionTone(state: SessionThreadState): Tone {
  if (state === "active") return "good";
  if (state === "degraded") return "bad";
  if (state === "detached") return "warn";
  return "neutral";
}

function runtimeTone(state: RuntimeSessionState): Tone {
  if (state === "ready" || state === "running") return "good";
  if (state === "error") return "bad";
  if (
    state === "blocked" ||
    state === "interrupted" ||
    state === "stale" ||
    state === "expired" ||
    state === "stopping"
  ) {
    return "warn";
  }
  if (state === "starting" || state === "resuming") return "info";
  return "neutral";
}

function formatDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.valueOf())) {
    return value;
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}
