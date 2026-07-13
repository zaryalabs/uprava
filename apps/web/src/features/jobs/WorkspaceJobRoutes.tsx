import { useQuery } from "@tanstack/react-query";
import { Navigate, useLocation, useParams } from "react-router-dom";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { LoadingState } from "../../shared/ui/system";
import { useInventory } from "../inventory/api";
import {
  lastWorkspaceId,
  routeWithSearch,
  workspaceJobRoute,
  workspaceJobRunRoute,
  workspaceJobsRoute,
} from "../workspaces/routes";

export function JobsCompatibilityRoute() {
  const inventory = useInventory();
  const location = useLocation();

  if (inventory.isError && !inventory.data) {
    return (
      <ErrorNotice error={inventory.error} title="Inventory load failed" />
    );
  }
  if (!inventory.data) return <LoadingState stage="Resolving Jobs workspace" />;
  const preferredId = lastWorkspaceId();
  const placement = inventory.data.placements.find(
    (candidate) => candidate.project_placement_id === preferredId,
  );
  return (
    <Navigate
      replace
      to={routeWithSearch(
        placement
          ? workspaceJobsRoute(placement.project_placement_id)
          : "/dashboard",
        location.search,
      )}
    />
  );
}

export function JobCompatibilityRoute() {
  const { jobId = "" } = useParams();
  const location = useLocation();
  const job = useQuery({
    queryKey: queryKeys.job(jobId),
    queryFn: () => coreApi.job(jobId),
    enabled: Boolean(jobId),
  });

  if (job.isError) {
    return <ErrorNotice error={job.error} title="Job load failed" />;
  }
  if (!job.data) return <LoadingState stage="Resolving Job" />;
  return (
    <Navigate
      replace
      to={routeWithSearch(
        workspaceJobRoute(job.data.job.project_placement_id, jobId),
        location.search,
      )}
    />
  );
}

export function JobRunCompatibilityRoute() {
  const { jobRunId = "" } = useParams();
  const location = useLocation();
  const run = useQuery({
    queryKey: queryKeys.jobRun(jobRunId),
    queryFn: () => coreApi.jobRun(jobRunId),
    enabled: Boolean(jobRunId),
  });
  const jobId = run.data?.job_id ?? "";
  const job = useQuery({
    queryKey: queryKeys.job(jobId),
    queryFn: () => coreApi.job(jobId),
    enabled: Boolean(jobId),
  });

  if (run.isError) {
    return <ErrorNotice error={run.error} title="Job Run load failed" />;
  }
  if (job.isError) {
    return <ErrorNotice error={job.error} title="Job load failed" />;
  }
  if (!run.data || !job.data) return <LoadingState stage="Resolving Job Run" />;
  return (
    <Navigate
      replace
      to={routeWithSearch(
        workspaceJobRunRoute(
          job.data.job.project_placement_id,
          jobId,
          jobRunId,
        ),
        location.search,
      )}
    />
  );
}
