import { GitCompare, RefreshCw } from "lucide-react";
import { lazy, Suspense } from "react";

import { Badge } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";

const MonacoDiffTextViewer = lazy(() =>
  import("./MonacoViews").then((module) => ({
    default: module.MonacoDiffTextViewer,
  })),
);

export function WorkspaceDiffPanel({
  isLoading,
  error,
  diff,
  onRefresh,
}: {
  isLoading: boolean;
  error: unknown;
  diff: {
    summary: string;
    diff: string;
    summary_truncated: boolean;
    diff_truncated: boolean;
    generated_at: string;
  } | null;
  onRefresh: () => void;
}) {
  return (
    <section className="border border-[var(--color-muted)] bg-[var(--color-bg)]">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-muted)] px-3 py-2">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[var(--color-muted)]">
          <GitCompare size={15} />
          Diff
        </div>
        <Button variant="secondary" disabled={isLoading} onClick={onRefresh}>
          <RefreshCw size={15} />
          {isLoading ? "Loading" : "Refresh"}
        </Button>
      </div>
      <div className="space-y-3 p-3">
        {error ? <ErrorNotice error={error} title="Diff unavailable" /> : null}
        {diff ? (
          <>
            <div className="flex flex-wrap items-center gap-2 text-xs text-[var(--color-muted)]">
              <span>{new Date(diff.generated_at).toLocaleString()}</span>
              {diff.summary_truncated || diff.diff_truncated ? (
                <Badge tone="warn">Truncated</Badge>
              ) : null}
            </div>
            <pre className="max-h-28 overflow-auto whitespace-pre-wrap bg-[var(--color-bg-muted)] p-3 font-mono text-xs leading-5 text-[var(--color-ink)]">
              {diff.summary}
            </pre>
            <Suspense fallback={<Fallback />}>
              <MonacoDiffTextViewer value={diff.diff || "No diff"} />
            </Suspense>
          </>
        ) : (
          <div className="text-sm text-[var(--color-muted)]">
            No diff loaded
          </div>
        )}
      </div>
    </section>
  );
}

function Fallback() {
  return (
    <div className="flex min-h-24 items-center justify-center text-sm text-[var(--color-muted)]">
      Loading diff
    </div>
  );
}
