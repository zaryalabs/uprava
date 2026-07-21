import type { ArtifactRendererProps } from "../../visual-renderers";

export function ReviewArtifactViewer({
  detail,
  fallback,
}: ArtifactRendererProps) {
  const payload = asRecord(detail.version.payload);
  if (!payload) return fallback;
  const summary = text(payload.summary) || detail.version.fallback_text;
  const diff = text(payload.diff);
  const checks = Array.isArray(payload.checks) ? payload.checks : [];

  return (
    <section className="space-y-3 border-l border-[var(--color-border-strong)] bg-[var(--color-bg-raised)] p-3">
      <div>
        <div className="text-xs font-bold text-[var(--color-muted)]">
          {detail.artifact.artifact_type === "uprava.diff-report"
            ? "Diff report"
            : "Check report"}
        </div>
        <div className="mt-1 text-sm text-[var(--color-ink)]">{summary}</div>
      </div>
      {diff ? (
        <pre className="max-h-80 overflow-auto border-l border-[var(--color-muted)] bg-[var(--color-bg-muted)] p-2 font-mono text-xs text-[var(--color-ink)]">
          {diff}
        </pre>
      ) : null}
      {checks.length > 0 ? (
        <div className="space-y-2">
          {checks.map((check, index) => (
            <div
              key={checkKey(check, index)}
              className="border-t border-[var(--color-border)] pt-2 text-xs"
            >
              {checkSummary(check)}
            </div>
          ))}
        </div>
      ) : null}
      <div className="font-mono text-xs text-[var(--color-muted)]">
        version {detail.version.version}
      </div>
    </section>
  );
}

function checkKey(value: unknown, index: number) {
  const record = asRecord(value);
  return text(record?.command_id) || `check-${index}`;
}

function checkSummary(value: unknown) {
  const record = asRecord(value);
  if (!record) return "Unknown check";
  const label = text(record.label) || text(record.command) || "Check";
  const success = record.success;
  return `${label} · ${success === true ? "passed" : success === false ? "failed" : "unknown"}`;
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : null;
}

function text(value: unknown) {
  return typeof value === "string" ? value : "";
}
