import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { FolderPlus, KeyRound, ShieldOff, Trash2 } from "lucide-react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { useInventory } from "../inventory/api";
import { HeartbeatAge } from "./HeartbeatAge";
import { NodeStatusBadge } from "./NodeStatusBadge";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import type { NodeCredentialRotationResponse } from "../../shared/protocol/types";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";

export function NodeDetailRoute() {
  const { nodeId } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [rotatedCredential, setRotatedCredential] =
    useState<NodeCredentialRotationResponse | null>(null);
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
  const deleteNode = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("node.delete", {
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
          navigate("/nodes");
        },
      }),
  });
  const rotateCredential = useMutation({
    mutationFn: async () => {
      const result = await runWorkbenchCommand("node.rotateCredential", {
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
      });
      return result as NodeCredentialRotationResponse;
    },
    onSuccess: setRotatedCredential,
  });

  if (!node) {
    return <div className="text-sm text-[#536257]">Node not found</div>;
  }

  const canRevoke = canRunCommand("node.revoke", { nodeId: node.node_id });
  const canRotate = canRunCommand("node.rotateCredential", {
    nodeId: node.node_id,
  });
  const canDelete = canRunCommand("node.delete", { nodeId: node.node_id });

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
            disabled={!canRotate || rotateCredential.isPending}
            onClick={() => rotateCredential.mutate()}
          >
            <KeyRound size={16} />
            Rotate
          </Button>
          <Button
            variant="danger"
            disabled={!canRevoke || revokeNode.isPending}
            onClick={() => revokeNode.mutate()}
          >
            <ShieldOff size={16} />
            Revoke
          </Button>
          <Button
            variant="danger"
            disabled={!canDelete || deleteNode.isPending}
            onClick={() => {
              if (confirmNodeDelete(node.display_name)) {
                deleteNode.mutate();
              }
            }}
          >
            <Trash2 size={16} />
            Delete
          </Button>
        </div>
      </div>
      {revokeNode.isError ? (
        <ErrorNotice error={revokeNode.error} title="Node revoke failed" />
      ) : null}
      {rotateCredential.isError ? (
        <ErrorNotice
          error={rotateCredential.error}
          title="Credential rotation failed"
        />
      ) : null}
      {rotatedCredential ? (
        <section className="rounded-md border border-[#bfd8ce] bg-[#e3f4ed] p-3 text-sm text-[#173f35]">
          <div className="font-medium">New node credential</div>
          <div className="mt-2 overflow-x-auto rounded-md bg-white px-2 py-1 font-mono text-xs">
            {rotatedCredential.credential}
          </div>
        </section>
      ) : null}
      {deleteNode.isError ? (
        <ErrorNotice error={deleteNode.error} title="Node delete failed" />
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

function confirmNodeDelete(displayName: string) {
  return window.confirm(
    `Delete node "${displayName}" and its workspaces/sessions from Cortex?`,
  );
}
