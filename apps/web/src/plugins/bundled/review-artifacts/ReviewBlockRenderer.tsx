import { AlertTriangle, CheckCircle2 } from "lucide-react";

import type { PluginBlockRendererProps } from "../../visual-renderers";

export function ReviewBlockRenderer({
  block,
  actions,
}: PluginBlockRendererProps) {
  const data = asRecord(block.data);
  const summary = text(data.summary) || block.fallback_text || block.type;
  const failed = /fail|error/i.test(summary);
  const Icon = failed ? AlertTriangle : CheckCircle2;
  return (
    <article className="border-l-2 border-[var(--color-muted)] py-3 pl-3">
      <div className="mb-1 flex items-center gap-2 text-xs font-bold text-[var(--color-muted)]">
        <Icon size={14} aria-hidden="true" />
        Review evidence
      </div>
      <div className="break-words text-sm text-[var(--color-ink)]">
        {summary}
      </div>
      {actions ? (
        <div className="mt-3 flex flex-wrap gap-2">{actions}</div>
      ) : null}
    </article>
  );
}

function asRecord(value: unknown) {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : {};
}

function text(value: unknown) {
  return typeof value === "string" ? value : "";
}
