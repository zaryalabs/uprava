import { useState, type ComponentType, type ReactNode } from "react";
import {
  Activity,
  AlertTriangle,
  Bot,
  CheckCircle2,
  CircleDot,
  HelpCircle,
  User,
} from "lucide-react";

import { Badge } from "../../shared/ui/badge";
import { DisclosureControl } from "../../shared/ui/system";
import type { UiBlock } from "./types";

type BadgeTone = "neutral" | "good" | "warn" | "bad" | "info";

export type BlockRendererProps = {
  block: UiBlock;
  actions?: ReactNode;
};

export type BlockFallbackProps = BlockRendererProps;

export type BlockRendererRegistration = {
  type: string;
  supportedSchemaVersions: number[];
  allowedSurfaces: string[];
  render: ComponentType<BlockRendererProps>;
  fallback: ComponentType<BlockFallbackProps>;
};

const rendererRegistrations: BlockRendererRegistration[] = [
  register("core.user-message", MessageBlock),
  register("core.assistant-message", MessageBlock),
  register("core.turn-activity", TurnActivityBlock),
  register("core.provider-output-stream", EventBlock),
  register("core.runtime-event", EventBlock),
  register("core.workspace-validation", EventBlock),
  register("core.resource-snapshot", EventBlock),
  register("core.warning", WarningBlock),
  register("core.error", ErrorBlock),
  register("core.approval-request", ApprovalBlock),
  register("core.unknown", UnknownBlock),
];

const rendererByType = new Map(
  rendererRegistrations.map((registration) => [
    registration.type,
    registration,
  ]),
);

export function TimelineBlockRenderer({ block, actions }: BlockRendererProps) {
  const registration = rendererByType.get(block.type);
  if (
    !registration ||
    !registration.supportedSchemaVersions.includes(block.schema_version)
  ) {
    const Fallback = registration?.fallback ?? UnknownBlock;
    return <Fallback block={block} actions={actions} />;
  }
  const Renderer = registration.render;
  return <Renderer block={block} actions={actions} />;
}

export function getTimelineBlockRenderer(type: string) {
  return rendererByType.get(type);
}

export function registeredTimelineBlockTypes() {
  return rendererRegistrations.map((registration) => registration.type);
}

function register(
  type: string,
  render: ComponentType<BlockRendererProps>,
  fallback: ComponentType<BlockFallbackProps> = UnknownBlock,
): BlockRendererRegistration {
  return {
    type,
    supportedSchemaVersions: [1],
    allowedSurfaces: ["session.timeline"],
    render,
    fallback,
  };
}

function MessageBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);
  const isAssistant = block.type === "core.assistant-message";
  const label = isAssistant ? "Agent Output" : "Operator Input";
  const Icon = isAssistant ? Bot : User;

  return (
    <article
      className={`border-l-2 py-3 pl-3 ${isAssistant ? "border-[var(--color-ink)]" : "border-[var(--color-muted)]"}`}
    >
      <div
        className={
          isAssistant
            ? "mb-1 flex items-center gap-1.5 text-xs font-bold text-[var(--color-muted)]"
            : "mb-1 flex items-center gap-1.5 text-xs font-bold text-[var(--color-muted)]"
        }
      >
        <Icon size={14} />
        {label}
      </div>
      <p className="whitespace-pre-wrap break-words text-sm">
        {stringField(data, "content", block.fallback_text ?? "")}
      </p>
      {isAssistant ? (
        <div className="mt-2 text-xs text-[var(--color-muted)]">
          Evidence & source are available through the <strong>+</strong>{" "}
          reference layer.
        </div>
      ) : null}
      <BlockActions actions={actions} />
    </article>
  );
}

function ApprovalBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);

  return (
    <article className="border-l-2 border-[var(--color-muted)] py-3 pl-3">
      <div className="mb-2 flex items-center gap-2">
        <Badge tone="warn">
          <CircleDot size={13} />
          Approval
        </Badge>
        <span className="font-mono text-xs text-[var(--color-muted)]">
          {stringField(data, "approvalId", "pending")}
        </span>
      </div>
      <p className="whitespace-pre-wrap break-words text-sm">
        {stringField(
          data,
          "prompt",
          block.fallback_text ?? "Approval requested",
        )}
      </p>
      <dl className="mt-3 grid gap-2 text-xs text-[var(--color-muted)] sm:grid-cols-3">
        <div>
          <dt>Affected Scope</dt>
          <dd className="text-[var(--color-ink)]">Current runtime</dd>
        </div>
        <div>
          <dt>Risk</dt>
          <dd className="text-[var(--color-ink)]">Command-dependent</dd>
        </div>
        <div>
          <dt>Reversibility</dt>
          <dd className="text-[var(--color-ink)]">Review before approval</dd>
        </div>
      </dl>
      <BlockActions actions={actions} />
    </article>
  );
}

function TurnActivityBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);
  const rows = arrayField(data, "rows").filter(isRecord);
  const completed = booleanField(data, "completed", false);
  const [manualExpanded, setManualExpanded] = useState<boolean | null>(null);
  const expanded = manualExpanded ?? !completed;
  const durationMs = numberField(data, "durationMs", 0);

  return (
    <article className="border-l-2 border-[var(--color-muted)] py-3 pl-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex min-w-0 items-center gap-2">
          <DisclosureControl
            expanded={expanded}
            label="activity"
            onClick={() => setManualExpanded(!expanded)}
          />
          <div className="min-w-0">
            <div className="flex items-center gap-1.5 text-xs font-bold text-[var(--color-muted)]">
              <Activity size={14} />
              Turn Activity
            </div>
            <div className="truncate font-mono text-xs text-[var(--color-muted)]">
              {stringField(data, "turnId", "turn")}
            </div>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <ActivityCounter label="events" value={rows.length} />
          <ActivityCounter
            label="commands"
            value={numberField(data, "commandCount", 0)}
          />
          <ActivityCounter
            label="files"
            value={numberField(data, "fileChangeCount", 0)}
          />
          <ActivityCounter
            label="reasoning"
            value={numberField(data, "reasoningCount", 0)}
          />
          <ActivityCounter
            label="warnings"
            value={numberField(data, "warningErrorCount", 0)}
          />
          {durationMs > 0 ? (
            <ActivityCounter label="time" value={formatDuration(durationMs)} />
          ) : null}
        </div>
      </div>
      {expanded ? (
        <div className="mt-3 max-h-80 overflow-y-auto border-t border-[var(--color-muted)] pt-2">
          <div className="space-y-2">
            {rows.map((row, index) => (
              <ActivityRow
                key={stringField(row, "eventId", `activity-row-${index}`)}
                row={row}
              />
            ))}
          </div>
        </div>
      ) : null}
      <BlockActions actions={actions} />
    </article>
  );
}

function ActivityCounter({
  label,
  value,
}: {
  label: string;
  value: number | string;
}) {
  return (
    <span className="border border-[var(--color-muted)] bg-[var(--color-bg)] px-1.5 py-0.5 text-xs text-[var(--color-muted)]">
      <span className="font-mono">{value}</span> {label}
    </span>
  );
}

function ActivityRow({ row }: { row: Record<string, unknown> }) {
  const phase = stringField(row, "phase", "observed");
  const status = stringField(row, "status", phase);
  const rawText = rawActivityText(row);

  return (
    <div className="grid gap-1 border-b border-[var(--color-muted)] pb-2 last:border-b-0">
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <span className="font-mono text-[var(--color-muted)]">
          seq {numberField(row, "seq", 0)}
        </span>
        <Badge tone={activityTone(status)}>
          {stringField(row, "providerEventType", "provider.activity")}
        </Badge>
        <span className="text-[var(--color-muted)]">{status}</span>
        {stringField(row, "providerItemType", "") ? (
          <span className="font-mono text-[var(--color-muted)]">
            {stringField(row, "providerItemType", "")}
          </span>
        ) : null}
      </div>
      <div className="break-words text-sm text-[var(--color-ink)]">
        {stringField(row, "summary", "Provider activity")}
      </div>
      {rawText ? (
        <details className="text-xs text-[var(--color-muted)]">
          <summary className="cursor-pointer select-none">Raw</summary>
          <pre className="mt-1 max-h-56 overflow-auto border-l border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-2 font-mono text-[11px] leading-4 text-[var(--color-ink)]">
            {rawText}
          </pre>
        </details>
      ) : null}
    </div>
  );
}

function EventBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);
  const eventKind = stringField(data, "eventKind", block.type);

  return (
    <article className="border-l-2 border-[var(--color-muted)] py-3 pl-3">
      <div className="mb-1 flex flex-wrap items-center gap-2">
        <Badge tone={eventTone(block.type)}>
          <CheckCircle2 size={13} />
          {eventKind}
        </Badge>
        <span className="font-mono text-xs text-[var(--color-muted)]">
          seq {numberField(data, "seq", 0)}
        </span>
      </div>
      <div className="break-words text-sm text-[var(--color-muted)]">
        {stringField(data, "summary", block.fallback_text ?? eventKind)}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function WarningBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);

  return (
    <article className="border-l-2 border-[var(--color-muted)] py-3 pl-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
        <AlertTriangle size={14} />
        {stringField(data, "eventKind", "Warning")}
      </div>
      <div className="break-words text-sm text-[var(--color-muted)]">
        {stringField(data, "summary", block.fallback_text ?? "Warning")}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function ErrorBlock({ block, actions }: BlockRendererProps) {
  const data = blockData(block);

  return (
    <article className="border-l-2 border-[var(--color-risk)] bg-[var(--color-risk-soft)] py-3 pl-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-normal text-[var(--color-risk)]">
        <AlertTriangle size={14} />
        {stringField(data, "eventKind", "Error")}
      </div>
      <div className="break-words text-sm text-[var(--color-muted)]">
        {stringField(data, "summary", block.fallback_text ?? "Runtime error")}
      </div>
      <div className="mt-2 text-xs text-[var(--color-risk)]">
        Affected scope: current runtime. Next safe step: inspect the source
        event, then retry or stop.
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function UnknownBlock({ block, actions }: BlockFallbackProps) {
  return (
    <article className="border-l-2 border-dashed border-[var(--color-muted)] py-3 pl-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-semibold uppercase tracking-normal text-[var(--color-muted)]">
        <HelpCircle size={14} />
        Unknown
      </div>
      <div className="break-words text-sm">
        {block.fallback_text || block.type}
      </div>
      <BlockActions actions={actions} />
    </article>
  );
}

function BlockActions({ actions }: { actions?: ReactNode }) {
  return actions ? (
    <div className="mt-3 flex flex-wrap items-center gap-2">{actions}</div>
  ) : null;
}

function eventTone(type: string): BadgeTone {
  if (
    type === "core.workspace-validation" ||
    type === "core.resource-snapshot"
  ) {
    return "good";
  }
  if (type === "core.provider-output-stream") return "info";
  return "warn";
}

function blockData(block: UiBlock): Record<string, unknown> {
  return typeof block.data === "object" && block.data !== null
    ? (block.data as Record<string, unknown>)
    : {};
}

function stringField(
  data: Record<string, unknown>,
  field: string,
  fallback: string,
) {
  const value = data[field];
  return typeof value === "string" && value.length > 0 ? value : fallback;
}

function numberField(
  data: Record<string, unknown>,
  field: string,
  fallback: number,
) {
  const value = data[field];
  return typeof value === "number" ? value : fallback;
}

function booleanField(
  data: Record<string, unknown>,
  field: string,
  fallback: boolean,
) {
  const value = data[field];
  return typeof value === "boolean" ? value : fallback;
}

function arrayField(data: Record<string, unknown>, field: string) {
  const value = data[field];
  return Array.isArray(value) ? value : [];
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function activityTone(status: string): BadgeTone {
  const normalized = status.toLowerCase();
  if (
    normalized.includes("error") ||
    normalized.includes("failed") ||
    normalized.includes("warning")
  ) {
    return normalized.includes("warning") ? "warn" : "bad";
  }
  if (normalized.includes("completed")) return "good";
  return "info";
}

function rawActivityText(row: Record<string, unknown>) {
  const rawEvent = row.rawEvent;
  if (rawEvent !== undefined) return formatRawValue(rawEvent);
  const preview = row.rawEventPreview;
  return typeof preview === "string" ? preview : null;
}

function formatRawValue(value: unknown) {
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function formatDuration(durationMs: number) {
  if (durationMs < 1000) return `${durationMs}ms`;
  const seconds = durationMs / 1000;
  if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 1 : 0)}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.round(seconds % 60);
  return `${minutes}m ${remainingSeconds}s`;
}
