import { RefreshCw } from "lucide-react";
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
    <section className="flex h-full min-h-0 flex-col bg-[var(--color-bg)]">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[var(--color-muted)] px-3 py-2">
        <div className="text-xs text-[var(--color-muted)]">
          Workspace snapshot
        </div>
        <Button variant="secondary" disabled={isLoading} onClick={onRefresh}>
          <RefreshCw size={15} />
          {isLoading ? "Loading" : "Refresh"}
        </Button>
      </div>
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto p-3">
        {error ? <ErrorNotice error={error} title="Diff unavailable" /> : null}
        {diff ? (
          <>
            <div className="flex flex-wrap items-center gap-2 text-xs text-[var(--color-muted)]">
              <span>{new Date(diff.generated_at).toLocaleString()}</span>
              {diff.summary_truncated || diff.diff_truncated ? (
                <Badge tone="warn">Truncated</Badge>
              ) : null}
            </div>
            <pre className="max-h-24 shrink-0 overflow-auto whitespace-pre-wrap bg-[var(--color-bg-muted)] p-3 font-mono text-xs leading-5 text-[var(--color-ink)]">
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
    <div className="flex min-h-24 flex-1 items-center justify-center text-sm text-[var(--color-muted)]">
      Loading diff
    </div>
  );
}
