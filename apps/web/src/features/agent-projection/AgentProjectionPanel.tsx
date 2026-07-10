import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  AgentProjection,
  SessionSummary,
} from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import {
  canRunCommand,
  runWorkbenchCommand,
} from "../../workbench/commands/registry";

export function AgentProjectionPanel({
  sessionThreadId,
}: {
  sessionThreadId: string;
}) {
  const queryClient = useQueryClient();
  const projection = useQuery({
    queryKey: queryKeys.agentProjection(sessionThreadId),
    queryFn: () => coreApi.agentProjection(sessionThreadId),
  });
  const invalidateSessionState = async () => {
    await Promise.all([
      queryClient.invalidateQueries({
        queryKey: queryKeys.agentProjection(sessionThreadId),
      }),
      queryClient.invalidateQueries({
        queryKey: queryKeys.session(sessionThreadId),
      }),
      queryClient.invalidateQueries({ queryKey: queryKeys.inventory }),
    ]);
  };
  const acknowledgeWarning = useMutation({
    mutationFn: (warningKind: string) =>
      runWorkbenchCommand("warning.acknowledge", {
        session: projection.data
          ? sessionSummaryFromProjection(projection.data)
          : undefined,
        warningKind,
        afterSuccess: invalidateSessionState,
      }),
  });

  if (!projection.data) {
    return null;
  }
  const sessionSummary = sessionSummaryFromProjection(projection.data);

  return (
    <section className="rounded-md border border-[#d9ded4] bg-white p-3">
      <h2 className="text-sm font-semibold">Agent Projection</h2>
      <div className="mt-2 space-y-2 text-sm text-[#536257]">
        <div>{projection.data.evidence_projection_summary}</div>
        <div>{projection.data.source_cause_summary}</div>
        {projection.data.active_warnings.length > 0 ? (
          <div className="space-y-2">
            {projection.data.active_warnings.map((warning) => (
              <div
                key={warning.kind}
                className="rounded-md border border-[#d9c47d] bg-[#fff5ce] p-2"
              >
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <Badge
                    tone={warning.severity === "hard_block" ? "bad" : "warn"}
                  >
                    {warning.label}
                  </Badge>
                  <Button
                    variant="secondary"
                    disabled={
                      acknowledgeWarning.isPending ||
                      !canRunCommand("warning.acknowledge", {
                        session: sessionSummary,
                        warningKind: warning.kind,
                      })
                    }
                    onClick={() => acknowledgeWarning.mutate(warning.kind)}
                  >
                    Acknowledge
                  </Button>
                </div>
              </div>
            ))}
          </div>
        ) : null}
        {acknowledgeWarning.isError ? (
          <ErrorNotice
            error={acknowledgeWarning.error}
            title="Warning acknowledgement failed"
          />
        ) : null}
        <div className="flex flex-wrap gap-1">
          {projection.data.available_commands.map((command) => (
            <Badge key={command}>{command}</Badge>
          ))}
        </div>
      </div>
    </section>
  );
}

export function sessionSummaryFromProjection(
  projection: AgentProjection,
): SessionSummary {
  return {
    session_thread_id: projection.session_thread_id,
    project_placement_id: projection.project_placement.project_placement_id,
    runtime_session_id: projection.runtime_summary.runtime_session_id,
    title: "Session",
    state: "active",
    runtime: projection.runtime_summary,
    message_count: projection.recent_message_refs.length,
    updated_at: projection.generated_at,
  };
}
