import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Pause, Play, RotateCcw } from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { formatSchedule, runTone } from "./JobsRoute";
import type { JobDetail } from "../../shared/protocol/types";

export function JobDetailRoute() {
  const { jobId } = useParams();
  const queryClient = useQueryClient();
  const [force, setForce] = useState(false);
  const job = useQuery({
    queryKey: queryKeys.job(jobId ?? ""),
    queryFn: () => coreApi.job(jobId ?? ""),
    enabled: Boolean(jobId),
    refetchInterval: 1_500,
  });
  const quota = useQuery({
    queryKey: queryKeys.providerQuota(job.data?.job.provider ?? "codex"),
    queryFn: () => coreApi.providerQuota(job.data?.job.provider ?? "codex"),
    enabled: Boolean(job.data),
  });
  const refresh = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: queryKeys.job(jobId ?? "") }),
      queryClient.invalidateQueries({ queryKey: queryKeys.jobs }),
      queryClient.invalidateQueries({ queryKey: queryKeys.inventory }),
    ]);
  };
  const run = useMutation({
    mutationFn: () => coreApi.runJob(jobId ?? "", force),
    onSuccess: refresh,
  });
  const toggle = useMutation({
    mutationFn: () =>
      job.data?.job.enabled
        ? coreApi.disableJob(jobId ?? "")
        : coreApi.enableJob(jobId ?? ""),
    onSuccess: refresh,
  });

  if (job.isError)
    return <ErrorNotice error={job.error} title="Job load failed" />;
  if (!job.data)
    return <div className="text-sm text-[var(--color-muted)]">Loading Job</div>;
  const detail = job.data;

  return (
    <section className="space-y-7">
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div>
          <div className="zarya-caption">
            BACKGROUND JOB / {detail.job.provider}
          </div>
          <h1 className="mt-2 text-2xl font-semibold">{detail.job.name}</h1>
          <div className="mt-2 text-sm text-[var(--color-muted)]">
            {detail.job.placement_name} ·{" "}
            {formatSchedule(detail.job.schedule, detail.job.timezone)}
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button
            variant="secondary"
            disabled={
              toggle.isPending || (!detail.job.schedule && !detail.job.enabled)
            }
            onClick={() => toggle.mutate()}
          >
            {detail.job.enabled ? <Pause size={16} /> : <Play size={16} />}
            {detail.job.enabled ? "Pause schedule" : "Enable schedule"}
          </Button>
          <Button
            variant="primary"
            disabled={run.isPending}
            onClick={() => run.mutate()}
          >
            <RotateCcw size={16} /> Manual run
          </Button>
        </div>
      </header>

      <div className="flex flex-wrap gap-2">
        <Badge tone={detail.job.enabled ? "good" : "neutral"}>
          {detail.job.enabled ? "enabled" : "paused"}
        </Badge>
        <Badge tone="neutral">overlap: skip</Badge>
        <Badge tone={detail.job.continue_after_error ? "warn" : "neutral"}>
          {detail.job.continue_after_error
            ? "continue on error"
            : "stop on error"}
        </Badge>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Datum label="Next automatic start">
          {detail.job.next_run_at
            ? new Date(detail.job.next_run_at).toLocaleString()
            : "—"}
        </Datum>
        <Datum label="Paused reason">{detail.job.paused_reason ?? "—"}</Datum>
        <Datum label="Provider quota">
          {quota.data?.state ?? "loading"}
          {quota.data?.unavailable_reason
            ? ` · ${quota.data.unavailable_reason}`
            : ""}
        </Datum>
      </div>

      <label className="flex items-center gap-2 border-l-2 border-[var(--color-notice)] bg-[var(--color-bg-muted)] p-3 text-sm">
        <input
          type="checkbox"
          checked={force}
          onChange={(event) => setForce(event.target.checked)}
        />
        Force this manual run if Codex reports 5% or less quota remaining. The
        override is audited.
      </label>

      {run.isError ? (
        <ErrorNotice error={run.error} title="Job Run failed to start" />
      ) : null}
      {toggle.isError ? (
        <ErrorNotice error={toggle.error} title="Schedule update failed" />
      ) : null}

      <JobConfigEditor
        key={detail.job.updated_at}
        detail={detail}
        onSaved={refresh}
      />

      <section>
        <h2 className="text-lg font-semibold">Run history</h2>
        <div className="mt-3 grid gap-2">
          {detail.runs.map((candidate) => (
            <Link
              key={candidate.job_run_id}
              to={`/job-runs/${candidate.job_run_id}`}
              className="grid gap-3 border border-black/20 p-3 hover:bg-[var(--color-bg-muted)] md:grid-cols-[auto_minmax(0,1fr)_auto]"
            >
              <Badge tone={runTone(candidate.state)}>{candidate.state}</Badge>
              <div className="min-w-0 text-sm">
                <div>
                  {candidate.summary ??
                    candidate.terminal_reason?.message ??
                    "No summary yet"}
                </div>
                <div className="mt-1 text-xs text-[var(--color-muted)]">
                  {candidate.trigger} · queued{" "}
                  {new Date(candidate.queued_at).toLocaleString()}
                </div>
              </div>
              <div className="text-xs text-[var(--color-muted)]">
                {candidate.session_thread_id
                  ? "session evidence"
                  : "no provider start"}
              </div>
            </Link>
          ))}
          {detail.runs.length === 0 ? (
            <div className="border border-dashed border-[var(--color-muted)] p-5 text-sm text-[var(--color-muted)]">
              No runs yet. Start a manual test before enabling the schedule.
            </div>
          ) : null}
        </div>
      </section>
    </section>
  );
}

function JobConfigEditor({
  detail,
  onSaved,
}: {
  detail: JobDetail;
  onSaved: () => Promise<void>;
}) {
  const [name, setName] = useState(detail.job.name);
  const [prompt, setPrompt] = useState(detail.prompt);
  const [timezone, setTimezone] = useState(detail.job.timezone);
  const [continueAfterError, setContinueAfterError] = useState(
    detail.job.continue_after_error,
  );
  const save = useMutation({
    mutationFn: () =>
      coreApi.updateJob(detail.job.job_id, {
        name,
        prompt,
        timezone,
        continue_after_error: continueAfterError,
      }),
    onSuccess: onSaved,
  });
  const fieldClass =
    "min-h-10 w-full border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 py-2 text-sm";

  return (
    <form
      className="grid gap-3 border border-black/20 p-4"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate();
      }}
    >
      <h2 className="text-sm font-bold">Future-run configuration</h2>
      <div className="grid gap-3 md:grid-cols-2">
        <label className="grid gap-1 text-xs font-bold text-[var(--color-muted)]">
          Name
          <input
            className={fieldClass}
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label className="grid gap-1 text-xs font-bold text-[var(--color-muted)]">
          IANA timezone
          <input
            className={fieldClass}
            value={timezone}
            onChange={(event) => setTimezone(event.target.value)}
          />
        </label>
      </div>
      <label className="grid gap-1 text-xs font-bold text-[var(--color-muted)]">
        Prompt / task contract
        <textarea
          className={`${fieldClass} min-h-32 resize-y font-normal text-[var(--color-ink)]`}
          value={prompt}
          onChange={(event) => setPrompt(event.target.value)}
        />
      </label>
      <label className="flex items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={continueAfterError}
          onChange={(event) => setContinueAfterError(event.target.checked)}
        />
        Keep schedule enabled after an error
      </label>
      {save.isError ? (
        <ErrorNotice error={save.error} title="Job update failed" />
      ) : null}
      <div>
        <Button type="submit" variant="secondary" disabled={save.isPending}>
          Save future-run configuration
        </Button>
      </div>
    </form>
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
      <div className="mt-1 break-words text-sm">{children}</div>
    </div>
  );
}
