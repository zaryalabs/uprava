import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { FolderOpen } from "lucide-react";
import { useEffect } from "react";
import { Link, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { openSessionStream } from "../../shared/api/sse-client";
import { Badge } from "../../shared/ui/badge";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";
import { sessionEventCursor } from "../../workbench/projection/apply-session-event";
import { applySessionStreamEventToCache } from "../../workbench/projection/session-stream-cache";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import {
  projectRefForPlacement,
  routeForRef,
  workspaceRefForPlacement,
} from "../../workbench/references/refs";
import { EvidenceProjection } from "../artifacts/EvidenceProjection";
import { AgentProjectionPanel } from "../agent-projection/AgentProjectionPanel";
import { ChatComposer } from "./ChatComposer";
import { LifecycleControls } from "./LifecycleControls";
import { SessionTimeline } from "./SessionTimeline";

export function SessionRoute() {
  const { sessionThreadId } = useParams();
  const queryClient = useQueryClient();
  const session = useQuery({
    queryKey: queryKeys.session(sessionThreadId ?? ""),
    queryFn: () => coreApi.session(sessionThreadId ?? ""),
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
  }, [queryClient, session.data?.events, sessionThreadId]);

  if (session.isError) {
    return <ErrorNotice error={session.error} title="Session load failed" />;
  }

  if (!session.data) {
    return <div className="text-sm text-[#536257]">Loading session</div>;
  }

  const isDetached = session.data.session.state === "detached";
  const canSendTurn = canRunCommand("session.sendTurn", {
    session: session.data.session,
    runtime: session.data.session.runtime,
    turnContent: "ready",
  });
  const projectRef = projectRefForPlacement(session.data.placement);
  const workspaceRef = workspaceRefForPlacement(session.data.placement);
  const workspaceRoute =
    routeForRef(workspaceRef) ??
    `/workspaces/${session.data.placement.project_placement_id}`;

  return (
    <section className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_280px]">
      <div className="min-w-0 space-y-4">
        <header className="rounded-md border border-[#d9ded4] bg-white p-4">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="min-w-0">
              <h1 className="truncate text-2xl font-semibold">
                {session.data.session.title}
              </h1>
              <div className="mt-1 truncate text-sm text-[#536257]">
                {session.data.placement.display_name} ·{" "}
                {session.data.placement.workspace_path}
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Link
                to={workspaceRoute}
                className="inline-flex h-9 items-center justify-center gap-2 rounded-md border border-[#bfc8bc] bg-[#fbfcf8] px-3 text-sm font-medium text-[#17211c] transition hover:bg-[#edf1e9]"
              >
                <FolderOpen size={16} />
                Workspace
              </Link>
              <ReferenceActions reference={workspaceRef} />
              {projectRef ? <ReferenceActions reference={projectRef} /> : null}
              <ReferenceActions
                reference={{
                  kind: "session",
                  session_thread_id: session.data.session.session_thread_id,
                }}
              />
              <ReferenceActions
                reference={{
                  kind: "runtime",
                  runtime_session_id:
                    session.data.session.runtime.runtime_session_id,
                }}
              />
              <Badge tone="info">{session.data.session.runtime.provider}</Badge>
              <Badge tone={isDetached ? "warn" : "good"}>
                {session.data.session.state}
              </Badge>
              <Badge tone="good">{session.data.session.runtime.state}</Badge>
            </div>
          </div>
          <div className="mt-4">
            <LifecycleControls
              session={session.data.session}
              runtime={session.data.session.runtime}
            />
          </div>
        </header>
        <SessionTimeline detail={session.data} />
        {sendTurn.isError ? (
          <ErrorNotice error={sendTurn.error} title="Send failed" />
        ) : null}
        <ChatComposer
          pending={sendTurn.isPending}
          disabled={!canSendTurn}
          onSend={(content) => sendTurn.mutateAsync(content).then(() => {})}
        />
      </div>
      <aside className="space-y-4">
        <EvidenceProjection
          sessionThreadId={session.data.session.session_thread_id}
        />
        <AgentProjectionPanel
          sessionThreadId={session.data.session.session_thread_id}
        />
      </aside>
    </section>
  );
}
