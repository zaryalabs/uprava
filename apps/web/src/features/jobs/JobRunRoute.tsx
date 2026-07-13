import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Link, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { runTone } from "./JobsRoute";
import {
  workspaceAgentSessionRoute,
  workspaceJobRoute,
} from "../workspaces/routes";

export function JobRunRoute() {
  const { placementId = "", jobId = "", jobRunId } = useParams();
  const queryClient = useQueryClient();
  const run = useQuery({
    queryKey: queryKeys.jobRun(jobRunId ?? ""),
    queryFn: () => coreApi.jobRun(jobRunId ?? ""),
    enabled: Boolean(jobRunId),
    refetchInterval: (query) =>
      query.state.data && isTerminal(query.state.data.state) ? false : 1_000,
  });
  const cancel = useMutation({
    mutationFn: () => coreApi.cancelJobRun(jobRunId ?? ""),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: queryKeys.jobRun(jobRunId ?? ""),
      });
    },
  });

  if (run.isError)
    return <ErrorNotice error={run.error} title="Job Run load failed" />;
  if (!run.data)
    return (
      <div className="text-sm text-[var(--color-muted)]">Loading Job Run</div>
    );
  const detail = run.data;
  const active = ["queued", "starting", "running"].includes(detail.state);

  return (
    <section className="space-y-7">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="zarya-caption">JOB RUN / {detail.trigger}</div>
          <h1 className="mt-2 text-2xl font-semibold">
            Run {detail.job_run_id.slice(0, 8)}
          </h1>
          <div className="mt-2 flex gap-2">
            <Badge tone={runTone(detail.state)}>{detail.state}</Badge>
            {detail.force ? <Badge tone="warn">quota override</Badge> : null}
          </div>
        </div>
        {active ? (
          <Button
            variant="danger"
            disabled={cancel.isPending}
            onClick={() => cancel.mutate()}
          >
            Cancel run
          </Button>
        ) : null}
      </header>

      {cancel.isError ? (
        <ErrorNotice error={cancel.error} title="Cancellation failed" />
      ) : null}
      <section className="border-l-2 border-[var(--color-ink)] bg-[var(--color-bg-muted)] p-4">
        <div className="text-xs font-bold text-[var(--color-muted)]">
          OUTCOME / SUMMARY
        </div>
        <div className="mt-2 whitespace-pre-wrap text-sm">
          {detail.summary ??
            detail.terminal_reason?.message ??
            "Summary is not available yet."}
        </div>
        {detail.terminal_reason ? (
          <div className="mt-2 font-mono text-xs text-[var(--color-risk)]">
            {detail.terminal_reason.code}
          </div>
        ) : null}
      </section>

      <div className="grid gap-4 text-sm md:grid-cols-3">
        <Datum label="Queued">
          {new Date(detail.queued_at).toLocaleString()}
        </Datum>
        <Datum label="Started">
          {detail.started_at
            ? new Date(detail.started_at).toLocaleString()
            : "—"}
        </Datum>
        <Datum label="Finished">
          {detail.finished_at
            ? new Date(detail.finished_at).toLocaleString()
            : "—"}
        </Datum>
      </div>

      <div className="flex flex-wrap gap-3 text-sm">
        <Link
          className="underline"
          to={workspaceJobRoute(placementId, jobId || detail.job_id)}
        >
          Open Job
        </Link>
        {detail.session_thread_id ? (
          <Link
            className="underline"
            to={workspaceAgentSessionRoute(
              placementId,
              detail.session_thread_id,
            )}
          >
            Open session output and evidence
          </Link>
        ) : null}
      </div>

      <details className="border border-black/20 p-4">
        <summary className="cursor-pointer text-sm font-bold">
          Effective configuration snapshot
        </summary>
        <pre className="mt-3 overflow-auto whitespace-pre-wrap break-words text-xs">
          {JSON.stringify(detail.config_snapshot, null, 2)}
        </pre>
      </details>
    </section>
  );
}

function isTerminal(state: string) {
  return ["succeeded", "failed", "cancelled", "timed_out", "skipped"].includes(
    state,
  );
}

function Datum({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="border-t border-black/20 pt-3">
      <div className="text-xs font-bold text-[var(--color-muted)]">{label}</div>
      <div className="mt-1">{children}</div>
    </div>
  );
}
