import { useQuery } from "@tanstack/react-query";
import { Link, Navigate, useLocation, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { EmptyState, LoadingState, PageHeader } from "../../shared/ui/system";
import { useWorkspaceContext } from "../workspaces/WorkspaceLayout";
import {
  routeWithSearch,
  workspaceAgentSessionRoute,
} from "../workspaces/routes";
import { SessionRoute } from "./SessionRoute";

export function WorkspaceAgentRoute() {
  const { placement, sessions } = useWorkspaceContext();
  const location = useLocation();
  const orderedSessions = [...sessions].sort((left, right) =>
    right.updated_at.localeCompare(left.updated_at),
  );

  return (
    <section>
      <PageHeader
        title="Agent"
        description="Managed sessions in this workspace. Session controls remain unchanged while the workspace navigation is introduced."
        meta="WORKSPACE / AGENT"
      />
      <div className="grid gap-2">
        {orderedSessions.map((session) => (
          <Link
            key={session.session_thread_id}
            to={routeWithSearch(
              workspaceAgentSessionRoute(
                placement.project_placement_id,
                session.session_thread_id,
              ),
              location.search,
            )}
            className="grid gap-2 border border-black/20 p-3 hover:bg-[var(--color-bg-muted)] md:grid-cols-[minmax(0,1fr)_auto] md:items-center"
          >
            <span className="min-w-0">
              <span className="block truncate font-medium">
                {session.title}
              </span>
              <span className="mt-1 block text-xs text-[var(--color-muted)]">
                Updated {new Date(session.updated_at).toLocaleString()}
              </span>
            </span>
            <span className="flex gap-1">
              <Badge tone={session.state === "active" ? "good" : "neutral"}>
                {session.state}
              </Badge>
              <Badge tone="neutral">{session.runtime.state}</Badge>
            </span>
          </Link>
        ))}
        {orderedSessions.length === 0 ? (
          <EmptyState
            title="No sessions in this workspace"
            detail="Session creation remains available from the existing Workbench screen until the Agent surface is completed."
          />
        ) : null}
      </div>
    </section>
  );
}

export function WorkspaceSessionRoute() {
  const { placementId = "", sessionThreadId = "" } = useParams();
  const location = useLocation();
  const session = useQuery({
    queryKey: queryKeys.session(sessionThreadId),
    queryFn: () => coreApi.session(sessionThreadId),
    enabled: Boolean(sessionThreadId),
  });

  if (session.isError) {
    return <ErrorNotice error={session.error} title="Session load failed" />;
  }
  if (!session.data) return <LoadingState stage="Validating session context" />;
  const actualPlacementId = session.data.placement.project_placement_id;
  if (actualPlacementId !== placementId) {
    return (
      <Navigate
        replace
        to={routeWithSearch(
          workspaceAgentSessionRoute(actualPlacementId, sessionThreadId),
          location.search,
        )}
      />
    );
  }
  return <SessionRoute />;
}

export function SessionCompatibilityRoute() {
  const { sessionThreadId = "" } = useParams();
  const location = useLocation();
  const session = useQuery({
    queryKey: queryKeys.session(sessionThreadId),
    queryFn: () => coreApi.session(sessionThreadId),
    enabled: Boolean(sessionThreadId),
  });

  if (session.isError) {
    return <ErrorNotice error={session.error} title="Session load failed" />;
  }
  if (!session.data) return <LoadingState stage="Resolving session" />;
  return (
    <Navigate
      replace
      to={routeWithSearch(
        workspaceAgentSessionRoute(
          session.data.placement.project_placement_id,
          sessionThreadId,
        ),
        location.search,
      )}
    />
  );
}
