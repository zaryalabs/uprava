import { useMutation, useQueryClient } from "@tanstack/react-query";
import { FolderPlus, ShieldOff } from "lucide-react";
import { Link, useParams } from "react-router-dom";

import { useInventory } from "../inventory/api";
import { HeartbeatAge } from "./HeartbeatAge";
import { NodeStatusBadge } from "./NodeStatusBadge";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";

export function NodeDetailRoute() {
  const { nodeId } = useParams();
  const queryClient = useQueryClient();
  const inventory = useInventory();
  const node = inventory.data?.nodes.find(
    (candidate) => candidate.node_id === nodeId,
  );
  const placements = inventory.data?.placements.filter(
    (placement) => placement.node_id === nodeId,
  );
  const revokeNode = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("node.revoke", {
        nodeId,
        afterSuccess: async () => {
          await queryClient.invalidateQueries({
            queryKey: queryKeys.inventory,
          });
          if (nodeId) {
            await queryClient.invalidateQueries({
              queryKey: queryKeys.node(nodeId),
            });
          }
        },
      }),
  });

  if (!node) {
    return <div className="text-sm text-[#536257]">Node not found</div>;
  }

  const canRevoke = canRunCommand("node.revoke", { nodeId: node.node_id });

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">{node.display_name}</h1>
          <div className="mt-1 flex flex-wrap items-center gap-2 text-sm text-[#536257]">
            <NodeStatusBadge presence={node.presence} />
            <span>
              heartbeat <HeartbeatAge seconds={node.heartbeat_age_seconds} />
            </span>
            <span>sleep hint {node.sleep_hint}</span>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <ReferenceActions
            reference={{ kind: "node", node_id: node.node_id }}
          />
          <Link to={`/nodes/${node.node_id}/placements/new`}>
            <Button>
              <FolderPlus size={16} />
              Workspace
            </Button>
          </Link>
          <Button
            variant="danger"
            disabled={!canRevoke || revokeNode.isPending}
            onClick={() => revokeNode.mutate()}
          >
            <ShieldOff size={16} />
            Revoke
          </Button>
        </div>
      </div>
      {revokeNode.isError ? (
        <ErrorNotice error={revokeNode.error} title="Node revoke failed" />
      ) : null}
      <section className="space-y-2">
        <h2 className="text-sm font-semibold uppercase tracking-normal text-[#667268]">
          Capabilities
        </h2>
        <div className="flex flex-wrap gap-2">
          {node.capabilities.map((capability) => (
            <Badge key={capability.key}>{capability.key}</Badge>
          ))}
        </div>
      </section>
      <section className="space-y-2">
        <h2 className="text-sm font-semibold uppercase tracking-normal text-[#667268]">
          Workspaces
        </h2>
        <div className="grid gap-2">
          {placements?.map((placement) => (
            <Link
              key={placement.project_placement_id}
              to={`/placements/${placement.project_placement_id}`}
              className="rounded-md border border-[#d9ded4] bg-white p-3 hover:bg-[#fbfcf8]"
            >
              <div className="font-medium">{placement.display_name}</div>
              <div className="truncate text-sm text-[#536257]">
                {placement.workspace_path}
              </div>
            </Link>
          ))}
        </div>
      </section>
    </section>
  );
}
