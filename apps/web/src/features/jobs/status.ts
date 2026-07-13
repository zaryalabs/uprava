import type { JobRunState } from "../../shared/protocol/types";

const activeJobRunStates = new Set<JobRunState>([
  "queued",
  "starting",
  "running",
]);

export function isJobRunActive(state: JobRunState) {
  return activeJobRunStates.has(state);
}
