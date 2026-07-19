import {
  Activity,
  CircleAlert,
  CircleCheck,
  CircleDashed,
  CircleOff,
  CirclePause,
  CirclePlay,
  CircleStop,
  FolderCheck,
  FolderClock,
  FolderLock,
  FolderX,
  LoaderCircle,
  Radio,
  RadioTower,
  TriangleAlert,
  WifiOff,
  XCircle,
  type LucideIcon,
} from "lucide-react";

import { Badge, type BadgeTone } from "./badge";

export type StatusDimension =
  | "presence"
  | "lifecycle"
  | "attention"
  | "workspace";

export type StatusPresentation = {
  label: string;
  tone: BadgeTone;
  icon: LucideIcon;
};

const mappings: Record<StatusDimension, Record<string, StatusPresentation>> = {
  presence: {
    reachable: { label: "Reachable", tone: "good", icon: RadioTower },
    stale: { label: "Stale", tone: "warn", icon: Radio },
    offline: { label: "Offline", tone: "bad", icon: WifiOff },
    revoked: { label: "Revoked", tone: "neutral", icon: CircleOff },
    ok: { label: "Available", tone: "good", icon: RadioTower },
    error: { label: "Unavailable", tone: "bad", icon: WifiOff },
    pending: { label: "Checking", tone: "warn", icon: LoaderCircle },
  },
  lifecycle: {
    active: { label: "Active", tone: "good", icon: CirclePlay },
    ready: { label: "Ready", tone: "good", icon: CircleCheck },
    running: { label: "Running", tone: "info", icon: Activity },
    enabled: { label: "Enabled", tone: "good", icon: CirclePlay },
    idle: { label: "Idle", tone: "neutral", icon: CircleDashed },
    paused: { label: "Paused", tone: "neutral", icon: CirclePause },
    detached: { label: "Detached", tone: "warn", icon: CircleDashed },
    degraded: { label: "Degraded", tone: "bad", icon: CircleAlert },
    starting: { label: "Starting", tone: "info", icon: LoaderCircle },
    resuming: { label: "Resuming", tone: "info", icon: LoaderCircle },
    queued: { label: "Queued", tone: "neutral", icon: CircleDashed },
    stopping: { label: "Stopping", tone: "warn", icon: CircleStop },
    stopped: { label: "Stopped", tone: "neutral", icon: CircleStop },
    interrupted: { label: "Interrupted", tone: "warn", icon: CircleStop },
    succeeded: { label: "Succeeded", tone: "good", icon: CircleCheck },
    failed: { label: "Failed", tone: "bad", icon: XCircle },
    cancelled: { label: "Cancelled", tone: "neutral", icon: CircleOff },
    timed_out: { label: "Timed out", tone: "bad", icon: CircleAlert },
    skipped: { label: "Skipped", tone: "warn", icon: CircleDashed },
    expired: { label: "Expired", tone: "warn", icon: CircleOff },
    stale: { label: "Stale", tone: "warn", icon: CircleDashed },
    error: { label: "Error", tone: "bad", icon: XCircle },
  },
  attention: {
    clear: { label: "Clear", tone: "neutral", icon: CircleCheck },
    warning: { label: "Needs attention", tone: "warn", icon: TriangleAlert },
    stalled: {
      label: "No recent activity",
      tone: "warn",
      icon: TriangleAlert,
    },
    blocked: { label: "Blocked", tone: "warn", icon: CircleAlert },
    degraded: { label: "Degraded", tone: "bad", icon: CircleAlert },
    error: { label: "Error", tone: "bad", icon: XCircle },
    hard_block: { label: "Blocked", tone: "bad", icon: XCircle },
  },
  workspace: {
    validated: { label: "Validated", tone: "good", icon: FolderCheck },
    pending: { label: "Pending", tone: "neutral", icon: FolderClock },
    read_only: { label: "Read only", tone: "warn", icon: FolderLock },
    missing: { label: "Missing", tone: "bad", icon: FolderX },
    error: { label: "Error", tone: "bad", icon: FolderX },
  },
};

const fallbackIcons: Record<StatusDimension, LucideIcon> = {
  presence: Radio,
  lifecycle: Activity,
  attention: TriangleAlert,
  workspace: FolderClock,
};

const dimensionLabels: Record<StatusDimension, string> = {
  presence: "Presence",
  lifecycle: "Lifecycle",
  attention: "Attention",
  workspace: "Workspace",
};

export function statusPresentation(
  dimension: StatusDimension,
  value: string,
): StatusPresentation {
  const normalized = value.toLowerCase().replaceAll(" ", "_");
  return (
    mappings[dimension][normalized] ?? {
      label: humanize(value),
      tone: "neutral",
      icon: fallbackIcons[dimension],
    }
  );
}

export function StatusIndicator({
  compact = false,
  dimension,
  label,
  showDimension = false,
  value,
}: {
  compact?: boolean;
  dimension: StatusDimension;
  label?: string;
  showDimension?: boolean;
  value: string;
}) {
  const presentation = statusPresentation(dimension, value);
  const Icon = presentation.icon;
  const visibleLabel = label ?? presentation.label;
  const accessibleLabel = `${dimensionLabels[dimension]}: ${visibleLabel}`;

  if (compact) {
    return (
      <span
        role="img"
        aria-label={accessibleLabel}
        title={accessibleLabel}
        className={`uprava-status-icon uprava-status-${presentation.tone}`}
      >
        <Icon size={13} strokeWidth={2.25} aria-hidden="true" />
      </span>
    );
  }

  return (
    <Badge tone={presentation.tone}>
      <Icon size={13} strokeWidth={2.25} aria-hidden="true" />
      <span>
        {showDimension ? `${dimensionLabels[dimension]}: ` : null}
        {visibleLabel}
      </span>
    </Badge>
  );
}

function humanize(value: string) {
  const normalized = value.replaceAll("_", " ");
  return normalized.charAt(0).toUpperCase() + normalized.slice(1);
}
