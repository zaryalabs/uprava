import { Folder, Monitor, MessageSquare } from "lucide-react";
import { Link, useLocation } from "react-router-dom";

import { useInventory } from "./api";
import { NodeStatusBadge } from "../nodes/NodeStatusBadge";
import type {
  InventorySnapshot,
  PlacementState,
  RuntimeSessionState,
  SessionThreadState,
  WarningSeverity,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  routeForRef,
  workspaceRefForPlacement,
} from "../../workbench/references/refs";

export function InventoryTree() {
  const inventory = useInventory();
  const location = useLocation();

  if (inventory.data) {
    return (
      <InventoryTreeContent
        snapshot={inventory.data}
        pathname={location.pathname}
        refreshError={inventory.isError ? inventory.error : null}
      />
    );
  }

  if (inventory.isLoading) {
    return (
      <nav aria-label="Inventory tree">
        <div className="text-sm text-[#667268]">Loading inventory</div>
      </nav>
    );
  }

  return (
    <nav aria-label="Inventory tree">
      <ErrorNotice error={inventory.error} title="Core API unavailable" />
    </nav>
  );
}

export function InventoryTreeContent({
  snapshot,
  pathname,
  refreshError = null,
}: {
  snapshot: InventorySnapshot;
  pathname: string;
  refreshError?: unknown;
}) {
  const { nodes, placements, sessions } = snapshot;

  return (
    <nav aria-label="Inventory tree" className="space-y-2">
      {refreshError ? (
        <ErrorNotice error={refreshError} title="Inventory refresh failed" />
      ) : null}
      <div className="px-1 text-xs font-semibold uppercase tracking-normal text-[#667268]">
        Nodes
      </div>
      {nodes.length === 0 ? (
        <div className="rounded-md border border-[#cad2c7] bg-[#f8faf5] p-3 text-sm text-[#536257]">
          No nodes registered
        </div>
      ) : null}
      {nodes.map((node) => {
        const nodePlacements = placements.filter(
          (placement) => placement.node_id === node.node_id,
        );
        const nodeRoute =
          routeForRef({ kind: "node", node_id: node.node_id }) ??
          `/nodes/${node.node_id}`;
        return (
          <div key={node.node_id} className="space-y-1">
            <Link
              to={nodeRoute}
              className={`flex min-h-9 items-center justify-between rounded-md px-2 text-sm hover:bg-[#e2e8dd] ${
                pathname === nodeRoute ? "bg-[#dfe8dc]" : ""
              }`}
            >
              <span className="flex min-w-0 items-center gap-2">
                <Monitor size={15} />
                <span className="truncate">{node.display_name}</span>
              </span>
              <span className="flex shrink-0 items-center gap-1">
                <NodeStatusBadge presence={node.presence} />
                <Badge
                  tone={node.active_runtime_count > 0 ? "info" : "neutral"}
                >
                  {node.active_runtime_count > 0 ? "active" : "idle"}
                </Badge>
                {node.sleep_hint && node.sleep_hint !== "unknown" ? (
                  <Badge tone="info">sleep {node.sleep_hint}</Badge>
                ) : null}
              </span>
            </Link>
            <div className="ml-4 space-y-1 border-l border-[#cfd8cb] pl-2">
              {nodePlacements.map((placement) => {
                const placementSessions = sessions.filter(
                  (session) =>
                    session.project_placement_id ===
                    placement.project_placement_id,
                );
                const workspaceRoute =
                  routeForRef(workspaceRefForPlacement(placement)) ??
                  `/workspaces/${placement.project_placement_id}`;
                return (
                  <div
                    key={placement.project_placement_id}
                    className="space-y-1"
                  >
                    <Link
                      to={workspaceRoute}
                      className="flex min-h-8 min-w-0 items-center justify-between gap-2 rounded-md px-2 text-sm hover:bg-[#e2e8dd]"
                    >
                      <span className="flex min-w-0 items-center gap-2">
                        <Folder size={14} />
                        <span className="truncate">
                          {placement.display_name}
                        </span>
                      </span>
                      <span className="flex shrink-0 items-center gap-1">
                        <Badge tone={placementTone(placement.state)}>
                          {placement.state}
                        </Badge>
                        {placement.resource_badges.slice(0, 2).map((badge) => (
                          <Badge
                            key={badge.kind}
                            tone={resourceTone(badge.severity)}
                          >
                            {badge.label}
                          </Badge>
                        ))}
                      </span>
                    </Link>
                    <div className="ml-4 space-y-1">
                      {placementSessions.map((session) => {
                        const sessionRoute =
                          routeForRef({
                            kind: "session",
                            session_thread_id: session.session_thread_id,
                          }) ?? `/sessions/${session.session_thread_id}`;
                        return (
                          <Link
                            key={session.session_thread_id}
                            to={sessionRoute}
                            className="flex min-h-8 min-w-0 items-center justify-between gap-2 rounded-md px-2 text-sm text-[#405047] hover:bg-[#e2e8dd]"
                          >
                            <span className="flex min-w-0 items-center gap-2">
                              <MessageSquare size={14} />
                              <span className="truncate">{session.title}</span>
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
                        );
                      })}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        );
      })}
    </nav>
  );
}

function placementTone(state: PlacementState) {
  if (state === "validated") return "good";
  if (state === "error" || state === "missing") return "bad";
  if (state === "read_only") return "warn";
  return "neutral";
}

function sessionTone(state: SessionThreadState) {
  if (state === "active") return "good";
  if (state === "degraded") return "bad";
  if (state === "detached") return "warn";
  return "neutral";
}

function runtimeTone(state: RuntimeSessionState) {
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

function resourceTone(severity: WarningSeverity) {
  if (severity === "hard_block") return "bad";
  if (severity === "warning") return "warn";
  return "info";
}
