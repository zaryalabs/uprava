import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderOpen } from "lucide-react";
import { useEffect } from "react";
import { Link, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { openSessionStream } from "../../shared/api/sse-client";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { LoadingState, PageHeader } from "../../shared/ui/system";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { sessionEventCursor } from "../../workbench/projection/apply-session-event";
import { applySessionStreamEventToCache } from "../../workbench/projection/session-stream-cache";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { workspaceRefForPlacement } from "../../workbench/references/refs";
import { EvidenceProjection } from "../artifacts/EvidenceProjection";
import { AgentProjectionPanel } from "../agent-projection/AgentProjectionPanel";
import { ChatComposer } from "./ChatComposer";
import { LifecycleControls } from "./LifecycleControls";
import { SessionTimeline } from "./SessionTimeline";
import { ScheduledMessagesPanel } from "./ScheduledMessagesPanel";
import { workspaceAgentRoute } from "../workspaces/routes";

export function SessionRoute() {
  const { sessionThreadId } = useParams();
  const queryClient = useQueryClient();
  const session = useQuery({
    queryKey: queryKeys.session(sessionThreadId ?? ""),
    queryFn: () => coreApi.session(sessionThreadId ?? ""),
    enabled: Boolean(sessionThreadId),
  });
  const agentProjection = useQuery({
    queryKey: queryKeys.agentProjection(sessionThreadId ?? ""),
    queryFn: () => coreApi.agentProjection(sessionThreadId ?? ""),
    enabled: Boolean(sessionThreadId),
  });
  const invalidateSession = async () => {
    await queryClient.invalidateQueries({
      queryKey: queryKeys.session(sessionThreadId ?? ""),
    });
    await queryClient.invalidateQueries({ queryKey: queryKeys.inventory });
  };
  const sendTurn = useMutation({
    mutationFn: (content: string) =>
      runWorkbenchCommand("session.sendTurn", {
        session: session.data?.session,
        runtime: session.data?.session.runtime,
        turnContent: content,
        availableCommands: agentProjection.data?.available_commands,
        afterSuccess: invalidateSession,
      }),
  });

  useEffect(() => {
    if (!sessionThreadId || !session.data) return;
    const afterSeq = session.data.events.reduce(
      (max, event) => Math.max(max, sessionEventCursor(event)),
      0,
    );
    return openSessionStream(
      sessionThreadId,
      afterSeq,
      (event) => {
        void applySessionStreamEventToCache(
          queryClient,
          sessionThreadId,
          event,
        );
      },
      () => {
        void queryClient.invalidateQueries({
          queryKey: queryKeys.session(sessionThreadId),
        });
      },
    );
  }, [queryClient, session.data?.session.session_thread_id, sessionThreadId]);

  if (session.isError) {
    return <ErrorNotice error={session.error} title="Session load failed" />;
  }

  if (!session.data) {
    return <LoadingState stage="Loading session" />;
  }

  const isDetached = session.data.session.state === "detached";
  const canSendTurn = canRunCommand("session.sendTurn", {
    session: session.data.session,
    runtime: session.data.session.runtime,
    turnContent: "ready",
    availableCommands: agentProjection.data?.available_commands,
  });
  const workspaceRef = workspaceRefForPlacement(session.data.placement);
  const workspaceRoute = workspaceAgentRoute(
    session.data.placement.project_placement_id,
  );

  return (
    <section>
      <PageHeader
        title={session.data.session.title}
        description={`${session.data.placement.display_name} / ${session.data.placement.workspace_path}`}
        meta={`SESSION / ${session.data.session.runtime.provider} / ${session.data.session.runtime.state}`}
        actions={
          <>
            <Link
              to={workspaceRoute}
              className="inline-flex h-9 items-center justify-center gap-2 border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 text-sm font-medium hover:border-[var(--color-ink)] hover:bg-[var(--color-bg-muted)]"
            >
              <FolderOpen size={16} aria-hidden="true" />
              Workspace
            </Link>
            <ReferenceActions
              reference={{
                kind: "session",
                session_thread_id: session.data.session.session_thread_id,
              }}
              showCopy={false}
            />
            <ReferenceActions
              reference={{
                kind: "runtime",
                runtime_session_id:
                  session.data.session.runtime.runtime_session_id,
              }}
              showCopy={false}
            />
            <Badge tone={isDetached ? "warn" : "good"}>
              {session.data.session.state}
            </Badge>
            <Badge tone="good">{session.data.session.runtime.state}</Badge>
          </>
        }
      />

      <section className="grid gap-4 border-l-2 border-[var(--color-ink)] py-4 pl-4 md:grid-cols-[minmax(0,1fr)_auto] md:items-center">
        <div>
          <div className="zarya-label">Runtime Context</div>
          <div className="mt-1 text-sm">
            Phase <strong>{session.data.session.runtime.state}</strong> ·
            Session <strong>{session.data.session.state}</strong> · Resume{" "}
            {session.data.session.runtime.resume_supported
              ? "supported"
              : "unavailable"}
          </div>
          <div className="mt-1 text-xs text-[var(--color-muted)]">
            Stop and interrupt can end active work. Detach preserves the managed
            runtime.
          </div>
        </div>
        <LifecycleControls
          session={session.data.session}
          runtime={session.data.session.runtime}
          availableCommands={agentProjection.data?.available_commands ?? []}
        />
      </section>

      <div className="mt-8 grid gap-8 xl:grid-cols-[minmax(0,1fr)_280px]">
        <div className="min-w-0 space-y-4">
          <SessionTimeline
            detail={session.data}
            availableCommands={agentProjection.data?.available_commands ?? []}
          />
          {sendTurn.isError ? (
            <ErrorNotice error={sendTurn.error} title="Send failed" />
          ) : null}
          <ChatComposer
            pending={sendTurn.isPending}
            disabled={!canSendTurn}
            onSend={(content) => sendTurn.mutateAsync(content).then(() => {})}
          />
          <ScheduledMessagesPanel
            sessionThreadId={session.data.session.session_thread_id}
            messages={session.data.scheduled_messages ?? []}
            onChanged={invalidateSession}
          />
        </div>
        <aside
          className="space-y-8 border-l border-black/10 pl-4 max-xl:border-l-0 max-xl:border-t max-xl:pt-6"
          aria-label="Session Evidence & Agent Context"
        >
          <EvidenceProjection
            sessionThreadId={session.data.session.session_thread_id}
          />
          <AgentProjectionPanel
            sessionThreadId={session.data.session.session_thread_id}
          />
        </aside>
      </div>
    </section>
  );
}
