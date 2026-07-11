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

  const canStart = canRunCommand("session.start", {
    placement: placement.data,
  });
  const canDelete = canRunCommand("placement.delete", {
    placement: placement.data,
  });
  const node = inventory.data?.nodes.find(
    (candidate) => candidate.node_id === placement.data.node_id,
  );
  const projectRef = projectRefForPlacement(placement.data);
  const workspaceRef = workspaceRefForPlacement(placement.data);
  const providerOptions = providerChoiceOptions(node);
  const selectedProviderLabel =
    providerOptions.find((option) => option.id === provider)?.label ?? provider;
  const selectedProviderAvailable =
    providerOptions.find((option) => option.id === provider)?.available ??
    false;
  const showProviderSelector = providerOptions.length > 1;

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
          {showProviderSelector ? (
            <div
              className="inline-flex h-10 overflow-hidden border border-[var(--color-muted)] bg-[var(--color-bg)]"
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
                      ? "bg-[var(--color-ink)] px-3 text-sm font-medium text-white disabled:bg-[var(--color-muted)]"
                      : "px-3 text-sm font-medium text-[var(--color-ink)] hover:bg-[var(--color-bg-muted)] disabled:text-[var(--color-muted)]"
                  }
                  onClick={() => setProvider(option.id)}
                >
                  {option.label}
                </button>
              ))}
            </div>
          ) : null}
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
          <Button
            variant="primary"
            disabled={
              mutation.isPending || !canStart || !selectedProviderAvailable
            }
            onClick={() => mutation.mutate()}
          >
            <Play size={16} />
            Start {selectedProviderLabel}
          </Button>
        </div>
      </div>
      {deleteMutation.isError ? (
        <ErrorNotice
          error={deleteMutation.error}
          title="Workspace delete failed"
        />
      ) : null}
      {mutation.isError ? (
        <ErrorNotice error={mutation.error} title="Session start failed" />
      ) : null}
      {refreshMutation.isError ? (
        <ErrorNotice
          error={refreshMutation.error}
          title="Workspace refresh failed"
        />
      ) : null}
      {inventory.isError ? (
        <ErrorNotice error={inventory.error} title="Inventory load failed" />
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
  return capability.value.kind === "provider" && capability.value.available;
}
