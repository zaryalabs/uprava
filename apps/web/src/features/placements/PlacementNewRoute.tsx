import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { queryKeys } from "../../shared/api/query-keys";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { runWorkbenchCommand } from "../../workbench/commands/registry";

export function PlacementNewRoute() {
  const { nodeId } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [displayName, setDisplayName] = useState("cortex");
  const [workspacePath, setWorkspacePath] = useState("");
  const mutation = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("placement.validate", {
        placementRequest: {
          node_id: nodeId ?? "",
          display_name: displayName,
          workspace_path: workspacePath,
        },
      }),
    onSuccess: async (placement) => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
      if (isPlacementResponse(placement)) {
        navigate(`/placements/${placement.project_placement_id}`);
      }
    },
  });

  return (
    <section className="max-w-2xl space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">New Workspace</h1>
        <p className="text-sm text-[#536257]">Node {nodeId}</p>
      </div>
      <form
        className="space-y-4 rounded-md border border-[#d9ded4] bg-white p-4"
        onSubmit={(event) => {
          event.preventDefault();
          if (!nodeId) return;
          mutation.mutate();
        }}
      >
        <label className="block space-y-1">
          <span className="text-sm font-medium">Display name</span>
          <input
            className="h-10 w-full rounded-md border border-[#bfc8bc] px-3"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </label>
        <label className="block space-y-1">
          <span className="text-sm font-medium">Workspace path</span>
          <input
            className="h-10 w-full rounded-md border border-[#bfc8bc] px-3"
            value={workspacePath}
            onChange={(event) => setWorkspacePath(event.target.value)}
          />
        </label>
        {mutation.isError ? (
          <ErrorNotice
            error={mutation.error}
            title="Workspace validation failed"
          />
        ) : null}
        <Button
          variant="primary"
          disabled={!workspacePath || mutation.isPending}
        >
          Validate
        </Button>
      </form>
    </section>
  );
}

function isPlacementResponse(
  value: unknown,
): value is { project_placement_id: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "project_placement_id" in value &&
    typeof value.project_placement_id === "string"
  );
}
