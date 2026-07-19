export const SESSION_ACTIVITY_STALE_AFTER_MS = 30_000;

export function sessionAttention(
  sessionState: string,
  runtimeState: string,
  degradedReason?: string | null,
  lastRuntimeStepAt?: string | null,
  now = Date.now(),
) {
  if (sessionState === "degraded" || runtimeState === "error") {
    return "degraded";
  }
  if (runtimeState === "blocked") return "blocked";
  if (
    runtimeState === "stale" ||
    runtimeState === "expired" ||
    Boolean(degradedReason)
  ) {
    return "warning";
  }
  if (
    runtimeState === "running" &&
    lastRuntimeStepAt &&
    now - Date.parse(lastRuntimeStepAt) >= SESSION_ACTIVITY_STALE_AFTER_MS
  ) {
    return "stalled";
  }
  return "clear";
}
