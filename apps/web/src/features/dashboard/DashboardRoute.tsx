import type { ReactNode } from "react";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Clock3,
  Cpu,
  Folder,
  Server,
  Workflow,
} from "lucide-react";
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

type Tone = "neutral" | "good" | "warn" | "bad" | "info";

type StatCardProps = {
  icon: ReactNode;
  label: string;
  value: string | number;
  detail: string;
  tone?: Tone;
};

type StatusRowProps = {
  label: string;
  value: string | number;
  tone?: Tone;
};

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
      <section className="space-y-4">
        <h1 className="text-2xl font-semibold">Dashboard</h1>
        <div className="rounded-md border border-[#cad2c7] bg-white p-5 text-sm text-[#536257]">
          Loading system state
        </div>
      </section>
    );
  }

  if (inventory.isError || !inventory.data) {
    return (
      <section className="space-y-4">
        <h1 className="text-2xl font-semibold">Dashboard</h1>
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

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">Dashboard</h1>
          <p className="text-sm text-[#536257]">
            Distributed runtime status and current workload.
          </p>
        </div>
        <div className="text-sm text-[#536257]">
          Updated {formatDateTime(inventory.data.generated_at)}
        </div>
      </div>

      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-4">
        <StatCard
          icon={<Activity size={18} />}
          label="Core API"
          value={coreStatus}
          detail={
            health.data?.profile.replace("_", " ") ?? "waiting for health"
          }
          tone={coreTone}
        />
        <StatCard
          icon={<Server size={18} />}
          label="Nodes"
          value={`${stats.reachableNodes}/${stats.totalNodes}`}
          detail="reachable nodes"
          tone={stats.unhealthyNodes > 0 ? "warn" : "good"}
        />
        <StatCard
          icon={<Workflow size={18} />}
          label="Active sessions"
          value={stats.activeRuntimeCount}
          detail={`${stats.openSessions} open session threads`}
          tone={stats.activeRuntimeCount > 0 ? "info" : "neutral"}
        />
        <StatCard
          icon={<AlertTriangle size={18} />}
          label="Attention"
          value={stats.attentionCount}
          detail="nodes, workspaces, or runtimes"
          tone={stats.attentionCount > 0 ? "warn" : "good"}
        />
      </div>

      <div className="grid gap-4 xl:grid-cols-3">
        <StatusPanel title="Node state" icon={<Server size={18} />}>
          <StatusRow
            label="Reachable"
            value={stats.nodePresence.reachable}
            tone="good"
          />
          <StatusRow
            label="Stale"
            value={stats.nodePresence.stale}
            tone="warn"
          />
          <StatusRow
            label="Offline"
            value={stats.nodePresence.offline}
            tone="bad"
          />
          <StatusRow
            label="Revoked"
            value={stats.nodePresence.revoked}
            tone="neutral"
          />
        </StatusPanel>

        <StatusPanel title="Runtime state" icon={<Cpu size={18} />}>
          <StatusRow
            label="Ready or running"
            value={stats.readyRuntimeCount}
            tone="good"
          />
          <StatusRow
            label="Blocked"
            value={stats.blockedRuntimeCount}
            tone="warn"
          />
          <StatusRow
            label="Error or stale"
            value={stats.errorRuntimeCount}
            tone="bad"
          />
          <StatusRow
            label="Stopped"
            value={stats.stoppedRuntimeCount}
            tone="neutral"
          />
        </StatusPanel>

        <StatusPanel title="Workspace state" icon={<Folder size={18} />}>
          <StatusRow
            label="Validated"
            value={stats.validatedPlacements}
            tone="good"
          />
          <StatusRow
            label="Pending"
            value={stats.pendingPlacements}
            tone="neutral"
          />
          <StatusRow
            label="Needs action"
            value={stats.problemPlacements}
            tone={stats.problemPlacements > 0 ? "warn" : "good"}
          />
          <StatusRow
            label="Resource warnings"
            value={stats.resourceWarningCount}
            tone={stats.resourceWarningCount > 0 ? "warn" : "good"}
          />
        </StatusPanel>
      </div>

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.2fr)_minmax(0,0.8fr)]">
        <section className="rounded-md border border-[#d9ded4] bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-2">
            <Clock3 size={18} />
            <h2 className="text-base font-semibold">Recent sessions</h2>
          </div>
          <div className="space-y-2">
            {recentSessions.length === 0 ? (
              <div className="rounded-md border border-[#cad2c7] bg-[#f8faf5] p-3 text-sm text-[#536257]">
                No session activity yet
              </div>
            ) : (
              recentSessions.map((session) => (
                <Link
                  key={session.session_thread_id}
                  to={`/sessions/${session.session_thread_id}`}
                  className="flex min-h-14 min-w-0 items-center justify-between gap-3 rounded-md px-3 py-2 hover:bg-[#f4f7f0]"
                >
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">
                      {session.title}
                    </span>
                    <span className="block truncate text-xs text-[#536257]">
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

        <section className="rounded-md border border-[#d9ded4] bg-white p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-2">
            <CheckCircle2 size={18} />
            <h2 className="text-base font-semibold">System attention</h2>
          </div>
          <div className="space-y-2">
            {attentionItems.length === 0 ? (
              <div className="rounded-md border border-[#cad2c7] bg-[#f8faf5] p-3 text-sm text-[#536257]">
                No current attention items
              </div>
            ) : (
              attentionItems.map((item) => (
                <Link
                  key={item.key}
                  to={item.to}
                  className="block rounded-md px-3 py-2 hover:bg-[#f4f7f0]"
                >
                  <span className="flex items-center justify-between gap-3">
                    <span className="min-w-0">
                      <span className="block truncate text-sm font-medium">
                        {item.title}
                      </span>
                      <span className="block truncate text-xs text-[#536257]">
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

function StatCard({
  icon,
  label,
  value,
  detail,
  tone = "neutral",
}: StatCardProps) {
  return (
    <article className="rounded-md border border-[#d9ded4] bg-white p-4 shadow-sm">
      <div className="mb-3 flex items-center justify-between gap-3">
        <span className="text-[#536257]">{icon}</span>
        <Badge tone={tone}>{label}</Badge>
      </div>
      <div className="text-2xl font-semibold">{value}</div>
      <div className="mt-1 text-sm text-[#536257]">{detail}</div>
    </article>
  );
}

function StatusPanel({
  title,
  icon,
  children,
}: {
  title: string;
  icon: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="rounded-md border border-[#d9ded4] bg-white p-4 shadow-sm">
      <div className="mb-3 flex items-center gap-2">
        <span className="text-[#536257]">{icon}</span>
        <h2 className="text-base font-semibold">{title}</h2>
      </div>
      <div className="space-y-2">{children}</div>
    </section>
  );
}

function StatusRow({ label, value, tone = "neutral" }: StatusRowProps) {
  return (
    <div className="flex min-h-9 items-center justify-between gap-3 rounded-md bg-[#f8faf5] px-3">
      <span className="text-sm text-[#536257]">{label}</span>
      <Badge tone={tone}>{value}</Badge>
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
      to: `/workspaces/${placement.project_placement_id}`,
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
      to: `/sessions/${session.session_thread_id}`,
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
