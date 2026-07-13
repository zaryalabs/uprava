import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RefreshCw, Trash2 } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { WorkspaceInspector } from "../workspace-inspector/WorkspaceInspector";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import {
  projectRefForPlacement,
  routeForRef,
  workspaceRefForPlacement,
} from "../../workbench/references/refs";

export function PlacementRoute() {
  const { placementId } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const placement = useQuery({
    queryKey: queryKeys.placement(placementId ?? ""),
    queryFn: () => coreApi.placement(placementId ?? ""),
    enabled: Boolean(placementId),
  });
  const refreshMutation = useMutation({
    mutationFn: () => coreApi.refreshResourceSnapshot(placementId ?? ""),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
      await queryClient.invalidateQueries({
        queryKey: queryKeys.placement(placementId ?? ""),
      });
    },
  });
  const deleteMutation = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("placement.delete", {
        placement: placement.data,
        navigate,
        afterSuccess: async () => {
          await queryClient.invalidateQueries({
            queryKey: queryKeys.inventory,
          });
          await queryClient.invalidateQueries({
            queryKey: queryKeys.placement(placementId ?? ""),
          });
        },
      }),
  });

  if (placement.isError) {
    return (
      <ErrorNotice error={placement.error} title="Workspace load failed" />
    );
  }

  if (!placement.data) {
    return (
      <div className="text-sm text-[var(--color-muted)]">Loading workspace</div>
    );
  }

  const canDelete = canRunCommand("placement.delete", {
    placement: placement.data,
  });
  const projectRef = projectRefForPlacement(placement.data);
  const workspaceRef = workspaceRefForPlacement(placement.data);

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold">
            {placement.data.display_name}
          </h1>
          <div className="truncate text-sm text-[var(--color-muted)]">
            {placement.data.workspace_path}
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <ReferenceActions reference={workspaceRef} />
          {projectRef ? <ReferenceActions reference={projectRef} /> : null}
          <Button
            variant="secondary"
            disabled={refreshMutation.isPending}
            onClick={() => refreshMutation.mutate()}
          >
            <RefreshCw size={16} />
            Refresh
          </Button>
          <Button
            variant="danger"
            disabled={!canDelete || deleteMutation.isPending}
            onClick={() => {
              if (confirmPlacementDelete(placement.data.display_name)) {
                deleteMutation.mutate();
              }
            }}
          >
            <Trash2 size={16} />
            Delete
          </Button>
        </div>
      </div>
      {deleteMutation.isError ? (
        <ErrorNotice
          error={deleteMutation.error}
          title="Workspace delete failed"
        />
      ) : null}
      {refreshMutation.isError ? (
        <ErrorNotice
          error={refreshMutation.error}
          title="Workspace refresh failed"
        />
      ) : null}
      <div className="flex flex-wrap gap-2">
        <Badge tone="good">{placement.data.state}</Badge>
        {placement.data.resource_badges.map((badge) => (
          <Badge
            key={badge.kind}
            tone={badge.severity === "hard_block" ? "bad" : "warn"}
          >
            {badge.label}
          </Badge>
        ))}
      </div>
      <Link
        to={
          routeForRef({ kind: "node", node_id: placement.data.node_id }) ??
          `/nodes/${placement.data.node_id}`
        }
        className="text-sm underline"
      >
        Open node
      </Link>
      <WorkspaceInspector
        placementId={placement.data.project_placement_id}
        workspacePath={placement.data.workspace_path}
      />
    </section>
  );
}

function confirmPlacementDelete(displayName: string) {
  return window.confirm(
    `Delete workspace "${displayName}" and its sessions from Uprava? Local files stay on the node.`,
  );
}
