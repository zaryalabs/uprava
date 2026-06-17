import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { openSessionStream } from "../../shared/api/sse-client";
import { Badge } from "../../shared/ui/badge";
import { runWorkbenchCommand } from "../../workbench/commands/registry";
import { applySessionStreamEventToCache } from "../../workbench/projection/session-stream-cache";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";
import { ArtifactTree } from "../artifacts/ArtifactTree";
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
        turnContent: content,
        afterSuccess: invalidateSession,
      }),
  });

  useEffect(() => {
    if (!sessionThreadId || !session.data?.events.length) return;
    const afterSeq = Math.max(...session.data.events.map((event) => event.seq));
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

  if (!session.data) {
    return <div className="text-sm text-[#536257]">Loading session</div>;
  }

  const isDetached = session.data.session.state === "detached";

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
        <ChatComposer
          pending={sendTurn.isPending}
          disabled={isDetached}
          onSend={(content) => sendTurn.mutate(content)}
        />
      </div>
      <aside className="space-y-4">
        <ArtifactTree
          sessionThreadId={session.data.session.session_thread_id}
        />
        <AgentProjectionPanel
          sessionThreadId={session.data.session.session_thread_id}
        />
      </aside>
    </section>
  );
}
