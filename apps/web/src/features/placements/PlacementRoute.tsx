import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Play, RefreshCw, Trash2 } from "lucide-react";
import { useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { NodeSummary } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";

export function PlacementRoute() {
  const { placementId } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [provider, setProvider] = useState<ProviderId>("codex");
  const placement = useQuery({
    queryKey: queryKeys.placement(placementId ?? ""),
    queryFn: () => coreApi.placement(placementId ?? ""),
    enabled: Boolean(placementId),
  });
  const inventory = useQuery({
    queryKey: queryKeys.inventory,
    queryFn: coreApi.inventory,
  });
  const mutation = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("session.start", {
        placement: placement.data,
        provider,
        navigate,
        afterSuccess: async () => {
          await queryClient.invalidateQueries({
            queryKey: queryKeys.inventory,
          });
        },
      }),
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

  if (!placement.data) {
    return <div className="text-sm text-[#536257]">Loading workspace</div>;
  }

  const canStart = canRunCommand("session.start", {
    placement: placement.data,
  });
  const canDelete = canRunCommand("placement.delete", {
    placement: placement.data,
  });
  const node = inventory.data?.nodes.find(
    (candidate) => candidate.node_id === placement.data.node_id,
  );
  const providerOptions = providerChoiceOptions(node);
  const selectedProviderAvailable =
    providerOptions.find((option) => option.id === provider)?.available ??
    false;

  return (
    <section className="space-y-5">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="min-w-0">
          <h1 className="text-2xl font-semibold">
            {placement.data.display_name}
          </h1>
          <div className="truncate text-sm text-[#536257]">
            {placement.data.workspace_path}
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <div
            className="inline-flex h-10 overflow-hidden rounded-md border border-[#bfc8bc] bg-white"
            role="group"
            aria-label="Provider"
          >
            {providerOptions.map((option) => (
              <button
                key={option.id}
                type="button"
                aria-pressed={provider === option.id}
                disabled={!option.available || mutation.isPending}
                className={
                  provider === option.id
                    ? "bg-[#1d4f3a] px-3 text-sm font-medium text-white disabled:bg-[#9aa8a0]"
                    : "px-3 text-sm font-medium text-[#253129] hover:bg-[#edf2ee] disabled:text-[#9aa8a0]"
                }
                onClick={() => setProvider(option.id)}
              >
                {option.label}
              </button>
            ))}
          </div>
          <ReferenceActions
            reference={{
              kind: "placement",
              placement_id: placement.data.project_placement_id,
            }}
          />
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
          <Button
            variant="primary"
            disabled={
              mutation.isPending || !canStart || !selectedProviderAvailable
            }
            onClick={() => mutation.mutate()}
          >
            <Play size={16} />
            Start
          </Button>
        </div>
      </div>
      {deleteMutation.isError ? (
        <ErrorNotice
          error={deleteMutation.error}
          title="Workspace delete failed"
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
        to={`/nodes/${placement.data.node_id}`}
        className="text-sm underline"
      >
        Open node
      </Link>
    </section>
  );
}

function confirmPlacementDelete(displayName: string) {
  return window.confirm(
    `Delete workspace "${displayName}" and its sessions from Cortex? Local files stay on the node.`,
  );
}

type ProviderId = "codex";

type ProviderChoiceOption = {
  id: ProviderId;
  label: string;
  available: boolean;
};

export function providerChoiceOptions(
  node: NodeSummary | undefined,
): ProviderChoiceOption[] {
  return [
    {
      id: "codex",
      label: "Codex",
      available: providerCapabilityAvailable(node, "codex"),
    },
  ];
}

function providerCapabilityAvailable(
  node: NodeSummary | undefined,
  provider: ProviderId,
): boolean {
  if (!node) {
    return false;
  }
  const capability = node.capabilities.find(
    (candidate) => candidate.key === `provider.${provider}`,
  );
  if (!capability) {
    return false;
  }
  return capabilityValueAvailable(capability.value);
}

function capabilityValueAvailable(value: unknown): boolean {
  if (!value || typeof value !== "object" || !("available" in value)) {
    return true;
  }
  const available = (value as { available?: unknown }).available;
  return typeof available === "boolean" ? available : true;
}
