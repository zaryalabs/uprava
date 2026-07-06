import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useInventory } from "../inventory/api";
import type { ProjectPlacementSummary } from "../../shared/protocol/types";
import { queryKeys } from "../../shared/api/query-keys";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { runWorkbenchCommand } from "../../workbench/commands/registry";
import { routeForRef } from "../../workbench/references/refs";

export const DEFAULT_WORKSPACE_PATH = "/workspace";

const WORKSPACE_PATH_SUGGESTION_LIMIT = 6;

export function PlacementNewRoute() {
  const { nodeId } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const inventory = useInventory();
  const [displayName, setDisplayName] = useState("cortex");
  const [workspacePath, setWorkspacePath] = useState(DEFAULT_WORKSPACE_PATH);
  const nodePlacements =
    inventory.data?.placements.filter(
      (placement) => placement.node_id === nodeId,
    ) ?? [];
  const pathSuggestions = workspacePathSuggestions(displayName, nodePlacements);
  const mutation = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("placement.validate", {
        placementRequest: {
          node_id: nodeId ?? "",
          display_name: displayName.trim(),
          workspace_path: workspacePath.trim(),
        },
      }),
    onSuccess: async (placement) => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
      if (isPlacementResponse(placement)) {
        navigate(
          routeForRef({
            kind: "workspace",
            placement_id: placement.project_placement_id,
          }) ?? `/workspaces/${placement.project_placement_id}`,
        );
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
        <div className="space-y-1">
          <label className="block text-sm font-medium" htmlFor="display-name">
            Display name
          </label>
          <input
            id="display-name"
            className="h-10 w-full rounded-md border border-[#bfc8bc] px-3"
            value={displayName}
            onChange={(event) => setDisplayName(event.target.value)}
          />
        </div>
        <div className="space-y-1">
          <label className="block text-sm font-medium" htmlFor="workspace-path">
            Workspace path
          </label>
          <input
            aria-describedby="workspace-path-help"
            className="h-10 w-full rounded-md border border-[#bfc8bc] px-3"
            id="workspace-path"
            list="workspace-path-suggestions"
            placeholder={DEFAULT_WORKSPACE_PATH}
            value={workspacePath}
            onChange={(event) => setWorkspacePath(event.target.value)}
          />
          <datalist id="workspace-path-suggestions">
            {pathSuggestions.map((path) => (
              <option key={path} value={path} />
            ))}
          </datalist>
          <p id="workspace-path-help" className="text-xs text-[#667268]">
            Node-local path; the compose node exposes {DEFAULT_WORKSPACE_PATH}.
          </p>
          <div
            aria-label="Workspace path suggestions"
            className="flex flex-wrap gap-2"
          >
            {pathSuggestions.map((path) => (
              <button
                key={path}
                aria-label={`Use ${path}`}
                className="min-w-0 max-w-full truncate rounded-md border border-[#d9ded4] bg-[#fbfcf8] px-2.5 py-1 font-mono text-xs text-[#27362f] hover:bg-[#edf1e9]"
                title={path}
                type="button"
                onClick={() => setWorkspacePath(path)}
              >
                {path}
              </button>
            ))}
          </div>
        </div>
        {mutation.isError ? (
          <ErrorNotice
            error={mutation.error}
            title="Workspace validation failed"
          />
        ) : null}
        <Button
          variant="primary"
          disabled={
            !displayName.trim() || !workspacePath.trim() || mutation.isPending
          }
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

export function workspacePathSuggestions(
  displayName: string,
  placements: Pick<ProjectPlacementSummary, "workspace_path">[],
) {
  const pathSegment = workspacePathSegment(displayName);
  return uniquePaths([
    ...placements.map((placement) => placement.workspace_path),
    DEFAULT_WORKSPACE_PATH,
    `${DEFAULT_WORKSPACE_PATH}/${pathSegment}`,
    `~/Projects/${pathSegment}`,
    `~/work/${pathSegment}`,
    `/tmp/${pathSegment}`,
  ]).slice(0, WORKSPACE_PATH_SUGGESTION_LIMIT);
}

function workspacePathSegment(displayName: string) {
  return (
    displayName
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9._-]+/g, "-")
      .replace(/^-+|-+$/g, "") || "workspace"
  );
}

function uniquePaths(paths: string[]) {
  return Array.from(new Set(paths.map((path) => path.trim()).filter(Boolean)));
}
