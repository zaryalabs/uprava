import { Activity, CheckCircle2, LoaderCircle } from "lucide-react";

import type { PluginBlockRendererProps } from "../../visual-renderers";

export function TraceBlockRenderer({
  block,
  actions,
}: PluginBlockRendererProps) {
  const data = asRecord(block.data);
  const rows = Array.isArray(data.rows) ? data.rows : [];
  const completed = data.completed === true;
  const Icon = completed ? CheckCircle2 : LoaderCircle;
  return (
    <article className="border-l-2 border-[var(--color-muted)] py-3 pl-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-xs font-bold text-[var(--color-muted)]">
          <Icon size={14} aria-hidden="true" />
          {completed ? "Observed activity" : "Agent is working"}
        </div>
        <div className="inline-flex items-center gap-1 border border-[var(--color-border)] px-2 py-1 font-mono text-xs text-[var(--color-muted)]">
          <Activity size={12} aria-hidden="true" /> {rows.length}
        </div>
      </div>
      {rows.length > 0 ? (
        <div className="mt-3 space-y-2 border-t border-[var(--color-border)] pt-2">
          {rows.slice(-8).map((row, index) => (
            <div
              key={rowKey(row, index)}
              className="text-xs text-[var(--color-ink)]"
            >
              {rowSummary(row)}
            </div>
          ))}
        </div>
      ) : null}
      {actions ? (
        <div className="mt-3 flex flex-wrap gap-2">{actions}</div>
      ) : null}
    </article>
  );
}

function rowKey(value: unknown, index: number) {
  const record = asRecord(value);
  return text(record.eventId) || `trace-row-${index}`;
}

function rowSummary(value: unknown) {
  const record = asRecord(value);
  return text(record.summary) || text(record.eventKind) || "Observed event";
}

function asRecord(value: unknown) {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : {};
}

function text(value: unknown) {
  return typeof value === "string" ? value : "";
}
