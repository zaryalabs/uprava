import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { ArrowUpRight, Cable, Eye, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { Link, useSearchParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  IntegrationConnectionSummary,
  McpDependencyStatus,
  NodeSummary,
  ProjectPlacementSummary,
  ToolAvailability,
  ToolCallDetail,
  ToolCallSummary,
  ToolDefinition,
} from "../../shared/protocol/types";
import { Badge, type BadgeTone } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  EmptyState,
  LoadingState,
  PageHeader,
  Surface,
} from "../../shared/ui/system";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { useInventory } from "../inventory/api";
import { workspaceAgentSessionRoute } from "../workspaces/routes";

export function ToolingRoute() {
  const inventory = useInventory();
  const [searchParams, setSearchParams] = useSearchParams();
  const queryClient = useQueryClient();
  const [pendingDisconnect, setPendingDisconnect] = useState<string | null>(
    null,
  );
  const [authorization, setAuthorization] = useState<{
    integrationId: string;
    url: string;
    expiresAt: string;
  } | null>(null);
  const placement = selectedPlacement(
    inventory.data?.placements ?? [],
    searchParams.get("placement"),
  );
  const sessions = (inventory.data?.sessions ?? []).filter(
    (session) =>
      session.project_placement_id === placement?.project_placement_id,
  );
  const session =
    sessions.find(
      (candidate) =>
        candidate.session_thread_id === searchParams.get("session"),
    ) ?? sessions[0];
  const scopeKey = session?.session_thread_id ?? "global";

  const definitions = useQuery({
    queryKey: queryKeys.toolDefinitions,
    queryFn: coreApi.toolDefinitions,
  });
  const integrations = useQuery({
    queryKey: queryKeys.integrationConnections,
    queryFn: coreApi.integrationConnections,
    refetchInterval: (query) =>
      query.state.data?.items.some(
        (connection) => connection.auth_state === "connecting",
      )
        ? 2_000
        : false,
  });
  const dependencies = useQuery({
    queryKey: queryKeys.mcpDependencies,
    queryFn: coreApi.mcpDependencies,
    refetchInterval: (query) =>
      query.state.data?.items.some((dependency) =>
        ["installing", "starting"].includes(dependency.actual_state),
      )
        ? 2_000
        : false,
  });
  const availability = useQuery({
    queryKey: queryKeys.toolAvailability(session?.session_thread_id ?? "none"),
    queryFn: () =>
      coreApi.toolAvailability({
        nodeId: placement!.node_id,
        projectId: placement!.project_id,
        placementId: placement!.project_placement_id,
        sessionThreadId: session!.session_thread_id,
      }),
    enabled: Boolean(placement && session),
  });
  const calls = useQuery({
    queryKey: queryKeys.toolCalls(scopeKey),
    queryFn: () =>
      coreApi.toolCalls(
        placement && session
          ? {
              nodeId: placement.node_id,
              projectId: placement.project_id,
              placementId: placement.project_placement_id,
              sessionThreadId: session.session_thread_id,
            }
          : {},
      ),
  });
  const observedQueries = useQueries({
    queries: (inventory.data?.nodes ?? []).map((node) => ({
      queryKey: queryKeys.observedCapabilities(node.node_id),
      queryFn: () => coreApi.observedCapabilities(node.node_id),
    })),
  });
  const selectedToolId =
    searchParams.get("tool") ?? definitions.data?.items[0]?.tool_id ?? null;
  const selectedTool = definitions.data?.items.find(
    (definition) => definition.tool_id === selectedToolId,
  );
  const selectedCallId = searchParams.get("call");
  const selectedCall = useQuery({
    queryKey: queryKeys.toolCall(selectedCallId ?? "none"),
    queryFn: () => coreApi.toolCall(selectedCallId!),
    enabled: Boolean(selectedCallId),
  });
  const disconnect = useMutation({
    mutationFn: coreApi.disconnectIntegration,
    onSuccess: async () => {
      setPendingDisconnect(null);
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: queryKeys.integrationConnections,
        }),
        queryClient.invalidateQueries({ queryKey: queryKeys.mcpDependencies }),
        queryClient.invalidateQueries({
          queryKey: ["tooling", "availability"],
        }),
      ]);
    },
  });
  const connect = useMutation({
    mutationFn: coreApi.connectIntegration,
    onSuccess: async (response) => {
      setAuthorization({
        integrationId: response.connection.integration_id,
        url: response.authorization_url,
        expiresAt: response.expires_at,
      });
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: queryKeys.integrationConnections,
        }),
        queryClient.invalidateQueries({ queryKey: queryKeys.mcpDependencies }),
        queryClient.invalidateQueries({
          queryKey: ["tooling", "availability"],
        }),
      ]);
    },
  });

  const requestConnect = (integrationId: string, nodeId: string) => {
    setAuthorization(null);
    connect.mutate({
      integration_id: integrationId,
      project_id: placement?.project_id ?? null,
      node_id: nodeId,
    });
  };

  const setParam = (name: string, value: string | null) => {
    const next = new URLSearchParams(searchParams);
    if (value) next.set(name, value);
    else next.delete(name);
    if (name === "placement") {
      next.delete("session");
      next.delete("call");
    }
    setSearchParams(next, { replace: true });
  };

  return (
    <section>
      <PageHeader
        title="Agent Tooling"
        description="Manage MCP integrations, inspect effective capabilities, and follow redacted tool-call traces. Managed tools and observed native capabilities remain deliberately separate."
        meta="CONTROL PLANE / TOOL REGISTRY"
      />

      {inventory.isError ? (
        <ErrorNotice
          error={inventory.error}
          title="Tooling scope unavailable"
        />
      ) : (
        <ScopeSelector
          placements={inventory.data?.placements ?? []}
          placement={placement}
          sessions={sessions}
          sessionId={session?.session_thread_id ?? null}
          onPlacement={(value) => setParam("placement", value)}
          onSession={(value) => setParam("session", value)}
        />
      )}

      <div className="mt-8 grid gap-8 xl:grid-cols-2">
        <Surface aria-labelledby="integration-management-title">
          <SectionHeading
            id="integration-management-title"
            icon={<Cable size={16} aria-hidden="true" />}
            title="Integrations"
            detail="Desired, authentication and Node runtime state"
          />
          {integrations.isError || dependencies.isError ? (
            <ErrorNotice
              error={integrations.error ?? dependencies.error}
              title="Integration state unavailable"
            />
          ) : !integrations.data || !dependencies.data ? (
            <LoadingState stage="Loading integration state" />
          ) : integrations.data.items.length === 0 ? (
            <UnconfiguredLinearCard
              nodeId={placement?.node_id ?? null}
              connecting={connect.isPending}
              error={connect.error}
              authorization={authorization}
              onConnect={(nodeId) =>
                requestConnect("integration-linear", nodeId)
              }
            />
          ) : (
            <div className="divide-y divide-[var(--color-border)]">
              {integrations.data.items.map((connection) => (
                <IntegrationCard
                  key={connection.integration_id}
                  connection={connection}
                  dependency={dependencies.data.items.find(
                    (item) => item.integration_id === connection.integration_id,
                  )}
                  node={inventory.data?.nodes.find(
                    (item) => item.node_id === connection.node_id,
                  )}
                  pendingDisconnect={
                    pendingDisconnect === connection.integration_id
                  }
                  disconnecting={
                    disconnect.isPending &&
                    disconnect.variables === connection.integration_id
                  }
                  error={
                    (disconnect.variables === connection.integration_id
                      ? disconnect.error
                      : null) ??
                    (connect.variables?.integration_id ===
                    connection.integration_id
                      ? connect.error
                      : null)
                  }
                  connecting={
                    connect.isPending &&
                    connect.variables?.integration_id ===
                      connection.integration_id
                  }
                  authorization={
                    authorization?.integrationId === connection.integration_id
                      ? authorization
                      : null
                  }
                  onConnect={() =>
                    requestConnect(
                      connection.integration_id,
                      connection.node_id ?? placement?.node_id ?? "",
                    )
                  }
                  onRequestDisconnect={() =>
                    setPendingDisconnect(connection.integration_id)
                  }
                  onCancelDisconnect={() => setPendingDisconnect(null)}
                  onDisconnect={() =>
                    disconnect.mutate(connection.integration_id)
                  }
                />
              ))}
            </div>
          )}
        </Surface>

        <Surface aria-labelledby="observed-capabilities-title">
          <SectionHeading
            id="observed-capabilities-title"
            icon={<Eye size={16} aria-hidden="true" />}
            title="Observed capabilities"
            detail="Informational Node inventory; execution remains native"
          />
          {!inventory.data ? (
            <LoadingState stage="Loading Node inventory" />
          ) : inventory.data.nodes.length === 0 ? (
            <EmptyState
              title="No Nodes"
              detail="Observed git, gh and glab capabilities appear after a Node heartbeat."
            />
          ) : (
            <div className="divide-y divide-[var(--color-border)]">
              {inventory.data.nodes.map((node, index) => {
                const query = observedQueries[index];
                return (
                  <article key={node.node_id} className="py-4 first:pt-0">
                    <div className="flex flex-wrap items-center justify-between gap-2">
                      <div>
                        <h3 className="text-sm font-bold">
                          {node.display_name}
                        </h3>
                        <p className="mt-1 font-mono text-xs text-[var(--color-muted)]">
                          {node.node_id}
                        </p>
                      </div>
                      <Badge
                        tone={node.presence === "reachable" ? "good" : "bad"}
                      >
                        {node.presence}
                      </Badge>
                    </div>
                    {query.isError ? (
                      <div className="mt-3">
                        <ErrorNotice
                          error={query.error}
                          title="Capability inventory unavailable"
                        />
                      </div>
                    ) : !query.data ? (
                      <LoadingState stage="Loading capabilities" />
                    ) : query.data.items.length === 0 ? (
                      <EmptyState title="No observed native capabilities" />
                    ) : (
                      <ul className="mt-3 grid gap-2 sm:grid-cols-3">
                        {query.data.items.map((capability) => (
                          <li
                            key={capability.capability_key}
                            className="border border-[var(--color-border)] p-3 text-xs"
                          >
                            <div className="flex items-center justify-between gap-2">
                              <span className="font-bold">
                                {capability.display_name}
                              </span>
                              <Badge tone={stateTone(capability.state)}>
                                observed
                              </Badge>
                            </div>
                            <div className="mt-2 text-[var(--color-muted)]">
                              {capability.version ?? "version unknown"}
                            </div>
                            <div className="mt-1 text-[var(--color-muted)]">
                              auth:{" "}
                              {capability.safe_authentication_state ??
                                "not reported"}
                            </div>
                          </li>
                        ))}
                      </ul>
                    )}
                  </article>
                );
              })}
            </div>
          )}
          <p className="mt-4 border-l-2 border-[var(--color-notice)] pl-3 text-xs text-[var(--color-muted)]">
            Uprava observes these binaries but does not proxy their execution or
            claim managed-tool trace coverage.
          </p>
        </Surface>
      </div>

      <div className="mt-8 grid gap-8 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.8fr)]">
        <Surface aria-labelledby="managed-capabilities-title">
          <SectionHeading
            id="managed-capabilities-title"
            icon={<ShieldCheck size={16} aria-hidden="true" />}
            title="Managed capabilities"
            detail="Human Inspect view for the selected session scope"
          />
          {definitions.isError || availability.isError ? (
            <ErrorNotice
              error={definitions.error ?? availability.error}
              title="Managed capabilities unavailable"
            />
          ) : !definitions.data ? (
            <LoadingState stage="Loading Tool Registry" />
          ) : (
            <div className="grid gap-5 lg:grid-cols-[minmax(14rem,0.7fr)_minmax(0,1.3fr)]">
              <ul className="divide-y divide-[var(--color-border)] border-y border-[var(--color-border)]">
                {definitions.data.items.map((definition) => {
                  const effective = availability.data?.items.find(
                    (item) => item.tool_id === definition.tool_id,
                  );
                  return (
                    <li key={definition.tool_id}>
                      <button
                        type="button"
                        className="flex w-full items-start justify-between gap-3 px-2 py-3 text-left hover:bg-[var(--color-bg-muted)]"
                        aria-current={
                          selectedToolId === definition.tool_id
                            ? "true"
                            : undefined
                        }
                        onClick={() => setParam("tool", definition.tool_id)}
                      >
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-bold">
                            {definition.display_name}
                          </span>
                          <span className="mt-1 block truncate font-mono text-xs text-[var(--color-muted)]">
                            {definition.tool_id}
                          </span>
                        </span>
                        <Badge tone={availabilityTone(effective)}>
                          {effective?.state ?? "select scope"}
                        </Badge>
                      </button>
                    </li>
                  );
                })}
              </ul>
              {selectedTool ? (
                <ToolInspectDetail
                  definition={selectedTool}
                  availability={availability.data?.items.find(
                    (item) => item.tool_id === selectedTool.tool_id,
                  )}
                />
              ) : (
                <EmptyState title="Select a managed tool" />
              )}
            </div>
          )}
        </Surface>

        <Surface aria-labelledby="tool-call-trace-title">
          <SectionHeading
            id="tool-call-trace-title"
            icon={<ArrowUpRight size={16} aria-hidden="true" />}
            title="Recent tool calls"
            detail={
              session
                ? `Current session · ${session.session_thread_id}`
                : "No session selected · recent global calls"
            }
          />
          {calls.isError ? (
            <ErrorNotice error={calls.error} title="Tool calls unavailable" />
          ) : !calls.data ? (
            <LoadingState stage="Loading tool-call journal" />
          ) : calls.data.items.length === 0 ? (
            <EmptyState
              title="No tool calls in this scope"
              detail="Search, Inspect and Execute activity will appear after an agent uses Uprava MCP."
            />
          ) : (
            <ol className="divide-y divide-[var(--color-border)] border-y border-[var(--color-border)]">
              {calls.data.items.map((call) => (
                <li key={call.tool_call_id}>
                  <button
                    type="button"
                    className="grid w-full gap-2 px-2 py-3 text-left hover:bg-[var(--color-bg-muted)] sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center"
                    aria-current={
                      selectedCallId === call.tool_call_id ? "true" : undefined
                    }
                    onClick={() => setParam("call", call.tool_call_id)}
                  >
                    <span className="min-w-0">
                      <span className="block truncate text-sm font-bold">
                        {call.tool_id}
                      </span>
                      <span className="mt-1 block truncate text-xs text-[var(--color-muted)]">
                        {actorLabel(call)} · {call.route} ·{" "}
                        {formatDateTime(call.requested_at)}
                      </span>
                    </span>
                    <Badge tone={toolCallTone(call.state)}>{call.state}</Badge>
                  </button>
                </li>
              ))}
            </ol>
          )}
          {selectedCallId ? (
            <div className="mt-5 border-t border-[var(--color-border)] pt-5">
              {selectedCall.isError ? (
                <ErrorNotice
                  error={selectedCall.error}
                  title="Tool-call detail unavailable"
                />
              ) : selectedCall.data ? (
                <ToolCallDetailPanel detail={selectedCall.data} />
              ) : (
                <LoadingState stage="Loading redacted tool-call detail" />
              )}
            </div>
          ) : null}
        </Surface>
      </div>
    </section>
  );
}

function ScopeSelector({
  placements,
  placement,
  sessions,
  sessionId,
  onPlacement,
  onSession,
}: {
  placements: ProjectPlacementSummary[];
  placement: ProjectPlacementSummary | undefined;
  sessions: Array<{ session_thread_id: string; title: string }>;
  sessionId: string | null;
  onPlacement: (value: string) => void;
  onSession: (value: string) => void;
}) {
  return (
    <fieldset className="grid gap-4 border border-[var(--color-border)] p-4 md:grid-cols-2">
      <legend className="px-2 text-xs font-bold uppercase tracking-normal">
        Effective scope
      </legend>
      <label
        className="grid gap-1 text-xs font-bold"
        htmlFor="tooling-placement"
      >
        Workspace
        <select
          id="tooling-placement"
          className="h-10 border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 text-sm font-normal"
          value={placement?.project_placement_id ?? ""}
          onChange={(event) => onPlacement(event.target.value)}
        >
          {placements.length === 0 ? (
            <option value="">No workspace available</option>
          ) : null}
          {placements.map((item) => (
            <option
              key={item.project_placement_id}
              value={item.project_placement_id}
            >
              {item.display_name} · {item.node_id}
            </option>
          ))}
        </select>
      </label>
      <label className="grid gap-1 text-xs font-bold" htmlFor="tooling-session">
        Session
        <select
          id="tooling-session"
          className="h-10 border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 text-sm font-normal"
          value={sessionId ?? ""}
          disabled={sessions.length === 0}
          onChange={(event) => onSession(event.target.value)}
        >
          {sessions.length === 0 ? (
            <option value="">No session in this workspace</option>
          ) : null}
          {sessions.map((item) => (
            <option key={item.session_thread_id} value={item.session_thread_id}>
              {item.title} · {item.session_thread_id}
            </option>
          ))}
        </select>
      </label>
    </fieldset>
  );
}

function SectionHeading({
  id,
  icon,
  title,
  detail,
}: {
  id: string;
  icon: React.ReactNode;
  title: string;
  detail: string;
}) {
  return (
    <header className="mb-5 flex items-start gap-3">
      <span className="mt-0.5 grid h-7 w-7 shrink-0 place-items-center border border-[var(--color-border-strong)]">
        {icon}
      </span>
      <div>
        <h2 id={id} className="text-base font-bold">
          {title}
        </h2>
        <p className="mt-1 text-xs text-[var(--color-muted)]">{detail}</p>
      </div>
    </header>
  );
}

function UnconfiguredLinearCard({
  nodeId,
  connecting,
  error,
  authorization,
  onConnect,
}: {
  nodeId: string | null;
  connecting: boolean;
  error: unknown;
  authorization: AuthorizationPrompt | null;
  onConnect: (nodeId: string) => void;
}) {
  return (
    <article className="border border-dashed border-[var(--color-muted)] p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h3 className="text-sm font-bold">Linear remote MCP</h3>
        <span className="flex gap-2">
          <Badge>not configured</Badge>
          <Badge tone="warn">auth unavailable</Badge>
        </span>
      </div>
      <p className="mt-3 text-sm text-[var(--color-muted)]">
        Start the pinned ToolHive OAuth flow on the selected Node. Uprava keeps
        the authorization URL ephemeral and never stores its state parameter.
      </p>
      <Button
        type="button"
        className="mt-4"
        disabled={!nodeId || connecting}
        onClick={() => {
          if (nodeId) onConnect(nodeId);
        }}
      >
        {connecting ? "Starting authorization…" : "Connect Linear"}
      </Button>
      {!nodeId ? (
        <p className="mt-2 text-xs text-[var(--color-risk)]">
          Select a workspace with a reachable Node first.
        </p>
      ) : null}
      {error ? (
        <div className="mt-3">
          <ErrorNotice error={error} title="Linear connect failed" />
        </div>
      ) : null}
      <AuthorizationLink authorization={authorization} />
    </article>
  );
}

type AuthorizationPrompt = {
  integrationId: string;
  url: string;
  expiresAt: string;
};

export function IntegrationCard({
  connection,
  dependency,
  node,
  pendingDisconnect,
  disconnecting,
  connecting,
  authorization,
  error,
  onConnect,
  onRequestDisconnect,
  onCancelDisconnect,
  onDisconnect,
}: {
  connection: IntegrationConnectionSummary;
  dependency?: McpDependencyStatus;
  node?: NodeSummary;
  pendingDisconnect: boolean;
  disconnecting: boolean;
  connecting: boolean;
  authorization: AuthorizationPrompt | null;
  error: unknown;
  onConnect: () => void;
  onRequestDisconnect: () => void;
  onCancelDisconnect: () => void;
  onDisconnect: () => void;
}) {
  const disconnected =
    connection.desired_state === "disabled" &&
    connection.auth_state === "disconnected";
  return (
    <article className="py-4 first:pt-0">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h3 className="text-sm font-bold">{connection.display_name}</h3>
          <p className="mt-1 font-mono text-xs text-[var(--color-muted)]">
            {connection.integration_id}
          </p>
        </div>
        <span className="flex flex-wrap gap-2">
          <Badge
            tone={connection.desired_state === "enabled" ? "good" : "neutral"}
          >
            desired {connection.desired_state}
          </Badge>
          <Badge tone={authTone(connection.auth_state)}>
            auth {connection.auth_state}
          </Badge>
          <Badge tone={dependencyTone(dependency?.actual_state)}>
            runtime {dependency?.actual_state ?? "not reported"}
          </Badge>
        </span>
      </div>
      <dl className="mt-4 grid gap-3 text-xs sm:grid-cols-2">
        <LabeledValue
          label="Node"
          value={
            node
              ? `${node.display_name} · ${node.presence}`
              : (connection.node_id ?? "not assigned")
          }
        />
        <LabeledValue
          label="Authenticated actor"
          value={connection.authenticated_actor_label ?? "not disclosed"}
        />
        <LabeledValue
          label="ToolHive"
          value={
            dependency?.runtime_version
              ? `${dependency.runtime_name} ${dependency.runtime_version}`
              : (dependency?.runtime_name ?? "not reported")
          }
        />
        <LabeledValue
          label="Updated"
          value={formatDateTime(connection.updated_at)}
        />
      </dl>
      <p className="mt-4 border-l-2 border-[var(--color-notice)] pl-3 text-xs text-[var(--color-muted)]">
        {integrationDiagnostic(connection, dependency, node)}
      </p>
      {error ? (
        <div className="mt-3">
          <ErrorNotice error={error} title="Integration action failed" />
        </div>
      ) : null}
      <div className="mt-4 flex flex-wrap gap-2">
        <Button
          type="button"
          disabled={!connection.node_id || connecting || pendingDisconnect}
          onClick={onConnect}
        >
          {connecting
            ? "Starting authorization…"
            : disconnected
              ? "Connect"
              : "Reconnect"}
        </Button>
        {pendingDisconnect ? (
          <>
            <Button
              type="button"
              variant="danger"
              disabled={disconnecting}
              onClick={onDisconnect}
            >
              {disconnecting ? "Disconnecting…" : "Confirm disconnect"}
            </Button>
            <Button type="button" onClick={onCancelDisconnect}>
              Cancel
            </Button>
          </>
        ) : (
          <Button
            type="button"
            variant="danger"
            disabled={disconnected}
            onClick={onRequestDisconnect}
          >
            Disconnect
          </Button>
        )}
      </div>
      {pendingDisconnect ? (
        <p className="mt-2 text-xs text-[var(--color-risk)]" role="status">
          Disconnect immediately disables effective availability. Remote OAuth
          revocation is reported separately from local ToolHive cleanup.
        </p>
      ) : null}
      <AuthorizationLink authorization={authorization} />
    </article>
  );
}

function AuthorizationLink({
  authorization,
}: {
  authorization: AuthorizationPrompt | null;
}) {
  if (!authorization) return null;
  return (
    <div
      className="mt-4 border-l-2 border-[var(--color-notice)] pl-3 text-xs"
      role="status"
    >
      <a
        href={authorization.url}
        target="_blank"
        rel="noreferrer noopener"
        className="font-bold underline underline-offset-4"
      >
        Continue authorization in Linear
      </a>
      <p className="mt-1 text-[var(--color-muted)]">
        This one-time link expires {formatDateTime(authorization.expiresAt)}.
        Connection status refreshes automatically after consent.
      </p>
    </div>
  );
}

function ToolInspectDetail({
  definition,
  availability,
}: {
  definition: ToolDefinition;
  availability?: ToolAvailability;
}) {
  return (
    <article aria-label={`${definition.display_name} Inspect detail`}>
      <div className="flex flex-wrap items-center gap-2">
        <Badge tone="info">managed tool</Badge>
        <Badge tone={availabilityTone(availability)}>
          {availability?.state ?? "scope unavailable"}
        </Badge>
        <Badge tone={riskTone(definition.risk_level)}>
          {definition.risk_level.replaceAll("_", " ")}
        </Badge>
      </div>
      <h3 className="mt-4 text-lg font-bold">{definition.display_name}</h3>
      <p className="mt-2 text-sm text-[var(--color-muted)]">
        {definition.short_description}
      </p>
      <dl className="mt-5 grid gap-3 text-xs sm:grid-cols-2">
        <LabeledValue
          label="Source"
          value={`${definition.source_kind} · ${definition.source_id}`}
        />
        <LabeledValue
          label="Schema version"
          value={`v${definition.version} · ${definition.schema_hash}`}
        />
        <LabeledValue label="Execution" value={definition.execution_kind} />
        <LabeledValue
          label="Approval policy"
          value={definition.approval_policy}
        />
        <LabeledValue
          label="Permissions"
          value={definition.required_permissions.join(", ") || "none"}
        />
        <LabeledValue
          label="Availability reason"
          value={availabilityReasonLabel(availability?.reason ?? null)}
        />
      </dl>
      <details className="mt-5 border border-[var(--color-border)] p-3 text-xs">
        <summary className="cursor-pointer font-bold">Input schema</summary>
        <pre className="mt-3 max-h-72 overflow-auto whitespace-pre-wrap break-all font-mono text-[var(--color-muted)]">
          {JSON.stringify(definition.input_schema, null, 2)}
        </pre>
      </details>
    </article>
  );
}

function ToolCallDetailPanel({ detail }: { detail: ToolCallDetail }) {
  const traceRoute = toolCallTraceRoute(detail.summary);
  return (
    <article aria-label="Tool-call detail">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div>
          <div className="zarya-label">{detail.summary.tool_id}</div>
          <div className="mt-1 break-all font-mono text-xs text-[var(--color-muted)]">
            {detail.summary.tool_call_id}
          </div>
        </div>
        <Badge tone={toolCallTone(detail.summary.state)}>
          {detail.summary.state}
        </Badge>
      </div>
      <dl className="mt-4 grid gap-3 text-xs sm:grid-cols-2">
        <LabeledValue label="Actor" value={actorLabel(detail.summary)} />
        <LabeledValue
          label="Source / route"
          value={`${detail.summary.source_kind} · ${detail.summary.route}`}
        />
        <LabeledValue
          label="Policy"
          value={`${detail.summary.policy_decision} · ${detail.policy_version}`}
        />
        <LabeledValue
          label="Timing"
          value={`${formatDateTime(detail.summary.started_at ?? detail.summary.requested_at)} → ${formatDateTime(detail.summary.completed_at)}`}
        />
      </dl>
      {traceRoute ? (
        <Link
          to={traceRoute}
          className="mt-4 inline-flex h-9 items-center gap-2 border border-[var(--color-muted)] px-3 text-sm font-medium hover:border-[var(--color-ink)] hover:bg-[var(--color-bg-muted)]"
        >
          Open session trace
          <ArrowUpRight size={14} aria-hidden="true" />
        </Link>
      ) : null}
      <RedactedSummary
        label="Arguments summary"
        value={detail.redacted_arguments_summary}
      />
      <RedactedSummary
        label="Result summary"
        value={detail.redacted_result_summary}
      />
      {detail.error ? (
        <div className="mt-4 border-l-2 border-[var(--color-risk)] pl-3 text-xs text-[var(--color-risk)]">
          {detail.error.code}: {detail.error.message}
        </div>
      ) : null}
      {[...detail.trace_refs, ...detail.result_refs].length > 0 ? (
        <div className="mt-4">
          <div className="zarya-label">Trace and result references</div>
          <ul className="mt-2 grid gap-2">
            {[...detail.trace_refs, ...detail.result_refs].map(
              (reference, index) => (
                <li
                  key={`${reference.kind}-${index}`}
                  className="flex items-center justify-between gap-2 border border-[var(--color-border)] px-2 py-1 text-xs"
                >
                  <span className="truncate font-mono">{reference.kind}</span>
                  <ReferenceActions reference={reference} />
                </li>
              ),
            )}
          </ul>
        </div>
      ) : null}
    </article>
  );
}

function RedactedSummary({
  label,
  value,
}: {
  label: string;
  value: string | null;
}) {
  if (!value) return null;
  return (
    <div className="mt-4">
      <div className="zarya-label">{label}</div>
      <pre className="mt-2 max-h-40 overflow-auto whitespace-pre-wrap break-all border border-[var(--color-border)] p-3 font-mono text-xs text-[var(--color-muted)]">
        {value}
      </pre>
    </div>
  );
}

function LabeledValue({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <dt className="zarya-label">{label}</dt>
      <dd className="mt-1 break-words text-[var(--color-ink)]">{value}</dd>
    </div>
  );
}

function selectedPlacement(
  placements: ProjectPlacementSummary[],
  requestedId: string | null,
) {
  return (
    placements.find(
      (placement) => placement.project_placement_id === requestedId,
    ) ?? placements[0]
  );
}

export function integrationDiagnostic(
  connection: IntegrationConnectionSummary,
  dependency?: McpDependencyStatus,
  node?: NodeSummary,
) {
  if (node && node.presence !== "reachable") {
    return `Node is ${node.presence}; managed external tools are unavailable for sessions placed there.`;
  }
  if (connection.auth_state !== "connected") {
    return connection.auth_state === "error"
      ? `Authentication failed${connection.error_code ? ` (${connection.error_code})` : ""}. No credential detail is exposed.`
      : "Linear authorization is missing, expired or disconnected; effective availability is closed.";
  }
  switch (dependency?.actual_state) {
    case "toolhive_missing":
      return "ToolHive is not installed on the selected Node. Install the pinned runtime before enabling Linear.";
    case "missing_auth":
      return "ToolHive is running without usable Linear authorization.";
    case "degraded":
      return "The ToolHive backend is degraded; Inspect remains available but Execute may fail.";
    case "failed":
      return `The ToolHive dependency failed${dependency.error_code ? ` (${dependency.error_code})` : ""}.`;
    case "running":
      return "Linear authorization and ToolHive runtime are ready; per-session policy still determines effective availability.";
    case "installing":
    case "starting":
      return `ToolHive is ${dependency.actual_state}; the integration is not available yet.`;
    case "stopped":
      return "ToolHive is stopped because the desired integration state is disabled.";
    default:
      return "ToolHive actual state has not been reported by the Node.";
  }
}

export function availabilityReasonLabel(reason: ToolAvailability["reason"]) {
  const labels: Record<NonNullable<ToolAvailability["reason"]>, string> = {
    node_offline: "Node offline",
    capability_missing: "Required Node capability missing",
    dependency_missing: "External dependency is not configured",
    dependency_unhealthy: "External dependency is unhealthy",
    not_authenticated: "Integration authentication missing",
    permission_denied: "Actor permission denied",
    policy_blocked: "Current policy blocks execution",
    project_not_enabled: "Integration not enabled for this project",
    session_not_enabled: "Tool not enabled for this session",
    schema_changed: "Schema changed since Inspect",
    backend_unreachable: "Backend unreachable",
    toolhive_missing: "ToolHive missing on Node",
  };
  return reason ? labels[reason] : "Available";
}

export function toolCallTraceRoute(call: ToolCallSummary) {
  const placementId = call.scope.project_placement_id;
  const sessionId = call.scope.session_thread_id;
  if (!placementId || !sessionId) return null;
  return `${workspaceAgentSessionRoute(placementId, sessionId)}?agentView=trace`;
}

function actorLabel(call: ToolCallSummary) {
  const actor = call.actor_ref;
  if (typeof actor === "string") return actor;
  if ("kind" in actor && actor.kind === "provider") return actor.provider;
  if ("kind" in actor && actor.kind === "node") return actor.node_id;
  return "local user";
}

function formatDateTime(value: string | null | undefined) {
  if (!value) return "pending";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

function stateTone(state: string): BadgeTone {
  return state === "available" ? "good" : state === "unknown" ? "warn" : "bad";
}

function availabilityTone(availability?: ToolAvailability): BadgeTone {
  if (!availability) return "neutral";
  if (availability.state === "available") return "good";
  if (
    availability.state === "degraded" ||
    availability.state === "approval_required"
  ) {
    return "warn";
  }
  return "bad";
}

function authTone(
  state: IntegrationConnectionSummary["auth_state"],
): BadgeTone {
  if (state === "connected") return "good";
  if (state === "connecting") return "warn";
  return state === "disconnected" ? "neutral" : "bad";
}

function dependencyTone(
  state: McpDependencyStatus["actual_state"] | undefined,
): BadgeTone {
  if (state === "running") return "good";
  if (state === "starting" || state === "installing" || state === "degraded") {
    return "warn";
  }
  return state ? "bad" : "neutral";
}

function riskTone(risk: ToolDefinition["risk_level"]): BadgeTone {
  return risk === "read_only" || risk === "external_read"
    ? "info"
    : risk === "workspace_write" || risk === "external_write"
      ? "warn"
      : "bad";
}

function toolCallTone(state: ToolCallSummary["state"]): BadgeTone {
  if (state === "completed") return "good";
  if (
    state === "requested" ||
    state === "authorized" ||
    state === "started" ||
    state === "approval_required"
  ) {
    return "warn";
  }
  return "bad";
}
