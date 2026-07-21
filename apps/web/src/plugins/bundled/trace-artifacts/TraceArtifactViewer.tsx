import type { ArtifactRendererProps } from "../../visual-renderers";

export function TraceArtifactViewer({
  detail,
  fallback,
}: ArtifactRendererProps) {
  const payload = asRecord(detail.version.payload);
  if (!payload) return fallback;
  const result = asRecord(payload.result);
  const steps = Array.isArray(payload.steps)
    ? payload.steps
    : Array.isArray(result?.steps)
      ? result.steps
      : [];
  const conclusion = text(payload.conclusion) || text(result?.conclusion);
  return (
    <section className="space-y-3 border-l border-[var(--color-border-strong)] bg-[var(--color-bg-raised)] p-3">
      <div className="text-xs font-bold text-[var(--color-muted)]">
        Trace artifact · version {detail.version.version}
      </div>
      {conclusion ? <div className="text-sm">{conclusion}</div> : null}
      {steps.length > 0 ? (
        <ol className="space-y-2 border-l border-[var(--color-border)] pl-3">
          {steps.slice(0, 80).map((step, index) => (
            <li key={stepKey(step, index)} className="text-xs">
              {stepSummary(step)}
            </li>
          ))}
        </ol>
      ) : (
        fallback
      )}
    </section>
  );
}

function stepKey(value: unknown, index: number) {
  const record = asRecord(value);
  return text(record?.step_id) || text(record?.block_id) || `step-${index}`;
}

function stepSummary(value: unknown) {
  const record = asRecord(value);
  return text(record?.summary) || text(record?.title) || "Trace step";
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : null;
}

function text(value: unknown) {
  return typeof value === "string" ? value : "";
}
