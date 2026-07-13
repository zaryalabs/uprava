import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Link } from "react-router-dom";
import { Plus, Trash2 } from "lucide-react";

import { useInventory } from "../inventory/api";
import { HeartbeatAge } from "./HeartbeatAge";
import { NodeEnrollmentPanel } from "./NodeEnrollmentPanel";
import { NodeStatusBadge } from "./NodeStatusBadge";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { StatusIndicator } from "../../shared/ui/status-indicator";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";

export function NodesRoute() {
  const queryClient = useQueryClient();
  const inventory = useInventory();
  const deleteNode = useMutation({
    mutationFn: (nodeId: string) =>
      runWorkbenchCommand("node.delete", {
        nodeId,
        afterSuccess: async () => {
          await queryClient.invalidateQueries({
            queryKey: queryKeys.inventory,
          });
        },
      }),
  });

  return (
    <section className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-semibold">Nodes</h1>
          <p className="text-sm text-[var(--color-muted)]">
            Registered runtime environments and current heartbeat state.
          </p>
        </div>
      </div>
      <NodeEnrollmentPanel />
      {deleteNode.isError ? (
        <ErrorNotice error={deleteNode.error} title="Node delete failed" />
      ) : null}
      <div className="grid gap-3">
        {inventory.data?.nodes.map((node) => (
          <article
            key={node.node_id}
            className="border border-[var(--color-muted)] bg-[var(--color-bg)] p-4"
          >
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div className="min-w-0">
                <Link
                  to={`/nodes/${node.node_id}`}
                  className="text-lg font-semibold hover:underline"
                >
                  {node.display_name}
                </Link>
                <div className="mt-1 text-sm text-[var(--color-muted)]">
                  heartbeat{" "}
                  <HeartbeatAge seconds={node.heartbeat_age_seconds} />
                </div>
              </div>
              <NodeStatusBadge presence={node.presence} />
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              <StatusIndicator
                dimension="lifecycle"
                value={node.active_runtime_count > 0 ? "active" : "idle"}
                label={`${node.active_runtime_count} active runtimes`}
              />
              {node.capabilities.map((capability) => (
                <Badge key={capability.key}>{capability.key}</Badge>
              ))}
            </div>
            <div className="mt-4 flex flex-wrap gap-2">
              <Link to={`/nodes/${node.node_id}/placements/new`}>
                <Button>
                  <Plus size={15} />
                  Workspace
                </Button>
              </Link>
              <Button
                variant="danger"
                disabled={
                  deleteNode.isPending ||
                  !canRunCommand("node.delete", { nodeId: node.node_id })
                }
                onClick={() => {
                  if (confirmNodeDelete(node.display_name)) {
                    deleteNode.mutate(node.node_id);
                  }
                }}
              >
                <Trash2 size={15} />
                Delete
              </Button>
            </div>
          </article>
        ))}
      </div>
      {inventory.data?.nodes.length === 0 ? (
        <div className="border border-[var(--color-muted)] bg-[var(--color-bg)] p-5 text-sm text-[var(--color-muted)]">
          Start a Node daemon and heartbeat will populate this list.
        </div>
      ) : null}
    </section>
  );
}

function confirmNodeDelete(displayName: string) {
  return window.confirm(
    `Delete node "${displayName}" and its workspaces/sessions from Uprava?`,
  );
}
