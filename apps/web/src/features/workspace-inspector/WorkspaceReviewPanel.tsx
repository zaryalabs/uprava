import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, Pin, RefreshCw, Square, XCircle } from "lucide-react";
import { lazy, Suspense, useEffect, useState } from "react";

import { coreApi } from "../../shared/api/http-client";
import { queryKeys } from "../../shared/api/query-keys";
import type {
  GitChangeKind,
  GitChangedFile,
  WorkspaceDiffScope,
  WorkspaceReviewProjection,
} from "../../shared/protocol/types";
import { Badge, type BadgeTone } from "../../shared/ui/badge";
import { Button } from "../../shared/ui/button";
import { ErrorNotice } from "../../shared/ui/error-notice";
import { ReferenceActions } from "../../workbench/references/ReferenceActions";

const MonacoWorkspaceDiffViewer = lazy(() =>
  import("./MonacoViews").then((module) => ({
    default: module.MonacoWorkspaceDiffViewer,
  })),
);

const terminalStates = new Set(["completed", "failed", "blocked", "expired"]);

export function WorkspaceReviewPanel({
  placementId,
  isLoading,
  error,
  review,
  scope,
  selectedPath,
  onScopeChange,
  onSelectPath,
  onOpenSource,
  onRefresh,
}: {
  placementId: string;
  isLoading: boolean;
  error: unknown;
  review: WorkspaceReviewProjection | null;
  scope: WorkspaceDiffScope;
  selectedPath: string | null;
  onScopeChange: (scope: WorkspaceDiffScope) => void;
  onSelectPath: (path: string) => void;
  onOpenSource: (path: string) => void;
  onRefresh: () => void;
}) {
  const queryClient = useQueryClient();
  const [activeCheckId, setActiveCheckId] = useState<string | null>(null);
  const activeCheck = useQuery({
    queryKey: queryKeys.workspaceCommandResource(
      placementId,
      activeCheckId ?? "",
    ),
    queryFn: () =>
      coreApi.workspaceCommandResource(placementId, activeCheckId ?? ""),
    enabled: Boolean(activeCheckId),
    refetchInterval: (query) =>
      query.state.data && terminalStates.has(query.state.data.state)
        ? false
        : 1_000,
  });
  const runCheck = useMutation({
    mutationFn: ({ full }: { full: boolean }) =>
      coreApi.runWorkspaceCommandAsync(placementId, {
        command: "make",
        args: [full ? "c" : "l"],
        intent: "check",
        label: full ? "Full check" : "Quick check",
        timeout_seconds: full ? 120 : 60,
      }),
    onSuccess: (accepted) => {
      setActiveCheckId(accepted.command_id);
    },
  });
  const cancelCheck = useMutation({
    mutationFn: (commandId: string) =>
      coreApi.cancelWorkspaceCommand(placementId, commandId),
  });
  const saveDiff = useMutation({
    mutationFn: () =>
      coreApi.createArtifact({
        artifact_type: "uprava.diff-report",
        title: `Workspace diff · ${review?.diff.scope ?? scope}`,
        scope_ref: { kind: "placement", project_placement_id: placementId },
        schema_version: 1,
        payload: review
          ? {
              summary: review.diff.summary,
              diff: review.diff.diff.slice(0, 400_000),
              scope: review.diff.scope,
              path: review.diff.path,
              changed_files: review.diff.changed_files,
              hunks: review.diff.hunks,
              generated_at: review.diff.generated_at,
            }
          : {},
        fallback_text: review?.diff.summary ?? "Workspace diff snapshot",
        source_version: review?.git_snapshot.commit ?? null,
        source_refs: review
          ? [
              {
                kind: "workspace_diff",
                diff_id: review.diff.diff_id,
                placement_id: placementId,
              },
            ]
          : [],
        evidence_refs: [],
        cause_refs: [],
        trace_refs: [],
        provenance: {
          kind: "workspace_review_snapshot",
          generated_at: review?.generated_at,
        },
      }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["artifacts"] }),
  });
  const saveChecks = useMutation({
    mutationFn: () => {
      const checks = (review?.checks ?? []).slice(-50).map((check) => ({
        ...check,
        stdout: check.stdout?.slice(0, 20_000) ?? null,
        stderr: check.stderr?.slice(0, 20_000) ?? null,
      }));
      return coreApi.createArtifact({
        artifact_type: "uprava.check-report",
        title: "Workspace checks",
        scope_ref: { kind: "placement", project_placement_id: placementId },
        schema_version: 1,
        payload: {
          summary: `${checks.length} recorded checks`,
          checks,
          generated_at: review?.generated_at,
        },
        fallback_text: `${checks.length} recorded workspace checks`,
        source_version: review?.git_snapshot.commit ?? null,
        source_refs: [],
        evidence_refs: checks.map((check) => ({
          kind: "command" as const,
          command_id: check.command_id,
        })),
        cause_refs: [],
        trace_refs: [],
        provenance: {
          kind: "workspace_review_snapshot",
          generated_at: review?.generated_at,
        },
      });
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["artifacts"] }),
  });
  const activeState = activeCheck.data?.state;

  useEffect(() => {
    if (!activeCheckId || !activeState || !terminalStates.has(activeState))
      return;
    void queryClient.invalidateQueries({
      queryKey: ["placement", placementId, "workspace-review"],
    });
    setActiveCheckId(null);
  }, [activeCheckId, activeState, placementId, queryClient]);

  if (error) return <ErrorNotice error={error} title="Review unavailable" />;
  if (!review) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-[var(--color-muted)]">
        {isLoading ? "Loading review" : "No review snapshot"}
      </div>
    );
  }

  const snapshot = review.git_snapshot;
  const selected = review.diff.path ? review.diff : null;

  return (
    <section className="flex h-full min-h-0 flex-col bg-[var(--color-bg)]">
      <header className="space-y-2 border-b border-[var(--color-muted)] px-3 py-2">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone={snapshot.state === "ready" ? "good" : "warn"}>
              {snapshot.branch ?? snapshot.head_state ?? snapshot.state}
            </Badge>
            {snapshot.commit ? (
              <span className="font-mono text-xs text-[var(--color-muted)]">
                {snapshot.commit.slice(0, 8)}
              </span>
            ) : null}
            {snapshot.worktree_kind ? (
              <Badge tone="info">{snapshot.worktree_kind} worktree</Badge>
            ) : null}
            {snapshot.upstream ? (
              <span className="text-xs text-[var(--color-muted)]">
                {snapshot.upstream} · ↑{snapshot.ahead} ↓{snapshot.behind}
              </span>
            ) : null}
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              variant="secondary"
              disabled={saveDiff.isPending || saveDiff.isSuccess}
              onClick={() => saveDiff.mutate()}
            >
              <Pin size={15} />
              {saveDiff.isSuccess
                ? "Diff pinned"
                : saveDiff.isPending
                  ? "Pinning…"
                  : "Pin diff"}
            </Button>
            {saveDiff.data ? (
              <ReferenceActions
                reference={{
                  kind: "artifact",
                  artifact_id: saveDiff.data.artifact.artifact_id,
                }}
                showCopy={false}
              />
            ) : null}
            <Button
              variant="secondary"
              disabled={isLoading}
              onClick={onRefresh}
            >
              <RefreshCw size={15} />
              {isLoading ? "Refreshing" : "Refresh"}
            </Button>
          </div>
        </div>
        {saveDiff.error ? (
          <ErrorNotice error={saveDiff.error} title="Diff artifact failed" />
        ) : null}
        <div className="flex flex-wrap items-center gap-2">
          {snapshot.head_state === "detached" ? (
            <Badge tone="warn">Detached HEAD</Badge>
          ) : null}
          {snapshot.operation ? (
            <Badge tone="warn">{snapshot.operation} in progress</Badge>
          ) : null}
          {snapshot.conflicted_count > 0 ? (
            <Badge tone="bad">{snapshot.conflicted_count} conflicts</Badge>
          ) : null}
          {snapshot.truncated ? <Badge tone="warn">Truncated</Badge> : null}
          <span className="text-xs text-[var(--color-muted)]">
            Snapshot {new Date(snapshot.generated_at).toLocaleString()}
          </span>
        </div>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(12rem,16rem)_minmax(0,1fr)]">
        <aside className="flex min-h-0 flex-col border-r border-[var(--color-muted)]">
          <div className="flex gap-1 border-b border-[var(--color-muted)] p-2">
            {(["all", "staged", "unstaged"] as const).map((candidate) => (
              <Button
                key={candidate}
                className="h-7 px-2 text-xs capitalize"
                variant={scope === candidate ? "primary" : "ghost"}
                onClick={() => onScopeChange(candidate)}
              >
                {candidate}
              </Button>
            ))}
          </div>
          <div className="min-h-0 flex-1 overflow-auto py-1">
            {review.diff.changed_files.length === 0 ? (
              <div className="p-3 text-sm text-[var(--color-muted)]">
                No changes in this scope
              </div>
            ) : (
              review.diff.changed_files.map((file) => (
                <ChangedFileButton
                  key={`${file.previous_path ?? ""}:${file.path}`}
                  file={file}
                  selected={selectedPath === file.path}
                  onClick={() => onSelectPath(file.path)}
                />
              ))
            )}
          </div>
        </aside>

        <div className="flex min-h-0 flex-col gap-3 overflow-auto p-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div>
              <div className="text-sm font-medium">
                {selectedPath ?? "Select a changed file"}
              </div>
              <div className="text-xs text-[var(--color-muted)]">
                {review.diff.summary}
              </div>
            </div>
            {selectedPath ? (
              <div className="flex items-center gap-1">
                <ReferenceActions
                  reference={{
                    kind: "workspace_diff",
                    diff_id: review.diff.diff_id,
                    placement_id: placementId,
                  }}
                  showCopy={false}
                />
                <Button
                  variant="ghost"
                  onClick={() => onOpenSource(selectedPath)}
                >
                  Open source
                </Button>
              </div>
            ) : null}
          </div>

          {selected &&
          selected.original !== null &&
          selected.modified !== null ? (
            <Suspense fallback={<ReviewFallback />}>
              <MonacoWorkspaceDiffViewer
                placementId={placementId}
                path={selected.path ?? "diff"}
                original={selected.original ?? ""}
                modified={selected.modified ?? ""}
              />
            </Suspense>
          ) : selected ? (
            <pre className="min-h-48 overflow-auto whitespace-pre bg-[var(--color-bg-muted)] p-3 font-mono text-xs leading-5">
              {selected.binary
                ? "Binary file changed; textual preview unavailable."
                : selected.diff || "No textual diff available."}
            </pre>
          ) : null}

          <section className="border-t border-[var(--color-muted)] pt-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <h3 className="text-sm font-semibold">Checks</h3>
                <p className="text-xs text-[var(--color-muted)]">
                  Bounded commands with durable output and trace references.
                </p>
              </div>
              <div className="flex gap-2">
                <Button
                  variant="secondary"
                  disabled={
                    review.checks.length === 0 ||
                    saveChecks.isPending ||
                    saveChecks.isSuccess
                  }
                  onClick={() => saveChecks.mutate()}
                >
                  <Pin size={14} />
                  {saveChecks.isSuccess
                    ? "Report pinned"
                    : saveChecks.isPending
                      ? "Pinning…"
                      : "Pin report"}
                </Button>
                {saveChecks.data ? (
                  <ReferenceActions
                    reference={{
                      kind: "artifact",
                      artifact_id: saveChecks.data.artifact.artifact_id,
                    }}
                    showCopy={false}
                  />
                ) : null}
                {activeCheckId ? (
                  <>
                    <Badge tone="info">{activeState ?? "starting"}</Badge>
                    <Button
                      variant="danger"
                      disabled={cancelCheck.isPending}
                      onClick={() => cancelCheck.mutate(activeCheckId)}
                    >
                      <Square size={14} /> Stop check
                    </Button>
                  </>
                ) : (
                  <>
                    <Button
                      disabled={runCheck.isPending}
                      onClick={() => runCheck.mutate({ full: false })}
                    >
                      Quick · make l
                    </Button>
                    <Button
                      disabled={runCheck.isPending}
                      onClick={() => runCheck.mutate({ full: true })}
                    >
                      Full · make c
                    </Button>
                  </>
                )}
              </div>
            </div>
            {runCheck.error ? (
              <ErrorNotice error={runCheck.error} title="Check start failed" />
            ) : null}
            {saveChecks.error ? (
              <ErrorNotice
                error={saveChecks.error}
                title="Check artifact failed"
              />
            ) : null}
            <div className="mt-3 space-y-2">
              {review.checks.length === 0 ? (
                <div className="text-sm text-[var(--color-muted)]">
                  No recorded checks
                </div>
              ) : (
                review.checks.map((check) => (
                  <details
                    key={check.command_id}
                    className="border border-[var(--color-muted)] p-2"
                  >
                    <summary className="flex cursor-pointer list-none items-center gap-2 text-sm">
                      {check.success === true ? (
                        <CheckCircle2 size={15} />
                      ) : check.success === false ? (
                        <XCircle
                          size={15}
                          className="text-[var(--color-risk)]"
                        />
                      ) : (
                        <RefreshCw size={15} />
                      )}
                      <span className="font-medium">
                        {check.label ??
                          `${check.command} ${check.args.join(" ")}`}
                      </span>
                      <Badge tone={checkTone(check.success)}>
                        {check.state}
                      </Badge>
                      <span className="ml-auto text-xs text-[var(--color-muted)]">
                        {check.duration_ms === null
                          ? new Date(check.created_at).toLocaleString()
                          : `${check.duration_ms} ms`}
                      </span>
                      <ReferenceActions
                        reference={{
                          kind: "command",
                          command_id: check.command_id,
                        }}
                        showCopy={false}
                      />
                    </summary>
                    <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap bg-[var(--color-bg-muted)] p-2 font-mono text-xs">
                      {[check.stdout, check.stderr]
                        .filter(Boolean)
                        .join("\n") || "No output"}
                    </pre>
                  </details>
                ))
              )}
            </div>
          </section>
        </div>
      </div>
    </section>
  );
}

function ChangedFileButton({
  file,
  selected,
  onClick,
}: {
  file: GitChangedFile;
  selected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      className={`flex w-full items-center gap-2 px-3 py-2 text-left text-xs hover:bg-[var(--color-bg-muted)] ${selected ? "bg-[var(--color-bg-muted)] font-semibold" : ""}`}
      onClick={onClick}
    >
      <span className="min-w-0 flex-1 truncate font-mono" title={file.path}>
        {file.path}
      </span>
      {file.index_status ? (
        <StatusBadge label="S" status={file.index_status} />
      ) : null}
      {file.worktree_status ? (
        <StatusBadge label="W" status={file.worktree_status} />
      ) : null}
    </button>
  );
}

function StatusBadge({
  label,
  status,
}: {
  label: string;
  status: GitChangeKind;
}) {
  return (
    <span
      className="font-mono text-[10px] text-[var(--color-muted)]"
      title={status}
    >
      {label}:{status.slice(0, 1).toUpperCase()}
    </span>
  );
}

function checkTone(success: boolean | null): BadgeTone {
  if (success === true) return "good";
  if (success === false) return "bad";
  return "info";
}

function ReviewFallback() {
  return (
    <div className="flex min-h-64 items-center justify-center text-sm text-[var(--color-muted)]">
      Loading diff renderer
    </div>
  );
}
