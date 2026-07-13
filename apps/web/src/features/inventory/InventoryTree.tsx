import { ChevronDown, ChevronRight, Folder, Monitor, Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { Link, useLocation } from "react-router-dom";

import type { InventorySnapshot } from "../../shared/protocol/types";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { StatusIndicator } from "../../shared/ui/status-indicator";
import { preferredWorkspaceRoute, workspaceRoute } from "../workspaces/routes";
import { useInventory } from "./api";

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
        <div className="text-sm text-[var(--color-muted)]">
          Loading inventory
        </div>
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
  const { nodes, placements } = snapshot;
  const activeNodeId = nodeIdForPath(snapshot, pathname);
  const [expandedNodeIds, setExpandedNodeIds] = useState<Set<string>>(
    () =>
      new Set(
        activeNodeId ? [activeNodeId] : nodes[0] ? [nodes[0].node_id] : [],
      ),
  );

  useEffect(() => {
    if (!activeNodeId) return;
    setExpandedNodeIds((current) => {
      if (current.has(activeNodeId)) return current;
      return new Set([...current, activeNodeId]);
    });
  }, [activeNodeId]);

  return (
    <nav aria-label="Inventory tree" className="flex min-h-0 flex-1 flex-col">
      {refreshError ? (
        <ErrorNotice error={refreshError} title="Inventory refresh failed" />
      ) : null}
      <section aria-labelledby="nodes-navigation-heading">
        <div className="mb-1 flex min-h-8 items-center justify-between px-1 text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
          <span id="nodes-navigation-heading">Nodes</span>
          <Link
            to="/nodes/pair"
            className="inline-flex h-7 w-7 items-center justify-center border border-transparent hover:border-[var(--color-muted)] hover:bg-[var(--color-bg-muted)] hover:text-[var(--color-ink)]"
            aria-label="Add Node"
            title="Add Node"
          >
            <Plus size={14} aria-hidden="true" />
          </Link>
        </div>
        {nodes.length === 0 ? (
          <div className="border border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-3 text-sm text-[var(--color-muted)]">
            No nodes registered
          </div>
        ) : null}
        <div className="space-y-1">
          {nodes.map((node, index) => {
            const nodePlacements = placements.filter(
              (placement) => placement.node_id === node.node_id,
            );
            const nodeRoute = `/nodes/${encodeURIComponent(node.node_id)}`;
            const expanded = expandedNodeIds.has(node.node_id);
            const workspaceListId = `node-workspaces-${index}`;
            return (
              <div key={node.node_id}>
                <div
                  className={`flex min-h-9 items-center border-l ${
                    activeNodeId === node.node_id
                      ? "border-[var(--color-ink)] bg-[var(--color-bg-muted)]"
                      : "border-transparent"
                  }`}
                >
                  <button
                    type="button"
                    className="inline-flex h-8 w-7 shrink-0 items-center justify-center text-[var(--color-muted)] hover:bg-[var(--color-bg-muted)] hover:text-[var(--color-ink)]"
                    aria-controls={workspaceListId}
                    aria-expanded={expanded}
                    aria-label={`${expanded ? "Collapse" : "Expand"} ${node.display_name} workspaces`}
                    onClick={() => {
                      setExpandedNodeIds((current) => {
                        const next = new Set(current);
                        if (next.has(node.node_id)) next.delete(node.node_id);
                        else next.add(node.node_id);
                        return next;
                      });
                    }}
                  >
                    {expanded ? (
                      <ChevronDown size={14} aria-hidden="true" />
                    ) : (
                      <ChevronRight size={14} aria-hidden="true" />
                    )}
                  </button>
                  <Link
                    to={nodeRoute}
                    className="flex min-h-9 min-w-0 flex-1 items-center gap-2 pr-2 text-sm hover:bg-[var(--color-bg-muted)]"
                  >
                    <Monitor size={14} aria-hidden="true" />
                    <span className="min-w-0 flex-1 truncate">
                      {node.display_name}
                    </span>
                    <StatusIndicator
                      compact
                      dimension="presence"
                      value={node.presence}
                    />
                  </Link>
                </div>
                {expanded ? (
                  <div
                    id={workspaceListId}
                    className="ml-4 border-l border-[var(--color-muted)] pl-3"
                  >
                    {nodePlacements.length === 0 ? (
                      <div className="px-2 py-2 text-xs text-[var(--color-muted)]">
                        No workspaces
                      </div>
                    ) : null}
                    {nodePlacements.map((placement) => {
                      const placementPath = workspaceRoute(
                        placement.project_placement_id,
                      );
                      const active =
                        pathname === placementPath ||
                        pathname.startsWith(`${placementPath}/`);
                      return (
                        <Link
                          key={placement.project_placement_id}
                          to={preferredWorkspaceRoute(
                            placement.project_placement_id,
                          )}
                          className={`flex min-h-8 min-w-0 items-center gap-2 border-l px-2 text-sm hover:bg-[var(--color-bg-muted)] ${
                            active
                              ? "border-[var(--color-ink)] bg-[var(--color-bg-muted)] font-bold"
                              : "border-transparent"
                          }`}
                        >
                          <Folder size={13} aria-hidden="true" />
                          <span className="min-w-0 flex-1 truncate">
                            {placement.display_name}
                          </span>
                          <span className="flex shrink-0 items-center gap-1">
                            <StatusIndicator
                              compact
                              dimension="workspace"
                              value={placement.state}
                            />
                            {placement.resource_badges.some(
                              (badge) => badge.severity !== "info",
                            ) ? (
                              <StatusIndicator
                                compact
                                dimension="attention"
                                label={
                                  placement.resource_badges.find(
                                    (badge) => badge.severity !== "info",
                                  )?.label
                                }
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
                      );
                    })}
                  </div>
                ) : null}
              </div>
            );
          })}
        </div>
      </section>
      <div className="mt-auto px-2 pt-5 text-xs text-[var(--color-muted)]">
        {nodes.length} {nodes.length === 1 ? "node" : "nodes"} ·{" "}
        {placements.length}{" "}
        {placements.length === 1 ? "workspace" : "workspaces"}
      </div>
    </nav>
  );
}

function nodeIdForPath(snapshot: InventorySnapshot, pathname: string) {
  const directNode = snapshot.nodes.find((node) => {
    const route = `/nodes/${encodeURIComponent(node.node_id)}`;
    return pathname === route || pathname.startsWith(`${route}/`);
  });
  if (directNode) return directNode.node_id;

  const placement = snapshot.placements.find((candidate) => {
    const route = workspaceRoute(candidate.project_placement_id);
    return pathname === route || pathname.startsWith(`${route}/`);
  });
  return placement?.node_id ?? null;
}
