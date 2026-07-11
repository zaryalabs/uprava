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
        availableCommands: projection.data?.available_commands,
        afterSuccess: invalidateSessionState,
      }),
  });

  if (!projection.data) {
    return null;
  }
  const sessionSummary = sessionSummaryFromProjection(projection.data);

  return (
    <section>
      <div className="zarya-label">Agent Context</div>
      <h2 className="mt-1 text-base font-bold">Agent Projection</h2>
      <div className="mt-3 space-y-3 text-sm text-[var(--color-muted)]">
        <div>{projection.data.evidence_projection_summary}</div>
        <div>{projection.data.source_cause_summary}</div>
        {projection.data.active_warnings.length > 0 ? (
          <div className="space-y-2">
            {projection.data.active_warnings.map((warning) => (
              <div
                key={warning.kind}
                className={`border-l-2 p-2 ${warning.severity === "hard_block" ? "border-[var(--color-risk)] bg-[var(--color-risk-soft)]" : "border-[var(--color-muted)]"}`}
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
                        availableCommands: projection.data.available_commands,
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
