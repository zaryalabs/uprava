import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderPlus, KeyRound, ShieldOff, Trash2 } from "lucide-react";
import { useEffect, useState, type ReactNode } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  JobSummary,
  NodeCredentialRotationResponse,
  NodeSummary,
  ProjectPlacementSummary,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { StatusIndicator } from "../../shared/ui/status-indicator";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import {
  routeForRef,
  workspaceRefForPlacement,
} from "../../workbench/references/refs";
import { useInventory } from "../inventory/api";
import { isJobRunActive } from "../jobs/status";
import { rememberNodeRoute } from "../workspaces/routes";
import { HeartbeatAge } from "./HeartbeatAge";

export function NodeDetailRoute() {
  const { nodeId = "" } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [rotatedCredential, setRotatedCredential] =
    useState<NodeCredentialRotationResponse | null>(null);
  const inventory = useInventory();
  const jobs = useQuery({ queryKey: queryKeys.jobs, queryFn: coreApi.jobs });
  const node = inventory.data?.nodes.find(
    (candidate) => candidate.node_id === nodeId,
  );
  const placements =
    inventory.data?.placements.filter(
      (placement) => placement.node_id === nodeId,
    ) ?? [];

  useEffect(() => {
    setRotatedCredential(null);
    if (nodeId) rememberNodeRoute(nodeId);
  }, [nodeId]);

  const invalidateNode = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.inventory }),
      queryClient.invalidateQueries({ queryKey: queryKeys.node(nodeId) }),
    ]);
  };
  const revokeNode = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("node.revoke", {
        nodeId,
        afterSuccess: invalidateNode,
      }),
  });
  const deleteNode = useMutation({
    mutationFn: () => {
      const destination = nextNodeRouteAfterDelete(
        inventory.data?.nodes ?? [],
        nodeId,
      );
      return runWorkbenchCommand("node.delete", {
        nodeId,
        afterSuccess: async () => {
          await invalidateNode();
          navigate(destination, { replace: true });
        },
      });
    },
  });
  const rotateCredential = useMutation({
    mutationFn: async () => {
      const result = await runWorkbenchCommand("node.rotateCredential", {
        nodeId,
        afterSuccess: invalidateNode,
      });
      return result as NodeCredentialRotationResponse;
    },
    onMutate: () => setRotatedCredential(null),
    onSuccess: setRotatedCredential,
    onError: () => setRotatedCredential(null),
  });

  if (inventory.isError && !inventory.data) {
    return (
      <ErrorNotice error={inventory.error} title="Node inventory failed" />
    );
  }
  if (!inventory.data) {
    return (
      <div className="text-sm text-[var(--color-muted)]">Loading node</div>
    );
  }
  if (!node) {
    return (
      <div className="text-sm text-[var(--color-muted)]">Node not found</div>
    );
  }

  const stats = buildNodeOverview(node, placements, jobs.data ?? []);
  const canRevoke = canRunCommand("node.revoke", { nodeId: node.node_id });
  const canRotate = canRunCommand("node.rotateCredential", {
    nodeId: node.node_id,
  });
  const canDelete = canRunCommand("node.delete", { nodeId: node.node_id });

  return (
    <section className="space-y-7">
      <header className="flex flex-wrap items-start justify-between gap-4 border-b border-[var(--color-border)] pb-5">
        <div className="min-w-0">
          <div className="zarya-caption">NODE OVERVIEW</div>
          <h1 className="mt-2 text-2xl font-semibold">{node.display_name}</h1>
          <div className="mt-2 flex flex-wrap items-center gap-2">
            <StatusIndicator dimension="presence" value={node.presence} />
            <span className="text-sm text-[var(--color-muted)]">
              Last heartbeat{" "}
              <HeartbeatAge seconds={node.heartbeat_age_seconds} />
            </span>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <ReferenceActions
            reference={{ kind: "node", node_id: node.node_id }}
          />
          <Link
            to={`/nodes/${encodeURIComponent(node.node_id)}/placements/new`}
          >
            <Button>
              <FolderPlus size={16} aria-hidden="true" />
              Add Workspace
            </Button>
          </Link>
          <Button
            disabled={!canRotate || rotateCredential.isPending}
            onClick={() => rotateCredential.mutate()}
          >
            <KeyRound size={16} aria-hidden="true" />
            Rotate
          </Button>
          <Button
            variant="danger"
            disabled={!canRevoke || revokeNode.isPending}
            onClick={() => revokeNode.mutate()}
          >
            <ShieldOff size={16} aria-hidden="true" />
            Revoke
          </Button>
          <Button
            variant="danger"
            disabled={!canDelete || deleteNode.isPending}
            onClick={() => {
              if (confirmNodeDelete(node.display_name)) deleteNode.mutate();
            }}
          >
            <Trash2 size={16} aria-hidden="true" />
            Delete
          </Button>
        </div>
      </header>

      {inventory.isError ? (
        <ErrorNotice error={inventory.error} title="Node refresh failed" />
      ) : null}
      {revokeNode.isError ? (
        <ErrorNotice error={revokeNode.error} title="Node revoke failed" />
      ) : null}
      {rotateCredential.isError ? (
        <ErrorNotice
          error={rotateCredential.error}
          title="Credential rotation failed"
        />
      ) : null}
      {deleteNode.isError ? (
        <ErrorNotice error={deleteNode.error} title="Node delete failed" />
      ) : null}
      {rotatedCredential ? (
        <section className="border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3 text-sm">
          <div className="font-medium">New node credential</div>
          <div className="mt-2 overflow-x-auto bg-[var(--color-bg)] px-2 py-1 font-mono text-xs">
            {rotatedCredential.credential}
          </div>
        </section>
      ) : null}

      <section
        className="grid grid-cols-2 gap-x-6 border-b border-[var(--color-border)] pb-7 md:grid-cols-4"
        aria-label="Node metrics"
      >
        <NodeMetric label="Last Heartbeat">
          <HeartbeatAge seconds={node.heartbeat_age_seconds} />
        </NodeMetric>
        <NodeMetric label="Workspaces">{stats.workspaceCount}</NodeMetric>
        <NodeMetric label="Active Runtimes">
          {stats.activeRuntimeCount}
        </NodeMetric>
        <NodeMetric label="Running Jobs">
          {jobs.isError ? "—" : stats.runningJobCount}
        </NodeMetric>
      </section>

      {jobs.isError ? (
        <ErrorNotice error={jobs.error} title="Running Jobs unavailable" />
      ) : null}

      <div className="grid gap-7 lg:grid-cols-[minmax(0,1.4fr)_minmax(260px,0.6fr)]">
        <section aria-labelledby="node-workspaces-title">
          <div className="mb-3 flex items-center justify-between gap-3">
            <h2 id="node-workspaces-title" className="text-lg font-semibold">
              Workspaces
            </h2>
            <span className="zarya-caption">{stats.workspaceCount} total</span>
          </div>
          <div className="divide-y divide-[var(--color-border)] border-y border-[var(--color-border)]">
            {placements.map((placement) => (
              <Link
                key={placement.project_placement_id}
                to={routeForRef(workspaceRefForPlacement(placement)) ?? "#"}
                className="grid min-w-0 gap-2 py-3 hover:bg-[var(--color-bg-muted)] sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center sm:px-2"
              >
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium">
                    {placement.display_name}
                  </span>
                  <span className="block truncate text-xs text-[var(--color-muted)]">
                    {placement.workspace_path}
                  </span>
                </span>
                <span className="flex flex-wrap gap-1">
                  <StatusIndicator
                    dimension="workspace"
                    value={placement.state}
                  />
                  {placement.resource_badges.some(
                    (badge) => badge.severity !== "info",
                  ) ? (
                    <StatusIndicator
                      dimension="attention"
                      value={
                        placement.resource_badges.some(
                          (badge) => badge.severity === "hard_block",
                        )
                          ? "hard_block"
                          : "warning"
                      }
                    />
                  ) : null}
                </span>
              </Link>
            ))}
            {placements.length === 0 ? (
              <div className="py-5 text-sm text-[var(--color-muted)]">
                No workspaces on this Node.
              </div>
            ) : null}
          </div>
        </section>

        <div className="space-y-6">
          <section aria-labelledby="node-capabilities-title">
            <h2 id="node-capabilities-title" className="text-sm font-bold">
              Capabilities
            </h2>
            <div className="mt-3 flex flex-wrap gap-2">
              {node.capabilities.map((capability) => (
                <Badge key={capability.key}>{capability.key}</Badge>
              ))}
              {node.capabilities.length === 0 ? (
                <span className="text-sm text-[var(--color-muted)]">
                  None advertised
                </span>
              ) : null}
            </div>
          </section>
          <section aria-labelledby="node-diagnostics-title">
            <h2 id="node-diagnostics-title" className="text-sm font-bold">
              Diagnostics
            </h2>
            <p className="mt-2 border-l-2 border-[var(--color-border-strong)] pl-3 text-sm text-[var(--color-muted)]">
              {node.diagnostics || "No diagnostics reported."}
            </p>
            <p className="mt-2 text-xs text-[var(--color-muted)]">
              Sleep hint: {node.sleep_hint}
            </p>
          </section>
        </div>
      </div>
    </section>
  );
}

function NodeMetric({
  children,
  label,
}: {
  children: ReactNode;
  label: string;
}) {
  return (
    <article className="min-w-0 py-3">
      <div className="zarya-label">{label}</div>
      <div className="mt-2 break-words text-2xl font-bold tabular-nums">
        {children}
      </div>
    </article>
  );
}

export function buildNodeOverview(
  node: NodeSummary,
  placements: ProjectPlacementSummary[],
  jobs: JobSummary[],
) {
  const placementIds = new Set(
    placements.map((placement) => placement.project_placement_id),
  );
  return {
    activeRuntimeCount: node.active_runtime_count,
    runningJobCount: jobs.filter(
      (job) =>
        placementIds.has(job.project_placement_id) &&
        job.latest_run !== null &&
        isJobRunActive(job.latest_run.state),
    ).length,
    workspaceCount: placements.length,
  };
}

export function nextNodeRouteAfterDelete(nodes: NodeSummary[], nodeId: string) {
  const currentIndex = nodes.findIndex((node) => node.node_id === nodeId);
  const nextNode =
    nodes[currentIndex + 1] ??
    nodes[currentIndex - 1] ??
    nodes.find((node) => node.node_id !== nodeId);
  return nextNode
    ? `/nodes/${encodeURIComponent(nextNode.node_id)}`
    : "/dashboard";
}

function confirmNodeDelete(displayName: string) {
  return window.confirm(
    `Delete node "${displayName}" and its workspaces/sessions from Uprava?`,
  );
}
