import { useQuery } from "@tanstack/react-query";
import { Clock3, Plus } from "lucide-react";
import type { ReactNode } from "react";
import {
  Link,
  NavLink,
  Outlet,
  useLocation,
  useOutletContext,
  useParams,
} from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type { JobSchedule, JobSummary } from "../../shared/protocol/types";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { StatusIndicator } from "../../shared/ui/status-indicator";
import { EmptyState, LoadingState } from "../../shared/ui/system";
import {
  type WorkspaceOutletContext,
  useWorkspaceContext,
} from "../workspaces/WorkspaceLayout";
import {
  routeWithSearch,
  workspaceJobNewRoute,
  workspaceJobRoute,
} from "../workspaces/routes";

type WorkspaceJobsOutletContext = WorkspaceOutletContext & {
  jobs: JobSummary[];
};

export function JobsRoute() {
  const workspace = useWorkspaceContext();
  const location = useLocation();
  const { jobId } = useParams();
  const placementId = workspace.placement.project_placement_id;
  const jobs = useQuery({
    queryKey: queryKeys.jobs,
    queryFn: coreApi.jobs,
    refetchInterval: 2_000,
  });
  const workspaceJobs = filterJobsByPlacement(jobs.data ?? [], placementId);
  const context: WorkspaceJobsOutletContext = {
    ...workspace,
    jobs: workspaceJobs,
  };

  return (
    <section className="space-y-4" aria-labelledby="workspace-jobs-title">
      <header className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <div className="zarya-caption">WORKSPACE AUTOMATION</div>
          <h2 id="workspace-jobs-title" className="mt-1 text-xl font-bold">
            Background Jobs
          </h2>
          <p className="mt-1 max-w-2xl text-sm text-[var(--color-muted)]">
            Durable scheduled and manual agent work for this workspace.
          </p>
        </div>
        <Link
          className="inline-flex h-9 items-center justify-center gap-2 border border-[var(--color-ink)] bg-[var(--color-ink)] px-3 text-sm font-medium text-[var(--color-bg)] hover:opacity-80"
          to={routeWithSearch(
            workspaceJobNewRoute(placementId),
            location.search,
          )}
        >
          <Plus size={16} aria-hidden="true" />
          Create Job
        </Link>
      </header>

      <div className="uprava-jobs-grid">
        <aside className="uprava-jobs-list" aria-label="Workspace Jobs">
          <div className="flex items-center justify-between gap-3 border-b border-black/10 pb-3">
            <div className="zarya-label">JOBS</div>
            <span className="text-xs text-[var(--color-muted)]">
              {workspaceJobs.length}
            </span>
          </div>
          {jobs.isError ? (
            <ErrorNotice error={jobs.error} title="Jobs load failed" />
          ) : null}
          {jobs.isPending ? <LoadingState stage="Loading Jobs" /> : null}
          {workspaceJobs.length > 0 ? (
            <nav className="mt-3" aria-label="Jobs">
              <ul className="space-y-1">
                {workspaceJobs.map((job) => (
                  <li key={job.job_id}>
                    <JobListLink
                      job={job}
                      selected={job.job_id === jobId}
                      to={routeWithSearch(
                        workspaceJobRoute(placementId, job.job_id),
                        location.search,
                      )}
                    />
                  </li>
                ))}
              </ul>
            </nav>
          ) : jobs.isSuccess ? (
            <div className="py-5 text-xs text-[var(--color-muted)]">
              No Jobs in this workspace.
            </div>
          ) : null}
        </aside>
        <div className="uprava-jobs-content">
          <Outlet context={context} />
        </div>
      </div>
    </section>
  );
}

export function WorkspaceJobsIndexRoute() {
  const { jobs, placement } = useWorkspaceJobsContext();
  const location = useLocation();

  if (jobs.length === 0) {
    return (
      <div className="grid min-h-80 place-items-center border border-dashed border-black/20 p-8 text-center">
        <div>
          <EmptyState
            title="No Jobs yet"
            detail="Create a paused Job, run a manual test, then enable its schedule when the result is ready."
          />
          <Link
            className="mt-2 inline-flex h-9 items-center justify-center gap-2 border border-[var(--color-ink)] bg-[var(--color-ink)] px-3 text-sm font-medium text-[var(--color-bg)] hover:opacity-80"
            to={routeWithSearch(
              workspaceJobNewRoute(placement.project_placement_id),
              location.search,
            )}
          >
            <Plus size={16} aria-hidden="true" />
            Create Job
          </Link>
        </div>
      </div>
    );
  }

  return (
    <div className="grid min-h-80 place-items-center border border-dashed border-black/20 p-8 text-center">
      <EmptyState
        title="Select a Job"
        detail="Choose a Job to inspect its configuration and run history, or create a new paused Job."
      />
    </div>
  );
}

export function useWorkspaceJobsContext() {
  return useOutletContext<WorkspaceJobsOutletContext>();
}

export function filterJobsByPlacement(jobs: JobSummary[], placementId: string) {
  return jobs.filter((job) => job.project_placement_id === placementId);
}

function JobListLink({
  job,
  selected,
  to,
}: {
  job: JobSummary;
  selected: boolean;
  to: string;
}) {
  return (
    <NavLink
      to={to}
      aria-current={selected ? "page" : undefined}
      className={`block border p-3 ${
        selected
          ? "border-[var(--color-ink)] bg-[var(--color-bg)]"
          : "border-transparent hover:border-black/20 hover:bg-[var(--color-bg)]"
      }`}
    >
      <span className="block truncate text-sm font-medium">{job.name}</span>
      <span className="mt-1 block text-xs text-[var(--color-muted)]">
        {formatSchedule(job.schedule, job.timezone)}
      </span>
      <span className="mt-2 flex flex-wrap items-center gap-1">
        <StatusIndicator
          dimension="lifecycle"
          value={job.enabled ? "enabled" : "paused"}
        />
        {job.latest_run ? (
          <StatusIndicator dimension="lifecycle" value={job.latest_run.state} />
        ) : null}
      </span>
      <span className="mt-2 flex items-center gap-1 text-xs text-[var(--color-muted)]">
        <Clock3 size={13} aria-hidden="true" />
        {job.next_run_at
          ? new Date(job.next_run_at).toLocaleString()
          : "No next start"}
      </span>
    </NavLink>
  );
}

export function formatSchedule(schedule: JobSchedule | null, timezone: string) {
  if (!schedule) return "manual only";
  if (schedule.kind === "interval") return `every ${schedule.minutes} min`;
  const time = `${String(schedule.hour).padStart(2, "0")}:${String(schedule.minute).padStart(2, "0")}`;
  if (schedule.kind === "daily") return `daily ${time} ${timezone}`;
  return `${weekdays[schedule.weekday - 1] ?? "weekly"} ${time} ${timezone}`;
}

export function Field({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
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

export const jobInputClass =
  "min-h-10 w-full border border-[var(--color-muted)] bg-[var(--color-bg)] px-3 py-2 text-sm text-[var(--color-ink)]";

export const weekdays = [
  "Monday",
  "Tuesday",
  "Wednesday",
  "Thursday",
  "Friday",
  "Saturday",
  "Sunday",
];
