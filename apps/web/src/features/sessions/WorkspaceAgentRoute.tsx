import { useQuery } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { Link, Navigate, useLocation, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { SessionSummary } from "../../shared/protocol/types";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { StatusIndicator } from "../../shared/ui/status-indicator";
import { EmptyState, LoadingState } from "../../shared/ui/system";
import { useWorkspaceContext } from "../workspaces/WorkspaceLayout";
import {
  routeWithSearch,
  workspaceAgentSessionRoute,
} from "../workspaces/routes";
import { SessionSurface } from "./SessionSurface";
import { StartSessionControl } from "./StartSessionControl";

export function WorkspaceAgentRoute() {
  const { placement, sessions } = useWorkspaceContext();
  const location = useLocation();
  const orderedSessions = orderWorkspaceSessions(sessions);
  const latestSession = orderedSessions[0];

  if (latestSession) {
    return (
      <Navigate
        replace
        to={routeWithSearch(
          workspaceAgentSessionRoute(
            placement.project_placement_id,
            latestSession.session_thread_id,
          ),
          location.search,
        )}
      />
    );
  }

  return (
    <WorkspaceAgentSurface selectedSessionThreadId={null}>
      <div className="grid min-h-72 place-items-center border border-dashed border-black/20 p-8 text-center">
        <EmptyState
          title="Start a session"
          detail="This workspace has no managed sessions yet. Use Start Codex in the session list to begin an Agent runtime."
        />
      </div>
    </WorkspaceAgentSurface>
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

  return (
    <WorkspaceAgentSurface selectedSessionThreadId={sessionThreadId}>
      <SessionSurface sessionThreadId={sessionThreadId} />
    </WorkspaceAgentSurface>
  );
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

function WorkspaceAgentSurface({
  children,
  selectedSessionThreadId,
}: {
  children: ReactNode;
  selectedSessionThreadId: string | null;
}) {
  const { node, placement, sessions } = useWorkspaceContext();
  const location = useLocation();
  const orderedSessions = orderWorkspaceSessions(sessions);

  return (
    <section className="space-y-4" aria-labelledby="workspace-agent-title">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <div className="zarya-caption">WORKSPACE AGENT</div>
          <h2 id="workspace-agent-title" className="mt-1 text-xl font-bold">
            Agent
          </h2>
        </div>
        <div className="text-xs text-[var(--color-muted)]">
          {orderedSessions.length}{" "}
          {orderedSessions.length === 1 ? "session" : "sessions"}
        </div>
      </header>
      <div className="uprava-agent-grid">
        <aside
          className="uprava-agent-sessions"
          aria-label="Workspace sessions"
        >
          <StartSessionControl node={node} placement={placement} />
          <nav className="mt-4" aria-label="Sessions">
            <div className="zarya-label mb-2">RECENT SESSIONS</div>
            {orderedSessions.length > 0 ? (
              <ul className="space-y-1">
                {orderedSessions.map((session) => (
                  <li key={session.session_thread_id}>
                    <SessionListLink
                      session={session}
                      selected={
                        session.session_thread_id === selectedSessionThreadId
                      }
                      to={routeWithSearch(
                        workspaceAgentSessionRoute(
                          placement.project_placement_id,
                          session.session_thread_id,
                        ),
                        location.search,
                      )}
                    />
                  </li>
                ))}
              </ul>
            ) : (
              <p className="py-3 text-xs text-[var(--color-muted)]">
                No sessions yet.
              </p>
            )}
          </nav>
        </aside>
        <div className="min-w-0">{children}</div>
      </div>
    </section>
  );
}

function SessionListLink({
  selected,
  session,
  to,
}: {
  selected: boolean;
  session: SessionSummary;
  to: string;
}) {
  const attention = sessionAttention(session);

  return (
    <Link
      to={to}
      aria-current={selected ? "page" : undefined}
      className={`block border p-3 ${
        selected
          ? "border-[var(--color-ink)] bg-[var(--color-bg)]"
          : "border-transparent hover:border-black/20 hover:bg-[var(--color-bg)]"
      }`}
    >
      <span className="block truncate text-sm font-medium">
        {session.title}
      </span>
      <span className="mt-1 block text-xs text-[var(--color-muted)]">
        <time dateTime={session.updated_at}>
          {new Date(session.updated_at).toLocaleString()}
        </time>
      </span>
      <span className="mt-2 flex flex-wrap gap-1">
        <StatusIndicator
          showDimension
          dimension="lifecycle"
          value={session.state}
        />
        <StatusIndicator
          showDimension
          dimension="attention"
          value={attention}
        />
      </span>
    </Link>
  );
}

export function orderWorkspaceSessions(sessions: SessionSummary[]) {
  return [...sessions].sort((left, right) => {
    const byUpdatedAt = right.updated_at.localeCompare(left.updated_at);
    return (
      byUpdatedAt ||
      left.session_thread_id.localeCompare(right.session_thread_id)
    );
  });
}

function sessionAttention(session: SessionSummary) {
  if (session.runtime.state === "error" || session.state === "degraded") {
    return "degraded";
  }
  if (
    session.runtime.state === "blocked" ||
    session.runtime.state === "stale" ||
    Boolean(session.runtime.degraded_reason)
  ) {
    return session.runtime.state === "blocked" ? "blocked" : "warning";
  }
  return "clear";
}
