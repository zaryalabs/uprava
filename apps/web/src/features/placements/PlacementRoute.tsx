import { useMutation, useQueryClient } from "@tanstack/react-query";
import { RefreshCw, Trash2 } from "lucide-react";
import { useNavigate } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { WorkspaceWorkbench } from "../workspace-inspector/WorkspaceWorkbench";
import { useWorkspaceContext } from "../workspaces/WorkspaceLayout";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { projectRefForPlacement } from "../../workbench/references/refs";

export function PlacementRoute() {
  const { placement } = useWorkspaceContext();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const placementId = placement.project_placement_id;
  const refreshMutation = useMutation({
    mutationFn: () => coreApi.refreshResourceSnapshot(placementId),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
      await queryClient.invalidateQueries({
        queryKey: queryKeys.placement(placementId),
      });
    },
  });
  const deleteMutation = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("placement.delete", {
        placement,
        navigate,
        afterSuccess: async () => {
          await queryClient.invalidateQueries({
            queryKey: queryKeys.inventory,
          });
          await queryClient.invalidateQueries({
            queryKey: queryKeys.placement(placementId),
          });
        },
      }),
  });

  const canDelete = canRunCommand("placement.delete", {
    placement,
  });
  const projectRef = projectRefForPlacement(placement);

  return (
    <section className="space-y-3">
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
      <WorkspaceWorkbench
        placementId={placementId}
        workspacePath={placement.workspace_path}
        actions={
          <>
            {projectRef ? <ReferenceActions reference={projectRef} /> : null}
            <Button
              variant="secondary"
              disabled={refreshMutation.isPending}
              onClick={() => refreshMutation.mutate()}
            >
              <RefreshCw size={16} />
              Refresh workspace
            </Button>
            <Button
              variant="danger"
              disabled={!canDelete || deleteMutation.isPending}
              onClick={() => {
                if (confirmPlacementDelete(placement.display_name)) {
                  deleteMutation.mutate();
                }
              }}
            >
              <Trash2 size={16} />
              Delete
            </Button>
          </>
        }
      />
    </section>
  );
}

function confirmPlacementDelete(displayName: string) {
  return window.confirm(
    `Delete workspace "${displayName}" and its sessions from Uprava? Local files stay on the node.`,
  );
}
