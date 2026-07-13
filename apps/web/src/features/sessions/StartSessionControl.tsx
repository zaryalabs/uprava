import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Play } from "lucide-react";
import { useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { queryKeys } from "../../shared/api/query-keys";
import type {
  NodeSummary,
  ProjectPlacementSummary,
} from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { routeWithSearch } from "../workspaces/routes";

export function StartSessionControl({
  node,
  placement,
}: {
  node: NodeSummary;
  placement: ProjectPlacementSummary;
}) {
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const [provider, setProvider] = useState<ProviderId>("codex");
  const [force, setForce] = useState(false);
  const mutation = useMutation({
    mutationFn: () =>
      runWorkbenchCommand("session.start", {
        placement,
        provider,
        force,
        navigate: (path) => navigate(routeWithSearch(path, location.search)),
        afterSuccess: async () => {
          await queryClient.invalidateQueries({
            queryKey: queryKeys.inventory,
          });
        },
      }),
  });
  const providerOptions = providerChoiceOptions(node);
  const selectedProvider =
    providerOptions.find((option) => option.id === provider) ??
    providerOptions[0];
  const canStart = canRunCommand("session.start", { placement });
  const unavailableReason = startUnavailableReasonFor({
    canStart,
    node,
    placement,
    provider,
    providerAvailable: selectedProvider?.available ?? false,
  });

  return (
    <section
      className="border-b border-black/10 pb-4"
      aria-labelledby="start-session-title"
    >
      <div className="zarya-label">NEW RUNTIME</div>
      <h3 id="start-session-title" className="mt-1 text-sm font-bold">
        Start Session
      </h3>
      {providerOptions.length > 1 ? (
        <div
          className="mt-3 grid grid-cols-2 border border-[var(--color-muted)] bg-[var(--color-bg)]"
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
                  ? "bg-[var(--color-ink)] px-2 py-2 text-xs font-medium text-white disabled:bg-[var(--color-muted)]"
                  : "px-2 py-2 text-xs font-medium hover:bg-[var(--color-bg-muted)] disabled:text-[var(--color-muted)]"
              }
              onClick={() => setProvider(option.id)}
            >
              {option.label}
            </button>
          ))}
        </div>
      ) : null}
      <Button
        className="mt-3 w-full"
        variant="primary"
        disabled={
          mutation.isPending || !canStart || !selectedProvider?.available
        }
        onClick={() => mutation.mutate()}
      >
        <Play size={15} aria-hidden="true" />
        {mutation.isPending
          ? "Starting…"
          : `Start ${selectedProvider?.label ?? provider}`}
      </Button>
      <label className="mt-3 flex items-start gap-2 text-xs text-[var(--color-muted)]">
        <input
          className="mt-0.5"
          type="checkbox"
          checked={force}
          disabled={mutation.isPending}
          onChange={(event) => setForce(event.target.checked)}
        />
        Force start at 5% or less provider quota
      </label>
      {unavailableReason ? (
        <div
          role="status"
          className="mt-3 border-l-2 border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-2 text-xs"
        >
          <div className="font-bold">Start unavailable</div>
          <div className="mt-1 break-words">{unavailableReason}</div>
        </div>
      ) : null}
      {mutation.isError ? (
        <div className="mt-3">
          <ErrorNotice error={mutation.error} title="Session start failed" />
        </div>
      ) : null}
    </section>
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
  if (!node) return false;
  const capability = node.capabilities.find(
    (candidate) => candidate.key === `provider.${provider}`,
  );
  return Boolean(
    capability?.value.kind === "provider" && capability.value.available,
  );
}

export function startUnavailableReasonFor({
  canStart,
  node,
  placement,
  provider,
  providerAvailable,
}: {
  canStart: boolean;
  node: NodeSummary | undefined;
  placement: {
    state: string;
    resource_badges: { label: string; severity: string }[];
  };
  provider: ProviderId;
  providerAvailable: boolean;
}): string | null {
  if (!canStart) {
    if (placement.state !== "validated") {
      return "Validate this workspace before starting Codex.";
    }
    const blockers = placement.resource_badges
      .filter((badge) => badge.severity === "hard_block")
      .map((badge) => badge.label);
    if (blockers.length > 0) {
      return `Clear workspace blockers before starting Codex: ${blockers.join(", ")}.`;
    }
  }

  if (providerAvailable) return null;
  if (!node) {
    return "Waiting for the node capability report. Refresh the workspace after the node reconnects.";
  }

  const capability = node.capabilities.find(
    (candidate) => candidate.key === `provider.${provider}`,
  );
  if (!capability || capability.value.kind !== "provider") {
    return "This node has not advertised Codex support. Verify that the Node Daemon is connected and refresh the workspace.";
  }
  if (capability.value.unavailable_reason === "binary_not_found") {
    return "Codex is not available to the Node Daemon. Install it for the daemon user, or set UPRAVA_CODEX_BINARY to its absolute path, then restart uprava-node.";
  }
  return capability.value.unavailable_reason
    ? `Codex is unavailable on this node: ${capability.value.unavailable_reason}.`
    : "Codex is unavailable on this node. Check the Node Daemon configuration and refresh the workspace.";
}
