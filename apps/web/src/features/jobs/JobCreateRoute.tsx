import { useMutation, useQueryClient } from "@tanstack/react-query";
import { Plus } from "lucide-react";
import { useState } from "react";
import { useLocation, useNavigate } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { JobSchedule } from "../../shared/protocol/types";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { routeWithSearch, workspaceJobRoute } from "../workspaces/routes";
import {
  Field,
  jobInputClass,
  useWorkspaceJobsContext,
  weekdays,
} from "./JobsRoute";

type ScheduleKind = "manual" | "interval" | "daily" | "weekly";

export function JobCreateRoute() {
  const { placement } = useWorkspaceJobsContext();
  const navigate = useNavigate();
  const location = useLocation();
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
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
        project_placement_id: placement.project_placement_id,
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
        routeWithSearch(
          workspaceJobRoute(placement.project_placement_id, detail.job.job_id),
          location.search,
        ),
      );
    },
  });
  const canCreate =
    Boolean(name.trim() && prompt.trim() && timezone.trim()) &&
    !create.isPending;

  return (
    <section className="space-y-5" aria-labelledby="new-job-title">
      <header>
        <div className="zarya-caption">NEW WORKSPACE JOB</div>
        <h3 id="new-job-title" className="mt-1 text-lg font-semibold">
          New paused Job
        </h3>
        <p className="mt-1 text-sm text-[var(--color-muted)]">
          Runs in {placement.display_name}. Test it manually before enabling the
          schedule.
        </p>
      </header>
      <form
        className="grid gap-4 border border-[var(--color-border-strong)] p-4"
        onSubmit={(event) => {
          event.preventDefault();
          create.mutate();
        }}
      >
        <div className="flex items-center gap-2 font-bold">
          <Plus size={16} aria-hidden="true" /> Configuration
        </div>
        <Field label="Name">
          <input
            className={jobInputClass}
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
          />
        </Field>
        <Field label="Prompt / task contract">
          <textarea
            className={`${jobInputClass} min-h-32 resize-y`}
            value={prompt}
            onChange={(event) => setPrompt(event.target.value)}
            required
          />
        </Field>
        <div className="grid gap-3 md:grid-cols-3">
          <Field label="Schedule">
            <select
              className={jobInputClass}
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
                className={jobInputClass}
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
                className={jobInputClass}
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
                  className={jobInputClass}
                  type="number"
                  min={0}
                  max={23}
                  value={hour}
                  onChange={(event) => setHour(event.target.valueAsNumber)}
                />
                <input
                  aria-label="Minute"
                  className={jobInputClass}
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
              className={jobInputClass}
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
          <Button type="submit" variant="primary" disabled={!canCreate}>
            Create paused Job
          </Button>
        </div>
      </form>
    </section>
  );
}

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
