import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Clock3, Plus } from "lucide-react";
import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { JobSchedule } from "../../shared/protocol/types";
import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { workspaceJobRoute } from "../workspaces/routes";

export function JobsRoute() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const jobs = useQuery({
    queryKey: queryKeys.jobs,
    queryFn: coreApi.jobs,
    refetchInterval: 2_000,
  });
  const inventory = useQuery({
    queryKey: queryKeys.inventory,
    queryFn: coreApi.inventory,
  });
  const [name, setName] = useState("");
  const [placementId, setPlacementId] = useState("");
  const [prompt, setPrompt] = useState("");
  const [timezone, setTimezone] = useState(
    Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC",
  );
  const [scheduleKind, setScheduleKind] = useState<ScheduleKind>("interval");
  const [intervalMinutes, setIntervalMinutes] = useState(60);
  const [hour, setHour] = useState(2);
  const [minute, setMinute] = useState(0);
  const [weekday, setWeekday] = useState(1);
  const [continueAfterError, setContinueAfterError] = useState(false);
  const create = useMutation({
    mutationFn: () =>
      coreApi.createJob({
        name,
        project_placement_id:
          placementId ||
          inventory.data?.placements[0]?.project_placement_id ||
          "",
        prompt,
        provider: "codex",
        schedule: scheduleFor({
          kind: scheduleKind,
          intervalMinutes,
          hour,
          minute,
          weekday,
        }),
        timezone,
        continue_after_error: continueAfterError,
      }),
    onSuccess: async (detail) => {
      await queryClient.invalidateQueries({ queryKey: queryKeys.jobs });
      navigate(
        workspaceJobRoute(detail.job.project_placement_id, detail.job.job_id),
      );
    },
  });
  const placements = inventory.data?.placements ?? [];
  const selectedPlacementId =
    placementId || placements[0]?.project_placement_id || "";
  const canCreate =
    Boolean(name.trim() && prompt.trim() && selectedPlacementId && timezone) &&
    !create.isPending;

  return (
    <section className="space-y-8">
      <header>
        <div className="zarya-caption">AUTOMATION / DURABLE WORK</div>
        <h1 className="mt-2 text-2xl font-semibold">Background Jobs</h1>
        <p className="mt-2 max-w-3xl text-sm text-[var(--color-muted)]">
          Each run gets its own managed session in the selected workspace. Jobs
          start paused so you can test them manually before enabling a schedule.
        </p>
      </header>

      <form
        className="grid gap-4 border border-[var(--color-muted)] p-4"
        onSubmit={(event) => {
          event.preventDefault();
          create.mutate();
        }}
      >
        <div className="flex items-center gap-2 font-bold">
          <Plus size={16} aria-hidden="true" /> New paused Job
        </div>
        <div className="grid gap-3 md:grid-cols-2">
          <Field label="Name">
            <input
              className={inputClass}
              value={name}
              onChange={(event) => setName(event.target.value)}
              required
            />
          </Field>
          <Field label="Workspace">
            <select
              className={inputClass}
              value={selectedPlacementId}
              onChange={(event) => setPlacementId(event.target.value)}
              required
            >
              {placements.map((placement) => (
                <option
                  key={placement.project_placement_id}
                  value={placement.project_placement_id}
                >
                  {placement.display_name}
                </option>
              ))}
            </select>
          </Field>
        </div>
        <Field label="Prompt / task contract">
          <textarea
            className={`${inputClass} min-h-32 resize-y`}
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            required
          />
        </Field>
        <div className="grid gap-3 md:grid-cols-4">
          <Field label="Schedule">
            <select
              className={inputClass}
              value={scheduleKind}
              onChange={(event) =>
                setScheduleKind(event.target.value as ScheduleKind)
              }
            >
              <option value="interval">Interval</option>
              <option value="daily">Daily</option>
              <option value="weekly">Weekly</option>
              <option value="manual">Manual only</option>
            </select>
          </Field>
          {scheduleKind === "interval" ? (
            <Field label="Every, minutes">
              <input
                className={inputClass}
                type="number"
                min={1}
                value={intervalMinutes}
                onChange={(event) =>
                  setIntervalMinutes(event.target.valueAsNumber)
                }
              />
            </Field>
          ) : null}
          {scheduleKind === "weekly" ? (
            <Field label="Weekday">
              <select
                className={inputClass}
                value={weekday}
                onChange={(event) => setWeekday(Number(event.target.value))}
              >
                {weekdays.map((label, index) => (
                  <option key={label} value={index + 1}>
                    {label}
                  </option>
                ))}
              </select>
            </Field>
          ) : null}
          {scheduleKind === "daily" || scheduleKind === "weekly" ? (
            <Field label="Local time">
              <div className="grid grid-cols-2 gap-2">
                <input
                  aria-label="Hour"
                  className={inputClass}
                  type="number"
                  min={0}
                  max={23}
                  value={hour}
                  onChange={(event) => setHour(event.target.valueAsNumber)}
                />
                <input
                  aria-label="Minute"
                  className={inputClass}
                  type="number"
                  min={0}
                  max={59}
                  value={minute}
                  onChange={(event) => setMinute(event.target.valueAsNumber)}
                />
              </div>
            </Field>
          ) : null}
          <Field label="IANA timezone">
            <input
              className={inputClass}
              value={timezone}
              onChange={(event) => setTimezone(event.target.value)}
              required
            />
          </Field>
        </div>
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={continueAfterError}
            onChange={(event) => setContinueAfterError(event.target.checked)}
          />
          Keep automatic schedule enabled after a failed run
        </label>
        {create.isError ? (
          <ErrorNotice error={create.error} title="Job creation failed" />
        ) : null}
        <div>
          <Button
            type="submit"
            variant="primary"
            disabled={!canCreate || placements.length === 0}
          >
            Create paused Job
          </Button>
        </div>
      </form>

      {jobs.isError ? (
        <ErrorNotice error={jobs.error} title="Jobs load failed" />
      ) : null}
      <div className="grid gap-3">
        {jobs.data?.map((job) => (
          <Link
            key={job.job_id}
            to={workspaceJobRoute(job.project_placement_id, job.job_id)}
            className="grid gap-3 border border-black/20 p-4 hover:bg-[var(--color-bg-muted)] md:grid-cols-[minmax(0,1fr)_auto]"
          >
            <div className="min-w-0">
              <div className="font-bold">{job.name}</div>
              <div className="mt-1 text-sm text-[var(--color-muted)]">
                {job.placement_name} ·{" "}
                {formatSchedule(job.schedule, job.timezone)}
              </div>
              <div className="mt-2 flex flex-wrap gap-2">
                <Badge tone={job.enabled ? "good" : "neutral"}>
                  {job.enabled ? "enabled" : "paused"}
                </Badge>
                {job.latest_run ? (
                  <Badge tone={runTone(job.latest_run.state)}>
                    latest: {job.latest_run.state}
                  </Badge>
                ) : null}
              </div>
            </div>
            <div className="flex items-center gap-2 text-xs text-[var(--color-muted)]">
              <Clock3 size={14} aria-hidden="true" />
              {job.next_run_at
                ? new Date(job.next_run_at).toLocaleString()
                : "No next start"}
            </div>
          </Link>
        ))}
        {jobs.data?.length === 0 ? (
          <div className="border border-dashed border-[var(--color-muted)] p-6 text-sm text-[var(--color-muted)]">
            No Jobs yet. Create one above and run a manual test.
          </div>
        ) : null}
      </div>
    </section>
  );
}

type ScheduleKind = "manual" | "interval" | "daily" | "weekly";

function scheduleFor(input: {
  kind: ScheduleKind;
  intervalMinutes: number;
  hour: number;
  minute: number;
  weekday: number;
}): JobSchedule | null {
  if (input.kind === "manual") return null;
  if (input.kind === "interval") {
    return { kind: "interval", minutes: input.intervalMinutes };
  }
  if (input.kind === "daily") {
    return { kind: "daily", hour: input.hour, minute: input.minute };
  }
  return {
    kind: "weekly",
    weekday: input.weekday,
    hour: input.hour,
    minute: input.minute,
  };
}

export function formatSchedule(schedule: JobSchedule | null, timezone: string) {
  if (!schedule) return "manual only";
  if (schedule.kind === "interval") return `every ${schedule.minutes} min`;
  const time = `${String(schedule.hour).padStart(2, "0")}:${String(schedule.minute).padStart(2, "0")}`;
  if (schedule.kind === "daily") return `daily ${time} ${timezone}`;
  return `${weekdays[schedule.weekday - 1] ?? "weekly"} ${time} ${timezone}`;
}

export function runTone(state: string): "good" | "bad" | "warn" | "neutral" {
  if (state === "succeeded") return "good";
  if (["failed", "timed_out"].includes(state)) return "bad";
  if (state === "skipped") return "warn";
  return "neutral";
}

function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="grid gap-1 text-sm">
      <span className="text-xs font-bold text-[var(--color-muted)]">
        {label}
      </span>
      {children}
    </label>
  );
}

const inputClass =
  "min-h-10 w-full border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-ink)]";
const weekdays = [
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
  "Sunday",
];
