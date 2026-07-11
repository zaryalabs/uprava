import { Folder, MessageSquare, Server } from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { useInventory } from "../inventory/api";
import { NodeStatusBadge } from "../nodes/NodeStatusBadge";
import type { ProjectPlacementSummary } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import {
  routeForRef,
  workspaceRefForPlacement,
} from "../../workbench/references/refs";

export function ProjectRoute() {
  const { projectId } = useParams();
  const inventory = useInventory();

  if (inventory.isLoading) {
    return (
      <div className="text-sm text-[var(--color-muted)]">Loading project</div>
    );
  }

  if (inventory.isError || !inventory.data) {
    return (
      <ErrorNotice
        error={inventory.error}
        title="Project snapshot unavailable"
      />
    );
  }

  const placements = inventory.data.placements.filter(
    (placement) => placement.project_id === projectId,
  );
  if (!projectId || placements.length === 0) {
    return (
      <div className="text-sm text-[var(--color-muted)]">Project not found</div>
    );
  }

  const placementIds = new Set(
    placements.map((placement) => placement.project_placement_id),
  );
  const sessions = inventory.data.sessions.filter((session) =>
    placementIds.has(session.project_placement_id),
  );
  const title = projectTitle(placements);

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <h1 className="truncate text-2xl font-semibold">{title}</h1>
          <div className="mt-1 break-all font-mono text-sm text-[var(--color-muted)]">
            {projectId}
          </div>
        </div>
        <ReferenceActions
          reference={{ kind: "project", project_id: projectId }}
        />
      </div>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold uppercase tracking-normal text-[var(--color-muted)]">
          Workspaces
        </h2>
        <div className="grid gap-2">
          {placements.map((placement) => {
            const node = inventory.data.nodes.find(
              (candidate) => candidate.node_id === placement.node_id,
            );
            return (
              <Link
                key={placement.project_placement_id}
                to={routeForRef(workspaceRefForPlacement(placement)) ?? "#"}
                className="border border-[var(--color-muted)] bg-[var(--color-bg)] p-3 hover:bg-[var(--color-bg-muted)]"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="flex min-w-0 items-center gap-2">
                    <Folder size={15} />
                    <span className="truncate font-medium">
                      {placement.display_name}
                    </span>
                  </span>
                  <Badge tone="info">{placement.state}</Badge>
                </div>
                <div className="mt-1 truncate text-sm text-[var(--color-muted)]">
                  {placement.workspace_path}
                </div>
                {node ? (
                  <div className="mt-2 flex items-center gap-2 text-xs text-[var(--color-muted)]">
                    <Server size={13} />
                    <span className="truncate">{node.display_name}</span>
                    <NodeStatusBadge presence={node.presence} />
                  </div>
                ) : null}
              </Link>
            );
          })}
        </div>
      </section>

      <section className="space-y-2">
        <h2 className="text-sm font-semibold uppercase tracking-normal text-[var(--color-muted)]">
          Sessions
        </h2>
        {sessions.length === 0 ? (
          <div className="border border-[var(--color-muted)] bg-[var(--color-bg)] p-3 text-sm text-[var(--color-muted)]">
            No sessions in this project
          </div>
        ) : (
          <div className="grid gap-2">
            {sessions.map((session) => (
              <Link
                key={session.session_thread_id}
                to={
                  routeForRef({
                    kind: "session",
                    session_thread_id: session.session_thread_id,
                  }) ?? "#"
                }
                className="border border-[var(--color-muted)] bg-[var(--color-bg)] p-3 hover:bg-[var(--color-bg-muted)]"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="flex min-w-0 items-center gap-2">
                    <MessageSquare size={15} />
                    <span className="truncate font-medium">
                      {session.title}
                    </span>
                  </span>
                  <span className="flex items-center gap-1">
                    <Badge tone="info">{session.state}</Badge>
                    <Badge tone="neutral">{session.runtime.state}</Badge>
                  </span>
                </div>
              </Link>
            ))}
          </div>
        )}
      </section>
    </section>
  );
}

function projectTitle(placements: ProjectPlacementSummary[]) {
  const names = new Set(placements.map((placement) => placement.display_name));
  return names.size === 1
    ? (placements[0]?.display_name ?? "Project")
    : "Project";
}
