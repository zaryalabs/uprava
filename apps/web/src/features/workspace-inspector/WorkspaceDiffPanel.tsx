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
    <section className="rounded-md border border-[#d9ded4] bg-white">
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-[#e0e5db] px-3 py-2">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-normal text-[#667268]">
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
            <div className="flex flex-wrap items-center gap-2 text-xs text-[#536257]">
              <span>{new Date(diff.generated_at).toLocaleString()}</span>
              {diff.summary_truncated || diff.diff_truncated ? (
                <Badge tone="warn">Truncated</Badge>
              ) : null}
            </div>
            <pre className="max-h-28 overflow-auto whitespace-pre-wrap rounded-md bg-[#f6f8f3] p-3 font-mono text-xs leading-5 text-[#17211c]">
              {diff.summary}
            </pre>
            <Suspense fallback={<Fallback />}>
              <MonacoDiffTextViewer value={diff.diff || "No diff"} />
            </Suspense>
          </>
        ) : (
          <div className="text-sm text-[#536257]">No diff loaded</div>
        )}
      </div>
    </section>
  );
}

function Fallback() {
  return (
    <div className="flex min-h-24 items-center justify-center text-sm text-[#536257]">
      Loading diff
    </div>
  );
}
