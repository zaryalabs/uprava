import { useQuery } from "@tanstack/react-query";
import { useEffect, type ReactNode } from "react";
import {
  Link,
  Navigate,
  NavLink,
  Outlet,
  useLocation,
  useOutletContext,
  useParams,
} from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  NodeSummary,
  ProjectPlacementSummary,
  SessionSummary,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { LoadingState } from "../../shared/ui/system";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { workspaceRefForPlacement } from "../../workbench/references/refs";
import { useInventory } from "../inventory/api";
import {
  preferredWorkspaceRoute,
  rememberWorkspaceRoute,
  routeWithSearch,
  workspaceAgentRoute,
  workspaceJobsRoute,
  workspaceSurfaceFromPathname,
  workspaceWorkbenchRoute,
} from "./routes";

export type WorkspaceOutletContext = {
  placement: ProjectPlacementSummary;
  node: NodeSummary;
  sessions: SessionSummary[];
};

export function WorkspaceLayout() {
  const { placementId = "" } = useParams();
  const location = useLocation();
  const placement = useQuery({
    queryKey: queryKeys.placement(placementId),
    queryFn: () => coreApi.placement(placementId),
    enabled: Boolean(placementId),
  });
  const inventory = useInventory();
  const node = inventory.data?.nodes.find(
    (candidate) => candidate.node_id === placement.data?.node_id,
  );
  const sessions =
    inventory.data?.sessions.filter(
      (session) => session.project_placement_id === placementId,
    ) ?? [];
  const surface = workspaceSurfaceFromPathname(placementId, location.pathname);
  const nodeId = placement.data?.node_id;

  useEffect(() => {
    if (!placementId || !nodeId) return;
    rememberWorkspaceRoute(placementId, nodeId, surface);
  }, [nodeId, placementId, surface]);

  if (placement.isError) {
    return (
      <ErrorNotice error={placement.error} title="Workspace load failed" />
    );
  }
  if (inventory.isError && !inventory.data) {
    return (
      <ErrorNotice error={inventory.error} title="Node inventory failed" />
    );
  }
  if (!placement.data || !inventory.data) {
    return <LoadingState stage="Loading workspace context" />;
  }
  if (!node) {
    return (
      <div className="text-sm text-[var(--color-muted)]" role="status">
        Workspace node not found
      </div>
    );
  }

  const activeSessions = sessions.filter(
    (session) => session.state === "active",
  ).length;
  const attentionCount = placement.data.resource_badges.filter(
    (badge) => badge.severity !== "info",
  ).length;
  const context: WorkspaceOutletContext = {
    placement: placement.data,
    node,
    sessions,
  };

  return (
    <section className="space-y-6">
      <header className="border-b border-black/10 pb-5">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="min-w-0">
            <div className="zarya-caption">WORKSPACE / {node.display_name}</div>
            <h1 className="mt-2 text-2xl font-semibold">
              {placement.data.display_name}
            </h1>
            <div className="mt-1 truncate font-mono text-xs text-[var(--color-muted)]">
              {placement.data.workspace_path}
            </div>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            <Link
              className="text-sm underline"
              to={`/nodes/${encodeURIComponent(node.node_id)}`}
            >
              Open node
            </Link>
            <ReferenceActions
              reference={workspaceRefForPlacement(placement.data)}
            />
          </div>
        </div>
        <div
          className="mt-4 flex flex-wrap gap-2"
          aria-label="Workspace status"
        >
          <StatusDimension label="Presence">
            <Badge tone={node.presence === "reachable" ? "good" : "warn"}>
              {node.presence}
            </Badge>
          </StatusDimension>
          <StatusDimension label="Lifecycle">
            <Badge tone={activeSessions > 0 ? "info" : "neutral"}>
              {activeSessions} active
            </Badge>
          </StatusDimension>
          <StatusDimension label="Attention">
            <Badge tone={attentionCount > 0 ? "warn" : "neutral"}>
              {attentionCount > 0 ? `${attentionCount} signals` : "clear"}
            </Badge>
          </StatusDimension>
          <StatusDimension label="Workspace">
            <Badge
              tone={placement.data.state === "validated" ? "good" : "warn"}
            >
              {placement.data.state}
            </Badge>
          </StatusDimension>
        </div>
      </header>

      <nav
        aria-label="Workspace surfaces"
        className="flex gap-1 border-b border-black/10"
      >
        <WorkspaceTab
          to={routeWithSearch(
            workspaceAgentRoute(placementId),
            location.search,
          )}
        >
          Agent
        </WorkspaceTab>
        <WorkspaceTab
          to={routeWithSearch(
            workspaceWorkbenchRoute(placementId),
            location.search,
          )}
        >
          Workbench
        </WorkspaceTab>
        <WorkspaceTab
          to={routeWithSearch(workspaceJobsRoute(placementId), location.search)}
        >
          Jobs
        </WorkspaceTab>
      </nav>

      <Outlet context={context} />
    </section>
  );
}

export function WorkspaceResolverRoute() {
  const { placement } = useWorkspaceContext();
  const location = useLocation();
  return (
    <Navigate
      replace
      to={routeWithSearch(
        preferredWorkspaceRoute(placement.project_placement_id),
        location.search,
      )}
    />
  );
}

export function useWorkspaceContext() {
  return useOutletContext<WorkspaceOutletContext>();
}

function WorkspaceTab({ to, children }: { to: string; children: ReactNode }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        `border-b-2 px-4 py-2 text-sm ${
          isActive
            ? "border-[var(--color-ink)] font-bold"
            : "border-transparent text-[var(--color-muted)] hover:text-[var(--color-ink)]"
        }`
      }
    >
      {children}
    </NavLink>
  );
}

function StatusDimension({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <span className="inline-flex items-center gap-1 text-xs text-[var(--color-muted)]">
      {label}
      {children}
    </span>
  );
}
